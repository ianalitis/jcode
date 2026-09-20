use super::{Tool, ToolContext, ToolOutput};
use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use url::Url;

const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MAX_TIMEOUT_MS: u64 = 30_000;
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_DIAGNOSTICS: usize = 100;
const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const EXIT_GRACE: Duration = Duration::from_secs(1);
const KILL_GRACE: Duration = Duration::from_secs(2);

pub struct ZlsDiagnosticsTool {
    program: PathBuf,
    env: Vec<(OsString, OsString)>,
}

impl ZlsDiagnosticsTool {
    pub fn new() -> Self {
        Self {
            program: PathBuf::from("zls"),
            env: Vec::new(),
        }
    }

    #[cfg(test)]
    fn with_test_command(program: PathBuf, env: Vec<(OsString, OsString)>) -> Self {
        Self { program, env }
    }
}

#[derive(Debug, Deserialize)]
struct ZlsDiagnosticsInput {
    file_path: String,
    workspace_root: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug)]
struct ValidatedRequest {
    root: PathBuf,
    display_path: String,
    root_uri: String,
    file_uri: String,
    source: String,
    timeout: Duration,
}

#[derive(Debug)]
struct ProtocolResult {
    diagnostics: DiagnosticBatch,
    server_name: Option<String>,
    server_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PublishDiagnosticsParams {
    uri: String,
    diagnostics: Vec<RawDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct RawDiagnostic {
    range: RawRange,
    #[serde(default)]
    severity: Option<u8>,
    #[serde(default)]
    code: Option<Value>,
    #[serde(default)]
    source: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RawRange {
    start: RawPosition,
}

#[derive(Debug, Deserialize)]
struct RawPosition {
    line: u64,
    character: u64,
}

#[derive(Debug)]
struct DiagnosticBatch {
    entries: Vec<Diagnostic>,
    reported_count: usize,
}

#[derive(Debug, Eq, PartialEq)]
struct Diagnostic {
    severity_rank: u8,
    severity: &'static str,
    line: u64,
    column: u64,
    source: Option<String>,
    code: Option<String>,
    message: String,
}

struct ChildGuard {
    child: Child,
    pid: Option<u32>,
    armed: bool,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        let pid = child.id();
        Self {
            child,
            pid,
            armed: true,
        }
    }

    fn take_stdio(&mut self) -> Result<(ChildStdin, ChildStdout)> {
        let stdin = self.child.stdin.take().context("ZLS stdin was not piped")?;
        let stdout = self
            .child
            .stdout
            .take()
            .context("ZLS stdout was not piped")?;
        Ok((stdin, stdout))
    }

    async fn finish_after_exit(&mut self) -> Result<()> {
        match tokio::time::timeout(EXIT_GRACE, self.child.wait()).await {
            Ok(status) => {
                let status = status.context("failed waiting for ZLS to exit")?;
                self.kill_process_tree();
                self.armed = false;
                if !status.success() {
                    bail!("ZLS exited unsuccessfully after orderly shutdown: {status}");
                }
                Ok(())
            }
            Err(_) => self.terminate_and_wait().await,
        }
    }

    async fn terminate_and_wait(&mut self) -> Result<()> {
        self.kill_process_tree();
        let _ = self.child.start_kill();
        match tokio::time::timeout(KILL_GRACE, self.child.wait()).await {
            Ok(status) => {
                status.context("failed waiting for terminated ZLS process")?;
                self.armed = false;
                Ok(())
            }
            Err(_) => bail!("ZLS process did not exit after process-tree termination"),
        }
    }

    fn kill_process_tree(&self) {
        if let Some(pid) = self.pid {
            let _ = crate::platform::signal_detached_process_group(pid, libc::SIGKILL);
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.armed {
            self.kill_process_tree();
            let _ = self.child.start_kill();
        }
    }
}

#[async_trait]
impl Tool for ZlsDiagnosticsTool {
    fn name(&self) -> &str {
        "zls_diagnostics"
    }

    fn description(&self) -> &str {
        "One local ZLS diagnostic snapshot; runs build.zig as you. Not proof of clean build."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["file_path", "workspace_root"],
            "properties": {
                "intent": super::intent_schema_property(),
                "file_path": {
                    "type": "string",
                    "description": "Saved .zig file. Relative paths resolve from the session working directory."
                },
                "workspace_root": {
                    "type": "string",
                    "description": "Trusted workspace root inside the session cwd that contains the file."
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_TIMEOUT_MS,
                    "description": "Protocol timeout in ms after spawn. Default 10000, max 30000."
                }
            }
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let params: ZlsDiagnosticsInput = serde_json::from_value(input)?;
        let request = validate_request(params, &ctx)?;
        if ctx
            .graceful_shutdown_signal
            .as_ref()
            .is_some_and(|signal| signal.is_set())
        {
            bail!("ZLS diagnostics interrupted before startup");
        }

        let started = Instant::now();
        let mut command = Command::new(&self.program);
        command
            .current_dir(&request.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .envs(self.env.iter().cloned());
        configure_process_group(&mut command);

        let child = command.spawn().map_err(|error| {
            anyhow!(
                "failed to start ZLS executable {:?}: {error}. install ZLS and ensure `zls` is on PATH",
                self.program
            )
        })?;
        let mut guard = ChildGuard::new(child);
        let (mut stdin, mut stdout) = guard.take_stdio()?;

        let run_result = {
            let protocol = run_protocol(&mut stdin, &mut stdout, &request);
            tokio::pin!(protocol);
            let timeout = tokio::time::sleep(request.timeout);
            tokio::pin!(timeout);
            tokio::select! {
                biased;
                _ = wait_for_interrupt(ctx.graceful_shutdown_signal.as_ref()) => {
                    Err(anyhow!("ZLS diagnostics interrupted"))
                }
                _ = &mut timeout => {
                    Err(anyhow!("ZLS diagnostics timed out after {} ms", request.timeout.as_millis()))
                }
                result = &mut protocol => result,
            }
        };
        drop(stdin);
        drop(stdout);

        let protocol = match run_result {
            Ok(protocol) => {
                guard.finish_after_exit().await?;
                protocol
            }
            Err(error) => {
                if let Err(cleanup) = guard.terminate_and_wait().await {
                    return Err(anyhow!("{error:#}; cleanup also failed: {cleanup:#}"));
                }
                return Err(error);
            }
        };

        let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let mut output = format_diagnostics(&request.display_path, &protocol.diagnostics);
        output.push_str(
            "\n\nSnapshot notice: ZLS provides no diagnostics-complete signal in this exchange. This incomplete snapshot is not proof of compile or build cleanliness.",
        );
        output.push_str(
            "\nExecution notice: this is not read-only or sandboxed. ZLS workspace code may execute with your user permissions; enable_build_on_save=false.",
        );
        let returned_count = protocol.diagnostics.entries.len();
        let reported_count = protocol.diagnostics.reported_count;
        Ok(ToolOutput::new(output)
            .with_title(format!("ZLS diagnostic snapshot: {}", request.display_path))
            .with_metadata(json!({
                "path": request.display_path,
                "diagnostic_snapshot": true,
                "snapshot_complete": false,
                "diagnostic_count": reported_count,
                "returned_diagnostic_count": returned_count,
                "diagnostics_truncated": reported_count > returned_count,
                "elapsed_ms": elapsed_ms,
                "server_name": protocol.server_name,
                "server_version": protocol.server_version,
            })))
    }
}

fn validate_request(input: ZlsDiagnosticsInput, ctx: &ToolContext) -> Result<ValidatedRequest> {
    let session_root = ctx
        .working_dir
        .as_ref()
        .ok_or_else(|| {
            anyhow!(
                "zls_diagnostics requires a harness-supplied session working directory; workspace trust cannot be inferred without one"
            )
        })?
        .canonicalize()
        .context("session working directory does not exist")?;
    if !session_root.is_dir() {
        bail!("session working directory is not a directory");
    }

    let requested_root = ctx
        .resolve_path(Path::new(&input.workspace_root))
        .canonicalize()
        .with_context(|| format!("workspace root does not exist: {}", input.workspace_root))?;
    if !requested_root.is_dir() {
        bail!(
            "workspace root is not a directory: {}",
            requested_root.display()
        );
    }
    if !requested_root.starts_with(&session_root) {
        bail!(
            "workspace root is outside the session working directory: {}",
            requested_root.display()
        );
    }

    let requested_file = ctx.resolve_path(Path::new(&input.file_path));
    let file = requested_file
        .canonicalize()
        .with_context(|| format!("Zig file does not exist: {}", requested_file.display()))?;
    if !file.starts_with(&requested_root) {
        bail!(
            "Zig file is outside the trusted workspace root: {}",
            file.display()
        );
    }
    if file.extension().and_then(|value| value.to_str()) != Some("zig") {
        bail!("zls_diagnostics requires a saved .zig file");
    }
    let source_file = std::fs::File::open(&file).context("failed to open saved Zig source")?;
    let metadata = source_file
        .metadata()
        .context("failed to inspect opened Zig file")?;
    if !metadata.is_file() {
        bail!("Zig path is not a regular file: {}", file.display());
    }
    if metadata.len() > MAX_SOURCE_BYTES as u64 {
        bail!("Zig source exceeds the 2 MiB limit");
    }
    let source = read_source_bounded(source_file)?;

    let root_uri = Url::from_directory_path(&requested_root)
        .map_err(|_| anyhow!("failed to convert workspace root to a file URI"))?
        .to_string();
    let file_uri = Url::from_file_path(&file)
        .map_err(|_| anyhow!("failed to convert Zig file to a file URI"))?
        .to_string();
    let display_path = file
        .strip_prefix(&requested_root)
        .unwrap_or(&file)
        .to_string_lossy()
        .into_owned();
    let timeout_ms = input
        .timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .min(MAX_TIMEOUT_MS);

    Ok(ValidatedRequest {
        root: requested_root,
        display_path,
        root_uri,
        file_uri,
        source,
        timeout: Duration::from_millis(timeout_ms.max(1)),
    })
}

fn read_source_bounded(reader: impl Read) -> Result<String> {
    let mut bytes = Vec::with_capacity(MAX_SOURCE_BYTES + 1);
    reader
        .take((MAX_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .context("failed to read saved Zig source")?;
    if bytes.len() > MAX_SOURCE_BYTES {
        bail!("Zig source exceeds the 2 MiB limit");
    }
    String::from_utf8(bytes).context("failed to read saved Zig source as UTF-8")
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

async fn wait_for_interrupt(signal: Option<&jcode_agent_runtime::InterruptSignal>) {
    match signal {
        Some(signal) => signal.notified().await,
        None => std::future::pending::<()>().await,
    }
}

async fn run_protocol(
    stdin: &mut ChildStdin,
    stdout: &mut ChildStdout,
    request: &ValidatedRequest,
) -> Result<ProtocolResult> {
    write_lsp_message(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": request.root_uri,
                "workspaceFolders": [{
                    "uri": request.root_uri,
                    "name": request.root.file_name().and_then(|name| name.to_str()).unwrap_or("workspace")
                }],
                "capabilities": {
                    "workspace": {"configuration": true},
                    "textDocument": {"publishDiagnostics": {"relatedInformation": true}}
                },
                "initializationOptions": zls_configuration()
            }
        }),
    )
    .await?;
    let initialize = wait_for_response(stdin, stdout, 1).await?;
    let server_name = initialize
        .pointer("/result/serverInfo/name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let server_version = initialize
        .pointer("/result/serverInfo/version")
        .and_then(Value::as_str)
        .map(str::to_string);

    write_lsp_message(
        stdin,
        &json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
    )
    .await?;
    write_lsp_message(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": request.file_uri,
                    "languageId": "zig",
                    "version": 1,
                    "text": request.source
                }
            }
        }),
    )
    .await?;

    let mut diagnostics = loop {
        let message = read_lsp_message(stdout).await?;
        if let Some(diagnostics) = diagnostics_from_message(&message, &request.file_uri)? {
            break diagnostics;
        }
        respond_to_server_request(stdin, &message).await?;
    };

    write_lsp_message(
        stdin,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}),
    )
    .await?;
    wait_for_shutdown_response(stdin, stdout, &request.file_uri, &mut diagnostics).await?;
    write_lsp_message(
        stdin,
        &json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    )
    .await?;
    stdin
        .shutdown()
        .await
        .context("failed to close ZLS stdin")?;

    Ok(ProtocolResult {
        diagnostics,
        server_name,
        server_version,
    })
}

