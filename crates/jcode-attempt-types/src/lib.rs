//! Data contracts for frozen attempts, receipts and data-class admission.
//!
//! This crate is pure: serde types, small validators, no I/O and no dependency
//! on app-core. It is the type home for the three deterministic authority
//! points described in `docs/HARNESS_LOOP_ARCHITECTURE.md`:
//!
//! 1. admission (before): [`DataClass`] and [`RouteClass`] eligibility,
//! 2. the frozen envelope (during): [`AttemptRecord`] and [`FrozenAttempt`],
//! 3. acceptance (after): [`Receipt`] and [`validate_receipt_for_gate`].
//!
//! Models propose. Code owns these three points.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Admission: data class and route class
// ---------------------------------------------------------------------------

/// Provenance class of a packet's content. The default is the most
/// restrictive value so that an unlabelled packet never leaves the machine.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
    /// Already public content (upstream repositories, public docs).
    Public,
    /// Fixtures authored for evaluation with no real user or client content.
    Synthetic,
    /// Anything derived from the working tree, sessions or client material.
    #[default]
    Private,
    /// Credentials, tokens, keys. Never leaves the process that resolved it.
    Secret,
}

/// Where a route executes and under which contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteClass {
    /// Deterministic code or a local model in-process or on loopback.
    Local,
    /// An explicitly pinned included-subscription route the operator admitted
    /// for private context (for example an OpenAI or Anthropic OAuth lane).
    IncludedSubscription,
    /// A metered remote route (OpenRouter, native TypeSafe, direct API keys).
    MeteredRemote,
}

impl DataClass {
    /// Fail-closed eligibility. Private content may only use local routes or
    /// explicitly admitted included-subscription routes. Secrets never travel.
    pub fn is_remote_eligible(self, route: RouteClass) -> bool {
        match (self, route) {
            (DataClass::Secret, _) => false,
            (_, RouteClass::Local) => true,
            (DataClass::Public | DataClass::Synthetic, _) => true,
            (DataClass::Private, RouteClass::IncludedSubscription) => true,
            (DataClass::Private, RouteClass::MeteredRemote) => false,
        }
    }
}

/// Model families that must never be reached through a router or aggregator,
/// including nested router slugs. Explicit included-subscription routes are
/// admitted separately and are not routers.
const BANNED_ROUTER_FAMILIES: &[&str] = &[
    "openai/",
    "anthropic/",
    "gpt-",
    "claude",
    "o1-",
    "o3-",
    "o4-",
];

/// Router or aggregator provider ids that require model-family screening.
const ROUTER_PROVIDERS: &[&str] = &["openrouter", "open-inference", "pareto", "openrouter/"];

/// Returns true when `provider` is a router/aggregator and `model` names a
/// banned family, directly or through a nested router slug such as
/// `openrouter/pareto-code` or `openrouter/auto`.
pub fn is_banned_router_family(provider: &str, model: &str) -> bool {
    let provider = provider.trim().to_ascii_lowercase();
    let model = model.trim().to_ascii_lowercase();
    let via_router = ROUTER_PROVIDERS
        .iter()
        .any(|p| provider == p.trim_end_matches('/') || provider.starts_with(p));
    if !via_router {
        return false;
    }
    // Nested routers (auto, pareto, free) may select any family: fail closed.
    if model.starts_with("openrouter/") || model == "auto" || model.contains("pareto") {
        return true;
    }
    BANNED_ROUTER_FAMILIES
        .iter()
        .any(|f| model.starts_with(f) || model.contains(&format!("/{f}")))
}

// ---------------------------------------------------------------------------
// Frozen attempt
// ---------------------------------------------------------------------------

/// Reasoning effort, mirroring the swarm spawn vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    XHigh,
    Max,
}

/// What this harness will initiate for one attempt. This is a local
/// reservation, not an account-side guarantee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalBudget {
    #[serde(default)]
    pub max_input_bytes: u64,
    #[serde(default)]
    pub max_output_bytes: u64,
    /// Micro-USD ceiling for metered routes; 0 for local and subscription routes.
    #[serde(default)]
    pub max_micro_usd: u64,
    /// Generations this attempt may request. One means no retry.
    #[serde(default = "one")]
    pub max_generations: u32,
}

fn one() -> u32 {
    1
}

impl Default for LocalBudget {
    /// Keep the derived default identical to the serde default: one generation,
    /// never zero, so a defaulted budget is still an admissible envelope.
    fn default() -> Self {
        Self {
            max_input_bytes: 0,
            max_output_bytes: 0,
            max_micro_usd: 0,
            max_generations: one(),
        }
    }
}

