use super::*;
use chrono::TimeZone;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 18, 20, 0, 0).unwrap()
}

fn record() -> AttemptRecord {
    AttemptRecord {
        task_id: "task-1".into(),
        attempt_id: "task-1/n3/a1".into(),
        node_id: "n3".into(),
        provider: "openai-oauth".into(),
        model_exact: "gpt-5.6-terra".into(),
        endpoint: "https://api.openai.com/v1/responses".into(),
        route_class: RouteClass::IncludedSubscription,
        effort: Effort::Medium,
        tool_allowlist: vec!["read".into()],
        data_class: DataClass::Private,
        router: None,
        deadline_secs: 60,
        budget: LocalBudget {
            max_input_bytes: 65_536,
            max_output_bytes: 16_384,
            max_micro_usd: 0,
            max_generations: 1,
        },
        prompt_hash: "a".repeat(64),
        policy_version: "2026-09-18".into(),
    }
}

fn receipt(attempt_id: &str) -> Receipt {
    Receipt {
        attempt_id: attempt_id.into(),
        kind: ReceiptKind::Command,
        cmd: "cargo test -p jcode-attempt-types --offline".into(),
        argv_hash: "b".repeat(64),
        cwd: "/repo".into(),
        exit_code: Some(0),
        stdout_sha256: "c".repeat(64),
        stderr_sha256: "d".repeat(64),
        started: now(),
        finished: now() + chrono::Duration::seconds(5),
        binary_id: "cargo@1.92.0".into(),
        usage: None,
        effective_telemetry: BTreeMap::new(),
        task_type: None,
    }
}

// --- DataClass admission -----------------------------------------------------

#[test]
fn data_class_defaults_to_private() {
    assert_eq!(DataClass::default(), DataClass::Private);
    let parsed: DataClass = serde_json::from_str("\"public\"").unwrap();
    assert_eq!(parsed, DataClass::Public);
}

#[test]
fn private_is_local_or_included_only() {
    assert!(DataClass::Private.is_remote_eligible(RouteClass::Local));
    assert!(DataClass::Private.is_remote_eligible(RouteClass::IncludedSubscription));
    assert!(!DataClass::Private.is_remote_eligible(RouteClass::MeteredRemote));
}

#[test]
fn secret_never_travels_anywhere() {
    for route in [
        RouteClass::Local,
        RouteClass::IncludedSubscription,
        RouteClass::MeteredRemote,
    ] {
        assert!(!DataClass::Secret.is_remote_eligible(route), "{route:?}");
    }
}

#[test]
fn public_and_synthetic_may_use_metered_routes() {
    assert!(DataClass::Public.is_remote_eligible(RouteClass::MeteredRemote));
    assert!(DataClass::Synthetic.is_remote_eligible(RouteClass::MeteredRemote));
}

// --- Banned router families ------------------------------------------------

#[test]
fn banned_families_through_openrouter_direct_and_nested() {
    assert!(is_banned_router_family("openrouter", "openai/gpt-5.6"));
    assert!(is_banned_router_family(
        "openrouter",
        "anthropic/claude-opus-5"
    ));
    assert!(is_banned_router_family("openrouter", "openrouter/auto"));
    assert!(is_banned_router_family(
        "openrouter",
        "openrouter/pareto-code"
    ));
    assert!(is_banned_router_family("openrouter", "auto"));
    assert!(is_banned_router_family(
        "open-inference",
        "vendor/claude-mirror"
    ));
}

#[test]
fn admitted_open_weight_and_direct_routes_are_not_banned() {
    assert!(!is_banned_router_family(
        "openrouter",
        "deepseek/deepseek-v4-flash-0731"
    ));
    assert!(!is_banned_router_family("openrouter", "z-ai/glm-5"));
    assert!(!is_banned_router_family("openai-oauth", "gpt-5.6-terra"));
    assert!(!is_banned_router_family("claude-oauth", "claude-fable-5-1"));
    assert!(!is_banned_router_family(
        "mlx-serve",
        "mlx-community/gemma-4-e2b-it-4bit"
    ));
}

// --- Freeze ----------------------------------------------------------------

