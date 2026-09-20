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
