#[test]
fn handle_server_event_applies_remote_memory_activity_snapshot() {
    crate::memory::clear_activity();

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut app = create_test_app();
    app.memory_enabled = true;
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    handle_server_event(
        &mut app,
        ServerEvent::MemoryActivity {
            activity: MemoryActivitySnapshot {
                state: MemoryStateSnapshot::SidecarChecking { count: 3 },
                state_age_ms: 180,
                pipeline: Some(MemoryPipelineSnapshot {
                    search: MemoryStepStatusSnapshot::Done,
                    search_result: None,
                    verify: MemoryStepStatusSnapshot::Running,
                    verify_result: None,
                    verify_progress: Some((1, 3)),
                    inject: MemoryStepStatusSnapshot::Pending,
                    inject_result: None,
                    maintain: MemoryStepStatusSnapshot::Pending,
                    maintain_result: None,
                }),
            },
        },
        &mut remote,
    );

    let activity = crate::memory::get_activity().expect("memory activity should be populated");
    assert_eq!(activity.state, MemoryState::SidecarChecking { count: 3 });
    let pipeline = activity.pipeline.expect("pipeline should be restored");
    assert_eq!(pipeline.search, StepStatus::Done);
    assert_eq!(pipeline.verify, StepStatus::Running);
    assert_eq!(pipeline.verify_progress, Some((1, 3)));
    assert!(activity.state_since.elapsed().as_millis() >= 100);

    crate::memory::clear_activity();
}

/// Reproduces the "stuck on loading session…" bug and verifies the watchdog
/// recovers it: a remote connection that never receives the bootstrap History
/// event (so `has_loaded_history()` stays false) must re-request `GetHistory`
/// once it has waited past the recovery delay, instead of staying stuck forever.
#[test]
fn remote_history_watchdog_rerequests_history_when_stuck() {
    use std::time::{Duration, Instant};
    use tokio::io::AsyncBufReadExt;

    let mut app = create_test_app();
    app.is_remote = true;
    app.remote_session_id = Some("session_stuck".to_string());

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut line = String::new();
    let (redraw, attempts) = rt.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        // The bug condition: history never loaded after (re)connect.
        assert!(!remote.has_loaded_history());
        let peer = remote
            .take_dummy_peer()
            .expect("dummy remote should retain peer stream");
        let (reader, _writer) = peer.into_split();
        let mut reader = tokio::io::BufReader::new(reader);

        // First tick simply starts tracking the wait; no re-request yet.
        let first = super::recover_stuck_remote_history(&mut app, &mut remote).await;
        assert!(!first, "first observation should only arm the watchdog");
        assert!(app.remote_history_wait_started.is_some());
        assert_eq!(app.remote_history_recovery_attempts, 0);

        // Simulate the connection having been stuck past the recovery delay.
        app.remote_history_wait_started = Instant::now().checked_sub(Duration::from_secs(60));

        let redraw = super::recover_stuck_remote_history(&mut app, &mut remote).await;
        reader
            .read_line(&mut line)
            .await
            .expect("history re-request should be readable by peer");
        (redraw, app.remote_history_recovery_attempts)
    });

    assert!(redraw, "re-requesting history should trigger a redraw");
    assert_eq!(
        attempts, 1,
        "watchdog should have re-requested history once"
    );
    assert!(matches!(
        serde_json::from_str::<crate::protocol::Request>(&line)
            .expect("history re-request should deserialize"),
        crate::protocol::Request::GetHistory { .. }
    ));
}

/// A partial inbound frame proves that the original History response is in
/// flight. The watchdog must not queue another full response behind it.
#[test]
fn remote_history_watchdog_does_not_rerequest_while_frame_is_arriving() {
    use std::time::{Duration, Instant};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let mut app = create_test_app();
    app.is_remote = true;
    app.remote_session_id = Some("session_large".to_string());
    app.remote_history_wait_started = Instant::now().checked_sub(Duration::from_secs(60));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        let peer = remote
            .take_dummy_peer()
            .expect("dummy remote should retain peer stream");
        let (reader, mut writer) = peer.into_split();
        let mut reader = tokio::io::BufReader::new(reader);

        // Deliberately omit the newline so next_event retains this partial
        // History-sized frame when its future is cancelled by the tick.
        writer
            .write_all(b"{\"type\":\"history\",\"messages\":[")
            .await
            .expect("partial frame should reach remote");
        assert!(
            tokio::time::timeout(Duration::from_millis(20), remote.next_event())
                .await
                .is_err(),
            "partial frame must remain incomplete"
        );
        assert!(remote.has_buffered_inbound_frame());

        let redraw = super::recover_stuck_remote_history(&mut app, &mut remote).await;
        assert!(!redraw, "in-flight history should not trigger recovery");
        assert_eq!(app.remote_history_recovery_attempts, 0);

        let mut line = String::new();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), reader.read_line(&mut line))
                .await
                .is_err(),
            "watchdog must not write a duplicate GetHistory request"
        );
    });
}

/// Once history loads, the watchdog must clear its budget and do nothing.
#[test]
fn remote_history_watchdog_clears_budget_once_history_loads() {
    use std::time::{Duration, Instant};

    let mut app = create_test_app();
    app.is_remote = true;
    app.remote_history_wait_started = Instant::now().checked_sub(Duration::from_secs(60));
    app.remote_history_recovery_attempts = 2;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let redraw = rt.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        remote.mark_history_loaded();
        super::recover_stuck_remote_history(&mut app, &mut remote).await
    });

    assert!(!redraw);
    assert!(app.remote_history_wait_started.is_none());
    assert_eq!(app.remote_history_recovery_attempts, 0);
    assert!(app.remote_history_recovery_last_attempt.is_none());
}