/// Everything that must be decided before an executor runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub task_id: String,
    pub attempt_id: String,
    pub node_id: String,
    pub provider: String,
    /// Exact model or version slug. No `latest` aliases.
    pub model_exact: String,
    pub endpoint: String,
    pub route_class: RouteClass,
    #[serde(default)]
    pub effort: Effort,
    /// Empty means no tools at all.
    #[serde(default)]
    pub tool_allowlist: Vec<String>,
    #[serde(default)]
    pub data_class: DataClass,
    pub deadline_secs: u64,
    #[serde(default)]
    pub budget: LocalBudget,
    /// Digest of the exact packet the executor will receive.
    pub prompt_hash: String,
    pub policy_version: String,
}

/// Errors raised by [`AttemptRecord::freeze`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum FreezeError {
    EmptyField(String),
    DataClassNotEligible {
        data_class: DataClass,
        route_class: RouteClass,
    },
    BannedRouterFamily {
        provider: String,
        model: String,
    },
    LatestAlias(String),
    ZeroDeadline,
    ZeroGenerations,
}

impl std::fmt::Display for FreezeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FreezeError::EmptyField(name) => write!(f, "attempt field `{name}` is empty"),
            FreezeError::DataClassNotEligible {
                data_class,
                route_class,
            } => write!(
                f,
                "data class {data_class:?} is not eligible for route class {route_class:?}"
            ),
            FreezeError::BannedRouterFamily { provider, model } => {
                write!(
                    f,
                    "model `{model}` may not be reached through router `{provider}`"
                )
            }
            FreezeError::LatestAlias(model) => write!(f, "model `{model}` is a floating alias"),
            FreezeError::ZeroDeadline => write!(f, "deadline must be positive"),
            FreezeError::ZeroGenerations => write!(f, "budget must allow at least one generation"),
        }
    }
}

impl std::error::Error for FreezeError {}

impl AttemptRecord {
    /// Validate admission invariants and return an immutable wrapper. There
    /// are no mutators on [`FrozenAttempt`]; a change means a new attempt id.
    pub fn freeze(self, frozen_at: DateTime<Utc>) -> Result<FrozenAttempt, FreezeError> {
        for (name, value) in [
            ("task_id", &self.task_id),
            ("attempt_id", &self.attempt_id),
            ("node_id", &self.node_id),
            ("provider", &self.provider),
            ("model_exact", &self.model_exact),
            ("endpoint", &self.endpoint),
            ("prompt_hash", &self.prompt_hash),
            ("policy_version", &self.policy_version),
        ] {
            if value.trim().is_empty() {
                return Err(FreezeError::EmptyField(name.to_string()));
            }
        }
        if !self.data_class.is_remote_eligible(self.route_class) {
            return Err(FreezeError::DataClassNotEligible {
                data_class: self.data_class,
                route_class: self.route_class,
            });
        }
        if is_banned_router_family(&self.provider, &self.model_exact) {
            return Err(FreezeError::BannedRouterFamily {
                provider: self.provider,
                model: self.model_exact,
            });
        }
        let lower = self.model_exact.to_ascii_lowercase();
        if lower.ends_with(":latest") || lower.ends_with("-latest") || lower == "latest" {
            return Err(FreezeError::LatestAlias(self.model_exact));
        }
        if self.deadline_secs == 0 {
            return Err(FreezeError::ZeroDeadline);
        }
        if self.budget.max_generations == 0 {
            return Err(FreezeError::ZeroGenerations);
        }
        Ok(FrozenAttempt {
            record: self,
            frozen_at,
        })
    }
}

/// An admitted, immutable attempt. Only read access is exposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenAttempt {
    record: AttemptRecord,
    frozen_at: DateTime<Utc>,
}

impl FrozenAttempt {
    pub fn record(&self) -> &AttemptRecord {
        &self.record
    }

    pub fn frozen_at(&self) -> DateTime<Utc> {
        self.frozen_at
    }

    pub fn attempt_id(&self) -> &str {
        &self.record.attempt_id
    }
}

// ---------------------------------------------------------------------------
// Receipts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    /// A shell command run by the harness (tests, linters, compilers).
    Command,
    /// A remote model call.
    ModelCall,
    /// A local model inference (loopback or in-process).
    LocalModel,
}

/// Token usage on a model call, if the route reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub micro_usd: Option<u64>,
}

