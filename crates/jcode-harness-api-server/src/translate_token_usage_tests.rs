#[test]
fn token_usage_preserves_cache_creation_and_missing_counters() {
    let mut state = BridgeState {
        session_id: Some("s1".into()),
        ..Default::default()
    };
    for cache_creation_input in [None, Some(0), Some(42)] {
        let mut legacy = json!({
            "type": "tokens", "input": 10, "output": 5, "cache_read_input": 2
        });
        if let Some(tokens) = cache_creation_input {
            legacy["cache_creation_input"] = json!(tokens);
        }
        let frames = state.legacy_event_to_api(&legacy);
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].event,
            ApiEvent::TokenUsage {
                session_id: "s1".into(),
                input: 10,
                output: 5,
                cache_read_input: Some(2),
                cache_creation_input,
            }
        );
    }
}
