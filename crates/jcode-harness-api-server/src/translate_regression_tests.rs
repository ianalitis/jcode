use super::*;

#[test]
fn persisted_metadata_reads_large_transcripts_from_bounded_windows() {
    let home = ScopedJcodeHome::new("bounded-metadata");
    let sessions = home.path.join("sessions");
    std::fs::create_dir_all(&sessions).expect("create sessions directory");
    let path = sessions.join("session_large.json");
    let mut file = std::fs::File::create(&path).expect("create large session");
    write!(
        file,
        "{{\"id\":\"session_large\",\"title\":\"Generated title\",\"messages\":[\""
    )
    .unwrap();
    for _ in 0..(2 * 1024) {
        file.write_all(&[b'x'; 1024]).unwrap();
    }
    write!(
        file,
        "\"],\"working_dir\":\"/workspace/large\",\"custom_title\":\"Pinned title\"}}"
    )
    .unwrap();
    drop(file);

    let metadata = BridgeState::resolve_session_metadata("session_large").expect("metadata");
    assert_eq!(metadata.working_dir.as_deref(), Some("/workspace/large"));
    assert_eq!(metadata.title.as_deref(), Some("Generated title"));
    assert_eq!(metadata.custom_title.as_deref(), Some("Pinned title"));
    assert_eq!(metadata.display_title().as_deref(), Some("Pinned title"));
}

#[test]
fn outbound_layout_does_not_inline_the_large_reply_payload() {
    let outbound = std::mem::size_of::<Outbound>();
    let legacy = std::mem::size_of::<Value>();
    let frame = std::mem::size_of::<ServerFrame>();
    eprintln!("layout bytes: Outbound={outbound}, Legacy={legacy}, ServerFrame={frame}");
    assert!(
        outbound <= 2 * legacy,
        "large replies inflate every legacy action: {outbound} bytes"
    );
}

#[test]
fn local_reply_retains_server_frame_wire_encoding() {
    let out = BridgeState::default().api_request_to_legacy(&json!({"req": "ping", "id": 7}));
    let [Outbound::Reply(frame)] = out.as_slice() else {
        panic!("expected local reply")
    };
    assert_eq!(
        serde_json::to_value(frame).unwrap(),
        serde_json::to_value(ServerFrame::reply(7, ApiEvent::Pong)).unwrap()
    );
}

#[test]
fn metadata_string_keeps_first_last_null_and_missing_semantics() {
    let bytes = br#"{"title":"first","nested":{"title":"last"}}"#;
    assert_eq!(
        BridgeState::metadata_string(bytes, "title", false).as_deref(),
        Some("first")
    );
    assert_eq!(
        BridgeState::metadata_string(bytes, "title", true).as_deref(),
        Some("last")
    );
    assert_eq!(BridgeState::metadata_string(bytes, "missing", true), None);
    let bytes = br#"[{"title":"first"},{"title":null}]"#;
    assert_eq!(BridgeState::metadata_string(bytes, "title", true), None);
}

#[test]
fn stored_sessions_keep_descending_mtime_order_and_limit() {
    let home = ScopedJcodeHome::new("mtime-order");
    for (id, seconds) in [("middle", 20), ("oldest", 10), ("newest", 30)] {
        let path = write_session_record(&home.path, id, &home.path);
        std::fs::File::open(path)
            .unwrap()
            .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(seconds))
            .unwrap();
    }
    assert_eq!(
        BridgeState::stored_session_ids(Some(2)),
        ["newest", "middle"]
    );
    assert_eq!(
        BridgeState::stored_session_ids(None),
        ["newest", "middle", "oldest"]
    );
    assert!(BridgeState::stored_session_ids(Some(0)).is_empty());
}

#[test]
fn late_recovered_suffix_completes_previously_empty_or_unseen_text() {
    for partial in [false, true] {
        let mut state = state_with_session();
        if partial {
            state.legacy_event_to_api(&json!({"type":"text_delta","text":"to=fun"}));
        }
        state.legacy_event_to_api(&json!({"type":"text_replace","text":""}));
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"message_end"}))
                .is_empty()
        );
        let recovered =
            state.legacy_event_to_api(&json!({"type":"text_replace","text":"Retained suffix"}));
        assert!(matches!(recovered.as_slice(), [
            ServerFrame { event: ApiEvent::TextReplace { text, message_id: Some(a), .. }, .. },
            ServerFrame { event: ApiEvent::TextDone { message_id: Some(b), .. }, .. }
        ] if text == "Retained suffix" && a == b));
        assert_eq!(
            state
                .legacy_event_to_api(&json!({"type":"tool_start","id":"t","name":"read"}))
                .len(),
            1
        );
        assert_eq!(
            state
                .legacy_event_to_api(&json!({"type":"tool_exec","id":"t","name":"read"}))
                .len(),
            1
        );
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"text_done"}))
                .is_empty()
        );
    }
}