/// Deterministic evidence that something ran. Generated by the harness, never
/// by the worker, so a worker with shell access cannot forge it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub attempt_id: String,
    pub kind: ReceiptKind,
    /// Human-readable command or route label. Never contains secret values.
    pub cmd: String,
    pub argv_hash: String,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub started: DateTime<Utc>,
    pub finished: DateTime<Utc>,
    /// Identity of the binary or model that produced the result: a path plus
    /// hash, or `provider:model_exact` for model calls.
    pub binary_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Effective telemetry settings observed at execution time, for example
    /// `("PI_TELEMETRY", "0")`. Recorded so R11 is checkable per receipt.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub effective_telemetry: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum ReceiptError {
    AttemptIdMismatch {
        expected: String,
        found: String,
    },
    FinishedBeforeStarted,
    MissingExitCode,
    MissingDigest(String),
    EmptyField(String),
    /// A telemetry key the policy requires is absent or not disabled.
    TelemetryNotDisabled(String),
}

impl std::fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReceiptError::AttemptIdMismatch { expected, found } => {
                write!(
                    f,
                    "receipt attempt id `{found}` does not match `{expected}`"
                )
            }
            ReceiptError::FinishedBeforeStarted => write!(f, "receipt finished before it started"),
            ReceiptError::MissingExitCode => write!(f, "command receipt has no exit code"),
            ReceiptError::MissingDigest(which) => write!(f, "receipt {which} digest is empty"),
            ReceiptError::EmptyField(name) => write!(f, "receipt field `{name}` is empty"),
            ReceiptError::TelemetryNotDisabled(key) => {
                write!(f, "receipt does not record `{key}` as disabled")
            }
        }
    }
}

impl std::error::Error for ReceiptError {}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Structural validity of a receipt independent of any attempt: fields
/// present, time ordered, command receipts carry an exit code, digests are
/// sha256 hex. Used by gates that only see the receipt.
pub fn validate_receipt_shape(receipt: &Receipt) -> Result<(), ReceiptError> {
    for (name, value) in [
        ("attempt_id", &receipt.attempt_id),
        ("cmd", &receipt.cmd),
        ("argv_hash", &receipt.argv_hash),
        ("cwd", &receipt.cwd),
        ("binary_id", &receipt.binary_id),
    ] {
        if value.trim().is_empty() {
            return Err(ReceiptError::EmptyField(name.to_string()));
        }
    }
    if receipt.finished < receipt.started {
        return Err(ReceiptError::FinishedBeforeStarted);
    }
    if receipt.kind == ReceiptKind::Command && receipt.exit_code.is_none() {
        return Err(ReceiptError::MissingExitCode);
    }
    if !is_sha256_hex(&receipt.stdout_sha256) {
        return Err(ReceiptError::MissingDigest("stdout".into()));
    }
    if !is_sha256_hex(&receipt.stderr_sha256) {
        return Err(ReceiptError::MissingDigest("stderr".into()));
    }
    Ok(())
}

/// The minimum a Verify gate must see before it may close a node in deep mode.
pub fn validate_receipt_for_gate(
    receipt: &Receipt,
    attempt: &FrozenAttempt,
) -> Result<(), ReceiptError> {
    if receipt.attempt_id != attempt.attempt_id() {
        return Err(ReceiptError::AttemptIdMismatch {
            expected: attempt.attempt_id().to_string(),
            found: receipt.attempt_id.clone(),
        });
    }
    validate_receipt_shape(receipt)?;
    validate_telemetry_disabled(receipt)
}

/// Telemetry keys, with the value that means "disabled", that must be recorded
/// on a receipt of the given kind. Local model receipts must prove vendor
/// telemetry was off (R11). `DO_NOT_TRACK=1` is the supplemental signal.
pub fn required_disabled_telemetry(kind: ReceiptKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        ReceiptKind::LocalModel => &[("DO_NOT_TRACK", "1")],
        ReceiptKind::Command | ReceiptKind::ModelCall => &[],
    }
}

/// Check that every required telemetry key is recorded with its disabling
/// value. Extra keys such as `NEEDLE_TELEMETRY=0` or `PI_TELEMETRY=0` are
/// checked when present: any value other than `0` fails.
pub fn validate_telemetry_disabled(receipt: &Receipt) -> Result<(), ReceiptError> {
    for (key, disabled) in required_disabled_telemetry(receipt.kind) {
        if receipt.effective_telemetry.get(*key).map(|v| v.trim()) != Some(*disabled) {
            return Err(ReceiptError::TelemetryNotDisabled(key.to_string()));
        }
    }
    for (key, value) in &receipt.effective_telemetry {
        if key.ends_with("_TELEMETRY") && value.trim() != "0" {
            return Err(ReceiptError::TelemetryNotDisabled(key.clone()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Namespaced confidence
// ---------------------------------------------------------------------------

/// A worker's free-text self-report, parsed to a rung by `jcode-plan`. A
/// breadth/honesty signal for gates, never a probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfReportedRung {
    Low,
    Medium,
    High,
}

/// A calibrated, group-level statistic from a typed System 1 judgment (Jev
/// Choice/Score). Not the probability that this individual answer is correct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibratedGroupConfidence {
    pub value: f32,
    pub model_hash: String,
    pub task_class: String,
}

/// Confidence from a local extractor (Needle-class). `None` when the engine
/// reports none, for example for tuned weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalExtractorConfidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,
    pub engine: String,
    pub weights_hash: String,
}

/// Every confidence a gate may consume, kept in separate variants so code, not
/// prompts, decides which kind gates what.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Confidence {
    SelfReported { rung: SelfReportedRung },
    CalibratedGroup(CalibratedGroupConfidence),
    LocalExtractor(LocalExtractorConfidence),
}

// ---------------------------------------------------------------------------
// Secretless packets
// ---------------------------------------------------------------------------

/// Shape-only secret detection for worker packets. This mirrors the token
/// families in `jcode_base::message::redact_secrets` without pulling that
/// crate in. It is a fail-closed gate on packet serialization, not a
/// declassifier: a clean result never authorizes export of private data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretShape {
    pub path: String,
    pub family: &'static str,
}