fn zls_configuration() -> Value {
    json!({"enable_build_on_save": false})
}

async fn wait_for_response(
    stdin: &mut ChildStdin,
    stdout: &mut ChildStdout,
    expected_id: u64,
) -> Result<Value> {
    loop {
        let message = read_lsp_message(stdout).await?;
        if message.get("id").and_then(Value::as_u64) == Some(expected_id)
            && (message.get("result").is_some() || message.get("error").is_some())
        {
            if let Some(error) = message.get("error") {
                bail!("ZLS returned an error for request {expected_id}: {error}");
            }
            return Ok(message);
        }
        respond_to_server_request(stdin, &message).await?;
    }
}

async fn wait_for_shutdown_response(
    stdin: &mut ChildStdin,
    stdout: &mut ChildStdout,
    target_uri: &str,
    diagnostics: &mut DiagnosticBatch,
) -> Result<()> {
    loop {
        let message = read_lsp_message(stdout).await?;
        if message.get("id").and_then(Value::as_u64) == Some(2)
            && (message.get("result").is_some() || message.get("error").is_some())
        {
            if let Some(error) = message.get("error") {
                bail!("ZLS returned an error for shutdown: {error}");
            }
            return Ok(());
        }
        if let Some(latest) = diagnostics_from_message(&message, target_uri)? {
            *diagnostics = latest;
            continue;
        }
        respond_to_server_request(stdin, &message).await?;
    }
}

