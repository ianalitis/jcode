use super::*;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

#[test]
fn token_usage_preserves_cache_creation_and_missing_counters() {
    let mut state = BridgeState {
        session_id: Some("s1".into()),
        ..Default::default()
    };
    for cache_creation_input in [None, Some(0), Some(42)] {
        let mut legacy = json!({
            "type": "tokens", "input": 10, "output": 5, "cache_read_input": 2
        });
        if let Some(tokens) = cache_creation_input {
            legacy["cache_creation_input"] = json!(tokens);
        }
        let frames = state.legacy_event_to_api(&legacy);
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].event,
            ApiEvent::TokenUsage {
                session_id: "s1".into(),
                input: 10,
                output: 5,
                cache_read_input: Some(2),
                cache_creation_input,
            }
        );
    }
}

struct ScopedJcodeHome {
    path: PathBuf,
    previous: Option<OsString>,
    _guard: MutexGuard<'static, ()>,
}

impl ScopedJcodeHome {
    fn new(label: &str) -> Self {
        let guard = jcode_home_test_lock();
        let previous = std::env::var_os("JCODE_HOME");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "jcode-harness-api-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create isolated JCODE_HOME");
        // SAFETY: all tests in this module that mutate JCODE_HOME share `LOCK`,
        // and this guard restores the prior value before it is released.
        unsafe { std::env::set_var("JCODE_HOME", &path) };
        Self {
            path,
            previous,
            _guard: guard,
        }
    }
}

impl Drop for ScopedJcodeHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => unsafe { std::env::set_var("JCODE_HOME", value) },
            None => unsafe { std::env::remove_var("JCODE_HOME") },
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write_session_record(home: &Path, session_id: &str, working_dir: &Path) -> PathBuf {
    write_session_record_with_titles(home, session_id, working_dir, None, None)
}

fn write_session_record_with_titles(
    home: &Path,
    session_id: &str,
    working_dir: &Path,
    title: Option<&str>,
    custom_title: Option<&str>,
) -> PathBuf {
    let sessions = home.join("sessions");
    std::fs::create_dir_all(&sessions).expect("create sessions directory");
    let path = sessions.join(format!("{session_id}.json"));
    std::fs::write(
        &path,
        json!({
            "working_dir": working_dir,
            "title": title,
            "custom_title": custom_title,
            "messages": [{"role": "user", "content": "hello"}],
        })
        .to_string(),
    )
    .expect("write session record");
    path
}

fn only_reply_event(outbound: Vec<Outbound>) -> ApiEvent {
    assert_eq!(outbound.len(), 1, "expected exactly one reply");
    match outbound.into_iter().next().expect("one outbound") {
        Outbound::Reply(frame) => frame.event,
        other => panic!("expected API reply, got {other:?}"),
    }
}

fn state_with_session() -> BridgeState {
    BridgeState {
        session_id: Some("s1".into()),
        ..Default::default()
    }
}

#[test]
fn connection_phase_is_forwarded_to_api_clients() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "connection_phase",
        "phase": "sending request",
    }));

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].reply_to, None);
    assert_eq!(
        frames[0].event,
        ApiEvent::ConnectionPhase {
            session_id: "s1".into(),
            phase: "sending request".into(),
        }
    );
}

#[test]
fn wake_request_is_forwarded_with_explicit_session_and_payload() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "wake_requested",
        "session_id": "target",
        "reason": "background_task_completed",
        "notification": "finished",
    }));
    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0].event,
        ApiEvent::WakeRequested {
            session_id: "target".into(),
            reason: "background_task_completed".into(),
            notification: "finished".into(),
        }
    );
}

#[test]
fn create_session_maps_to_subscribe() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req": "create_session", "id": 1}));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(value["type"], "subscribe");
    assert!(value["working_dir"].is_string());
    assert!(value.get("system_prompt").is_none());
}

#[test]
fn create_session_forwards_full_system_prompt_including_empty() {
    for prompt in ["You are a helpful tutor.\nAnswer briefly.", ""] {
        let mut state = BridgeState::default();
        let out = state.api_request_to_legacy(&json!({
            "req": "create_session", "id": 1, "system_prompt": prompt,
        }));
        let Outbound::Legacy(value) = &out[0] else {
            panic!("expected legacy outbound");
        };
        assert_eq!(value["system_prompt"], prompt);
    }
}