const SECRET_PREFIXES: &[(&str, usize, &str)] = &[
    ("sk-ant-", 30, "anthropic_key"),
    ("sk-or-v1-", 30, "openrouter_key"),
    ("sk-", 32, "openai_style_key"),
    ("ghp_", 24, "github_token"),
    ("github_pat_", 30, "github_pat"),
    ("ya29.", 25, "google_oauth"),
    ("AIza", 24, "google_api_key"),
    ("xoxb-", 15, "slack_token"),
    ("xoxp-", 15, "slack_token"),
    ("AKIA", 20, "aws_access_key"),
    ("-----BEGIN ", 20, "private_key_block"),
];

fn token_like_len(s: &str) -> usize {
    s.bytes()
        .take_while(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'+' | b'/' | b'=')
        })
        .count()
}

fn scan_str(path: &str, text: &str, out: &mut Vec<SecretShape>) {
    for (prefix, min_len, family) in SECRET_PREFIXES {
        for (idx, _) in text.match_indices(prefix) {
            let rest = &text[idx..];
            if *prefix == "-----BEGIN " {
                if rest.contains("PRIVATE KEY-----") {
                    out.push(SecretShape {
                        path: path.to_string(),
                        family,
                    });
                }
                continue;
            }
            if token_like_len(rest) >= *min_len {
                out.push(SecretShape {
                    path: path.to_string(),
                    family,
                });
            }
        }
    }
    // Bearer <token>
    let lower = text.to_ascii_lowercase();
    for (idx, _) in lower.match_indices("bearer ") {
        let rest = text[idx + 7..].trim_start();
        if token_like_len(rest) >= 20 {
            out.push(SecretShape {
                path: path.to_string(),
                family: "bearer_token",
            });
        }
    }
    // JWT: three base64url segments starting with eyJ
    for (idx, _) in text.match_indices("eyJ") {
        let rest = &text[idx..];
        let segs: Vec<&str> = rest.split('.').take(3).collect();
        if segs.len() == 3
            && segs
                .iter()
                .all(|s| token_like_len(s) >= 10 && !s.contains('/'))
        {
            out.push(SecretShape {
                path: path.to_string(),
                family: "jwt",
            });
        }
    }
}

fn scan_value(path: &str, value: &serde_json::Value, out: &mut Vec<SecretShape>) {
    match value {
        serde_json::Value::String(s) => scan_str(path, s, out),
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                scan_value(&format!("{path}[{i}]"), item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                let key_lower = k.to_ascii_lowercase();
                if let serde_json::Value::String(s) = v
                    && (key_lower.contains("api_key")
                        || key_lower.contains("apikey")
                        || key_lower == "authorization"
                        || key_lower.contains("password")
                        || key_lower.contains("secret")
                        || key_lower.ends_with("_token"))
                    && !s.trim().is_empty()
                {
                    out.push(SecretShape {
                        path: child.clone(),
                        family: "secret_named_field",
                    });
                }
                scan_value(&child, v, out);
            }
        }
        _ => {}
    }
}

/// Return every secret-shaped string in a JSON packet, with its path.
pub fn find_secret_shapes(packet: &serde_json::Value) -> Vec<SecretShape> {
    let mut out = Vec::new();
    scan_value("", packet, &mut out);
    out
}

/// Fail-closed helper: `Err` lists offending paths.
pub fn assert_no_secret_shapes(packet: &serde_json::Value) -> Result<(), Vec<SecretShape>> {
    let found = find_secret_shapes(packet);
    if found.is_empty() { Ok(()) } else { Err(found) }
}

#[cfg(test)]
mod tests;
