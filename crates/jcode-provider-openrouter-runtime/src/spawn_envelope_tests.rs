use super::openrouter_provider_impl::{
    settle_spawn_open_result, spawn_envelope_metered_budget, validate_spawn_request_cost_bounds,
    wrap_spawn_enforced_stream,
};
use futures::stream;
use jcode_attempt_types::{DataClass, SpawnExecutionEnvelope};
use jcode_message_types::StreamEvent;
use jcode_provider_core::EventStream;

#[test]
fn concrete_openrouter_request_is_refused_without_enforceable_cost_bounds() {
    let error = validate_spawn_request_cost_bounds("deepseek/deepseek-v4-flash-0731")
        .expect_err("reservation accounting alone cannot guarantee a hard cap");

    assert!(
        error
            .to_string()
            .contains("no enforceable per-request input, output, and pricing bounds")
    );
}

#[test]
fn pre_stream_error_keeps_the_full_reservation_ambiguous() {
    let envelope = SpawnExecutionEnvelope::new(Some(100), Some(5), Some(DataClass::Public), None);
    let reservation_id = "spawn:pre-stream-error";
    envelope
        .ledger
        .reserve(reservation_id, 100)
        .expect("reserve child budget");
    let result: anyhow::Result<EventStream> = Err(anyhow::anyhow!(
        "transport failed after billing became ambiguous"
    ));

    let result = settle_spawn_open_result(result, &envelope, reservation_id);

    assert!(result.is_err());
    assert_eq!(envelope.ledger.exposure_micro_usd(), 100);
    assert_eq!(
        envelope.ledger.get(reservation_id).unwrap().state,
        jcode_attempt_types::ReservationState::Ambiguous
    );
}

#[tokio::test]
async fn slow_consumer_cannot_hold_the_stream_past_the_child_deadline() {
    let envelope = SpawnExecutionEnvelope::new(Some(100), Some(1), Some(DataClass::Public), None);
    let reservation_id = "spawn:slow-consumer";
    envelope
        .ledger
        .reserve(reservation_id, 100)
        .expect("reserve child budget");
    let source: EventStream = Box::pin(stream::iter(
        (0..200).map(|_| Ok(StreamEvent::TextDelta("x".to_string()))),
    ));
    let _unconsumed = wrap_spawn_enforced_stream(
        source,
        envelope.clone(),
        reservation_id.to_string(),
        100,
        envelope.deadline_at().map(tokio::time::Instant::from_std),
    );

    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;

    assert_eq!(
        envelope.ledger.get(reservation_id).unwrap().state,
        jcode_attempt_types::ReservationState::Ambiguous
    );
}

#[test]
fn only_a_positive_envelope_budget_enters_metered_enforcement() {
    // An included-subscription spawn carries no per-token budget, so it must
    // take the shared no-budget spawn contract instead of being refused for a
    // missing metered reservation. A zero budget is equally unmeterable.
    for (budget, expected) in [(None, None), (Some(0), None), (Some(100), Some(100))] {
        let envelope =
            SpawnExecutionEnvelope::new(budget, Some(30), Some(DataClass::Private), None);
        assert_eq!(
            spawn_envelope_metered_budget(&envelope),
            expected,
            "budget {budget:?}"
        );
    }
}
