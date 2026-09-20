#[tokio::test]
async fn clear_resets_runtime_interrupt_and_queue_state() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    seed_transient_session_state(&mut agent);
    assert_eq!(agent.soft_interrupt_count(), 1);
    assert!(agent.background_tool_signal().is_set());
    assert!(agent.graceful_shutdown_signal().is_set());

    agent.clear();

    assert_eq!(agent.soft_interrupt_count(), 0);
    assert!(!agent.background_tool_signal().is_set());
    assert!(!agent.graceful_shutdown_signal().is_set());
    assert_eq!(agent.pending_alert_count(), 0);
    assert!(agent.tool_call_ids.is_empty());
    assert!(agent.tool_result_ids.is_empty());
    assert_eq!(agent.tool_output_scan_index, 0);
    assert!(agent.last_upstream_provider.is_none());
    assert!(agent.last_connection_type.is_none());
    assert!(agent.current_turn_system_reminder.is_none());
    assert_eq!(agent.last_usage.input_tokens, 0);
    assert_eq!(agent.last_usage.output_tokens, 0);
    assert!(agent.locked_tools.is_none());
}

#[tokio::test]
async fn restore_session_resets_runtime_interrupt_and_queue_state() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    let mut restored_session = crate::session::Session::create_with_id(
        "session_restore_resets_runtime_state".to_string(),
        None,
        None,
    );
    restored_session
        .save_for_resume()
        .expect("save restored session");

    seed_transient_session_state(&mut agent);
    assert_eq!(agent.soft_interrupt_count(), 1);
    assert!(agent.background_tool_signal().is_set());
    assert!(agent.graceful_shutdown_signal().is_set());

    let status = agent
        .restore_session(&restored_session.id)
        .expect("restore session should succeed");

    assert_eq!(status, crate::session::SessionStatus::Active);
    assert_eq!(agent.session_id(), restored_session.id);
    assert_eq!(agent.soft_interrupt_count(), 0);
    assert!(!agent.background_tool_signal().is_set());
    assert!(!agent.graceful_shutdown_signal().is_set());
    assert_eq!(agent.pending_alert_count(), 0);
    assert!(agent.tool_call_ids.is_empty());
    assert!(agent.tool_result_ids.is_empty());
    assert_eq!(agent.tool_output_scan_index, 0);
    assert!(agent.last_upstream_provider.is_none());
    assert!(agent.last_connection_type.is_none());
    assert!(agent.current_turn_system_reminder.is_none());
    assert_eq!(agent.last_usage.input_tokens, 0);
    assert_eq!(agent.last_usage.output_tokens, 0);
    assert!(agent.locked_tools.is_none());
}

#[tokio::test]
async fn explicit_provider_pin_is_persisted_and_reapplied_on_restore() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let provider = Arc::new(ExplicitPinProvider::new("z-ai/glm-5.2"));
    let provider_dyn: Arc<dyn Provider> = provider.clone();
    let registry = Registry::new(provider_dyn.clone()).await;
    let mut agent = Agent::new(provider_dyn, registry);

    agent
        .set_model("z-ai/glm-5.2@Novita")
        .expect("set explicitly pinned model");
    assert_eq!(agent.provider_model(), "z-ai/glm-5.2@Novita");
    let persisted = crate::session::Session::load(agent.session_id()).expect("load saved session");
    assert_eq!(persisted.model.as_deref(), Some("z-ai/glm-5.2@Novita"));

    let restored_provider = Arc::new(ExplicitPinProvider::new("other/model"));
    let restored_provider_dyn: Arc<dyn Provider> = restored_provider.clone();
    let restored_registry = Registry::new(restored_provider_dyn.clone()).await;
    let restored_agent =
        Agent::new_with_session(restored_provider_dyn, restored_registry, persisted, None);

    assert_eq!(
        restored_provider
            .set_model_requests
            .lock()
            .unwrap()
            .as_slice(),
        ["openrouter:z-ai/glm-5.2@Novita"]
    );
    assert_eq!(restored_agent.provider_model(), "z-ai/glm-5.2@Novita");
}

