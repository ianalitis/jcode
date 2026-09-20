use super::*;
use jcode_agent_runtime::InterruptSignal;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncWriteExt, duplex};

fn context(root: &Path, signal: Option<InterruptSignal>) -> ToolContext {
    ToolContext {
        session_id: "zls-diagnostics-test".to_string(),
        message_id: "message".to_string(),
        tool_call_id: "call".to_string(),
        working_dir: Some(root.to_path_buf()),
        stdin_request_tx: None,
        graceful_shutdown_signal: signal,
        execution_mode: super::super::ToolExecutionMode::Direct,
    }
}

fn write_zig(root: &Path, relative: &str, source: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, source).unwrap();
    path
}

struct CountingReader {
    remaining: usize,
    bytes_read: Arc<AtomicUsize>,
}

impl std::io::Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = buffer.len().min(self.remaining);
        buffer[..count].fill(b'x');
        self.remaining -= count;
        self.bytes_read.fetch_add(count, Ordering::SeqCst);
        Ok(count)
    }
}

#[test]
fn zls_diagnostics_source_reader_stops_at_limit_plus_one() {
    let bytes_read = Arc::new(AtomicUsize::new(0));
    let error = read_source_bounded(CountingReader {
        remaining: MAX_SOURCE_BYTES + 4096,
        bytes_read: Arc::clone(&bytes_read),
    })
    .unwrap_err()
    .to_string();

    assert!(error.contains("2 MiB"), "error was: {error}");
    assert_eq!(bytes_read.load(Ordering::SeqCst), MAX_SOURCE_BYTES + 1);
}

#[test]
fn zls_diagnostics_schema_exposes_only_bounded_diagnostic_inputs() {
    let tool = ZlsDiagnosticsTool::new();
    let definition = tool.to_definition();
    let schema = definition.input_schema;
    let properties = schema["properties"].as_object().unwrap();
    let required = schema["required"].as_array().unwrap();

    assert!(required.iter().any(|value| value == "file_path"));
    assert!(required.iter().any(|value| value == "workspace_root"));
    assert_eq!(properties["timeout_ms"]["maximum"], MAX_TIMEOUT_MS);
    for forbidden in [
        "server",
        "command",
        "args",
        "content",
        "trusted",
        "allow_build_execution",
    ] {
        assert!(
            !properties.contains_key(forbidden),
            "unexpected {forbidden} input"
        );
    }
    // Descriptions are prompt-visible and capped (see tool::tests token caps),
    // so assert the safety semantics, not the prose.
    assert!(tool.description().contains("diagnostic snapshot"));
    assert!(tool.description().contains("Not proof"));
    assert!(tool.description().contains("runs build.zig as you"));
    let timeout_description = properties["timeout_ms"]["description"].as_str().unwrap();
    assert!(timeout_description.contains("after spawn"));
    assert!(timeout_description.contains("max 30000"));
}