#[test]
fn freeze_accepts_valid_record_and_exposes_read_only_view() {
    let frozen = record().freeze(now()).unwrap();
    assert_eq!(frozen.attempt_id(), "task-1/n3/a1");
    assert_eq!(frozen.record().model_exact, "gpt-5.6-terra");
    assert_eq!(frozen.frozen_at(), now());
    let json = serde_json::to_value(&frozen).unwrap();
    assert_eq!(json["record"]["data_class"], "private");
    assert_eq!(json["record"]["route_class"], "included_subscription");
    let back: FrozenAttempt = serde_json::from_value(json).unwrap();
    assert_eq!(back, frozen);
}

#[test]
fn freeze_rejects_private_data_on_metered_route() {
    let mut r = record();
    r.route_class = RouteClass::MeteredRemote;
    r.provider = "openrouter".into();
    r.model_exact = "deepseek/deepseek-v4-flash-0731".into();
    let err = r.freeze(now()).unwrap_err();
    assert!(
        matches!(err, FreezeError::DataClassNotEligible { .. }),
        "{err}"
    );
}

#[test]
fn freeze_rejects_banned_family_via_router_even_for_public_data() {
    let mut r = record();
    r.data_class = DataClass::Public;
    r.route_class = RouteClass::MeteredRemote;
    r.provider = "openrouter".into();
    r.model_exact = "openai/gpt-5.6".into();
    let err = r.freeze(now()).unwrap_err();
    assert!(
        matches!(err, FreezeError::BannedRouterFamily { .. }),
        "{err}"
    );
}

#[test]
fn freeze_rejects_latest_alias_empty_fields_and_zero_bounds() {
    let mut r = record();
    r.model_exact = "deepseek-v4:latest".into();
    assert!(matches!(r.freeze(now()), Err(FreezeError::LatestAlias(_))));

    let mut r = record();
    r.prompt_hash = "  ".into();
    assert!(matches!(r.freeze(now()), Err(FreezeError::EmptyField(f)) if f == "prompt_hash"));

    let mut r = record();
    r.deadline_secs = 0;
    assert!(matches!(r.freeze(now()), Err(FreezeError::ZeroDeadline)));

    let mut r = record();
    r.budget.max_generations = 0;
    assert!(matches!(r.freeze(now()), Err(FreezeError::ZeroGenerations)));
}

#[test]
fn local_budget_defaults_to_one_generation() {
    let b: LocalBudget = serde_json::from_str("{}").unwrap();
    assert_eq!(b.max_generations, 1);
    // The derived and serde defaults must agree, or a defaulted budget is
    // inadmissible (`zero generations`) by accident.
    assert_eq!(LocalBudget::default().max_generations, 1);
}

// --- Receipts --------------------------------------------------------------

#[test]
fn valid_command_receipt_passes_gate() {
    let frozen = record().freeze(now()).unwrap();
    validate_receipt_for_gate(&receipt(frozen.attempt_id()), &frozen).unwrap();
}

#[test]
fn receipt_attempt_id_must_match() {
    let frozen = record().freeze(now()).unwrap();
    let err = validate_receipt_for_gate(&receipt("other"), &frozen).unwrap_err();
    assert!(matches!(err, ReceiptError::AttemptIdMismatch { .. }));
}

#[test]
fn receipt_rejects_time_travel_missing_exit_and_bad_digests() {
    let frozen = record().freeze(now()).unwrap();
    let id = frozen.attempt_id();

    let mut r = receipt(id);
    r.finished = r.started - chrono::Duration::seconds(1);
    assert_eq!(
        validate_receipt_for_gate(&r, &frozen),
        Err(ReceiptError::FinishedBeforeStarted)
    );

    let mut r = receipt(id);
    r.exit_code = None;
    assert_eq!(
        validate_receipt_for_gate(&r, &frozen),
        Err(ReceiptError::MissingExitCode)
    );

    let mut r = receipt(id);
    r.kind = ReceiptKind::ModelCall;
    r.exit_code = None;
    assert!(
        validate_receipt_for_gate(&r, &frozen).is_ok(),
        "model calls need no exit code"
    );

    let mut r = receipt(id);
    r.stdout_sha256 = "deadbeef".into();
    assert_eq!(
        validate_receipt_for_gate(&r, &frozen),
        Err(ReceiptError::MissingDigest("stdout".into()))
    );

    let mut r = receipt(id);
    r.binary_id = String::new();
    assert_eq!(
        validate_receipt_for_gate(&r, &frozen),
        Err(ReceiptError::EmptyField("binary_id".into()))
    );
}

