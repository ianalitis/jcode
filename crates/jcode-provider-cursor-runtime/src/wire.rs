//! Low-level protobuf wire codecs, bounded google.protobuf.Value encoders/decoders,
//! tool schema serialization, and server-native execution rejection encoders for Cursor.
//!
//! Conforms to OpenCodex `agent_pb.ts` and `native-exec.ts` specifications.

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::write::GzEncoder;
use jcode_message_types::ToolDefinition;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write;

/// Maximum recursion depth allowed during google.protobuf.Value encoding and decoding.
pub const MAX_VALUE_DEPTH: usize = 32;

/// Maximum payload size allowed for google.protobuf.Value bytes (10 MiB).
pub const MAX_VALUE_BYTES: usize = 10 * 1024 * 1024;

/// Cursor's AgentService rejects MCP names that collide with its builtins.
/// `cc_` is the namespace used by the proven public bridge implementation.
pub const JCODE_TOOL_PROVIDER: &str = "ccbridge";
pub const JCODE_TOOL_PREFIX: &str = "cc_";
pub const MAX_CURSOR_TOOL_NAME_LEN: usize = 128;

// --------------------------------------------------------------------------
// Protobuf Wire Primitives
// --------------------------------------------------------------------------

/// Encode an unsigned 64-bit integer into variable-length protobuf varint bytes.
pub fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push(((value as u8) & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Encode a length-delimited protobuf field (wire type 2).
pub fn field_ld(field: u64, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 4);
    encode_varint((field << 3) | 2, &mut out);
    encode_varint(data.len() as u64, &mut out);
    out.extend_from_slice(data);
    out
}

/// Encode a varint protobuf field (wire type 0).
pub fn field_varint(field: u64, value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    encode_varint(field << 3, &mut out);
    encode_varint(value, &mut out);
    out
}

/// Encode a UTF-8 string field (wire type 2).
pub fn field_str(field: u64, s: &str) -> Vec<u8> {
    field_ld(field, s.as_bytes())
}

/// Encode a 64-bit fixed float/double field (wire type 1).
pub fn field_fixed64(field: u64, value: f64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    encode_varint((field << 3) | 1, &mut out);
    out.extend_from_slice(&value.to_le_bytes());
    out
}

/// Encode a boolean field (wire type 0).
pub fn field_bool(field: u64, value: bool) -> Vec<u8> {
    field_varint(field, if value { 1 } else { 0 })
}

/// Read a varint from a byte slice.
pub fn read_varint(buf: &[u8]) -> Option<(u64, &[u8])> {
    let mut result = 0u64;
    let mut shift = 0u32;
    for (i, &byte) in buf.iter().enumerate() {
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((result, &buf[i + 1..]));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// A parsed protobuf field header and data slice.
#[derive(Debug, Clone, Copy)]
pub struct PbWireField<'a> {
    pub field: u64,
    pub wire: u8,
    pub varint: u64,
    pub data: &'a [u8],
}

/// Iterate over top-level protobuf fields in a byte buffer.
pub fn iter_fields(mut buf: &[u8]) -> impl Iterator<Item = PbWireField<'_>> {
    std::iter::from_fn(move || {
        if buf.is_empty() {
            return None;
        }
        let (tag, rest) = read_varint(buf)?;
        let field = tag >> 3;
        let wire = (tag & 7) as u8;
        buf = rest;
        match wire {
            0 => {
                let (val, rest) = read_varint(buf)?;
                buf = rest;
                Some(PbWireField {
                    field,
                    wire,
                    varint: val,
                    data: &[],
                })
            }
            1 => {
                if buf.len() < 8 {
                    return None;
                }
                let data = &buf[..8];
                buf = &buf[8..];
                Some(PbWireField {
                    field,
                    wire,
                    varint: 0,
                    data,
                })
            }
            2 => {
                let (len, rest) = read_varint(buf)?;
                let len = len as usize;
                if rest.len() < len {
                    return None;
                }
                let data = &rest[..len];
                buf = &rest[len..];
                Some(PbWireField {
                    field,
                    wire,
                    varint: 0,
                    data,
                })
            }
            5 => {
                if buf.len() < 4 {
                    return None;
                }
                let data = &buf[..4];
                buf = &buf[4..];
                Some(PbWireField {
                    field,
                    wire,
                    varint: 0,
                    data,
                })
            }
            _ => None,
        }
    })
}

// --------------------------------------------------------------------------
// google.protobuf.Value Encoding & Decoding
// --------------------------------------------------------------------------

/// Encode a JSON value into `google.protobuf.Value` binary wire bytes with recursion depth check.
pub fn encode_google_protobuf_value(val: &Value, depth: usize) -> Result<Vec<u8>> {
    if depth > MAX_VALUE_DEPTH {
        bail!("google.protobuf.Value recursion depth limit ({MAX_VALUE_DEPTH}) exceeded");
    }

    match val {
        Value::Null => Ok(field_varint(1, 0)),
        Value::Number(num) => {
            let f = num.as_f64().unwrap_or(0.0);
            Ok(field_fixed64(2, f))
        }
        Value::String(s) => Ok(field_str(3, s)),
        Value::Bool(b) => Ok(field_bool(4, *b)),
        Value::Object(map) => {
            let mut struct_bytes = Vec::new();
            for (k, v) in map {
                let v_bytes = encode_google_protobuf_value(v, depth + 1)?;
                let mut entry = field_str(1, k);
                entry.extend(field_ld(2, &v_bytes));
                struct_bytes.extend(field_ld(1, &entry));
            }
            Ok(field_ld(5, &struct_bytes))
        }
        Value::Array(arr) => {
            let mut list_bytes = Vec::new();
            for item in arr {
                let item_bytes = encode_google_protobuf_value(item, depth + 1)?;
                list_bytes.extend(field_ld(1, &item_bytes));
            }
            Ok(field_ld(6, &list_bytes))
        }
    }
}

/// Decode `google.protobuf.Value` binary wire bytes into a JSON value with depth and size bounds.
pub fn decode_google_protobuf_value(bytes: &[u8], depth: usize) -> Result<Value> {
    if depth > MAX_VALUE_DEPTH {
        bail!("google.protobuf.Value decoding depth limit ({MAX_VALUE_DEPTH}) exceeded");
    }
    if bytes.len() > MAX_VALUE_BYTES {
        bail!("google.protobuf.Value byte size limit ({MAX_VALUE_BYTES}) exceeded");
    }

    for f in iter_fields(bytes) {
        match f.field {
            1 => return Ok(Value::Null),
            2 => {
                if let Some(bytes) = f.data.first_chunk::<8>() {
                    let num_val = f64::from_le_bytes(*bytes);
                    if num_val.fract() == 0.0
                        && num_val >= (i64::MIN as f64)
                        && num_val <= (i64::MAX as f64)
                    {
                        return Ok(Value::Number(serde_json::Number::from(num_val as i64)));
                    }
                    if let Some(num) = serde_json::Number::from_f64(num_val) {
                        return Ok(Value::Number(num));
                    }
                    return Ok(Value::Number(serde_json::Number::from(0)));
                }
            }
            3 => {
                let s = std::str::from_utf8(f.data).context("Invalid UTF-8 in StringValue")?;
                return Ok(Value::String(s.to_string()));
            }
            4 => {
                return Ok(Value::Bool(f.varint != 0));
            }
            5 => {
                let mut map = Map::new();
                for entry_field in iter_fields(f.data) {
                    if entry_field.field == 1 && entry_field.wire == 2 {
                        let mut key = None;
                        let mut val = None;
                        for kv in iter_fields(entry_field.data) {
                            if kv.field == 1 && kv.wire == 2 {
                                key = Some(
                                    std::str::from_utf8(kv.data)
                                        .context("Invalid UTF-8 in Struct key")?
                                        .to_string(),
                                );
                            } else if kv.field == 2 && kv.wire == 2 {
                                val = Some(decode_google_protobuf_value(kv.data, depth + 1)?);
                            }
                        }
                        if let (Some(k), Some(v)) = (key, val)
                            && k != "__proto__"
                            && k != "constructor"
                            && k != "prototype"
                        {
                            map.insert(k, v);
                        }
                    }
                }
                return Ok(Value::Object(map));
            }
            6 => {
                let mut arr = Vec::new();
                for item_field in iter_fields(f.data) {
                    if item_field.field == 1 && item_field.wire == 2 {
                        let item_val = decode_google_protobuf_value(item_field.data, depth + 1)?;
                        arr.push(item_val);
                    }
                }
                return Ok(Value::Array(arr));
            }
            _ => continue,
        }
    }

    bail!("Empty or unrecognized google.protobuf.Value payload");
}

// --------------------------------------------------------------------------
// McpToolDefinition & McpTools
// --------------------------------------------------------------------------

/// Derive the `cc_` namespaced wire name used by Cursor's AgentService.
pub fn mcp_wire_name(tool_name: &str) -> String {
    let bare = tool_name
        .strip_prefix(JCODE_TOOL_PREFIX)
        .unwrap_or(tool_name);
    let mut sanitized = String::with_capacity(bare.len() + JCODE_TOOL_PREFIX.len());
    sanitized.push_str(JCODE_TOOL_PREFIX);
    sanitized.push_str(&sanitize_cursor_name(bare));
    fit_cursor_name(&sanitized, tool_name)
}

/// Normalize a name at the final Cursor protocol boundary.
///
/// Cursor validates every name-bearing field with `^[a-zA-Z0-9_-]+$`. Keep
/// this helper independent from MCP alias allocation so every encoder path can
/// defensively apply the same contract, including secondary name fields.
fn sanitize_cursor_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        sanitized.push_str("tool");
    }
    sanitized
}

