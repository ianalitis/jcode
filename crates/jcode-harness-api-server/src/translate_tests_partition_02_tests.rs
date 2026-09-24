#[test]
fn route_availability_changes_are_broadcast_without_polling() {
    let mut state = state_with_session();
    for available in [true, false] {
        let frames = state.legacy_event_to_api(&json!({
            "type": "available_models_updated", "provider_model": "model",
            "available_models": ["model"],
            "available_model_routes": [{
                "model": "model", "provider": "provider", "api_method": "api",
                "available": available, "detail": "status"
            }]
        }));
        assert_eq!(frames[1].reply_to, None);
        assert!(
            matches!(&frames[1].event, ApiEvent::RuntimeInfo { routes, .. }
            if routes.len() == 1 && routes[0].available == available)
        );
    }
}

#[test]
fn reattaching_does_not_reuse_the_previous_sessions_catalog() {
    let mut state = state_with_session();
    state.legacy_event_to_api(&json!({
        "type": "available_models_updated", "provider_model": "old-model",
        "reasoning_effort": "high", "available_models": ["old-model"]
    }));
    let out = state.api_request_to_legacy(&json!({
        "req": "attach_session", "id": 83, "session_id": "s2"
    }));
    let Outbound::Legacy(request) = &out[1] else {
        panic!("state request");
    };
    state.legacy_event_to_api(&json!({
        "type": "state", "id": request["id"], "session_id": "s2"
    }));
    assert!(state.available_models.is_empty());
    assert!(state.current_model.is_none());
    assert!(state.current_effort.is_none());
    let out = state.api_request_to_legacy(&json!({
        "req": "get_runtime_info", "id": 84, "session_id": "s2"
    }));
    assert!(matches!(&out[0], Outbound::Legacy(request) if request["type"] == "get_model_catalog"));
}

#[test]
fn explicit_null_effort_clears_cached_identity() {
    let mut state = state_with_session();
    state.note_models(&json!({"reasoning_effort": "high"}));
    state.note_models(&json!({"reasoning_effort": null}));
    assert!(state.current_effort.is_none());
}

#[test]
fn switching_provider_does_not_reuse_the_previous_providers_effort() {
    for event in [
        json!({"type": "model_changed", "id": 99, "provider_name": "second", "model": "new"}),
        json!({"type": "available_models_updated", "provider_name": "second", "provider_model": "new"}),
    ] {
        let mut state = state_with_session();
        state.note_models(&json!({"provider_name": "first", "reasoning_effort": "high"}));
        let frames = state.legacy_event_to_api(&event);
        assert!(matches!(
            &frames[0].event,
            ApiEvent::ModelInfo {
                reasoning_effort: None,
                ..
            }
        ));
        assert!(state.current_effort.is_none());
    }
}

#[test]
fn model_change_without_provider_preserves_known_provider() {
    let mut state = state_with_session();
    state.note_models(&json!({"provider_name": "known", "reasoning_effort": "high"}));
    let frames =
        state.legacy_event_to_api(&json!({"type": "model_changed", "id": 99, "model": "new"}));
    assert!(
        matches!(&frames[0].event, ApiEvent::ModelInfo { provider, reasoning_effort, .. }
        if provider.as_deref() == Some("known") && reasoning_effort.as_deref() == Some("high"))
    );
}

#[test]
fn observer_and_server_initiated_turns_finish_without_a_local_message_id() {
    for id in [0, 999_999] {
        let mut state = state_with_session();
        state.legacy_event_to_api(&json!({"type":"text_delta", "text":"finished"}));
        let frames = state.legacy_event_to_api(&json!({"type":"done", "id":id}));
        assert!(
            matches!(&frames.last().unwrap().event, ApiEvent::TurnDone { session_id } if session_id == "s1")
        );
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"done", "id":id}))
                .is_empty()
        );
    }
}

#[test]
fn observer_turn_ignores_control_done_even_after_the_control_reply() {
    let mut state = state_with_session();
    let actions = state.api_request_to_legacy(&json!({"req":"clear", "id":22, "session_id":"s1"}));
    let Outbound::Legacy(control) = &actions[0] else {
        panic!()
    };
    state.legacy_event_to_api(&json!({"type":"ack", "id":control["id"]}));
    state.legacy_event_to_api(&json!({"type":"text_delta", "text":"still working"}));
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"done", "id":control["id"]}))
            .is_empty()
    );
    assert!(state.observed_turn_active);
    assert!(matches!(
        state
            .legacy_event_to_api(&json!({"type":"done", "id":0}))
            .last()
            .unwrap()
            .event,
        ApiEvent::TurnDone { .. }
    ));
}

#[test]
fn reconnect_activity_is_forwarded_and_busy_attach_can_finish_without_more_text() {
    for active in [false, true] {
        let mut state = BridgeState::default();
        let actions = state
            .api_request_to_legacy(&json!({"req":"attach_session", "id":22, "session_id":"s1"}));
        let Outbound::Legacy(probe) = &actions[1] else {
            panic!()
        };
        let frames = state.legacy_event_to_api(
            &json!({"type":"state", "id":probe["id"], "session_id":"s1", "is_processing":active}),
        );
        assert!(
            matches!(&frames[1].event, ApiEvent::SessionStatus { status, .. } if status == if active { "running" } else { "idle" })
        );
        let Outbound::Legacy(subscribe) = &actions[0] else {
            panic!()
        };
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"done", "id":subscribe["id"]}))
                .is_empty()
        );
        let done = state.legacy_event_to_api(&json!({"type":"done", "id":0}));
        assert_eq!(!done.is_empty(), active);
    }
}