#[test]
fn attach_session_does_not_forward_system_prompt_override() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "attach_session", "id": 1, "session_id": "existing",
        "system_prompt": "must not replace the existing prompt",
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert!(value.get("system_prompt").is_none());
}

#[test]
fn create_session_preserves_explicit_working_dir() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "create_session",
        "id": 1,
        "working_dir": "/workspace/explicit",
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(value["working_dir"], "/workspace/explicit");
}

#[test]
fn created_session_reports_requested_working_dir_before_first_prompt() {
    let _home = ScopedJcodeHome::new("create-unpersisted-working-dir");
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "create_session", "id": 7, "working_dir": "/workspace/fresh",
    }));
    let Outbound::Legacy(probe) = &out[1] else {
        panic!("expected state")
    };
    let reply = state.legacy_event_to_api(&json!({
        "type": "state", "id": probe["id"], "session_id": "fresh",
        "message_count": 0, "is_processing": false,
    }));
    let ApiEvent::Attached { session } = &reply[0].event else {
        panic!("expected attached")
    };
    assert_eq!(session.working_dir.as_deref(), Some("/workspace/fresh"));
    assert!(state.pending_create_dir.is_none());
}

#[test]
fn attach_session_defers_to_daemon_even_with_persisted_working_dir() {
    let home = ScopedJcodeHome::new("attach-working-dir");
    let original = home.path.join("original");
    std::fs::create_dir_all(&original).unwrap();
    write_session_record(&home.path, "existing", &original);
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "attach_session",
        "id": 1,
        "session_id": "existing",
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(value["target_session_id"], "existing");
    assert!(
        value.get("working_dir").is_none(),
        "disk cwd must not override a newer live root"
    );
}

#[test]
fn attach_session_without_persisted_working_dir_reclaims_live_target() {
    let _home = ScopedJcodeHome::new("attach-missing-working-dir");
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "attach_session", "id": 41, "session_id": "live-empty",
    }));
    assert_eq!(out.len(), 3);
    let Outbound::Legacy(subscribe) = &out[0] else {
        panic!("expected subscribe")
    };
    assert_eq!(subscribe["type"], "subscribe");
    assert_eq!(subscribe["target_session_id"], "live-empty");
    assert!(subscribe.get("working_dir").is_none());
    let Outbound::Legacy(probe) = &out[1] else {
        panic!("expected state")
    };
    let reply = state.legacy_event_to_api(&json!({
        "type": "state", "id": probe["id"], "session_id": "live-empty",
        "message_count": 0, "is_processing": false,
    }));
    assert_eq!(reply.len(), 2);
    assert_eq!(reply[0].reply_to, Some(41));
    assert!(
        matches!(&reply[0].event, ApiEvent::Attached { session } if session.session_id == "live-empty")
    );
    assert_eq!(state.session_id.as_deref(), Some("live-empty"));
    assert!(state.pending_attach_id.is_none());
    assert!(state.pending_attach_subscribe_id.is_none());
}

#[test]
fn attach_session_unknown_target_error_is_correlated_and_clears_pending_attach() {
    let _home = ScopedJcodeHome::new("attach-unknown-target");
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "attach_session", "id": 43, "session_id": "missing",
    }));
    let Outbound::Legacy(subscribe) = &out[0] else {
        panic!("expected subscribe")
    };
    let frames = state.legacy_event_to_api(&json!({
        "type": "error", "id": subscribe["id"],
        "message": "Unknown session 'missing' or session has no working directory",
    }));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].reply_to, Some(43));
    assert!(matches!(
        frames[0].event,
        ApiEvent::Error {
            code: ErrorCode::UnknownSession,
            ..
        }
    ));
    assert!(state.pending_attach_id.is_none());
    assert!(state.pending_model_probe.is_none());
    assert!(state.session_id.is_none());
    let event = only_reply_event(state.api_request_to_legacy(&json!({
        "req": "clear_session", "id": 44, "session_id": "missing",
    })));
    assert!(matches!(event, ApiEvent::Error { .. }));
}

#[test]
fn desktop_owned_session_requests_crash_on_disconnect() {
    let mut state = BridgeState::with_crash_on_disconnect(true);
    let out = state.api_request_to_legacy(&json!({"req": "create_session", "id": 1}));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(value["crash_on_disconnect"], true);
}