#[test]
fn local_model_receipt_must_record_telemetry_off() {
    let mut r = receipt("x");
    r.kind = ReceiptKind::LocalModel;
    assert_eq!(
        validate_telemetry_disabled(&r),
        Err(ReceiptError::TelemetryNotDisabled("DO_NOT_TRACK".into()))
    );
    r.effective_telemetry
        .insert("DO_NOT_TRACK".into(), "1".into());
    r.effective_telemetry
        .insert("NEEDLE_TELEMETRY".into(), "0".into());
    assert!(validate_telemetry_disabled(&r).is_ok());
    r.effective_telemetry
        .insert("NEEDLE_TELEMETRY".into(), "1".into());
    assert_eq!(
        validate_telemetry_disabled(&r),
        Err(ReceiptError::TelemetryNotDisabled(
            "NEEDLE_TELEMETRY".into()
        ))
    );
}

#[test]
fn local_model_receipt_requires_telemetry_off_at_the_gate() {
    let frozen = record().freeze(now()).unwrap();
    let mut r = receipt(frozen.attempt_id());
    r.kind = ReceiptKind::LocalModel;
    r.exit_code = None;
    assert_eq!(
        validate_receipt_for_gate(&r, &frozen),
        Err(ReceiptError::TelemetryNotDisabled("DO_NOT_TRACK".into())),
        "R11 must gate the receipt, not just the standalone validator"
    );
    r.effective_telemetry
        .insert("DO_NOT_TRACK".into(), "1".into());
    validate_receipt_for_gate(&r, &frozen).unwrap();
}

#[test]
fn command_receipt_has_no_telemetry_requirement() {
    assert!(validate_telemetry_disabled(&receipt("x")).is_ok());
}

// --- Confidence namespacing ------------------------------------------------