#[test]
fn history_activity_is_forwarded_but_catalog_history_is_not_a_turn_boundary() {
    let mut state = state_with_session();
    for active in [true, false] {
        let actions =
            state.api_request_to_legacy(&json!({"req":"get_history", "id":22, "session_id":"s1"}));
        let Outbound::Legacy(probe) = &actions[0] else {
            panic!()
        };
        let frames = state.legacy_event_to_api(&json!({"type":"history", "id":probe["id"], "session_id":"s1", "messages":[], "activity":{"is_processing":active}}));
        assert!(
            matches!(&frames[1].event, ApiEvent::SessionStatus { status, .. } if status == if active { "running" } else { "idle" })
        );
    }
    assert!(
        state
            .legacy_event_to_api(
                &json!({"type":"history", "id":999999, "activity":{"is_processing":false}})
            )
            .is_empty()
    );
}

#[test]
fn delayed_history_activity_cannot_resurrect_or_stop_a_newer_turn() {
    for active in [true, false] {
        let mut state = state_with_session();
        state.legacy_event_to_api(&json!({"type":"text_delta", "text":"first"}));
        let actions =
            state.api_request_to_legacy(&json!({"req":"get_history", "id":22, "session_id":"s1"}));
        let Outbound::Legacy(probe) = &actions[0] else {
            panic!()
        };
        state.legacy_event_to_api(&json!({"type":"done", "id":0}));
        if !active {
            state.legacy_event_to_api(&json!({"type":"text_delta", "text":"next"}));
        }
        let frames = state.legacy_event_to_api(&json!({"type":"history", "id":probe["id"], "messages":[], "activity":{"is_processing":active}}));
        assert_eq!(frames.len(), 2, "history and panel only, no stale activity");
        assert_eq!(state.observed_turn_active, !active);
    }
}

#[test]
fn list_and_attach_expose_swarm_ownership_without_nesting_forks() {
    let home = ScopedJcodeHome::new("swarm-sidebar");
    // Keep the runtime override isolated as well, including under test runners
    // that already set JCODE_RUNTIME_DIR.
    let previous_runtime = std::env::var_os("JCODE_RUNTIME_DIR");
    struct RuntimeGuard(Option<OsString>);
    impl Drop for RuntimeGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => unsafe { std::env::set_var("JCODE_RUNTIME_DIR", value) },
                None => unsafe { std::env::remove_var("JCODE_RUNTIME_DIR") },
            }
        }
    }
    let _runtime = RuntimeGuard(previous_runtime);
    let runtime = home.path.join("runtime");
    unsafe { std::env::set_var("JCODE_RUNTIME_DIR", &runtime) };
    let swarm_dir = runtime.join("durable-state/swarm");
    std::fs::create_dir_all(&swarm_dir).unwrap();
    let snapshot_path = swarm_dir.join("swarm.json");
    let snapshot = |status: &str| {
        json!({"updated_at_unix_ms": 1, "members": [
            {"session_id": "child", "report_back_to_session_id": "root", "task_label": "API reviewer", "status": status}
        ]})
    };
    std::fs::write(&snapshot_path, snapshot("running").to_string()).unwrap();
    for id in ["root", "child", "fork"] {
        write_session_record_with_titles(
            &home.path,
            id,
            &home.path,
            Some("Generated"),
            Some("Custom"),
        );
    }
    let fork_path = home.path.join("sessions/fork.json");
    let mut fork: Value = serde_json::from_slice(&std::fs::read(&fork_path).unwrap()).unwrap();
    fork["parent_id"] = json!("root");
    std::fs::write(fork_path, fork.to_string()).unwrap();
    let mut state = BridgeState::default();
    let list = |state: &mut BridgeState| {
        let ApiEvent::Sessions { sessions } = only_reply_event(
            state.api_request_to_legacy(&json!({"req": "list_sessions", "id": 1})),
        ) else {
            panic!("expected sessions")
        };
        sessions
    };
    let sessions = list(&mut state);
    let child = sessions.iter().find(|s| s.session_id == "child").unwrap();
    assert_eq!(child.parent_session_id.as_deref(), Some("root"));
    assert_eq!(child.agent_label.as_deref(), Some("API reviewer"));
    assert_eq!(child.swarm_status.as_deref(), Some("running"));
    assert_eq!(child.title.as_deref(), Some("Custom"));
    assert!(
        sessions
            .iter()
            .filter(|s| s.session_id != "child")
            .all(|s| s.parent_session_id.is_none() && s.swarm_status.is_none())
    );
    std::fs::write(&snapshot_path, snapshot("completed").to_string()).unwrap();
    let sessions = list(&mut state);
    assert_eq!(
        sessions
            .iter()
            .find(|s| s.session_id == "child")
            .unwrap()
            .swarm_status
            .as_deref(),
        Some("completed")
    );

    let out = state
        .api_request_to_legacy(&json!({"req": "attach_session", "id": 2, "session_id": "child"}));
    let Outbound::Legacy(probe) = &out[1] else {
        panic!("expected state probe")
    };
    let frames = state.legacy_event_to_api(
        &json!({"type": "state", "id": probe["id"], "session_id": "child", "is_processing": false}),
    );
    let ApiEvent::Attached { session } = &frames[0].event else {
        panic!("expected attach")
    };
    assert_eq!(session.parent_session_id.as_deref(), Some("root"));
    assert_eq!(session.agent_label.as_deref(), Some("API reviewer"));
    assert_eq!(session.swarm_status.as_deref(), Some("completed"));

    std::fs::remove_file(snapshot_path).unwrap();
    assert!(
        list(&mut state)
            .iter()
            .all(|s| s.parent_session_id.is_none() && s.swarm_status.is_none())
    );
}

