use super::*;

/// Run the real helper in a subprocess so PATH changes never affect other tests.
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn desktop_notification_children_are_reaped() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const FIXTURE: &str = "JCODE_TEST_NOTIFICATION_DIR";
    if let Some(dir) = std::env::var_os(FIXTURE) {
        let dir = std::path::PathBuf::from(dir);
        for _ in 0..3 {
            send_desktop_notification("reap-test", "body");
        }
        std::fs::write(dir.join("returned"), "").unwrap();
        // Keep the notifier's parent alive until PID observation is complete.
        std::io::stdin().read_line(&mut String::new()).unwrap();
        return;
    }

    fn wait_until(mut ready: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ready() {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        true
    }

    let dir = tempfile::tempdir().unwrap();
    let notifier = dir.path().join(if cfg!(target_os = "macos") {
        "osascript"
    } else {
        "notify-send"
    });
    std::fs::write(
        &notifier,
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$$\" >> \"$JCODE_TEST_NOTIFICATION_DIR/pids\"\n",
            "while [ -d \"$JCODE_TEST_NOTIFICATION_DIR\" ] && ",
            "[ ! -f \"$JCODE_TEST_NOTIFICATION_DIR/release\" ]; do /bin/sleep 0.01; done\n",
        ),
    )
    .unwrap();
    std::fs::set_permissions(&notifier, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut helper = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "notifications::tests::desktop_notification_children_are_reaped",
        ])
        .env(FIXTURE, dir.path())
        .env("PATH", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let mut pids = String::new();
    let returned_while_running = wait_until(|| {
        pids = std::fs::read_to_string(dir.path().join("pids")).unwrap_or_default();
        dir.path().join("returned").exists() && pids.lines().count() == 3
    });
    // Release even on failure, then join the helper before asserting.
    std::fs::write(dir.path().join("release"), "").unwrap();
    let reaped = returned_while_running
        && wait_until(|| {
            let output = Command::new("ps")
                .args([
                    "-p",
                    &pids.lines().collect::<Vec<_>>().join(","),
                    "-o",
                    "pid=",
                ])
                .output()
                .unwrap();
            output.status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty()
        });
    let parent_alive = helper.try_wait().unwrap().is_none();
    drop(helper.stdin.take());
    let status = helper.wait().unwrap();
    assert!(status.success(), "notification helper failed: {status}");
    assert!(
        returned_while_running,
        "notifications must return before notifier exit"
    );
    assert!(
        parent_alive,
        "PID disappearance must be observed before parent exit"
    );
    assert!(reaped, "notification children were not reaped: {pids}");
}

#[test]
fn test_format_cycle_body_safe() {
    let transcript = AmbientTranscript {
        session_id: "test_001".to_string(),
        started_at: chrono::Utc::now(),
        ended_at: Some(chrono::Utc::now()),
        status: crate::safety::TranscriptStatus::Complete,
        provider: "claude".to_string(),
        model: "claude-sonnet-4".to_string(),
        actions: Vec::new(),
        pending_permissions: 0,
        summary: Some("Cleaned up 3 stale memories.".to_string()),
        compactions: 1,
        memories_modified: 3,
        conversation: None,
    };

    let body = format_cycle_body_safe(&transcript);
    assert!(body.contains("Memories modified: 3"));
    assert!(body.contains("Compactions: 1"));
    assert!(body.contains("Check jcode for full details"));
    // Safe body must NOT include model-generated summary
    assert!(!body.contains("Cleaned up"));
    assert!(!body.contains("permission"));
}

#[test]
fn test_format_cycle_body_detailed() {
    let transcript = AmbientTranscript {
        session_id: "test_001".to_string(),
        started_at: chrono::Utc::now(),
        ended_at: Some(chrono::Utc::now()),
        status: crate::safety::TranscriptStatus::Complete,
        provider: "claude".to_string(),
        model: "claude-sonnet-4".to_string(),
        actions: Vec::new(),
        pending_permissions: 0,
        summary: Some("Cleaned up 3 stale memories.".to_string()),
        compactions: 1,
        memories_modified: 3,
        conversation: Some("### User\n\nBegin cycle.\n\n### Assistant\n\nDone.\n".to_string()),
    };

    let body = format_cycle_body_detailed(&transcript);
    // Detailed body SHOULD include the summary
    assert!(body.contains("Cleaned up 3 stale memories."));
    assert!(body.contains("**Memories:** 3"));
    assert!(body.contains("claude"));
    // Should include conversation transcript
    assert!(body.contains("# Full Transcript"));
    assert!(body.contains("### User"));
    assert!(body.contains("Begin cycle."));
}

#[test]
fn test_format_cycle_body_with_pending_permissions() {
    let transcript = AmbientTranscript {
        session_id: "test_002".to_string(),
        started_at: chrono::Utc::now(),
        ended_at: Some(chrono::Utc::now()),
        status: crate::safety::TranscriptStatus::Complete,
        provider: "claude".to_string(),
        model: "claude-sonnet-4".to_string(),
        actions: Vec::new(),
        pending_permissions: 2,
        summary: None,
        compactions: 0,
        memories_modified: 0,
        conversation: None,
    };

    let safe = format_cycle_body_safe(&transcript);
    assert!(safe.contains("2 permission request(s) pending"));
    assert!(safe.contains("Check jcode for full details"));

    let detailed = format_cycle_body_detailed(&transcript);
    assert!(detailed.contains("2 permission request(s) pending"));
}