#[test]
fn new_request_retry_does_not_retract_previous_response() {
    for activity in ["reasoning_delta", "tool_start"] {
        let mut state = state_with_session();
        state.legacy_event_to_api(&json!({"type":"text_delta","text":"Committed"}));
        state.legacy_event_to_api(&json!({"type":"message_end"}));
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"kv_cache_request"}))
                .is_empty()
        );
        state.legacy_event_to_api(
            &json!({"type":activity,"text":"thinking","id":"t","name":"read"}),
        );
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"retry_rollback","attempt":2,"max":3}))
                .is_empty()
        );
    }
}

#[test]
fn pdf_panels_opt_in_and_survive_native_api_attach_reconnect_and_live_updates() {
    let snapshot = json!({"focus_revision":123,"focused_page_id":"report","pages":[{
        "id":"report","title":"Report","file_path":"/report.pdf","format":"pdf",
        "source":"linked_file","content":"PDF document fallback","updated_at_ms":42,
        "pdf_data":"JVBERi0xLjQKJSVFT0Y="
    }]});
    for action in ["create_session", "attach_session"] {
        let mut state = BridgeState::default();
        let actions = state.api_request_to_legacy(&json!({
            "req":action,"id":71,"session_id":"recover","working_dir":"/workspace"
        }));
        assert!(actions.iter().any(|action| matches!(action,
            Outbound::Legacy(request) if request["type"] == "subscribe"
                && request["supports_pdf_panels"] == true
        )));
    }
    let mut state = BridgeState::default();
    for _ in 0..2 {
        let (mut history, reply) = recovery_attach(&mut state, Some("recover"));
        history["side_panel"] = snapshot.clone();
        state.legacy_event_to_api(&reply);
        assert_panel(
            &state.legacy_event_to_api(&history),
            "recover",
            snapshot.clone(),
        );
        assert_panel(
            &state.legacy_event_to_api(&json!({
                "type":"side_panel_state","snapshot":snapshot
            })),
            "recover",
            snapshot.clone(),
        );
    }
}

#[test]
fn request_boundary_closes_provider_without_message_end_and_ids_survive_turns() {
    let mut state = state_with_session();
    let first = state.legacy_event_to_api(&json!({"type":"text_delta","text":"first"}));
    assert!(matches!(
        &state.legacy_event_to_api(&json!({"type":"kv_cache_request"}))[0].event,
        ApiEvent::TextDone { .. }
    ));
    state.legacy_event_to_api(&json!({"type":"done","id":0}));
    let next = state.legacy_event_to_api(&json!({"type":"text_delta","text":"next turn"}));
    let ApiEvent::TextDelta { message_id: a, .. } = &first[0].event else {
        panic!()
    };
    let ApiEvent::TextDelta { message_id: b, .. } = &next[0].event else {
        panic!()
    };
    assert_ne!(a, b);
}

#[test]
fn session_list_exposes_durable_edit_stats_without_phantom_sidecar_sessions() {
    let home = ScopedJcodeHome::new("edit-stats-list");
    write_session_record(&home.path, "session_edits", Path::new("/workspace"));
    let stats = jcode_harness_api::SessionEditStats {
        added: 42,
        removed: 9,
        approximate: false,
    };
    jcode_harness_api::record_session_edit(&home.path.join("sessions"), "session_edits", stats)
        .unwrap();
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req":"list_sessions","id":77}));
    let Outbound::Reply(frame) = &out[0] else {
        panic!("expected direct reply")
    };
    let ApiEvent::Sessions { sessions } = &frame.event else {
        panic!("expected sessions")
    };
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "session_edits");
    assert_eq!(sessions[0].edit_stats, Some(stats));
    // Repeated refresh reads the updated sidecar, without a daemon restart.
    jcode_harness_api::record_session_edit(&home.path.join("sessions"), "session_edits", stats)
        .unwrap();
    let out = state.api_request_to_legacy(&json!({"req":"list_sessions","id":78}));
    let Outbound::Reply(frame) = &out[0] else {
        panic!("expected direct reply")
    };
    let ApiEvent::Sessions { sessions } = &frame.event else {
        panic!("expected sessions")
    };
    assert_eq!(sessions[0].edit_stats.unwrap().added, 84);
}