#[test]
fn history_image_boundaries_survive_loss_of_tool_data() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req": "get_history", "id": 4}));
    let Outbound::Legacy(get) = &out[0] else {
        panic!("expected legacy request")
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "history", "id": get["id"], "session_id": "s1",
        "messages": [
            {"role": "assistant", "content": "before"},
            {"role": "tool", "content": "read image", "tool_data": {"id": "read-1", "name": "read"}},
            {"role": "assistant", "content": "after"}
        ],
        "images": [{"media_type": "image/png", "data": "bytes", "label": null,
            "source": {"kind": "tool_result", "tool_name": "read"},
            "anchor": {"kind": "tool_call", "id": "read-1"}, "history_message_index": 2}]
    }));
    let ApiEvent::History {
        messages, images, ..
    } = &frames[0].event
    else {
        panic!("expected history")
    };
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1].role, "tool");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].history_message_index, Some(2));
    assert_eq!(
        messages[images[0].history_message_index.unwrap()].content,
        "after"
    );
}

#[test]
fn history_response_stats_roundtrip_and_active_turn_suppression() {
    for active in [false, true] {
        let mut state = state_with_session();
        let out = state.api_request_to_legacy(&json!({"req":"get_history", "id":44}));
        let Outbound::Legacy(request) = &out[0] else {
            panic!("expected history request")
        };
        let stats = json!({"input_tokens":30,"output_tokens":4,"cache_read_tokens":6,"cache_creation_tokens":8});
        let frames = state.legacy_event_to_api(&json!({
            "type":"history", "id":request["id"], "session_id":"s1",
            "messages":[
                {"role":"user","content":"earlier"},
                {"role":"assistant","content":"earlier answer","response_stats":stats},
                {"role":"user","content":"current"},
                {"role":"assistant","content":"current answer","response_stats":stats}
            ], "activity":{"is_processing":active}
        }));
        let ApiEvent::History { messages, .. } = &frames[0].event else {
            panic!("expected history")
        };
        assert!(messages[0].response_stats.is_none());
        let previous = messages[1].response_stats.as_ref().unwrap();
        assert_eq!(previous.input_tokens, Some(30));
        assert_eq!(previous.output_tokens, Some(4));
        assert_eq!(previous.cache_read_tokens, Some(6));
        assert_eq!(previous.cache_creation_tokens, Some(8));
        assert_eq!(previous.duration_secs, None);
        assert_eq!(messages[3].response_stats.is_some(), !active);
    }
}

#[test]
fn history_response_stats_old_and_malformed_fields_are_optional() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req":"get_history", "id":45}));
    let Outbound::Legacy(request) = &out[0] else {
        panic!("expected history request")
    };
    let frames =
        state.legacy_event_to_api(&json!({"type":"history", "id":request["id"], "messages":[
            {"role":"assistant","content":"old"},
            {"role":"assistant","content":"bad","response_stats":{"input_tokens":"oops"}}
        ]}));
    let ApiEvent::History { messages, .. } = &frames[0].event else {
        panic!("expected history")
    };
    assert!(
        messages
            .iter()
            .all(|message| message.response_stats.is_none())
    );
}

#[test]
fn history_response_stats_cross_real_render_protocol_and_sdk_boundary() {
    let mut session = jcode_base::session::Session::create(None, None);
    session.messages = serde_json::from_value(json!([
        {"id":"u","role":"user","content":[{"type":"text","text":"question"}]},
        {"id":"a","role":"assistant","content":[{"type":"text","text":"answer"}],
            "token_usage":{"input_tokens":123,"output_tokens":45,"cache_read_input_tokens":7,"cache_creation_input_tokens":8}}
    ])).unwrap();
    let legacy: Vec<_> = jcode_base::session::render_messages(&session)
        .into_iter()
        .map(|row| jcode_base::protocol::HistoryMessage {
            role: row.role,
            content: row.content,
            tool_calls: None,
            tool_data: row.tool_data,
            response_stats: row.response_stats,
        })
        .collect();
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req":"get_history", "id":46}));
    let Outbound::Legacy(request) = &out[0] else {
        panic!("expected history request")
    };
    let frames = state.legacy_event_to_api(&json!({"type":"history", "id":request["id"],
        "messages":legacy,"activity":{"is_processing":false}}));
    let ApiEvent::History { messages, .. } = &frames[0].event else {
        panic!("expected history")
    };
    let stats = messages[1].response_stats.as_ref().unwrap();
    assert_eq!(stats.input_tokens, Some(123));
    assert_eq!(stats.output_tokens, Some(45));
    assert_eq!(stats.cache_read_tokens, Some(7));
    assert_eq!(stats.cache_creation_tokens, Some(8));
    assert_eq!(stats.duration_secs, None);
}