/// Build a deterministic, reversible mapping from local registry names to
/// Cursor-safe names. Normalization is intentionally lossy, so collisions are
/// resolved with a stable hash instead of silently dispatching the wrong tool.
pub fn mcp_wire_aliases(tools: &[ToolDefinition]) -> HashMap<String, String> {
    let mut names = tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();

    let mut aliases = HashMap::with_capacity(names.len());
    let mut used = HashSet::with_capacity(names.len());
    for name in names {
        let base = mcp_wire_name(&name);
        let alias = if used.insert(base.clone()) {
            base
        } else {
            let hash = stable_name_hash(&name);
            let mut candidate = fit_cursor_name(&base, &name);
            let suffix = format!("__{hash:08x}");
            let prefix_len = MAX_CURSOR_TOOL_NAME_LEN.saturating_sub(suffix.len());
            candidate.truncate(prefix_len);
            candidate.push_str(&suffix);
            let mut counter = 2u32;
            while !used.insert(candidate.clone()) {
                let numbered = format!("{suffix}_{counter}");
                let prefix_len = MAX_CURSOR_TOOL_NAME_LEN.saturating_sub(numbered.len());
                candidate = base.chars().take(prefix_len).collect();
                candidate.push_str(&numbered);
                counter = counter.saturating_add(1);
            }
            candidate
        };
        aliases.insert(name, alias);
    }
    aliases
}

