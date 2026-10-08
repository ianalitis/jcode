//! Native Cursor Agent transport implementing `agent.v1.AgentService/Run`.
//!
//! Cursor decommissioned the old `api2.cursor.sh/aiserver.v1.ChatService/
//! StreamUnifiedChatWithTools` endpoint for API-key / CLI tokens (it now returns
//! `resource_exhausted` "Update Required" / `actionRequired: payment`). The
//! current, working transport used by the `cursor-agent` CLI is a *paced,
//! bidirectional* Connect-over-HTTP/2 stream against
//! `agentn.global.api5.cursor.sh/agent.v1.AgentService/Run`.
//!
//! Wire format (reverse-engineered by MITM-capturing the real `cursor-agent`):
//!
//! * Connect streaming framing: each message is `[1 flag byte][4-byte BE len]
//!   [payload]`. Flag `0x01` = payload gzip-compressed, `0x02` = end-of-stream
//!   trailer (JSON, `{}` on success or `{"error":...}`).
//! * frame 0 = field 1 (`RunRequest`: prompt, model, MCP tools, catalog),
//! * subsequent context, tool-result, and key/value messages are exchanged on
//!   the same open bidirectional stream as requested by the server.
//! * The client keeps the request stream **open** while reading the response,
//!   emitting periodic `f7:''` heartbeats (~5s) and only half-closing after the
//!   server completes.
//!
//! Response text arrives as `f1.f1.f1` string chunks (assistant answer) and
//! `f1.f4.f1` chunks (reasoning). A trailing flag-`0x02` frame closes the turn.

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
#[cfg(test)]
use bytes::Bytes;
use flate2::read::GzDecoder;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::Instant;
use uuid::Uuid;

use jcode_message_types::StreamEvent;

use crate::wire::{field_ld, field_str, field_varint, iter_fields};

#[path = "agent_transport_tasks.rs"]
mod tasks;

const AGENT_HOST: &str = "agentn.global.api5.cursor.sh";
const AGENT_PATH: &str = "/agent.v1.AgentService/Run";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// Fallback client build advertised to Cursor's agent service when no local
/// `cursor-agent` installation is available. Prefer the installed build below:
/// Cursor gates AgentService features on this value and rejects stale clients.
const CLI_CLIENT_VERSION_DEFAULT: &str = "cli-2026.08.25-3e8eec8";

fn stream_debug(message: impl std::fmt::Display) {
    if std::env::var_os("CURSOR_STREAM_DEBUG").is_some() {
        eprintln!("cursor-stream: {message}");
    }
}

fn valid_cli_build_id(value: &str) -> bool {
    let Some((date, commit)) = value.split_once('-') else {
        return false;
    };
    let mut date_parts = date.split('.');
    matches!(
        (date_parts.next(), date_parts.next(), date_parts.next(), date_parts.next()),
        (Some(year), Some(month), Some(day), None)
            if year.len() == 4
                && month.len() == 2
                && day.len() == 2
                && year.chars().all(|ch| ch.is_ascii_digit())
                && month.chars().all(|ch| ch.is_ascii_digit())
                && day.chars().all(|ch| ch.is_ascii_digit())
    ) && !commit.is_empty()
        && commit.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn installed_cli_build_id() -> Option<String> {
    let shim = jcode_base::storage::user_home_path(".local/bin/cursor-agent").ok()?;
    let resolved = std::fs::canonicalize(shim).ok()?;
    let mut components = resolved.components();
    while let Some(component) = components.next() {
        if component.as_os_str() == "versions" {
            let version = components.next()?.as_os_str().to_str()?.to_string();
            return valid_cli_build_id(&version).then_some(version);
        }
    }
    None
}

pub(crate) fn cli_client_version() -> String {
    if let Some(version) = std::env::var("JCODE_CURSOR_CLI_VERSION")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
    {
        return version;
    }
    installed_cli_build_id()
        .map(|version| format!("cli-{version}"))
        .unwrap_or_else(|| CLI_CLIENT_VERSION_DEFAULT.to_string())
}

/// Extract a bare host from a value that may be a full URL, a host with a
/// scheme, or a host with trailing path/port noise.
///
/// The result is used directly as a DNS name (`TcpStream::connect`) and as the
/// TLS `ServerName`, so it has to be a bare host: a leftover `:443` suffix or a
/// trailing path makes both of those fail.
fn normalize_agent_host(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let raw = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))
        .unwrap_or(raw);
    let host = raw.split('/').next().unwrap_or_default().trim();
    // Drop any explicit port. Connections are always made on 443, so a port in
    // the cached or overridden value is noise that would otherwise land inside
    // the DNS name and the TLS SNI value.
    let host = host.split(':').next().unwrap_or(host).trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