fn recovery_attach(state: &mut BridgeState, target: Option<&str>) -> (Value, Value) {
    let request = match target {
        Some(target) => json!({"req":"attach_session", "id":71, "session_id":target}),
        None => json!({"req":"create_session", "id":71}),
    };
    let out = state.api_request_to_legacy(&request);
    let Outbound::Legacy(subscribe) = &out[0] else {
        panic!("subscribe")
    };
    let Outbound::Legacy(snapshot) = &out[1] else {
        panic!("state")
    };
    (
        json!({"type":"history", "id":subscribe["id"], "session_id":"recover",
            "messages":[{"role":"user", "content":"finish task"}],
            "activity":{"is_processing":false}, "was_interrupted":true,
            "reload_recovery":{"continuation_message":"Continue the exact task", "reconnect_notice":"Recovered build"}}),
        json!({"type":"state", "id":snapshot["id"], "session_id":"recover", "is_processing":false}),
    )
}

#[test]
fn attachment_recovery_preserves_directive_in_both_history_state_orders() {
    for history_first in [true, false] {
        for target in [None, Some("recover")] {
            let mut state = BridgeState::default();
            let (history, snapshot) = recovery_attach(&mut state, target);
            if !history_first {
                let frames = state.legacy_event_to_api(&snapshot);
                assert!(matches!(frames[0].event, ApiEvent::Attached { .. }));
            }
            let frames = state.legacy_event_to_api(&history);
            assert_eq!(frames.len(), 2);
            assert_eq!(
                frames[0],
                ServerFrame::event(ApiEvent::SessionRecovery {
                    session_id: "recover".into(),
                    continuation_message: "Continue the exact task".into(),
                    reconnect_notice: Some("Recovered build".into()),
                })
            );
            if history_first {
                assert!(
                    state.session_id.is_none(),
                    "history must not establish attachment identity"
                );
                let frames = state.legacy_event_to_api(&snapshot);
                assert!(matches!(frames[0].event, ApiEvent::Attached { .. }));
            }
            assert!(
                state.legacy_event_to_api(&history).is_empty(),
                "duplicate attach history"
            );
            let out = state.api_request_to_legacy(&json!({"req":"get_history", "id":72}));
            let Outbound::Legacy(refresh) = &out[0] else {
                panic!("get_history")
            };
            let mut refreshed = history.clone();
            refreshed["id"] = refresh["id"].clone();
            let frames = state.legacy_event_to_api(&refreshed);
            assert!(matches!(frames[0].event, ApiEvent::History { .. }));
            assert!(
                !frames
                    .iter()
                    .any(|f| matches!(f.event, ApiEvent::SessionRecovery { .. }))
            );
            // Each new attachment gets its own single opportunity.
            let (history, _) = recovery_attach(&mut state, target);
            assert_eq!(state.legacy_event_to_api(&history).len(), 2);
        }
    }
}

#[test]
fn attachment_recovery_ignores_wrong_session_and_request_without_consuming_intent() {
    let mut state = BridgeState {
        session_id: Some("previous".into()),
        ..Default::default()
    };
    let (history, _) = recovery_attach(&mut state, Some("recover"));
    let mut unrelated = history.clone();
    unrelated["session_id"] = json!("other");
    assert!(state.legacy_event_to_api(&unrelated).is_empty());
    unrelated = history.clone();
    unrelated["id"] = json!(u64::MAX);
    assert!(state.legacy_event_to_api(&unrelated).is_empty());
    let frames = state.legacy_event_to_api(&history);
    assert!(
        matches!(&frames[0].event, ApiEvent::SessionRecovery {session_id, ..} if session_id == "recover")
    );
}

#[test]
fn attachment_recovery_suppresses_empty_active_completed_and_blank_directives_once() {
    for case in ["empty", "active", "completed", "blank"] {
        let mut state = BridgeState::default();
        let (mut history, _) = recovery_attach(&mut state, Some("recover"));
        let recoverable = history.clone();
        match case {
            "empty" => history["messages"] = json!([]),
            "active" => history["activity"]["is_processing"] = json!(true),
            "completed" => {
                history["was_interrupted"] = json!(false);
                history["reload_recovery"] = Value::Null;
            }
            "blank" => history["reload_recovery"]["continuation_message"] = json!("  "),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                state.legacy_event_to_api(&history).as_slice(),
                [ServerFrame {
                    event: ApiEvent::SidePanelState { .. },
                    ..
                }]
            ),
            "{case}"
        );
        assert!(
            state.legacy_event_to_api(&recoverable).is_empty(),
            "{case} duplicate"
        );
    }
}