#[test]
fn zls_diagnostics_path_validation_requires_canonical_workspace_containment() {
    let workspace = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let good = write_zig(workspace.path(), "src/main.zig", "pub fn main() void {}\n");
    let outside_file = write_zig(outside.path(), "outside.zig", "const x = 1;\n");

    let validated = validate_request(
        ZlsDiagnosticsInput {
            file_path: good.to_string_lossy().into_owned(),
            workspace_root: workspace.path().to_string_lossy().into_owned(),
            timeout_ms: None,
        },
        &context(workspace.path(), None),
    )
    .unwrap();
    assert_eq!(validated.display_path, "src/main.zig");
    assert_eq!(validated.root, workspace.path().canonicalize().unwrap());

    let no_root = ToolContext {
        working_dir: None,
        ..context(workspace.path(), None)
    };
    assert!(
        validate_request(
            ZlsDiagnosticsInput {
                file_path: good.to_string_lossy().into_owned(),
                workspace_root: workspace.path().to_string_lossy().into_owned(),
                timeout_ms: None,
            },
            &no_root,
        )
        .unwrap_err()
        .to_string()
        .contains("session working directory")
    );

    let outside_error = validate_request(
        ZlsDiagnosticsInput {
            file_path: outside_file.to_string_lossy().into_owned(),
            workspace_root: workspace.path().to_string_lossy().into_owned(),
            timeout_ms: None,
        },
        &context(workspace.path(), None),
    )
    .unwrap_err()
    .to_string();
    assert!(outside_error.contains("outside the trusted workspace root"));

    let root_error = validate_request(
        ZlsDiagnosticsInput {
            file_path: outside_file.to_string_lossy().into_owned(),
            workspace_root: outside.path().to_string_lossy().into_owned(),
            timeout_ms: None,
        },
        &context(workspace.path(), None),
    )
    .unwrap_err()
    .to_string();
    assert!(root_error.contains("workspace root is outside the session working directory"));

    let capped = validate_request(
        ZlsDiagnosticsInput {
            file_path: good.to_string_lossy().into_owned(),
            workspace_root: workspace.path().to_string_lossy().into_owned(),
            timeout_ms: Some(MAX_TIMEOUT_MS + 1),
        },
        &context(workspace.path(), None),
    )
    .unwrap();
    assert_eq!(capped.timeout, Duration::from_millis(MAX_TIMEOUT_MS));

    for (relative, source, expected) in [
        ("missing.zig", None, "does not exist"),
        ("not-zig.txt", Some("text"), "saved .zig file"),
    ] {
        let path = workspace.path().join(relative);
        if let Some(source) = source {
            std::fs::write(&path, source).unwrap();
        }
        let error = validate_request(
            ZlsDiagnosticsInput {
                file_path: path.to_string_lossy().into_owned(),
                workspace_root: workspace.path().to_string_lossy().into_owned(),
                timeout_ms: None,
            },
            &context(workspace.path(), None),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(expected), "error was: {error}");
    }

    let directory = workspace.path().join("directory.zig");
    std::fs::create_dir(&directory).unwrap();
    assert!(
        validate_request(
            ZlsDiagnosticsInput {
                file_path: directory.to_string_lossy().into_owned(),
                workspace_root: workspace.path().to_string_lossy().into_owned(),
                timeout_ms: None,
            },
            &context(workspace.path(), None),
        )
        .unwrap_err()
        .to_string()
        .contains("regular file")
    );

    let oversized = workspace.path().join("oversized.zig");
    let file = std::fs::File::create(&oversized).unwrap();
    file.set_len((MAX_SOURCE_BYTES + 1) as u64).unwrap();
    assert!(
        validate_request(
            ZlsDiagnosticsInput {
                file_path: oversized.to_string_lossy().into_owned(),
                workspace_root: workspace.path().to_string_lossy().into_owned(),
                timeout_ms: None,
            },
            &context(workspace.path(), None),
        )
        .unwrap_err()
        .to_string()
        .contains("2 MiB")
    );
}

#[cfg(unix)]
#[test]
fn zls_diagnostics_path_validation_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let workspace = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let outside_file = write_zig(outside.path(), "outside.zig", "const x = 1;\n");
    let link = workspace.path().join("linked.zig");
    symlink(&outside_file, &link).unwrap();

    let error = validate_request(
        ZlsDiagnosticsInput {
            file_path: link.to_string_lossy().into_owned(),
            workspace_root: workspace.path().to_string_lossy().into_owned(),
            timeout_ms: None,
        },
        &context(workspace.path(), None),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("outside the trusted workspace root"));
}

#[tokio::test]
async fn zls_diagnostics_frames_support_fragmentation_and_consecutive_messages() {
    let first = json!({"jsonrpc": "2.0", "id": 1, "result": {}});
    let second = json!({"jsonrpc": "2.0", "method": "initialized", "params": {}});
    let mut bytes = encode_lsp_message(&first).unwrap();
    bytes.extend(encode_lsp_message(&second).unwrap());

    let (mut writer, mut reader) = duplex(bytes.len() + 8);
    let task = tokio::spawn(async move {
        for chunk in bytes.chunks(3) {
            writer.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }
    });

    assert_eq!(read_lsp_message(&mut reader).await.unwrap(), first);
    assert_eq!(read_lsp_message(&mut reader).await.unwrap(), second);
    task.await.unwrap();
}

#[tokio::test]
async fn zls_diagnostics_rejects_oversized_frames_before_body_allocation() {
    let (mut writer, mut reader) = duplex(256);
    let task = tokio::spawn(async move {
        for fragment in [b"Content-Len".as_slice(), b"gth: 99999999\r\n\r\n"] {
            writer.write_all(fragment).await.unwrap();
        }
    });

    let error = read_lsp_message(&mut reader).await.unwrap_err().to_string();
    assert!(error.contains("exceeds"), "error was: {error}");
    task.await.unwrap();

    let (mut writer, mut reader) = duplex(MAX_HEADER_BYTES + 32);
    let task = tokio::spawn(async move {
        writer
            .write_all(&vec![b'x'; MAX_HEADER_BYTES + 1])
            .await
            .unwrap();
    });
    let error = read_lsp_message(&mut reader).await.unwrap_err().to_string();
    assert!(error.contains("header exceeds"), "error was: {error}");
    task.await.unwrap();
}

