#[test]
fn model_usage_before_catalog_is_retained_and_legacy_routes_stay_optional() {
    let mut state = state_with_session();
    let route = json!({"model":"m","provider":"OpenAI","api_method":"openai-oauth",
        "available":true,"detail":"ready"});
    let mut update = route.clone();
    update["usage"] = json!({"count":1,"last_used_unix_secs":20,"tracking_started_unix_secs":10});
    state.legacy_event_to_api(&json!({"type":"model_usage_updated","route":update}));
    state.legacy_event_to_api(
        &json!({"type":"available_models_updated","available_models":["m"],
        "available_model_routes":[route.clone()]}),
    );
    assert_eq!(state.available_routes[0].usage.as_ref().unwrap().count, 1);
    let legacy: jcode_harness_api::ModelRouteInfo = serde_json::from_value(route).unwrap();
    assert_eq!(legacy.usage, None);
    assert!(serde_json::to_value(legacy).unwrap().get("usage").is_none());
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"model_usage_updated","route":{"usage":"bad"}}))
            .is_empty()
    );
}

/// A client can ask before the catalog lands. Answering "no models" then would
/// be a lie that empties its picker, so the request waits for the real answer.
#[test]
fn list_models_before_the_catalog_asks_the_daemon() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"id": 9, "req": "list_models"}));
    match &out[..] {
        [Outbound::Legacy(value)] => assert_eq!(value["type"], "get_model_catalog"),
        other => panic!("expected a daemon round trip, got {other:?}"),
    }

    let legacy_id = match &out[0] {
        Outbound::Legacy(value) => value["id"].as_u64().unwrap(),
        _ => unreachable!(),
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "history", "id": legacy_id, "session_id": "s1",
        "available_models": ["a", "b"], "provider_model": "a",
    }));
    match &frames[0].event {
        ApiEvent::Models { models, .. } => assert_eq!(models, &["a", "b"]),
        other => panic!("unexpected: {other:?}"),
    }
    assert_eq!(frames[0].reply_to, Some(9));
}

/// A switch must resolve the caller's request *and* tell every other client
/// watching the session that the model moved under them.
#[test]
fn a_requested_model_change_replies_and_broadcasts() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "id": 4, "req": "set_model", "model": "claude-fable-5",
    }));
    let legacy_id = match &out[..] {
        [Outbound::Legacy(value)] => {
            assert_eq!(value["type"], "set_model");
            assert_eq!(value["model"], "claude-fable-5");
            value["id"].as_u64().unwrap()
        }
        other => panic!("expected a daemon request, got {other:?}"),
    };

    let frames = state.legacy_event_to_api(&json!({
        "type": "model_changed", "id": legacy_id,
        "model": "claude-fable-5", "provider_name": "anthropic",
    }));
    assert_eq!(frames.len(), 2, "expected a reply and a broadcast");
    assert_eq!(frames[0].reply_to, Some(4));
    assert!(matches!(frames[0].event, ApiEvent::Ok));
    assert_eq!(frames[1].reply_to, None);
    assert!(matches!(frames[1].event, ApiEvent::ModelInfo { .. }));
    // The cache must follow, or a picker reopened after the switch is wrong.
    assert_eq!(state.current_model.as_deref(), Some("claude-fable-5"));
}

/// The daemon reports a rejected switch in-band, on a success-shaped event.
/// Reporting success there would leave the client's picker showing a model
/// the session is not using.
#[test]
fn a_rejected_model_change_fails_the_request() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "id": 4, "req": "set_model", "model": "nope",
    }));
    let legacy_id = match &out[0] {
        Outbound::Legacy(value) => value["id"].as_u64().unwrap(),
        _ => unreachable!(),
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "model_changed", "id": legacy_id,
        "model": "nope", "error": "unknown model",
    }));
    match &frames[..] {
        [frame] => {
            assert_eq!(frame.reply_to, Some(4));
            match &frame.event {
                ApiEvent::Error { code, message } => {
                    assert_eq!(*code, ErrorCode::InvalidRequest);
                    assert_eq!(message, "unknown model");
                }
                other => panic!("unexpected: {other:?}"),
            }
        }
        other => panic!("expected one error reply, got {other:?}"),
    }
    assert_eq!(
        state.current_model, None,
        "a failed switch must not be cached"
    );
}