fn fit_cursor_name(name: &str, original: &str) -> String {
    if name.len() <= MAX_CURSOR_TOOL_NAME_LEN {
        return name.to_string();
    }
    let suffix = format!("__{:08x}", stable_name_hash(original));
    let prefix_len = MAX_CURSOR_TOOL_NAME_LEN.saturating_sub(suffix.len());
    let mut fitted = name.chars().take(prefix_len).collect::<String>();
    fitted.push_str(&suffix);
    fitted
}

fn stable_name_hash(value: &str) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// Extract bare tool name from an advertised or inbound wire name.
pub fn mcp_bare_name(wire_name: &str) -> &str {
    if let Some(rest) = wire_name.strip_prefix(JCODE_TOOL_PREFIX) {
        rest
    } else if let Some((_, rest)) = wire_name.split_once("__") {
        rest
    } else {
        wire_name
    }
}

/// Encode an `McpToolDefinition` message:
/// - field 1: name (string)
/// - field 2: description (string)
/// - field 3: input_schema (bytes, serialized google.protobuf.Value)
/// - field 4: provider_identifier (string, `ccbridge`)
/// - field 5: tool_name (string, bare/safe name)
pub fn encode_mcp_tool_definition(def: &ToolDefinition) -> Result<Vec<u8>> {
    let wire_name = mcp_wire_name(&def.name);
    encode_mcp_tool_definition_with_wire_name(def, &wire_name)
}