#[test]
fn zls_diagnostics_filters_target_uri_and_formats_stably() {
    let target = "file:///workspace/main.zig";
    assert!(
        diagnostics_from_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": {"uri": "file:///workspace/other.zig", "diagnostics": []}
            }),
            target,
        )
        .unwrap()
        .is_none()
    );

    let diagnostics = diagnostics_from_message(
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": target,
                "diagnostics": [
                    {
                        "range": {"start": {"line": 2, "character": 18}, "end": {"line": 2, "character": 19}},
                        "severity": 1,
                        "code": "ast-check",
                        "source": "zls",
                        "message": "expected expression, found ';'"
                    },
                    {
                        "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}},
                        "severity": 2,
                        "message": "warning"
                    }
                ]
            }
        }),
        target,
    )
    .unwrap()
    .unwrap();
    let rendered = format_diagnostics("src/main.zig", &diagnostics);
    assert!(rendered.contains("- error 3:19 [zls/ast-check] expected expression, found ';'"));
    assert!(rendered.contains("- warning 1:1 warning"));
    assert!(rendered.find("error 3:19").unwrap() < rendered.find("warning 1:1").unwrap());
}

#[test]
fn zls_diagnostics_sorts_all_diagnostics_before_the_hundred_item_cap() {
    let target = "file:///workspace/main.zig";
    let mut diagnostics: Vec<Value> = (0..MAX_DIAGNOSTICS)
        .map(|line| {
            json!({
                "range": {
                    "start": {"line": line, "character": 0},
                    "end": {"line": line, "character": 1}
                },
                "severity": 4,
                "message": format!("hint {line}")
            })
        })
        .collect();
    diagnostics.push(json!({
        "range": {
            "start": {"line": 999, "character": 0},
            "end": {"line": 999, "character": 1}
        },
        "severity": 1,
        "message": "late error"
    }));
    let parsed = diagnostics_from_message(
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": target, "diagnostics": diagnostics}
        }),
        target,
    )
    .unwrap()
    .unwrap();
    assert_eq!(parsed.entries.len(), MAX_DIAGNOSTICS);
    assert_eq!(parsed.reported_count, MAX_DIAGNOSTICS + 1);
    assert_eq!(parsed.entries[0].message, "late error");
    let rendered = format_diagnostics("src/main.zig", &parsed);
    assert!(
        rendered.contains("showing 100 of 101 reported diagnostics"),
        "output was: {rendered}"
    );
}

