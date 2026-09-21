use super::*;

/// Explicit opt-in: uses an installed CLI, isolated empty credential homes,
/// and only offline begin/cancel. Never opens a browser or exchanges tokens.
#[test]
#[ignore = "set JCODE_AUTH_TEST_BINARY to an installed CLI for compatibility validation"]
fn installed_cli_begin_cancel_isolated() {
    let binary =
        PathBuf::from(std::env::var_os("JCODE_AUTH_TEST_BINARY").expect("explicit CLI path"));
    for provider in ["claude", "openai"] {
        let home = tempfile::tempdir().unwrap();
        let client = AuthClient::new(AuthOptions {
            binary: binary.clone(),
            jcode_home: Some(home.path().to_owned()),
            socket: Some(home.path().join("absent-daemon.sock")),
            timeout: Duration::from_secs(20),
        });
        let flow = client.begin(provider, None).unwrap();
        let prompt = flow.start().unwrap();
        assert!(prompt.auth_url.starts_with("https://"));
        drop(prompt);
        flow.cancel().unwrap();
        assert!(!home.path().join("auth.json").exists());
        assert!(!home.path().join("openai-auth.json").exists());
        assert!(
            !home
                .path()
                .join("pending-login/flows")
                .join(&flow.0.flow_id)
                .join(format!("{provider}.json"))
                .exists()
        );
    }
}

#[test]
fn catalog_is_capability_filtered_and_uses_shared_aliases() {
    let client = AuthClient::default();
    assert_eq!(client.resolve_provider("OpenAI").unwrap().id, "openai");
    assert_eq!(client.resolve_provider("anthropic").unwrap().id, "claude");
    assert_eq!(
        client.resolve_provider("anthropic-api").unwrap().method,
        LoginMethod::ApiKey
    );
    assert_eq!(
        client.resolve_provider("gemini-api").unwrap().method,
        LoginMethod::ApiKey
    );
    assert_eq!(
        client.resolve_provider("jcode").unwrap().method,
        LoginMethod::ApiKey
    );
    assert!(client.resolve_provider("grok-build").is_none());
    for provider in client.providers() {
        assert_eq!(client.resolve_provider(provider.id), Some(provider));
        assert!(!provider.display_name.is_empty());
    }
    assert!(client.begin("jcode", None).is_err());
}

#[cfg(unix)]
mod processes {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::thread;

    fn poison<T: Send>(mutex: &Mutex<T>) {
        thread::scope(|scope| {
            assert!(
                scope
                    .spawn(|| {
                        let _guard = mutex.lock().unwrap();
                        panic!("injected auth test mutex poison");
                    })
                    .join()
                    .is_err()
            );
        });
    }

    #[test]
    fn poisoned_state_blocks_start_and_completion_without_panicking() {
        let (dir, client) = fixture("success");
        let flow = client.begin("openai", None).unwrap();
        // Avoid background Drop cleanup if the red-phase assertion fails.
        flow.0.finished.store(true, Ordering::Release);
        poison(&flow.0.state);
        let start = std::panic::catch_unwind(|| flow.start());
        let complete = std::panic::catch_unwind(|| flow.submit_callback("test-input"));
        assert!(!dir.path().join("argv").exists());
        for error in [
            start.expect("start must not panic").err().unwrap(),
            complete.expect("completion must not panic").err().unwrap(),
        ] {
            assert_eq!(error.kind, ErrorKind::Transport);
            assert!(!error.to_string().contains("test-input"));
        }
    }

    #[test]
    fn poisoned_child_blocks_spawn_before_executable_lookup() {
        let (dir, mut client) = fixture("success");
        client.options.binary = dir.path().join("must-not-be-invoked");
        let flow = client.begin("openai", None).unwrap();
        flow.0.finished.store(true, Ordering::Release);
        poison(&flow.0.child);
        let outcome = std::panic::catch_unwind(|| flow.start());
        // Make the original Drop safe even when this regression is still red.
        flow.0.child.clear_poison();
        let error = outcome.expect("start must not panic").err().unwrap();
        assert_eq!(error.kind, ErrorKind::Transport);
        assert!(!dir.path().join("argv").exists());
    }