fn encode_mcp_tool_definition_with_wire_name(
    def: &ToolDefinition,
    wire_name: &str,
) -> Result<Vec<u8>> {
    // Cursor validates both name-bearing fields even though the public bridge
    // calls field 5 `tool_name`. Use the same safe spelling for both fields.
    // The runtime resolves this alias back to the original registry key when a
    // call returns, so sanitizing cannot break local MCP dispatch.
    let safe_wire_name = fit_cursor_name(&sanitize_cursor_name(wire_name), wire_name);
    let safe_tool_name = fit_cursor_name(
        &sanitize_cursor_name(mcp_bare_name(wire_name)),
        mcp_bare_name(wire_name),
    );
    let schema_bytes = encode_google_protobuf_value(&def.input_schema, 0)
        .context("Failed to encode tool input_schema to google.protobuf.Value")?;

    let mut out = field_str(1, &safe_wire_name);
    out.extend(field_str(2, &def.description));
    out.extend(field_ld(3, &schema_bytes));
    out.extend(field_str(4, JCODE_TOOL_PROVIDER));
    out.extend(field_str(5, &safe_tool_name));
    Ok(out)
}

/// Encode an `McpTools` message containing repeated `mcp_tools` (field 1).
pub fn encode_mcp_tools(tools: &[ToolDefinition]) -> Result<Vec<u8>> {
    let aliases = mcp_wire_aliases(tools);
    let mut out = Vec::new();
    for tool in tools {
        let wire_name = aliases
            .get(&tool.name)
            .cloned()
            .unwrap_or_else(|| mcp_wire_name(&tool.name));
        let def_bytes = encode_mcp_tool_definition_with_wire_name(tool, &wire_name)?;
        out.extend(field_ld(1, &def_bytes));
    }
    Ok(out)
}

/// Encode an `ExecClientMessage` answering `McpStateExecArgs` with
/// `mcp_state_exec_result` (field 36) -> `McpStateSuccess` (case 1).
///
/// jcode advertises every bridged tool under one pseudo server,
/// [`JCODE_TOOL_PROVIDER`]. Report that server as ready with the same tool
/// definitions sent in `RunRequest.mcp_tools` so Cursor proceeds to the
/// `McpArgs` call instead of waiting for a server it thinks is still loading.
pub fn encode_mcp_state_result(
    id: u32,
    exec_id: &str,
    requested: &McpStateExecArgs,
    tools: &[ToolDefinition],
) -> Result<Vec<u8>> {
    let mut success = Vec::new();
    let wants_bridge = requested.server_identifiers.is_empty()
        || requested
            .server_identifiers
            .iter()
            .any(|id| id == JCODE_TOOL_PROVIDER);
    if wants_bridge {
        // McpStateServer: server_name=1, server_identifier=2, tools=5 (repeated
        // McpToolDefinition), status=7
        let mut server = field_str(1, JCODE_TOOL_PROVIDER);
        server.extend(field_str(2, JCODE_TOOL_PROVIDER));
        let aliases = mcp_wire_aliases(tools);
        for tool in tools {
            let wire_name = aliases
                .get(&tool.name)
                .cloned()
                .unwrap_or_else(|| mcp_wire_name(&tool.name));
            let def = encode_mcp_tool_definition_with_wire_name(tool, &wire_name)?;
            server.extend(field_ld(5, &def));
        }
        server.extend(field_str(7, "ready"));
        // McpStateSuccess: servers=1 (repeated)
        success.extend(field_ld(1, &server));
    }
    // McpStateExecResult: success = 1
    let result = field_ld(1, &success);
    Ok(encode_exec_client_message(id, exec_id, 36, &result))
}

/// Encode an `McpStateError` result (case 2) for `McpStateExecArgs`.
pub fn encode_mcp_state_error(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    let err = field_str(1, error);
    let result = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 36, &result)
}

/// Answer an allowlist precheck (`*AllowlistPrecheckResult { allowlisted = 1 }`)
/// on `response_field` (41 shell, 42 mcp, 43 web fetch). jcode applies its own
/// permission policy when it executes a bridged tool, so its `ccbridge` MCP
/// tools are pre-approved. Cursor's native shell/fetch are not (jcode rejects
/// those exec requests anyway). Leaving a precheck unanswered stalls the turn.
pub fn encode_allowlist_precheck_result(
    id: u32,
    exec_id: &str,
    response_field: u64,
    allowlisted: bool,
) -> Vec<u8> {
    let result = field_bool(1, allowlisted);
    encode_exec_client_message(id, exec_id, response_field, &result)
}