#[test]
fn detach_disarms_crash_on_disconnect() {
    let mut state = BridgeState::with_crash_on_disconnect(true);
    state.session_id = Some("abc".into());
    let out = state.api_request_to_legacy(&json!({
        "req": "detach_session",
        "id": 2,
        "session_id": "abc",
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(value["type"], "prepare_disconnect");
}

#[test]
fn state_event_answers_pending_attach() {
    let home = ScopedJcodeHome::new("attach-title");
    let project = home.path.join("project");
    std::fs::create_dir_all(&project).unwrap();
    write_session_record_with_titles(
        &home.path,
        "abc",
        &project,
        Some("Generated attach title"),
        Some("Persisted attach rename"),
    );
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req": "create_session", "id": 5}));
    assert_eq!(
        out.len(),
        3,
        "subscribe + state chase + model catalog probe"
    );
    let Outbound::Legacy(state_req) = &out[1] else {
        panic!("expected legacy state request");
    };
    assert_eq!(state_req["type"], "state");
    let state_id = state_req["id"].as_u64().unwrap();

    // A subscribe `done` must not leak a turn_done.
    let done = state.legacy_event_to_api(&json!({"type": "done", "id": 1}));
    assert!(done.is_empty());

    let frames = state.legacy_event_to_api(&json!({
        "type": "state", "id": state_id, "session_id": "abc",
        "message_count": 0, "is_processing": false,
    }));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].reply_to, Some(5));
    match &frames[0].event {
        ApiEvent::Attached { session } => {
            assert_eq!(session.session_id, "abc");
            assert_eq!(session.title.as_deref(), Some("Persisted attach rename"));
            assert_eq!(session.working_dir.as_deref(), project.to_str());
        }
        other => panic!("unexpected: {other:?}"),
    }
    assert_eq!(state.session_id.as_deref(), Some("abc"));
}

#[test]
fn send_message_then_done_becomes_turn_done() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(
        &json!({"req": "send_message", "id": 2, "session_id": "s1", "content": "hi"}),
    );
    let Outbound::Legacy(message) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(message["type"], "message");
    let legacy_id = message["id"].as_u64().unwrap();

    let deltas = state.legacy_event_to_api(&json!({"type": "text_delta", "text": "yo"}));
    assert!(matches!(
        &deltas[0].event,
        ApiEvent::TextDelta { session_id, text, .. } if session_id == "s1" && text == "yo"
    ));

    let done = state.legacy_event_to_api(&json!({"type": "done", "id": legacy_id}));
    assert!(matches!(
        &done[1].event,
        ApiEvent::TurnDone { session_id } if session_id == "s1"
    ));
    assert!(matches!(&done[0].event, ApiEvent::TextDone { .. }));
}

/// The daemon acking the in-flight message is the only signal that the agent
/// took delivery, so it must surface as its own event rather than being
/// swallowed as a bookkeeping ack. A client that shows "sent" until the first
/// token of the reply is showing a lie for as long as the model thinks.
#[test]
fn acking_the_pending_message_reports_acceptance() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(
        &json!({"req": "send_message", "id": 2, "session_id": "s1", "content": "hi"}),
    );
    let Outbound::Legacy(message) = &out[0] else {
        panic!("expected legacy outbound");
    };
    let legacy_id = message["id"].as_u64().unwrap();

    let accepted = state.legacy_event_to_api(&json!({"type": "ack", "id": legacy_id}));
    assert!(matches!(
        &accepted[0].event,
        ApiEvent::MessageAccepted { session_id } if session_id == "s1"
    ));
    // The turn must still end normally: the acceptance event must not consume
    // the pending id the `done` boundary depends on.
    let done = state.legacy_event_to_api(&json!({"type": "done", "id": legacy_id}));
    assert!(matches!(&done[0].event, ApiEvent::TurnDone { .. }));
}

