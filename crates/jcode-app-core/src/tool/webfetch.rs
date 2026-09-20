use super::{Tool, ToolContext, ToolOutput};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};
use url::{Host, Url};

const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB
/// Cap on the text handed back to the model. Full pages routinely exceed 150 KB
/// (~40k tokens) which is rarely worth the context budget.
const MAX_OUTPUT_CHARS: usize = 40_000;
/// Links whose target exceeds this length are rendered as their anchor text
/// only. Long URLs are typically encoded payloads (pre-filled editors, tracking
/// parameters, data URIs) whose cost far exceeds their navigational value.
const MAX_URL_CHARS: usize = 300;
const DEFAULT_TIMEOUT: u64 = 30;
const MAX_TIMEOUT: u64 = 120;
const MAX_REDIRECTS: usize = 10;

pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug)]
struct ResolvedTarget {
    url: Url,
    domain: Option<String>,
    addrs: Vec<SocketAddr>,
}

impl ResolvedTarget {
    async fn resolve(url: Url) -> Result<Self> {
        if !matches!(url.scheme(), "http" | "https") {
            bail!("URL must use http:// or https://");
        }

        let port = url
            .port_or_known_default()
            .context("URL has no usable port")?;
        let host = url.host().context("URL must include a host")?;

        match host {
            Host::Ipv4(ip) => {
                ensure_public_ip(IpAddr::V4(ip))?;
                Ok(Self {
                    url,
                    domain: None,
                    addrs: Vec::new(),
                })
            }
            Host::Ipv6(ip) => {
                ensure_public_ip(IpAddr::V6(ip))?;
                Ok(Self {
                    url,
                    domain: None,
                    addrs: Vec::new(),
                })
            }
            Host::Domain(domain) => {
                let domain = domain.to_string();
                let mut addrs: Vec<_> = tokio::net::lookup_host((domain.as_str(), port))
                    .await
                    .with_context(|| format!("failed to resolve Webfetch host {domain}"))?
                    .collect();
                addrs.sort_unstable();
                addrs.dedup();
                if addrs.is_empty() {
                    bail!("Webfetch host {domain} resolved to no addresses");
                }
                for addr in &addrs {
                    ensure_public_ip(addr.ip())?;
                }
                Ok(Self {
                    url,
                    domain: Some(domain),
                    addrs,
                })
            }
        }
    }

    fn client(&self) -> Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            // A proxy can resolve the original hostname again and bypass the
            // validated, pinned addresses below.
            .no_proxy()
            .connect_timeout(Duration::from_secs(15))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .pool_max_idle_per_host(0);
        if let Some(domain) = &self.domain {
            builder = builder.resolve_to_addrs(domain, &self.addrs);
        }
        builder.build().context("failed to build Webfetch client")
    }
}

fn ensure_public_ip(ip: IpAddr) -> Result<()> {
    let public = match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    };
    if !public {
        bail!("Webfetch blocked non-public destination {ip}");
    }
    Ok(())
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 168)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();

    // IPv4-mapped IPv6 follows the embedded IPv4 destination policy.
    if segments[..5] == [0; 5] && segments[5] == 0xffff {
        return is_public_ipv4(Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            segments[6] as u8,
            (segments[7] >> 8) as u8,
            segments[7] as u8,
        ));
    }

    // Current globally routable unicast space is 2000::/3. Exclude special
    // allocations inside it that can tunnel or represent non-public targets.
    (segments[0] & 0xe000) == 0x2000
        && !(segments[0] == 0x2001 && segments[1] <= 0x01ff)
        && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
        && segments[0] != 0x2002
        && !(segments[0] == 0x3fff && (segments[1] & 0xf000) == 0)
}

fn followed_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

fn redirect_url(base: &Url, location: &str) -> Result<Url> {
    base.join(location)
        .with_context(|| format!("invalid Webfetch redirect from {base}"))
}

