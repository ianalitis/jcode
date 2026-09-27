//! Shared Jev typed Decisions transport, separate from chat completions.
//!
//! BYOK credentials are bound to fixed provider endpoints. The Jcode route uses
//! the configured trusted account gateway, and checks its live purpose-specific
//! capability before each evaluation. Credential presence is not entitlement.

use anyhow::{Result, anyhow, bail, ensure};
use reqwest::{Client, Response, Url};
use serde_json::{Map, Value, json};
use std::time::Duration;

const PROVIDER_ENV: &str = "JCODE_MEMORY_JEV_PROVIDER";
const BROWSER_PROVIDER_ENV: &str = "JCODE_BROWSER_JEV_PROVIDER";
/// Routing decisions are a third consumer with their own key. The two live
/// consumers above are deliberately not reused: a routing decision and a memory
/// recall have different failure costs, and one switch must not move both.
const ROUTING_PROVIDER_ENV: &str = "JCODE_ROUTING_JEV_PROVIDER";
const VOICE_PROVIDER_ENV: &str = "JCODE_VOICE_JEV_PROVIDER";
const MAX_REQUEST_BYTES: usize = 80 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_ME_BYTES: usize = 16 * 1024;
pub(crate) const MAX_QUESTIONS: usize = 24;
/// The typed decision contract's closed option set is 2 to 16 entries
/// (`jcode-s1-eval::MAX_OPTIONS`). The routing consumer is that contract's
/// transport, so it enforces the same cap rather than the wider 255 the wire
/// format allows.
const MAX_ROUTING_OPTIONS: usize = 16;
/// Typesafe direct accepts far more (256 verified live on 2026-09-23). A voice
/// request has at most 26 questions, so it always fits in one round trip.
const TYPESAFE_MAX_QUESTIONS: usize = 64;