async fn respond_to_server_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &Value,
) -> Result<()> {
    let Some(id) = message.get("id") else {
        return Ok(());
    };
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return Ok(());
    };

    let response = match method {
        "workspace/configuration" => {
            let count = message
                .pointer("/params/items")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(1);
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": vec![zls_configuration(); count]
            })
        }
        "client/registerCapability" | "window/workDoneProgress/create" => {
            json!({"jsonrpc": "2.0", "id": id, "result": null})
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32601, "message": "method not supported by zls_diagnostics"}
        }),
    };
    write_lsp_message(writer, &response).await
}

fn diagnostics_from_message(message: &Value, target_uri: &str) -> Result<Option<DiagnosticBatch>> {
    if message.get("method").and_then(Value::as_str) != Some("textDocument/publishDiagnostics") {
        return Ok(None);
    }
    let params: PublishDiagnosticsParams = serde_json::from_value(
        message
            .get("params")
            .cloned()
            .ok_or_else(|| anyhow!("ZLS diagnostics notification omitted params"))?,
    )
    .context("invalid ZLS diagnostics notification")?;
    if params.uri != target_uri {
        return Ok(None);
    }

    let reported_count = params.diagnostics.len();
    let mut diagnostics: Vec<Diagnostic> = params
        .diagnostics
        .into_iter()
        .map(|raw| {
            let severity_rank = raw.severity.unwrap_or(5);
            let severity = match raw.severity {
                Some(1) => "error",
                Some(2) => "warning",
                Some(3) => "information",
                Some(4) => "hint",
                _ => "diagnostic",
            };
            let code = raw.code.and_then(|value| match value {
                Value::String(value) => Some(value),
                Value::Number(value) => Some(value.to_string()),
                _ => None,
            });
            Diagnostic {
                severity_rank,
                severity,
                line: raw.range.start.line.saturating_add(1),
                column: raw.range.start.character.saturating_add(1),
                source: raw.source,
                code,
                message: raw.message.split_whitespace().collect::<Vec<_>>().join(" "),
            }
        })
        .collect();
    diagnostics.sort_by(|left, right| {
        (
            left.severity_rank,
            left.line,
            left.column,
            left.source.as_deref(),
            left.code.as_deref(),
            left.message.as_str(),
        )
            .cmp(&(
                right.severity_rank,
                right.line,
                right.column,
                right.source.as_deref(),
                right.code.as_deref(),
                right.message.as_str(),
            ))
    });
    diagnostics.truncate(MAX_DIAGNOSTICS);
    Ok(Some(DiagnosticBatch {
        entries: diagnostics,
        reported_count,
    }))
}