#[test]
fn test_priority_values() {
    assert_eq!(Priority::Default.ntfy_value(), "3");
    assert_eq!(Priority::High.ntfy_value(), "4");
    assert_eq!(Priority::Urgent.ntfy_value(), "5");
}

#[test]
fn test_dispatcher_creation() {
    // Just verify it doesn't panic
    let cfg = SafetyConfig::default();
    let _dispatcher = NotificationDispatcher::from_config(cfg);
}

#[test]
fn macos_origin_detects_terminal_identifiers() {
    let terminal = MacosNotificationOrigin::from_values(
        "Apple_Terminal",
        "xterm-256color",
        Some("/dev/ttys007"),
        Some("4F3C"),
        None,
        false,
    );
    assert_eq!(terminal.terminal, MacosTerminalKind::AppleTerminal);
    assert_eq!(terminal.bundle_id.as_deref(), Some("com.apple.Terminal"));
    assert_eq!(terminal.tty.as_deref(), Some("/dev/ttys007"));
    assert_eq!(terminal.session_id.as_deref(), Some("4F3C"));

    let iterm = MacosNotificationOrigin::from_values(
        "iTerm.app",
        "xterm-256color",
        Some("/dev/ttys011"),
        None,
        Some("w0t1p0:ABC"),
        false,
    );
    assert_eq!(iterm.terminal, MacosTerminalKind::Iterm2);
    assert_eq!(iterm.session_id.as_deref(), Some("w0t1p0:ABC"));

    let ghostty = MacosNotificationOrigin::from_values(
        "",
        "xterm-ghostty",
        Some("/dev/ttys019"),
        None,
        None,
        true,
    );
    assert_eq!(ghostty.terminal, MacosTerminalKind::Ghostty);
    assert_eq!(ghostty.bundle_id.as_deref(), Some("com.mitchellh.ghostty"));
}

#[test]
fn macos_origin_rejects_untrusted_route_values() {
    let origin = MacosNotificationOrigin::from_values(
        "Apple_Terminal",
        "",
        Some("/dev/ttys001\"\nrun script"),
        Some("bad\nidentifier"),
        None,
        false,
    );
    assert_eq!(origin.tty, None);
    assert_eq!(origin.session_id, None);

    let (_, args) = macos_notification_activation_command(&origin).expect("Terminal route");
    assert_eq!(
        args,
        vec!["-e", "tell application \"Terminal\" to activate"]
    );
}

#[test]
fn macos_activation_targets_terminal_and_iterm_ttys() {
    let terminal = MacosNotificationOrigin {
        terminal: MacosTerminalKind::AppleTerminal,
        bundle_id: Some("com.apple.Terminal".to_string()),
        tty: Some("/dev/ttys003".to_string()),
        session_id: Some("session-a".to_string()),
    };
    let (program, args) =
        macos_notification_activation_command(&terminal).expect("Terminal command");
    assert_eq!(program, "/usr/bin/osascript");
    assert!(args[1].contains("if tty of t is \"/dev/ttys003\""));
    assert!(args[1].contains("set selected tab of w to t"));

    let iterm = MacosNotificationOrigin {
        terminal: MacosTerminalKind::Iterm2,
        bundle_id: Some("com.googlecode.iterm2".to_string()),
        tty: Some("/dev/ttys004".to_string()),
        session_id: Some("w0t0p0:guid".to_string()),
    };
    let (_, args) = macos_notification_activation_command(&iterm).expect("iTerm command");
    assert!(args[1].contains("if tty of s is \"/dev/ttys004\""));
    assert!(args[1].contains("select s"));
}

#[test]
fn macos_ghostty_activation_is_application_scoped() {
    let origin = MacosNotificationOrigin {
        terminal: MacosTerminalKind::Ghostty,
        bundle_id: Some("evil.bundle".to_string()),
        tty: Some("/dev/ttys005".to_string()),
        session_id: None,
    };
    assert_eq!(
        macos_notification_activation_command(&origin),
        Some((
            "/usr/bin/open".to_string(),
            vec!["-b".to_string(), "com.mitchellh.ghostty".to_string()]
        ))
    );
}

#[test]
fn macos_envelope_roundtrip_preserves_origin_metadata() {
    let envelope = MacosNotificationEnvelope {
        schema_version: MACOS_NOTIFICATION_SCHEMA_VERSION,
        notification_id: "jcode-turn-test".to_string(),
        title: "jcode · done".to_string(),
        subtitle: Some("2/2 todos".to_string()),
        body: "Finished broker".to_string(),
        sound: Some("Glass".to_string()),
        origin: MacosNotificationOrigin {
            terminal: MacosTerminalKind::Iterm2,
            bundle_id: Some("com.googlecode.iterm2".to_string()),
            tty: Some("/dev/ttys009".to_string()),
            session_id: Some("w1t2p0:route".to_string()),
        },
    };
    let encoded = serde_json::to_vec(&envelope).expect("encode envelope");
    let decoded: MacosNotificationEnvelope =
        serde_json::from_slice(&encoded).expect("decode envelope");
    assert_eq!(decoded, envelope);
}
