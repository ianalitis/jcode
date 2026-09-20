//! Exercise the normal library build, not a test-only telemetry substitute.
use jcode_telemetry_core as telemetry;
use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn collection_cannot_be_reenabled() {
    if std::env::var_os("JCODE_PRIVACY_TEST_CHILD").is_some() {
        // Fail before exercising any old upload paths in the red test.
        assert!(
            !telemetry::is_enabled(),
            "this fork must never collect telemetry"
        );
        assert!(!telemetry::content_sharing_enabled());
        assert!(!telemetry::set_usage_telemetry_enabled(true));
        assert!(!telemetry::set_content_sharing_enabled(true));
        assert!(telemetry::set_usage_telemetry_enabled(false));
        assert!(telemetry::set_content_sharing_enabled(false));
        let status = telemetry::status();
        assert!(!status.enabled && !status.content_sharing_enabled);
        assert_eq!(status.opt_out_source.unwrap().as_str(), "build_policy");
        assert!(status.telemetry_id.is_none());

        telemetry::record_install_if_first_run();
        telemetry::record_upgrade_if_needed();
        telemetry::record_setup_step_once("privacy_test");
        telemetry::record_auth_success("test", "test");
        telemetry::begin_session_with_parent("test", "test", Some("parent".into()), false);
        let mut guard = telemetry::begin_concurrency_session("session", Some("parent"));
        assert!(!guard.is_active());
        assert_eq!(guard.session_id(), "session");
        telemetry::record_turn();
        telemetry::record_assistant_response();
        telemetry::record_feedback("must not leave this process");
        telemetry::record_tool_execution("read", &serde_json::json!({"path": "private"}), true, 1);
        telemetry::record_token_usage(10, 20, Some(30), Some(40));
        telemetry::record_todo_update(Default::default());
        telemetry::record_todo_gate(telemetry::TodoGateKind::Completion);
        assert!(!telemetry::record_transcript(
            "test",
            "test",
            telemetry::SessionEndReason::NormalExit,
            serde_json::json!([{"role": "user", "content": "private transcript"}]),
        ));
        telemetry::end_session("test", "test");
        telemetry::record_crash("test", "test", telemetry::SessionEndReason::Panic);
        guard.finish();
        assert!(!guard.is_active());
        assert!(telemetry::current_session_correlation_id().is_none());
        assert!(telemetry::current_provider_model().is_none());
        return;
    }

    for legacy_consent in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let before = if legacy_consent {
            vec![
                ("telemetry_share_transcripts_v1", "1"),
                ("telemetry_id", "existing-private-id"),
                (
                    "install_conversion_id",
                    "11111111-2222-4333-8444-555555555555",
                ),
            ]
        } else {
            vec![]
        };
        for (file, value) in &before {
            std::fs::write(home.path().join(file), value).unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let proxy = format!("http://{}", listener.local_addr().unwrap());
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "collection_cannot_be_reenabled", "--nocapture"])
            .env("JCODE_PRIVACY_TEST_CHILD", "1")
            .env("JCODE_HOME", home.path())
            .env("HOME", home.path())
            .env("NO_PROXY", "")
            .env("no_proxy", "");
        for key in [
            "HTTPS_PROXY",
            "https_proxy",
            "HTTP_PROXY",
            "http_proxy",
            "ALL_PROXY",
            "all_proxy",
        ] {
            command.env(key, &proxy);
        }
        for key in ["JCODE_NO_TELEMETRY", "DO_NOT_TRACK"] {
            if legacy_consent {
                command.env(key, "0");
            } else {
                command.env_remove(key);
            }
        }
        let mut child = command.spawn().unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(10) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("privacy subprocess hung");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(status.success(), "privacy subprocess failed");
        assert!(
            matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
            "telemetry attempted network delivery"
        );
        let files = std::fs::read_dir(home.path()).unwrap().count();
        assert_eq!(
            files,
            before.len(),
            "telemetry created local tracking state"
        );
        for (file, value) in before {
            assert_eq!(
                std::fs::read_to_string(home.path().join(file)).unwrap(),
                value,
                "existing data must not be silently changed or deleted"
            );
        }
    }
}