#[tokio::test]
async fn explicit_user_route_is_persisted_before_first_message() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(ExplicitPinProvider::new("z-ai/glm-5.2"));
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    let selection = crate::provider::RouteSelection {
        model: "z-ai/glm-5.2".to_string(),
        runtime_key: crate::provider::RuntimeKey::OpenRouter,
        api_method: "openrouter".to_string(),
        provider_label: "Novita".to_string(),
        detail: String::new(),
    };

    agent
        .set_route_selection(&selection)
        .expect("set explicit user route");

    let persisted = crate::session::Session::load(agent.session_id())
        .expect("explicit user route should persist a resumable session");
    assert_eq!(persisted.model.as_deref(), Some("z-ai/glm-5.2@Novita"));
    assert_eq!(persisted.provider_key.as_deref(), Some("openrouter"));
    assert_eq!(persisted.route_api_method.as_deref(), Some("openrouter"));
}

#[tokio::test]
async fn auth_model_and_route_selection_leave_blank_session_lazy() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(ExplicitPinProvider::new("z-ai/glm-5.2"));
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    let session_id = agent.session_id().to_string();

    agent
        .set_model_from_auth("z-ai/glm-5.2@Novita")
        .expect("apply auth-selected model");
    let selection = crate::provider::RouteSelection {
        model: "z-ai/glm-5.2".to_string(),
        runtime_key: crate::provider::RuntimeKey::OpenRouter,
        api_method: "openrouter".to_string(),
        provider_label: "Novita".to_string(),
        detail: String::new(),
    };
    agent
        .set_route_selection_from_auth(&selection)
        .expect("apply auth-selected route");

    assert!(
        !crate::session::session_exists(&session_id),
        "auth/default selection must not make a blank session durable"
    );
}

#[tokio::test]
async fn restore_session_rehydrates_injected_memory_ids() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    crate::memory::clear_all_pending_memory();

    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    let mut restored_session = crate::session::Session::create_with_id(
        "session_restore_memory_dedup".to_string(),
        None,
        None,
    );
    restored_session.record_memory_injection(
        "🧠 auto-recalled 1 memory".to_string(),
        "persisted memory".to_string(),
        1,
        5,
        vec!["memory-persisted".to_string()],
    );
    restored_session
        .save_for_resume()
        .expect("save restored session");

    crate::memory::mark_memories_injected(&restored_session.id, &["memory-stale".to_string()]);

    agent
        .restore_session(&restored_session.id)
        .expect("restore session should succeed");

    assert!(crate::memory::is_memory_injected(
        &restored_session.id,
        "memory-persisted"
    ));
    assert!(
        !crate::memory::is_memory_injected(&restored_session.id, "memory-stale"),
        "restore should replace stale in-memory dedup state with persisted session data"
    );

    crate::memory::clear_all_pending_memory();
}

struct IsolatedAgentTestEnv {
    _home: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl IsolatedAgentTestEnv {
    fn new() -> Self {
        let home = tempfile::tempdir().expect("temp memory home");
        let previous = [
            "JCODE_HOME",
            "JCODE_MEMORY_ENABLED",
            "JCODE_MEMORY_SIDECAR_ENABLED",
            "JCODE_NO_TELEMETRY",
        ]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
        crate::env::set_var("JCODE_HOME", home.path());
        crate::env::remove_var("JCODE_MEMORY_ENABLED");
        crate::env::set_var("JCODE_MEMORY_SIDECAR_ENABLED", "false");
        crate::env::set_var("JCODE_NO_TELEMETRY", "1");
        crate::config::invalidate_config_cache();
        Self {
            _home: home,
            previous,
        }
    }
}

impl Drop for IsolatedAgentTestEnv {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            if let Some(value) = value {
                crate::env::set_var(key, value);
            } else {
                crate::env::remove_var(key);
            }
        }
        crate::config::invalidate_config_cache();
    }
}