fn remaining_timeout(
    deadline: tokio::time::Instant,
    now: tokio::time::Instant,
) -> Result<Duration> {
    let remaining = deadline.saturating_duration_since(now);
    if remaining.is_zero() {
        bail!("Webfetch timed out");
    }
    Ok(remaining)
}

async fn fetch_response(url: &str, timeout: Duration) -> Result<(reqwest::Response, Url)> {
    let mut current = Url::parse(url).context("invalid Webfetch URL")?;
    let deadline = tokio::time::Instant::now() + timeout;

    for redirect_count in 0..=MAX_REDIRECTS {
        let target = tokio::time::timeout_at(deadline, ResolvedTarget::resolve(current))
            .await
            .context("Webfetch timed out while resolving a destination")??;
        let client = target.client()?;
        let request_timeout = remaining_timeout(deadline, tokio::time::Instant::now())?;
        let response = client
            .get(target.url.clone())
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (compatible; JCode/1.0)",
            )
            .timeout(request_timeout)
            .send()
            .await?;

        if !followed_redirect(response.status()) {
            return Ok((response, target.url));
        }
        if redirect_count == MAX_REDIRECTS {
            bail!("Webfetch exceeded {MAX_REDIRECTS} redirects");
        }

        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .context("Webfetch redirect is missing a Location header")?
            .to_str()
            .context("Webfetch redirect Location is not valid text")?;
        current = redirect_url(&target.url, location)?;
    }

    unreachable!("redirect loop always returns or errors")
}

#[derive(Deserialize)]
struct WebFetchInput {
    url: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default)]
    retain_evidence: bool,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "Fetch a URL."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "intent": super::intent_schema_property(),
                "url": {
                    "type": "string",
                    "description": "URL, or an evidence: reference to reread a retained snapshot without network access."
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "description": "Output format."
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds."
                },
                "retain_evidence": {
                    "type": "boolean",
                    "default": false,
                    "description": "Retain this public HTTPS page as session evidence (private cache, 24h, 32 max). Still untrusted."
                },
                "offset": {
                    "type": "integer", "minimum": 0,
                    "description": "For evidence: references only: UTF-8 byte offset into the stored transformed text."
                },
                "limit": {
                    "type": "integer", "minimum": 1, "maximum": 40000,
                    "description": "For evidence: references only: excerpt byte limit (default 8000)."
                }
            }
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let params: WebFetchInput = serde_json::from_value(input)?;
        if params.url.starts_with("evidence:") {
            anyhow::ensure!(
                !params.retain_evidence && params.format.is_none() && params.timeout.is_none(),
                "Evidence rereads accept only url, offset and limit; no network request was made"
            );
            return read_evidence(
                &evidence_root()?,
                &params,
                &ctx.session_id,
                chrono::Utc::now().timestamp(),
            );
        }
        anyhow::ensure!(
            params.offset.is_none() && params.limit.is_none(),
            "Paging requires an evidence: reference"
        );
        if params.retain_evidence {
            let url = research_url(&params.url)?;
            anyhow::ensure!(
                !ctx.session_id.is_empty(),
                "Research capture requires a session"
            );
            anyhow::ensure!(
                matches!(
                    params.format.as_deref(),
                    None | Some("text" | "markdown" | "html")
                ),
                "Invalid research format"
            );
            let root = evidence_root()?;
            prepare_evidence(&root)?;
            let timeout = params.timeout.unwrap_or(DEFAULT_TIMEOUT).min(MAX_TIMEOUT);
            return tokio::time::timeout(Duration::from_secs(timeout), async {
                let client = research_client(&url).await?;
                let response = client
                    .get(url)
                    .header(
                        reqwest::header::USER_AGENT,
                        "Mozilla/5.0 (compatible; JCode/1.0)",
                    )
                    .send()
                    .await?;
                render_response(response, &params, &ctx.session_id, Some(&root)).await
            })
            .await
            .map_err(|_| {
                anyhow::anyhow!("Research capture timed out; no reference was returned")
            })?;
        }

        let timeout = params.timeout.unwrap_or(DEFAULT_TIMEOUT).min(MAX_TIMEOUT);

        // `render_response` reports `response.url()`, which is the pinned final
        // destination after redirects, so the returned URL is not needed here.
        let (response, _final_url) =
            fetch_response(&params.url, Duration::from_secs(timeout)).await?;

        render_response(response, &params, &ctx.session_id, None).await
    }
}