/// Typesafe latency is bimodal and sticky per connection: a connection is
/// served in ~150ms or 2-12s (measured live 2026-09-23). Voice waits on this
/// before inserting or sending, so duplicates on fresh connections race the
/// primary. Decisions are side-effect-free and cost fractions of a cent.
#[cfg(not(test))]
const VOICE_HEDGE_DELAYS: [Duration; 3] = [
    Duration::from_millis(300),
    Duration::from_millis(700),
    Duration::from_millis(1500),
];
#[cfg(test)]
const VOICE_HEDGE_DELAYS: [Duration; 3] = [
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(150),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JevPurpose {
    Memory,
    Browser,
    Routing,
    Voice,
}

impl JevPurpose {
    fn name(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Browser => "browser",
            Self::Routing => "routing",
            Self::Voice => "voice",
        }
    }

    fn capability(self) -> &'static str {
        match self {
            Self::Memory => "memory_jev",
            Self::Browser => "browser_jev",
            Self::Routing => "routing_jev",
            // Voice uses the gateway's existing typed noul contract.
            Self::Voice => "memory_jev",
        }
    }

    fn selector_with(
        self,
        env: impl FnOnce(&str) -> Result<String, std::env::VarError>,
        memory_default: impl FnOnce() -> String,
    ) -> Result<String> {
        let key = match self {
            Self::Memory => PROVIDER_ENV,
            Self::Browser => BROWSER_PROVIDER_ENV,
            Self::Routing => ROUTING_PROVIDER_ENV,
            Self::Voice => VOICE_PROVIDER_ENV,
        };
        match env(key) {
            Ok(value) => Ok(value),
            Err(std::env::VarError::NotPresent) => Ok(match self {
                Self::Memory => memory_default(),
                // Browser and routing are independent of memory configuration and
                // default to subscription-first auto selection.
                Self::Browser | Self::Routing => "auto".into(),
                Self::Voice => "auto".into(),
            }),
            Err(_) => bail!("{key} must contain a valid provider name"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JevProvider {
    OpenRouter,
    TypeSafe,
    Aimlapi,
    Jcode,
}

impl JevProvider {
    fn name(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::TypeSafe => "typesafe",
            Self::Aimlapi => "aimlapi",
            Self::Jcode => "jcode",
        }
    }

    fn credentials(self) -> (&'static str, &'static str) {
        match self {
            Self::OpenRouter => ("OPENROUTER_API_KEY", "openrouter.env"),
            Self::TypeSafe => ("TYPESAFE_API_KEY", "typesafe.env"),
            Self::Aimlapi => ("AIMLAPI_API_KEY", "aimlapi.env"),
            Self::Jcode => (
                crate::subscription_catalog::JCODE_API_KEY_ENV,
                crate::subscription_catalog::JCODE_ENV_FILE,
            ),
        }
    }

    fn max_questions(self) -> usize {
        match self {
            Self::TypeSafe => TYPESAFE_MAX_QUESTIONS,
            _ => MAX_QUESTIONS,
        }
    }

    fn model(self) -> &'static str {
        match self {
            Self::OpenRouter | Self::Jcode => "typesafe/jev-1.13",
            Self::TypeSafe => "jev-latest",
            Self::Aimlapi => "typesafe/jev",
        }
    }

    fn endpoint(self, gateway_base: &str) -> Result<String> {
        Ok(match self {
            Self::OpenRouter => "https://openrouter.ai/api/alpha/decisions".into(),
            Self::TypeSafe => "https://api.typesafe.ai/v1/systemone".into(),
            Self::Aimlapi => "https://api.aimlapi.com/v1/decisions".into(),
            Self::Jcode => format!("{}/decisions", trusted_gateway_base(gateway_base)?),
        })
    }
}

/// Do not derive Debug: this contains a provider secret.
#[derive(Clone)]
pub struct JevClient {
    client: Client,
    /// Voice only. Typesafe speed is sticky per connection (a slow connection
    /// stays slow), so each hedge needs its own client and connection.
    hedge_clients: Vec<Client>,
    purpose: JevPurpose,
    provider: JevProvider,
    api_key: String,
    endpoint: String,
    me_endpoint: Option<String>,
}

impl JevClient {
    /// A configured credential route exists. This is not a health or entitlement
    /// probe. In particular, Jcode entitlement is checked live by `evaluate`.
    pub fn available() -> bool {
        Self::resolve(JevPurpose::Memory).is_ok()
    }

    pub fn new() -> Result<Self> {
        Self::for_purpose(JevPurpose::Memory)
    }

    /// Browser routing is independent of memory configuration and defaults to
    /// subscription-first auto selection. Evaluation never changes accounts.
    pub fn for_browser() -> Result<Self> {
        Self::for_purpose(JevPurpose::Browser)
    }

    /// Routing decisions are a third consumer, keyed by
    /// `JCODE_ROUTING_JEV_PROVIDER` and independent of the memory and browser
    /// switches, exactly as the browser consumer is independent of memory.
    pub fn for_routing() -> Result<Self> {
        Self::for_purpose(JevPurpose::Routing)
    }

    /// Voice uses included Jcode access (whose gateway uses Typesafe directly),
    /// then Typesafe BYOK when no Jcode credential exists. Other provider keys and
    /// memory/browser configuration are ignored. JCODE_VOICE_JEV_PROVIDER may
    /// explicitly select typesafe or jcode. Evaluation never changes accounts.
    pub fn for_voice() -> Result<Self> {
        Self::for_purpose(JevPurpose::Voice)
    }

    fn for_purpose(purpose: JevPurpose) -> Result<Self> {
        let (provider, api_key, endpoint, me_endpoint) = Self::resolve(purpose)?;
        let build = || {
            client_builder()
                .build()
                .map_err(|_| anyhow!("Could not initialize the Jev decision client"))
        };
        let client = build()?;
        let hedge_clients = if purpose == JevPurpose::Voice {
            VOICE_HEDGE_DELAYS
                .iter()
                .map(|_| build())
                .collect::<Result<_>>()?
        } else {
            Vec::new()
        };
        Ok(Self {
            client,
            hedge_clients,
            purpose,
            provider,
            api_key,
            endpoint,
            me_endpoint,
        })
    }

    fn resolve(purpose: JevPurpose) -> Result<(JevProvider, String, String, Option<String>)> {
        let selector = purpose.selector_with(
            |key| std::env::var(key),
            || crate::config::config().agents.memory_jev_provider.clone(),
        )?;
        // Unlike the API-key helper, this does not consult registered
        // cross-provider fallback resolvers or the shared compatible slot.
        let load = |env: &str, file: &str| {
            crate::provider_catalog::load_env_value_from_env_or_config(env, file)
        };
        let (provider, api_key) = if purpose == JevPurpose::Voice {
            resolve_voice_with(&selector, load)?
        } else {
            resolve_with(&selector, load)?
        };
        let base = if provider == JevProvider::Jcode {
            crate::subscription_api::configured_api_base()
        } else {
            String::new()
        };
        let endpoint = provider.endpoint(&base)?;
        let me_endpoint = if provider == JevProvider::Jcode {
            Some(format!("{}/me", trusted_gateway_base(&base)?))
        } else {
            None
        };
        Ok((provider, api_key, endpoint, me_endpoint))
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    pub fn model_id(&self) -> &str {
        self.provider.model()
    }

    /// Largest question batch this route accepts in one request.
    pub(crate) fn max_questions(&self) -> usize {
        self.provider.max_questions()
    }

    /// Return the full typed Decisions response, including provider usage.
    /// Never retries using another provider or account after an auth/billing
    /// failure. Callers own the relevance threshold and uncertainty policy.
    pub async fn evaluate(&self, state: Value, questions: Map<String, Value>) -> Result<Value> {
        let body = request_body_for(self.purpose, self.provider, state, &questions)?;
        if let Some(endpoint) = &self.me_endpoint {
            let response = self
                .client
                .get(endpoint)
                .bearer_auth(&self.api_key)
                .timeout(crate::subscription_api::ME_FETCH_TIMEOUT)
                .send()
                .await
                .map_err(|_| {
                    anyhow!(
                        "Could not verify Jcode {} entitlement; try again later",
                        self.purpose.name()
                    )
                })?;
            let me = read_response(response, MAX_ME_BYTES, self.provider).await?;
            ensure!(
                me["capabilities"]
                    .get(self.purpose.capability())
                    .and_then(Value::as_bool)
                    == Some(true),
                "Jcode Jev {} is unavailable for this account or gateway. An active entitled subscription and a gateway with {} support are required. Configure a Jev BYOK provider to use your own account.",
                self.purpose.name(),
                self.purpose.capability()
            );
        }
        if self.purpose == JevPurpose::Voice {
            return self.send_hedged(body, &questions).await;
        }
        let value = self.send(&self.endpoint, body).await?;
        validate_answers(&value, &questions)?;
        Ok(value)
    }

    /// Send once, then again on a fresh connection after each
    /// [`VOICE_HEDGE_DELAYS`] step while nothing has answered. The first valid
    /// answer wins and the rest are dropped. A primary failure returns at once,
    /// so auth/billing errors are never duplicated. Hedge failures are ignored
    /// while any other attempt is still pending.
    async fn send_hedged(&self, body: Vec<u8>, questions: &Map<String, Value>) -> Result<Value> {
        use futures::StreamExt;
        let attempt = |client: &Client, delay: Duration, primary: bool| {
            let body = body.clone();
            let client = client.clone();
            async move {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                let result = async {
                    let value = self.send_with(&client, &self.endpoint, body).await?;
                    validate_answers(&value, questions)?;
                    Ok::<_, anyhow::Error>(value)
                }
                .await;
                (primary, result)
            }
        };
        let mut attempts = futures::stream::FuturesUnordered::new();
        attempts.push(attempt(&self.client, Duration::ZERO, true));
        for (client, delay) in self.hedge_clients.iter().zip(VOICE_HEDGE_DELAYS) {
            attempts.push(attempt(client, delay, false));
        }
        let mut hedge_error = None;
        while let Some((primary, result)) = attempts.next().await {
            match result {
                Ok(value) => return Ok(value),
                Err(error) if primary => return Err(error),
                Err(error) => hedge_error = Some(error),
            }
        }
        Err(hedge_error.unwrap_or_else(|| anyhow!("Jev decision request failed")))
    }

    async fn send(&self, endpoint: &str, body: Vec<u8>) -> Result<Value> {
        self.send_with(&self.client, endpoint, body).await
    }

    /// Decisions requests are side-effect-free classifications, so transient
    /// overload responses (429/502/503/504/529) are retried with a short bounded
    /// backoff on the same provider and account. Auth, billing, redirect, and
    /// other failures are never retried and never fall back to another account.
    async fn send_with(&self, client: &Client, endpoint: &str, body: Vec<u8>) -> Result<Value> {
        let mut attempt = 0;
        loop {
            let response = self.send_once(client, endpoint, body.clone()).await?;
            let status = response.status().as_u16();
            // A long Retry-After means a plan quota (hours), not overload.
            // Retrying cannot succeed, so surface the upgrade prompt at once.
            let long_wait = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<u64>().ok())
                .is_some_and(|secs| Duration::from_secs(secs) > MAX_RETRY_AFTER);
            if attempt < TRANSIENT_RETRY_DELAYS.len() && is_transient_status(status) && !long_wait {
                let delay = retry_after(&response).unwrap_or(TRANSIENT_RETRY_DELAYS[attempt]);
                attempt += 1;
                crate::logging::info(&format!(
                    "Jev {} returned HTTP {status}; retry {attempt}/{} in {}ms",
                    self.provider.name(),
                    TRANSIENT_RETRY_DELAYS.len(),
                    delay.as_millis()
                ));
                tokio::time::sleep(delay).await;
                continue;
            }
            return read_response(response, MAX_RESPONSE_BYTES, self.provider).await;
        }
    }

    async fn send_once(&self, client: &Client, endpoint: &str, body: Vec<u8>) -> Result<Response> {
        let mut request = client
            .post(endpoint)
            .bearer_auth(&self.api_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        if self.provider == JevProvider::Jcode && self.purpose == JevPurpose::Browser {
            // The gateway budgets browser handoffs separately so background
            // memory recall cannot exhaust interactive browsing.
            request = request.header("X-Jcode-Jev-Purpose", "browser");
        }
        if self.provider == JevProvider::OpenRouter {
            request = request.header("HTTP-Referer", "https://jcode.sh").header(
                "X-Title",
                match self.purpose {
                    JevPurpose::Memory => "Jcode Memory",
                    JevPurpose::Browser => "Jcode Browser",
                    JevPurpose::Routing => "Jcode Routing",
                    JevPurpose::Voice => "Jcode Voice",
                },
            );
        }
        request.send().await.map_err(|_| {
            anyhow!("Jev decision request failed or timed out; check the selected provider")
        })
    }
}

#[cfg(not(test))]
const TRANSIENT_RETRY_DELAYS: [Duration; 2] =
    [Duration::from_millis(600), Duration::from_millis(1800)];
#[cfg(test)]
const TRANSIENT_RETRY_DELAYS: [Duration; 2] = [Duration::from_millis(1), Duration::from_millis(1)];
const MAX_RETRY_AFTER: Duration = Duration::from_secs(5);

fn is_transient_status(status: u16) -> bool {
    matches!(status, 429 | 502 | 503 | 504 | 529)
}

/// Honor a short numeric Retry-After. Long waits fail fast instead of
/// stalling the caller beyond the bounded retry budget.
fn retry_after(response: &Response) -> Option<Duration> {
    let secs: u64 = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let delay = Duration::from_secs(secs);
    if cfg!(test) {
        return Some(Duration::from_millis(1));
    }
    (delay <= MAX_RETRY_AFTER).then_some(delay)
}

fn client_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::none())
}