#[test]
fn build_memory_prompt_nonblocking_defers_pending_memory_during_tool_loop() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");
    runtime.block_on(async {
        crate::memory::clear_all_pending_memory();

        let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
        let registry = Registry::new(provider.clone()).await;
        let mut agent = Agent::new(provider, registry);
        agent.set_memory_enabled(true);
        let session_id = agent.session.id.clone();

        crate::memory::set_pending_memory_with_ids(
            &session_id,
            "remember this later".to_string(),
            1,
            vec!["memory-deferred".to_string()],
        );

        let tool_loop_messages = vec![
            Message::user("hello"),
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({}),
                    thought_signature: None,
                }],
                timestamp: Some(chrono::Utc::now()),
                tool_duration_ms: None,
            },
            Message::tool_result("call_1", "ok", false),
        ];

        let pending = agent.build_memory_prompt_nonblocking(&tool_loop_messages, None);
        assert!(pending.is_none(), "memory should not inject mid tool loop");
        assert!(crate::memory::has_pending_memory(&session_id));

        let next_turn_messages = vec![Message::user("follow up")];
        let pending = agent.build_memory_prompt_nonblocking(&next_turn_messages, None);
        assert!(
            pending.is_some(),
            "memory should inject on the next real user turn"
        );
        assert!(!crate::memory::has_pending_memory(&session_id));

        crate::memory::clear_all_pending_memory();
    });
}

#[test]
fn build_memory_prompt_nonblocking_respects_explicitly_disabled_memory() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");
    runtime.block_on(async {
        crate::memory::clear_all_pending_memory();

        let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
        let registry = Registry::new(provider.clone()).await;
        let mut agent = Agent::new(provider, registry);
        agent.set_memory_enabled(false);
        let session_id = agent.session.id.clone();

        crate::memory::set_pending_memory_with_ids(
            &session_id,
            "leave this pending".to_string(),
            1,
            vec!["memory-disabled".to_string()],
        );

        let pending = agent.build_memory_prompt_nonblocking(&[Message::user("hello")], None);
        assert!(pending.is_none(), "disabled memory must not inject");
        assert!(crate::memory::has_pending_memory(&session_id));

        crate::memory::clear_all_pending_memory();
    });
}

#[tokio::test]
async fn memory_injection_message_defaults_to_ephemeral_history() {
    let _guard = crate::storage::lock_test_env();
    let previous = std::env::var_os("JCODE_PERSIST_MEMORY_INJECTIONS");
    crate::env::set_var("JCODE_PERSIST_MEMORY_INJECTIONS", "false");
    crate::config::invalidate_config_cache();

    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    let before = agent.session.messages.len();
    let memory = crate::memory::PendingMemory {
        prompt: "# Memory\n\n## Facts\n1. Use ephemeral mode".to_string(),
        display_prompt: None,
        computed_at: Instant::now(),
        count: 1,
        memory_ids: vec!["mem-ephemeral".to_string()],
    };

    let (message, persisted) = agent.prepare_memory_injection_message(&memory);

    assert!(!persisted);
    assert_eq!(agent.session.messages.len(), before);
    assert!(matches!(message.role, Role::User));
    assert!(message_text(&message).contains("Use ephemeral mode"));

    match previous {
        Some(value) => crate::env::set_var("JCODE_PERSIST_MEMORY_INJECTIONS", value),
        None => crate::env::remove_var("JCODE_PERSIST_MEMORY_INJECTIONS"),
    }
    crate::config::invalidate_config_cache();
}

#[tokio::test]
async fn memory_injection_message_can_persist_to_history() {
    let _guard = crate::storage::lock_test_env();
    let previous = std::env::var_os("JCODE_PERSIST_MEMORY_INJECTIONS");
    crate::env::set_var("JCODE_PERSIST_MEMORY_INJECTIONS", "true");
    crate::config::invalidate_config_cache();

    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    let before = agent.session.messages.len();
    let memory = crate::memory::PendingMemory {
        prompt: "# Memory\n\n## Facts\n1. Persist for cache".to_string(),
        display_prompt: None,
        computed_at: Instant::now(),
        count: 1,
        memory_ids: vec!["mem-persisted".to_string()],
    };

    let (message, persisted) = agent.prepare_memory_injection_message(&memory);

    assert!(persisted);
    assert_eq!(agent.session.messages.len(), before + 1);
    assert_eq!(
        content_text(&agent.session.messages.last().unwrap().content),
        message_text(&message)
    );
    assert!(
        content_text(&agent.session.messages.last().unwrap().content).contains("Persist for cache")
    );

    match previous {
        Some(value) => crate::env::set_var("JCODE_PERSIST_MEMORY_INJECTIONS", value),
        None => crate::env::remove_var("JCODE_PERSIST_MEMORY_INJECTIONS"),
    }
    crate::config::invalidate_config_cache();
}