#[test]
fn side_panel_attach_and_reconnect_hydrate_in_either_order_and_clear_empty() {
    for history_first in [false, true] {
        for target in [None, Some("recover")] {
            let mut state = BridgeState::default();
            for snapshot in [Some(markdown_panel()), None] {
                let (mut history, state_reply) = recovery_attach(&mut state, target);
                if let Some(snapshot) = &snapshot {
                    history["side_panel"] = snapshot.clone();
                }
                if !history_first {
                    state.legacy_event_to_api(&state_reply);
                }
                assert_panel(
                    &state.legacy_event_to_api(&history),
                    "recover",
                    snapshot.unwrap_or(json!({})),
                );
                if history_first {
                    state.legacy_event_to_api(&state_reply);
                }
                assert!(state.legacy_event_to_api(&history).is_empty());
            }
        }
    }
}

#[test]
fn side_panel_history_refresh_hydrates_but_catalog_does_not_clear_panel() {
    let mut state = state_with_session();
    let actions =
        state.api_request_to_legacy(&json!({"req":"get_history", "id":12, "session_id":"s1"}));
    let Outbound::Legacy(request) = &actions[0] else {
        panic!("history request")
    };
    assert_panel(&state.legacy_event_to_api(&json!({"type":"history", "id":request["id"], "session_id":"s1", "messages":[], "side_panel":markdown_panel()})), "s1", markdown_panel());
    let (_, _) = recovery_attach(&mut state, Some("recover"));
    let id = state.pending_model_probe.unwrap();
    let frames =
        state.legacy_event_to_api(&json!({"type":"history", "id":id, "session_id":"recover"}));
    assert!(
        !frames
            .iter()
            .any(|f| matches!(f.event, ApiEvent::SidePanelState { .. }))
    );
}

#[test]
fn side_panel_live_updates_preserve_content_focus_and_session_routing() {
    let mut state = state_with_session();
    for snapshot in [markdown_panel(), json!({})] {
        assert_panel(
            &state.legacy_event_to_api(&json!({"type":"side_panel_state", "snapshot":snapshot})),
            "s1",
            snapshot,
        );
    }
    assert_panel(
        &state.legacy_event_to_api(
            &json!({"type":"side_panel_state", "session_id":"other", "snapshot":markdown_panel()}),
        ),
        "other",
        markdown_panel(),
    );
    assert_eq!(state.session_id.as_deref(), Some("s1"));
    for snapshot in [Value::Null, json!({"pages":"broken"})] {
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"side_panel_state","snapshot":snapshot}))
                .is_empty()
        );
    }
    assert!(
        BridgeState::default()
            .legacy_event_to_api(&json!({"type":"side_panel_state", "snapshot":markdown_panel()}))
            .is_empty()
    );
}

#[test]
fn side_panel_state_hydration_requires_correlated_attachment() {
    let mut state = BridgeState::default();
    let (_, mut reply) = recovery_attach(&mut state, Some("recover"));
    reply["side_panel"] = markdown_panel();
    let mut wrong = reply.clone();
    wrong["session_id"] = json!("other");
    assert!(state.legacy_event_to_api(&wrong).is_empty());
    wrong = reply.clone();
    wrong["id"] = json!(u64::MAX);
    assert!(state.legacy_event_to_api(&wrong).is_empty());
    assert_panel(
        &state.legacy_event_to_api(&reply),
        "recover",
        markdown_panel(),
    );
}

#[test]
fn text_framing_preserves_chunks_and_reasoning_then_separates_messages() {
    let mut state = state_with_session();
    let first = state.legacy_event_to_api(&json!({"type":"text_delta","text":"The cause is "}));
    let ApiEvent::TextDelta {
        message_id: Some(id),
        ..
    } = &first[0].event
    else {
        panic!("missing id")
    };
    state.legacy_event_to_api(&json!({"type":"reasoning_delta","text":"thinking"}));
    state.legacy_event_to_api(&json!({"type":"reasoning_done"}));
    let second = state.legacy_event_to_api(&json!({"type":"text_delta","text":"the retry loop."}));
    assert!(
        matches!(&second[0].event, ApiEvent::TextDelta { message_id: Some(next), .. } if next == id)
    );
    let end = state.legacy_event_to_api(&json!({"type":"text_done"}));
    assert!(
        matches!(&end[0].event, ApiEvent::TextDone { message_id: Some(next), .. } if next == id)
    );
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"text_done"}))
            .is_empty()
    );
    let next = state.legacy_event_to_api(&json!({"type":"text_delta","text":"Another message"}));
    assert!(
        matches!(&next[0].event, ApiEvent::TextDelta { message_id: Some(next), .. } if next != id)
    );
    assert!(matches!(
        &state.legacy_event_to_api(&json!({"type":"message_end"}))[0].event,
        ApiEvent::TextDone { .. }
    ));
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"message_end"}))
            .is_empty()
    );
    assert!(matches!(
        state
            .legacy_event_to_api(&json!({"type":"done","id":0}))
            .as_slice(),
        [ServerFrame {
            event: ApiEvent::TurnDone { .. },
            ..
        }]
    ));
}