#[test]
fn confidence_variants_serialize_with_distinct_tags() {
    let variants = [
        Confidence::SelfReported {
            rung: SelfReportedRung::High,
        },
        Confidence::CalibratedGroup(CalibratedGroupConfidence {
            value: 0.9,
            model_hash: "jev-1.13.0".into(),
            task_class: "ci_scout".into(),
        }),
        Confidence::LocalExtractor(LocalExtractorConfidence {
            value: None,
            engine: "needle-3".into(),
            weights_hash: "sha256:...".into(),
        }),
    ];
    let tags: Vec<String> = variants
        .iter()
        .map(|c| {
            serde_json::to_value(c).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        tags,
        ["self_reported", "calibrated_group", "local_extractor"]
    );
    for c in &variants {
        let back: Confidence = serde_json::from_value(serde_json::to_value(c).unwrap()).unwrap();
        assert_eq!(&back, c);
    }
}

// --- Secretless packets ----------------------------------------------------

#[test]
fn clean_packet_has_no_secret_shapes() {
    let packet = serde_json::json!({
        "task": "parse cargo json",
        "text": "error[E0308]: mismatched types --> src/lib.rs:10:5",
        "tokens": ["sk-", "ghp_short", "AKIA"],
        "note": "the bearer of this message has a jwt eyJ in name only"
    });
    assert!(assert_no_secret_shapes(&packet).is_ok());
}

#[test]
fn secret_shapes_are_found_with_paths() {
    let packet = serde_json::json!({
        "env": { "OPENROUTER_API_KEY": "sk-or-v1-abcdefghijklmnopqrstuvwxyz0123456789" },
        "headers": [ { "Authorization": "Bearer abcdefghijklmnopqrstuvwxyz0123" } ],
        "jwt": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        "pem": "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----",
        "gh": "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123",
        "password": "hunter2"
    });
    let found = assert_no_secret_shapes(&packet).unwrap_err();
    let families: std::collections::BTreeSet<&str> = found.iter().map(|s| s.family).collect();
    for f in [
        "openrouter_key",
        "bearer_token",
        "jwt",
        "private_key_block",
        "github_token",
        "secret_named_field",
    ] {
        assert!(families.contains(f), "missing {f}: {found:?}");
    }
    assert!(found.iter().any(|s| s.path == "env.OPENROUTER_API_KEY"));
    assert!(found.iter().any(|s| s.path == "headers[0].Authorization"));
}

#[test]
fn secret_named_field_with_empty_value_is_allowed() {
    let packet = serde_json::json!({ "api_key": "" });
    assert!(assert_no_secret_shapes(&packet).is_ok());
}

// --- Dynamic router admission (J2) -----------------------------------------

fn auto_router_record(policy: Option<RouterPolicy>) -> AttemptRecord {
    let mut r = record();
    r.data_class = DataClass::Public;
    r.route_class = RouteClass::MeteredRemote;
    r.provider = "openrouter".into();
    r.model_exact = "openrouter/auto-beta".into();
    r.endpoint = "https://openrouter.ai/api/v1/chat/completions".into();
    r.router = policy;
    r
}

fn full_exclusions() -> RouterPolicy {
    RouterPolicy {
        excluded_models: vec!["openai/*".into(), "anthropic/*".into()],
        cost_tier: Some("low".into()),
    }
}

#[test]
fn auto_router_is_admitted_only_with_banned_family_exclusions() {
    let frozen = auto_router_record(Some(full_exclusions()))
        .freeze(now())
        .unwrap();
    assert_eq!(frozen.record().model_exact, "openrouter/auto-beta");

    let err = auto_router_record(None).freeze(now()).unwrap_err();
    assert!(
        matches!(err, FreezeError::RouterExclusionsMissing { .. }),
        "{err}"
    );

    let partial = RouterPolicy {
        excluded_models: vec!["openai/*".into()],
        cost_tier: None,
    };
    let err = auto_router_record(Some(partial)).freeze(now()).unwrap_err();
    assert!(
        matches!(err, FreezeError::RouterExclusionsMissing { .. }),
        "{err}"
    );
}

#[test]
fn pareto_router_is_refused_even_with_exclusions() {
    // Measured 2026-09-19: pareto-code ignores account and request exclusions
    // and served openai/gpt-5.6-sol at API rates.
    let mut r = auto_router_record(Some(full_exclusions()));
    r.model_exact = "openrouter/pareto-code".into();
    let err = r.freeze(now()).unwrap_err();
    assert!(
        matches!(err, FreezeError::BannedRouterFamily { .. }),
        "{err}"
    );
}

#[test]
fn router_policy_never_widens_data_class_eligibility() {
    let mut r = auto_router_record(Some(full_exclusions()));
    r.data_class = DataClass::Private;
    let err = r.freeze(now()).unwrap_err();
    assert!(
        matches!(err, FreezeError::DataClassNotEligible { .. }),
        "{err}"
    );
}

#[test]
fn concrete_router_slugs_ignore_router_policy() {
    assert!(check_router_admission("openrouter", "z-ai/glm-5", None).is_ok());
    assert!(
        check_router_admission("openrouter", "openai/gpt-5.6", Some(&full_exclusions())).is_err()
    );
    assert!(check_router_admission("openai-oauth", "gpt-5.6-terra", None).is_ok());
}

#[test]
fn router_receipt_must_name_a_served_model_outside_banned_families() {
    let frozen = auto_router_record(Some(full_exclusions()))
        .freeze(now())
        .unwrap();
    let mut rc = receipt(frozen.attempt_id());
    rc.kind = ReceiptKind::ModelCall;
    rc.exit_code = Some(0);

    rc.binary_id = "openrouter:xiaomi/mimo-v2.5".into();
    assert!(validate_receipt_for_gate(&rc, &frozen).is_ok());

    rc.binary_id = "openrouter:openai/gpt-5.6-sol".into();
    let err = validate_receipt_for_gate(&rc, &frozen).unwrap_err();
    assert!(
        matches!(err, ReceiptError::ServedModelBanned { .. }),
        "{err}"
    );

    rc.binary_id = "openrouter:openrouter/auto-beta".into();
    let err = validate_receipt_for_gate(&rc, &frozen).unwrap_err();
    assert!(
        matches!(err, ReceiptError::ServedModelBanned { .. }),
        "{err}"
    );

    rc.binary_id = "openrouter".into();
    let err = validate_receipt_for_gate(&rc, &frozen).unwrap_err();
    assert!(matches!(err, ReceiptError::ServedModelUnknown(_)), "{err}");
}

#[test]
fn concrete_attempts_skip_served_model_check() {
    let frozen = record().freeze(now()).unwrap();
    let mut rc = receipt(frozen.attempt_id());
    rc.binary_id = "openai-oauth".into();
    assert!(validate_served_model(&rc, &frozen).is_ok());
}