#[test]
fn attachment_recovery_supports_interrupted_legacy_history_and_server_directive_priority() {
    for interrupted in [true, false] {
        let mut state = BridgeState::default();
        let (mut history, _) = recovery_attach(&mut state, Some("recover"));
        history["was_interrupted"] = json!(interrupted);
        if interrupted {
            history["reload_recovery"] = Value::Null;
        }
        let frames = state.legacy_event_to_api(&history);
        let ApiEvent::SessionRecovery {
            continuation_message,
            reconnect_notice,
            ..
        } = &frames[0].event
        else {
            panic!("recovery")
        };
        if interrupted {
            assert_eq!(
                continuation_message,
                "Your session was interrupted by a server reload while a tool was running. The tool was aborted and results may be incomplete. Continue exactly where you left off and do not ask the user what to do next."
            );
            assert_eq!(reconnect_notice, &None);
        } else {
            assert_eq!(continuation_message, "Continue the exact task");
            assert_eq!(reconnect_notice.as_deref(), Some("Recovered build"));
        }
    }
}

#[test]
fn attachment_recovery_is_cleared_on_attach_failure() {
    let mut state = BridgeState::default();
    let (history, _) = recovery_attach(&mut state, Some("recover"));
    let frames = state
        .legacy_event_to_api(&json!({"type":"error", "id":history["id"], "message":"missing"}));
    assert!(matches!(frames[0].event, ApiEvent::Error { .. }));
    assert!(state.legacy_event_to_api(&history).is_empty());
}

#[test]
fn hidden_system_reminder_is_forwarded_without_visible_content_or_no_reply() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req":"send_message", "id":91,
        "session_id":"s1", "content":"", "system_reminder":"continue task"}));
    assert_eq!(out.len(), 1);
    let Outbound::Legacy(message) = &out[0] else {
        panic!("message")
    };
    assert_eq!(message["type"], "message");
    assert_eq!(message["content"], "");
    assert_eq!(message["system_reminder"], "continue task");
    assert!(message.get("no_reply").is_none());
    let frames = state.legacy_event_to_api(&json!({"type":"done", "id":message["id"]}));
    assert!(matches!(&frames[0].event, ApiEvent::TurnDone {session_id} if session_id == "s1"));
    let out = state.api_request_to_legacy(&json!({"req":"send_message", "id":92,
        "session_id":"s1", "content":"normal user message"}));
    let Outbound::Legacy(message) = &out[0] else {
        panic!("message")
    };
    assert!(message.get("system_reminder").is_none());
}

/// The only action, when it is a reply. `Outbound::Reply` is boxed in this
/// fork, and patterns cannot destructure through a `Box`.
fn single_reply(actions: &[Outbound]) -> Option<&ServerFrame> {
    match actions {
        [Outbound::Reply(frame)] => Some(frame),
        _ => None,
    }
}

fn tool_state() -> BridgeState {
    BridgeState {
        session_id: Some("s1".into()),
        session_tools_supported: true,
        ..BridgeState::default()
    }
}

#[test]
fn session_tool_controls_translate_and_ack_without_ending_turn() {
    for mut request in [
        json!({"req":"configure_tools","tools":{}}),
        json!({"req":"configure_tools","tools":{"enabled":null}}),
        json!({"req":"configure_tools","tools":{"enabled":[],"disabled":["bash"],"custom":[{"name":"lookup","description":"Lookup","parameters":{"type":"object"}}]}}),
        json!({"req":"tool_result","call_id":"c1","output":"ok"}),
        json!({"req":"tool_result","call_id":"c1","output":"","error":"failed"}),
    ] {
        let mut state = tool_state();
        state.observed_turn_active = true;
        request["id"] = json!(7);
        request["session_id"] = json!("s1");
        let actions = state.api_request_to_legacy(&request);
        let [Outbound::Legacy(legacy)] = actions.as_slice() else {
            panic!("{actions:?}")
        };
        let mut expected = request.clone();
        expected.as_object_mut().unwrap().remove("session_id");
        expected.as_object_mut().unwrap().remove("req");
        expected["type"] = request["req"].clone();
        expected["id"] = legacy["id"].clone();
        assert_eq!(legacy, &expected);
        assert_eq!(
            state.legacy_event_to_api(&json!({"type":"ack","id":legacy["id"]})),
            vec![ServerFrame::reply(7, ApiEvent::Ok)]
        );
        // Tool controls have no legacy Done, so retain no completion IDs.
        assert!(state.pending_control_done_ids.is_empty());
        assert!(state.pending_simple.is_empty());
        assert!(state.observed_turn_active);
    }
}

#[test]
fn session_tool_inventory_correlates_and_validates_daemon_response() {
    let mut state = tool_state();
    for tools in [
        json!([]),
        json!([{"name":"lookup","description":"Lookup","parameters":{}}]),
        json!([{"name":"bad","description":"Bad","parameters":[]}]),
    ] {
        let actions =
            state.api_request_to_legacy(&json!({"id":8,"req":"list_tools","session_id":"s1"}));
        let [Outbound::Legacy(legacy)] = actions.as_slice() else {
            panic!()
        };
        assert_eq!(legacy["type"], "list_tools");
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"ack","id":legacy["id"]}))
                .is_empty()
        );
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"tools","id":u64::MAX,"tools":tools}))
                .is_empty()
        );
        let result =
            state.legacy_event_to_api(&json!({"type":"tools","id":legacy["id"],"tools":tools}));
        assert_eq!(result[0].reply_to, Some(8));
        if tools.get(0).is_some_and(|t| t["name"] == "bad") {
            assert!(matches!(
                result[0].event,
                ApiEvent::Error {
                    code: ErrorCode::Internal,
                    ..
                }
            ));
        } else {
            assert_eq!(
                serde_json::to_value(&result[0]).unwrap(),
                json!({"v":1,"reply_to":8,"ev":"tools","session_id":"s1","tools":tools})
            );
        }
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"tools","id":legacy["id"],"tools":tools}))
                .is_empty()
        );
    }
}

