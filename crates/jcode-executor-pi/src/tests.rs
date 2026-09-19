use super::*;
use chrono::Utc;
use jcode_attempt_types::{AttemptRecord, DataClass, Effort, LocalBudget, RouteClass};
use std::os::unix::fs::PermissionsExt;

fn frozen(model: &str) -> FrozenAttempt {
    AttemptRecord {
        task_id: "t".into(),
        attempt_id: "t/n1/a-1".into(),
        node_id: "n1".into(),
        provider: "pi".into(),
        model_exact: model.into(),
        endpoint: "pi-rpc".into(),
        route_class: RouteClass::IncludedSubscription,
        effort: Effort::Low,
        tool_allowlist: vec![],
        data_class: DataClass::Synthetic,
        deadline_secs: 60,
        budget: LocalBudget::default(),
        prompt_hash: "a".repeat(64),
        policy_version: "test".into(),
    }
    .freeze(Utc::now())
    .unwrap()
}

fn config(binary: &std::path::Path) -> PiConfig {
    PiConfig {
        binary: binary.to_path_buf(),
        provider: Some("anthropic".into()),
        model: Some("approved/model".into()),
        cwd: std::env::temp_dir(),
    }
}

fn write_stub(name: &str, body: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "jcode-pi-stub-{}-{}-{name}.sh",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, body).unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

// --- pure protocol ---------------------------------------------------------

#[test]
fn protocol_folds_text_usage_and_settlement() {
    let mut proto = PiProtocol::default();
    proto
        .ingest_line(r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"stub "}}"#)
        .unwrap();
    proto
        .ingest_line(r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"reply"}}"#)
        .unwrap();
    proto
        .ingest_line(
            r#"{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"stub reply"}],"usage":{"input":10,"output":5,"cost":{"total":0.0002}}}}"#,
        )
        .unwrap();
    proto.ingest_line(r#"{"type":"agent_settled"}"#).unwrap();
    assert_eq!(proto.final_text(), "stub reply");
    assert_eq!(proto.streamed_text, "stub reply");
    assert_eq!(proto.input_tokens, 10);
    assert_eq!(proto.output_tokens, 5);
    assert_eq!(proto.cost_usd_micros, Some(200));
    assert!(proto.settled);
}

