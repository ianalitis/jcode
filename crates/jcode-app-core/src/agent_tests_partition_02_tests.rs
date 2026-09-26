#[tokio::test]
async fn empty_post_tool_response_is_retried_in_shared_helper() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    let mut attempts = 0u32;
    // Empty response right after tool results: inject continuation.
    let retried = agent
        .maybe_continue_empty_post_tool_response(true, true, Some("stop"), &mut attempts)
        .expect("helper must not error");
    assert!(retried);
    assert_eq!(attempts, 1);
    let recovery = agent
        .session
        .messages
        .last()
        .expect("recovery instruction must be persisted");
    assert_eq!(recovery.role, Role::User);
    assert!(
        recovery
            .content
            .iter()
            .find_map(|block| match block {
                ContentBlock::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .is_some_and(|text| text.starts_with("<system-reminder>")),
        "synthetic recovery instruction must be hidden from the transcript"
    );

    // A guardrail refusal is deliberate and must not be retried.
    let retried = agent
        .maybe_continue_empty_post_tool_response(true, true, Some("refusal"), &mut attempts)
        .expect("helper must not error");
    assert!(!retried);

    // Visible output or no recent tool result: no retry.
    assert!(
        !agent
            .maybe_continue_empty_post_tool_response(false, true, Some("stop"), &mut attempts)
            .unwrap()
    );
    assert!(
        !agent
            .maybe_continue_empty_post_tool_response(true, false, Some("stop"), &mut attempts)
            .unwrap()
    );

    // Retry budget is bounded.
    attempts = Agent::MAX_EMPTY_POST_TOOL_CONTINUATION_ATTEMPTS;
    assert!(
        !agent
            .maybe_continue_empty_post_tool_response(true, true, Some("stop"), &mut attempts)
            .unwrap()
    );
}

include!("agent_tests/retention_readiness.rs");

/// Provider that reproduces the DeepSWE Opus 5 incident: the first response
/// ends with `stop_reason: "tool_use"` while carrying no tool-use block at all,
/// which is what happens when an unrecognized content block is dropped from the
/// stream. The second response is a normal completion, so a correct agent
/// recovers and this provider's queue is exhausted.
#[derive(Clone, Default)]
struct StrandedToolUseProvider {
    calls: Arc<std::sync::Mutex<usize>>,
}

#[async_trait]
impl Provider for StrandedToolUseProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let call = {
            let mut guard = self.calls.lock().unwrap();
            *guard += 1;
            *guard
        };
        let (tx, rx) = tokio_mpsc::channel::<Result<StreamEvent>>(8);
        tokio::spawn(async move {
            if call == 1 {
                let _ = tx
                    .send(Ok(StreamEvent::TextDelta("working on it".to_string())))
                    .await;
                // No ToolUseStart: the tool block was lost, yet the provider
                // still reports that it stopped in order to call a tool.
                let _ = tx
                    .send(Ok(StreamEvent::MessageEnd {
                        stop_reason: Some("tool_use".to_string()),
                    }))
                    .await;
            } else {
                let _ = tx
                    .send(Ok(StreamEvent::TextDelta("all done".to_string())))
                    .await;
                let _ = tx
                    .send(Ok(StreamEvent::MessageEnd {
                        stop_reason: Some("end_turn".to_string()),
                    }))
                    .await;
            }
        });
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        "stranded-tool-use"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

/// End-to-end guard for the incident. Before the fix the agent took the
/// "no tool calls" branch and ended the turn on the very first response, so a
/// benchmark trial stopped mid-task and its uncommitted work was never
/// captured. The agent must instead ask the model to continue, which shows up
/// as a second provider call and a final turn that ends normally.
#[tokio::test]
async fn stranded_tool_use_stop_continues_instead_of_ending_the_turn() {
    let _guard = crate::storage::lock_test_env();
    let stranded = StrandedToolUseProvider::default();
    let calls = stranded.calls.clone();
    let provider: Arc<dyn Provider> = Arc::new(stranded);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    agent
        .run_once_streaming_mpsc("do the task", Vec::new(), None, tx)
        .await
        .expect("turn should complete");

    let mut text = String::new();
    while let Ok(event) = rx.try_recv() {
        if let ServerEvent::TextDelta { text: delta } = event {
            text.push_str(&delta);
        }
    }

    assert_eq!(
        *calls.lock().unwrap(),
        2,
        "a tool_use stop with no tool call must trigger exactly one continuation request"
    );
    assert!(
        text.contains("all done"),
        "the recovered turn must deliver the model's real completion, got {text:?}"
    );
}

#[derive(Clone, Default)]
struct FableGuardrailProvider {
    calls: Arc<std::sync::Mutex<usize>>,
    prompts_seen: Arc<std::sync::Mutex<Vec<String>>>,
}

#[async_trait]
impl Provider for FableGuardrailProvider {
    async fn complete(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let call = {
            let mut calls = self.calls.lock().unwrap();
            *calls += 1;
            *calls
        };
        if call > 1 {
            let prompt = messages
                .last()
                .map(message_text)
                .unwrap_or_default()
                .to_string();
            self.prompts_seen.lock().unwrap().push(prompt);
        }

        let (tx, rx) = tokio_mpsc::channel::<Result<StreamEvent>>(4);
        tokio::spawn(async move {
            if call <= 3 {
                let _ = tx
                    .send(Ok(StreamEvent::MessageEnd {
                        stop_reason: Some("refusal".to_string()),
                    }))
                    .await;
            } else {
                let _ = tx
                    .send(Ok(StreamEvent::TextDelta(
                        "Reconsidered and completed safely".to_string(),
                    )))
                    .await;
                let _ = tx
                    .send(Ok(StreamEvent::MessageEnd {
                        stop_reason: Some("end_turn".to_string()),
                    }))
                    .await;
            }
        });
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> String {
        "claude-fable-5".to_string()
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

#[tokio::test]
async fn fable_guardrail_reconsideration_recovers_the_streaming_turn() {
    let _guard = crate::storage::lock_test_env();
    let fable = FableGuardrailProvider::default();
    let calls = fable.calls.clone();
    let prompts_seen = fable.prompts_seen.clone();
    let provider: Arc<dyn Provider> = Arc::new(fable);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    agent
        .run_once_streaming_mpsc("do this ordinary coding task", Vec::new(), None, tx)
        .await
        .expect("turn should recover from the guardrail");

    let mut text = String::new();
    while let Ok(event) = rx.try_recv() {
        if let ServerEvent::TextDelta { text: delta } = event {
            text.push_str(&delta);
        }
    }

    assert_eq!(*calls.lock().unwrap(), 4);
    let prompts = prompts_seen.lock().unwrap();
    assert_eq!(prompts.len(), 3);
    assert!(prompts[0].contains("concrete harmful action"));
    assert!(prompts[1].contains("safe portions"));
    assert!(prompts[2].contains("final, independent policy check"));
    assert!(
        text.contains("Reconsidered and completed safely"),
        "{text:?}"
    );
}

#[tokio::test]
async fn sdk_custom_compile_remote_schema_survives_locked_refresh() {
    let _lock = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(SignatureSessionProvider::default());
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    crate::tool::sdk::configure(
        agent.session_id(),
        "cache-owner",
        crate::protocol::SessionToolConfig {
            enabled: Some(vec![]),
            disabled: vec![],
            custom: vec![crate::protocol::SessionToolDefinition {
                name: "compile_remote".into(),
                description: "SDK override".into(),
                parameters: serde_json::json!({"type":"object", "additionalProperties":false}),
            }],
        },
        tx,
    )
    .unwrap();
    for _ in 0..2 {
        let definitions = agent.tool_definitions().await;
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].description, "SDK override");
        assert_eq!(
            definitions[0].input_schema,
            serde_json::json!({"type":"object", "additionalProperties":false})
        );
    }
}

#[test]
fn system_prompt_override_restores_and_does_not_leak_across_sessions() {
    let _lock = crate::storage::lock_test_env();
    let home = tempfile::tempdir().unwrap();
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            match self.0.take() {
                Some(home) => crate::env::set_var("JCODE_HOME", home),
                None => crate::env::remove_var("JCODE_HOME"),
            }
        }
    }
    let _restore = RestoreHome(std::env::var_os("JCODE_HOME"));
    crate::env::set_var("JCODE_HOME", home.path());
    for prompt in ["custom system prompt", ""] {
        let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
        let mut agent = Agent::new(provider.clone(), Registry::empty());
        agent.set_system_prompt(prompt);
        let id = agent.session_id().to_string();
        let split = agent.build_system_prompt_split(Some("memory must not be appended"));
        assert_eq!(split.static_part, prompt);
        assert!(split.dynamic_part.is_empty());
        assert_eq!(
            Session::load(&id).unwrap().system_prompt.as_deref(),
            Some(prompt)
        );

        agent.clear();
        assert_eq!(agent.session.system_prompt, None);
        assert_ne!(agent.build_system_prompt_split(None).static_part, prompt);
        agent.restore_session(&id).unwrap();
        assert_eq!(agent.build_system_prompt_split(None).static_part, prompt);

        let mut other = Session::create(None, Some("plain session".into()));
        other.save().unwrap();
        agent.restore_session(&other.id).unwrap();
        assert_eq!(agent.session.system_prompt, None);
        assert_ne!(agent.build_system_prompt_split(None).static_part, prompt);
        let loaded = Session::load(&id).unwrap();
        let attached = Agent::new_with_session(provider, Registry::empty(), loaded, None);
        assert_eq!(attached.build_system_prompt_split(None).static_part, prompt);
    }
}

