use super::*;

/// Streams a plain reply with a tiny context window so the agent's own hard
/// compaction (not a provider-native one) produces the Compaction event.
struct JcodeCompactionStreamProvider;

#[async_trait]
impl Provider for JcodeCompactionStreamProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let (tx, rx) = tokio_mpsc::channel::<Result<StreamEvent>>(2);
        tokio::spawn(async move {
            let _ = tx
                .send(Ok(StreamEvent::TextDelta("done".to_string())))
                .await;
            let _ = tx
                .send(Ok(StreamEvent::MessageEnd {
                    stop_reason: Some("end_turn".to_string()),
                }))
                .await;
        });
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        "jcode-compaction-test"
    }

    fn supports_compaction(&self) -> bool {
        true
    }

    fn context_window(&self) -> usize {
        1_000
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self)
    }
}

#[tokio::test]
async fn remote_hard_compaction_preserves_message_drop_metrics() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(JcodeCompactionStreamProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new(provider, registry);
    for index in 0..12 {
        agent.add_message(
            Role::User,
            vec![ContentBlock::Text {
                text: format!("message {index} {}", "x".repeat(1_000)),
                cache_control: None,
            }],
        );
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    agent.run_turn_streaming_mpsc(tx).await.unwrap();

    while let Ok(event) = rx.try_recv() {
        if let ServerEvent::Compaction {
            trigger,
            messages_dropped,
            messages_compacted,
            ..
        } = event
        {
            assert_eq!(trigger, "hard_compact");
            assert!(messages_dropped.is_some_and(|count| count > 0));
            assert_eq!(messages_compacted, messages_dropped);
            return;
        }
    }

    panic!("hard compaction must reach remote clients with its drop metrics");
}