/// Regional agent host cached by the official `cursor-agent` CLI.
///
/// `cursor-agent` bootstraps `GetServerConfig` and caches the endpoint its team
/// is actually routed to. Teams pinned to a region reject the `global` host with
/// "This region is not yet available for your team" (issue #637), so reuse the
/// CLI's cached value when it is present.
fn agent_host_from_cursor_cli_config() -> Option<String> {
    let path = match jcode_base::storage::user_home_path(".cursor/cli-config.json") {
        Ok(path) => path,
        Err(err) => {
            jcode_base::logging::warn(&format!(
                "Cursor: cannot locate ~/.cursor/cli-config.json ({err}); \
                 falling back to the global agent host"
            ));
            return None;
        }
    };
    // A missing file is the normal case for users who never ran `cursor-agent`,
    // so that is not worth warning about. Anything else (unreadable, malformed,
    // unexpected shape) is worth surfacing: silently discarding it is what made
    // issue #637 hard to diagnose in the first place.
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            jcode_base::logging::warn(&format!(
                "Cursor: cannot read {} ({err}); falling back to the global agent host",
                path.display()
            ));
            return None;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            jcode_base::logging::warn(&format!(
                "Cursor: {} is not valid JSON ({err}); falling back to the global agent host",
                path.display()
            ));
            return None;
        }
    };
    let Some(url_config) = value
        .get("serverConfigCache")
        .and_then(|cache| cache.get("agentUrlConfig"))
    else {
        jcode_base::logging::warn(&format!(
            "Cursor: {} has no serverConfigCache.agentUrlConfig; \
             falling back to the global agent host. Running `cursor-agent status` \
             usually repopulates it.",
            path.display()
        ));
        return None;
    };
    let candidate = url_config
        .get("agentnUrl")
        .and_then(|v| v.as_str())
        .or_else(|| url_config.get("agentUrl").and_then(|v| v.as_str()));
    let Some(host) = candidate.and_then(normalize_agent_host) else {
        jcode_base::logging::warn(&format!(
            "Cursor: {} has no usable agentnUrl/agentUrl host (found {candidate:?}); \
             falling back to the global agent host",
            path.display()
        ));
        return None;
    };
    jcode_base::logging::info(&format!(
        "Cursor: using regional agent host {host} from {}",
        path.display()
    ));
    Some(host)
}

/// Resolve the Cursor agent host, in precedence order:
/// 1. `JCODE_CURSOR_AGENT_HOST` / `CURSOR_AGENT_HOST` (explicit override)
/// 2. `~/.cursor/cli-config.json` regional endpoint cached by `cursor-agent`
/// 3. the `global` host, as a last-resort fallback
pub(crate) fn agent_host() -> String {
    for var in ["JCODE_CURSOR_AGENT_HOST", "CURSOR_AGENT_HOST"] {
        if let Ok(raw) = std::env::var(var) {
            // Normalize overrides too. These get copied straight out of
            // `cli-config.json` or a browser, so `https://host/path` and
            // `host:443` are both likely spellings and both have to work.
            if let Some(host) = normalize_agent_host(&raw) {
                return host;
            }
        }
    }
    agent_host_from_cursor_cli_config().unwrap_or_else(|| AGENT_HOST.to_string())
}

// --------------------------------------------------------------------------
// Protobuf + Connect framing helpers
// --------------------------------------------------------------------------

pub(crate) fn routed_prompt(
    prompt: &str,
    system: &str,
    tools: &[jcode_message_types::ToolDefinition],
) -> String {
    let mut sections = Vec::with_capacity(3);
    if !system.trim().is_empty() {
        sections.push(system.trim().to_string());
    }
    if !tools.is_empty() {
        let aliases = crate::wire::mcp_wire_aliases(tools);
        let names = tools
            .iter()
            .filter_map(|tool| aliases.get(&tool.name))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        sections.push(format!(
            "TOOL ROUTING CONSTRAINT (highest priority): You have no built-in tools. \
             The only executable tools are these MCP tools: {names}. \
             Call the matching cc_* MCP tool directly and wait for its result. \
             Never call Read, Write, Edit, Shell, Bash, Glob, Grep, or another built-in tool."
        ));
    }
    sections.push(prompt.to_string());
    sections.join("\n\n")
}

/// Wrap a protobuf message payload in a Connect data frame using the same
/// threshold as connect-es. Large tool results and context messages must carry
/// the gzip flag when the request advertises gzip content encoding.
fn connect_frame(payload: &[u8]) -> Vec<u8> {
    crate::wire::connect_frame(payload)
}

/// Encode the model descriptor used by the current Cursor AgentService.
///
/// The service expects the selected model in both the requested-model field and
/// the catalog field. The `fast` value is represented as the string-valued
/// metadata entry used by the official CLI, rather than as the legacy boolean
/// field that older bridges emitted.
fn encode_model_meta(name: &str, fast: bool) -> Vec<u8> {
    let mut out = field_str(1, name);
    let mut fast_entry = field_str(1, "fast");
    fast_entry.extend(field_str(2, if fast { "true" } else { "false" }));
    out.extend(field_ld(3, &fast_entry));
    out
}

/// A Cursor model selection split into the base id the AgentService accepts
/// and the separately-encoded `fast` parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedModel {
    pub id: String,
    pub fast: bool,
}

