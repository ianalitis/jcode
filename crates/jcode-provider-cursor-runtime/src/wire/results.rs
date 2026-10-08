// Policy guidance and response encoders for Cursor execution requests.
// Included by wire.rs so the wire API and denial semantics stay unchanged.

/// Encode control replies without executing native tools. MCP calls are handled
/// by the authorized registry bridge, and unknown variants have no known reply.
/// The result is already an AgentClientMessage, including request-context acks.
pub fn encode_control_exec_reply(
    msg: &ExecServerMessage,
    tools: &[jcode_message_types::ToolDefinition],
) -> Option<Vec<u8>> {
    use ExecServerMessageVariant as V;
    let id = msg.id;
    let exec_id = &msg.exec_id;
    let local = native_local_exec_disabled_message(false);
    let shell = native_shell_disabled_message(false);
    let reply = match &msg.variant {
        V::RequestContext(_) => return Some(encode_request_context_result(id, exec_id, &[])),
        V::Shell(args) => {
            reject_shell_exec(id, exec_id, &args.command, &args.working_directory, shell)
        }
        V::Write(args) => reject_write_exec(id, exec_id, &args.path, local),
        V::Delete(args) => reject_delete_exec(id, exec_id, &args.path, local),
        V::Grep(_) => reject_grep_exec(id, exec_id, local),
        V::Read(args) => reject_read_exec(id, exec_id, &args.path, local),
        V::Ls(args) => reject_ls_exec(id, exec_id, &args.path, local),
        V::Diagnostics(args) => reject_diagnostics_exec(id, exec_id, &args.path, local),
        V::ShellStream(_) => reject_shell_stream_exec(id, exec_id, shell),
        V::BackgroundShellSpawn(args) => reject_background_shell_spawn_exec(
            id,
            exec_id,
            &args.command,
            &args.working_directory,
            shell,
        ),
        V::ListMcpResources(_) => reject_list_mcp_resources_exec(id, exec_id, local),
        V::ReadMcpResource(args) => reject_read_mcp_resource_exec(id, exec_id, &args.uri, local),
        V::Fetch(args) => {
            reject_fetch_exec(id, exec_id, &args.url, native_fetch_disabled_message(false))
        }
        V::RecordScreen(_) => reject_record_screen_exec(id, exec_id, local),
        V::ComputerUse(_) => reject_computer_use_exec(id, exec_id, local),
        V::WriteShellStdin(_) => reject_write_shell_stdin_exec(id, exec_id, shell),
        V::McpState(args) => {
            encode_mcp_state_result(id, exec_id, args, tools).unwrap_or_else(|error| {
                encode_mcp_state_error(
                    id,
                    exec_id,
                    &format!("jcode could not describe its tools: {error}"),
                )
            })
        }
        V::McpAllowlistPrecheck {
            provider_identifier,
        } => encode_allowlist_precheck_result(
            id,
            exec_id,
            42,
            provider_identifier == JCODE_TOOL_PROVIDER,
        ),
        V::OtherAllowlistPrecheck(field) => {
            encode_allowlist_precheck_result(id, exec_id, *field, false)
        }
        V::Mcp(_) | V::Unknown(_, _) => return None,
    };
    Some(encode_agent_client_exec_message(&reply))
}

pub fn native_local_exec_disabled_message(code_mode: bool) -> &'static str {
    if code_mode {
        r#"Cursor-native local tools are policy-redirected to the jcode bridge for this request; this is not a permissions denial. Use the top-level `exec` tool and call nested helpers inside its JavaScript body as `await tools.<name>(args)` (for example `await tools.exec_command({cmd: "ls"})`, `await tools.apply_patch(input)`, or another helper listed in `ALL_TOOLS`). Do not call `shell_command` or `exec_command` at the top level in code mode."#
    } else {
        "Re-issue this operation NOW through an authorized jcode tool bridge. Do NOT narrate this redirect, comment on tool availability, or re-announce the task — just make the bridge call."
    }
}

pub fn native_shell_disabled_message(code_mode: bool) -> &'static str {
    if code_mode {
        r#"Cursor-native shell is policy-redirected to the jcode bridge for this request; this is not a permissions denial. Use the top-level `exec` tool and call a nested shell helper inside its JavaScript body as `await tools.exec_command({cmd: "..."})` (or another helper listed in `ALL_TOOLS`). Do not call `shell_command` or `exec_command` at the top level in code mode."#
    } else {
        "Re-issue this command NOW through an authorized jcode shell tool bridge. Do NOT narrate this redirect, comment on tool availability, or re-announce the task — just make the bridge call."
    }
}