#[tokio::test]
async fn self_compact_tool_note_is_drained_on_next_poll() {
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    for i in 0..30 {
        agent.add_message(
            Role::User,
            vec![ContentBlock::Text {
                text: format!("turn {i} {}", "x".repeat(120)),
                cache_control: None,
            }],
        );
    }

    // The tool path: the note is parked under the session id, not on the agent.
    let note = "next: run cargo test -p jcode-app-core --lib self_compact";
    crate::tool::self_compact::store_pending_self_compact_note(&agent.session.id, note);
    assert!(agent.pending_self_compact_note().is_none());

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut delivered = false;
    while Instant::now() < deadline {
        if agent.poll_compaction_completion_event().is_some() {
            delivered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(delivered, "poll must start and complete the tool-requested compaction");
    assert!(agent.pending_self_compact_note().is_none());
    assert!(
        crate::tool::self_compact::take_pending_self_compact_note(&agent.session.id).is_none(),
        "the session-keyed note must be consumed exactly once"
    );
    let last = agent.session.messages.last().expect("messages");
    assert_eq!(last.role, Role::User);
    let text = last
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(
        text,
        format!("{}\n{}", crate::tool::self_compact::SELF_COMPACT_NOTE_PREFIX, note)
    );
}

/// Test provider advertising provider-native deferred tool loading.
struct NativeDeferredToolsProvider;

#[async_trait]
impl Provider for NativeDeferredToolsProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let (_tx, rx) = tokio_mpsc::channel::<Result<StreamEvent>>(1);
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        "claude"
    }

    fn supports_deferred_tools(&self) -> bool {
        true
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self)
    }
}