#[test]
fn an_empty_model_is_refused_locally() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"id": 4, "req": "set_model", "model": ""}));
    match &out[..] {
        [Outbound::Reply(frame)] => {
            assert!(matches!(frame.event, ApiEvent::Error { .. }));
        }
        other => panic!("expected a local rejection, got {other:?}"),
    }
}

#[test]
fn reasoning_effort_reports_provider_refusal() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "id": 5, "req": "set_reasoning_effort", "effort": "max",
    }));
    let legacy_id = match &out[0] {
        Outbound::Legacy(value) => {
            assert_eq!(value["effort"], "max");
            value["id"].as_u64().unwrap()
        }
        _ => unreachable!(),
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "reasoning_effort_changed", "id": legacy_id,
        "error": "provider does not support reasoning effort",
    }));
    assert_eq!(frames[0].reply_to, Some(5));
    assert!(matches!(frames[0].event, ApiEvent::Error { .. }));
}

/// An effort change is identity, like a model change: every attached client
/// needs to hear it, not only the requester. A change made by another client
/// (no pending request here) must still arrive as a `model_info` broadcast,
/// and the requester's own change gets the broadcast after its `Ok`.
#[test]
fn reasoning_effort_changes_are_broadcast_as_model_info() {
    let mut state = state_with_session();

    // Unsolicited change (another client's request id): broadcast only.
    let frames = state.legacy_event_to_api(&json!({
        "type": "reasoning_effort_changed", "id": 999, "effort": "high",
    }));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].reply_to, None);
    match &frames[0].event {
        ApiEvent::ModelInfo {
            reasoning_effort, ..
        } => assert_eq!(reasoning_effort.as_deref(), Some("high")),
        other => panic!("expected model_info, got {other:?}"),
    }

    // The same effort again is not news: no broadcast.
    let frames = state.legacy_event_to_api(&json!({
        "type": "reasoning_effort_changed", "id": 999, "effort": "high",
    }));
    assert!(frames.is_empty(), "unchanged effort must not re-broadcast");

    // This client's own change: Ok reply first, then the broadcast.
    let out = state.api_request_to_legacy(&json!({
        "id": 7, "req": "set_reasoning_effort", "effort": "low",
    }));
    let legacy_id = match &out[0] {
        Outbound::Legacy(value) => value["id"].as_u64().unwrap(),
        _ => unreachable!(),
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "reasoning_effort_changed", "id": legacy_id, "effort": "low",
    }));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].reply_to, Some(7));
    assert!(matches!(frames[0].event, ApiEvent::Ok));
    assert!(matches!(
        &frames[1].event,
        ApiEvent::ModelInfo { reasoning_effort, .. }
            if reasoning_effort.as_deref() == Some("low")
    ));
}

/// Compaction can be refused (nothing to compact, a turn in flight) and the
/// daemon says so with `success: false`, not an error frame. Telling the
/// client "done" would claim work that never happened.
#[test]
fn a_refused_compaction_is_an_error_not_a_success() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"id": 6, "req": "compact"}));
    let legacy_id = match &out[0] {
        Outbound::Legacy(value) => value["id"].as_u64().unwrap(),
        _ => unreachable!(),
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "compact_result", "id": legacy_id,
        "message": "nothing to compact", "success": false,
    }));
    match &frames[0].event {
        ApiEvent::Error { message, .. } => assert_eq!(message, "nothing to compact"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn a_scheduled_compaction_reports_its_status() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"id": 6, "req": "compact"}));
    let legacy_id = match &out[0] {
        Outbound::Legacy(value) => value["id"].as_u64().unwrap(),
        _ => unreachable!(),
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "compact_result", "id": legacy_id,
        "message": "compacting in the background", "success": true,
    }));
    match &frames[0].event {
        ApiEvent::Compacted { message, .. } => assert_eq!(message, "compacting in the background"),
        other => panic!("unexpected: {other:?}"),
    }
}