#[test]
fn image_soft_interrupt_preserves_pending_message_correlation() {
    let mut state = state_with_session();
    let normal = state.api_request_to_legacy(
        &json!({"req": "send_message", "id": 2, "session_id": "s1", "content": "first"}),
    );
    let Outbound::Legacy(normal) = &normal[0] else {
        panic!("expected legacy message");
    };
    let normal_id = normal["id"].as_u64().unwrap();

    let interrupt = state.api_request_to_legacy(&json!({
        "req": "soft_interrupt", "id": 3, "session_id": "s1", "content": "look",
        "images": [["image/png", "aW1hZ2U="]], "urgent": true
    }));
    let Outbound::Legacy(interrupt) = &interrupt[0] else {
        panic!("expected legacy soft interrupt");
    };
    assert_eq!(interrupt["type"], "soft_interrupt");
    assert_eq!(interrupt["images"], json!([["image/png", "aW1hZ2U="]]));
    let interrupt_id = interrupt["id"].as_u64().unwrap();

    let interrupt_ack = state.legacy_event_to_api(&json!({"type": "ack", "id": interrupt_id}));
    assert_eq!(interrupt_ack.len(), 1);
    assert_eq!(interrupt_ack[0].reply_to, Some(3));
    assert!(matches!(interrupt_ack[0].event, ApiEvent::Ok));

    let accepted = state.legacy_event_to_api(&json!({"type": "ack", "id": normal_id}));
    assert!(matches!(
        &accepted[0].event,
        ApiEvent::MessageAccepted { session_id } if session_id == "s1"
    ));
    let done = state.legacy_event_to_api(&json!({"type": "done", "id": normal_id}));
    assert!(matches!(&done[0].event, ApiEvent::TurnDone { .. }));
}

#[test]
fn idle_soft_interrupt_done_becomes_turn_done() {
    let mut state = state_with_session();
    let interrupt = state.api_request_to_legacy(&json!({
        "req": "soft_interrupt", "id": 3, "session_id": "s1", "content": "start"
    }));
    let Outbound::Legacy(interrupt) = &interrupt[0] else {
        panic!("expected legacy soft interrupt");
    };
    let interrupt_id = interrupt["id"].as_u64().unwrap();

    let done = state.legacy_event_to_api(&json!({"type": "done", "id": interrupt_id}));
    assert!(matches!(
        &done[0].event,
        ApiEvent::TurnDone { session_id } if session_id == "s1"
    ));
}

#[test]
fn context_only_message_waits_for_persistence_event_and_replies_ok() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "req": "send_message", "id": 27, "session_id": "s1",
        "content": "context", "no_reply": true
    }));
    let Outbound::Legacy(message) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(message["type"], "message");
    assert_eq!(message["no_reply"], true);
    let legacy_id = message["id"].as_u64().unwrap();

    assert!(
        state
            .legacy_event_to_api(&json!({"type": "ack", "id": legacy_id}))
            .is_empty(),
        "the daemon's early ack does not prove persistence"
    );
    let frames =
        state.legacy_event_to_api(&json!({"type": "context_message_added", "id": legacy_id}));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].reply_to, Some(27));
    assert!(matches!(frames[0].event, ApiEvent::Ok));
    assert!(
        state
            .legacy_event_to_api(&json!({"type": "done", "id": legacy_id}))
            .is_empty(),
        "context-only messages never create turn boundaries"
    );
}

#[test]
fn context_only_message_error_is_correlated_to_the_request() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "req": "send_message", "id": 28, "session_id": "s1",
        "content": "context", "no_reply": true
    }));
    let Outbound::Legacy(message) = &out[0] else {
        panic!("expected legacy outbound");
    };
    let legacy_id = message["id"].as_u64().unwrap();
    let frames = state
        .legacy_event_to_api(&json!({"type": "error", "id": legacy_id, "message": "save failed"}));
    assert_eq!(frames[0].reply_to, Some(28));
    assert!(matches!(
        &frames[0].event,
        ApiEvent::Error { message, .. } if message == "save failed"
    ));
    assert!(
        state
            .legacy_event_to_api(&json!({"type": "context_message_added", "id": legacy_id}))
            .is_empty()
    );
}

/// An ack for anything else (a ping, a clear) is still a plain request reply:
/// promoting those to acceptance would wiggle a message that nobody sent.
#[test]
fn acking_an_unrelated_request_stays_a_reply() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req": "clear", "id": 9, "session_id": "s1"}));
    let Outbound::Legacy(clear) = &out[0] else {
        panic!("expected legacy outbound");
    };
    let legacy_id = clear["id"].as_u64().unwrap();
    let frames = state.legacy_event_to_api(&json!({"type": "ack", "id": legacy_id}));
    assert_eq!(frames[0].reply_to, Some(9));
    assert!(matches!(&frames[0].event, ApiEvent::Ok));
}