async fn native_deferred_agent(mode: crate::config::McpToolsMode) -> Agent {
    let provider: Arc<dyn Provider> = Arc::new(NativeDeferredToolsProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = mode;
    agent.mcp_tools_token_threshold = 1;
    agent
}

fn eager_names(tools: &[ToolDefinition]) -> Vec<String> {
    tools
        .iter()
        .filter(|t| !t.defer_loading)
        .map(|t| t.name.clone())
        .collect()
}

/// With provider-native deferred loading, MCP tools registering after the
/// first turn (async connect, `mcp connect`, reconnect) only extend the
/// deferred set. The eager, cached prefix must be byte-identical and the
/// cache tracker must not be reset.
#[tokio::test]
async fn native_deferred_mcp_late_registration_keeps_eager_prefix() {
    let _guard = crate::storage::lock_test_env();
    for mode in [
        crate::config::McpToolsMode::Auto,
        crate::config::McpToolsMode::Deferred,
    ] {
        let mut agent = native_deferred_agent(mode).await;
        let before = agent.tool_definitions().await;
        assert!(agent.native_deferred_mcp());
        assert!(
            before
                .iter()
                .any(|t| t.name == "mcp_search" && !t.defer_loading),
            "mcp_search stays eager for discovery"
        );
        assert!(
            before
                .iter()
                .any(|t| t.name == "mcp_call" && t.defer_loading),
            "mcp_call is a stable deferred placeholder"
        );

        agent
            .registry
            .register(
                "mcp__late__tool".to_string(),
                Arc::new(FakeMcpTool {
                    name: "late".to_string(),
                }) as Arc<dyn crate::tool::Tool>,
            )
            .await;
        agent.unlock_tools_if_needed("mcp");
        let after = agent.tool_definitions().await;

        assert_eq!(eager_names(&before), eager_names(&after), "mode={mode:?}");
        let serialized = |tools: &[ToolDefinition]| {
            serde_json::to_string(&ToolDefinition::eager(tools)).unwrap()
        };
        assert_eq!(serialized(&before), serialized(&after));
        let late = after
            .iter()
            .find(|t| t.name == "mcp__late__tool")
            .expect("late MCP tool is offered to the provider");
        assert!(late.defer_loading, "late MCP tools are deferred");
        assert!(agent.locked_tools.is_some(), "eager snapshot stays locked");
    }
}

#[tokio::test]
async fn eager_mode_ignores_native_deferred_support() {
    let _guard = crate::storage::lock_test_env();
    let mut agent = native_deferred_agent(crate::config::McpToolsMode::Eager).await;
    agent
        .registry
        .register(
            "mcp__srv__tool".to_string(),
            Arc::new(FakeMcpTool {
                name: "tool".to_string(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let tools = agent.tool_definitions().await;
    assert!(!agent.native_deferred_mcp());
    assert!(tools.iter().all(|t| !t.defer_loading));
    assert!(tools.iter().any(|t| t.name == "mcp__srv__tool"));
}

#[test]
fn tool_reference_metadata_becomes_reference_blocks() {
    let output = ToolOutput::new("found").with_metadata(serde_json::json!({
        "tool_references": ["mcp__a__x", "mcp__b__y"],
    }));
    let blocks = tool_output_to_content_blocks("call_9".to_string(), output);
    assert!(matches!(blocks[0], ContentBlock::ToolResult { .. }));
    let refs: Vec<(&str, &str)> = blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolReference {
                tool_use_id,
                tool_name,
            } => Some((tool_use_id.as_str(), tool_name.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(refs, vec![("call_9", "mcp__a__x"), ("call_9", "mcp__b__y")]);
}

/// Provider whose deferred-loading capability can flip, like a mid-session
/// switch between Claude and a non-native route.
struct SwitchableDeferredProvider(Arc<std::sync::atomic::AtomicBool>);

#[async_trait]
impl Provider for SwitchableDeferredProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let (_tx, rx) = tokio_mpsc::channel::<Result<StreamEvent>>(1);
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        "switchable"
    }

    fn supports_deferred_tools(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self(Arc::clone(&self.0)))
    }
}

#[tokio::test]
async fn switching_away_from_native_deferred_restores_mcp_fallback_surface() {
    let _guard = crate::storage::lock_test_env();
    let native = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let provider: Arc<dyn Provider> = Arc::new(SwitchableDeferredProvider(Arc::clone(&native)));
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Deferred;

    let on_native = agent.tool_definitions().await;
    assert!(
        on_native
            .iter()
            .any(|t| t.name == "mcp_call" && t.defer_loading)
    );

    native.store(false, std::sync::atomic::Ordering::SeqCst);
    let fallback = agent.tool_definitions().await;
    assert!(
        fallback
            .iter()
            .any(|t| t.name == "mcp_call" && !t.defer_loading),
        "non-native provider must get an eager mcp_call"
    );
    assert!(fallback.iter().all(|t| !t.defer_loading));

    native.store(true, std::sync::atomic::Ordering::SeqCst);
    let back = agent.tool_definitions().await;
    assert!(back.iter().any(|t| t.name == "mcp_call" && t.defer_loading));
}