/// Clearing a title is distinct from setting an empty one, so an absent title
/// must not be sent as `""`, which the daemon would store as a real title.
#[test]
fn renaming_distinguishes_clearing_from_setting() {
    let mut state = state_with_session();
    let set = state.api_request_to_legacy(&json!({
        "id": 7, "req": "rename_session", "title": "my session",
    }));
    match &set[0] {
        Outbound::Legacy(value) => assert_eq!(value["title"], "my session"),
        other => panic!("unexpected: {other:?}"),
    }

    let clear = state.api_request_to_legacy(&json!({"id": 8, "req": "rename_session"}));
    match &clear[0] {
        Outbound::Legacy(value) => assert!(
            value.get("title").is_none(),
            "a cleared title must be absent, not empty: {value}"
        ),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn a_rename_push_becomes_a_typed_event() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "session_renamed", "session_id": "s1",
        "title": "my session", "display_title": "my session",
    }));
    match &frames[0].event {
        ApiEvent::SessionRenamed {
            session_id,
            title,
            display_title,
        } => {
            assert_eq!(session_id, "s1");
            assert_eq!(title.as_deref(), Some("my session"));
            assert_eq!(display_title, "my session");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

/// Every capability request is stateful, so none may be forwarded before the
/// connection is attached: the daemon closes the connection on those.
#[test]
fn capability_requests_need_an_attached_session() {
    for (req, extra) in [
        ("list_models", json!({})),
        ("set_model", json!({"model": "x"})),
        ("set_reasoning_effort", json!({"effort": "high"})),
        ("compact", json!({})),
        ("rename_session", json!({})),
        ("rewind_undo", json!({})),
        ("cancel_soft_interrupts", json!({})),
    ] {
        let mut state = BridgeState::default();
        let mut request = json!({"id": 1, "req": req});
        for (key, value) in extra.as_object().unwrap() {
            request[key] = value.clone();
        }
        let out = state.api_request_to_legacy(&request);
        match &out[..] {
            [Outbound::Reply(frame)] => match &frame.event {
                ApiEvent::Error { code, .. } => assert_eq!(
                    *code,
                    ErrorCode::UnknownSession,
                    "{req} should report an unattached session"
                ),
                other => panic!("{req}: unexpected {other:?}"),
            },
            other => panic!("{req} reached the daemon unattached: {other:?}"),
        }
    }
}

#[test]
fn another_sessions_broadcast_does_not_replace_the_attachment() {
    let home = ScopedJcodeHome::new("other-session-broadcast");
    write_session_record(&home.path, "session_retriever_1_a", Path::new("/workspace"));
    let mut state = BridgeState::default();
    let attach = state.api_request_to_legacy(&json!({
        "id": 7,
        "req": "attach_session",
        "session_id": "session_retriever_1_a",
    }));
    let state_id = match &attach[1] {
        Outbound::Legacy(value) => value["id"].as_u64().expect("state request id"),
        other => panic!("unexpected attach output: {other:?}"),
    };
    state.legacy_event_to_api(&json!({
        "type": "state",
        "id": state_id,
        "session_id": "session_retriever_1_a",
    }));

    state.legacy_event_to_api(&json!({
        "type": "session",
        "session_id": "session_pawprint_2_b",
    }));

    assert!(matches!(
        state
            .api_request_to_legacy(&json!({
                "id": 8,
                "req": "send_message",
                "session_id": "session_retriever_1_a",
                "content": "still routed to the attached session",
            }))
            .as_slice(),
        [Outbound::Legacy(_)]
    ));
}

#[test]
fn another_sessions_state_does_not_replace_the_attachment() {
    let home = ScopedJcodeHome::new("other-session-state");
    write_session_record(&home.path, "session_retriever_1_a", Path::new("/workspace"));
    let mut state = BridgeState::default();
    let attach = state.api_request_to_legacy(&json!({
        "id": 7,
        "req": "attach_session",
        "session_id": "session_retriever_1_a",
    }));
    let state_id = match &attach[1] {
        Outbound::Legacy(value) => value["id"].as_u64().expect("state request id"),
        other => panic!("unexpected attach output: {other:?}"),
    };
    state.legacy_event_to_api(&json!({
        "type": "state",
        "id": state_id,
        "session_id": "session_retriever_1_a",
    }));

    state.legacy_event_to_api(&json!({
        "type": "state",
        "id": state_id + 100,
        "session_id": "session_pawprint_2_b",
    }));

    assert!(matches!(
        state
            .api_request_to_legacy(&json!({
                "id": 8,
                "req": "send_message",
                "session_id": "session_retriever_1_a",
                "content": "still routed to the attached session",
            }))
            .as_slice(),
        [Outbound::Legacy(_)]
    ));
}

#[test]
fn legacy_request_ids_are_unique_across_bridge_connections() {
    let home = ScopedJcodeHome::new("unique-legacy-request-ids");
    write_session_record(&home.path, "session_first", Path::new("/workspace/first"));
    write_session_record(&home.path, "session_second", Path::new("/workspace/second"));
    let mut first = BridgeState::default();
    let mut second = BridgeState::default();
    let first_attach = first.api_request_to_legacy(&json!({
        "id": 1,
        "req": "attach_session",
        "session_id": "session_first",
    }));
    let second_attach = second.api_request_to_legacy(&json!({
        "id": 1,
        "req": "attach_session",
        "session_id": "session_second",
    }));
    let request_id = |outbound: &[Outbound]| match &outbound[1] {
        Outbound::Legacy(value) => value["id"].as_u64().expect("state request id"),
        other => panic!("unexpected attach output: {other:?}"),
    };

    assert_ne!(request_id(&first_attach), request_id(&second_attach));
}

#[test]
fn colliding_state_id_for_another_target_does_not_complete_attach() {
    let home = ScopedJcodeHome::new("colliding-state-id");
    write_session_record(&home.path, "session_wanted", Path::new("/workspace"));
    let mut state = BridgeState::default();
    let attach = state.api_request_to_legacy(&json!({
        "id": 7,
        "req": "attach_session",
        "session_id": "session_wanted",
    }));
    let state_id = match &attach[1] {
        Outbound::Legacy(value) => value["id"].as_u64().expect("state request id"),
        other => panic!("unexpected attach output: {other:?}"),
    };

    assert!(
        state
            .legacy_event_to_api(&json!({
                "type": "state",
                "id": state_id,
                "session_id": "session_other",
            }))
            .is_empty()
    );
    assert!(state.session_id.is_none());
}

/// A session id becomes a filesystem path, so it must be treated as untrusted.
///
/// The id arrives straight off the wire and is interpolated into
/// `<home>/sessions/<id>.json`. Without validation, a traversal id is a
/// readable path, and `peek_session` returns whatever it finds there.
#[test]
fn a_session_id_cannot_escape_the_sessions_directory() {
    for hostile in [
        "../../../etc/passwd",
        "../.ssh/id_rsa",
        "a/b",
        "a\\b",
        "..",
        "",
        "with space",
        "semi;colon",
    ] {
        assert!(
            BridgeState::session_record_path(hostile).is_none(),
            "`{hostile}` must not resolve to a session record path"
        );
    }
}

#[test]
fn a_plain_session_id_still_resolves() {
    let path = BridgeState::session_record_path("session_otter_1785728596263_80eb5ad6012a1864")
        .expect("a normal session id must resolve");
    assert!(path.ends_with("session_otter_1785728596263_80eb5ad6012a1864.json"));
    assert!(
        path.parent().is_some_and(|dir| dir.ends_with("sessions")),
        "records live in the sessions directory: {}",
        path.display()
    );
}

/// Session records must be read from the *instance's* home, not the user's.
///
/// `launch()` gives an embedded instance its own `JCODE_HOME` precisely so it
/// cannot see the user's work. Reading the user's home directly made
/// `peek_session` return the real transcripts of the jcode the user runs
/// interactively, from a client that was supposed to be sandboxed.
#[test]
fn session_records_are_read_from_the_instance_home() {
    let home = ScopedJcodeHome::new("instance-home");
    let path = BridgeState::session_record_path("session_x_1_a");
    let path = path.expect("a normal session id must resolve");
    assert!(
        path.starts_with(&home.path),
        "JCODE_HOME must scope session records, got {}",
        path.display()
    );
}

#[test]
fn unattached_list_sessions_handles_no_session_candidates() {
    let home = ScopedJcodeHome::new("empty-session-discovery");
    let sessions_dir = home.path.join("sessions");
    let mut state = BridgeState::default();

    // Missing directory, empty directory, and entries that are all filtered out.
    for layout in 0..3 {
        if layout == 1 {
            std::fs::create_dir(&sessions_dir).unwrap();
        } else if layout == 2 {
            std::fs::write(sessions_dir.join("not-a-session.txt"), "ignored").unwrap();
            std::fs::create_dir(sessions_dir.join("directory.json")).unwrap();
        }
        for limit in [None, Some(0), Some(10)] {
            let event = only_reply_event(state.api_request_to_legacy(&json!({
                "req": "list_sessions", "id": 1, "limit": limit,
            })));
            assert_eq!(event, ApiEvent::Sessions { sessions: vec![] });
            assert_eq!(
                only_reply_event(state.api_request_to_legacy(&json!({"req": "ping", "id": 2}))),
                ApiEvent::Pong,
            );
        }
    }

    write_session_record(&home.path, "first_session", &home.path);
    assert_eq!(BridgeState::stored_session_ids(None), ["first_session"]);
}

#[test]
fn unattached_list_sessions_discovers_all_persisted_records() {
    let home = ScopedJcodeHome::new("persisted-discovery");
    let first_root = home.path.join("first-project");
    let second_root = home.path.join("second-project");
    std::fs::create_dir_all(&first_root).unwrap();
    std::fs::create_dir_all(&second_root).unwrap();
    write_session_record_with_titles(
        &home.path,
        "persisted_one",
        &first_root,
        Some("  Generated first title  "),
        None,
    );
    write_session_record_with_titles(
        &home.path,
        "persisted_two",
        &second_root,
        Some("Generated second title"),
        Some("  Custom second title  "),
    );
    std::fs::write(home.path.join("sessions/not-a-session.txt"), "ignored").unwrap();

    let event = only_reply_event(
        BridgeState::default().api_request_to_legacy(&json!({"req": "list_sessions", "id": 1})),
    );
    let ApiEvent::Sessions { sessions } = event else {
        panic!("expected sessions reply, got {event:?}");
    };
    assert_eq!(
        sessions
            .iter()
            .map(|session| session.session_id.as_str())
            .collect::<Vec<_>>(),
        ["persisted_one", "persisted_two"]
    );
    assert_eq!(sessions[0].working_dir.as_deref(), first_root.to_str());
    assert_eq!(sessions[1].working_dir.as_deref(), second_root.to_str());
    assert_eq!(sessions[0].title.as_deref(), Some("Generated first title"));
    assert_eq!(sessions[1].title.as_deref(), Some("Custom second title"));
}

#[test]
fn limited_session_list_reads_compact_index_without_transcript_records() {
    let home = ScopedJcodeHome::new("metadata-index");
    assert!(BridgeState::recent_session_index_entries().is_empty());
    let mut connection = Connection::open(home.path.join("session-metadata-v1.sqlite3")).unwrap();
    let transaction = connection.transaction().unwrap();
    for index in 0..100 {
        transaction
            .execute(
                "INSERT INTO recent_sessions (
                     session_id, working_dir, todo_title, saved, updated_at_ms, last_active_at_ms
                 ) VALUES (?1, '/indexed/project', ?2, ?4, ?3, ?3)",
                params![
                    format!("indexed_{index:03}"),
                    format!("Indexed goal {index}"),
                    index,
                    index == 99,
                ],
            )
            .unwrap();
    }
    transaction.commit().unwrap();

    let event = only_reply_event(
        BridgeState::default()
            .api_request_to_legacy(&json!({"req": "list_sessions", "id": 1, "limit": 100})),
    );
    let ApiEvent::Sessions { sessions } = event else {
        panic!("expected sessions reply, got {event:?}");
    };
    assert_eq!(sessions.len(), 100);
    let newest = sessions
        .iter()
        .find(|session| session.session_id == "indexed_099")
        .expect("indexed newest session");
    assert!(newest.saved);
    assert_eq!(newest.updated_at_ms, Some(99));
    assert_eq!(newest.last_active_at_ms, Some(99));
    assert!(sessions.iter().all(|session| {
        session
            .title
            .as_deref()
            .is_some_and(|title| title.starts_with("Indexed goal "))
    }));
}

#[test]
fn runtime_info_reports_the_active_provider_and_complete_route_catalog() {
    let mut state = state_with_session();
    state.legacy_event_to_api(&json!({
        "type": "available_models_updated",
        "provider_name": "anthropic",
        "provider_model": "claude-sonnet",
        "reasoning_effort": "high",
        "available_models": ["claude-sonnet", "gemini-pro"],
        "available_model_routes": [
            {
                "model": "claude-sonnet",
                "provider": "anthropic",
                "api_method": "messages",
                "available": true,
                "detail": "ready"
            },
            {
                "model": "gemini-pro",
                "provider": "gemini",
                "api_method": "generateContent",
                "available": false,
                "detail": "credential missing"
            }
        ]
    }));

    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "get_runtime_info",
        "id": 4,
        "session_id": "s1"
    })));
    let ApiEvent::RuntimeInfo {
        session_id,
        provider,
        model,
        reasoning_effort,
        routes,
    } = event
    else {
        panic!("expected runtime info, got {event:?}");
    };
    assert_eq!(session_id, "s1");
    assert_eq!(provider.as_deref(), Some("anthropic"));
    assert_eq!(model.as_deref(), Some("claude-sonnet"));
    assert_eq!(reasoning_effort.as_deref(), Some("high"));
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[1].provider, "gemini");
    assert!(!routes[1].available);
}