    #[test]
    fn poisoned_locks_still_allow_scoped_cancel_cleanup() {
        let (dir, client) = fixture("success");
        let flow = client.begin("openai", None).unwrap();
        flow.start().unwrap();
        flow.0.finished.store(true, Ordering::Release);
        poison(&flow.0.state);
        poison(&flow.0.child);
        let outcome = std::panic::catch_unwind(|| flow.cancel());
        flow.0.child.clear_poison();
        outcome.expect("cancel must not panic").unwrap();
        assert!(
            dir.path()
                .join(format!("cancel-{}", flow.0.flow_id))
                .exists()
        );
        assert!(flow.start().is_err());
    }

    #[test]
    fn poisoned_child_cleanup_reaps_process_and_drop_does_not_panic() {
        let (dir, client) = fixture("hang");
        let flow = client.begin("copilot", None).unwrap();
        flow.0.finished.store(true, Ordering::Release);
        *flow.0.child.lock().unwrap() = Some(flow.0.command(Operation::Complete).spawn().unwrap());
        let pid = wait_for_pid(dir.path());
        poison(&flow.0.child);
        let cleanup = std::panic::catch_unwind(|| flow.0.kill_child());
        let mut slot = flow
            .0
            .child
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let child = slot.as_mut().unwrap();
        let exited = child.try_wait().unwrap().is_some();
        // Always reap the fixture, including when original cleanup panics.
        let _ = child.kill();
        let _ = child.wait();
        slot.take();
        drop(slot);
        let dropped = std::panic::catch_unwind(|| drop(flow));
        assert!(cleanup.is_ok(), "owned-child cleanup must not panic");
        assert!(exited, "cleanup must stop the owned process");
        assert!(dropped.is_ok(), "Drop must not panic on poison");
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    }