#[test]
fn protocol_reads_stats_response_and_reports_errors() {
    let mut proto = PiProtocol::default();
    proto
        .ingest_line(
            r#"{"type":"response","command":"get_session_stats","success":true,"data":{"cost":0.0002,"tokens":{"input":10,"output":5,"total":15}}}"#,
        )
        .unwrap();
    assert!(proto.stats_seen);
    assert_eq!(proto.cost_usd_micros, Some(200));

    let mut proto = PiProtocol::default();
    proto
        .ingest_line(r#"{"type":"error","error":"boom"}"#)
        .unwrap();
    assert_eq!(proto.error.as_deref(), Some("boom"));

    let mut proto = PiProtocol::default();
    assert!(proto.ingest_line("not json").is_err());
}

// --- argv ------------------------------------------------------------------

#[test]
fn argv_always_disables_tools_and_binds_the_model() {
    let attempt = frozen("approved/model");
    let argv = build_argv(&config(std::path::Path::new("/usr/bin/pi")), &attempt).unwrap();
    assert!(argv.contains(&"--no-tools".to_string()));
    assert!(argv.contains(&"--no-session".to_string()));
    assert_eq!(argv.last().unwrap(), "approved/model");
}

#[test]
fn argv_refuses_a_model_that_does_not_match_the_frozen_attempt() {
    let attempt = frozen("approved/model");
    let mut cfg = config(std::path::Path::new("/usr/bin/pi"));
    cfg.model = Some("other/model".into());
    assert_eq!(
        build_argv(&cfg, &attempt),
        Err(PiError::ModelMismatch {
            frozen: "approved/model".into(),
            configured: "other/model".into(),
        })
    );
}

// --- driver ----------------------------------------------------------------

const OK_STUB: &str = r#"#!/bin/sh
read prompt
printf '%s\n' '{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"stub "}}'
printf '%s\n' '{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"stub reply"}],"usage":{"input":10,"output":5,"cost":{"total":0.0002}}}}'
printf '%s\n' '{"type":"agent_settled"}'
read stats
printf '%s\n' '{"type":"response","command":"get_session_stats","success":true,"data":{"cost":0.0002,"tokens":{"input":10,"output":5,"total":15}}}'
"#;

#[test]
fn run_completes_and_yields_a_valid_receipt() {
    let binary = write_stub("ok", OK_STUB);
    let attempt = frozen("approved/model");
    let cfg = config(&binary);
    let result = rt()
        .block_on(run_pi_attempt(
            &cfg,
            &attempt,
            "hello",
            Duration::from_secs(5),
            None,
        ))
        .unwrap();
    assert_eq!(
        result.outcome,
        PiOutcome::Completed {
            text: "stub reply".into()
        }
    );
    assert_eq!(result.receipt.exit_code, Some(0));
    assert_eq!(result.receipt.attempt_id, "t/n1/a-1");
    assert_eq!(result.receipt.stdout_sha256, sha256_hex(b"stub reply"));
    assert_eq!(result.receipt.usage.as_ref().unwrap().micro_usd, Some(200));
    assert!(validate_receipt_for_gate(&result.receipt, &attempt).is_ok());
    let _ = std::fs::remove_file(&binary);
}

const STALL_STUB: &str = r#"#!/bin/sh
read prompt
printf '%s\n' '{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"partial"}}'
sleep 30
"#;

#[test]
fn run_times_out_mid_stream_and_still_digests_partial_output() {
    let binary = write_stub("stall", STALL_STUB);
    let attempt = frozen("approved/model");
    let cfg = config(&binary);
    let result = rt()
        .block_on(run_pi_attempt(
            &cfg,
            &attempt,
            "hello",
            Duration::from_secs(1),
            None,
        ))
        .unwrap();
    assert_eq!(result.outcome, PiOutcome::DeadlineExceeded);
    assert_eq!(result.receipt.exit_code, Some(124));
    assert_eq!(result.receipt.stdout_sha256, sha256_hex(b"partial"));
    assert!(validate_receipt_for_gate(&result.receipt, &attempt).is_ok());
    let _ = std::fs::remove_file(&binary);
}

#[test]
fn run_respects_a_preset_cancel_signal() {
    let binary = write_stub("cancel", STALL_STUB);
    let attempt = frozen("approved/model");
    let cfg = config(&binary);
    let cancel: CancelSignal = Arc::new(AtomicBool::new(true));
    let result = rt()
        .block_on(run_pi_attempt(
            &cfg,
            &attempt,
            "hello",
            Duration::from_secs(5),
            Some(cancel),
        ))
        .unwrap();
    assert_eq!(result.outcome, PiOutcome::Cancelled);
    assert_eq!(result.receipt.exit_code, Some(130));
    let _ = std::fs::remove_file(&binary);
}

// A1: agent_settled means idle, not successful. These fixtures use only the
// documented Pi 0.84.2 RPC events and a local shell stub, never a Pi provider.
const REJECTED_PROMPT: &str =
    r#"{"type":"response","command":"prompt","success":false,"error":"prompt rejected"}"#;
const ASSISTANT_ERROR: &str = r#"{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"partial"}],"stopReason":"error","errorMessage":"provider failed"}}"#;
const ASSISTANT_ABORT: &str = r#"{"type":"message_end","message":{"role":"assistant","content":[],"stopReason":"aborted","errorMessage":"request aborted"}}"#;
const RETRY_START: &str = r#"{"type":"auto_retry_start","attempt":1,"maxAttempts":1,"delayMs":0,"errorMessage":"provider failed"}"#;
const RETRY_FAILED: &str =
    r#"{"type":"auto_retry_end","success":false,"attempt":1,"finalError":"retries exhausted"}"#;
const RETRY_SUCCEEDED: &str = r#"{"type":"auto_retry_end","success":true,"attempt":1}"#;
const ASSISTANT_OK: &str = r#"{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"recovered"}],"stopReason":"stop"}}"#;

fn protocol_events(events: &[&str]) -> PiProtocol {
    let mut proto = PiProtocol::default();
    for event in events {
        proto.ingest_line(event).unwrap();
    }
    proto
}

fn run_synthetic_events(events: &[&str]) -> PiResult {
    let script = format!(
        "#!/bin/sh\nread prompt\ncat <<'A1_EVENTS'\n{}\n{{\"type\":\"agent_settled\"}}\nA1_EVENTS\nread stats\nprintf '%s\\n' '{{\"type\":\"response\",\"command\":\"get_session_stats\",\"success\":true,\"data\":{{\"cost\":0}}}}'\n",
        events.join("\n")
    );
    let binary = write_stub("a1-outcome", &script);
    let attempt = frozen("approved/model");
    let result = rt().block_on(run_pi_attempt(
        &config(&binary),
        &attempt,
        "synthetic prompt",
        Duration::from_secs(5),
        None,
    ));
    std::fs::remove_file(binary).unwrap();
    let result = result.unwrap();
    assert!(validate_receipt_for_gate(&result.receipt, &attempt).is_ok());
    result
}

fn assert_failed_receipt(events: &[&str], error: &str) {
    let result = run_synthetic_events(events);
    assert_eq!(
        result.outcome,
        PiOutcome::Failed {
            message: error.into()
        }
    );
    assert_eq!(result.receipt.exit_code, Some(1));
}

#[test]
fn rejected_prompt_cannot_produce_success_after_settlement() {
    assert_failed_receipt(&[REJECTED_PROMPT], "prompt rejected");
}

#[test]
fn terminal_assistant_error_cannot_produce_success_after_settlement() {
    assert_failed_receipt(&[ASSISTANT_ERROR], "provider failed");
}

#[test]
fn terminal_assistant_abort_cannot_produce_success_after_settlement() {
    assert_failed_receipt(&[ASSISTANT_ABORT], "request aborted");
}

#[test]
fn exhausted_retries_cannot_produce_success_after_settlement() {
    assert_failed_receipt(
        &[ASSISTANT_ERROR, RETRY_START, ASSISTANT_ERROR, RETRY_FAILED],
        "retries exhausted",
    );
}

#[test]
fn retry_success_event_does_not_erase_an_assistant_abort() {
    // Pi can emit retry success for any non-error stop reason, including abort.
    assert_failed_receipt(
        &[
            ASSISTANT_ERROR,
            RETRY_START,
            ASSISTANT_ABORT,
            RETRY_SUCCEEDED,
        ],
        "request aborted",
    );
}

#[test]
fn intermediate_retry_error_can_recover_before_settlement() {
    let events = [ASSISTANT_ERROR, RETRY_START, ASSISTANT_OK, RETRY_SUCCEEDED];
    let proto = protocol_events(&events);
    assert!(
        proto.error.is_none(),
        "intermediate errors are not terminal"
    );
    let result = run_synthetic_events(&events);
    assert_eq!(
        result.outcome,
        PiOutcome::Completed {
            text: "recovered".into()
        }
    );
    assert_eq!(result.receipt.exit_code, Some(0));
    assert_eq!(result.receipt.stdout_sha256, sha256_hex(b"recovered"));
}

#[test]
fn agent_end_terminal_assistant_error_cannot_produce_success() {
    assert_failed_receipt(
        &[
            r#"{"type":"agent_end","messages":[{"role":"assistant","content":[],"stopReason":"error","errorMessage":"terminal failure"}],"willRetry":false}"#,
        ],
        "terminal failure",
    );
}

#[test]
fn settlement_keeps_explicit_rejection_and_retry_exhaustion_sticky() {
    for (event, expected) in [
        (REJECTED_PROMPT, "prompt rejected"),
        (RETRY_FAILED, "retries exhausted"),
    ] {
        let proto = protocol_events(&[
            event,
            ASSISTANT_OK,
            RETRY_SUCCEEDED,
            r#"{"type":"agent_settled"}"#,
        ]);
        assert!(proto.settled);
        assert_eq!(proto.error.as_deref(), Some(expected));
    }
}

#[test]
fn terminal_failures_without_error_text_still_fail() {
    for event in [
        r#"{"type":"response","command":"prompt","success":false}"#,
        r#"{"type":"message_end","message":{"role":"assistant","stopReason":"error"}}"#,
        r#"{"type":"message_end","message":{"role":"assistant","stopReason":"aborted"}}"#,
        r#"{"type":"auto_retry_end","success":false}"#,
    ] {
        let proto = protocol_events(&[event, r#"{"type":"agent_settled"}"#]);
        assert!(proto.error.as_ref().is_some_and(|error| !error.is_empty()));
    }
}
