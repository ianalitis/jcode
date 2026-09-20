use super::*;
use crate::single_send_tests::{
    ServerAction, TestServer, fixture_request, response, success_response, synthetic_provider,
};
use jcode_attempt_types::{AttemptRecord, DataClass, Effort, LocalBudget};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn frozen(deadline_secs: u64, max_micro_usd: u64) -> FrozenAttempt {
    AttemptRecord {
        task_id: "t".into(),
        attempt_id: format!("t/n1/a-{}", uuid::Uuid::new_v4()),
        node_id: "n1".into(),
        provider: "openrouter".into(),
        model_exact: "approved/model".into(),
        endpoint: "loopback".into(),
        route_class: RouteClass::MeteredRemote,
        effort: Effort::Medium,
        tool_allowlist: vec![],
        data_class: DataClass::Synthetic,
        router: None,
        deadline_secs,
        budget: LocalBudget {
            max_input_bytes: 4096,
            max_output_bytes: 4096,
            max_micro_usd,
            max_generations: 1,
        },
        prompt_hash: "p".repeat(64),
        policy_version: "test".into(),
    }
    .freeze(Utc::now())
    .unwrap()
}

fn run(
    server: &TestServer,
    attempt: &FrozenAttempt,
    ledger: &LocalLedger,
    expected: Option<Value>,
    cancel: Option<CancelSignal>,
) -> Result<AttemptResult, CallerError> {
    let provider = synthetic_provider(server.api_base.clone());
    let messages = vec![Message::user("approved prompt")];
    let expected = expected.unwrap_or_else(|| fixture_request(&messages));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(run_frozen_attempt(
        &provider,
        attempt,
        ledger,
        expected,
        &server.destination,
        &messages,
        &[],
        "",
        cancel,
    ))
}

// --- ledger ---------------------------------------------------------------

#[test]
fn ledger_reserve_is_atomic_against_cap() {
    let l = LocalLedger::new(100);
    l.reserve("a", 60).unwrap();
    assert_eq!(
        l.reserve("b", 50),
        Err(LedgerError::CapExceeded {
            cap: 100,
            held: 60,
            requested: 50
        })
    );
    l.reserve("b", 40).unwrap();
    assert_eq!(
        l.reserve("b", 1),
        Err(LedgerError::DuplicateAttempt("b".into()))
    );
    assert_eq!(l.exposure_micro_usd(), 100);
}

#[test]
fn ledger_settle_releases_only_the_unused_part_and_ambiguous_holds() {
    let l = LocalLedger::new(100);
    l.reserve("a", 60).unwrap();
    l.settle("a", 10).unwrap();
    assert_eq!(l.exposure_micro_usd(), 10);
    l.reserve("b", 80).unwrap();
    l.mark_ambiguous("b").unwrap();
    assert_eq!(
        l.exposure_micro_usd(),
        90,
        "ambiguous keeps full reservation"
    );
    assert!(l.reserve("c", 20).is_err());
    l.reconcile("b", 5).unwrap();
    assert_eq!(l.exposure_micro_usd(), 15);
    l.reserve("c", 20).unwrap();
    assert_eq!(
        l.settle("zzz", 0),
        Err(LedgerError::UnknownAttempt("zzz".into()))
    );
}

#[test]
fn ledger_concurrent_reservations_never_exceed_cap() {
    let l = LocalLedger::new(1000);
    let handles: Vec<_> = (0..32)
        .map(|i| {
            let l = l.clone();
            std::thread::spawn(move || l.reserve(&format!("a{i}"), 100).is_ok())
        })
        .collect();
    let ok = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .filter(|ok| *ok)
        .count();
    assert_eq!(ok, 10);
    assert_eq!(l.exposure_micro_usd(), 1000);
}

// --- caller ---------------------------------------------------------------