#[test]
fn custom_tool_call_is_distinct_from_streaming_lifecycle() {
    let mut state = tool_state();
    let frames = state.legacy_event_to_api(
        &json!({"type":"tool_call","session_id":"s1","call_id":"c1","name":"lookup","input":{"x":1}}),
    );
    assert_eq!(
        serde_json::to_value(&frames[0]).unwrap(),
        json!({"v":1,"ev":"tool_call","session_id":"s1","call_id":"c1","name":"lookup","input":{"x":1}})
    );
    assert!(
        state
            .legacy_event_to_api(
                &json!({"type":"tool_call","session_id":"s1","call_id":"c1","name":"lookup"})
            )
            .is_empty()
    );
    for session_id in [json!("other"), Value::Null] {
        assert!(state.legacy_event_to_api(&json!({"type":"tool_call","session_id":session_id,"call_id":"c1","name":"lookup","input":{}})).is_empty());
    }
    assert!(
        state
            .legacy_event_to_api(
                &json!({"type":"tool_call","call_id":"c1","name":"lookup","input":{}})
            )
            .is_empty()
    );
    state.session_id = None;
    assert!(
        state
            .legacy_event_to_api(
                &json!({"type":"tool_call","session_id":"s1","call_id":"c1","name":"lookup","input":null})
            )
            .is_empty()
    );
}

#[test]
fn session_tool_controls_require_attachment_and_negotiated_support() {
    for mut request in [
        json!({"req":"configure_tools","tools":{}}),
        json!({"req":"list_tools"}),
        json!({"req":"tool_result","call_id":"c1","output":"ok"}),
    ] {
        request["id"] = json!(9);
        request["session_id"] = json!("s1");
        for (mut state, code) in [
            (BridgeState::default(), ErrorCode::UnknownSession),
            (
                BridgeState {
                    session_id: Some("other".into()),
                    ..tool_state()
                },
                ErrorCode::UnknownSession,
            ),
            (
                BridgeState {
                    session_tools_supported: false,
                    ..tool_state()
                },
                ErrorCode::UnknownRequest,
            ),
        ] {
            let actions = state.api_request_to_legacy(&request);
            assert!(
                matches!(single_reply(&actions), Some(ServerFrame { reply_to: Some(9), event: ApiEvent::Error { code: actual, .. }, .. }) if actual == &code)
            );
            assert!(state.pending_simple.is_empty());
        }
        let mut state = tool_state();
        state.observed_turn_active = true;
        let actions = state.api_request_to_legacy(&request);
        let [Outbound::Legacy(legacy)] = actions.as_slice() else {
            panic!()
        };
        let result = state.legacy_event_to_api(
            &json!({"type":"error","id":legacy["id"],"message":"invalid tool"}),
        );
        assert_eq!(
            result,
            vec![ServerFrame::reply(
                9,
                ApiEvent::Error {
                    code: ErrorCode::Internal,
                    message: "invalid tool".into()
                }
            )]
        );
        assert!(state.observed_turn_active);
    }
}

#[test]
fn malformed_tool_controls_never_reach_daemon() {
    let mut state = tool_state();
    for mut request in [
        json!({"req":"configure_tools","tools":{"enabled":false}}),
        json!({"req":"configure_tools","tools":{"custom":[{"name":"x","description":"x","parameters":[]}]}}),
        json!({"req":"configure_tools"}),
        json!({"req":"tool_result","call_id":"x","output":{}}),
        json!({"req":"tool_result","call_id":"x","output":"","error":false}),
    ] {
        request["id"] = json!(10);
        request["session_id"] = json!("s1");
        let actions = state.api_request_to_legacy(&request);
        assert!(matches!(
            single_reply(&actions),
            Some(ServerFrame {
                reply_to: Some(10),
                event: ApiEvent::Error {
                    code: ErrorCode::InvalidRequest,
                    ..
                },
                ..
            })
        ));
    }
}

#[test]
fn tool_controls_validate_pending_attachment_and_required_session() {
    let mut state = BridgeState {
        session_tools_supported: true,
        ..BridgeState::default()
    };
    state.api_request_to_legacy(&json!({"id":1,"req":"attach_session","session_id":"s1"}));
    assert!(matches!(
        state
            .api_request_to_legacy(&json!({"id":2,"req":"list_tools","session_id":"s1"}))
            .as_slice(),
        [Outbound::Legacy(_)]
    ));
    for (session_id, code) in [
        (json!("s2"), ErrorCode::UnknownSession),
        (json!(""), ErrorCode::InvalidRequest),
        (Value::Null, ErrorCode::InvalidRequest),
    ] {
        let actions = state
            .api_request_to_legacy(&json!({"id":3,"req":"list_tools","session_id":session_id}));
        assert!(
            matches!(single_reply(&actions), Some(ServerFrame { event: ApiEvent::Error { code: actual, .. }, .. }) if actual == &code)
        );
    }
}