#[test]
fn ping_pong_roundtrip() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req": "ping", "id": 9}));
    let Outbound::Legacy(ping) = &out[0] else {
        panic!("expected legacy outbound");
    };
    let legacy_id = ping["id"].as_u64().unwrap();
    let frames = state.legacy_event_to_api(&json!({"type": "pong", "id": legacy_id}));
    assert_eq!(frames[0].reply_to, Some(9));
    assert!(matches!(frames[0].event, ApiEvent::Pong));
}

#[test]
fn history_reply_is_mapped() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req": "get_history", "id": 4}));
    let Outbound::Legacy(get) = &out[0] else {
        panic!("expected legacy outbound");
    };
    let legacy_id = get["id"].as_u64().unwrap();
    let frames = state.legacy_event_to_api(&json!({
        "type": "history",
        "id": legacy_id,
        "session_id": "s1",
        "messages": [{"role": "user", "content": "hi"}],
    }));
    match &frames[0].event {
        ApiEvent::History { messages, .. } => {
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].role, "user");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn unknown_legacy_events_are_dropped() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({"type": "swarm_event", "data": {}}));
    assert!(frames.is_empty());
}

#[test]
fn unknown_api_request_gets_error_reply() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req": "frobnicate", "id": 3}));
    let Outbound::Reply(frame) = &out[0] else {
        panic!("expected direct reply");
    };
    assert_eq!(frame.reply_to, Some(3));
    assert!(matches!(
        frame.event,
        ApiEvent::Error {
            code: ErrorCode::UnknownRequest,
            ..
        }
    ));
}

#[test]
fn error_routes_to_pending_request() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req": "clear", "id": 7}));
    let Outbound::Legacy(clear) = &out[0] else {
        panic!("expected legacy outbound");
    };
    let legacy_id = clear["id"].as_u64().unwrap();
    let frames =
        state.legacy_event_to_api(&json!({"type": "error", "id": legacy_id, "message": "nope"}));
    assert_eq!(frames[0].reply_to, Some(7));
}

/// Attaching must volunteer the model identity: a client that has to know to
/// ask would show "unknown model" forever, which is what this fixes.
#[test]
fn attaching_probes_and_reports_the_model() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req": "create_session", "id": 7}));
    let Outbound::Legacy(catalog) = &out[2] else {
        panic!("expected a legacy catalog probe");
    };
    assert_eq!(catalog["type"], "get_model_catalog");
    let catalog_id = catalog["id"].as_u64().unwrap();

    // The daemon answers the probe with a `history`-shaped reply carrying no
    // messages. That must become an unsolicited model_info event, not a reply
    // to some client request that never asked for history.
    let frames = state.legacy_event_to_api(&json!({
        "type": "history", "id": catalog_id, "messages": [],
        "provider_name": "anthropic", "provider_model": "claude-sonnet-4-5",
    }));
    assert_eq!(frames.len(), 2);
    assert!(matches!(frames[1].event, ApiEvent::RuntimeInfo { .. }));
    assert_eq!(
        frames[0].reply_to, None,
        "the probe was not client-initiated"
    );
    match &frames[0].event {
        ApiEvent::ModelInfo {
            provider, model, ..
        } => {
            assert_eq!(provider.as_deref(), Some("anthropic"));
            assert_eq!(model.as_deref(), Some("claude-sonnet-4-5"));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

/// A real `get_history` reply must still be a history reply after the probe has
/// been consumed, or the probe would swallow the client's own request.
#[test]
fn a_client_history_request_is_untouched_by_the_probe() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req": "create_session", "id": 1}));
    let Outbound::Legacy(catalog) = &out[2] else {
        panic!("expected a catalog probe");
    };
    let catalog_id = catalog["id"].as_u64().unwrap();
    state.legacy_event_to_api(&json!({"type": "history", "id": catalog_id, "messages": []}));

    let out = state.api_request_to_legacy(&json!({"req": "get_history", "id": 9}));
    let Outbound::Legacy(request) = &out[0] else {
        panic!("expected a legacy history request");
    };
    let history_id = request["id"].as_u64().unwrap();
    let frames = state.legacy_event_to_api(&json!({
        "type": "history", "id": history_id,
        "messages": [{"role": "user", "content": "hi"}],
    }));
    assert_eq!(frames[0].reply_to, Some(9));
    assert!(matches!(frames[0].event, ApiEvent::History { .. }));
}

/// Switching model mid-session must reach the client, or the caption goes stale
/// and confidently lies about which model answered.
#[test]
fn a_model_change_is_forwarded() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "model_changed", "id": 3,
        "model": "gpt-5.6", "provider_name": "openai",
    }));
    match &frames[0].event {
        ApiEvent::ModelInfo {
            provider, model, ..
        } => {
            assert_eq!(provider.as_deref(), Some("openai"));
            assert_eq!(model.as_deref(), Some("gpt-5.6"));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

/// A failed model change must not be reported as the active model.
#[test]
fn a_failed_model_change_is_not_reported() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "model_changed", "id": 3, "model": "nope", "error": "no such model",
    }));
    assert!(frames.is_empty());
}