#[cfg(unix)]
fn fake_server(workspace: &Path, mode: &str) -> (ZlsDiagnosticsTool, PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let script = workspace.join("fake-zls.py");
    let pid_file = workspace.join("pids.txt");
    let exit_file = workspace.join("exited.txt");
    std::fs::write(
        &script,
        r#"#!/usr/bin/env python3
import json, os, subprocess, sys, time

def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            raise SystemExit(20)
        if line == b"\r\n":
            break
        key, value = line.decode().split(":", 1)
        headers[key.lower()] = value.strip()
    return json.loads(sys.stdin.buffer.read(int(headers["content-length"])))

def write_message(value):
    body = json.dumps(value, separators=(",", ":")).encode()
    sys.stdout.buffer.write(b"Content-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body)
    sys.stdout.buffer.flush()

mode = os.environ["FAKE_ZLS_MODE"]
pid_file = os.environ["FAKE_ZLS_PID_FILE"]
exit_file = os.environ["FAKE_ZLS_EXIT_FILE"]

if mode in ("hang", "cancel", "drop"):
    descendant = subprocess.Popen(["sleep", "60"])
    with open(pid_file, "w") as handle:
        handle.write(f"{os.getpid()}\n{descendant.pid}\n")
    time.sleep(60)
    raise SystemExit(21)

initialize = read_message()
write_message({"jsonrpc":"2.0","id":initialize["id"],"result":{"capabilities":{},"serverInfo":{"name":"fake-zls","version":"0.test"}}})
read_message()
did_open = read_message()
uri = did_open["params"]["textDocument"]["uri"]

if mode == "normal_descendant":
    descendant = subprocess.Popen(["sleep", "60"])
    with open(pid_file, "w") as handle:
        handle.write(f"{os.getpid()}\n{descendant.pid}\n")

if mode == "oversized":
    with open(pid_file, "w") as handle:
        handle.write(f"{os.getpid()}\n")
    sys.stdout.buffer.write(b"Content-Length: 99999999\r\n\r\n")
    sys.stdout.buffer.flush()
    time.sleep(60)
    raise SystemExit(22)

write_message({"jsonrpc":"2.0","id":99,"method":"workspace/configuration","params":{"items":[{"section":"zls"}]}})
configuration = read_message()
assert configuration["result"][0]["enable_build_on_save"] is False
write_message({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///unrelated.zig","diagnostics":[]}})
diagnostics = [] if mode in ("clean", "empty_then_error") else [{
    "range":{"start":{"line":2,"character":18},"end":{"line":2,"character":19}},
    "severity":1,"code":"ast-check","source":"fake","message":"expected expression"
}]
write_message({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":diagnostics}})
shutdown = read_message()
if mode == "empty_then_error":
    write_message({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":[{
        "range":{"start":{"line":4,"character":2},"end":{"line":4,"character":3}},
        "severity":1,"source":"fake","message":"late error after empty snapshot"
    }]}})
write_message({"jsonrpc":"2.0","id":shutdown["id"],"result":None})
exit_message = read_message()
assert exit_message["method"] == "exit"
with open(exit_file, "w") as handle:
    handle.write("exited\n")
if mode == "exit23":
    raise SystemExit(23)
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script, permissions).unwrap();

    let tool = ZlsDiagnosticsTool::with_test_command(
        script,
        vec![
            ("FAKE_ZLS_MODE".into(), mode.into()),
            (
                "FAKE_ZLS_PID_FILE".into(),
                pid_file.clone().into_os_string(),
            ),
            (
                "FAKE_ZLS_EXIT_FILE".into(),
                exit_file.clone().into_os_string(),
            ),
        ],
    );
    (tool, pid_file, exit_file)
}

#[cfg(unix)]
async fn wait_for_path(path: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("path should appear");
}

#[cfg(unix)]
async fn assert_pids_stop(pid_file: &Path) {
    let pids: Vec<u32> = std::fs::read_to_string(pid_file)
        .unwrap()
        .lines()
        .map(|line| line.parse().unwrap())
        .collect();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if pids
                .iter()
                .all(|pid| !crate::platform::is_process_running(*pid))
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("ZLS process tree survived cleanup: {pids:?}"));
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_fake_server_handles_requests_and_reaps_cleanly() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(
        workspace.path(),
        "src/main.zig",
        "pub fn main() void {\n    const value: u8 = ;\n    _ = value;\n}\n",
    );
    let (tool, pid_file, exit_file) = fake_server(workspace.path(), "normal_descendant");
    let output = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path(), "timeout_ms": 5_000}),
            context(workspace.path(), None),
        )
        .await
        .unwrap();

    assert!(
        output
            .output
            .contains("ZLS diagnostic snapshot for src/main.zig: 1")
    );
    assert!(
        output
            .output
            .contains("workspace code may execute with your user permissions")
    );
    assert_eq!(output.metadata.as_ref().unwrap()["server_name"], "fake-zls");
    assert!(
        exit_file.exists(),
        "fake server should receive shutdown and exit"
    );
    assert_pids_stop(&pid_file).await;
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_fake_server_reports_clean_file_explicitly() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let (tool, _pid_file, _) = fake_server(workspace.path(), "clean");
    let output = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap();
    assert!(
        output
            .output
            .contains("no diagnostics observed before shutdown completed"),
        "output was: {}",
        output.output
    );
    assert!(
        output
            .output
            .contains("not proof of compile or build cleanliness")
    );
    assert_eq!(
        output.metadata.as_ref().unwrap()["snapshot_complete"],
        false
    );
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_retains_late_publication_before_shutdown_response() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let (tool, _pid_file, _) = fake_server(workspace.path(), "empty_then_error");
    let output = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap();

    assert!(
        output.output.contains("late error after empty snapshot"),
        "output was: {}",
        output.output
    );
    assert!(!output.output.contains("no diagnostics observed"));
    assert_eq!(output.metadata.as_ref().unwrap()["diagnostic_count"], 1);
    assert_eq!(
        output.metadata.as_ref().unwrap()["snapshot_complete"],
        false
    );
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_rejects_nonzero_orderly_exit() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let (tool, _pid_file, exit_file) = fake_server(workspace.path(), "exit23");
    let error = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("23"), "error was: {error}");
    assert!(error.contains("unsuccessfully"), "error was: {error}");
    assert!(exit_file.exists(), "fake server should reach orderly exit");
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_timeout_kills_descendants_instead_of_detaching() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let (tool, pid_file, _) = fake_server(workspace.path(), "hang");
    let error = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path(), "timeout_ms": 5_000}),
            context(workspace.path(), None),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("timed out"), "error was: {error}");
    wait_for_path(&pid_file).await;
    assert_pids_stop(&pid_file).await;
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_oversized_server_frame_kills_the_process() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let (tool, pid_file, _) = fake_server(workspace.path(), "oversized");
    let error = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("exceeds"), "error was: {error}");
    wait_for_path(&pid_file).await;
    assert_pids_stop(&pid_file).await;
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_interrupt_preserves_signal_and_reaps_descendants() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let signal = InterruptSignal::new();
    let (tool, pid_file, _) = fake_server(workspace.path(), "cancel");
    let ctx = context(workspace.path(), Some(signal.clone()));
    let workspace_root = workspace.path().to_path_buf();
    let task = tokio::spawn(async move {
        tool.execute(
            json!({"file_path": file, "workspace_root": workspace_root}),
            ctx,
        )
        .await
    });
    wait_for_path(&pid_file).await;
    signal.fire();
    let error = task.await.unwrap().unwrap_err().to_string();
    assert!(error.contains("interrupted"), "error was: {error}");
    assert!(signal.is_set(), "the tool must not reset the shared signal");
    assert_pids_stop(&pid_file).await;
}