pub fn native_fetch_disabled_message(code_mode: bool) -> &'static str {
    if code_mode {
        r#"Cursor-native fetch is policy-redirected to the jcode bridge for this request; this is not a permissions denial. Use the top-level `exec` tool and call a nested shell helper inside its JavaScript body as `await tools.exec_command({cmd: "curl ..."})` (or another helper listed in `ALL_TOOLS`). Do not call `shell_command` or `exec_command` at the top level in code mode."#
    } else {
        "Re-issue this fetch NOW through an authorized jcode network tool bridge. Do NOT narrate this redirect or comment on tool availability — just make the bridge call."
    }
}

// --------------------------------------------------------------------------
// Rejection Encoders for Server-Native Exec Variants (CURS-03)
// --------------------------------------------------------------------------

/// Reject `shellArgs` -> returns `shell_result` (field 2) containing `ShellFailure` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_shell_exec(
    id: u32,
    exec_id: &str,
    command: &str,
    cwd: &str,
    reason: &str,
) -> Vec<u8> {
    // ShellFailure: command=1, working_directory=2, exit_code=3 (1), signal=4 (""), stderr=6 (reason), execution_time=7 (0), aborted=11 (true)
    let mut failure = field_str(1, command);
    failure.extend(field_str(2, cwd));
    failure.extend(field_varint(3, 1));
    failure.extend(field_str(4, ""));
    failure.extend(field_str(6, reason));
    failure.extend(field_varint(7, 0));
    failure.extend(field_bool(11, true));

    // ShellResult: failure = 2
    let shell_result = field_ld(2, &failure);
    encode_exec_client_message(id, exec_id, 2, &shell_result)
}

/// Reject `writeArgs` -> returns `write_result` (field 3) containing `WriteRejected` (case 6).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_write_exec(id: u32, exec_id: &str, path: &str, reason: &str) -> Vec<u8> {
    // WriteRejected: path=1, reason=2
    let mut rejected = field_str(1, path);
    rejected.extend(field_str(2, reason));

    // WriteResult: rejected = 6
    let write_result = field_ld(6, &rejected);
    encode_exec_client_message(id, exec_id, 3, &write_result)
}

/// Reject `deleteArgs` -> returns `delete_result` (field 4) containing `DeleteRejected` (case 6).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_delete_exec(id: u32, exec_id: &str, path: &str, reason: &str) -> Vec<u8> {
    // DeleteRejected: path=1, reason=2
    let mut rejected = field_str(1, path);
    rejected.extend(field_str(2, reason));

    // DeleteResult: rejected = 6
    let delete_result = field_ld(6, &rejected);
    encode_exec_client_message(id, exec_id, 4, &delete_result)
}

/// Reject `grepArgs` -> returns `grep_result` (field 5) containing `GrepError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_grep_exec(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    // GrepError: error=1
    let grep_err = field_str(1, error);
    // GrepResult: error = 2
    let grep_result = field_ld(2, &grep_err);
    encode_exec_client_message(id, exec_id, 5, &grep_result)
}

/// Reject `readArgs` -> returns `read_result` (field 7) containing `ReadError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_read_exec(id: u32, exec_id: &str, path: &str, error: &str) -> Vec<u8> {
    // ReadError: path=1, error=2
    let mut read_err = field_str(1, path);
    read_err.extend(field_str(2, error));
    // ReadResult: error = 2
    let read_result = field_ld(2, &read_err);
    encode_exec_client_message(id, exec_id, 7, &read_result)
}

/// Reject `lsArgs` -> returns `ls_result` (field 8) containing `LsError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_ls_exec(id: u32, exec_id: &str, path: &str, error: &str) -> Vec<u8> {
    // LsError: path=1, error=2
    let mut ls_err = field_str(1, path);
    ls_err.extend(field_str(2, error));
    // LsResult: error = 2
    let ls_result = field_ld(2, &ls_err);
    encode_exec_client_message(id, exec_id, 8, &ls_result)
}

/// Reject `diagnosticsArgs` -> returns `diagnostics_result` (field 9) containing `DiagnosticsError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_diagnostics_exec(id: u32, exec_id: &str, path: &str, error: &str) -> Vec<u8> {
    // DiagnosticsError: path=1, error=2
    let mut diag_err = field_str(1, path);
    diag_err.extend(field_str(2, error));
    // DiagnosticsResult: error = 2
    let diag_result = field_ld(2, &diag_err);
    encode_exec_client_message(id, exec_id, 9, &diag_result)
}

/// Reject `shellStreamArgs` -> returns `shell_stream` (field 14) containing `ShellStreamStderr` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_shell_stream_exec(id: u32, exec_id: &str, stderr: &str) -> Vec<u8> {
    // ShellStreamStderr: data=1
    let stderr_item = field_str(1, stderr);
    // ShellStream: stderr = 2
    let stream_result = field_ld(2, &stderr_item);
    encode_exec_client_message(id, exec_id, 14, &stream_result)
}