#[test]
fn completed_attempt_sends_once_settles_and_yields_valid_receipt() {
    let server = TestServer::spawn(vec![success_response()]);
    let attempt = frozen(5, 500);
    let ledger = LocalLedger::new(1_000);
    let r = run(&server, &attempt, &ledger, None, None).unwrap();
    assert_eq!(r.outcome, AttemptOutcome::Completed { text: "ok".into() });
    assert_eq!(r.receipt.exit_code, Some(0));
    assert_eq!(r.receipt.attempt_id, attempt.attempt_id());
    assert_eq!(r.receipt.stdout_sha256, sha256_hex(b"ok"));
    assert!(validate_receipt_for_gate(&r.receipt, &attempt).is_ok());
    assert_eq!(server.join(), 1);
    let res = ledger.get(attempt.attempt_id()).unwrap();
    assert_eq!(res.state, ReservationState::Settled);
    // No usage-reported cost: settled at the reservation, never above it.
    assert_eq!(res.settled_micro_usd, Some(500));
}

#[test]
fn guard_mismatch_sends_zero_and_releases_reservation() {
    let server = TestServer::spawn(vec![success_response()]);
    let attempt = frozen(5, 500);
    let ledger = LocalLedger::new(1_000);
    let tampered = serde_json::json!({"model": "other/model", "messages": []});
    let r = run(&server, &attempt, &ledger, Some(tampered), None).unwrap();
    assert!(
        matches!(r.outcome, AttemptOutcome::Failed { sent: false, .. }),
        "{:?}",
        r.outcome
    );
    assert_eq!(r.receipt.exit_code, Some(2));
    assert!(validate_receipt_for_gate(&r.receipt, &attempt).is_ok());
    assert_eq!(server.join(), 0, "zero sends on body mismatch");
    assert_eq!(
        ledger.get(attempt.attempt_id()).unwrap().settled_micro_usd,
        Some(0)
    );
}

#[test]
fn retryable_status_is_one_send_and_ambiguous_exposure() {
    let server = TestServer::spawn(vec![response("429 Too Many Requests", "slow down")]);
    let attempt = frozen(5, 500);
    let ledger = LocalLedger::new(1_000);
    let r = run(&server, &attempt, &ledger, None, None).unwrap();
    assert!(
        matches!(r.outcome, AttemptOutcome::Failed { sent: true, .. }),
        "{:?}",
        r.outcome
    );
    assert_eq!(r.receipt.exit_code, Some(1));
    assert_eq!(server.join(), 1, "a 429 is a closed attempt, not a retry");
    assert_eq!(
        ledger.get(attempt.attempt_id()).unwrap().state,
        ReservationState::Ambiguous
    );
    assert_eq!(ledger.exposure_micro_usd(), 500);
}

#[test]
fn deadline_mid_stream_yields_124_and_ambiguous_exposure() {
    let server = TestServer::spawn(vec![ServerAction::Stall { hold_ms: 1500 }]);
    let attempt = frozen(1, 500);
    let ledger = LocalLedger::new(1_000);
    let r = run(&server, &attempt, &ledger, None, None).unwrap();
    assert_eq!(r.outcome, AttemptOutcome::DeadlineExceeded);
    assert_eq!(r.receipt.exit_code, Some(124));
    assert_eq!(
        r.receipt.stdout_sha256,
        sha256_hex(b"partial"),
        "partial output is digested"
    );
    assert!(validate_receipt_for_gate(&r.receipt, &attempt).is_ok());
    assert_eq!(
        ledger.get(attempt.attempt_id()).unwrap().state,
        ReservationState::Ambiguous
    );
    assert_eq!(server.join(), 1);
}

#[test]
fn deadline_includes_provider_opening_time() {
    let server = TestServer::spawn(vec![ServerAction::Stall { hold_ms: 1100 }]);
    let provider = synthetic_provider(server.api_base.clone());
    let attempt = frozen(1, 500);
    let ledger = LocalLedger::new(1_000);
    let messages = vec![Message::user("approved prompt")];
    let expected = fixture_request(&messages);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (r, elapsed) = rt.block_on(async {
        let cache_guard = Arc::clone(&provider.models_cache).write_owned().await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(750)).await;
            drop(cache_guard);
        });
        let started = Instant::now();
        let result = run_frozen_attempt(
            &provider,
            &attempt,
            &ledger,
            expected,
            &server.destination,
            &messages,
            &[],
            "",
            None,
        )
        .await
        .unwrap();
        (result, started.elapsed())
    });
    let sends = server.join();
    let exposure = ledger.exposure_micro_usd();

    assert_eq!(r.outcome, AttemptOutcome::DeadlineExceeded);
    assert_eq!(r.receipt.exit_code, Some(124));
    assert_eq!(
        ledger.get(attempt.attempt_id()).unwrap().state,
        ReservationState::Ambiguous
    );
    assert!(
        elapsed < Duration::from_millis(1500),
        "one 1s absolute deadline must cover provider opening and streaming; elapsed={elapsed:?}, sends={sends}, exposure={exposure}"
    );
    assert_eq!(sends, 1, "exactly one send; elapsed={elapsed:?}");
    assert_eq!(exposure, 500, "ambiguous exposure must remain held");
}