fn resolve_voice_with(
    selector: &str,
    load: impl FnMut(&str, &str) -> Option<String>,
) -> Result<(JevProvider, String)> {
    match selector.trim().to_ascii_lowercase().as_str() {
        "auto" => resolve_providers(&[JevProvider::Jcode, JevProvider::TypeSafe], load),
        "typesafe" | "jcode" | "subscription" | "jcode-subscription" => {
            resolve_with(selector, load)
        }
        _ => bail!(
            "Invalid voice Jev provider. Choose auto, typesafe, or jcode; voice never uses OpenRouter or AIMLAPI"
        ),
    }
}

fn resolve_with(
    selector: &str,
    load: impl FnMut(&str, &str) -> Option<String>,
) -> Result<(JevProvider, String)> {
    let providers: &[JevProvider] = match selector.trim().to_ascii_lowercase().as_str() {
        "auto" => &[
            // Included subscriber access wins over personal paid provider keys.
            // Entitlement is checked live before evaluation. Failure must not
            // silently spend a BYOK balance; users can select BYOK explicitly.
            // Typesafe serves Jev directly, so it beats resellers of the same model.
            JevProvider::Jcode,
            JevProvider::TypeSafe,
            JevProvider::OpenRouter,
            JevProvider::Aimlapi,
        ],
        "openrouter" => &[JevProvider::OpenRouter],
        "typesafe" => &[JevProvider::TypeSafe],
        "aimlapi" => &[JevProvider::Aimlapi],
        "jcode" | "subscription" | "jcode-subscription" => &[JevProvider::Jcode],
        _ => bail!("Invalid Jev provider. Choose auto, openrouter, typesafe, aimlapi, or jcode"),
    };
    resolve_providers(providers, load)
}