/// Reject `backgroundShellSpawnArgs` -> returns `background_shell_spawn_result` (field 16) containing `BackgroundShellSpawnError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_background_shell_spawn_exec(
    id: u32,
    exec_id: &str,
    command: &str,
    cwd: &str,
    error: &str,
) -> Vec<u8> {
    // BackgroundShellSpawnError: command=1, working_directory=2, error=3
    let mut err = field_str(1, command);
    err.extend(field_str(2, cwd));
    err.extend(field_str(3, error));
    // BackgroundShellSpawnResult: error = 2
    let bg_result = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 16, &bg_result)
}

/// Reject `listMcpResourcesExecArgs` -> returns `list_mcp_resources_exec_result` (field 17) containing `ListMcpResourcesError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_list_mcp_resources_exec(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    // ListMcpResourcesError: error=1
    let err = field_str(1, error);
    // ListMcpResourcesExecResult: error = 2
    let res = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 17, &res)
}

/// Reject `readMcpResourceExecArgs` -> returns `read_mcp_resource_exec_result` (field 18) containing `ReadMcpResourceError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_read_mcp_resource_exec(id: u32, exec_id: &str, uri: &str, error: &str) -> Vec<u8> {
    // ReadMcpResourceError: uri=1, error=2
    let mut err = field_str(1, uri);
    err.extend(field_str(2, error));
    // ReadMcpResourceExecResult: error = 2
    let res = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 18, &res)
}

/// Reject `fetchArgs` -> returns `fetch_result` (field 20) containing `FetchError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_fetch_exec(id: u32, exec_id: &str, url: &str, error: &str) -> Vec<u8> {
    // FetchError: url=1, error=2
    let mut err = field_str(1, url);
    err.extend(field_str(2, error));
    // FetchResult: error = 2
    let res = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 20, &res)
}

/// Reject `recordScreenArgs` -> returns `record_screen_result` (field 21) containing `RecordScreenFailure` (case 4).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_record_screen_exec(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    // RecordScreenFailure: error=1
    let err = field_str(1, error);
    // RecordScreenResult: failure = 4
    let res = field_ld(4, &err);
    encode_exec_client_message(id, exec_id, 21, &res)
}

/// Reject `computerUseArgs` -> returns `computer_use_result` (field 22) containing `ComputerUseError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_computer_use_exec(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    // ComputerUseError: error=1, action_count=2, duration_ms=3
    let mut err = field_str(1, error);
    err.extend(field_varint(2, 0));
    err.extend(field_varint(3, 0));
    // ComputerUseResult: error = 2
    let res = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 22, &res)
}

/// Reject `writeShellStdinArgs` -> returns `write_shell_stdin_result` (field 23) containing `WriteShellStdinError` (case 2).
/// NON-MCP VARIANT: Never returns mcp_result!
pub fn reject_write_shell_stdin_exec(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    // WriteShellStdinError: error=1
    let err = field_str(1, error);
    // WriteShellStdinResult: error = 2
    let res = field_ld(2, &err);
    encode_exec_client_message(id, exec_id, 23, &res)
}

// --------------------------------------------------------------------------
// McpResult Encoders (Field 11)
// --------------------------------------------------------------------------

/// Encode a successful McpResult with text output:
/// McpResult -> success (case 1) -> McpSuccess:
/// - content (repeated field 1): McpToolResultContentItem: text (case 1) -> McpTextContent: text=1
/// - is_error (field 2): bool
pub fn encode_mcp_success_result(id: u32, exec_id: &str, text: &str, is_error: bool) -> Vec<u8> {
    // McpTextContent: text = 1
    let text_content = field_str(1, text);
    // McpToolResultContentItem: text = 1
    let content_item = field_ld(1, &text_content);

    // McpSuccess: content = 1 (repeated), is_error = 2
    let mut success = field_ld(1, &content_item);
    success.extend(field_bool(2, is_error));

    // McpResult: success = 1
    let mcp_result = field_ld(1, &success);
    encode_exec_client_message(id, exec_id, 11, &mcp_result)
}

/// Encode an error McpResult:
/// McpResult -> error (case 2) -> McpError: error=1
pub fn encode_mcp_error_result(id: u32, exec_id: &str, error: &str) -> Vec<u8> {
    // McpError: error = 1
    let mcp_err = field_str(1, error);
    // McpResult: error = 2
    let mcp_result = field_ld(2, &mcp_err);
    encode_exec_client_message(id, exec_id, 11, &mcp_result)
}

/// Encode a toolNotFound McpResult:
/// McpResult -> tool_not_found (case 5) -> McpToolNotFound: name=1, available_tools=2 (repeated)
pub fn encode_mcp_not_found_result(
    id: u32,
    exec_id: &str,
    tool_name: &str,
    available_tools: &[String],
) -> Vec<u8> {
    let mut not_found = field_str(1, tool_name);
    for t in available_tools {
        not_found.extend(field_str(2, t));
    }
    // McpResult: tool_not_found = 5
    let mcp_result = field_ld(5, &not_found);
    encode_exec_client_message(id, exec_id, 11, &mcp_result)
}