// --------------------------------------------------------------------------
// McpArgs & Args Map Decoding
// --------------------------------------------------------------------------

/// Decode an inbound Cursor McpArgs `args` map (map<string, bytes>) into a JSON Object.
/// Falls back to UTF-8 JSON text decoding if canonical protobuf Value decoding fails.
pub fn decode_cursor_args_map(args_data: &[u8]) -> Result<Value> {
    let mut map = Map::new();

    for field in iter_fields(args_data) {
        if field.field == 2 && field.wire == 2 {
            // Entry in repeated map<string, bytes>
            let mut key = None;
            let mut val_bytes: Option<&[u8]> = None;
            for kv in iter_fields(field.data) {
                if kv.field == 1 && kv.wire == 2 {
                    key = Some(
                        std::str::from_utf8(kv.data)
                            .context("Invalid UTF-8 in Cursor argument key")?
                            .to_string(),
                    );
                } else if kv.field == 2 && kv.wire == 2 {
                    val_bytes = Some(kv.data);
                }
            }

            if let (Some(k), Some(bytes)) = (key, val_bytes) {
                if k == "__proto__" || k == "constructor" || k == "prototype" {
                    continue;
                }
                // Try canonical protobuf Value first
                let decoded = match decode_google_protobuf_value(bytes, 0) {
                    Ok(val) => val,
                    Err(_) => {
                        // Fallback: UTF-8 JSON text parse or string
                        match std::str::from_utf8(bytes) {
                            Ok(text) => match serde_json::from_str::<Value>(text) {
                                Ok(json_val) => json_val,
                                Err(_) => Value::String(text.to_string()),
                            },
                            Err(_) => Value::String(String::from_utf8_lossy(bytes).into_owned()),
                        }
                    }
                };
                map.insert(k, decoded);
            }
        }
    }

    Ok(Value::Object(map))
}

