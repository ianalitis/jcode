use super::*;

#[cfg(unix)]
#[tokio::test]
async fn registry_execute_pre_tool_hook_blocks_and_allows() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider).await;
    let temp = tempfile::TempDir::new().expect("temp dir");

    // Policy script: block bash calls whose input mentions "secret".
    let policy = temp.path().join("policy.sh");
    std::fs::write(
        &policy,
        "#!/bin/sh\ninput=$(cat)\ncase \"$input\" in\n  *secret*) echo \"no secrets\" >&2; exit 2 ;;\nesac\nexit 0\n",
    )
    .expect("write policy");
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o755))
        .expect("chmod policy");

    let prev = std::env::var_os("JCODE_HOOK_PRE_TOOL");
    crate::env::set_var("JCODE_HOOK_PRE_TOOL", policy.to_string_lossy().to_string());
    // jcode-base is compiled without cfg(test) here, so the config cache only
    // re-checks env every 500ms; force a reload so the hook is visible now.
    crate::config::invalidate_config_cache();

    let ctx = || ToolContext {
        session_id: "test-pre-tool-hook".to_string(),
        message_id: "test".to_string(),
        tool_call_id: "test".to_string(),
        working_dir: Some(std::env::temp_dir()),
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: ToolExecutionMode::Direct,
    };

    let blocked = registry
        .execute(
            "bash",
            serde_json::json!({
                "command": "echo secret"
            }),
            ctx(),
        )
        .await;
    let allowed = registry
        .execute(
            "bash",
            serde_json::json!({
                "command": "true"
            }),
            ctx(),
        )
        .await;

    match prev {
        Some(value) => crate::env::set_var("JCODE_HOOK_PRE_TOOL", value),
        None => crate::env::remove_var("JCODE_HOOK_PRE_TOOL"),
    }
    crate::config::invalidate_config_cache();

    let error = blocked.expect_err("pre_tool hook should block matching input");
    assert!(
        error.to_string().contains("no secrets"),
        "hook stderr should surface in the error: {error}"
    );
    assert!(allowed.is_ok(), "non-matching input should pass the gate");
}

#[cfg(unix)]
#[tokio::test]
async fn post_tool_feedback_hook_appends_stdout_to_tool_result() {
    use std::os::unix::fs::PermissionsExt;
    let _env_lock = crate::storage::lock_test_env();
    let dir = tempfile::tempdir().expect("temp dir");
    let target = dir.path().join("prose.md");
    std::fs::write(&target, "hello\n").expect("write target");
    let script = dir.path().join("feedback.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\ninput=$(cat)\necho \"call=$JCODE_HOOK_TOOL_CALL_ID tool=$JCODE_HOOK_TOOL_NAME\"\necho \"paths=$JCODE_HOOK_TOUCHED_PATHS\"\ncase \"$input\" in *prose.md*) echo stdin-ok ;; esac\nexit 1\n",
    )
    .expect("write script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let previous = std::env::var_os("JCODE_HOOK_POST_TOOL_FEEDBACK");
    crate::env::set_var("JCODE_HOOK_POST_TOOL_FEEDBACK", &script);
    crate::config::invalidate_config_cache();

    let registry = Registry::new(Arc::new(MockProvider) as Arc<dyn Provider>).await;
    let mut ctx = mcp_test_context(dir.path());
    ctx.tool_call_id = "call_feedback".to_string();
    let result = registry
        .execute("read", serde_json::json!({"file_path": "prose.md"}), ctx)
        .await;

    match previous {
        Some(value) => crate::env::set_var("JCODE_HOOK_POST_TOOL_FEEDBACK", value),
        None => crate::env::remove_var("JCODE_HOOK_POST_TOOL_FEEDBACK"),
    }
    crate::config::invalidate_config_cache();
    let output = result.expect("read should succeed").output;
    assert!(output.contains("hello"), "{output}");
    assert!(output.contains("[post_tool_feedback]"), "{output}");
    assert!(output.contains("call=call_feedback tool=read"), "{output}");
    assert!(
        output.contains(&format!("paths={}", target.display())),
        "{output}"
    );
    assert!(output.contains("stdin-ok"), "{output}");
}
