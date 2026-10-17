use std::process::Command;

const FIXTURE_KEY: &str = "fixture-anthropic-credential-only";
const CHILD_ENV: &str = "JCODE_CREDENTIAL_LOGGING_TEST_CHILD";

#[test]
fn credential_resolution_child() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    assert_eq!(
        jcode_base::provider::anthropic::load_anthropic_api_key().unwrap(),
        FIXTURE_KEY
    );
}

#[test]
fn service_tier_diagnostics_do_not_print_credentials() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "credential_resolution_child", "--nocapture"])
        .env_clear()
        .env("HOME", home.path())
        .env("JCODE_HOME", home.path())
        .env(CHILD_ENV, "1")
        .env("ANTHROPIC_API_KEY", FIXTURE_KEY)
        .env("JCODE_LOG_SERVICE_TIER", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "credential resolution child failed"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    for stream in [&output.stdout, &output.stderr] {
        let text = String::from_utf8_lossy(stream);
        assert!(!text.contains(&FIXTURE_KEY[..14]));
        assert!(!text.contains("resolved API key"));
    }
}