/// An auth change re-resolves the route, so the push must update the caption.
#[test]
fn an_available_models_push_updates_the_model() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "available_models_updated",
        "provider_name": "anthropic", "provider_model": "claude-opus-4-5",
        "available_models": ["claude-opus-4-5"],
    }));
    match &frames[0].event {
        ApiEvent::ModelInfo {
            session_id, model, ..
        } => {
            assert_eq!(session_id, "s1");
            assert_eq!(model.as_deref(), Some("claude-opus-4-5"));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn create_session_in_a_jcode_checkout_requests_selfdev() {
    // Regression: external client opens its own crate, and without the `selfdev`
    // flag the daemon hands back an agent with no self-dev tools or prompt.
    let mut state = BridgeState::default();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .join("crates/jcode-tui");
    let out = state.api_request_to_legacy(&json!({
        "req": "create_session",
        "id": 1,
        "working_dir": repo.display().to_string(),
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert_eq!(value["selfdev"], json!(true));
}

#[test]
fn create_session_outside_a_checkout_leaves_selfdev_unset() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({
        "req": "create_session",
        "id": 1,
        "working_dir": "/",
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("expected legacy outbound");
    };
    assert!(value.get("selfdev").is_none(), "got {value}");
}

/// A turn that fails ends with `error` instead of `done`. The bridge must let
/// go of the pending message, or a later unrelated `done` reusing that legacy
/// id would be reported to the client as this turn finally finishing, and a
/// client that trusts `turn_done` would unblock on a turn that never ran.
#[test]
fn a_failed_turn_clears_the_pending_message() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "req": "send_message", "id": 11, "content": "hi",
    }));
    let Outbound::Legacy(message) = &out[0] else {
        panic!("expected a legacy message");
    };
    let legacy_id = message["id"].as_u64().expect("a legacy id");

    let frames = state.legacy_event_to_api(&json!({
        "type": "error", "id": legacy_id, "message": "dns error",
    }));
    assert!(
        frames
            .iter()
            .any(|frame| matches!(frame.event, ApiEvent::Error { .. })),
        "the failure was not forwarded"
    );

    // The same id arriving as `done` afterwards is no longer this turn.
    let frames = state.legacy_event_to_api(&json!({"type": "done", "id": legacy_id}));
    assert!(
        !frames
            .iter()
            .any(|frame| matches!(frame.event, ApiEvent::TurnDone { .. })),
        "a failed turn reported a second, phantom completion"
    );
}

#[test]
fn background_notifications_become_progress_events() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "notification",
        "from_session": "background_task",
        "message": "**Background task progress** `t9` · `bash`\n\n[#####-----] 50% · Running tests (reported)",
    }));
    assert_eq!(frames.len(), 1);
    match &frames[0].event {
        ApiEvent::BackgroundProgress {
            session_id,
            task_id,
            percent,
            done,
            ..
        } => {
            assert_eq!(session_id, "s1");
            assert_eq!(task_id, "t9");
            assert_eq!(*percent, Some(50.0));
            assert!(!done);
        }
        other => panic!("unexpected background event: {other:?}"),
    }
}