fn format_diagnostics(display_path: &str, diagnostics: &DiagnosticBatch) -> String {
    if diagnostics.entries.is_empty() {
        return format!(
            "ZLS diagnostic snapshot for {display_path}: no diagnostics observed before shutdown completed"
        );
    }
    let mut output = if diagnostics.reported_count > diagnostics.entries.len() {
        format!(
            "ZLS diagnostic snapshot for {display_path}: showing {} of {} reported diagnostics",
            diagnostics.entries.len(),
            diagnostics.reported_count
        )
    } else {
        format!(
            "ZLS diagnostic snapshot for {display_path}: {} diagnostic{} observed",
            diagnostics.entries.len(),
            if diagnostics.entries.len() == 1 {
                ""
            } else {
                "s"
            }
        )
    };
    for diagnostic in &diagnostics.entries {
        output.push_str(&format!(
            "\n- {} {}:{}",
            diagnostic.severity, diagnostic.line, diagnostic.column
        ));
        match (&diagnostic.source, &diagnostic.code) {
            (Some(source), Some(code)) => output.push_str(&format!(" [{source}/{code}]")),
            (Some(source), None) => output.push_str(&format!(" [{source}]")),
            (None, Some(code)) => output.push_str(&format!(" [{code}]")),
            (None, None) => {}
        }
        output.push(' ');
        output.push_str(&diagnostic.message);
    }
    output
}

