#[test]
fn test_remote_review_shows_processing_until_split_response() {
    let mut app = create_test_app();
    app.is_remote = true;
    app.input = "/review".to_string();
    app.cursor_pos = app.input.len();

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
        .expect("/review should launch split request");

    assert!(
        app.is_processing,
        "review launch should show client processing state"
    );
    assert!(matches!(app.status, ProcessingStatus::Sending));
    assert!(app.current_message_id.is_none());
    assert_eq!(app.status_notice(), Some("Review launching".to_string()));
    assert!(app.pending_split_startup_message.is_some());
    assert_eq!(app.pending_split_label.as_deref(), Some("Review"));
    assert!(!app.pending_split_request);

    app.handle_server_event(
        crate::protocol::ServerEvent::SplitResponse {
            id: 1,
            new_session_id: "session_review_child".to_string(),
            new_session_name: "review_child".to_string(),
        },
        &mut remote,
    );

    assert!(
        !app.is_processing,
        "split response should clear transient launch state"
    );
    assert!(matches!(app.status, ProcessingStatus::Idle));
    assert!(app.processing_started.is_none());
    assert!(app.pending_split_startup_message.is_none());
    assert!(app.pending_split_label.is_none());
}

#[test]
fn test_remote_super_space_routes_next_prompt_to_new_session() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.is_remote = true;
        app.input = "hello from split".to_string();
        app.cursor_pos = app.input.len();

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let _guard = rt.enter();
        let mut remote = crate::tui::backend::RemoteConnection::dummy();

        rt.block_on(app.handle_remote_key(KeyCode::Char(' '), KeyModifiers::SUPER, &mut remote))
            .expect("Super+Space should arm routing");
        assert!(app.route_next_prompt_to_new_session);

        app.is_processing = true;
        app.status = ProcessingStatus::Streaming;
        app.processing_started = Some(std::time::Instant::now());
        let active_started = app.processing_started;

        rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
            .expect("armed prompt should launch split request immediately");

        assert!(!app.route_next_prompt_to_new_session);
        assert!(app.pending_split_prompt.is_some());
        assert_eq!(app.pending_split_label.as_deref(), Some("Prompt"));
        assert!(!app.pending_split_request);
        assert!(app.is_processing);
        assert!(matches!(app.status, ProcessingStatus::Streaming));
        assert_eq!(app.processing_started, active_started);
        assert!(app.current_message_id.is_none());

        app.handle_server_event(
            crate::protocol::ServerEvent::SplitResponse {
                id: 1,
                new_session_id: "session_prompt_child".to_string(),
                new_session_name: "prompt_child".to_string(),
            },
            &mut remote,
        );

        let restored = App::restore_input_for_reload("session_prompt_child")
            .expect("new prompt session should have startup submission saved");
        assert_eq!(restored.input, "hello from split");
        assert!(restored.submit_on_restore);
        assert!(restored.pending_images.is_empty());
        assert!(app.pending_split_prompt.is_none());
        assert!(app.pending_split_label.is_none());
    });
}

#[test]
fn test_remote_judge_shows_processing_until_split_response() {
    let mut app = create_test_app();
    app.is_remote = true;
    app.input = "/judge".to_string();
    app.cursor_pos = app.input.len();

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
        .expect("/judge should launch split request");

    assert!(
        app.is_processing,
        "judge launch should show client processing state"
    );
    assert!(matches!(app.status, ProcessingStatus::Sending));
    assert!(app.current_message_id.is_none());
    assert_eq!(app.status_notice(), Some("Judge launching".to_string()));
    assert!(app.pending_split_startup_message.is_some());
    assert_eq!(app.pending_split_label.as_deref(), Some("Judge"));
    assert!(!app.pending_split_request);

    app.handle_server_event(
        crate::protocol::ServerEvent::SplitResponse {
            id: 1,
            new_session_id: "session_judge_child".to_string(),
            new_session_name: "judge_child".to_string(),
        },
        &mut remote,
    );

    assert!(
        !app.is_processing,
        "split response should clear transient launch state"
    );
    assert!(matches!(app.status, ProcessingStatus::Idle));
    assert!(app.processing_started.is_none());
    assert!(app.pending_split_startup_message.is_none());
    assert!(app.pending_split_label.is_none());
}

// ====================================================================