fn resolve_providers(
    providers: &[JevProvider],
    mut load: impl FnMut(&str, &str) -> Option<String>,
) -> Result<(JevProvider, String)> {
    for &provider in providers {
        let (env, file) = provider.credentials();
        if let Some(key) = load(env, file) {
            let key = jcode_provider_env::sanitize_secret_value(&key);
            if !key.is_empty() {
                // Reject malformed headers now rather than leaking a provider's
                // request-builder error through a later error chain.
                ensure!(
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {key}")).is_ok(),
                    "The selected Jev provider credential is not a valid HTTP bearer value"
                );
                return Ok((provider, key.to_owned()));
            }
        }
    }
    if let [provider] = providers {
        let (env, file) = provider.credentials();
        bail!(
            "No credential for the selected Jev provider {}. Configure {env} ({file}). Explicit provider selection never falls back to another account.",
            provider.name()
        );
    }
    let credentials = providers
        .iter()
        .map(|provider| {
            let (env, file) = provider.credentials();
            format!("{env} ({file})")
        })
        .collect::<Vec<_>>()
        .join(" or ");
    bail!(
        "No credential for the selected Jev route. Configure {credentials}. Provider failures never fall back to another account."
    )
}

fn trusted_gateway_base(base: &str) -> Result<String> {
    let url = Url::parse(base.trim()).map_err(|_| anyhow!("Invalid Jcode gateway base URL"))?;
    let loopback = url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    ensure!(
        url.host_str().is_some()
            && (url.scheme() == "https" || (url.scheme() == "http" && loopback))
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Jcode Jev gateway requires HTTPS (HTTP only for loopback), without URL credentials, query, or fragment"
    );
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

#[cfg(test)]
fn request_body(
    provider: JevProvider,
    state: Value,
    questions: &Map<String, Value>,
) -> Result<Vec<u8>> {
    request_body_for(JevPurpose::Memory, provider, state, questions)
}

fn request_body_for(
    purpose: JevPurpose,
    provider: JevProvider,
    state: Value,
    questions: &Map<String, Value>,
) -> Result<Vec<u8>> {
    if purpose == JevPurpose::Browser {
        ensure!(
            questions.len() == 1
                && questions.get("action").is_some_and(|question| {
                    question["type"] == "choice"
                        && question["instructions"]
                            .as_str()
                            .is_some_and(|s| !s.trim().is_empty())
                        && question["criteria"].as_object().is_some_and(|criteria| {
                            (2..=255).contains(&criteria.len())
                                && criteria.values().all(Value::is_string)
                        })
                }),
            "Browser Decisions requires exactly one action choice question with text instructions and 2 to 255 text criteria"
        );
    }
    ensure!(
        state.is_string() || state.is_object() || state.is_array(),
        "Jev state must be text, an object, or an array"
    );
    ensure!(
        (1..=provider.max_questions()).contains(&questions.len()),
        "Jev requests require between 1 and {} questions",
        provider.max_questions()
    );
    for (id, question) in questions {
        ensure!(!id.is_empty() && id.len() <= 64, "Invalid Jev question ID");
        let instructions = &question["instructions"];
        ensure!(
            instructions.is_string() || instructions.is_object() || instructions.is_array(),
            "Jev questions require instructions"
        );
        match question["type"].as_str() {
            Some("noul") => {}
            Some("choice") => {
                let criteria = question["criteria"].as_object();
                ensure!(
                    criteria.is_some_and(|criteria| {
                        (2..=255).contains(&criteria.len())
                            && criteria.values().all(|v| v.is_string() || v.is_null())
                    }),
                    "Jev choice questions require 2 to 255 described options"
                );
                if purpose == JevPurpose::Routing {
                    ensure!(
                        criteria.is_some_and(|criteria| criteria.len() <= MAX_ROUTING_OPTIONS),
                        "Routing Decisions follows the typed decision contract: at most 16 described options"
                    );
                }
            }
            Some("score") => {
                let criteria = question["criteria"].as_array();
                ensure!(
                    criteria.is_some_and(|criteria| {
                        (2..=255).contains(&criteria.len()) && criteria.iter().all(Value::is_string)
                    }),
                    "Jev score questions require 2 to 255 level descriptions"
                );
                if purpose == JevPurpose::Routing {
                    ensure!(
                        criteria.is_some_and(|criteria| criteria.len() <= MAX_ROUTING_OPTIONS),
                        "Routing Decisions follows the typed decision contract: at most 16 levels"
                    );
                }
            }
            _ => bail!("Unsupported Jev question type; expected noul, choice, or score"),
        }
        if provider == JevProvider::Jcode && purpose != JevPurpose::Browser {
            ensure!(
                question["type"] == "noul"
                    && instructions.as_str().is_some_and(|s| !s.trim().is_empty())
                    && question["criteria"]["true"].is_string()
                    && question["criteria"]["false"].is_string(),
                "Jcode memory Decisions supports noul questions with text instructions and true/false criteria"
            );
        }
    }
    // String state is accepted by every provider and preserves the existing
    // OpenRouter Decisions wire contract. Direct providers retain structured data.
    let state =
        if matches!(provider, JevProvider::OpenRouter | JevProvider::Jcode) && !state.is_string() {
            Value::String(serde_json::to_string(&state).map_err(|_| anyhow!("Invalid Jev state"))?)
        } else {
            state
        };
    let body = serde_json::to_vec(&json!({
        "model": provider.model(), "state": state, "questions": questions
    }))
    .map_err(|_| anyhow!("Could not encode Jev request"))?;
    ensure!(
        body.len() <= MAX_REQUEST_BYTES,
        "Jev request exceeds the bounded context size"
    );
    Ok(body)
}

/// Parse the gateway's plan-quota body. Only trusted, bounded fields are kept,
/// and the upgrade link must be an https jcode.sh URL so a compromised or
/// misconfigured gateway cannot inject an arbitrary link into the UI.
async fn read_quota_exceeded(
    response: Response,
) -> Option<crate::subscription_notice::QuotaExceeded> {
    let bytes = response.bytes().await.ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let body: Value = serde_json::from_slice(&bytes).ok()?;
    let error = body.get("error")?;
    if error.get("code")?.as_str()? != "quota_exceeded" {
        return None;
    }
    let text = |key: &str| {
        error
            .get(key)
            .and_then(Value::as_str)
            .map(|value| value.chars().take(300).collect::<String>())
    };
    let upgrade_url = text("upgrade_url").filter(|url| {
        url.starts_with("https://jcode.sh/") || url.starts_with("https://www.jcode.sh/")
    });
    Some(crate::subscription_notice::QuotaExceeded {
        feature: text("purpose").unwrap_or_else(|| "decision".into()),
        tier: text("tier"),
        upgrade_tier: text("upgrade_tier"),
        upgrade_url,
        resets_at: text("resets_at"),
    })
}

async fn read_response(
    mut response: Response,
    limit: usize,
    provider: JevProvider,
) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        if provider == JevProvider::Jcode && status.as_u16() == 429 {
            if let Some(quota) = read_quota_exceeded(response).await {
                crate::subscription_notice::record(quota.clone());
                return Err(quota.into());
            }
            bail!(
                "Jev returned HTTP 429: selected provider is rate limited or overloaded; try again later"
            );
        }
        let hint = match status.as_u16() {
            401 => "selected provider credential is invalid or revoked",
            403 => "selected provider denied access or the account is not entitled",
            402 => "selected provider credits or account spending limit are exhausted",
            404 => "selected gateway does not support this Jev endpoint",
            429 | 529 => "selected provider is rate limited or overloaded; try again later",
            300..=399 => "redirect refused to protect provider credentials",
            _ => "selected provider is unavailable or rejected the request",
        };
        bail!("Jev returned HTTP {}: {hint}", status.as_u16());
    }
    ensure!(
        !response
            .content_length()
            .is_some_and(|length| length > limit as u64),
        "Jev response exceeds the bounded response size"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow!("Could not read Jev response"))?
    {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= limit,
            "Jev response exceeds the bounded response size"
        );
        bytes.extend_from_slice(&chunk);
    }
    // Never retain serde's diagnostic, which can quote untrusted response data.
    serde_json::from_slice(&bytes).map_err(|_| anyhow!("Jev returned invalid response JSON"))
}