#[test]
fn unrepresentably_large_deadline_does_not_panic() {
    let server = TestServer::spawn(vec![success_response()]);
    let attempt = frozen(u64::MAX, 500);
    let ledger = LocalLedger::new(1_000);
    let r = run(&server, &attempt, &ledger, None, None).unwrap();

    assert_eq!(r.outcome, AttemptOutcome::Completed { text: "ok".into() });
    assert_eq!(server.join(), 1);
    assert_eq!(ledger.exposure_micro_usd(), 500);
}

#[test]
fn cancel_signal_set_before_first_poll_sends_at_most_once_and_yields_130() {
    // The provider issues the HTTP send on a spawned task, so a signal that
    // is already set when the caller first polls stops consumption before or
    // just after that send. Either way: at most one send, no events
    // consumed, exposure held as ambiguous because the outcome is unknown.
    let server = TestServer::spawn(vec![ServerAction::Stall { hold_ms: 300 }]);
    let attempt = frozen(5, 500);
    let ledger = LocalLedger::new(1_000);
    let cancel: CancelSignal = Arc::new(AtomicBool::new(true));
    let r = run(&server, &attempt, &ledger, None, Some(cancel)).unwrap();
    assert_eq!(r.outcome, AttemptOutcome::Cancelled);
    assert_eq!(r.receipt.exit_code, Some(130));
    assert_eq!(r.receipt.stdout_sha256, sha256_hex(b""));
    assert!(validate_receipt_for_gate(&r.receipt, &attempt).is_ok());
    assert_eq!(
        ledger.get(attempt.attempt_id()).unwrap().state,
        ReservationState::Ambiguous
    );
    assert!(server.join() <= 1);
}

#[test]
fn refuses_non_metered_route_and_model_mismatch_before_any_send() {
    let server = TestServer::spawn(vec![success_response()]);
    let ledger = LocalLedger::new(1_000);

    let mut rec = AttemptRecord {
        route_class: RouteClass::Local,
        ..frozen(5, 1).record().clone()
    };
    rec.attempt_id = "local-1".into();
    let local = rec.clone().freeze(Utc::now()).unwrap();
    assert!(matches!(
        run(&server, &local, &ledger, None, None),
        Err(CallerError::NotMeteredRoute(_))
    ));

    rec.route_class = RouteClass::MeteredRemote;
    rec.model_exact = "other/model".into();
    rec.attempt_id = "mm-1".into();
    let mm = rec.freeze(Utc::now()).unwrap();
    assert!(matches!(
        run(&server, &mm, &ledger, None, None),
        Err(CallerError::ModelMismatch { .. })
    ));

    assert_eq!(server.join(), 0);
    assert_eq!(
        ledger.exposure_micro_usd(),
        0,
        "nothing reserved on pre-send refusal"
    );
}

#[test]
fn ledger_cap_blocks_the_send() {
    let server = TestServer::spawn(vec![success_response()]);
    let attempt = frozen(5, 500);
    let ledger = LocalLedger::new(100);
    assert!(matches!(
        run(&server, &attempt, &ledger, None, None),
        Err(CallerError::Ledger(LedgerError::CapExceeded { .. }))
    ));
    assert_eq!(server.join(), 0);
}

#[test]
fn same_attempt_id_cannot_run_twice() {
    let server = TestServer::spawn(vec![success_response(), success_response()]);
    let attempt = frozen(5, 10);
    let ledger = LocalLedger::new(1_000);
    run(&server, &attempt, &ledger, None, None).unwrap();
    assert!(matches!(
        run(&server, &attempt, &ledger, None, None),
        Err(CallerError::Ledger(LedgerError::DuplicateAttempt(_)))
    ));
    assert_eq!(server.join(), 1, "replaying an attempt id does not resend");
}