#[test]
fn archive_restore_and_retention_are_reversible_and_owner_only() {
    let home = ScopedJcodeHome::new("archive");
    let root = home.path.join("project");
    std::fs::create_dir_all(&root).unwrap();
    write_session_record(&home.path, "recent_session", &root);
    let old_record = write_session_record(&home.path, "old_session", &root);
    let old_time = SystemTime::now() - std::time::Duration::from_secs(3 * 86_400);
    std::fs::File::options()
        .write(true)
        .open(&old_record)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(old_time))
        .unwrap();

    let mut state = BridgeState::default();
    assert!(matches!(
        only_reply_event(state.api_request_to_legacy(&json!({
            "req": "archive_session",
            "id": 1,
            "session_id": "recent_session"
        }))),
        ApiEvent::Ok
    ));
    let ApiEvent::Sessions { sessions } =
        only_reply_event(state.api_request_to_legacy(&json!({"req": "list_sessions", "id": 2})))
    else {
        panic!("expected sessions");
    };
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "old_session");

    assert!(matches!(
        only_reply_event(state.api_request_to_legacy(&json!({
            "req": "restore_session",
            "id": 3,
            "session_id": "recent_session"
        }))),
        ApiEvent::Ok
    ));
    assert!(matches!(
        only_reply_event(state.api_request_to_legacy(&json!({
            "req": "set_retention_policy",
            "id": 4,
            "archive_after_days": 1
        }))),
        ApiEvent::Ok
    ));

    let ApiEvent::Sessions { sessions } = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "list_sessions",
        "id": 5,
        "include_archived": true
    }))) else {
        panic!("expected sessions");
    };
    let old = sessions
        .iter()
        .find(|session| session.session_id == "old_session")
        .expect("old session remains restorable");
    assert!(old.archived);
    assert!(old.archived_at_ms.is_some());
    let recent = sessions
        .iter()
        .find(|session| session.session_id == "recent_session")
        .expect("restored session is listed");
    assert!(!recent.archived);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(home.path.join("sdk-archive.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        let home_mode = std::fs::metadata(&home.path).unwrap().permissions().mode() & 0o777;
        assert_eq!(home_mode, 0o700);
    }
}