/// Note appended when the fetched body was cut to [`MAX_OUTPUT_CHARS`]. Names
/// the on-disk copy when one was written, so the tail stays reachable.
fn truncation_note(full_len: usize, spill: Option<&std::path::Path>) -> String {
    match spill {
        Some(path) => format!(
            "\n\n(output truncated to {MAX_OUTPUT_CHARS} of {full_len} chars; \
             full response saved at {} — read that path with offset/limit, \
             or fetch a more specific URL or anchor)",
            path.display()
        ),
        None => format!(
            "\n\n(output truncated to {MAX_OUTPUT_CHARS} of {full_len} chars; \
             fetch a more specific URL or anchor for the rest)"
        ),
    }
}

async fn render_response(
    response: reqwest::Response,
    params: &WebFetchInput,
    session: &str,
    evidence: Option<&std::path::Path>,
) -> Result<ToolOutput> {
    let format = params.format.as_deref().unwrap_or("markdown");
    let final_url = response.url().to_string();

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", status));
    }

    // Check content length
    if let Some(len) = response.content_length()
        && len as usize > MAX_SIZE
    {
        return Err(anyhow::anyhow!(
            "Response too large: {} bytes (max {} bytes)",
            len,
            MAX_SIZE
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if evidence.is_some() {
        let mime = content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        anyhow::ensure!(
            content_type.len() <= 256
                && (mime.starts_with("text/")
                    || matches!(
                        mime.as_str(),
                        "application/json" | "application/xml" | "application/javascript"
                    )),
            "Research capture requires a supported textual content type"
        );
    }

    let mut body_bytes = Vec::new();
    let mut truncated = false;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let remaining = MAX_SIZE.saturating_sub(body_bytes.len());
        if chunk.len() > remaining {
            body_bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body_bytes.extend_from_slice(&chunk);
    }

    let mut body = String::from_utf8_lossy(&body_bytes).into_owned();
    if truncated && evidence.is_none() {
        body.push_str(&format!(
            "...\n\n(truncated, showing first {} bytes)",
            MAX_SIZE
        ));
    }

    // Format output
    let output = match format {
        "html" => body,
        "text" => html_to_text(&body),
        "markdown" => {
            if content_type.contains("text/html") {
                html_to_markdown(&body)
            } else {
                body
            }
        }
        _ => {
            if content_type.contains("text/html") {
                html_to_markdown(&body)
            } else {
                body
            }
        }
    };

    if let Some(root) = evidence {
        let now = chrono::Utc::now();
        let mut receipt = EvidenceReceipt {
            version: 1,
            reference: String::new(),
            session_hash: digest(session.as_bytes()),
            requested_url: params.url.clone(),
            final_url,
            fetched_at: now.to_rfc3339(),
            expires_at: now.timestamp() + EVIDENCE_TTL_SECS,
            status: status.as_u16(),
            content_type,
            transform: format!("webfetch-v1:{format}"),
            raw_sha256: digest(&body_bytes),
            text_sha256: digest(output.as_bytes()),
            raw_bytes: body_bytes.len(),
            text_bytes: output.len(),
            body_complete: !truncated,
            lossy_utf8: std::str::from_utf8(&body_bytes).is_err(),
        };
        save_evidence(root, &mut receipt, &body_bytes, &output)?;
        return evidence_output(&receipt, &output, 0, EVIDENCE_PREVIEW_BYTES);
    }

    let full_len = output.len();
    // Keep the whole response reachable before dropping the tail: a caller who
    // needs the rest should not have to guess a narrower URL or anchor.
    let spill = (full_len > MAX_OUTPUT_CHARS)
        .then(|| {
            crate::agent::tool_output_spill::spill_truncated_output(session, "webfetch", &output)
        })
        .flatten();
    let (output, output_truncated) = truncate_output(output);

    let note = if output_truncated {
        truncation_note(full_len, spill.as_deref())
    } else {
        String::new()
    };

    Ok(ToolOutput::new(format!(
        "Fetched {} ({} bytes)\n\n{}{}",
        params.url, full_len, output, note
    )))
}

// Retention is explicit and confined to a bounded, private cache. These receipts
// are untrusted source snapshots, not immutable attestations or permission grants.
const EVIDENCE_TTL_SECS: i64 = 24 * 60 * 60;
const MAX_EVIDENCE_RECORDS: usize = 32;
const EVIDENCE_PREVIEW_BYTES: usize = 8_000;

#[derive(serde::Serialize, Deserialize)]
struct EvidenceReceipt {
    version: u32,
    reference: String,
    session_hash: String,
    requested_url: String,
    final_url: String,
    fetched_at: String,
    expires_at: i64,
    status: u16,
    content_type: String,
    transform: String,
    raw_sha256: String,
    text_sha256: String,
    raw_bytes: usize,
    text_bytes: usize,
    body_complete: bool,
    lossy_utf8: bool,
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn public_address(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && (b == 168 || (b == 0 && matches!(c, 0 | 2)) || (b == 88 && c == 99)))
                || (a == 198 && (matches!(b, 18 | 19) || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        std::net::IpAddr::V6(ip) => {
            let s = ip.segments();
            // Conservative global-unicast subset. Exclude mapped/translation,
            // special-use, documentation and 6to4 ranges rather than guessing.
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

fn research_url(input: &str) -> Result<reqwest::Url> {
    anyhow::ensure!(input.len() <= 4096, "Research URL exceeds byte cap");
    let url = reqwest::Url::parse(input).map_err(|_| anyhow::anyhow!("Invalid research URL"))?;
    anyhow::ensure!(
        url.scheme() == "https" && url.port_or_known_default() == Some(443),
        "Research capture requires HTTPS on port 443"
    );
    anyhow::ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Research capture rejects credentials, query strings and fragments"
    );
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Research URL requires a host"))?;
    anyhow::ensure!(
        !host.eq_ignore_ascii_case("localhost") && !host.ends_with(".localhost"),
        "Research capture rejects local hosts"
    );
    if let Ok(ip) = host.trim_matches(['[', ']']).parse() {
        anyhow::ensure!(
            public_address(ip),
            "Research capture rejects non-public addresses"
        );
    }
    Ok(url)
}

async fn research_client(url: &reqwest::Url) -> Result<reqwest::Client> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Missing research host"))?;
    let addresses: Vec<_> = tokio::net::lookup_host((host.trim_matches(['[', ']']), 443))
        .await?
        .collect();
    pinned_research_client(host, &addresses)
}

fn pinned_research_client(
    host: &str,
    addresses: &[std::net::SocketAddr],
) -> Result<reqwest::Client> {
    anyhow::ensure!(
        !addresses.is_empty()
            && addresses
                .iter()
                .all(|a| public_address(a.ip()) && a.port() == 443),
        "Research DNS must resolve exclusively to public addresses"
    );
    // The checked addresses are the ones used for the connection. No second DNS
    // lookup, proxy, shared authentication, cookies or automatic redirects.
    Ok(reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, addresses)
        .build()?)
}

fn evidence_root() -> Result<std::path::PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("research-evidence"))
}

fn check_private_directory(path: &std::path::Path) -> Result<()> {
    anyhow::ensure!(
        cfg!(unix),
        "Research retention requires Unix private-file support"
    );
    let meta = std::fs::symlink_metadata(path)?;
    anyhow::ensure!(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "Invalid evidence directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        // SAFETY: geteuid has no preconditions and does not dereference pointers.
        anyhow::ensure!(
            meta.permissions().mode() & 0o077 == 0 && meta.uid() == unsafe { libc::geteuid() },
            "Evidence directory must be owner-only"
        );
    }
    Ok(())
}

fn private_file(path: &std::path::Path, create: bool) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(create).create(create);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    anyhow::ensure!(meta.is_file(), "Evidence must be a regular file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        // SAFETY: geteuid has no preconditions and does not dereference pointers.
        anyhow::ensure!(
            meta.permissions().mode() & 0o077 == 0
                && meta.nlink() == 1
                && meta.uid() == unsafe { libc::geteuid() },
            "Evidence file must be private and unlinked elsewhere"
        );
    }
    Ok(file)
}