/// A DM or a shared-context push is not progress, and inventing a bar for it
/// would put a phantom task on every client's screen.
#[test]
fn unrelated_notifications_are_dropped() {
    let mut state = state_with_session();
    let frames = state.legacy_event_to_api(&json!({
        "type": "notification",
        "from_session": "fox",
        "message": "hello from another agent",
    }));
    assert!(frames.is_empty());
}

/// The daemon answers a `ping` that arrives as the first frame on a connection
/// and then closes it, because it classifies ping as a one-shot lightweight
/// control request. Forwarding an unattached ping therefore destroys the
/// client's connection before it ever gets a session, which is the opposite of
/// what a liveness probe should do.
#[test]
fn ping_before_attach_is_answered_locally() {
    let mut state = BridgeState::default();
    let out = state.api_request_to_legacy(&json!({"req": "ping", "id": 4}));
    match out.as_slice() {
        [Outbound::Reply(frame)] => {
            assert_eq!(frame.reply_to, Some(4));
            assert_eq!(frame.event, ApiEvent::Pong);
        }
        other => panic!("ping must not reach the daemon before attach: {other:?}"),
    }
}

/// Once attached the connection is a normal session connection, so ping is a
/// genuine round trip and should measure the daemon, not the bridge.
#[test]
fn ping_after_attach_reaches_the_daemon() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({"req": "ping", "id": 5}));
    match out.as_slice() {
        [Outbound::Legacy(value)] => assert_eq!(value["type"], "ping"),
        other => panic!("expected a forwarded ping: {other:?}"),
    }
}

/// The daemon closes the connection on a stateful request that arrives before
/// a subscribe. Forwarding one therefore does not just fail the request: it
/// destroys the client's whole connection, taking every other in-flight
/// request with it, and the SDK sees a bare EPIPE. Answer locally.
#[test]
fn stateful_requests_before_attach_are_refused_locally() {
    for req in [
        "send_message",
        "cancel",
        "soft_interrupt",
        "clear",
        "rewind",
        "get_history",
    ] {
        let mut state = BridgeState::default();
        let out = state.api_request_to_legacy(&json!({
            "req": req,
            "id": 7,
            "session_id": "session_does_not_exist",
        }));
        assert_eq!(out.len(), 1, "{req} should produce exactly one reply");
        let Outbound::Reply(frame) = &out[0] else {
            panic!("{req} was forwarded to the daemon, which will close the connection");
        };
        assert_eq!(frame.reply_to, Some(7));
        match &frame.event {
            ApiEvent::Error { code, message } => {
                assert_eq!(*code, ErrorCode::UnknownSession, "{req}");
                assert!(
                    message.contains("session_does_not_exist"),
                    "{req} error should name the session: {message}"
                );
            }
            other => panic!("{req} expected an error frame, got {other:?}"),
        }
    }
}

/// The legacy protocol has no session field, so a request naming a *different*
/// session than the attached one would be applied to the attached one. A
/// `clear` or `rewind` aimed at the wrong id would then destroy a transcript
/// the caller never named.
#[test]
fn requests_for_another_session_do_not_hit_the_attached_one() {
    let mut state = state_with_session();
    let out = state.api_request_to_legacy(&json!({
        "req": "clear",
        "id": 9,
        "session_id": "some_other_session",
    }));
    let Outbound::Reply(frame) = &out[0] else {
        panic!("clear for another session must not reach the daemon");
    };
    match &frame.event {
        ApiEvent::Error { code, message } => {
            assert_eq!(*code, ErrorCode::UnknownSession);
            assert!(message.contains("s1") && message.contains("some_other_session"));
        }
        other => panic!("expected an error frame, got {other:?}"),
    }
}

/// The guard must not break the normal path: the attached session's own id,
/// and an omitted id, both still reach the daemon.
#[test]
fn attached_requests_still_reach_the_daemon() {
    let mut state = state_with_session();
    let named = state.api_request_to_legacy(&json!({
        "req": "get_history", "id": 1, "session_id": "s1",
    }));
    assert!(matches!(named[0], Outbound::Legacy(_)), "explicit id");

    let bare = state.api_request_to_legacy(&json!({"req": "get_history", "id": 2}));
    assert!(matches!(bare[0], Outbound::Legacy(_)), "omitted id");
}