#[test]
fn detach_waits_for_release_barrier_and_drops_late_callbacks() {
    let mut state = tool_state();
    state.observed_turn_active = true;
    let actions =
        state.api_request_to_legacy(&json!({"id":11,"req":"detach_session","session_id":"s1"}));
    let [Outbound::Legacy(legacy)] = actions.as_slice() else {
        panic!("detach must not acknowledge locally")
    };
    assert!(
        state
            .legacy_event_to_api(&json!({"type":"ack","id":legacy["id"]}))
            .is_empty()
    );
    assert_eq!(state.session_id.as_deref(), Some("s1"));
    assert_eq!(
        state.legacy_event_to_api(&json!({"type":"done","id":legacy["id"]})),
        vec![ServerFrame::reply(11, ApiEvent::Ok)]
    );
    assert!(state.session_id.is_none());
    assert!(state.pending_simple.is_empty());
    assert!(state.pending_control_done_ids.is_empty());
    assert!(state.legacy_event_to_api(&json!({"type":"tool_call","session_id":"s1","call_id":"c1","name":"lookup","input":{}})).is_empty());
    let actions =
        state.api_request_to_legacy(&json!({"id":12,"req":"list_tools","session_id":"s1"}));
    assert!(matches!(
        single_reply(&actions),
        Some(ServerFrame {
            event: ApiEvent::Error {
                code: ErrorCode::UnknownSession,
                ..
            },
            ..
        })
    ));
}

#[test]
fn abnormal_stop_precedes_error_and_done_and_preserves_specific_reason() {
    use jcode_harness_api::TurnStopReason;
    for (reason, expected) in [
        ("failure", TurnStopReason::Failure),
        ("crash", TurnStopReason::Crash),
        ("limit_reached", TurnStopReason::LimitReached),
    ] {
        let mut state = state_with_session();
        let stopped = state.legacy_event_to_api(
            &json!({"type":"turn_stopped", "reason":reason, "message":"Specific explanation"}),
        );
        assert!(
            matches!(&stopped[0].event, ApiEvent::TurnStopped { session_id, reason, message, .. } if session_id == "s1" && *reason == expected && message == "Specific explanation")
        );
        assert!(
            state
                .legacy_event_to_api(
                    &json!({"type":"turn_stopped","reason":"failure","message":"generic wrapper"})
                )
                .is_empty()
        );
        let end = state.legacy_event_to_api(&json!({"type":"error","id":0,"message":"failed"}));
        assert!(matches!(&end[0].event, ApiEvent::Error { .. }));
        assert!(matches!(
            &end.last().unwrap().event,
            ApiEvent::TurnDone { .. }
        ));
        assert!(
            state
                .legacy_event_to_api(&json!({"type":"done","id":0}))
                .is_empty()
        );
        state.legacy_event_to_api(&json!({"type":"text_delta","text":"next turn"}));
        let natural = state.legacy_event_to_api(&json!({"type":"done","id":0}));
        assert!(
            natural
                .iter()
                .all(|f| !matches!(f.event, ApiEvent::TurnStopped { .. }))
        );
    }
}

#[test]
fn guardrail_stop_preserves_provider_reason_and_finishes_observer_turn() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(
        &json!({"type":"provider_guardrail","stop_reason":"refusal","message":"Provider refused"}),
    );
    assert!(
        matches!(&frames[0].event, ApiEvent::TurnStopped { reason: jcode_harness_api::TurnStopReason::ProviderGuardrail, provider_stop_reason: Some(reason), .. } if reason == "refusal")
    );
    assert!(matches!(
        &state.legacy_event_to_api(&json!({"type":"done","id":0}))[0].event,
        ApiEvent::TurnDone { .. }
    ));
}

#[test]
fn legacy_interrupt_finishes_observer_turn_but_idle_cancel_is_not_a_stop() {
    let mut state = state_with_session();
    let idle = state.legacy_event_to_api(&json!({"type":"interrupted"}));
    assert!(
        idle.iter()
            .all(|f| !matches!(f.event, ApiEvent::TurnStopped { .. }))
    );
    state.legacy_event_to_api(&json!({"type":"text_delta","text":"partial"}));
    let stopped = state.legacy_event_to_api(&json!({"type":"interrupted"}));
    assert!(matches!(
        &stopped[0].event,
        ApiEvent::TurnStopped {
            reason: jcode_harness_api::TurnStopReason::Interrupted,
            ..
        }
    ));
    let done = state.legacy_event_to_api(&json!({"type":"done","id":0}));
    assert!(matches!(
        &done.last().unwrap().event,
        ApiEvent::TurnDone { .. }
    ));
    let late = state.legacy_event_to_api(&json!({"type":"interrupted"}));
    assert!(
        late.iter()
            .all(|f| !matches!(f.event, ApiEvent::TurnStopped { .. }))
    );
}

#[test]
fn request_error_does_not_fabricate_a_turn_stop() {
    let mut state = state_with_session();
    let frames =
        state.legacy_event_to_api(&json!({"type":"error","id":777,"message":"request failed"}));
    assert!(matches!(
        frames.as_slice(),
        [ServerFrame {
            event: ApiEvent::Error { .. },
            ..
        }]
    ));
}

