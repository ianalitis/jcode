#[test]
fn limited_session_list_always_includes_saved_sessions() {
    let home = ScopedJcodeHome::new("saved-beyond-limit");
    assert!(BridgeState::recent_session_index_entries().is_empty());
    let connection = Connection::open(home.path.join("session-metadata-v1.sqlite3")).unwrap();
    for index in 0..5 {
        connection
            .execute(
                "INSERT INTO recent_sessions (session_id, saved, save_label, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    format!("indexed_{index}"),
                    index == 0,
                    (index == 0).then_some("old bookmark"),
                    index,
                ],
            )
            .unwrap();
    }

    let event = only_reply_event(
        BridgeState::default()
            .api_request_to_legacy(&json!({"req": "list_sessions", "id": 1, "limit": 2})),
    );
    let ApiEvent::Sessions { sessions } = event else {
        panic!("expected sessions reply, got {event:?}");
    };
    let saved = sessions
        .iter()
        .find(|session| session.session_id == "indexed_0")
        .expect("saved session beyond the limit is listed");
    assert_eq!(saved.save_label.as_deref(), Some("old bookmark"));
}

/// Applet actions and closes forward to the daemon with their payload, and
/// live applet state reaches clients as a full snapshot.
#[test]
fn applet_requests_and_state_translate() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "id": 3, "req": "applet_action", "session_id": "s", "instance": "chart-1",
        "action": {"action": "select", "args": {"i": 2}}, "state": {"q": "x"}, "source_key": "row",
    }));
    match &out[0] {
        Outbound::Legacy(v) => {
            assert_eq!(v["type"], "applet_action");
            assert_eq!(v["instance"], "chart-1");
            assert_eq!(v["action"]["action"], "select");
            assert_eq!(v["state"]["q"], "x");
            assert_eq!(v["source_key"], "row");
        }
        other => panic!("unexpected: {other:?}"),
    }
    let out = state.api_request_to_legacy(&json!({
        "id": 4, "req": "close_applet", "session_id": "s", "instance": "chart-1",
    }));
    assert!(matches!(&out[0], Outbound::Legacy(v) if v["type"] == "close_applet"));
    let frames = state.legacy_event_to_api(&json!({
        "type": "applet_state", "session_id": "s", "snapshot": {"instances": []},
    }));
    assert!(matches!(
        &frames[..],
        [ServerFrame { event: ApiEvent::AppletState { snapshot, .. }, .. }] if snapshot.instances.is_empty()
    ));
}

#[test]
fn untitled_indexed_sessions_are_named_after_their_first_prompt_once() {
    let home = ScopedJcodeHome::new("first-prompt-title");
    std::fs::create_dir_all(home.path.join("sessions")).unwrap();
    std::fs::write(
        home.path.join("sessions/session_fox_1_aa.json"),
        json!({
            "id": "session_fox_1_aa",
            "title": null,
            "messages": [
                {"id": "m0", "role": "user", "content": [{"type": "text", "text": "<system-reminder>\n# Session Context\n</system-reminder>"}]},
                {"id": "m1", "role": "user", "display_role": "background_task", "content": [{"type": "text", "text": "background done"}]},
                {"id": "m2", "role": "user", "content": [{"type": "text", "text": "<transcription>\nRename the   sidebar rows\n</transcription>"}]},
                {"id": "m3", "role": "user", "content": [{"type": "text", "text": "later prompt"}]}
            ]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        home.path.join("sessions/session_owl_2_bb.json"),
        json!({"id": "session_owl_2_bb", "title": null, "messages": []}).to_string(),
    )
    .unwrap();
    std::fs::write(
        home.path.join("sessions/session_owl_2_bb.journal.jsonl"),
        format!(
            "{}\n",
            json!({"meta": {}, "append_messages": [{"id": "j1", "role": "user", "content": [{"type": "text", "text": "Journal prompt"}]}]})
        ),
    )
    .unwrap();
    assert!(BridgeState::recent_session_index_entries().is_empty());
    let connection = Connection::open(home.path.join("session-metadata-v1.sqlite3")).unwrap();
    for (id, at) in [("session_fox_1_aa", 2), ("session_owl_2_bb", 1)] {
        connection
            .execute(
                "INSERT INTO recent_sessions (session_id, updated_at_ms) VALUES (?1, ?2)",
                params![id, at],
            )
            .unwrap();
    }

    let list = || {
        let event = only_reply_event(
            BridgeState::default()
                .api_request_to_legacy(&json!({"req": "list_sessions", "id": 1, "limit": 10})),
        );
        let ApiEvent::Sessions { sessions } = event else {
            panic!("expected sessions reply, got {event:?}");
        };
        sessions
            .into_iter()
            .map(|session| (session.session_id, session.title))
            .collect::<BTreeMap<_, _>>()
    };
    let titles = list();
    assert_eq!(
        titles["session_fox_1_aa"].as_deref(),
        Some("Rename the sidebar rows")
    );
    assert_eq!(
        titles["session_owl_2_bb"].as_deref(),
        Some("Journal prompt")
    );
    let cached: String = connection
        .query_row(
            "SELECT generated_title FROM recent_sessions WHERE session_id = 'session_fox_1_aa'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(cached, "Rename the sidebar rows");
    // A cached title is served without rescanning the transcript.
    std::fs::write(
        home.path.join("sessions/session_fox_1_aa.json"),
        json!({"id": "session_fox_1_aa", "title": null, "messages": [
            {"id": "x", "role": "user", "content": [{"type": "text", "text": "different"}]}
        ]})
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        list()["session_fox_1_aa"].as_deref(),
        Some("Rename the sidebar rows")
    );
}