/// Mirror the part of the remote tick loop that reveals paced stream text and
/// replays a deferred `Done` (see `remote.rs`), so tests can settle a turn
/// without spinning the real event loop.
fn drain_paced_stream_and_replay_deferred_done(
    app: &mut crate::tui::app::App,
    remote: &mut crate::tui::backend::RemoteConnection,
) {
    for _ in 0..2000 {
        // The pacer reveals against wall-clock time, so a tight loop would spin
        // without ever releasing a frame.
        std::thread::sleep(std::time::Duration::from_millis(2));
        let backlog_was_empty = app.stream_buffer.is_empty();
        let ops = app.stream_buffer.flush_smooth_frame();
        app.apply_stream_ops(ops);
        if backlog_was_empty
            && app.stream_buffer.is_empty()
            && let Some(id) = app.deferred_stream_done_id.take()
        {
            app.handle_server_event(crate::protocol::ServerEvent::Done { id }, remote);
            return;
        }
        if app.stream_buffer.is_empty() && app.deferred_stream_done_id.is_none() {
            return;
        }
    }
    panic!("paced stream backlog never drained");
}

#[test]
fn test_externally_started_turn_adopts_processing_state_and_settles_on_done() {
    // A swarm wake / background-task wake / scheduled task can start a turn in
    // this session without this client sending a message. The client must show
    // the turn as in-progress (spinner) and settle it when the terminal Done
    // arrives, instead of staying visually idle while text streams in.
    let mut app = create_test_app();
    app.is_remote = true;
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    assert!(!app.is_processing);
    assert!(app.current_message_id.is_none());

    app.handle_server_event(
        crate::protocol::ServerEvent::TextDelta {
            text: "Wake turn streaming text".to_string(),
        },
        &mut remote,
    );

    assert!(
        app.is_processing,
        "stream events while idle must adopt the externally started turn"
    );
    assert!(
        app.processing_started.is_some(),
        "adopted turn should start the elapsed/spinner clock"
    );
    assert!(
        matches!(app.status, ProcessingStatus::Streaming),
        "adopted turn should show streaming status, got {:?}",
        app.status
    );

    app.handle_server_event(
        crate::protocol::ServerEvent::MessageEnd { stop_reason: None },
        &mut remote,
    );
    app.handle_server_event(crate::protocol::ServerEvent::Done { id: 0 }, &mut remote);

    // Streaming text is revealed at a paced rate, so a `Done` that arrives with
    // a backlog is deliberately deferred (`deferred_stream_done_id`) and replayed
    // by the remote tick loop once the pacer drains. Drive that drain here rather
    // than asserting mid-flight: without it the turn correctly stays in
    // `Streaming`, and the assertion below would be testing the pacer rather than
    // turn adoption.
    drain_paced_stream_and_replay_deferred_done(&mut app, &mut remote);

    assert!(
        !app.is_processing,
        "terminal Done must settle the adopted turn"
    );
    assert!(matches!(app.status, ProcessingStatus::Idle));
    assert!(app.processing_started.is_none());
    assert!(
        app.display_messages
            .iter()
            .any(|message| message.role == "assistant"
                && message.content.contains("Wake turn streaming text")),
        "adopted turn's streamed text should commit to the transcript"
    );
}

#[test]
fn test_externally_started_tool_turn_shows_running_tool_status() {
    let mut app = create_test_app();
    app.is_remote = true;
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    app.handle_server_event(
        crate::protocol::ServerEvent::ToolStart {
            id: "tool_1".to_string(),
            name: "bash".to_string(),
        },
        &mut remote,
    );

    assert!(app.is_processing);
    assert!(
        matches!(&app.status, ProcessingStatus::RunningTool(name) if name == "bash"),
        "adopted tool turn should show the running tool, got {:?}",
        app.status
    );
}

#[test]
fn test_remote_fork_with_prompt_stages_split_prompt() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.is_remote = true;
        app.input = "/fork explore plan b".to_string();
        app.cursor_pos = app.input.len();

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let _guard = rt.enter();
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        remote.mark_history_loaded();

        rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
            .expect("/fork <prompt> should launch split request");

        assert!(app.pending_split_prompt.is_some());
        assert_eq!(app.pending_split_label.as_deref(), Some("Prompt"));
        assert!(!app.pending_split_request);

        app.handle_server_event(
            crate::protocol::ServerEvent::SplitResponse {
                id: 1,
                new_session_id: "session_fork_prompt_child".to_string(),
                new_session_name: "fork_prompt_child".to_string(),
            },
            &mut remote,
        );

        let restored = App::restore_input_for_reload("session_fork_prompt_child")
            .expect("forked session should stage the prompt");
        assert_eq!(restored.input, "explore plan b");
        assert!(restored.submit_on_restore);
        assert!(app.pending_split_prompt.is_none());
    });
}