/// Resolve a jcode/catalog model id into Cursor's base id plus `fast` param.
///
/// Cursor identifies models by *base id* (`composer-2.5`, `grok-4.6`,
/// `claude-opus-5`) and carries speed/effort/thinking as separate params.
/// Older catalogs, persisted sessions, and usage labels use composite strings
/// (`cursor-grok-4.6-high-fast`, `claude-opus-5-thinking-high`,
/// `composer-2.5-fast`) that the backend rejects with
/// `ERROR_BAD_MODEL_NAME: Unknown model ID` (issue #1226).
///
/// `fast_override` is the user's `/fast` choice. `None` keeps the model's own
/// default: an explicit `-fast` suffix wins, and Composer defaults to Fast,
/// matching Cursor's API (`composer-2.5` with no params runs the Fast variant).
pub(crate) fn resolve_model_id(model: &str, fast_override: Option<bool>) -> ResolvedModel {
    resolve_model_id_with_catalog(model, fast_override, &[])
}

/// Split a catalog-style id into (base, has_fast_suffix, has_effort_suffix).
fn split_model_suffixes(model: &str) -> (&str, bool, bool) {
    let mut name = model;
    let mut fast = false;
    let mut effort = false;
    const SUFFIXES: &[&str] = &[
        "-fast",
        "-xhigh",
        "-high",
        "-medium",
        "-low",
        "-minimal",
        "-max",
        "-thinking",
    ];
    loop {
        let mut stripped_any = false;
        for suffix in SUFFIXES {
            if let Some(stripped) = name.strip_suffix(suffix)
                && !stripped.is_empty()
            {
                if *suffix == "-fast" {
                    fast = true;
                } else {
                    effort = true;
                }
                name = stripped;
                stripped_any = true;
                break;
            }
        }
        if !stripped_any {
            break;
        }
    }
    (name, fast, effort)
}

/// Models whose bare base id Cursor rejects, mapped to the catalog entry
/// Cursor itself labels with the plain display name. Used only when the live
/// catalog is unavailable. Verified against AgentService on 2026-09-29:
/// `grok-4.6` -> `ERROR_BAD_MODEL_NAME`, `cursor-grok-4.6-high` ("Grok 4.6")
/// is accepted.
const OFFLINE_CATALOG_ALIASES: &[(&str, &str)] = &[("grok-4.6", "cursor-grok-4.6-high")];

/// Resolve a jcode model id against Cursor's live `GetUsableModels` catalog.
///
/// Cursor's AgentService accepts the exact catalog ids it advertises
/// (`cursor-grok-4.6-high`, `claude-opus-5-5-medium`, `grok-4.7-high-fast`)
/// but only *some* bare base ids (`grok-4.7`, `composer-2.5` work, while
/// `grok-4.6`, `claude-opus-5-5` are rejected with `ERROR_BAD_MODEL_NAME`).
/// So an id that is in the catalog is sent verbatim, and a base id is mapped
/// onto the catalog variant that matches it. Only when there is no catalog
/// entry at all do we fall back to stripping suffixes.
pub(crate) fn resolve_model_id_with_catalog(
    model: &str,
    fast_override: Option<bool>,
    catalog: &[String],
) -> ResolvedModel {
    let mut name = model.trim();
    name = name.strip_prefix("cursor:").unwrap_or(name);
    let in_catalog = |id: &str| catalog.iter().any(|entry| entry == id);

    // 1. Exact catalog id: send verbatim, honoring /fast by switching to the
    //    sibling `-fast` variant when one exists.
    if in_catalog(name) {
        let (_, suffix_fast, _) = split_model_suffixes(name);
        let mut id = name.to_string();
        let mut fast = suffix_fast;
        match fast_override {
            Some(true) if !suffix_fast && in_catalog(&format!("{name}-fast")) => {
                id = format!("{name}-fast");
                fast = true;
            }
            Some(false) if suffix_fast => {
                let slow = name.strip_suffix("-fast").unwrap_or(name);
                if in_catalog(slow) {
                    id = slow.to_string();
                    fast = false;
                }
            }
            _ => {}
        }
        if name.starts_with("composer-") && fast_override.is_none() {
            fast = true;
        }
        return ResolvedModel {
            id,
            fast: fast_override.unwrap_or(fast),
        };
    }

    // 2. Base or legacy composite id: map onto a catalog variant with the same
    //    base. Prefer an entry without an effort suffix, then `-high` (what
    //    Cursor labels as the plain model name), then anything else.
    let stripped_prefix = name.strip_prefix("cursor-").unwrap_or(name);
    let (base, suffix_fast, _) = split_model_suffixes(stripped_prefix);
    let want_fast = fast_override.unwrap_or(suffix_fast);
    if !catalog.is_empty() {
        let mut best: Option<(u8, &String)> = None;
        for entry in catalog {
            let entry_unprefixed = entry.strip_prefix("cursor-").unwrap_or(entry);
            let (entry_base, entry_fast, entry_effort) = split_model_suffixes(entry_unprefixed);
            if entry_base != base {
                continue;
            }
            let effort_rank = if !entry_effort {
                0
            } else if entry_unprefixed.contains("-high") && !entry_unprefixed.contains("-xhigh") {
                1
            } else {
                2
            };
            let rank = effort_rank * 2 + u8::from(entry_fast != want_fast);
            if best.is_none_or(|(best_rank, _)| rank < best_rank) {
                best = Some((rank, entry));
            }
        }
        if let Some((_, entry)) = best
            && entry != base
        {
            let (_, entry_fast, _) = split_model_suffixes(entry);
            return ResolvedModel {
                id: entry.clone(),
                fast: fast_override.unwrap_or(entry_fast),
            };
        }
    }

    // 3. No catalog match: known offline aliases, then the base id.
    if let Some((_, alias)) = OFFLINE_CATALOG_ALIASES
        .iter()
        .find(|(bare, _)| *bare == base)
    {
        return ResolvedModel {
            id: if want_fast {
                format!("{alias}-fast")
            } else {
                (*alias).to_string()
            },
            fast: want_fast,
        };
    }
    let name = base;
    let default_fast = suffix_fast || name.starts_with("composer-");
    ResolvedModel {
        id: name.to_string(),
        fast: fast_override.unwrap_or(default_fast),
    }
}