#[test]
fn text_framing_tools_and_turn_fallback_close_once_without_phantom_messages() {
    let mut state = state_with_session();
    state.legacy_event_to_api(&json!({"type":"text_delta","text":"Checking"}));
    let tool = state.legacy_event_to_api(&json!({"type":"tool_start","id":"t","name":"read"}));
    assert!(matches!(
        tool.as_slice(),
        [ServerFrame {
            event: ApiEvent::ToolStart { .. },
            ..
        }]
    ));
    // A text content block after streamed tool arguments is still the same
    // assistant message. Only execution forces a fallback boundary.
    state.legacy_event_to_api(&json!({"type":"text_delta","text":" logs"}));
    let exec = state.legacy_event_to_api(&json!({"type":"tool_exec","id":"t","name":"read"}));
    assert!(matches!(
        exec.as_slice(),
        [
            ServerFrame {
                event: ApiEvent::TextDone { .. },
                ..
            },
            ServerFrame {
                event: ApiEvent::ToolExec { .. },
                ..
            }
        ]
    ));
    assert_eq!(
        state
            .legacy_event_to_api(&json!({"type":"tool_exec","id":"t2","name":"read"}))
            .len(),
        1
    );
    state.legacy_event_to_api(&json!({"type":"message_end"}));
    state.legacy_event_to_api(&json!({"type":"text_delta","text":"Answer"}));
    let end = state.legacy_event_to_api(&json!({"type":"done","id":0}));
    assert!(matches!(
        end.as_slice(),
        [
            ServerFrame {
                event: ApiEvent::TextDone { .. },
                ..
            },
            ServerFrame {
                event: ApiEvent::TurnDone { .. },
                ..
            }
        ]
    ));
    state.legacy_event_to_api(&json!({"type":"reasoning_delta","text":"thinking only"}));
    state.legacy_event_to_api(&json!({"type":"text_delta","text":""}));
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"message_end"}))
            .is_empty()
    );
    assert_eq!(
        state
            .legacy_event_to_api(&json!({"type":"done","id":0}))
            .len(),
        1
    );
}

#[test]
fn text_retry_retracts_completed_and_live_messages_and_late_replacements_keep_ids() {
    let mut state = state_with_session();
    state.legacy_event_to_api(&json!({"type":"text_delta","text":"first"}));
    state.legacy_event_to_api(&json!({"type":"text_done"}));
    state.legacy_event_to_api(&json!({"type":"text_delta","text":"second"}));
    let rollback = state.legacy_event_to_api(&json!({"type":"retry_rollback","attempt":2,"max":3}));
    assert_eq!(rollback.len(), 2);
    assert!(rollback.iter().all(|frame| matches!(&frame.event, ApiEvent::TextReplace { text, message_id: Some(_), .. } if text.is_empty())));
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"text_done"}))
            .is_empty()
    );
    let retry = state.legacy_event_to_api(&json!({"type":"text_delta","text":"valid <tool>"}));
    let ApiEvent::TextDelta { message_id, .. } = &retry[0].event else {
        panic!()
    };
    state.legacy_event_to_api(&json!({"type":"message_end"}));
    let corrected = state.legacy_event_to_api(&json!({"type":"text_replace","text":"valid"}));
    assert!(
        matches!(&corrected[0].event, ApiEvent::TextReplace { message_id: id, text, .. } if id == message_id && text == "valid")
    );
}

fn assert_panel(frames: &[ServerFrame], session: &str, snapshot: Value) {
    let panels: Vec<_> = frames
        .iter()
        .filter(|f| matches!(f.event, ApiEvent::SidePanelState { .. }))
        .collect();
    assert_eq!(panels.len(), 1);
    assert_eq!(
        panels[0],
        &ServerFrame::event(ApiEvent::SidePanelState {
            session_id: session.into(),
            snapshot: serde_json::from_value(snapshot).unwrap(),
        })
    );
}

fn markdown_panel() -> Value {
    json!({"focused_page_id":"notes","pages":[{"id":"notes","title":"Notes",
        "file_path":"/notes.md","format":"markdown","source":"managed",
        "content":"# Notes\n```mermaid\ngraph LR; A-->B\n```","updated_at_ms":42}]})
}