#[tokio::test]
async fn mark_closed_persists_soft_interrupts_for_restore_after_reload() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();

    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider.clone(), registry.clone());
    let session_id = agent.session_id().to_string();
    agent.queue_soft_interrupt(
        "resume me after reload".to_string(),
        Vec::new(),
        true,
        SoftInterruptSource::System,
    );

    agent.mark_closed();

    let mut restored = Agent::new(provider, registry);
    restored
        .restore_session(&session_id)
        .expect("restore session with persisted interrupts");

    assert_eq!(restored.soft_interrupt_count(), 1);
    assert!(restored.has_urgent_interrupt());
    assert!(
        crate::soft_interrupt_store::load(&session_id)
            .expect("store should be readable after restore")
            .is_empty()
    );
}

#[tokio::test]
async fn mark_closed_leaves_untouched_blank_session_lazy() {
    let _guard = crate::storage::lock_test_env();
    let _env = IsolatedAgentTestEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    let session_id = agent.session_id().to_string();

    agent.mark_closed();

    assert!(
        !crate::session::session_exists(&session_id),
        "ordinary blank close must remain lazy when no resume state is pending"
    );
}

#[tokio::test]
async fn env_snapshot_detail_is_minimal_for_empty_sessions_and_full_after_history() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    assert_eq!(agent.env_snapshot_detail(), EnvSnapshotDetail::Minimal);
    let minimal = agent.build_env_snapshot("create", agent.env_snapshot_detail());
    assert!(minimal.jcode_git_hash.is_none());
    assert!(minimal.jcode_git_dirty.is_none());
    assert!(minimal.working_git.is_none());

    agent
        .session
        .append_stored_message(crate::session::StoredMessage {
            id: "msg_env_snapshot_detail".to_string(),
            role: crate::message::Role::User,
            content: vec![ContentBlock::Text {
                text: "hello".to_string(),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });

    assert_eq!(agent.env_snapshot_detail(), EnvSnapshotDetail::Full);
}

/// A trivial tool used to simulate an MCP tool registering on the registry
/// after the agent has already locked its tool snapshot.
struct FakeMcpTool {
    name: String,
}

#[async_trait]
impl crate::tool::Tool for FakeMcpTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "fake mcp tool"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }
    async fn execute(
        &self,
        _input: serde_json::Value,
        _ctx: crate::tool::ToolContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(ToolOutput::new("ok"))
    }
}

struct VerboseFakeMcpTool {
    name: String,
    description: String,
}

#[async_trait]
impl crate::tool::Tool for VerboseFakeMcpTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": "string"}}
        })
    }
    async fn execute(
        &self,
        _input: serde_json::Value,
        _ctx: crate::tool::ToolContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(ToolOutput::new("ok"))
    }
}

async fn register_fake_deferred_mcp_surface(registry: &Registry) {
    for name in ["mcp_search", "mcp_call"] {
        registry
            .register(
                name.to_string(),
                Arc::new(FakeMcpTool {
                    name: name.to_string(),
                }) as Arc<dyn crate::tool::Tool>,
            )
            .await;
    }
}