#[test]
fn notify_auth_changed_is_secret_free_and_acknowledged() {
    let mut state = BridgeState::default();
    let outbound = state.api_request_to_legacy(&json!({
        "req": "notify_auth_changed", "id": 42, "provider": "openai"
    }));
    let [Outbound::Legacy(notify)] = outbound.as_slice() else {
        panic!("expected one non-transcript control request");
    };
    assert_eq!(notify["type"], "notify_auth_changed");
    assert_eq!(notify["provider"], "openai");
    assert!(notify.get("content").is_none());
    let frames = state.legacy_event_to_api(&json!({"type": "ack", "id": notify["id"]}));
    assert_eq!(frames[0].reply_to, Some(42));
    assert!(matches!(frames[0].event, ApiEvent::Ok));
    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "notify_auth_changed", "id": 43, "provider": "invalid\nprivate-fixture-secret"
    })));
    assert!(matches!(
        event,
        ApiEvent::Error {
            code: ErrorCode::InvalidRequest,
            ..
        }
    ));
    assert!(!format!("{event:?}").contains("private-fixture-secret"));
}

#[test]
fn credential_provisioning_normalizes_gemini_and_supports_jcode() {
    let home = ScopedJcodeHome::new("credentials");
    let config = home.path.join("config/jcode");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("gemini.env"),
        "GOOGLE_API_KEY=stale\nKEEP_ME=yes\n",
    )
    .unwrap();
    let mut state = BridgeState::default();

    let outbound = state.api_request_to_legacy(&json!({
        "req": "set_api_key",
        "id": 7,
        "provider": "google-gemini",
        "api_key": "gemini-secret"
    }));
    let [Outbound::Legacy(notify)] = outbound.as_slice() else {
        panic!("credential change should notify the daemon: {outbound:?}");
    };
    assert_eq!(notify["provider"], "gemini");
    let legacy_id = notify["id"].as_u64().unwrap();
    let frames = state.legacy_event_to_api(&json!({"type": "ack", "id": legacy_id}));
    assert!(matches!(
        &frames[0].event,
        ApiEvent::CredentialUpdated { provider, configured }
            if provider == "gemini" && *configured
    ));
    let gemini = std::fs::read_to_string(config.join("gemini.env")).unwrap();
    assert!(gemini.contains("GEMINI_API_KEY=gemini-secret\n"));
    assert!(gemini.contains("KEEP_ME=yes\n"));
    assert!(!gemini.contains("GOOGLE_API_KEY"));

    let outbound = state.api_request_to_legacy(&json!({
        "req": "set_api_key",
        "id": 8,
        "provider": "subscription",
        "api_key": "jcode-secret"
    }));
    let [Outbound::Legacy(notify)] = outbound.as_slice() else {
        panic!("jcode credential should notify the daemon: {outbound:?}");
    };
    assert_eq!(notify["provider"], "jcode");
    assert_eq!(
        std::fs::read_to_string(config.join("jcode-subscription.env")).unwrap(),
        "JCODE_API_KEY=jcode-secret\n"
    );

    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "set_api_key",
        "id": 9,
        "provider": "gemini",
        "api_key": "line one\nline two"
    })));
    assert!(matches!(
        event,
        ApiEvent::Error {
            code: ErrorCode::InvalidRequest,
            ..
        }
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&config).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(config.join("gemini.env"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[cfg(unix)]
#[test]
fn owner_only_writes_refuse_symlink_targets_and_directories() {
    use std::os::unix::fs::symlink;

    let home = ScopedJcodeHome::new("credential-symlinks");
    let outside_file = home.path.join("outside.env");
    std::fs::write(&outside_file, "unchanged\n").unwrap();
    let config = home.path.join("config/jcode");
    std::fs::create_dir_all(&config).unwrap();
    symlink(&outside_file, config.join("gemini.env")).unwrap();
    let mut state = BridgeState::default();
    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "set_api_key",
        "id": 1,
        "provider": "gemini",
        "api_key": "must-not-land"
    })));
    assert!(matches!(
        event,
        ApiEvent::Error {
            code: ErrorCode::Internal,
            ..
        }
    ));
    assert_eq!(
        std::fs::read_to_string(&outside_file).unwrap(),
        "unchanged\n"
    );

    std::fs::remove_file(config.join("gemini.env")).unwrap();
    std::fs::remove_dir(&config).unwrap();
    let outside_dir = home.path.join("outside-config");
    std::fs::create_dir_all(&outside_dir).unwrap();
    symlink(&outside_dir, &config).unwrap();
    let event = only_reply_event(BridgeState::default().api_request_to_legacy(&json!({
        "req": "set_api_key",
        "id": 2,
        "provider": "jcode",
        "api_key": "must-not-land"
    })));
    assert!(matches!(
        event,
        ApiEvent::Error {
            code: ErrorCode::Internal,
            ..
        }
    ));
    assert!(!outside_dir.join("jcode-subscription.env").exists());
}