// --------------------------------------------------------------------------
// Inbound ExecServerMessage Types & Decoder
// --------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ShellArgs {
    pub command: String,
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WriteArgs {
    pub path: String,
    pub file_text: String,
    pub file_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteArgs {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrepArgs {
    pub pattern: String,
    pub path: String,
    pub case_insensitive: bool,
    pub glob: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadArgs {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LsArgs {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticsArgs {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestContextArgs {}

#[derive(Debug, Clone, PartialEq)]
pub struct McpArgs {
    pub name: String,
    pub args: Value,
    pub tool_call_id: String,
    pub provider_identifier: String,
    pub tool_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellStreamArgs {
    pub command: String,
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundShellSpawnArgs {
    pub command: String,
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListMcpResourcesExecArgs {}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadMcpResourceExecArgs {
    pub server: String,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FetchArgs {
    pub url: String,
    pub tool_call_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordScreenArgs {}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputerUseArgs {}

#[derive(Debug, Clone, PartialEq)]
pub struct WriteShellStdinArgs {
    pub shell_id: u32,
    pub stdin: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecServerMessageVariant {
    Shell(ShellArgs),
    Write(WriteArgs),
    Delete(DeleteArgs),
    Grep(GrepArgs),
    Read(ReadArgs),
    Ls(LsArgs),
    Diagnostics(DiagnosticsArgs),
    RequestContext(RequestContextArgs),
    Mcp(McpArgs),
    ShellStream(ShellStreamArgs),
    BackgroundShellSpawn(BackgroundShellSpawnArgs),
    ListMcpResources(ListMcpResourcesExecArgs),
    ReadMcpResource(ReadMcpResourceExecArgs),
    Fetch(FetchArgs),
    RecordScreen(RecordScreenArgs),
    ComputerUse(ComputerUseArgs),
    WriteShellStdin(WriteShellStdinArgs),
    McpState(McpStateExecArgs),
    /// `mcp_allowlist_precheck_args` (42): provider_identifier=1, tool_name=2.
    McpAllowlistPrecheck {
        provider_identifier: String,
    },
    /// `shell_allowlist_precheck_args` (41) / `web_fetch_allowlist_precheck_args`
    /// (43). Carries the ExecClientMessage response field number.
    OtherAllowlistPrecheck(u64),
    Unknown(u64, Vec<u8>),
}

/// `McpStateExecArgs` (ExecServerMessage field 36). Cursor asks for the state
/// of the listed MCP servers before it dispatches an `McpArgs` call. Leaving
/// it unanswered stalls the turn with only heartbeat frames.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct McpStateExecArgs {
    pub server_identifiers: Vec<String>,
    pub kick_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecServerMessage {
    pub id: u32,
    pub exec_id: String,
    pub variant: ExecServerMessageVariant,
}

/// Decode an `ExecServerMessage` from protobuf wire bytes.
pub fn decode_exec_server_message(bytes: &[u8]) -> Result<ExecServerMessage> {
    let mut id = 0u32;
    let mut exec_id = String::new();
    let mut variant = None;

    for field in iter_fields(bytes) {
        match field.field {
            1 => id = field.varint as u32,
            15 => {
                if let Ok(s) = std::str::from_utf8(field.data) {
                    exec_id = s.to_string();
                }
            }
            19 => {
                // span_context: tracing metadata, ignored
            }
            2 => {
                // ShellArgs (command=1, working_directory=2)
                let mut cmd = String::new();
                let mut cwd = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        cmd = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        cwd = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Shell(ShellArgs {
                    command: cmd,
                    working_directory: cwd,
                }));
            }
            3 => {
                // WriteArgs (path=1, file_text=2, file_bytes=3)
                let mut path = String::new();
                let mut file_text = String::new();
                let mut file_bytes = Vec::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        path = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        file_text = s.to_string();
                    } else if f.field == 3 {
                        file_bytes = f.data.to_vec();
                    }
                }
                variant = Some(ExecServerMessageVariant::Write(WriteArgs {
                    path,
                    file_text,
                    file_bytes,
                }));
            }
            4 => {
                // DeleteArgs (path=1)
                let mut path = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        path = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Delete(DeleteArgs { path }));
            }
            5 => {
                // GrepArgs (pattern=1, path=2, case_insensitive=3, glob=4)
                let mut pattern = String::new();
                let mut path = String::new();
                let mut case_insensitive = false;
                let mut glob = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        pattern = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        path = s.to_string();
                    } else if f.field == 3 {
                        case_insensitive = f.varint != 0;
                    } else if f.field == 4
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        glob = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Grep(GrepArgs {
                    pattern,
                    path,
                    case_insensitive,
                    glob,
                }));
            }
            7 => {
                // ReadArgs (path=1)
                let mut path = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        path = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Read(ReadArgs { path }));
            }
            8 => {
                // LsArgs (path=1)
                let mut path = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        path = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Ls(LsArgs { path }));
            }
            9 => {
                // DiagnosticsArgs (path=1)
                let mut path = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        path = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Diagnostics(DiagnosticsArgs {
                    path,
                }));
            }
            10 => {
                variant = Some(ExecServerMessageVariant::RequestContext(
                    RequestContextArgs {},
                ));
            }
            11 => {
                // McpArgs (name=1, args=2 map, tool_call_id=3, provider_identifier=4, tool_name=5)
                let mut name = String::new();
                let mut tool_call_id = String::new();
                let mut provider_identifier = String::new();
                let mut tool_name = String::new();
                let args = decode_cursor_args_map(field.data).unwrap_or(Value::Object(Map::new()));

                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        name = s.to_string();
                    } else if f.field == 3
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        tool_call_id = s.to_string();
                    } else if f.field == 4
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        provider_identifier = s.to_string();
                    } else if f.field == 5
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        tool_name = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Mcp(McpArgs {
                    name,
                    args,
                    tool_call_id,
                    provider_identifier,
                    tool_name,
                }));
            }
            14 => {
                let mut cmd = String::new();
                let mut cwd = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        cmd = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        cwd = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::ShellStream(ShellStreamArgs {
                    command: cmd,
                    working_directory: cwd,
                }));
            }
            16 => {
                let mut cmd = String::new();
                let mut cwd = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        cmd = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        cwd = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::BackgroundShellSpawn(
                    BackgroundShellSpawnArgs {
                        command: cmd,
                        working_directory: cwd,
                    },
                ));
            }
            17 => {
                variant = Some(ExecServerMessageVariant::ListMcpResources(
                    ListMcpResourcesExecArgs {},
                ));
            }
            18 => {
                let mut server = String::new();
                let mut uri = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        server = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        uri = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::ReadMcpResource(
                    ReadMcpResourceExecArgs { server, uri },
                ));
            }
            20 => {
                let mut url = String::new();
                let mut tool_call_id = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        url = s.to_string();
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        tool_call_id = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::Fetch(FetchArgs {
                    url,
                    tool_call_id,
                }));
            }
            21 => variant = Some(ExecServerMessageVariant::RecordScreen(RecordScreenArgs {})),
            22 => variant = Some(ExecServerMessageVariant::ComputerUse(ComputerUseArgs {})),
            23 => {
                let mut shell_id = 0u32;
                let mut stdin = String::new();
                for f in iter_fields(field.data) {
                    if f.field == 1 {
                        shell_id = f.varint as u32;
                    } else if f.field == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        stdin = s.to_string();
                    }
                }
                variant = Some(ExecServerMessageVariant::WriteShellStdin(
                    WriteShellStdinArgs { shell_id, stdin },
                ));
            }
            36 => {
                // McpStateExecArgs (server_identifiers=1 repeated, kick_only=2)
                let mut args = McpStateExecArgs::default();
                for f in iter_fields(field.data) {
                    if f.field == 1
                        && f.wire == 2
                        && let Ok(s) = std::str::from_utf8(f.data)
                    {
                        args.server_identifiers.push(s.to_string());
                    } else if f.field == 2 && f.wire == 0 {
                        args.kick_only = f.varint != 0;
                    }
                }
                variant = Some(ExecServerMessageVariant::McpState(args));
            }
            42 => {
                let provider_identifier = iter_fields(field.data)
                    .find(|f| f.field == 1 && f.wire == 2)
                    .map(|f| std::str::from_utf8(f.data))
                    .transpose()
                    .context("Invalid UTF-8 in Cursor provider identifier")?
                    .unwrap_or_default()
                    .to_string();
                variant = Some(ExecServerMessageVariant::McpAllowlistPrecheck {
                    provider_identifier,
                });
            }
            41 | 43 => {
                variant = Some(ExecServerMessageVariant::OtherAllowlistPrecheck(
                    field.field,
                ));
            }
            other => {
                if variant.is_none() {
                    variant = Some(ExecServerMessageVariant::Unknown(
                        other,
                        field.data.to_vec(),
                    ));
                }
            }
        }
    }

    let variant = variant.context("ExecServerMessage contained no execution oneof payload")?;
    Ok(ExecServerMessage {
        id,
        exec_id,
        variant,
    })
}