/// Reading around without attaching is the entire point of `peek_session` and
/// `list_sessions`, so the attach guard must leave them alone.
#[test]
fn browsing_requests_work_without_attaching() {
    let _home = ScopedJcodeHome::new("browsing-without-attach");
    let mut state = BridgeState::default();
    for req in ["list_sessions", "peek_session", "ping"] {
        let out = state.api_request_to_legacy(&json!({
            "req": req, "id": 1, "session_id": "whatever",
        }));
        let Outbound::Reply(frame) = &out[0] else {
            panic!("{req} should be answered locally");
        };
        assert!(
            !matches!(frame.event, ApiEvent::Error { .. }),
            "{req} must not be refused by the attach guard: {:?}",
            frame.event
        );
    }
}

/// A client may pipeline: `create_session` then `send_message` without
/// awaiting the attach. The subscribe is already on the wire, so the daemon
/// will have a session by the time the message lands. Refusing here would
/// break the SDK's own `run()` path.
#[test]
fn a_message_pipelined_behind_create_session_is_forwarded() {
    let mut state = BridgeState::default();
    state.api_request_to_legacy(&json!({"req": "create_session", "id": 1}));
    let out = state.api_request_to_legacy(&json!({
        "req": "send_message", "id": 2, "content": "hi",
    }));
    let Outbound::Legacy(value) = &out[0] else {
        panic!("a pipelined message must reach the daemon, not be refused");
    };
    assert_eq!(value["type"], "message");
}

// --- Capabilities added to close the API coverage gaps --------------------

/// The catalog arrives on attach, so a picker must open without a round trip.
#[test]
fn list_models_is_answered_from_the_cached_catalog() {
    let mut state = state_with_session();
    state.legacy_event_to_api(&json!({
        "type": "available_models_updated",
        "provider_model": "claude-opus-5",
        "available_models": ["claude-opus-5", "claude-fable-5"],
    }));

    let out = state.api_request_to_legacy(&json!({"id": 9, "req": "list_models"}));
    match &out[..] {
        [Outbound::Reply(frame)] => match &frame.event {
            ApiEvent::Models {
                models, current, ..
            } => {
                assert_eq!(models, &["claude-opus-5", "claude-fable-5"]);
                assert_eq!(current.as_deref(), Some("claude-opus-5"));
            }
            other => panic!("unexpected: {other:?}"),
        },
        other => panic!("expected one local reply, got {other:?}"),
    }
}

#[test]
fn model_usage_survives_catalogs_and_live_updates_without_a_round_trip() {
    let mut state = state_with_session();
    let mut route = json!({"model":"test-model","provider":"OpenAI","api_method":"openai-oauth",
        "available":true,"detail":"ready", "usage":{"count":2,"last_used_unix_secs":30,
        "tracking_started_unix_secs":10,"selection_count":5,"last_selected_unix_secs":9}});
    let catalog = json!({"type":"available_models_updated","provider_model":"test-model",
        "available_models":["test-model"],"available_model_routes":[route.clone()]});
    state.legacy_event_to_api(&catalog);
    route["usage"]["count"] = json!(3);
    route["usage"]["last_used_unix_secs"] = json!(40);
    let frames = state.legacy_event_to_api(&json!({"type":"model_usage_updated","route":route}));
    let ApiEvent::RuntimeInfo { routes, .. } = &frames[0].event else {
        panic!("usage must push runtime info");
    };
    assert_eq!(routes[0].usage.as_ref().unwrap().count, 3);
    // A pending catalog reply must not undo a newer usage delta.
    state.legacy_event_to_api(&catalog);
    let out = state.api_request_to_legacy(&json!({"id":88,"req":"get_runtime_info"}));
    let [Outbound::Reply(frame)] = &out[..] else {
        panic!("warm cache must answer locally");
    };
    let ApiEvent::RuntimeInfo { routes, .. } = &frame.event else {
        panic!("expected runtime info");
    };
    let usage = routes[0].usage.as_ref().unwrap();
    assert_eq!(usage.count, 3);
    assert_eq!(usage.last_used_unix_secs, Some(40));
    assert_eq!(usage.selection_count, 5);
    assert_eq!(usage.tracking_started_unix_secs, Some(10));
}

include!("translate_tests_partition_01_tests.rs");
include!("translate_tests_partition_02_tests.rs");

#[path = "translate_regression_tests.rs"]
mod regression_tests;