async fn agent_with_fake_mcp_surface(mode: crate::config::McpToolsMode, threshold: usize) -> Agent {
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    registry
        .register(
            "mcp__test__verbose".to_string(),
            Arc::new(VerboseFakeMcpTool {
                name: "verbose".to_string(),
                description: "large MCP definition ".repeat(32),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = mode;
    agent.mcp_tools_token_threshold = threshold;
    agent
}

#[tokio::test]
async fn mcp_exposure_modes_select_eager_or_fixed_definitions() {
    let _guard = crate::storage::lock_test_env();

    let mut eager = agent_with_fake_mcp_surface(crate::config::McpToolsMode::Eager, 0).await;
    let eager_names: Vec<String> = eager
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert!(eager_names.iter().any(|name| name == "mcp__test__verbose"));
    assert!(!eager_names.iter().any(|name| name == "mcp_search"));
    assert!(!eager_names.iter().any(|name| name == "mcp_call"));

    let mut deferred =
        agent_with_fake_mcp_surface(crate::config::McpToolsMode::Deferred, usize::MAX).await;
    let deferred_names: Vec<String> = deferred
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert!(!deferred_names.iter().any(|name| name.starts_with("mcp__")));
    assert!(deferred_names.iter().any(|name| name == "mcp_search"));
    assert!(deferred_names.iter().any(|name| name == "mcp_call"));

    let mut auto_eager =
        agent_with_fake_mcp_surface(crate::config::McpToolsMode::Auto, usize::MAX).await;
    let auto_eager_names: Vec<String> = auto_eager
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert!(
        auto_eager_names
            .iter()
            .any(|name| name == "mcp__test__verbose")
    );

    let mut auto_deferred = agent_with_fake_mcp_surface(crate::config::McpToolsMode::Auto, 1).await;
    let auto_deferred_names: Vec<String> = auto_deferred
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert!(
        !auto_deferred_names
            .iter()
            .any(|name| name.starts_with("mcp__"))
    );
    assert!(auto_deferred_names.iter().any(|name| name == "mcp_search"));
    assert!(auto_deferred_names.iter().any(|name| name == "mcp_call"));
    let stable_auto_names: Vec<String> = auto_deferred
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(auto_deferred_names, stable_auto_names);
    assert!(auto_deferred.mcp_late_register_resolved);
}

#[tokio::test]
async fn deferred_mcp_surface_ignores_late_per_tool_registration() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Deferred;

    let before: Vec<String> = agent
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    agent
        .registry
        .register(
            "mcp__late__tool".to_string(),
            Arc::new(FakeMcpTool {
                name: "late".to_string(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let after: Vec<String> = agent
        .tool_definitions()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect();

    assert_eq!(
        before, after,
        "fixed deferred surface must stay cache-stable"
    );
    assert!(agent.mcp_late_register_resolved);
    assert!(!after.iter().any(|name| name.starts_with("mcp__")));
}

#[tokio::test]
async fn auto_mode_rechecks_late_mcp_definitions_before_deferring() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Auto;
    agent.mcp_tools_token_threshold = 1;

    let before = agent.tool_definitions().await;
    assert!(!before.iter().any(|tool| tool.name == "mcp_search"));
    agent
        .registry
        .register(
            "mcp__late__large".to_string(),
            Arc::new(VerboseFakeMcpTool {
                name: "large".to_string(),
                description: "late large definition ".repeat(32),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;

    let after = agent.tool_definitions().await;
    assert!(after.iter().any(|tool| tool.name == "mcp_search"));
    assert!(after.iter().any(|tool| tool.name == "mcp_call"));
    assert!(!after.iter().any(|tool| tool.name.starts_with("mcp__")));
    assert!(agent.mcp_late_register_resolved);
}

/// Reproduction for #206: MCP tools that register on the registry *after* the
/// first turn locks the tool snapshot never reach the provider, because
/// `tool_definitions()` returns the frozen `locked_tools` snapshot and the only
/// unlock path (`unlock_tools_if_needed`) fires solely when the LLM invokes the
/// `"mcp"` management tool — which it never does, since it cannot see the
/// `mcp__*` tools it would need to trigger that unlock.
#[tokio::test]
async fn mcp_tools_registered_after_lock_are_visible_to_agent() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Eager;

    // First turn locks the snapshot (this is what happens before the async MCP
    // registration spawn completes).
    let before = agent.tool_definitions().await;
    let before_len = before.len();
    assert!(
        !before.iter().any(|t| t.name.starts_with("mcp__")),
        "precondition: no mcp tools before async registration completes"
    );

    // Simulate the spawned MCP registration task finishing: a new mcp__* tool
    // lands on the shared registry.
    agent
        .registry
        .register(
            "mcp__test__write_memory".to_string(),
            Arc::new(FakeMcpTool {
                name: "mcp__test__write_memory".to_string(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;

    // The next turn should now advertise the MCP tool to the provider.
    let after = agent.tool_definitions().await;
    assert!(
        after.iter().any(|t| t.name == "mcp__test__write_memory"),
        "regression #206: MCP tool registered after the first turn never reaches \
         the agent's tool surface (locked snapshot of {} tools is reused forever)",
        before_len
    );

    // Once MCP tools are present in the locked snapshot, subsequent turns must
    // return the *same* stable snapshot so provider prompt-cache hits stay warm
    // (the whole point of locked_tools). The #206 fix must not flap.
    let names =
        |defs: &[ToolDefinition]| -> Vec<String> { defs.iter().map(|t| t.name.clone()).collect() };
    let stable_a = agent.tool_definitions().await;
    let stable_b = agent.tool_definitions().await;
    assert_eq!(
        names(&stable_a),
        names(&stable_b),
        "tool snapshot must be stable across turns once MCP tools are present"
    );
    assert_eq!(
        names(&stable_a),
        names(&after),
        "snapshot must not change after MCP tools are already included"
    );
}

/// The intentional, MCP-driven prompt-cache miss must happen at most ONCE per
/// locked snapshot. After the first late-registered `mcp__*` tool is picked up
/// (the one accepted miss), a *second* MCP tool that registers even later must
/// NOT trigger another rebuild — otherwise a server that connects in waves would
/// thrash the provider prompt cache. Guards the `mcp_late_register_resolved`
/// one-shot flag (#206 follow-up).
#[tokio::test]
async fn mcp_late_registration_rebuild_happens_at_most_once() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Eager;

    // First turn locks the snapshot with no MCP tools yet.
    let _ = agent.tool_definitions().await;

    // First MCP tool arrives -> one accepted rebuild exposes it.
    agent
        .registry
        .register(
            "mcp__test__first".to_string(),
            Arc::new(FakeMcpTool {
                name: "mcp__test__first".to_string(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let after_first = agent.tool_definitions().await;
    assert!(
        after_first.iter().any(|t| t.name == "mcp__test__first"),
        "first late MCP tool must be picked up by the one accepted rebuild"
    );
    assert!(
        agent.mcp_late_register_resolved,
        "one-shot guard must latch after the accepted rebuild"
    );

    // A SECOND MCP tool registers even later (server connected in a second
    // wave). The one-shot guard means we do NOT rebuild again, so the snapshot
    // stays cache-stable and this tool is intentionally not surfaced until the
    // tool list is explicitly unlocked.
    agent
        .registry
        .register(
            "mcp__test__second".to_string(),
            Arc::new(FakeMcpTool {
                name: "mcp__test__second".to_string(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let after_second = agent.tool_definitions().await;
    let names: Vec<String> = after_second.iter().map(|t| t.name.clone()).collect();
    assert!(
        names.iter().any(|n| n == "mcp__test__first"),
        "previously surfaced MCP tool must remain"
    );
    assert!(
        !names.iter().any(|n| n == "mcp__test__second"),
        "second-wave MCP tool must NOT trigger a second cache-busting rebuild"
    );

    // An explicit unlock (e.g. the `mcp` reload tool) re-arms the one-shot guard
    // and lets the next snapshot pick up everything currently registered.
    agent.unlock_tools();
    assert!(
        !agent.mcp_late_register_resolved,
        "explicit unlock must re-arm the one-shot guard"
    );
    let after_unlock = agent.tool_definitions().await;
    let unlocked_names: Vec<String> = after_unlock.iter().map(|t| t.name.clone()).collect();
    assert!(
        unlocked_names.iter().any(|n| n == "mcp__test__second"),
        "after explicit unlock, the second-wave MCP tool must finally surface"
    );
}

/// Without any newly-registered MCP tools, the locked snapshot must be returned
/// verbatim on every turn (no rebuild, no cache invalidation). Guards the #206
/// fix against re-snapshotting on turns where nothing changed.
#[tokio::test]
async fn tool_snapshot_is_stable_without_new_mcp_tools() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    let first = agent.tool_definitions().await;
    // Register a NON-mcp tool after locking — this should NOT trigger a rebuild,
    // because the cache-stability optimization only yields to MCP arrival.
    agent
        .registry
        .register(
            "not_an_mcp_tool".to_string(),
            Arc::new(FakeMcpTool {
                name: "not_an_mcp_tool".to_string(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let second = agent.tool_definitions().await;
    let first_names: Vec<String> = first.iter().map(|t| t.name.clone()).collect();
    let second_names: Vec<String> = second.iter().map(|t| t.name.clone()).collect();
    assert_eq!(
        first_names, second_names,
        "non-MCP registry changes must not invalidate the locked tool snapshot"
    );
    assert!(
        !second_names.iter().any(|n| n == "not_an_mcp_tool"),
        "non-MCP tool registered after lock must not leak into the snapshot"
    );
}

#[test]
fn empty_post_tool_response_gets_more_than_one_retry() {
    // Regression guard for the Claude Opus 5 benchmark incident. A provider can
    // return an empty response immediately after tool results; that is a
    // transient hiccup, not a finished task. With only one retry allowed, a
    // single empty response (observed once in 43 turns) ended a 20-hour agent
    // run with the work half-done and the submission unoptimized.
    const {
        assert!(
            Agent::MAX_EMPTY_POST_TOOL_CONTINUATION_ATTEMPTS > 1,
            "a single retry lets one transient empty response end a long run"
        );
        // Bounded, so a genuinely finished agent still exits instead of looping.
        assert!(Agent::MAX_EMPTY_POST_TOOL_CONTINUATION_ATTEMPTS <= 10);
    }
}

#[test]
fn output_budget_truncation_requests_a_continuation() {
    // Regression guard for the Claude Opus 5 benchmark incident. A turn cut off
    // by the output budget reports stop_reason=max_tokens and can contain zero
    // tool calls, which otherwise looks exactly like a finished turn. The agent
    // must treat it as incomplete and continue rather than ending the run.
    assert!(Agent::should_continue_after_stop_reason("max_tokens"));
    assert!(Agent::should_continue_after_stop_reason("MAX_TOKENS"));
    assert!(Agent::should_continue_after_stop_reason(" max_tokens "));
    assert!(Agent::should_continue_after_stop_reason(
        "max_output_tokens"
    ));
    assert!(Agent::should_continue_after_stop_reason("length"));
    assert!(Agent::should_continue_after_stop_reason("truncated"));
    assert!(Agent::should_continue_after_stop_reason("incomplete"));

    // Normal completions must not trigger a continuation loop.
    assert!(!Agent::should_continue_after_stop_reason("end_turn"));
    assert!(!Agent::should_continue_after_stop_reason("tool_use"));
    assert!(!Agent::should_continue_after_stop_reason("stop"));
    // An absent reason is the pre-fix wire behaviour: it cannot be recovered
    // from, which is precisely why MessageEnd must forward the real reason.
    assert!(!Agent::should_continue_after_stop_reason(""));
}

#[test]
fn stranded_tool_use_stop_is_detected() {
    // Second half of the Opus 5 DeepSWE incident: the provider reported
    // stop_reason="tool_use" while the parsed tool-call list was empty, so the
    // turn loop had nothing to execute and broke out mid-task, discarding every
    // uncommitted edit. `tool_use` is a normal completion reason, so
    // `should_continue_after_stop_reason` must keep rejecting it; the stranded
    // case is only recoverable when it is paired with zero tool calls, which is
    // exactly what this predicate is for.
    assert!(Agent::is_stranded_tool_use_stop(Some("tool_use")));
    assert!(Agent::is_stranded_tool_use_stop(Some("TOOL_USE")));
    assert!(Agent::is_stranded_tool_use_stop(Some(" tool_use ")));

    assert!(!Agent::is_stranded_tool_use_stop(Some("end_turn")));
    assert!(!Agent::is_stranded_tool_use_stop(Some("max_tokens")));
    assert!(!Agent::is_stranded_tool_use_stop(Some("")));
    assert!(!Agent::is_stranded_tool_use_stop(None));
    // Must stay disjoint from the truncation path so a turn never takes both
    // continuation branches for one stop reason.
    assert!(!Agent::should_continue_after_stop_reason("tool_use"));
}

#[test]
fn guardrail_stop_reason_detection() {
    assert!(Agent::is_guardrail_stop_reason(Some("refusal")));
    assert!(Agent::is_guardrail_stop_reason(Some("REFUSAL")));
    assert!(Agent::is_guardrail_stop_reason(Some(" content_filter ")));
    assert!(Agent::is_guardrail_stop_reason(Some("safety")));
    assert!(Agent::is_guardrail_stop_reason(Some("model_guardrail")));
    assert!(Agent::is_guardrail_stop_reason(Some("policy_violation_x")));
    assert!(!Agent::is_guardrail_stop_reason(Some("end_turn")));
    assert!(!Agent::is_guardrail_stop_reason(Some("max_tokens")));
    assert!(!Agent::is_guardrail_stop_reason(Some("tool_use")));
    assert!(!Agent::is_guardrail_stop_reason(Some("stop")));
    assert!(!Agent::is_guardrail_stop_reason(None));
}

#[test]
fn fable_guardrail_reconsideration_is_narrow_and_bounded() {
    assert!(Agent::should_reconsider_fable_guardrail(
        "claude-fable-5",
        Some("refusal"),
        0,
        1,
    ));
    assert!(Agent::should_reconsider_fable_guardrail(
        "CLAUDE-FABLE-5-20260801",
        Some("content_filter"),
        0,
        1,
    ));
    assert!(Agent::should_reconsider_fable_guardrail(
        "claude-fable-5",
        Some("refusal"),
        1,
        3,
    ));
    assert!(Agent::should_reconsider_fable_guardrail(
        "claude-fable-5",
        Some("refusal"),
        2,
        3,
    ));
    assert!(!Agent::should_reconsider_fable_guardrail(
        "claude-fable-5",
        Some("refusal"),
        3,
        3,
    ));
    assert!(!Agent::should_reconsider_fable_guardrail(
        "claude-fable-5",
        Some("end_turn"),
        0,
        1,
    ));
    assert!(!Agent::should_reconsider_fable_guardrail(
        "claude-opus-5",
        Some("refusal"),
        0,
        1,
    ));
}

#[test]
fn fable_guardrail_prompt_suite_is_distinct_and_safety_preserving() {
    let prompts = Agent::FABLE_GUARDRAIL_RECONSIDERATION_PROMPTS;
    assert_eq!(prompts.len(), 3);
    assert_ne!(prompts[0], prompts[1]);
    assert_ne!(prompts[1], prompts[2]);
    assert!(prompts[0].contains("full context"));
    assert!(prompts[1].contains("safe portions"));
    assert!(prompts[2].contains("Do not weaken a refusal"));
}

#[test]
fn guardrail_notice_for_refusal_stop() {
    let notice = Agent::provider_guardrail_notice(Some("refusal"), true, true)
        .expect("refusal with empty text must produce a notice");
    assert!(
        notice.contains("refusal"),
        "notice should name the stop reason: {notice}"
    );
    assert!(notice.to_lowercase().contains("guardrail"));
    // Guardrail stop with visible text still surfaces (partial output then refusal).
    assert!(Agent::provider_guardrail_notice(Some("refusal"), false, false).is_some());
}

#[test]
fn guardrail_notice_for_silent_empty_turn() {
    // end_turn with zero visible output and reasoning-only content: surface it.
    let notice = Agent::provider_guardrail_notice(Some("end_turn"), true, true)
        .expect("empty visible output must produce a notice");
    assert!(notice.contains("internal reasoning"), "{notice}");
    assert!(notice.contains("end_turn"), "{notice}");
    // Unknown stop reason, empty output, no reasoning.
    let notice = Agent::provider_guardrail_notice(None, true, false)
        .expect("empty visible output must produce a notice");
    assert!(notice.contains("unknown"), "{notice}");
    assert!(!notice.contains("internal reasoning"), "{notice}");
}

#[test]
fn guardrail_notice_absent_for_normal_turns() {
    // Normal turn with visible text: no notice.
    assert!(Agent::provider_guardrail_notice(Some("end_turn"), false, false).is_none());
    assert!(Agent::provider_guardrail_notice(None, false, true).is_none());
}

#[test]
fn empty_turn_log_event_separates_guardrails_from_transient_empties() {
    assert_eq!(
        Agent::empty_turn_log_event(Some("refusal")),
        "PROVIDER_GUARDRAIL"
    );
    assert_eq!(
        Agent::empty_turn_log_event(Some("content_filter")),
        "PROVIDER_GUARDRAIL"
    );
    assert_eq!(
        Agent::empty_turn_log_event(Some("stop")),
        "PROVIDER_EMPTY_RESPONSE"
    );
    assert_eq!(Agent::empty_turn_log_event(None), "PROVIDER_EMPTY_RESPONSE");
}

#[test]
fn guardrail_notice_for_transient_empty_does_not_blame_content_filter() {
    let notice = Agent::provider_guardrail_notice(Some("stop"), true, false)
        .expect("empty visible output must produce a notice");
    assert!(
        !notice.contains("usually a provider-side guardrail"),
        "transient empty responses must not be blamed on a guardrail: {notice}"
    );
    assert!(notice.contains("empty response"), "{notice}");
}