// --------------------------------------------------------------------------
// Outbound ExecClientMessage Codecs
// --------------------------------------------------------------------------

/// Wrap a raw response oneof into an `ExecClientMessage`:
/// - field 1: id (uint32)
/// - field 15: exec_id (string, if non-empty)
/// - field `response_field`: response_payload (wire type 2)
pub fn encode_exec_client_message(
    id: u32,
    exec_id: &str,
    response_field: u64,
    response_payload: &[u8],
) -> Vec<u8> {
    let mut out = field_varint(1, id as u64);
    if !exec_id.is_empty() {
        out.extend(field_str(15, exec_id));
    }
    out.extend(field_ld(response_field, response_payload));
    out
}

/// Wrap an `ExecClientMessage` into an `AgentClientMessage` (field 2).
pub fn encode_agent_client_exec_message(exec_client_bytes: &[u8]) -> Vec<u8> {
    field_ld(2, exec_client_bytes)
}

/// Wrap a stream close control message into an `AgentClientMessage` (field 5).
pub fn encode_agent_client_stream_close(id: u32) -> Vec<u8> {
    // ExecClientControlMessage: field 1 = stream_close (ExecClientStreamClose: id=1)
    let close_bytes = field_varint(1, id as u64);
    let control_bytes = field_ld(1, &close_bytes);
    field_ld(5, &control_bytes)
}