    #[test]
    fn poisoned_child_during_polling_fails_closed_and_reaps_process() {
        let (dir, client) = fixture("hang");
        let flow = client.begin("copilot", None).unwrap();
        flow.start().unwrap();
        let worker = flow.clone();
        let task = thread::spawn(move || worker.complete_device());
        let pid = wait_for_pid(dir.path());
        poison(&flow.0.child);
        let outcome = task.join();
        // Clean up even if a regressed worker panics instead of returning an error.
        flow.0.kill_child();
        let error = outcome.expect("polling must not panic").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Transport);
        assert!(error.message.contains("process state unavailable"));
        assert!(matches!(
            *flow.0.state.lock().unwrap(),
            State::Pending(AuthInputKind::DeviceCode)
        ));
        assert!(flow.0.child.lock().unwrap_err().into_inner().is_none());
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        flow.cancel().unwrap();
    }

    fn fixture(mode: &str) -> (tempfile::TempDir, AuthClient) {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("fixture.py");
        std::fs::write(
            &binary,
            r#"#!/usr/bin/env python3
import json, os, sys, time
from pathlib import Path
home = Path(os.environ['JCODE_HOME'])
args = sys.argv[1:]
provider = args[args.index('--provider') + 1]
flow = args[args.index('--flow-id') + 1]
mode = (home / 'mode').read_text()
(home / 'argv').write_text(json.dumps(args))
if '--cancel' in args:
    (home / ('cancel-' + flow)).write_text('cancelled')
    print(json.dumps(dict(status='cancelled', provider=provider)))
    sys.exit(0)
if '--print-auth-url' in args:
    if mode == 'oversized':
        print('X' * 70000)
        sys.exit(0)
    if mode == 'bad-json':
        print('private-fixture-secret')
        sys.exit(1)
    print(json.dumps(dict(status='pending', provider=provider,
        auth_url=(home / 'auth-url').read_text() if (home / 'auth-url').exists() else 'https://example.com/oauth?state=private-fixture-secret',
        input_kind='complete' if provider == 'copilot' else 'callback_url',
        user_code='ABCD-1234', expires_at_ms=9999999999999)))
    sys.exit(0)
if mode == 'hang':
    (home / 'pid').write_text(str(os.getpid()))
    time.sleep(60)
    sys.exit(1)
payload = sys.stdin.read()
(home / 'callback-input').write_text(payload)
(home / 'stdin-ok').write_text(str(payload == 'private-fixture-secret'))
print('private-fixture-secret', file=sys.stderr)
print(json.dumps(dict(status='authenticated', provider=provider)))
sys.exit(1 if mode == 'warning' else 0)
"#,
        )
        .unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(dir.path().join("mode"), mode).unwrap();
        let client = AuthClient::new(AuthOptions {
            binary,
            jcode_home: Some(dir.path().to_owned()),
            socket: Some(dir.path().join("daemon.sock")),
            timeout: Duration::from_secs(3),
        });
        (dir, client)
    }

    #[test]
    fn oauth_round_trip_uses_stdin_and_scoped_flow_not_argv() {
        let (dir, client) = fixture("success");
        let flow = client.begin("OpenAI", Some("work")).unwrap();
        let prompt = flow.start().unwrap();
        assert_eq!(prompt.input_kind, AuthInputKind::CallbackUrl);
        assert!(flow.submit_code("private-fixture-secret").is_err());
        assert!(
            !flow
                .submit_callback("private-fixture-secret")
                .unwrap()
                .validation_warning
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("stdin-ok")).unwrap(),
            "True"
        );
        let args = std::fs::read_to_string(dir.path().join("argv")).unwrap();
        assert!(!args.contains("private-fixture-secret"));
        assert!(args.contains("--callback-url"));
        assert!(args.contains("--flow-id"));
        assert!(args.contains("daemon.sock"));
        assert!(!args.contains("--account"));
        assert!(flow.start().is_err());
    }

    #[test]
    fn saved_credentials_and_validation_failure_are_distinct() {
        let (dir, client) = fixture("warning");
        let listener =
            std::os::unix::net::UnixListener::bind(dir.path().join("daemon.sock")).unwrap();
        let notified = thread::spawn(move || {
            use std::io::BufRead;
            let (stream, _) = listener.accept().unwrap();
            // macOS rejects SO_RCVTIMEO after the notifier has closed its peer.
            // Nonblocking reads retain a deadline without racing that close.
            stream.set_nonblocking(true).unwrap();
            let mut reader = std::io::BufReader::new(stream);
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line) {
                    Ok(_) => break,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "notification read timed out");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("notification read failed: {error}"),
                }
            }
            serde_json::from_str::<serde_json::Value>(&line).unwrap()
        });
        let flow = client.begin("openai", None).unwrap();
        flow.start().unwrap();
        assert!(
            flow.submit_callback("private-fixture-secret")
                .unwrap()
                .validation_warning
        );
        assert!(flow.submit_callback("private-fixture-secret").is_err());
        let request = notified.join().unwrap();
        assert_eq!(request["type"], "notify_auth_changed");
        assert_eq!(request["provider"], "openai");
        assert!(!request.to_string().contains("private-fixture-secret"));
    }

    #[test]
    fn errors_never_include_untrusted_output_or_callback_input() {
        for mode in ["bad-json", "oversized"] {
            let (_dir, client) = fixture(mode);
            let flow = client.begin("openai", None).unwrap();
            let error = flow.start().err().unwrap();
            assert!(!error.to_string().contains("private-fixture-secret"));
            flow.cancel().unwrap();
        }
    }

    fn wait_for_pid(dir: &std::path::Path) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(text) = std::fs::read_to_string(dir.join("pid")) {
                return text.parse().unwrap();
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("fixture process did not start")
    }

    #[test]
    fn concurrent_cancel_interrupts_and_reaps_device_polling() {
        let (dir, client) = fixture("hang");
        let flow = client.begin("copilot", None).unwrap();
        assert_eq!(flow.start().unwrap().input_kind, AuthInputKind::DeviceCode);
        let worker = flow.clone();
        let task = thread::spawn(move || worker.complete_device());
        let pid = wait_for_pid(dir.path());
        let start = Instant::now();
        flow.cancel().unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(task.join().unwrap().is_err());
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert!(
            dir.path()
                .join(format!("cancel-{}", flow.0.flow_id))
                .exists()
        );
        assert!(flow.complete_device().is_err());
    }

    #[test]
    fn timeout_reaps_process_and_unique_ids_isolate_cancellation() {
        let (dir, client) = fixture("hang");
        let mut flow = client.begin("copilot", None).unwrap();
        let other = client.begin("copilot", None).unwrap();
        assert_ne!(flow.0.flow_id, other.0.flow_id);
        flow.start().unwrap();
        // Exercise the short polling deadline, not Python startup or cleanup.
        Arc::get_mut(&mut flow.0).unwrap().options.timeout = Duration::from_millis(150);
        let err = flow.complete_device().unwrap_err();
        assert_eq!(err.kind, ErrorKind::Timeout);
        let pid = wait_for_pid(dir.path());
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        Arc::get_mut(&mut flow.0).unwrap().options.timeout = client.options.timeout;
        flow.cancel().unwrap();
        assert!(
            !dir.path()
                .join(format!("cancel-{}", other.0.flow_id))
                .exists()
        );
        other.cancel().unwrap();
    }

    #[test]
    fn last_drop_cleans_pending_flow_without_blocking_ui() {
        let (dir, client) = fixture("success");
        let flow = client.begin("openai", None).unwrap();
        flow.start().unwrap();
        let marker = dir.path().join(format!("cancel-{}", flow.0.flow_id));
        let clone = flow.clone();
        drop(flow);
        assert!(!marker.exists());
        drop(clone);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !marker.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(marker.exists());
    }

    fn loopback_fixture() -> (tempfile::TempDir, AuthClient, std::net::TcpListener) {
        let (dir, client) = fixture("success");
        let reserved = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = reserved.local_addr().unwrap().port();
        let mut url = url::Url::parse("https://example.com/oauth").unwrap();
        url.query_pairs_mut()
            .append_pair(
                "redirect_uri",
                &format!("http://localhost:{port}/auth/callback"),
            )
            .append_pair("state", "fixture-state");
        std::fs::write(dir.path().join("auth-url"), url.as_str()).unwrap();
        (dir, client, reserved)
    }

    fn send_callback(port: u16, target: &str) -> String {
        use std::net::TcpStream;
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "GET {target} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        if let Err(error) = stream.read_to_string(&mut response) {
            // Rejecting an oversized request can reset its unread remainder.
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
        }
        response
    }

    #[test]
    fn browser_callback_rejects_unrelated_requests_then_completes_over_stdin() {
        let (dir, client, reserved) = loopback_fixture();
        let port = reserved.local_addr().unwrap().port();
        drop(reserved);
        let flow = client.begin("openai", None).unwrap();
        flow.start().unwrap();
        assert!(flow.has_callback_listener());
        let worker = flow.clone();
        let task = thread::spawn(move || worker.wait_for_callback());
        for target in [
            "/favicon.ico",
            "/wrong?state=fixture-state&code=secret",
            "/auth/callback?state=wrong&code=secret",
            "/auth/callback?state=fixture-state&state=wrong&code=secret",
            "/auth/callback?state=fixture-state&code=secret&code=duplicate",
            "/auth/callback?state=fixture-state",
            "/\\evil.invalid/auth/callback?state=fixture-state&code=secret",
        ] {
            let response = send_callback(port, target);
            assert!(response.starts_with("HTTP/1.1 400"));
            assert!(!response.contains("secret"));
            assert!(!dir.path().join("callback-input").exists());
        }
        let oversized = format!(
            "/auth/callback?state=fixture-state&code={}",
            "x".repeat(INPUT_LIMIT)
        );
        assert!(send_callback(port, &oversized).starts_with("HTTP/1.1 400"));
        assert!(!dir.path().join("callback-input").exists());
        let target = "/auth/callback?state=fixture-state&code=fixture-code";
        let response = send_callback(port, target);
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(!response.contains("fixture-code"));
        assert!(!task.join().unwrap().unwrap().validation_warning);
        assert!(!flow.has_callback_listener());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("callback-input")).unwrap(),
            format!("http://localhost:{port}{target}")
        );
        assert!(
            !std::fs::read_to_string(dir.path().join("argv"))
                .unwrap()
                .contains("fixture-code")
        );
        // Repeated logins on a provider's fixed port must work immediately,
        // even while the prior HTTP connection is in TIME_WAIT.
        let next = client.begin("openai", None).unwrap();
        next.start().unwrap();
        assert!(next.has_callback_listener());
        next.cancel().unwrap();
    }

    #[test]
    fn busy_callback_port_keeps_manual_completion_available() {
        let (_dir, client, _reserved) = loopback_fixture();
        let flow = client.begin("openai", None).unwrap();
        flow.start().unwrap();
        assert!(!flow.has_callback_listener());
        assert!(flow.wait_for_callback().is_err());
        assert!(flow.submit_callback("private-fixture-secret").is_ok());
    }

    #[test]
    fn hosted_redirects_never_bind_local_listener() {
        let (dir, client) = fixture("success");
        for redirect in [
            "https://example.com/callback",
            "http://192.0.2.1:1234/callback",
            "http://user@localhost:1234/callback",
        ] {
            let mut url = url::Url::parse("https://example.com/oauth").unwrap();
            url.query_pairs_mut()
                .append_pair("redirect_uri", redirect)
                .append_pair("state", "fixture-state");
            std::fs::write(dir.path().join("auth-url"), url.as_str()).unwrap();
            let flow = client.begin("openai", None).unwrap();
            flow.start().unwrap();
            assert!(!flow.has_callback_listener());
            flow.cancel().unwrap();
        }
    }

    #[test]
    fn callback_wait_is_interrupted_by_cancel_and_manual_completion() {
        for cancel in [true, false] {
            let (_dir, client, reserved) = loopback_fixture();
            let port = reserved.local_addr().unwrap().port();
            drop(reserved);
            let flow = client.begin("openai", None).unwrap();
            flow.start().unwrap();
            let worker = flow.clone();
            let task = thread::spawn(move || worker.wait_for_callback());
            // A stalled HTTP peer cannot prevent cancellation or manual input.
            let _peer =
                std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
            let started = Instant::now();
            if cancel {
                flow.cancel().unwrap();
            } else {
                flow.submit_callback("private-fixture-secret").unwrap();
            }
            assert!(task.join().unwrap().is_err());
            assert!(started.elapsed() < Duration::from_secs(2));
            assert!(!flow.has_callback_listener());
            assert!(std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).is_ok());
        }
    }

    #[test]
    fn callback_timeout_keeps_manual_completion_available() {
        let (_dir, mut client, reserved) = loopback_fixture();
        let port = reserved.local_addr().unwrap().port();
        drop(reserved);
        client.options.timeout = Duration::from_millis(300);
        let flow = client.begin("openai", None).unwrap();
        flow.start().unwrap();
        let _peer = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        let started = Instant::now();
        assert_eq!(
            flow.wait_for_callback().unwrap_err().kind,
            ErrorKind::Timeout
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(!flow.has_callback_listener());
        assert!(flow.submit_callback("private-fixture-secret").is_ok());
    }
}
