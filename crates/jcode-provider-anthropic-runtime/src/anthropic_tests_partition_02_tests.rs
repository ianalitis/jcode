#[test]
fn ping_keepalive_emits_streaming_phase_event() {
    // Issue #451: during silent reasoning phases, `ping` events can be the
    // only upstream traffic. They must surface as a StreamEvent so the client
    // stall guard sees activity instead of cancelling a healthy stream.
    let mut state = SseStreamState::default();
    let event = SseEvent {
        event_type: "ping".to_string(),
        data: r#"{"type": "ping"}"#.to_string(),
    };
    let events = process_sse_event(&event, &mut state, true);
    assert!(
        events.iter().any(|e| matches!(
            e,
            StreamEvent::ConnectionPhase {
                phase: jcode_message_types::ConnectionPhase::Streaming
            }
        )),
        "expected ping to emit a Streaming ConnectionPhase event, got {events:?}"
    );
}

#[test]
fn test_anthropic_opus_5_low_effort_reaches_the_wire() {
    // Benchmark campaigns pin `claude-opus-5` at `low` effort. Opus 5 also
    // *defaults* to `low` (jcode's default model/effort pairing), and an
    // explicit `low` must survive normalization, must NOT be silently
    // promoted, and must land in `output_config.effort` on the request.
    assert!(AnthropicProvider::model_supports_output_effort(
        "claude-opus-5"
    ));
    assert_eq!(
        AnthropicProvider::default_reasoning_effort_for_model("claude-opus-5").as_deref(),
        Some("low"),
    );
    assert_eq!(
        AnthropicProvider::normalize_reasoning_effort("low").as_deref(),
        Some("low"),
    );
    // Downward selection is never clamped upward toward the model default.
    assert_eq!(
        AnthropicProvider::actual_effort_for_model("claude-opus-5", "low"),
        "low",
    );
    assert_eq!(
        AnthropicProvider::store_effort_for_model("claude-opus-5", "low"),
        "low",
    );

    let provider = AnthropicProvider::new();
    *provider
        .model
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = "claude-opus-5".to_string();
    provider.set_reasoning_effort("low").unwrap();
    assert_eq!(provider.reasoning_effort().as_deref(), Some("low"));

    let (thinking, output_config, _temp) =
        provider.build_reasoning_request_parts_inner("claude-opus-5", true, false);
    assert_eq!(
        output_config
            .expect("explicit low effort should set output_config")
            .effort,
        "low",
    );
    // Opus 5 rejects `thinking.type.enabled`; it requires adaptive thinking.
    assert!(matches!(thinking, Some(ApiThinking::Adaptive { .. })));
}

/// A `content_block_start` carrying an unrecognized block type must still
/// deserialize. Before the `Unknown` catch-all the whole event failed to parse
/// and was dropped, so an unknown *tool* block produced a turn that reported
/// `stop_reason: tool_use` with no tool call for the agent to run.
#[test]
fn test_anthropic_unknown_content_block_start_does_not_drop_event() {
    for block_type in [
        "server_tool_use",
        "web_search_tool_result",
        "some_future_block",
    ] {
        let mut state = SseStreamState::default();
        let event = SseEvent {
            event_type: "content_block_start".to_string(),
            data: serde_json::json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": block_type, "id": "srvtoolu_1", "name": "web_search"}
            })
            .to_string(),
        };
        let events = process_sse_event(&event, &mut state, false);
        assert!(
            events.is_empty(),
            "{block_type}: unknown block must not synthesize stream events"
        );
        assert!(
            state.current_tool_use.is_none(),
            "{block_type}: unknown block must not start tool accumulation"
        );
        assert!(
            !state.current_thinking_block,
            "{block_type}: unknown block must not start a thinking block"
        );
    }
}