fn evidence_lock(root: &std::path::Path) -> Result<std::fs::File> {
    check_private_directory(root)?;
    let file = private_file(&root.join(".lock"), true)?;
    file.try_lock()
        .map_err(|_| anyhow::anyhow!("Evidence store is busy; do not refetch"))?;
    let count = std::fs::read_dir(root)?
        .try_fold(0usize, |n, entry| -> std::io::Result<usize> {
            Ok(n + usize::from(entry?.file_name() != ".lock"))
        })?;
    anyhow::ensure!(
        count < MAX_EVIDENCE_RECORDS,
        "Evidence store is full (32 snapshots); operator cleanup required, do not refetch"
    );
    Ok(file)
}

fn prepare_evidence(root: &std::path::Path) -> Result<()> {
    anyhow::ensure!(
        cfg!(unix),
        "Research retention requires Unix private-file support"
    );
    if let Some(parent) = root.parent() {
        crate::storage::ensure_dir(parent)?;
    }
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(root) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.into()),
    }
    let _lock = evidence_lock(root)?;
    Ok(())
}

fn save_evidence(
    root: &std::path::Path,
    receipt: &mut EvidenceReceipt,
    raw: &[u8],
    text: &str,
) -> Result<()> {
    use std::io::Write;
    anyhow::ensure!(
        raw.len() <= MAX_SIZE && text.len() <= MAX_SIZE,
        "Evidence exceeds storage byte cap"
    );
    let _lock = evidence_lock(root)?;
    let mut builder = tempfile::Builder::new();
    builder.prefix("fetch-").rand_bytes(16);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let dir = builder.tempdir_in(root)?;
    check_private_directory(dir.path())?;
    let id = dir
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid evidence identifier"))?;
    receipt.reference = format!("evidence:{id}");
    let metadata = serde_json::to_vec(receipt)?;
    anyhow::ensure!(
        metadata.len() <= 16_384,
        "Evidence metadata exceeds byte cap"
    );
    for (name, bytes) in [
        ("raw", raw),
        ("text", text.as_bytes()),
        ("receipt.json", metadata.as_slice()),
    ] {
        let mut file = private_file(&dir.path().join(name), true)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    // Successful snapshots survive process restart. Expiry denies retrieval;
    // deletion is operator-controlled, and the fixed quota bounds retained data.
    let _retained = dir.keep();
    Ok(())
}

fn read_evidence_file(path: &std::path::Path, max: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = private_file(path, false)?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= max, "Evidence file exceeds byte cap");
    Ok(bytes)
}