#[test]
fn tool_streaming_forwards_zero_argument_starts_and_keyed_interleaved_input() {
    let mut state = state_with_session();
    for id in ["a", "b"] {
        let frames = state.legacy_event_to_api(&json!({"type":"tool_start","id":id,"name":"bash"}));
        assert!(matches!(frames.as_slice(), [ServerFrame {
            event: ApiEvent::ToolStart { call_id, name, .. }, ..
        }] if call_id == id && name == "bash"));
    }
    for (id, delta) in [
        ("b", "{\"command\":"),
        ("a", "{\"command\":\"echo a\"}"),
        ("b", "\"echo b\"}"),
    ] {
        let frames = state.legacy_event_to_api(&json!({"type":"tool_input","id":id,"delta":delta}));
        assert!(matches!(frames.as_slice(), [ServerFrame {
            event: ApiEvent::ToolInputDelta { call_id, delta: actual, .. }, ..
        }] if call_id == id && actual == delta));
    }
    let legacy = state.legacy_event_to_api(&json!({"type":"tool_input","delta":"{}"}));
    assert!(matches!(legacy.as_slice(), [ServerFrame {
        event: ApiEvent::ToolInputDelta { call_id, delta, .. }, ..
    }] if call_id.is_empty() && delta == "{}"));
}

#[test]
fn pre_stream_activity_invalidates_older_idle_history() {
    for phase in [
        None,
        Some("authenticating"),
        Some("connecting"),
        Some("sending request"),
        Some("waiting for response"),
        Some("streaming"),
        Some("retrying (2/4)"),
    ] {
        let mut state = state_with_session();
        let history =
            state.api_request_to_legacy(&json!({"req":"get_history", "id":22, "session_id":"s1"}));
        let Outbound::Legacy(probe) = &history[0] else {
            panic!()
        };
        match phase {
            Some(phase) => {
                state.legacy_event_to_api(&json!({"type":"connection_phase", "phase":phase}));
            }
            None => {
                let send = state.api_request_to_legacy(
                    &json!({"req":"send_message", "id":23, "session_id":"s1", "content":"hi"}),
                );
                let Outbound::Legacy(message) = &send[0] else {
                    panic!()
                };
                state.legacy_event_to_api(&json!({"type":"ack", "id":message["id"]}));
            }
        }
        let frames = state.legacy_event_to_api(&json!({
            "type":"history", "id":probe["id"], "session_id":"s1",
            "messages":[], "activity":{"is_processing":false}
        }));
        assert!(
            frames
                .iter()
                .all(|frame| !matches!(frame.event, ApiEvent::SessionStatus { .. })),
            "{phase:?}"
        );
        assert!(state.observed_turn_active, "{phase:?}");
        // Observer turns can end before any text, including provider retries.
        let done = state.legacy_event_to_api(&json!({"type":"done", "id":0}));
        assert!(
            matches!(
                done.last().map(|frame| &frame.event),
                Some(ApiEvent::TurnDone { .. })
            ),
            "{phase:?}"
        );
        assert!(!state.observed_turn_active);
    }
}

#[test]
fn transport_bookkeeping_does_not_start_a_turn() {
    let mut state = state_with_session();
    for phase in ["connected", "", "unknown"] {
        state.legacy_event_to_api(&json!({"type":"connection_phase", "phase":phase}));
        assert!(!state.observed_turn_active);
        assert_eq!(state.activity_version, 0);
    }
    state.legacy_event_to_api(&json!({"type":"ack", "id":999999}));
    assert!(!state.observed_turn_active);
    assert_eq!(state.activity_version, 0);
}

#[test]
fn session_list_recovers_save_label_missing_from_older_index_rows() {
    let home = ScopedJcodeHome::new("save-label-fallback");
    let sessions_dir = home.path.join("sessions");
    std::fs::create_dir_all(&sessions_dir).unwrap();
    std::fs::write(
        sessions_dir.join("session_labelled.json"),
        json!({
            "title": "Fundraising catch-up",
            "messages": [{"role": "user", "content": "hello"}],
            "saved": true,
            "save_label": "investor catch up work",
        })
        .to_string(),
    )
    .unwrap();
    assert!(BridgeState::recent_session_index_entries().is_empty());
    let connection = Connection::open(home.path.join("session-metadata-v1.sqlite3")).unwrap();
    connection
        .execute(
            "INSERT INTO recent_sessions (session_id, generated_title, saved, updated_at_ms)
             VALUES ('session_labelled', 'Fundraising catch-up', 1, 5)",
            [],
        )
        .unwrap();

    let event = only_reply_event(
        BridgeState::default().api_request_to_legacy(&json!({"req": "list_sessions", "id": 1})),
    );
    let ApiEvent::Sessions { sessions } = event else {
        panic!("expected sessions reply, got {event:?}");
    };
    let labelled = sessions
        .iter()
        .find(|session| session.session_id == "session_labelled")
        .expect("labelled session listed");
    assert!(labelled.saved);
    assert_eq!(
        labelled.save_label.as_deref(),
        Some("investor catch up work")
    );
}

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
