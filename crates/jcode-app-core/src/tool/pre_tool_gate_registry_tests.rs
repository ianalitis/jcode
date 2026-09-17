use super::*;
use serde_json::json;
use std::path::PathBuf;

struct MarkerTool {
    marker: PathBuf,
}

#[async_trait::async_trait]
impl Tool for MarkerTool {
    fn name(&self) -> &str {
        "marker"
    }

    fn description(&self) -> &str {
        "Writes a marker for pre-tool gate regression tests"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        std::fs::write(&self.marker, "executed")?;
        Ok(ToolOutput::new("executed"))
    }
}

fn test_context(tool_call_id: &str, working_dir: &std::path::Path) -> ToolContext {
    ToolContext {
        session_id: "pre-tool-registry-regression".to_string(),
        message_id: "message".to_string(),
        tool_call_id: tool_call_id.to_string(),
        working_dir: Some(working_dir.to_path_buf()),
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: ToolExecutionMode::Direct,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn registry_direct_and_batch_calls_cannot_run_tools_after_failed_gate() {
    use std::os::unix::fs::PermissionsExt;

    struct HookEnvReset {
        hook: Option<std::ffi::OsString>,
        timeout: Option<std::ffi::OsString>,
    }
    impl Drop for HookEnvReset {
        fn drop(&mut self) {
            match self.hook.take() {
                Some(value) => crate::env::set_var("JCODE_HOOK_PRE_TOOL", value),
                None => crate::env::remove_var("JCODE_HOOK_PRE_TOOL"),
            }
            match self.timeout.take() {
                Some(value) => crate::env::set_var("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS", value),
                None => crate::env::remove_var("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS"),
            }
            crate::config::invalidate_config_cache();
        }
    }

    let _guard = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let hook = temp.path().join("policy.sh");
    std::fs::write(
        &hook,
        "#!/bin/sh\nif [ \"$JCODE_HOOK_TOOL_NAME\" = batch ]; then cat > /dev/null; exit 0; fi\necho 'private policy diagnostics' >&2\nexit 7\n",
    )
    .expect("write policy hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("chmod policy hook");
    let _env = HookEnvReset {
        hook: std::env::var_os("JCODE_HOOK_PRE_TOOL"),
        timeout: std::env::var_os("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS"),
    };
    crate::env::set_var("JCODE_HOOK_PRE_TOOL", hook.as_os_str());
    crate::env::set_var("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS", "1000");
    crate::config::invalidate_config_cache();

    let registry = Registry::empty();
    let direct_marker = temp.path().join("direct-marker");
    let batch_marker = temp.path().join("batch-marker");
    {
        let mut tools = registry.tools.write().await;
        tools.insert(
            "direct_marker".to_string(),
            Arc::new(MarkerTool {
                marker: direct_marker.clone(),
            }),
        );
        tools.insert(
            "batch_marker".to_string(),
            Arc::new(MarkerTool {
                marker: batch_marker.clone(),
            }),
        );
        tools.insert(
            "batch".to_string(),
            Arc::new(batch::BatchTool::new(registry.downgrade())),
        );
    }

    let direct = registry
        .execute(
            "direct_marker",
            json!({}),
            test_context("direct", temp.path()),
        )
        .await;
    let direct_error = direct.expect_err("failed gate must block direct tool execution");
    let direct_error = direct_error.to_string();
    assert!(direct_error.contains("pre_tool hook infrastructure error"));
    assert!(!direct_error.contains("private policy diagnostics"));
    assert!(!direct_marker.exists(), "direct tool must not run");

    let batch = registry
        .execute(
            "batch",
            json!({
                "tool_calls": [{
                    "tool": "batch_marker",
                    "parameters": {}
                }]
            }),
            test_context("batch", temp.path()),
        )
        .await
        .expect("batch wrapper is explicitly allowed by the hook");
    assert!(batch.output.contains("1 failed"));
    assert!(batch.output.contains("pre_tool hook infrastructure error"));
    assert!(!batch.output.contains("private policy diagnostics"));
    assert!(!batch_marker.exists(), "batch subcall tool must not run");
}