fn encode_lsp_message(message: &Value) -> Result<Vec<u8>> {
    let body = serde_json::to_vec(message)?;
    if body.len() > MAX_BODY_BYTES {
        bail!("outbound LSP frame exceeds the {MAX_BODY_BYTES}-byte body limit");
    }
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut frame = Vec::with_capacity(header.len() + body.len());
    frame.extend_from_slice(header.as_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

async fn write_lsp_message<W: AsyncWrite + Unpin>(writer: &mut W, message: &Value) -> Result<()> {
    writer
        .write_all(&encode_lsp_message(message)?)
        .await
        .context("failed to write an LSP frame")?;
    writer.flush().await.context("failed to flush an LSP frame")
}

async fn read_lsp_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Value> {
    let mut header = [0_u8; MAX_HEADER_BYTES];
    let mut header_len = 0;
    loop {
        if header_len == MAX_HEADER_BYTES {
            bail!("LSP header exceeds the {MAX_HEADER_BYTES}-byte limit");
        }
        let read = reader
            .read(&mut header[header_len..header_len + 1])
            .await
            .context("failed to read LSP header")?;
        if read == 0 {
            bail!("ZLS closed stdout before completing an LSP header");
        }
        header_len += read;
        if header[..header_len].ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let header_text =
        std::str::from_utf8(&header[..header_len]).context("ZLS sent a non-UTF-8 LSP header")?;
    let mut content_length = None;
    for line in header_text[..header_text.len() - 4].split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            bail!("ZLS sent a malformed LSP header line");
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some() {
                bail!("ZLS sent duplicate Content-Length headers");
            }
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .context("ZLS sent an invalid Content-Length")?,
            );
        }
    }
    let content_length = content_length.context("ZLS LSP frame omitted Content-Length")?;
    if content_length > MAX_BODY_BYTES {
        bail!("LSP body declaration {content_length} exceeds the {MAX_BODY_BYTES}-byte limit");
    }

    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .context("ZLS closed stdout before completing an LSP body")?;
    serde_json::from_slice(&body).context("ZLS sent malformed JSON")
}

#[cfg(test)]
#[path = "zls_tests.rs"]
mod zls_tests;
