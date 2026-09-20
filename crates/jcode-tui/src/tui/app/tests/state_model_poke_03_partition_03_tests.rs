#[test]
fn test_todo_confidence_summary_hidden_queue_is_not_user_prompt() {
    let summary =
        "All todos are done. Todo confidence summary:\n- Weighted completion confidence: 94%."
            .to_string();

    let (user_messages, reminder, display_system_messages) =
        super::helpers::partition_queued_messages(Vec::new(), vec![summary.clone()]);

    assert!(user_messages.is_empty());
    assert!(display_system_messages.is_empty());
    assert_eq!(reminder.as_deref(), Some(summary.as_str()));
}

#[test]
fn test_finish_turn_without_auto_poke_does_not_queue_confidence_summary() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Done without poke".to_string(),
                status: "completed".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: Some(crate::todo::ConfidenceState::from_legacy_score(90)),
                completion_confidence: Some(crate::todo::ConfidenceState::from_legacy_score(90)),
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        app.auto_poke_incomplete_todos = false;
        app.is_processing = true;
        super::local::finish_turn(&mut app);

        assert!(!app.pending_queued_dispatch);
        assert!(app.queued_messages().is_empty());
        assert!(
            !app.display_messages()
                .iter()
                .any(|msg| msg.content.contains("confidence summary"))
        );
    });
}

#[test]
fn test_finish_turn_auto_poke_preserves_visible_turn_started() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Keep going".to_string(),
                status: "in_progress".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        let started = Instant::now() - Duration::from_secs(45);
        app.auto_poke_incomplete_todos = true;
        app.is_processing = true;
        app.visible_turn_started = Some(started);

        super::local::finish_turn(&mut app);

        assert_eq!(app.visible_turn_started, Some(started));
        assert!(app.pending_queued_dispatch);
    });
}

#[test]
fn test_help_topic_shows_overnight_command_details() {
    let mut app = create_test_app();
    app.input = "/help overnight".to_string();
    app.submit_input();

    let msg = app
        .display_messages()
        .last()
        .expect("missing help response");
    assert_eq!(msg.role, "system");
    assert!(msg.content.contains("/overnight <hours>[h|m] [mission]"));
    assert!(msg.content.contains("review HTML page"));
    assert!(msg.content.contains("/overnight status"));
}

#[test]
fn test_overnight_status_without_runs_is_handled() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        assert!(super::commands::handle_session_command(
            &mut app,
            "/overnight status"
        ));

        let msg = app
            .display_messages()
            .last()
            .expect("missing overnight status response");
        assert_eq!(msg.role, "system");
        assert!(msg.content.contains("No overnight runs found"));
    });
}

#[test]
fn test_overnight_help_command_is_handled() {
    let mut app = create_test_app();
    assert!(super::commands::handle_session_command(
        &mut app,
        "/overnight help"
    ));

    let msg = app
        .display_messages()
        .last()
        .expect("missing overnight help response");
    assert_eq!(msg.role, "system");
    assert!(msg.content.contains("/overnight <hours>[h|m] [mission]"));
    assert!(msg.content.contains("/overnight review"));
}

#[test]
fn test_overnight_start_runs_as_visible_local_turn() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        assert!(super::commands::handle_session_command(
            &mut app,
            "/overnight 1m hi"
        ));

        assert!(
            app.pending_turn,
            "local overnight should start a visible turn"
        );
        assert!(
            app.is_processing,
            "local overnight should enter processing state"
        );
        assert!(
            app.queued_messages.is_empty(),
            "local overnight should not use remote queue"
        );
        let last_message = app
            .session
            .messages
            .last()
            .expect("overnight prompt message");
        assert!(last_message.content.iter().any(|block| matches!(
            block,
            crate::message::ContentBlock::Text { text, .. }
                if text.contains("visible Overnight Coordinator")
        )));
    });
}

#[test]
fn test_overnight_start_queues_remote_turn_without_stuck_sending() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.is_remote = true;
        assert!(super::commands::handle_session_command(
            &mut app,
            "/overnight 1m hi"
        ));

        assert!(
            !app.pending_turn,
            "remote overnight should not set local pending_turn"
        );
        assert!(
            !app.is_processing,
            "remote overnight should not get stuck in local Sending"
        );
        assert_eq!(app.queued_messages.len(), 1);
        assert!(app.queued_messages[0].contains("visible Overnight Coordinator"));
    });
}