#[cfg(unix)]
#[test]
fn rooted_file_operations_reject_traversal_and_symlink_escapes_and_bound_results() {
    use std::os::unix::fs::symlink;

    let home = ScopedJcodeHome::new("rooted-files");
    let root = home.path.join("project");
    let outside = home.path.join("outside");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(root.join("src/unicode.txt"), "éx secret\n").unwrap();
    for index in 0..8 {
        std::fs::write(root.join(format!("src/match-{index}.txt")), "needle\n").unwrap();
    }
    std::fs::write(outside.join("outside-secret.txt"), "outside needle\n").unwrap();
    symlink(&outside, root.join("escape")).unwrap();
    write_session_record(&home.path, "s1", &root);
    let mut state = state_with_session();

    for hostile in ["../outside/outside-secret.txt", "escape/outside-secret.txt"] {
        let event = only_reply_event(state.api_request_to_legacy(&json!({
            "req": "read_file",
            "id": 1,
            "session_id": "s1",
            "path": hostile
        })));
        assert!(matches!(
            event,
            ApiEvent::Error {
                code: ErrorCode::InvalidRequest,
                ..
            }
        ));
    }

    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "read_file",
        "id": 2,
        "session_id": "s1",
        "path": "src/unicode.txt",
        "max_bytes": 2
    })));
    assert!(matches!(
        event,
        ApiEvent::FileContent {
            content,
            truncated: true,
            ..
        } if content == "é"
    ));

    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "find_files",
        "id": 3,
        "session_id": "s1",
        "query": "outside-secret",
        "limit": 1000000
    })));
    assert!(matches!(event, ApiEvent::Files { paths, .. } if paths.is_empty()));

    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "search_text",
        "id": 4,
        "session_id": "s1",
        "query": "needle",
        "limit": 3
    })));
    let ApiEvent::TextMatches { matches, .. } = event else {
        panic!("expected bounded text matches, got {event:?}");
    };
    assert_eq!(matches.len(), 3);
    assert!(
        matches
            .iter()
            .all(|found| !found.path.starts_with("escape/"))
    );

    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "file_status",
        "id": 5,
        "session_id": "s1",
        "path": "src/missing.txt"
    })));
    assert!(matches!(
        event,
        ApiEvent::FileStatus {
            exists: false,
            ref kind,
            ..
        } if kind == "missing"
    ));
}