#[cfg(unix)]
#[tokio::test]
async fn zls_diagnostics_dropped_future_kills_descendants() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let (tool, pid_file, _) = fake_server(workspace.path(), "drop");
    let ctx = context(workspace.path(), None);
    let workspace_root = workspace.path().to_path_buf();
    let task = tokio::spawn(async move {
        tool.execute(
            json!({"file_path": file, "workspace_root": workspace_root}),
            ctx,
        )
        .await
    });
    wait_for_path(&pid_file).await;
    task.abort();
    let _ = task.await;
    assert_pids_stop(&pid_file).await;
}

#[tokio::test]
async fn zls_diagnostics_missing_executable_is_actionable() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(workspace.path(), "main.zig", "pub fn main() void {}\n");
    let tool =
        ZlsDiagnosticsTool::with_test_command(workspace.path().join("missing-zls"), Vec::new());
    let error = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("install ZLS"), "error was: {error}");
    assert!(!error.contains("fallback"));
}

#[tokio::test]
#[ignore = "requires installed ZLS 0.16.0 and may evaluate trusted scratch build metadata"]
async fn live_zls_reports_then_clears_syntax_diagnostic() {
    let workspace = TempDir::new().unwrap();
    let file = write_zig(
        workspace.path(),
        "src/main.zig",
        "pub fn main() void {\n    const value: u8 = ;\n    _ = value;\n}\n",
    );
    let tool = ZlsDiagnosticsTool::new();
    let bad = tool
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap();
    assert!(
        bad.output
            .contains("ZLS diagnostic snapshot for src/main.zig:")
    );
    assert!(
        !bad.output.contains("no diagnostics observed"),
        "output was: {}",
        bad.output
    );
    assert!(bad.output.contains("2:"), "output was: {}", bad.output);

    std::fs::write(
        &file,
        "pub fn main() void {\n    const value: u8 = 1;\n    _ = value;\n}\n",
    )
    .unwrap();
    let clean = ZlsDiagnosticsTool::new()
        .execute(
            json!({"file_path": file, "workspace_root": workspace.path()}),
            context(workspace.path(), None),
        )
        .await
        .unwrap();
    assert!(
        clean
            .output
            .contains("no diagnostics observed before shutdown completed"),
        "output was: {}",
        clean.output
    );
    assert!(
        clean
            .output
            .contains("not proof of compile or build cleanliness")
    );
}