/// Build the request frames for a single-shot prompt turn.
///
/// Returns the initial Connect frame for the streamed `RunInput`.
///
/// The current connect-es client sends only `AgentRunRequest` up front. The
/// server requests environment context and other execution messages on the
/// same bidirectional stream later. Sending the old captured context and marker
/// frames proactively is tolerated for plain chat but causes current Cursor
/// tool turns to remain in an unacknowledged state.
fn build_run_frames(
    prompt: &str,
    model: &ResolvedModel,
    _cwd: &str,
    tools: &[jcode_message_types::ToolDefinition],
    request_id: &str,
) -> Vec<Vec<u8>> {
    let fast = model.fast;
    let model = model.id.as_str();
    let conv = Uuid::new_v4().to_string();
    let msg = Uuid::new_v4().to_string();

    // frame 0: field 1 = RunRequest
    // messages: f2 { f1 { f1 { f1:prompt, f2:msg_id, f3:'', f4:1 } } }
    let mut inner = field_str(1, prompt);
    inner.extend(field_str(2, &msg));
    inner.extend(field_str(3, ""));
    inner.extend(field_varint(4, 1));
    let messages = field_ld(2, &field_ld(1, &field_ld(1, &inner)));

    let mut req = field_str(1, "");
    req.extend(messages);
    if !tools.is_empty() {
        if let Ok(mcp_tools_bytes) = crate::wire::encode_mcp_tools(tools) {
            req.extend(field_ld(4, &mcp_tools_bytes));
        }
    } else {
        req.extend(field_str(4, ""));
    }
    req.extend(field_str(5, &conv));
    // AgentRunRequest.f9 is a model descriptor. Omitting the `fast` metadata can
    // leave AgentService at HTTP 200 with only heartbeats instead of starting
    // the turn.
    let selected_model = encode_model_meta(model, fast);
    // ModelDetails (f3) carries the canonical/display model identity used by the
    // current AgentService model resolver. The server accepts the request without
    // it, but may leave the stream pending while resolving the model.
    let mut model_details = field_str(1, model);
    model_details.extend(field_str(3, model));
    model_details.extend(field_str(4, model));
    req.extend(field_ld(3, &model_details));
    req.extend(field_ld(9, &selected_model));
    req.extend(field_varint(12, 0));
    // The catalog contains the default entry followed by the selected model.
    // Both entries use the same `{name, fast}` metadata shape as cursor-agent.
    req.extend(field_ld(14, &encode_model_meta("default", false)));
    req.extend(field_ld(14, &encode_model_meta(model, fast)));
    req.extend(field_str(16, &conv));
    req.extend(field_str(25, request_id));
    let frame0 = connect_frame(&field_ld(1, &req));

    vec![frame0]
}

/// A single `f7:''` heartbeat frame.
fn heartbeat_frame() -> Vec<u8> {
    connect_frame(&field_ld(7, &[]))
}

// --------------------------------------------------------------------------
// Response parsing
// --------------------------------------------------------------------------

/// Incrementally decode Connect frames from a byte buffer, returning
/// `(flag, payload, consumed)` for the next complete frame or `Ok(None)`.
/// Malformed compressed frames are rejected instead of being passed to the
/// protobuf parser as if they were valid uncompressed data.
fn next_frame(buf: &[u8]) -> Result<Option<(u8, Vec<u8>, usize)>> {
    if buf.len() < 5 {
        return Ok(None);
    }
    let flag = buf[0];
    let len = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;
    let end = 5 + len;
    if buf.len() < end {
        return Ok(None);
    }
    let mut payload = buf[5..end].to_vec();
    if flag & 0x01 != 0 {
        // gzip-compressed payload
        let mut decoded = Vec::new();
        GzDecoder::new(&payload[..])
            .read_to_end(&mut decoded)
            .context("Invalid gzip payload in Cursor agent stream")?;
        payload = decoded;
    }
    Ok(Some((flag, payload, end)))
}