fn evidence_output(
    receipt: &EvidenceReceipt,
    text: &str,
    offset: usize,
    limit: usize,
) -> Result<ToolOutput> {
    anyhow::ensure!(
        offset <= text.len() && text.is_char_boundary(offset),
        "Invalid UTF-8 byte offset"
    );
    anyhow::ensure!(
        (1..=MAX_OUTPUT_CHARS).contains(&limit),
        "Evidence limit must be 1..40000 bytes"
    );
    let end = text.floor_char_boundary(offset.saturating_add(limit).min(text.len()));
    anyhow::ensure!(
        end > offset || offset == text.len(),
        "Limit is too small for the next UTF-8 character"
    );
    let metadata = json!({"evidence": receipt, "offset": offset, "next_offset": end,
        "excerpt_truncated": end < text.len(), "trust": "untrusted_source", "craap": "not_assessed"});
    Ok(ToolOutput::new(format!(
        "Evidence {} (untrusted source, CRAAP not assessed)\n{}\n\nBytes {offset}..{end} of {}:\n{}",
        receipt.reference, serde_json::to_string(&metadata)?, text.len(), &text[offset..end]
    )).with_metadata(metadata))
}

fn read_evidence(
    root: &std::path::Path,
    params: &WebFetchInput,
    session: &str,
    now: i64,
) -> Result<ToolOutput> {
    anyhow::ensure!(!session.is_empty(), "Evidence retrieval requires a session");
    let id = params
        .url
        .strip_prefix("evidence:fetch-")
        .ok_or_else(|| anyhow::anyhow!("Invalid evidence reference"))?;
    anyhow::ensure!(
        id.len() == 16 && id.bytes().all(|b| b.is_ascii_alphanumeric()),
        "Invalid evidence reference"
    );
    check_private_directory(root)?;
    let dir = root.join(format!("fetch-{id}"));
    check_private_directory(&dir)?;
    let receipt: EvidenceReceipt =
        serde_json::from_slice(&read_evidence_file(&dir.join("receipt.json"), 16_384)?)?;
    anyhow::ensure!(
        receipt.version == 1
            && receipt.reference == params.url
            && receipt.session_hash == digest(session.as_bytes()),
        "Evidence is not available to this session"
    );
    anyhow::ensure!(
        now < receipt.expires_at,
        "Evidence expired; no network request was made"
    );
    let raw = read_evidence_file(&dir.join("raw"), MAX_SIZE)?;
    let text = String::from_utf8(read_evidence_file(&dir.join("text"), MAX_SIZE)?)?;
    anyhow::ensure!(
        digest(&raw) == receipt.raw_sha256
            && digest(text.as_bytes()) == receipt.text_sha256
            && raw.len() == receipt.raw_bytes
            && text.len() == receipt.text_bytes,
        "Evidence integrity check failed"
    );
    evidence_output(
        &receipt,
        &text,
        params.offset.unwrap_or(0),
        params.limit.unwrap_or(EVIDENCE_PREVIEW_BYTES),
    )
}

