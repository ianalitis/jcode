use super::*;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const MISSING_POLICY_ERROR: &str = "Agent tool execution is not authorized for this session";

struct HookEnvReset {
    hook: Option<std::ffi::OsString>,
    timeout: Option<std::ffi::OsString>,
    disabled: Option<std::ffi::OsString>,
}

impl HookEnvReset {
    fn disabled() -> Self {
        let reset = Self::capture();
        crate::env::set_var("JCODE_HOOKS_DISABLED", "1");
        crate::env::remove_var("JCODE_HOOK_PRE_TOOL");
        crate::config::invalidate_config_cache();
        reset
    }

    #[cfg(unix)]
    fn with_pre_tool(hook: &std::path::Path) -> Self {
        let reset = Self::capture();
        crate::env::remove_var("JCODE_HOOKS_DISABLED");
        crate::env::set_var("JCODE_HOOK_PRE_TOOL", hook.as_os_str());
        crate::env::set_var("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS", "3000");
        crate::config::invalidate_config_cache();
        reset
    }

    fn capture() -> Self {
        Self {
            hook: std::env::var_os("JCODE_HOOK_PRE_TOOL"),
            timeout: std::env::var_os("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS"),
            disabled: std::env::var_os("JCODE_HOOKS_DISABLED"),
        }
    }
}

impl Drop for HookEnvReset {
    fn drop(&mut self) {
        for (name, value) in [
            ("JCODE_HOOK_PRE_TOOL", self.hook.take()),
            ("JCODE_HOOK_PRE_TOOL_TIMEOUT_MS", self.timeout.take()),
            ("JCODE_HOOKS_DISABLED", self.disabled.take()),
        ] {
            match value {
                Some(value) => crate::env::set_var(name, value),
                None => crate::env::remove_var(name),
            }
        }
        crate::config::invalidate_config_cache();
    }
}

struct MarkerTool {
    name: &'static str,
    effects: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Tool for MarkerTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Records a deterministic test-only effect"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        self.effects.fetch_add(1, Ordering::SeqCst);
        Ok(ToolOutput::new("executed"))
    }
}

fn context(session_id: &str, mode: ToolExecutionMode) -> ToolContext {
    ToolContext {
        session_id: session_id.to_string(),
        message_id: "message".to_string(),
        tool_call_id: format!("call-{session_id}"),
        working_dir: None,
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: mode,
    }
}

async fn marker_registry(name: &'static str) -> (Registry, Arc<AtomicUsize>) {
    let registry = Registry::empty();
    let effects = Arc::new(AtomicUsize::new(0));
    registry
        .register(
            name.to_string(),
            Arc::new(MarkerTool {
                name,
                effects: Arc::clone(&effects),
            }),
        )
        .await;
    (registry, effects)
}

