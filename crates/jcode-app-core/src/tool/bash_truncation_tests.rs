//! Command-output truncation tests, kept out of `bash.rs` so the tool file
//! stays inside its size budget.

use super::MAX_OUTPUT_LEN;
#[cfg(any(windows, unix))]
use super::build_shell_command;
use super::format_command_output;

struct SpillHome {
    previous: Option<std::ffi::OsString>,
    dir: tempfile::TempDir,
}

impl SpillHome {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let previous = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_HOME", dir.path());
        Self { previous, dir }
    }
}

impl Drop for SpillHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => crate::env::set_var("JCODE_HOME", value),
            None => crate::env::remove_var("JCODE_HOME"),
        }
    }
}

#[test]
fn format_command_output_truncates_on_utf8_boundary() {
    let _lock = crate::storage::lock_test_env();
    let _home = SpillHome::new();
    let input = format!("{}é", "a".repeat(29_999));
    let output = format_command_output("session_a", input, None);
    assert!(output.starts_with(&"a".repeat(29_999)));
    assert!(output.contains("output truncated"), "{output}");
    assert!(
        output.contains("full output saved at") || output.ends_with("output truncated)"),
        "a spill names the file; a failed spill keeps the plain marker"
    );
}

#[test]
fn truncated_command_output_stays_reachable_on_disk() {
    let _lock = crate::storage::lock_test_env();
    let _home = SpillHome::new();
    let input = "b".repeat(MAX_OUTPUT_LEN + 5_000);

    let output = format_command_output("session_a", input.clone(), None);

    let saved = output
        .split("full output saved at ")
        .nth(1)
        .and_then(|rest| rest.split(" —").next())
        .expect("notice names the spilled path");
    assert_eq!(
        std::fs::read_to_string(saved.trim()).expect("read spill"),
        input,
        "the truncated tail must be recoverable from the named file"
    );
}

#[test]
fn short_command_output_is_never_spilled() {
    let _lock = crate::storage::lock_test_env();
    let home = SpillHome::new();
    let output = format_command_output("session_a", "small output".to_string(), None);
    assert_eq!(output, "small output");
    assert!(!home.dir.path().join("tool-output").exists());
}

#[cfg(windows)]
#[tokio::test]
async fn build_shell_command_uses_cmd_and_executes_command() {
    let output = build_shell_command("echo hello-from-cmd")
        .output()
        .await
        .expect("run cmd command");
    assert!(output.status.success(), "cmd command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_ascii_lowercase().contains("hello-from-cmd"),
        "unexpected stdout: {}",
        stdout
    );

    let probe_path = std::env::temp_dir().join(format!(
        "jcode-cmd-quoting-probe-{}.cmd",
        std::process::id()
    ));
    std::fs::write(
        &probe_path,
        concat!(
            "@echo off\r\n",
            "if \"%~1\"==\"text with spaces\" if \"%~2\"==\"\" (\r\n",
            "  echo quoted-argument-ok\r\n",
            "  exit /b 0\r\n",
            ")\r\n",
            "echo first=[%~1] second=[%~2]\r\n",
            "exit /b 1\r\n",
        ),
    )
    .expect("write cmd quoting probe");

    let quoted_command = format!("call \"{}\" \"text with spaces\"", probe_path.display());
    let quoted_output = build_shell_command(&quoted_command)
        .output()
        .await
        .expect("run cmd quoting probe");
    let _ = std::fs::remove_file(&probe_path);
    let quoted_stdout = String::from_utf8_lossy(&quoted_output.stdout);
    let quoted_stderr = String::from_utf8_lossy(&quoted_output.stderr);
    assert!(
        quoted_output.status.success(),
        "quoted argument should remain one child-process argument; stdout={quoted_stdout:?} stderr={quoted_stderr:?}"
    );
    assert!(
        quoted_stdout.contains("quoted-argument-ok"),
        "unexpected quoted-command stdout: {quoted_stdout}"
    );
}

#[cfg(unix)]
#[test]
fn build_shell_command_uses_disk_backed_scratch_directory() {
    let _env_lock = crate::storage::lock_test_env();
    let mut runtime = tokio::runtime::Builder::new_current_thread();
    runtime.enable_all();
    runtime
        .build()
        .expect("current-thread runtime")
        .block_on(async {
            let command = "printf '%s\\n%s\\n' \"$TMPDIR\" \"$JCODE_SCRATCH_DIR\"";
            let expected = super::tool_scratch_dir().expect("jcode scratch directory");
            let output = build_shell_command(command)
                .output()
                .await
                .expect("run bash command");
            let expected_output = format!("{0}\n{0}\n", expected.display());
            assert!(output.status.success() && output.stdout == expected_output.as_bytes());
            assert!(expected.is_dir());
        });
}