/// After exhausting re-requests the watchdog surfaces an actionable `/restart`
/// hint exactly once instead of silently leaving the user stuck.
#[test]
fn remote_history_watchdog_advises_restart_after_giving_up() {
    use std::time::{Duration, Instant};

    let mut app = create_test_app();
    app.is_remote = true;
    app.remote_history_wait_started = Instant::now().checked_sub(Duration::from_secs(60));
    app.remote_history_recovery_attempts = super::REMOTE_HISTORY_RECOVERY_MAX_ATTEMPTS;
    app.remote_history_recovery_last_attempt = Some(Instant::now());

    let before = app.display_messages().len();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let redraw = rt.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        super::recover_stuck_remote_history(&mut app, &mut remote).await
    });

    assert!(redraw);
    let messages = app.display_messages();
    assert_eq!(messages.len(), before + 1, "should add exactly one hint");
    assert!(
        messages.last().unwrap().content.contains("/restart"),
        "hint should advise /restart: {}",
        messages.last().unwrap().content
    );
    // last_attempt cleared so the hint is not repeated every tick.
    assert!(app.remote_history_recovery_last_attempt.is_none());

    // A subsequent tick must not add another hint.
    let rt2 = tokio::runtime::Runtime::new().unwrap();
    let redraw2 = rt2.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        super::recover_stuck_remote_history(&mut app, &mut remote).await
    });
    assert!(!redraw2);
    assert_eq!(app.display_messages().len(), before + 1);
}

/// Regression for issue #427: picking an effort-variant model row (e.g.
/// "gpt-5.5 (high)") in remote mode must forward the chosen effort to the
/// server after the model-switch request. Previously the effort was applied
/// only to the local stand-in provider, so the server kept its configured
/// default (low by default) and silently ran the new model at low effort.
#[test]
fn forward_pending_reasoning_effort_sends_effort_request_to_server() {
    use tokio::io::AsyncBufReadExt;

    let mut app = create_test_app();
    app.is_remote = true;
    app.pending_reasoning_effort = Some("high".to_string());

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let line = rt.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        let peer = remote
            .take_dummy_peer()
            .expect("dummy remote should retain peer stream");
        let (reader, _writer) = peer.into_split();
        let mut reader = tokio::io::BufReader::new(reader);

        super::forward_pending_reasoning_effort(&mut app, &mut remote).await;

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("effort request should be readable by peer");
        line
    });

    match serde_json::from_str::<crate::protocol::Request>(&line)
        .expect("effort request should deserialize")
    {
        crate::protocol::Request::SetReasoningEffort { effort, .. } => {
            assert_eq!(effort, "high", "the picker-selected effort must be sent");
        }
        other => panic!("expected SetReasoningEffort request, got {:?}", other),
    }

    assert!(
        app.pending_reasoning_effort.is_none(),
        "staged effort must be consumed after dispatch"
    );
    assert_eq!(
        app.remote_reasoning_effort.as_deref(),
        Some("high"),
        "requested effort should be tracked optimistically for the UI"
    );
}

/// The dispatcher must be a no-op when no effort variant was staged (plain
/// model rows without an effort suffix).
#[test]
fn forward_pending_reasoning_effort_is_noop_without_staged_effort() {
    let mut app = create_test_app();
    app.is_remote = true;
    assert!(app.pending_reasoning_effort.is_none());

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        super::forward_pending_reasoning_effort(&mut app, &mut remote).await;
    });

    assert!(app.remote_reasoning_effort.is_none());
    assert!(app.pending_reasoning_effort.is_none());
}

#[test]
fn remote_dropped_file_path_is_sent_as_a_prompt_not_a_slash_command() {
    // Regression: a terminal file drop like `/tmp/shot.png` starts with `/`, so
    // remote submit routed it to `submit_remote_slash_input`, which fell back to
    // `App::submit_input`. That only sets `pending_turn`, which no remote run
    // loop consumes, so the client hung in "Sending" forever.
    let temp = tempfile::tempdir().expect("create temp dir");
    let file = temp.path().join("shot.png");
    std::fs::write(&file, b"x").expect("write file");
    let dropped = file.to_string_lossy().to_string();

    let mut app = create_test_app();
    app.is_remote = true;
    app.runtime_mode = crate::tui::app::AppRuntimeMode::RemoteClient;

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();
    rt.block_on(crate::tui::app::remote::submit_remote_slash_input(
        &mut app,
        &mut remote,
        crate::tui::app::input::PreparedInput {
            raw_input: dropped.clone(),
            expanded: dropped.clone(),
            images: vec![],
        },
    ))
    .expect("dropped path should send as a normal remote turn");

    assert!(
        app.active_skill.is_none(),
        "a file path must never activate a skill"
    );
    assert!(
        app.is_processing,
        "a dropped path must start a remote turn instead of stranding pending_turn"
    );
    assert!(
        !app.pending_turn,
        "remote submissions must never park on the local-only pending_turn flag"
    );
}

#[test]
fn remote_submit_input_never_strands_a_local_pending_turn() {
    // Safety net: any path that reaches `App::submit_input` while attached to a
    // remote session must queue for the remote tick loop rather than set
    // `pending_turn`, which only the local run loop consumes.
    let mut app = create_test_app();
    app.is_remote = true;
    app.runtime_mode = crate::tui::app::AppRuntimeMode::RemoteClient;
    app.input = "plain prompt".to_string();
    app.cursor_pos = app.input.len();

    app.submit_input();

    assert!(
        !app.pending_turn,
        "remote submit_input must not set the local-only pending_turn flag"
    );
    assert_eq!(
        app.queued_messages,
        vec!["plain prompt".to_string()],
        "the prompt should be queued for the remote tick loop"
    );
}