struct PolicyReset(&'static str);

impl Drop for PolicyReset {
    fn drop(&mut self) {
        clear_session_tool_policy(self.0);
    }
}

fn install_policy(
    session_id: &'static str,
    allowed_tools: Option<HashSet<String>>,
    disabled_tools: HashSet<String>,
) -> PolicyReset {
    set_session_tool_policy(session_id, allowed_tools, disabled_tools);
    PolicyReset(session_id)
}

#[tokio::test]
async fn agent_turn_requires_a_registered_policy_before_tool_lookup_or_effects() {
    let _env_lock = crate::storage::lock_test_env();
    let _hooks = HookEnvReset::disabled();
    let (registry, effects) = marker_registry("marker").await;
    let input = json!({"private": "must-not-appear"});

    let unknown_error = registry
        .execute(
            "unknown-private-tool",
            input.clone(),
            context("missing-unknown-policy", ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("missing policy must deny before unknown-tool catalog lookup")
        .to_string();
    assert_eq!(unknown_error, MISSING_POLICY_ERROR);
    assert!(!unknown_error.contains("unknown-private-tool"));
    assert!(!unknown_error.contains("Available tools"));

    let error = registry
        .execute(
            "marker",
            input,
            context("missing-agent-policy", ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("missing AgentTurn policy must fail closed")
        .to_string();

    assert_eq!(error, MISSING_POLICY_ERROR);
    assert!(!error.contains("marker"));
    assert!(!error.contains("must-not-appear"));
    assert!(!error.contains("Available tools"));
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn removed_agent_turn_policy_blocks_effects() {
    let _env_lock = crate::storage::lock_test_env();
    let _hooks = HookEnvReset::disabled();
    const SESSION: &str = "removed-agent-policy";
    let (registry, effects) = marker_registry("marker").await;
    let registration = register_session_tool_policy(SESSION, None, HashSet::new());
    drop(registration);

    let error = registry
        .execute(
            "marker",
            json!({}),
            context(SESSION, ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("removed AgentTurn policy must fail closed")
        .to_string();

    assert_eq!(error, MISSING_POLICY_ERROR);
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn registered_unrestricted_agent_turn_and_unregistered_direct_dispatch_succeed() {
    let _env_lock = crate::storage::lock_test_env();
    let _hooks = HookEnvReset::disabled();
    const SESSION: &str = "registered-unrestricted-policy";
    let (registry, effects) = marker_registry("marker").await;
    let _policy = install_policy(SESSION, None, HashSet::new());

    registry
        .execute(
            "marker",
            json!({}),
            context(SESSION, ToolExecutionMode::AgentTurn),
        )
        .await
        .expect("registered unrestricted AgentTurn policy should permit dispatch");
    registry
        .execute(
            "marker",
            json!({}),
            context("trusted-direct-without-policy", ToolExecutionMode::Direct),
        )
        .await
        .expect("trusted Direct dispatch should remain compatible without a policy");

    assert_eq!(effects.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn registered_empty_allowlist_and_alias_disabled_policy_still_deny() {
    let _env_lock = crate::storage::lock_test_env();
    let _hooks = HookEnvReset::disabled();
    const EMPTY_SESSION: &str = "empty-agent-policy";
    const DISABLED_SESSION: &str = "disabled-alias-agent-policy";
    let (registry, effects) = marker_registry("read").await;
    let _empty = install_policy(EMPTY_SESSION, Some(HashSet::new()), HashSet::new());

    let empty_error = registry
        .execute(
            "read",
            json!({}),
            context(EMPTY_SESSION, ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("registered empty allowlist must deny")
        .to_string();
    assert_eq!(empty_error, "Tool 'read' is not allowed");

    let _disabled = install_policy(DISABLED_SESSION, None, HashSet::from(["read".to_string()]));
    let alias_error = registry
        .execute(
            "file_read",
            json!({}),
            context(DISABLED_SESSION, ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("disabled canonical tool must also deny its alias")
        .to_string();
    assert_eq!(alias_error, "Tool 'read' is disabled");
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn fixed_mcp_surfaces_deny_missing_agent_context_without_starting_a_server() {
    let _env_lock = crate::storage::lock_test_env();
    let _hooks = HookEnvReset::disabled();
    let manager = Arc::new(tokio::sync::RwLock::new(
        crate::mcp::McpManager::with_config(crate::mcp::McpConfig::default()),
    ));
    let call = mcp::McpCallTool::new(Arc::clone(&manager));
    let search = mcp::McpSearchTool::new(manager);

    let call_error = call
        .execute(
            json!({"server": "unconfigured", "tool": "noop", "arguments": {}}),
            context("missing-mcp-call-policy", ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("missing AgentTurn policy must block before MCP manager dispatch")
        .to_string();
    assert_eq!(call_error, MISSING_POLICY_ERROR);

    let search_error = search
        .execute(
            json!({}),
            context("missing-mcp-search-policy", ToolExecutionMode::AgentTurn),
        )
        .await
        .expect_err("missing AgentTurn policy must block deferred MCP search")
        .to_string();
    assert_eq!(search_error, MISSING_POLICY_ERROR);
}

#[tokio::test]
async fn mcp_call_rechecks_policy_after_waiting_for_the_manager_lock() {
    let _env_lock = crate::storage::lock_test_env();
    let _hooks = HookEnvReset::disabled();
    const SESSION: &str = "mcp-policy-removal-during-lock-wait";
    let manager = Arc::new(tokio::sync::RwLock::new(
        crate::mcp::McpManager::with_config(crate::mcp::McpConfig::default()),
    ));
    let manager_lock = manager.write().await;
    let call = mcp::McpCallTool::new(Arc::clone(&manager));
    let registration = register_session_tool_policy(SESSION, None, HashSet::new());
    let mut dispatch = Box::pin(call.execute(
        json!({"server": "unconfigured", "tool": "noop", "arguments": {}}),
        context(SESSION, ToolExecutionMode::AgentTurn),
    ));

    assert!(
        matches!(futures::poll!(&mut dispatch), std::task::Poll::Pending),
        "dispatch must wait on the held MCP manager lock"
    );
    drop(registration);
    drop(manager_lock);

    let error = dispatch
        .await
        .expect_err("policy removal during the manager wait must block dispatch")
        .to_string();
    assert_eq!(error, MISSING_POLICY_ERROR);
}

#[cfg(unix)]
#[tokio::test]
async fn batch_child_rechecks_policy_after_awaited_pre_tool_hook() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    let _env_lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let ready = temp.path().join("hook-ready");
    let release = temp.path().join("hook-release");
    let hook = temp.path().join("policy.sh");
    std::fs::write(
        &hook,
        format!(
            "#!/bin/sh\ncat > /dev/null\nif [ \"$JCODE_HOOK_TOOL_NAME\" = batch ]; then exit 0; fi\n: > {}\nwhile [ ! -e {} ]; do sleep 0.01; done\nexit 0\n",
            crate::terminal_launch::sh_escape(&ready.to_string_lossy()),
            crate::terminal_launch::sh_escape(&release.to_string_lossy())
        ),
    )
    .expect("write hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod hook");
    let _hook_env = HookEnvReset::with_pre_tool(&hook);

    const SESSION: &str = "batch-policy-removal";
    let registry = Registry::empty();
    let effects = Arc::new(AtomicUsize::new(0));
    registry
        .register(
            "marker".to_string(),
            Arc::new(MarkerTool {
                name: "marker",
                effects: Arc::clone(&effects),
            }),
        )
        .await;
    registry
        .register(
            "batch".to_string(),
            Arc::new(batch::BatchTool::new(registry.downgrade())),
        )
        .await;
    let registration = register_session_tool_policy(SESSION, None, HashSet::new());
    let registry_for_call = registry.clone();
    let call = tokio::spawn(async move {
        registry_for_call
            .execute(
                "batch",
                json!({"tool_calls": [{"tool": "marker", "parameters": {}}]}),
                context(SESSION, ToolExecutionMode::AgentTurn),
            )
            .await
    });

    tokio::time::timeout(Duration::from_secs(2), async {
        while !ready.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child pre_tool hook did not start");
    drop(registration);
    std::fs::write(&release, "release").expect("release hook");

    let output = tokio::time::timeout(Duration::from_secs(2), call)
        .await
        .expect("batch call timed out")
        .expect("batch task panicked")
        .expect("batch wrapper should report its child failure in-band");
    assert!(output.output.contains(MISSING_POLICY_ERROR));
    assert!(output.output.contains("1 failed"));
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}