#[test]
fn test_remote_btw_stages_question_in_forked_session() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.is_remote = true;
        app.input = "/btw what are we doing?".to_string();
        app.cursor_pos = app.input.len();

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let _guard = rt.enter();
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        remote.mark_history_loaded();

        rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
            .expect("/btw should launch split request");

        assert!(app.pending_split_prompt.is_some());

        app.handle_server_event(
            crate::protocol::ServerEvent::SplitResponse {
                id: 1,
                new_session_id: "session_btw_child".to_string(),
                new_session_name: "btw_child".to_string(),
            },
            &mut remote,
        );

        let restored = App::restore_input_for_reload("session_btw_child")
            .expect("btw fork should stage the question");
        assert_eq!(restored.input, "what are we doing?");
        assert!(restored.submit_on_restore);
    });
}

#[test]
fn test_remote_fork_without_prompt_splits_immediately() {
    let mut app = create_test_app();
    app.is_remote = true;
    app.input = "/fork".to_string();
    app.cursor_pos = app.input.len();

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
        .expect("/fork should send split request");

    assert!(app.pending_split_prompt.is_none());
    assert!(
        app.display_messages()
            .iter()
            .any(|msg| msg.content.contains("Forking session...")),
        "bare /fork should split immediately like /split"
    );
}

#[test]
fn test_credential_failure_breaker_trips_after_consecutive_auth_errors() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    // Auto-poke on with an auto-retryable pending message: this is the
    // runaway-loop shape that produced thousands of 401s per session.
    app.auto_poke_incomplete_todos = true;

    for attempt in 0..App::CREDENTIAL_FAILURE_BREAKER_THRESHOLD {
        app.rate_limit_pending_message = Some(PendingRemoteMessage {
            content: "poke".to_string(),
            images: vec![],
            is_system: true,
            system_reminder: None,
            auto_retry: true,
            retry_attempts: 0,
            retry_at: None,
        });
        app.is_processing = true;
        app.status = ProcessingStatus::Streaming;
        app.handle_server_event(
            crate::protocol::ServerEvent::Error {
                id: 100 + u64::from(attempt),
                message: "401 Unauthorized: invalid api key".to_string(),
                retry_after_secs: None,
            },
            &mut remote,
        );
    }

    assert!(
        app.rate_limit_pending_message.is_none(),
        "breaker must clear the pending auto-retry"
    );
    assert!(
        !app.auto_poke_incomplete_todos,
        "breaker must disable auto-poke"
    );
    assert!(app.overnight_auto_poke.is_none());
    assert_eq!(app.consecutive_credential_failures, 0);
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "error" && m.content.contains("Stopped automatic retries")),
        "breaker must surface an actionable stop message"
    );
}

#[test]
fn test_credential_failure_breaker_streak_resets_on_other_errors() {
    let mut app = create_test_app();

    assert!(!app.note_error_for_credential_breaker("401 unauthorized"));
    assert!(!app.note_error_for_credential_breaker("invalid api key"));
    // A non-credential error resets the streak: mixed transient failures must
    // not trip the breaker.
    assert!(!app.note_error_for_credential_breaker("500 internal server error"));
    assert_eq!(app.consecutive_credential_failures, 0);
    assert!(!app.note_error_for_credential_breaker("401 unauthorized"));
    assert!(!app.note_error_for_credential_breaker("token expired"));
    assert!(app.note_error_for_credential_breaker("unauthorized"));
}

#[test]
fn test_credential_failure_breaker_resets_on_turn_success() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    assert!(!app.note_error_for_credential_breaker("401 unauthorized"));
    assert!(!app.note_error_for_credential_breaker("401 unauthorized"));

    app.is_processing = true;
    app.current_message_id = Some(7);
    app.handle_server_event(crate::protocol::ServerEvent::Done { id: 7 }, &mut remote);

    assert_eq!(
        app.consecutive_credential_failures, 0,
        "a successful turn must reset the credential-failure streak"
    );
}
