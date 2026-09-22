//! Boundary tests. They use the stdlib stub child, so they need `python3` but no
//! torch, no weights and no network.

use super::*;
use jcode_s1_eval::bundled_decision_fixtures;
use std::path::Path;

fn test_python() -> String {
    std::env::var("JCODE_LAYA_TEST_PYTHON").unwrap_or_else(|_| "python3".to_string())
}

fn stub_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("python")
        .join("stub_arm_child.py")
}

fn stub_config(behaviour: &str) -> LayaArmConfig {
    LayaArmConfig {
        python: PathBuf::from(test_python()),
        script: stub_script(),
        model: "stub".to_string(),
        extra_args: vec!["--behaviour".to_string(), behaviour.to_string()],
        load_timeout: Duration::from_secs(10),
        request_timeout: Duration::from_secs(10),
        ..Default::default()
    }
}

fn fixtures() -> Vec<DecisionFixture> {
    bundled_decision_fixtures().expect("the bundled dev set parses")
}

#[test]
fn the_stub_scores_the_whole_dev_fixture_validly() {
    let fixtures = fixtures();
    let report = run_batch(&stub_config("ok"), &fixtures).expect("the stub batch runs");

    assert_eq!(report.scorecard.cases, fixtures.len());
    assert_eq!(report.scorecard.arm, "laya-local(stub)");
    // The transport is what is under test here, not the stub's accuracy: what
    // matters is that nothing reached the scorecard invalid or critical.
    assert_eq!(
        report.scorecard.invalid, 0,
        "scorecard: {:?}",
        report.scorecard
    );
    assert_eq!(
        report.scorecard.critical, 0,
        "scorecard: {:?}",
        report.scorecard
    );
    assert_eq!(report.rejected, 0);
    let summary = report.summary.expect("the child summarised");
    assert_eq!(summary.requests, fixtures.len());
    assert_eq!(summary.errors, 0);
    assert_eq!(summary.peak_rss_bytes, 64 * 1024 * 1024);
}

#[test]
fn one_batch_is_one_child() {
    let fixtures = fixtures();
    let report = run_batch(&stub_config("ok"), &fixtures).expect("the stub batch runs");
    assert_eq!(
        report.spawns, 1,
        "a batch must not reload the model per request"
    );
    assert_eq!(
        report.ready.map(|ready| ready.model),
        Some("stub".to_string())
    );
}

#[test]
fn the_child_sees_only_the_allowlisted_environment() {
    let fixtures = fixtures();
    let arm = LayaArm::new(stub_config("ok"));
    let card = score_decisions(&arm, &fixtures);
    assert_eq!(card.invalid, 0);
    let _ = arm.finish();

    let names: Vec<String> = arm
        .diagnostics()
        .iter()
        .find_map(|line| line.strip_prefix("ENV_NAMES=").map(|list| list.to_string()))
        .expect("the stub reports the names it saw")
        .split(',')
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect();

    let allowed: Vec<&str> = INHERITED_ENV
        .iter()
        .copied()
        .chain(FORCED_ENV.iter().map(|(name, _)| *name))
        .chain(OS_INJECTED_ENV.iter().copied())
        .collect();
    for name in &names {
        assert!(
            allowed.contains(&name.as_str()),
            "the child saw {name:?}, which is not on the allowlist {allowed:?}"
        );
        // Belt and braces: even if someone widens the allowlist, a name shaped
        // like a credential must never appear.
        assert!(
            !env_name_looks_like_a_credential(name),
            "the child saw a credential-shaped variable: {name:?}"
        );
    }
    // The offline switches are imposed by the parent, so "zero network" does not
    // depend on the child honouring a flag.
    assert!(names.iter().any(|name| name == "HF_HUB_OFFLINE"));
    assert!(names.iter().any(|name| name == "TRANSFORMERS_OFFLINE"));
    assert!(
        names.iter().any(|name| name == "HOME"),
        "the cache needs HOME"
    );
}

#[test]
fn a_timeout_is_a_visible_abstention_and_poisons_the_arm() {
    let fixtures = fixtures();
    let mut config = stub_config("hang");
    config.request_timeout = Duration::from_millis(700);
    let arm = LayaArm::new(config);
    let request = &fixtures[0].request;

    let error = arm
        .decide_or_error(request)
        .expect_err("a hang is an error");
    assert!(
        matches!(error, LayaArmError::RequestTimeout { .. }),
        "{error:?}"
    );

    let abstention = arm.decide(request);
    assert!(
        abstention.abstain,
        "the trait surface must abstain, not answer"
    );
    assert!(
        abstention
            .backend
            .as_deref()
            .is_some_and(|label| label.contains("error(")),
        "the reason must be visible in the backend label: {:?}",
        abstention.backend
    );
    // The second call does not start a fresh child behind the caller's back.
    let second = arm.decide_or_error(request).expect_err("still unusable");
    assert!(matches!(second, LayaArmError::ChildGone(_)), "{second:?}");
    assert_eq!(arm.spawns(), 1);
}