/// Truncate at a char boundary, preferring to cut at the last newline so the tail
/// is not a half-formed line.
fn truncate_output(output: String) -> (String, bool) {
    if output.len() <= MAX_OUTPUT_CHARS {
        return (output, false);
    }
    let mut cut = MAX_OUTPUT_CHARS;
    while cut > 0 && !output.is_char_boundary(cut) {
        cut -= 1;
    }
    let slice = &output[..cut];
    let cut = match slice.rfind('\n') {
        Some(nl) if nl > MAX_OUTPUT_CHARS / 2 => nl,
        _ => cut,
    };
    (output[..cut].to_string(), true)
}

mod html_regex {
    use regex::Regex;
    use std::sync::OnceLock;

    fn compile_regex(pattern: &str, label: &str) -> Option<Regex> {
        match Regex::new(pattern) {
            Ok(regex) => Some(regex),
            Err(err) => {
                crate::logging::warn(&format!(
                    "webfetch: failed to compile static regex {label}: {}",
                    err
                ));
                None
            }
        }
    }

    macro_rules! static_regex {
        ($name:ident, $pat:expr_2021) => {
            pub fn $name() -> Option<&'static Regex> {
                static RE: OnceLock<Option<Regex>> = OnceLock::new();
                RE.get_or_init(|| compile_regex($pat, stringify!($name)))
                    .as_ref()
            }
        };
    }

    static_regex!(script, r"(?is)<script[^>]*>.*?</script>");
    static_regex!(style, r"(?is)<style[^>]*>.*?</style>");
    // Match attribute values (which may themselves contain `>`) before falling
    // back to bare `>`-terminated content, so tags carrying JSON payloads such as
    // Parsoid's `data-mw` do not leak their contents into the output.
    static_regex!(
        tag,
        r#"(?s)</?[A-Za-z!/][^\s/>]*(?:\s+[^\s=/>]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*))?)*\s*/?>"#
    );
    static_regex!(whitespace, r"\n\s*\n\s*\n");
    // Runs of empty markdown list items left behind after tag stripping.
    static_regex!(empty_bullets, r"(?m)^[ \t]*-[ \t]*$\n?");

    /// HTML elements whose content is non-prose by specification: navigation,
    /// complementary/tangential content, interactive controls, and embedded
    /// non-text resources. This is deliberately limited to elements whose *spec
    /// definition* excludes primary content, so it generalizes across sites
    /// rather than encoding any single site's markup.
    ///
    /// Notably excludes `<header>`, which commonly wraps the article `<h1>`,
    /// byline, and publication date, and `<footer>`, which can carry
    /// article-level attribution when nested inside `<article>`.
    const CHROME_TAGS: [&str; 10] = [
        "nav", "aside", "form", "noscript", "svg", "iframe", "template", "select", "dialog",
        "canvas",
    ];

    static CHROME: OnceLock<Vec<Regex>> = OnceLock::new();

    pub fn chrome() -> &'static [Regex] {
        CHROME.get_or_init(|| {
            CHROME_TAGS
                .iter()
                .filter_map(|tag| {
                    compile_regex(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}\s*>"), "chrome")
                })
                .collect()
        })
    }
    static_regex!(link, r#"(?i)<a[^>]*href=["']([^"']+)["'][^>]*>([^<]*)</a>"#);
    static_regex!(strong, r"(?i)<(?:strong|b)>([^<]*)</(?:strong|b)>");
    static_regex!(em, r"(?i)<(?:em|i)>([^<]*)</(?:em|i)>");
    static_regex!(code, r"(?i)<code>([^<]*)</code>");
    static_regex!(pre_code, r"(?is)<pre[^>]*><code[^>]*>(.+?)</code></pre>");
    static_regex!(li, r"(?i)<li[^>]*>");
    // HTML comments frequently contain build metadata, conditional markup, and
    // commented-out blocks, none of which are rendered content.
    static_regex!(comment, r"(?s)<!--.*?-->");

    static H_OPEN: OnceLock<Option<[Regex; 6]>> = OnceLock::new();
    static H_CLOSE: OnceLock<Option<[Regex; 6]>> = OnceLock::new();

    pub fn h_open() -> Option<&'static [Regex; 6]> {
        H_OPEN
            .get_or_init(|| {
                let mut compiled = Vec::with_capacity(6);
                for i in 0..6 {
                    let pattern = format!(r"(?i)<h{}[^>]*>", i + 1);
                    compiled.push(compile_regex(&pattern, "heading open")?);
                }
                compiled.try_into().ok()
            })
            .as_ref()
    }

    pub fn h_close() -> Option<&'static [Regex; 6]> {
        H_CLOSE
            .get_or_init(|| {
                let mut compiled = Vec::with_capacity(6);
                for i in 0..6 {
                    let pattern = format!(r"(?i)</h{}>", i + 1);
                    compiled.push(compile_regex(&pattern, "heading close")?);
                }
                compiled.try_into().ok()
            })
            .as_ref()
    }
}

fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    let (Some(script), Some(style), Some(tag), Some(whitespace)) = (
        html_regex::script(),
        html_regex::style(),
        html_regex::tag(),
        html_regex::whitespace(),
    ) else {
        return html.trim().to_string();
    };

    text = script.replace_all(&text, "").to_string();
    text = style.replace_all(&text, "").to_string();
    if let Some(comment) = html_regex::comment() {
        text = comment.replace_all(&text, "").to_string();
    }
    for re in html_regex::chrome() {
        text = re.replace_all(&text, "").to_string();
    }

    text = text.replace("<br>", "\n");
    text = text.replace("<br/>", "\n");
    text = text.replace("<br />", "\n");
    text = text.replace("</p>", "\n\n");
    text = text.replace("</div>", "\n");
    text = text.replace("</li>", "\n");
    text = text.replace("</tr>", "\n");

    text = tag.replace_all(&text, "").to_string();

    text = text.replace("&nbsp;", " ");
    text = text.replace("&lt;", "<");
    text = text.replace("&gt;", ">");
    text = text.replace("&amp;", "&");
    text = text.replace("&quot;", "\"");
    text = text.replace("&#39;", "'");

    text = whitespace.replace_all(&text, "\n\n").to_string();

    text.trim().to_string()
}

/// Render one anchor as markdown, dropping targets that cost more context than
/// they convey.
///
/// Three general cases, none specific to any site:
/// - Empty anchor text means the link is a bare icon or control. Emitting
///   `[](url)` conveys nothing, so the whole link is dropped.
/// - Overlong targets are encoded payloads rather than addresses; the anchor
///   text is kept and the target dropped.
/// - Pure in-page fragments (`#foo`) are navigation aids with no destination
///   content, so the text is kept and the target dropped.
fn render_link(href: &str, text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let href = href.trim();
    if href.is_empty() || href.starts_with('#') || href.chars().count() > MAX_URL_CHARS {
        return text.to_string();
    }
    format!("[{text}]({href})")
}

