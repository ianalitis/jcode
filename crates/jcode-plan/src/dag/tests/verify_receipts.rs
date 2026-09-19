// --- Verify gates require receipts (R7) -------------------------------------

fn verify_gate_ready() -> TaskGraph {
    let mut g = dag(Mode::Deep, vec![spec("impl1", NodeKind::Implement)]);
    dispatch(&mut g, "impl1", "w0");
    complete_node(&mut g, "impl1", "w0", sim::deep_artifact("did impl1")).unwrap();
    dispatch_with_attempt(&mut g, "plan::gate", "w1", "sim/attempt");
    assert_eq!(g.get("plan::gate").unwrap().kind, NodeKind::Verify);
    g
}

#[test]
fn deep_verify_gate_rejects_pass_without_receipt() {
    let mut g = verify_gate_ready();
    let err = complete_node(
        &mut g,
        "plan::gate",
        "w1",
        HandoffArtifact {
            validation: Some("cargo test passed".into()),
            ..HandoffArtifact::brief("audited impl1; clean")
        },
    )
    .unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");
    assert!(!g.get("plan::gate").unwrap().is_done());
}

#[test]
fn deep_verify_gate_rejects_failed_or_malformed_receipt() {
    let mut g = verify_gate_ready();
    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    artifact
        .receipts
        .push(sim::command_receipt("cargo test", 101));
    let err = complete_node(&mut g, "plan::gate", "w1", artifact).unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");

    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    let mut bad = sim::command_receipt("cargo test", 0);
    bad.stdout_sha256 = "nope".into();
    artifact.receipts.push(bad);
    let err = complete_node(&mut g, "plan::gate", "w1", artifact).unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");
}

fn assert_verify_rejects_model_call_exit(exit_code: i32) {
    let mut g = verify_gate_ready();
    let mut receipt = sim::command_receipt("offline model call", exit_code);
    receipt.kind = jcode_attempt_types::ReceiptKind::ModelCall;
    assert_eq!(receipt.attempt_id, "sim/attempt");
    jcode_attempt_types::validate_receipt_shape(&receipt).unwrap();
    jcode_attempt_types::validate_telemetry_disabled(&receipt).unwrap();
    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    artifact.receipts.push(receipt);
    let err = complete_node(&mut g, "plan::gate", "w1", artifact).unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");
    assert!(err.to_string().contains("needs exit 0"), "{err}");
    let gate = g.get("plan::gate").unwrap();
    assert_eq!(gate.status, NodeStatus::Running);
    assert!(gate.output.is_none());
}

#[test]
fn deep_verify_gate_rejects_model_call_exit_1() {
    assert_verify_rejects_model_call_exit(1);
}

#[test]
fn deep_verify_gate_rejects_model_call_exit_124() {
    assert_verify_rejects_model_call_exit(124);
}

#[test]
fn deep_verify_gate_rejects_model_call_exit_130() {
    assert_verify_rejects_model_call_exit(130);
}

#[test]
fn deep_verify_gate_preserves_successful_model_call_receipts() {
    for exit_code in [Some(0), None] {
        let mut g = verify_gate_ready();
        let mut receipt = sim::command_receipt("offline model call", 0);
        receipt.kind = jcode_attempt_types::ReceiptKind::ModelCall;
        receipt.exit_code = exit_code;
        assert_eq!(receipt.attempt_id, "sim/attempt");
        let mut artifact = HandoffArtifact::brief("audited impl1; clean");
        artifact.receipts.push(receipt);
        complete_node(&mut g, "plan::gate", "w1", artifact).unwrap();
        assert!(g.get("plan::gate").unwrap().is_done());
        assert!(g.all_terminal());
    }
}

#[test]
fn deep_verify_gate_rejects_local_model_receipt_without_telemetry_off() {
    let mut g = verify_gate_ready();
    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    let mut r = sim::command_receipt("needle classify", 0);
    r.kind = jcode_attempt_types::ReceiptKind::LocalModel;
    r.exit_code = None;
    artifact.receipts.push(r);
    let err = complete_node(&mut g, "plan::gate", "w1", artifact).unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");

    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    let mut r = sim::command_receipt("needle classify", 0);
    r.kind = jcode_attempt_types::ReceiptKind::LocalModel;
    r.exit_code = None;
    r.effective_telemetry
        .insert("DO_NOT_TRACK".into(), "1".into());
    artifact.receipts.push(r);
    complete_node(&mut g, "plan::gate", "w1", artifact).unwrap();
}

#[test]
fn deep_verify_gate_binds_receipts_to_the_gate_attempt() {
    let mut g = verify_gate_ready();
    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    let mut r = sim::command_receipt("cargo test", 0);
    r.attempt_id = "some-other-attempt".into();
    artifact.receipts.push(r);
    let err = complete_node(&mut g, "plan::gate", "w1", artifact).unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");
    assert!(!g.get("plan::gate").unwrap().is_done());
}

#[test]
fn deep_verify_gate_rejects_receipts_from_mixed_attempts_when_gate_unfrozen() {
    // Legacy/unfrozen gate (no attempt id): receipts still may not mix executions.
    let mut g = dag(Mode::Deep, vec![spec("impl1", NodeKind::Implement)]);
    dispatch(&mut g, "impl1", "w0");
    complete_node(&mut g, "impl1", "w0", sim::deep_artifact("did impl1")).unwrap();
    dispatch(&mut g, "plan::gate", "w1");
    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    let mut a = sim::command_receipt("cargo test", 0);
    a.attempt_id = "attempt-a".into();
    let mut b = sim::command_receipt("cargo build", 0);
    b.attempt_id = "attempt-b".into();
    artifact.receipts.push(a);
    artifact.receipts.push(b);
    let err = complete_node(&mut g, "plan::gate", "w1", artifact).unwrap_err();
    assert!(matches!(err, DagError::MissingReceipt { .. }), "{err}");
}

#[test]
fn deep_verify_gate_passes_with_green_receipt() {
    let mut g = verify_gate_ready();
    let mut artifact = HandoffArtifact::brief("audited impl1; clean");
    artifact
        .receipts
        .push(sim::command_receipt("cargo test -p x", 0));
    complete_node(&mut g, "plan::gate", "w1", artifact).unwrap();
    assert!(g.get("plan::gate").unwrap().is_done());
    assert!(g.all_terminal());
}

#[test]
fn critique_gate_and_light_mode_need_no_receipt() {
    let mut g = dag(Mode::Deep, vec![spec("a", NodeKind::Explore)]);
    dispatch(&mut g, "a", "w0");
    complete_node(&mut g, "a", "w0", sim::deep_artifact("did a")).unwrap();
    dispatch(&mut g, "plan::gate", "w1");
    assert_eq!(g.get("plan::gate").unwrap().kind, NodeKind::Critique);
    complete_node(
        &mut g,
        "plan::gate",
        "w1",
        HandoffArtifact::brief("audited a; clean"),
    )
    .unwrap();

    let mut g = dag(Mode::Light, vec![spec("impl1", NodeKind::Implement)]);
    dispatch(&mut g, "impl1", "w0");
    complete_node(&mut g, "impl1", "w0", HandoffArtifact::brief("done")).unwrap();
    assert!(g.all_terminal());
}