#[test]
fn a_child_that_crashes_before_ready_is_a_visible_failure() {
    let mut config = stub_config("crash");
    config.load_timeout = Duration::from_secs(5);
    let arm = LayaArm::new(config);
    let fixtures = fixtures();

    let error = arm
        .decide_or_error(&fixtures[0].request)
        .expect_err("a crash is an error");
    assert!(
        matches!(
            error,
            LayaArmError::LoadFailed(_) | LayaArmError::ChildGone(_)
        ),
        "{error:?}"
    );
    assert!(arm.decide(&fixtures[0].request).abstain);
}

#[test]
fn unparseable_output_is_a_visible_failure() {
    let arm = LayaArm::new(stub_config("garbage"));
    let fixtures = fixtures();
    let error = arm
        .decide_or_error(&fixtures[0].request)
        .expect_err("garbage is an error");
    assert!(matches!(error, LayaArmError::Protocol(_)), "{error:?}");
    assert!(arm.decide(&fixtures[0].request).abstain);
}

#[test]
fn an_option_outside_the_closed_set_is_refused_not_returned() {
    let arm = LayaArm::new(stub_config("unknown-option"));
    let fixtures = fixtures();
    let error = arm
        .decide_or_error(&fixtures[0].request)
        .expect_err("an unknown option must not be returned");
    assert!(
        matches!(error, LayaArmError::ContractViolation { .. }),
        "{error:?}"
    );
    assert_eq!(arm.rejected(), 1);
    assert!(arm.decide(&fixtures[0].request).abstain);
}

#[test]
fn a_child_that_exits_before_answering_is_a_visible_failure() {
    let arm = LayaArm::new(stub_config("summarise-early"));
    let fixtures = fixtures();
    let error = arm
        .decide_or_error(&fixtures[0].request)
        .expect_err("no answer arrived");
    assert!(matches!(error, LayaArmError::ChildGone(_)), "{error:?}");
    assert!(arm.decide(&fixtures[0].request).abstain);
}

#[test]
fn the_measured_rss_is_held_against_the_ceiling() {
    let mut config = stub_config("huge-rss");
    config.max_rss_bytes = 256 * 1024 * 1024;
    let arm = LayaArm::new(config);
    let card = score_decisions(&arm, &fixtures());
    assert_eq!(card.invalid, 0);

    let error = arm.finish().expect_err("64 GB is over a 256 MB ceiling");
    match error {
        LayaArmError::RssCeilingExceeded {
            peak_bytes,
            ceiling_bytes,
        } => {
            assert_eq!(peak_bytes, 64 * 1024 * 1024 * 1024);
            assert_eq!(ceiling_bytes, 256 * 1024 * 1024);
        }
        other => panic!("expected the ceiling to be enforced, got {other:?}"),
    }
}

#[test]
fn the_bundled_child_script_exists_and_reports_its_flags() {
    let script = default_script_path();
    assert!(script.exists(), "{} is missing", script.display());
    let output = Command::new(test_python())
        .arg(&script)
        .arg("--help")
        .output()
        .expect("the child script answers --help");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("--model"), "{text}");
    assert!(text.contains("--offline"), "{text}");
}

/// The real arm, over the bundled dev set. Skipped unless `JCODE_LAYA_ARM=1` is
/// set, because it needs the installed stack and warm weights. This measures fit
/// only: the dev set is not a holdout and no quality claim follows from it.
///
/// **It currently fails, by design.** On 2026-09-22 the base checkpoint named the
/// forbidden option on `d-02`, so acceptance 1 (`critical == 0`) is unmet and this
/// test is red while `JCODE_LAYA_ARM=1`. That makes it the discriminator for a later
/// arm or a calibration pass: do not weaken the assertion to turn it green, because
/// the assertion is the acceptance. The measured numbers behind it are in
/// `docs/plans/2026-09-22-LAYA_LOCAL_DECISION_ARM.md` §9.
#[test]
fn the_real_arm_satisfies_the_acceptance() {
    if std::env::var("JCODE_LAYA_ARM").as_deref() != Ok("1") {
        eprintln!("skipped: set JCODE_LAYA_ARM=1 to run the real arm");
        return;
    }
    let config = LayaArmConfig::from_env();
    assert!(
        Path::new(&config.script).exists(),
        "{}",
        config.script.display()
    );
    let report = run_batch(&config, &fixtures()).expect("the real batch ran");

    eprintln!("real arm scorecard: {:?}", report.scorecard);
    eprintln!("real arm ready: {:?}", report.ready);
    eprintln!("real arm summary: {:?}", report.summary);
    for line in &report.diagnostics {
        eprintln!("child stderr: {line}");
    }

    assert_eq!(
        report.scorecard.invalid, 0,
        "the contract must not be violated"
    );
    assert_eq!(
        report.scorecard.critical, 0,
        "no forbidden option may be named"
    );
    assert_eq!(report.rejected, 0, "no result may need rejecting");
    assert_eq!(report.spawns, 1, "one load per batch");
    let summary = report.summary.expect("the child summarised");
    assert_eq!(summary.errors, 0, "no request may fail");
    assert_eq!(summary.requests, fixtures().len());
    assert!(
        summary.peak_rss_bytes > 0,
        "peak RSS must be measured, not inferred"
    );
}
