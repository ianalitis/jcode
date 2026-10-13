// Pinned-todo boundary alignment.
//
// With `display.pin_todos` enabled the todo card is omitted from the prepared
// transcript, but the message it came from must still record a boundary:
// `matching_prefix_len` matches `message_boundaries[k]` against
// `display_messages[k]` positionally, so a skipped message that records
// nothing shifts every later boundary by one and forces a full body rebuild on
// the next append.

// Uses the `PinTodosGuard` defined in `render_boundaries.rs` (same module).

/// Transcript whose third message is a `todo` tool call. With pinned todos on,
/// that message's card is skipped from the prepared body.
fn pinned_todo_transcript() -> TestState {
    TestState {
        display_messages: vec![
            DisplayMessage::user("first prompt"),
            DisplayMessage::assistant("initial answer"),
            DisplayMessage {
                role: "tool".to_string(),
                content: "Todos updated".to_string(),
                tool_calls: Vec::new(),
                duration_secs: None,
                title: None,
                tool_data: Some(ToolCall {
                    id: "call-todo".to_string(),
                    name: "todo".to_string(),
                    input: serde_json::json!({"action": "update"}),
                    intent: Some("Track the work".to_string()),
                    thought_signature: None,
                }),
            },
            DisplayMessage::assistant("follow-up answer"),
        ],
        messages_version: 1,
        ..Default::default()
    }
}

/// The pinned todo card is omitted from the prepared body, but the message must
/// still record a boundary. `matching_prefix_len` compares
/// `message_boundaries[k]` against `display_messages[k]` positionally, so a
/// skipped message that records nothing shifts every later boundary by one and
/// the next append can only reuse the prefix up to the todo card (or nothing).
#[test]
fn test_pinned_todo_skip_keeps_one_boundary_per_display_message() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();
    assert!(
        crate::config::config().display.pin_todos,
        "JCODE_PIN_TODOS must take effect for this test to exercise the skip path"
    );

    let state = pinned_todo_transcript();
    let prepared = super::prepare::prepare_body(&state, 80, false);

    assert_eq!(
        prepared.message_boundaries.len(),
        state.display_messages.len(),
        "the pinned todo message renders no lines but must still own a boundary"
    );
    assert_eq!(
        prepared.message_boundaries[2].msg_hash,
        state.display_messages[2].stable_cache_hash(),
        "the skipped message's boundary must carry its own hash"
    );
    // A skipped message contributes a zero-length segment: it must not claim
    // wrapped, raw, or prompt lines from its neighbours.
    let skipped = prepared.message_boundaries[2];
    let previous = prepared.message_boundaries[1];
    assert_eq!(skipped.wrapped_len, previous.wrapped_len);
    assert_eq!(skipped.raw_len, previous.raw_len);
    assert_eq!(skipped.user_prompt_len, previous.user_prompt_len);
    assert!(
        !prepared
            .wrapped_plain_lines
            .iter()
            .any(|line| line.contains("Todos updated")),
        "the pinned card must stay out of the prepared body"
    );
}

/// Appending to a transcript that contains a pinned todo card must reuse the
/// whole existing prefix and still produce a body identical to a full rebuild
/// (including one boundary per display message).
#[test]
fn test_prefix_reuse_append_across_pinned_todo_matches_full_build() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();
    assert!(
        crate::config::config().display.pin_todos,
        "JCODE_PIN_TODOS must take effect for this test to exercise the skip path"
    );

    let width = 72;
    let base_state = pinned_todo_transcript();
    let mut grown_state = base_state.clone();
    grown_state
        .display_messages
        .push(DisplayMessage::user("second prompt"));
    grown_state.display_messages.push(DisplayMessage::assistant(
        "follow-up answer to the second question",
    ));
    grown_state.messages_version = 2;

    let base = Arc::new(super::prepare::prepare_body(&base_state, width, false));
    let k = super::prepare::matching_prefix_len(base.as_ref(), &grown_state.display_messages);
    assert_eq!(
        k,
        base_state.display_messages.len(),
        "every unchanged message must hash-match, including the skipped todo card"
    );

    let (reused, path) = super::prepare::build_body_from_base(
        &grown_state,
        width,
        base,
        k,
        0,
        grown_state.display_messages.len(),
    );
    assert_eq!(
        path, "incremental",
        "an append must extend the body, not rebuild it"
    );
    let full = super::prepare::prepare_body(&grown_state, width, false);
    assert_prepared_equivalent(
        &reused,
        &full,
        &grown_state.display_messages,
        "pinned_todo_append",
    );
}