pub fn encode_request_context_result(
    id: u32,
    exec_id: &str,
    request_context_bytes: &[u8],
) -> Vec<u8> {
    let request_context_success = field_ld(1, request_context_bytes);
    let result = field_ld(1, &request_context_success);
    // Preserve the server's correlation fields when supplied. Current Cursor
    // sends these on ExecServerMessage and expects them on the response; omit
    // proto3 defaults only when they are genuinely absent.
    let mut exec = Vec::new();
    if id != 0 {
        exec.extend(field_varint(1, id as u64));
    }
    exec.extend(field_str(15, exec_id));
    exec.extend(field_ld(10, &result));
    field_ld(2, &exec)
}

pub fn encode_request_context(system_prompt: &str, cwd: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    if !system_prompt.trim().is_empty() {
        let mut rule = field_str(1, "/jcode/system-prompt/0.mdc");
        rule.extend(field_str(2, system_prompt.trim()));
        rule.extend(field_varint(3, 1));
        let rule_type = field_ld(1, &[]);
        rule.extend(field_ld(4, &rule_type));
        out.extend(field_ld(2, &rule));
    }
    let mut env = field_str(1, std::env::consts::OS);
    env.extend(field_str(2, cwd));
    env.extend(field_str(
        3,
        std::env::var("SHELL").ok().as_deref().unwrap_or("bash"),
    ));
    env.extend(field_str(10, "UTC"));
    // These fields are emitted by the official cursor-agent client. They are
    // proto defaults for local execution, but omitting them makes some Agent
    // Service deployments keep the stream alive with heartbeats after the ack.
    env.extend(field_varint(14, 0));
    env.extend(field_varint(16, 1));
    env.extend(field_varint(19, 0));
    env.extend(field_varint(20, 0));
    env.extend(field_str(21, cwd));
    env.extend(field_varint(22, 0));
    out.extend(field_ld(4, &env));
    Ok(out)
}

/// Extract cumulative used tokens from a `conversationCheckpointUpdate` payload (field 3 of AgentServerMessage).
/// Path: f3 (conversationCheckpointUpdate) -> f5 (tokenDetails) -> f1 (usedTokens varint).
pub fn extract_checkpoint_used_tokens(payload: &[u8]) -> Option<u64> {
    for f3 in iter_fields(payload) {
        if f3.field == 3 && f3.wire == 2 {
            for f5 in iter_fields(f3.data) {
                if f5.field == 5 && f5.wire == 2 {
                    for f1 in iter_fields(f5.data) {
                        if f1.field == 1 && f1.wire == 0 {
                            return Some(f1.varint);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Wrap an uncompressed payload in a Connect data frame (flag 0).
pub fn connect_frame(payload: &[u8]) -> Vec<u8> {
    const COMPRESS_MIN_BYTES: usize = 1024;
    let (flag, framed_payload) = if payload.len() >= COMPRESS_MIN_BYTES {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        if encoder.write_all(payload).is_ok()
            && let Ok(compressed) = encoder.finish()
        {
            (1u8, compressed)
        } else {
            (0u8, payload.to_vec())
        }
    } else {
        (0u8, payload.to_vec())
    };
    let mut out = Vec::with_capacity(payload.len() + 5);
    out.push(flag);
    out.extend_from_slice(&(framed_payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&framed_payload);
    out
}

/// Encode an `AgentClientMessage.kv_client_message` acknowledgement for a
/// server-side blob write. Cursor sends these KV messages even for turns that
/// do not use tools, and leaving them unanswered eventually stalls the stream.
pub fn encode_kv_set_blob_ack(kv_id: u32) -> Vec<u8> {
    let mut client = Vec::new();
    if kv_id != 0 {
        client.extend(field_varint(1, kv_id as u64));
    }
    client.extend(field_ld(3, &[]));
    field_ld(3, &client)
}

/// Encode an `AgentClientMessage.kv_client_message` response for a server-side
/// blob read. The blob id is intentionally opaque and is never interpreted.
pub fn encode_kv_get_blob_result(kv_id: u32, data: &[u8]) -> Vec<u8> {
    let result = field_ld(1, data);
    let mut client = Vec::new();
    if kv_id != 0 {
        client.extend(field_varint(1, kv_id as u64));
    }
    client.extend(field_ld(2, &result));
    field_ld(3, &client)
}

include!("wire/results.rs");
