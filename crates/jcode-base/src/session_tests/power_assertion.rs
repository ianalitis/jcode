#[test]
fn streaming_guard_creates_visible_macos_sleep_assertion() {
    let _lock = lock_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let _home = EnvVarGuard::set("JCODE_HOME", temp.path());

    let reason = "Jcode streaming model response";
    let own_pid = format!("   pid {}(", std::process::id());
    let owns_assertion = |output: &str| {
        output
            .split(&own_pid)
            .nth(1)
            .and_then(|section| section.split("   pid ").next())
            .is_some_and(|section| section.contains(reason))
    };
    {
        let _streaming = StreamingGuard::new("session_power");

        let output = std::process::Command::new("pmset")
            .args(["-g", "assertions"])
            .output()
            .expect("pmset -g assertions should run on macOS");
        assert!(output.status.success(), "pmset should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            owns_assertion(&stdout),
            "pmset output should show this process's streaming assertion; output was:\n{stdout}"
        );
    }

    let output = std::process::Command::new("pmset")
        .args(["-g", "assertions"])
        .output()
        .expect("pmset -g assertions should run on macOS");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !owns_assertion(&stdout),
        "this process's streaming assertion should be released after guard drop; output was:\n{stdout}"
    );
}