fn html_to_markdown(html: &str) -> String {
    let mut md = html.to_string();

    let (
        Some(script),
        Some(style),
        Some(link),
        Some(strong),
        Some(em),
        Some(code),
        Some(pre_code),
        Some(li),
        Some(tag),
        Some(whitespace),
    ) = (
        html_regex::script(),
        html_regex::style(),
        html_regex::link(),
        html_regex::strong(),
        html_regex::em(),
        html_regex::code(),
        html_regex::pre_code(),
        html_regex::li(),
        html_regex::tag(),
        html_regex::whitespace(),
    )
    else {
        return html.trim().to_string();
    };

    md = script.replace_all(&md, "").to_string();
    md = style.replace_all(&md, "").to_string();
    if let Some(comment) = html_regex::comment() {
        md = comment.replace_all(&md, "").to_string();
    }
    for re in html_regex::chrome() {
        md = re.replace_all(&md, "").to_string();
    }

    if let (Some(h_open), Some(h_close)) = (html_regex::h_open(), html_regex::h_close()) {
        for i in 0..6 {
            let prefix = "#".repeat(i + 1);
            md = h_open[i]
                .replace_all(&md, &format!("\n{} ", prefix))
                .to_string();
            md = h_close[i].replace_all(&md, "\n").to_string();
        }
    }

    md = link
        .replace_all(&md, |caps: &regex::Captures<'_>| {
            render_link(
                caps.get(1).map_or("", |m| m.as_str()),
                caps.get(2).map_or("", |m| m.as_str()),
            )
        })
        .to_string();
    md = strong.replace_all(&md, "**$1**").to_string();
    md = em.replace_all(&md, "*$1*").to_string();
    md = code.replace_all(&md, "`$1`").to_string();
    md = pre_code.replace_all(&md, "\n```\n$1\n```\n").to_string();
    md = li.replace_all(&md, "\n- ").to_string();

    md = md.replace("<br>", "\n");
    md = md.replace("<br/>", "\n");
    md = md.replace("<br />", "\n");
    md = md.replace("</p>", "\n\n");

    md = tag.replace_all(&md, "").to_string();

    md = md.replace("&nbsp;", " ");
    md = md.replace("&lt;", "<");
    md = md.replace("&gt;", ">");
    md = md.replace("&amp;", "&");
    md = md.replace("&quot;", "\"");
    md = md.replace("&#39;", "'");

    if let Some(empty_bullets) = html_regex::empty_bullets() {
        md = empty_bullets.replace_all(&md, "").to_string();
    }
    md = whitespace.replace_all(&md, "\n\n").to_string();

    md.trim().to_string()
}

#[cfg(test)]
#[path = "webfetch_evidence_tests.rs"]
mod evidence_tests;

#[cfg(test)]
#[path = "webfetch_corpus_tests.rs"]
mod corpus_tests;

#[cfg(test)]
#[path = "webfetch_tests.rs"]
mod tests;