/// Extract the assistant answer text delta from one response message payload.
///
/// The assistant-answer chunk shape is `f1 { f1 { f1: <str> } }`. We ignore
/// reasoning (`f1.f4`) so the emitted stream matches plain chat text.
fn extract_answer_text(payload: &[u8]) -> Option<String> {
    for f1 in iter_fields(payload) {
        if f1.field != 1 || f1.wire != 2 {
            continue;
        }
        for f1_1 in iter_fields(f1.data) {
            if f1_1.field != 1 || f1_1.wire != 2 {
                continue;
            }
            for leaf in iter_fields(f1_1.data) {
                if leaf.field == 1
                    && leaf.wire == 2
                    && let Ok(s) = std::str::from_utf8(leaf.data)
                    && !s.is_empty()
                {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

/// Extract reasoning delta (thinking text) from one response message payload.
/// The reasoning chunk shape is `f1 { f4 { f1: <str> } }`.
pub(crate) fn extract_thinking_text(payload: &[u8]) -> Option<String> {
    for f1 in iter_fields(payload) {
        if f1.field != 1 || f1.wire != 2 {
            continue;
        }
        for f4 in iter_fields(f1.data) {
            if f4.field != 4 || f4.wire != 2 {
                continue;
            }
            for leaf in iter_fields(f4.data) {
                if leaf.field == 1
                    && leaf.wire == 2
                    && let Ok(s) = std::str::from_utf8(leaf.data)
                    && !s.is_empty()
                {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn tls_config() -> Arc<tokio_rustls::rustls::ClientConfig> {
    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec()];
    Arc::new(config)
}

pub(crate) fn parse_tool_request_id(request_id: &str) -> (u32, &str) {
    let mut parts = request_id.splitn(3, ':');
    let _stream_uuid = parts.next();
    let id = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(1);
    (id, parts.next().unwrap_or(""))
}

pub(crate) fn resolve_native_tool_name(
    advertised_name: &str,
    auxiliary_name: &str,
    tools: &[jcode_message_types::ToolDefinition],
) -> String {
    let candidates = [auxiliary_name, advertised_name];
    let aliases = crate::wire::mcp_wire_aliases(tools);
    for tool in tools {
        let wire_name = aliases.get(&tool.name).map(String::as_str).unwrap_or("");
        let safe_bare_name = crate::wire::mcp_bare_name(wire_name);
        if candidates.iter().any(|candidate| {
            !candidate.is_empty()
                && (*candidate == tool.name
                    || *candidate == wire_name
                    || *candidate == safe_bare_name)
        }) {
            return tool.name.clone();
        }
    }

    if auxiliary_name.is_empty() {
        crate::wire::mcp_bare_name(advertised_name).to_string()
    } else {
        auxiliary_name.to_string()
    }
}

/// Deliver one framed reply, reporting closure without including its payload.
pub(crate) async fn send_agent_reply(
    outbound: &mpsc::Sender<Vec<u8>>,
    payload: &[u8],
) -> Result<()> {
    outbound
        .send(crate::wire::connect_frame(payload))
        .await
        .map_err(|_| anyhow!("Cursor agent outbound stream closed before reply"))
}

/// Run one Cursor agent turn and forward assistant text as [`StreamEvent`]s.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_turn(
    access_token: &str,
    prompt: &str,
    model: &str,
    fast_override: Option<bool>,
    model_catalog: &[String],
    logical_session_id: Option<&str>,
    stream_uuid: &str,
    tools: &[jcode_message_types::ToolDefinition],
    system: &str,
    tool_result_rx: &mut mpsc::Receiver<jcode_provider_core::NativeToolResult>,
    tx: mpsc::Sender<Result<StreamEvent>>,
) -> Result<()> {
    use tokio_rustls::TlsConnector;

    let host = agent_host();
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "/".to_string());

    let _ = tx
        .send(Ok(StreamEvent::ConnectionType {
            connection: "native http2 (agent)".to_string(),
        }))
        .await;

    // Establish TLS + HTTP/2.
    let tcp = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio::net::TcpStream::connect((host.as_str(), 443)),
    )
    .await
    .with_context(|| format!("Timed out connecting to {host}:443"))?
    .with_context(|| format!("Failed to connect to {host}:443"))?;
    tcp.set_nodelay(true).ok();
    let connector = TlsConnector::from(tls_config());
    let server_name = tokio_rustls::rustls::pki_types::ServerName::try_from(host.clone())
        .context("Invalid Cursor agent host name")?;
    let tls = tokio::time::timeout(CONNECT_TIMEOUT, connector.connect(server_name, tcp))
        .await
        .context("Timed out during TLS handshake with Cursor agent host")?
        .context("TLS handshake with Cursor agent host failed")?;

    run_agent_turn_on_io(
        tls,
        &host,
        &cwd,
        access_token,
        prompt,
        model,
        fast_override,
        model_catalog,
        logical_session_id,
        stream_uuid,
        tools,
        system,
        tool_result_rx,
        tx,
        #[cfg(test)]
        tests::TaskProbes::default(),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_agent_turn_on_io<IO>(
    io: IO,
    host: &str,
    cwd: &str,
    access_token: &str,
    prompt: &str,
    model: &str,
    fast_override: Option<bool>,
    model_catalog: &[String],
    logical_session_id: Option<&str>,
    stream_uuid: &str,
    tools: &[jcode_message_types::ToolDefinition],
    system: &str,
    tool_result_rx: &mut mpsc::Receiver<jcode_provider_core::NativeToolResult>,
    tx: mpsc::Sender<Result<StreamEvent>>,
    #[cfg(test)] probes: tests::TaskProbes,
) -> Result<()>
where
    IO: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use h2::client;
    use http::{Method, Request};

    let (h2, connection) = tokio::time::timeout(CONNECT_TIMEOUT, client::handshake(io))
        .await
        .context("Timed out during HTTP/2 handshake with Cursor agent host")?
        .context("HTTP/2 handshake with Cursor agent host failed")?;
    // Dropping this owner requests abort of both real children, not a join.
    let mut tasks = JoinSet::new();
    #[cfg(test)]
    let driver_probe = probes.guard(0);
    tasks.spawn(async move {
        #[cfg(test)]
        let _probe = driver_probe.started();
        tasks::Completion::Driver(connection.await.context("Cursor HTTP/2 driver failed"))
    });
    let setup = async {
        let mut h2 = tokio::time::timeout(CONNECT_TIMEOUT, h2.ready())
            .await
            .context("Timed out waiting for HTTP/2 connection readiness")?
            .context("HTTP/2 connection not ready")?;

        let request_id = Uuid::new_v4().to_string();
        let session_id = logical_session_id
            .map(|id| Uuid::new_v5(&Uuid::NAMESPACE_DNS, id.as_bytes()))
            .unwrap_or_else(Uuid::new_v4)
            .to_string();
        let traceparent = format!(
            "00-{}-{}-01",
            Uuid::new_v4().simple(),
            &Uuid::new_v4().simple().to_string()[..16]
        );
        let blob_encryption_key = Uuid::new_v4().simple().to_string();
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("https://{host}{AGENT_PATH}"))
            .header("authorization", format!("Bearer {access_token}"))
            .header("connect-accept-encoding", "gzip,br")
            .header("connect-content-encoding", "gzip")
            .header("connect-protocol-version", "1")
            .header("te", "trailers")
            .header("content-type", "application/connect+proto")
            .header("backend-traceparent", &traceparent)
            .header("traceparent", &traceparent)
            .header("user-agent", "connect-es/1.6.1")
            .header("x-blob-encryption-key", blob_encryption_key)
            .header("x-cursor-client-type", "cli")
            .header("x-cursor-client-version", cli_client_version())
            .header("x-ghost-mode", "true")
            .header("x-request-id", &request_id)
            .header("x-original-request-id", &request_id)
            .header("x-session-id", &session_id)
            .body(())
            .context("Failed to build Cursor agent request")?;

        let (response, send_stream) = h2
            .send_request(request, false)
            .context("Failed to send Cursor agent request headers")?;
        Ok::<_, anyhow::Error>((response, send_stream, session_id, request_id))
    }
    .await;
    let (response, send_stream, session_id, request_id) = match setup {
        Ok(setup) => setup,
        Err(error) => {
            if tasks::abort_and_drain(&mut tasks).await.is_err() {
                stream_debug("Cursor cleanup failed after request setup error");
            }
            return Err(error);
        }
    };

    let _ = tx.send(Ok(StreamEvent::SessionId(session_id))).await;

    let (outbound_tx, outbound_rx) = mpsc::channel::<Vec<u8>>(32);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    let prompt = routed_prompt(prompt, system, tools);
    let resolved_model = resolve_model_id_with_catalog(model, fast_override, model_catalog);
    stream_debug(format_args!(
        "model requested={model} resolved={} fast={}",
        resolved_model.id, resolved_model.fast
    ));
    let frames = build_run_frames(&prompt, &resolved_model, cwd, tools, &request_id);
    #[cfg(test)]
    let sender_probe = probes.guard(1);
    tasks.spawn(async move {
        #[cfg(test)]
        let _probe = sender_probe.started();
        tasks::Completion::Sender(tasks::send(send_stream, frames, outbound_rx, stop_rx).await)
    });

    // Receiver: read response body frames and forward assistant text.
    let headers = tokio::select! {
        biased;
        response = tokio::time::timeout(CONNECT_TIMEOUT, response) => {
            response.context("Timed out waiting for Cursor agent response headers")
                .and_then(|response| response.context("Cursor agent request failed before response headers"))
        }
        Some(completion) = tasks.join_next() => Err(tasks::premature(completion)),
    };
    let response = match headers {
        Ok(response) => response,
        Err(error) => {
            if tasks::abort_and_drain(&mut tasks).await.is_err() {
                stream_debug("Cursor cleanup failed after response header error");
            }
            return Err(error);
        }
    };
    let status = response.status();
    stream_debug(format_args!("response status={status}"));
    let mut body = response.into_body();
    let mut pending: Vec<u8> = Vec::new();
    let mut error_message: Option<String> = None;
    let mut got_text = false;
    let mut in_thinking = false;
    let mut active_tool_calls: usize = 0;
    let mut blob_store: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let mut terminal_frame = false;
    let mut idle_finished = false;
    let mut stream_error: Option<anyhow::Error> = None;

    // Idle budgets tolerate reasoning before output and mid-answer pauses.
    // A short post-text budget previously truncated reasoning-model answers.
    // Tool execution has its own longer budget below.
    let first_byte_timeout = Duration::from_secs(120);
    let idle_timeout = Duration::from_secs(30);
    let tool_exec_timeout = Duration::from_secs(300);
    // Heartbeat, KV, and acknowledgement frames are transport activity, not
    // model progress. Keep a phase deadline so a server that only emits those
    // frames cannot keep a user-facing turn alive forever.
    let mut phase_started = Instant::now();
    let mut last_meaningful = phase_started;

    'read: loop {
        let budget = if active_tool_calls > 0 {
            tool_exec_timeout.saturating_sub(last_meaningful.elapsed())
        } else if got_text {
            idle_timeout.saturating_sub(last_meaningful.elapsed())
        } else {
            first_byte_timeout.saturating_sub(phase_started.elapsed())
        };
        let next = tokio::select! {
            biased;
            res = tokio::time::timeout(budget, body.data()) => {
                match res {
                    Ok(Some(Ok(chunk))) => {
                        chunk
                    }
                    Ok(Some(Err(error))) => {
                        stream_error = Some(error.into());
                        break 'read;
                    }
                    Ok(None) => {
                        if !terminal_frame {
                            stream_error = Some(anyhow!(
                                "Cursor agent stream ended before an authoritative terminal frame"
                            ));
                        }
                        break 'read;
                    }
                    Err(_) => {
                        if got_text && active_tool_calls == 0 {
                            // The answer was delivered and the server just
                            // never sent its trailer. Finish the turn with
                            // what we have instead of discarding it.
                            stream_debug(format_args!(
                                "idle after text; finishing turn without trailer"
                            ));
                            idle_finished = true;
                        } else {
                            stream_error = Some(anyhow!(
                                "Cursor agent stream timed out waiting for meaningful progress"
                            ));
                        }
                        break 'read;
                    }
                }
            }
            Some(completion) = tasks.join_next() => {
                stream_error = Some(tasks::premature(completion));
                break 'read;
            }
            Some(tool_res) = tool_result_rx.recv() => {
                let (id_u32, exec_id) = parse_tool_request_id(&tool_res.request_id);
                let raw_content = if tool_res.is_error {
                    tool_res.result.error.unwrap_or_default()
                } else {
                    tool_res.result.output.unwrap_or_default()
                };
                let redacted = jcode_base::message::redact_secrets(&raw_content);
                let mcp_res_bytes = crate::wire::encode_mcp_success_result(
                    id_u32,
                    exec_id,
                    &redacted,
                    tool_res.is_error,
                );
                let agent_client_bytes = crate::wire::encode_agent_client_exec_message(&mcp_res_bytes);
                if let Err(error) = send_agent_reply(&outbound_tx, &agent_client_bytes).await {
                    stream_error = Some(error);
                    break 'read;
                }
                active_tool_calls = active_tool_calls.saturating_sub(1);
                phase_started = Instant::now();
                last_meaningful = phase_started;
                got_text = false;
                continue 'read;
            }
        };
        let chunk = next;
        let _ = body.flow_control().release_capacity(chunk.len());
        stream_debug(format_args!("response chunk={}", chunk.len()));
        pending.extend_from_slice(&chunk);
        loop {
            let frame = match next_frame(&pending) {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(error) => {
                    stream_error = Some(error);
                    break 'read;
                }
            };
            let (flag, payload, consumed) = frame;
            stream_debug(format_args!(
                "frame flag={flag:#04x} payload_len={}",
                payload.len()
            ));
            pending.drain(..consumed);
            if flag & 0x02 != 0 {
                // end-of-stream trailer (JSON). Detect errors, then finish.
                if let Ok(text) = std::str::from_utf8(&payload)
                    && let Ok(json) = serde_json::from_str::<serde_json::Value>(text)
                    && let Some(err) = json.get("error")
                {
                    error_message = Some(err.to_string());
                }
                terminal_frame = true;
                break 'read;
            }
            if let Some(text) = extract_answer_text(&payload) {
                if in_thinking {
                    let _ = tx.send(Ok(StreamEvent::ThinkingEnd)).await;
                    in_thinking = false;
                }
                got_text = true;
                last_meaningful = Instant::now();
                if tx.send(Ok(StreamEvent::TextDelta(text))).await.is_err() {
                    break 'read;
                }
                continue;
            }
            if let Some(thinking) = extract_thinking_text(&payload) {
                if !in_thinking {
                    let _ = tx.send(Ok(StreamEvent::ThinkingStart)).await;
                    in_thinking = true;
                }
                last_meaningful = Instant::now();
                if tx
                    .send(Ok(StreamEvent::ThinkingDelta(thinking)))
                    .await
                    .is_err()
                {
                    break 'read;
                }
                continue;
            }
            if let Some(used_tokens) = crate::wire::extract_checkpoint_used_tokens(&payload) {
                last_meaningful = Instant::now();
                let _ = tx
                    .send(Ok(StreamEvent::TokenUsage {
                        input_tokens: Some(used_tokens),
                        output_tokens: Some(0),
                        cache_read_input_tokens: None,
                        cache_creation_input_tokens: None,
                    }))
                    .await;
                continue;
            }
            // KV server messages are emitted throughout a turn, including
            // ordinary chat. They must be acknowledged or Cursor keeps the
            // bidirectional stream alive indefinitely.
            let mut kv_server_data: Option<&[u8]> = None;
            for f in crate::wire::iter_fields(&payload) {
                if f.field == 4 && f.wire == 2 {
                    kv_server_data = Some(f.data);
                    break;
                }
            }
            if let Some(data) = kv_server_data {
                let mut kv_id = 0u32;
                let mut get_blob_id: Option<Vec<u8>> = None;
                let mut set_blob: Option<(Vec<u8>, Vec<u8>)> = None;
                for field in crate::wire::iter_fields(data) {
                    match field.field {
                        1 if field.wire == 0 => kv_id = field.varint as u32,
                        2 if field.wire == 2 => {
                            let id = crate::wire::iter_fields(field.data)
                                .find(|f| f.field == 1 && f.wire == 2)
                                .map(|f| f.data.to_vec());
                            get_blob_id = id;
                        }
                        3 if field.wire == 2 => {
                            let mut id = None;
                            let mut blob = None;
                            for f in crate::wire::iter_fields(field.data) {
                                if f.field == 1 && f.wire == 2 {
                                    id = Some(f.data.to_vec());
                                } else if f.field == 2 && f.wire == 2 {
                                    blob = Some(f.data.to_vec());
                                }
                            }
                            if let (Some(id), Some(blob)) = (id, blob) {
                                set_blob = Some((id, blob));
                            }
                        }
                        _ => {}
                    }
                }
                if let Some((id, blob)) = set_blob {
                    blob_store.insert(id, blob);
                    if let Err(error) =
                        send_agent_reply(&outbound_tx, &crate::wire::encode_kv_set_blob_ack(kv_id))
                            .await
                    {
                        stream_error = Some(error);
                        break 'read;
                    }
                } else if let Some(id) = get_blob_id {
                    let blob = blob_store.get(&id).map(Vec::as_slice).unwrap_or(&[]);
                    if let Err(error) = send_agent_reply(
                        &outbound_tx,
                        &crate::wire::encode_kv_get_blob_result(kv_id, blob),
                    )
                    .await
                    {
                        stream_error = Some(error);
                        break 'read;
                    }
                }
                continue;
            }

            // Check for ExecServerMessage (field 2 of AgentServerMessage)
            let mut exec_server_data: Option<&[u8]> = None;
            for f in crate::wire::iter_fields(&payload) {
                if f.field == 2 && f.wire == 2 {
                    exec_server_data = Some(f.data);
                    break;
                }
            }
            if let Some(data) = exec_server_data {
                let msg = match crate::wire::decode_exec_server_message(data) {
                    Ok(msg) => msg,
                    Err(_) => {
                        stream_error = Some(anyhow!("Malformed Cursor execution request"));
                        break 'read;
                    }
                };
                use crate::wire::ExecServerMessageVariant;
                match msg.variant {
                    ExecServerMessageVariant::Mcp(mcp_args) => {
                        if in_thinking {
                            let _ = tx.send(Ok(StreamEvent::ThinkingEnd)).await;
                            in_thinking = false;
                        }
                        // Cursor echoes the advertised wire name in `name`,
                        // but the original registry name is carried in
                        // `tool_name`. Prefer the latter so sanitizing a
                        // name for Cursor's identifier grammar does not
                        // break local MCP dispatch.
                        let bare_tool_name =
                            resolve_native_tool_name(&mcp_args.name, &mcp_args.tool_name, tools);
                        let corr_request_id = format!("{}:{}:{}", stream_uuid, msg.id, msg.exec_id);
                        active_tool_calls += 1;
                        last_meaningful = Instant::now();
                        if tx
                            .send(Ok(StreamEvent::NativeToolCall {
                                request_id: corr_request_id,
                                tool_name: bare_tool_name,
                                input: mcp_args.args,
                            }))
                            .await
                            .is_err()
                        {
                            break 'read;
                        }
                    }
                    _ => {
                        if let Some(reply) = crate::wire::encode_control_exec_reply(&msg, tools) {
                            if let Err(error) = send_agent_reply(&outbound_tx, &reply).await {
                                stream_error = Some(error);
                                break 'read;
                            }
                        } else if let ExecServerMessageVariant::Unknown(field, _) = msg.variant {
                            // No reply schema is known for this field. Keep the
                            // existing bounded idle deadline and visible warning.
                            jcode_base::logging::warn(&format!(
                                "Cursor agent sent unsupported exec request field {field}; the turn may stall"
                            ));
                        }
                    }
                }
            }
        }
    }

    let _ = stop_tx.send(());
    let cleanup = if stream_error.is_some() || (!terminal_frame && !idle_finished) {
        tasks::abort_and_drain(&mut tasks).await
    } else {
        tasks::finish(&mut tasks).await
    };
    if let Err(error) = cleanup {
        if stream_error.is_none() && !terminal_frame {
            stream_error = Some(error);
        } else {
            // Never print a secondary panic payload or replace primary errors.
            stream_debug("Cursor cleanup failed after terminal result");
        }
    }

    if in_thinking {
        let _ = tx.send(Ok(StreamEvent::ThinkingEnd)).await;
    }

    if let Some(err) = stream_error {
        anyhow::bail!("{err}");
    }
    if let Some(err) = error_message {
        anyhow::bail!("Cursor agent stream error: {err}");
    }
    if !status.is_success() {
        anyhow::bail!("Cursor agent request failed with HTTP {status}");
    }
    if !terminal_frame && !idle_finished {
        anyhow::bail!(
            "Cursor agent stream ended without a terminal frame{}",
            if got_text { " after text output" } else { "" }
        );
    }

    let _ = tx
        .send(Ok(StreamEvent::MessageEnd {
            stop_reason: Some("end_turn".to_string()),
        }))
        .await;
    Ok(())
}

#[cfg(test)]
#[path = "agent_transport_tests.rs"]
mod tests;