fn validate_answers(value: &Value, questions: &Map<String, Value>) -> Result<()> {
    let answers = value["answers"]
        .as_object()
        .ok_or_else(|| anyhow!("Jev returned no typed answers"))?;
    ensure!(
        answers.len() == questions.len(),
        "Jev returned an incomplete or unexpected answer set"
    );
    for (id, question) in questions {
        let answer = answers
            .get(id)
            .ok_or_else(|| anyhow!("Jev omitted a requested answer"))?;
        ensure!(
            answer["type"] == question["type"],
            "Jev answer type does not match its question"
        );
        if question["type"] == "noul" {
            ensure!(
                answer["noul"]
                    .as_f64()
                    .is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v)),
                "Jev returned an invalid noul probability"
            );
        } else if question["type"] == "choice" {
            // The declared criteria keys are the authoritative option set: the
            // browser path offers its action IDs as exactly these keys, so an
            // answer naming anything else is not a decision the caller offered.
            ensure!(
                answer["choice"].as_str().is_some_and(|choice| {
                    question["criteria"]
                        .as_object()
                        .is_some_and(|options| options.contains_key(choice))
                }),
                "Jev returned a choice outside the question's declared options"
            );
        } else if question["type"] == "score" {
            // A score question declares 2 to 255 level descriptions, so a
            // selected level cannot exceed that count. The exact wire encoding
            // (zero- or one-based level index) is not established by any caller
            // or fixture here, so accept the union of both plausible bases and
            // reject only values out of range under either.
            let levels = question["criteria"].as_array().map_or(0, Vec::len);
            ensure!(
                answer["score"].as_f64().is_some_and(|score| {
                    score.is_finite() && (0.0..=levels as f64).contains(&score)
                }),
                "Jev returned a score outside the question's declared levels"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    include!("jev_routing_tests.rs");
    include!("jev_transport_tests.rs");
}