#[test]
fn an_empty_catalog_replaces_stale_models_and_is_cached() {
    let mut state = state_with_session();
    state.legacy_event_to_api(&json!({
        "type": "available_models_updated", "available_models": ["old-model"]
    }));
    let frames = state.legacy_event_to_api(&json!({
        "type": "available_models_updated", "available_models": [],
        "available_model_routes": []
    }));
    assert!(matches!(&frames[1].event, ApiEvent::RuntimeInfo { routes, .. } if routes.is_empty()));
    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "list_models", "id": 80, "session_id": "s1"
    })));
    assert!(matches!(event, ApiEvent::Models { models, .. } if models.is_empty()));
}

#[test]
fn runtime_info_waits_for_initial_catalog_and_propagates_errors() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "req": "get_runtime_info", "id": 81, "session_id": "s1"
    }));
    let Outbound::Legacy(probe) = &out[0] else {
        panic!("must fetch catalog");
    };
    assert_eq!(probe["type"], "get_model_catalog");
    let frames = state.legacy_event_to_api(&json!({
        "type": "error", "id": probe["id"], "message": "catalog unavailable"
    }));
    assert_eq!(frames[0].reply_to, Some(81));
    assert!(matches!(frames[0].event, ApiEvent::Error { .. }));

    let out = state.api_request_to_legacy(&json!({
        "req": "get_runtime_info", "id": 82, "session_id": "s1"
    }));
    let Outbound::Legacy(probe) = &out[0] else {
        panic!("must retry catalog");
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "history", "id": probe["id"], "provider_model": "ready-model",
        "available_models": ["ready-model"], "messages": []
    }));
    assert_eq!(frames[0].reply_to, Some(82));
    assert!(
        matches!(&frames[0].event, ApiEvent::RuntimeInfo { model, .. }
        if model.as_deref() == Some("ready-model"))
    );
}
