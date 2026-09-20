use super::*;
use ratatui::style::Modifier;

#[test]
fn running_tool_header_emphasizes_detail_over_tool_name() {
    let accent = Color::Rgb(12, 34, 56);
    let spans = running_tool_header_spans("*", "bash", Some("cargo test"), accent);

    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content.as_ref(), "*");
    assert_eq!(spans[0].style.fg, Some(accent));
    assert_eq!(spans[1].content.as_ref(), " running bash");
    assert_eq!(spans[1].style.fg, Some(dim_color()));
    assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(spans[2].content.as_ref(), " · cargo test");
    assert_eq!(spans[2].style.fg, Some(accent));
    assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn visual_line_move_follows_soft_wrapped_rows() {
    // 20 chars, width 10 => two visual rows, no newline in the input.
    let input = "abcdefghijklmnopqrst";
    // Cursor at col 3 of row 1 (char 13) moving up lands on col 3 of row 0.
    assert_eq!(visual_line_move(input, 13, 10, -1), Some(3));
    // And back down again.
    assert_eq!(visual_line_move(input, 3, 10, 1), Some(13));
    // Already on the first/last row => None so history recall can take over.
    assert_eq!(visual_line_move(input, 3, 10, -1), None);
    assert_eq!(visual_line_move(input, 13, 10, 1), None);
}

#[test]
fn visual_line_move_clamps_to_shorter_target_row() {
    let input = "abcdefghij\nxy";
    // Cursor at end of the short second row, up goes to col 2 of row 0.
    assert_eq!(visual_line_move(input, input.len(), 10, -1), Some(2));
    // From far along row 0, down clamps to the end of the short row.
    assert_eq!(visual_line_move(input, 8, 10, 1), Some(input.len()));
}

#[test]
fn right_fact_stack_shifts_up_as_a_unit_when_bottom_row_is_occupied() {
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    for x in 12..40 {
        buffer[(x, 4)].set_symbol("x");
    }
    let lines = ["oauth", "model", "dir", "context"]
        .into_iter()
        .map(|text| RightFactLine::new(vec![Span::raw(text)]).expect("fact line"))
        .collect();

    let placements = right_fact_placements(
        &buffer,
        lines,
        0,
        5,
        0,
        40,
        Rect::new(0, 0, 40, 3),
        false,
        None,
    );
    let rows = placements
        .iter()
        .map(|placement| placement.area.y)
        .collect::<Vec<_>>();
    assert_eq!(rows, vec![0, 1, 2, 3]);
    assert!(placements.iter().all(|placement| placement.area.y != 4));
}

#[test]
fn right_fact_stack_never_leaves_an_occupied_row_between_facts() {
    let area = Rect::new(0, 0, 40, 7);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    buffer[(39, 4)].set_symbol("x");
    let lines = ["oauth", "model", "dir", "context"]
        .into_iter()
        .map(|text| RightFactLine::new(vec![Span::raw(text)]).expect("fact line"))
        .collect();

    let placements = right_fact_placements(
        &buffer,
        lines,
        0,
        7,
        0,
        40,
        Rect::new(0, 0, 40, 5),
        false,
        None,
    );
    let rows = placements
        .iter()
        .map(|placement| placement.area.y)
        .collect::<Vec<_>>();
    assert_eq!(rows, vec![0, 1, 2, 3]);
    assert_eq!(
        placements
            .iter()
            .map(|placement| placement.line.spans[0].content.as_ref())
            .collect::<Vec<_>>(),
        vec!["oauth", "model", "dir", "context"]
    );
}

#[test]
fn right_fact_stack_collision_state_space_is_contiguous_or_hidden() {
    const HEIGHT: u16 = 8;
    const STACK_HEIGHT: u16 = 4;

    for occupied_mask in 0_u16..(1 << HEIGHT) {
        let area = Rect::new(0, 0, 40, HEIGHT);
        let mut buffer = ratatui::buffer::Buffer::empty(area);
        for row in 0..HEIGHT {
            if occupied_mask & (1 << row) != 0 {
                buffer[(39, row)].set_symbol("x");
            }
        }
        let lines = ["oauth", "model", "dir", "context"]
            .into_iter()
            .map(|text| RightFactLine::new(vec![Span::raw(text)]).expect("fact line"))
            .collect();

        let placements = right_fact_placements(
            &buffer,
            lines,
            0,
            HEIGHT,
            0,
            40,
            Rect::new(0, 0, 40, HEIGHT),
            false,
            None,
        );
        let expected_top = (0..=HEIGHT - STACK_HEIGHT).rev().find(|&start| {
            (start..start + STACK_HEIGHT).all(|row| occupied_mask & (1 << row) == 0)
        });

        match expected_top {
            Some(start) => {
                assert_eq!(
                    placements.len(),
                    STACK_HEIGHT as usize,
                    "mask {occupied_mask:08b}"
                );
                assert_eq!(
                    placements
                        .iter()
                        .map(|placement| placement.area.y)
                        .collect::<Vec<_>>(),
                    (start..start + STACK_HEIGHT).collect::<Vec<_>>(),
                    "mask {occupied_mask:08b}"
                );
                assert_eq!(
                    placements
                        .iter()
                        .map(|placement| placement.line.spans[0].content.as_ref())
                        .collect::<Vec<_>>(),
                    vec!["oauth", "model", "dir", "context"],
                    "mask {occupied_mask:08b}"
                );
            }
            None => assert!(placements.is_empty(), "mask {occupied_mask:08b}"),
        }
    }
}

#[test]
fn right_fact_stack_treats_styled_blank_cells_as_occupied() {
    let area = Rect::new(0, 0, 32, 2);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    for x in 12..32 {
        buffer[(x, 1)].set_bg(Color::Blue);
    }
    let line = RightFactLine::new(vec![Span::raw("context")]).expect("fact line");
    let placements = right_fact_placements(
        &buffer,
        vec![line],
        0,
        2,
        0,
        32,
        Rect::new(0, 0, 32, 1),
        false,
        None,
    );
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].area.y, 0);
}

#[test]
fn right_fact_stack_never_draws_over_the_input_cursor() {
    let area = Rect::new(0, 0, 32, 2);
    let buffer = ratatui::buffer::Buffer::empty(area);
    let line = RightFactLine::new(vec![Span::raw("context")]).expect("fact line");
    let placements = right_fact_placements(
        &buffer,
        vec![line],
        0,
        2,
        0,
        32,
        Rect::new(0, 0, 32, 1),
        false,
        Some(Position::new(28, 1)),
    );
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].area.y, 0);
}

#[test]
fn overscroll_provider_display_is_credential_neutral() {
    // The credential (OAuth vs API key) is reported by the adjacent auth
    // chip from canonical resolution; the provider name must not bake in a
    // credential or the two can contradict (e.g. "Claude OAuth · API key").
    assert_eq!(overscroll_provider_display("claude"), "Claude");
    assert_eq!(overscroll_provider_display("anthropic"), "Anthropic");
    assert!(!overscroll_provider_display("claude").contains("OAuth"));
    assert!(!overscroll_provider_display("anthropic").contains("API"));
}

#[test]
fn session_history_warning_is_clear_and_occasional() {
    assert!(occasional_session_history_warning(249_999, 0, None, 100, 0).is_none());
    assert!(occasional_session_history_warning(300_000, 0, None, 63, 0).is_none());
    assert!(occasional_session_history_warning(300_000, 0, None, 100, 10).is_none());
    assert!(occasional_session_history_warning(199_999, 0, Some(200_000), 100, 0).is_none());
    assert!(occasional_session_history_warning(300_000, 0, Some(400_000), 100, 0).is_none());
    assert!(occasional_session_history_warning(450_000, 0, Some(400_000), 100, 0).is_none());

    let warning = occasional_session_history_warning(2_500_000, 4, Some(500_000), 100, 0)
        .expect("large sessions should get a brief reminder");
    assert!(warning.contains("Session history: 2.5M tokens processed and 4 compacts"));
    assert!(warning.contains("/clear starts fresh context"));
    assert!(!warning.contains("Context usage"));
}

#[test]
fn command_suggestion_window_start_scrolls_after_visible_limit() {
    let limit = app::COMMAND_SUGGESTION_VISIBLE_LIMIT;
    assert_eq!(command_suggestion_window_start(0, limit + 3), 0);
    assert_eq!(command_suggestion_window_start(limit - 1, limit + 3), 0);
    assert_eq!(command_suggestion_window_start(limit, limit + 3), 1);
    assert_eq!(command_suggestion_window_start(limit + 2, limit + 3), 3);
}

#[test]
fn command_suggestions_overlay_prefers_space_below_input() {
    let frame = Rect::new(0, 0, 80, 20);
    let input_area = Rect::new(0, 10, 80, 1);

    assert_eq!(
        command_suggestions_overlay_rect(input_area, 3, frame),
        Some(Rect::new(0, 11, 80, 3))
    );
}

#[test]
fn command_suggestions_overlay_flips_above_at_terminal_bottom() {
    let frame = Rect::new(0, 0, 80, 20);
    let input_area = Rect::new(0, 19, 80, 1);

    assert_eq!(
        command_suggestions_overlay_rect(input_area, 3, frame),
        Some(Rect::new(0, 16, 80, 3))
    );
}

#[test]
fn command_suggestions_overlay_handles_zero_lines_and_no_space() {
    let frame = Rect::new(0, 0, 80, 20);
    let input_area = Rect::new(0, 10, 80, 1);
    assert_eq!(command_suggestions_overlay_rect(input_area, 0, frame), None);

    // Input fills the whole frame: nowhere to float, so no popover.
    let full = Rect::new(0, 0, 80, 20);
    assert_eq!(command_suggestions_overlay_rect(full, 3, frame), None);
}

#[test]
fn batch_progress_spans_use_batch_chroma_for_initial_count() {
    let mut spans = Vec::new();
    let anim_color = rgb(12, 34, 56);

    append_batch_progress_spans(&mut spans, anim_color, None, Some(3));

    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), " · 0/3 done");
    assert_eq!(spans[0].style.fg, Some(anim_color));
    assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn batch_progress_spans_make_last_completed_explicit() {
    let mut spans = Vec::new();

    append_batch_progress_spans(
        &mut spans,
        rgb(120, 130, 140),
        Some(crate::bus::BatchProgress {
            session_id: "s".to_string(),
            tool_call_id: "tc".to_string(),
            total: 3,
            completed: 1,
            last_completed: Some("read".to_string()),
            running: Vec::new(),
            subcalls: Vec::new(),
        }),
        Some(3),
    );

    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content.as_ref(), " · 1/3 done");
    assert_eq!(spans[1].content.as_ref(), " · last done: read");
}

#[test]
fn batch_progress_spans_hide_last_completed_when_batch_finished() {
    let mut spans = Vec::new();

    append_batch_progress_spans(
        &mut spans,
        rgb(120, 130, 140),
        Some(crate::bus::BatchProgress {
            session_id: "s".to_string(),
            tool_call_id: "tc".to_string(),
            total: 3,
            completed: 3,
            last_completed: Some("read".to_string()),
            running: Vec::new(),
            subcalls: Vec::new(),
        }),
        Some(3),
    );

    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), " · 3/3 done");
}

#[test]
fn batch_progress_spans_show_running_subcall_detail() {
    let mut spans = Vec::new();

    append_batch_progress_spans(
        &mut spans,
        rgb(120, 130, 140),
        Some(crate::bus::BatchProgress {
            session_id: "s".to_string(),
            tool_call_id: "tc".to_string(),
            total: 2,
            completed: 0,
            last_completed: None,
            running: vec![crate::message::ToolCall {
                id: "batch-1-bash".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({"command": "cargo test -p jcode"}),
                intent: None,
                thought_signature: None,
            }],
            subcalls: Vec::new(),
        }),
        Some(2),
    );

    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content.as_ref(), " · 0/2 done");
    assert_eq!(spans[1].content.as_ref(), " · running: #1 bash");
}

#[test]
fn batch_progress_spans_show_multiple_running_subcalls() {
    let mut spans = Vec::new();

    append_batch_progress_spans(
        &mut spans,
        rgb(120, 130, 140),
        Some(crate::bus::BatchProgress {
            session_id: "s".to_string(),
            tool_call_id: "tc".to_string(),
            total: 3,
            completed: 0,
            last_completed: None,
            running: vec![
                crate::message::ToolCall {
                    id: "batch-2-grep".to_string(),
                    name: "grep".to_string(),
                    input: serde_json::json!({"pattern": "foo", "path": "src"}),
                    intent: None,
                    thought_signature: None,
                },
                crate::message::ToolCall {
                    id: "batch-1-bash".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "cargo build --release --workspace"}),
                    intent: None,
                    thought_signature: None,
                },
                crate::message::ToolCall {
                    id: "batch-3-read".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({"file_path": "README.md"}),
                    intent: None,
                    thought_signature: None,
                },
            ],
            subcalls: Vec::new(),
        }),
        Some(3),
    );

    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content.as_ref(), " · 0/3 done");
    assert_eq!(spans[1].content.as_ref(), " · running: #1 bash +2");
}

#[test]
fn connection_phase_waiting_label_is_generic_response_wait() {
    assert_eq!(
        connection_phase_label(&ConnectionPhase::WaitingForResponse),
        "waiting for response"
    );
}

#[test]
fn streaming_liveness_label_shows_quiet_stream_warning_before_message_end() {
    assert_eq!(
        streaming_liveness_label("4.2s".to_string(), Some(3.4), false),
        "(no tokens 3s) · 4.2s"
    );
    assert_eq!(
        streaming_liveness_label("12.0s".to_string(), Some(12.1), false),
        "(stalled 12s) · 12.0s"
    );
}

#[test]
fn streaming_liveness_label_suppresses_quiet_stream_warning_after_message_end() {
    assert_eq!(
        streaming_liveness_label("4.2s".to_string(), Some(3.4), true),
        "4.2s"
    );
    assert_eq!(
        streaming_liveness_label("12.0s".to_string(), Some(12.1), true),
        "12.0s"
    );
}

#[test]
fn streaming_status_spans_keep_spinner_while_finalizing() {
    let spans = streaming_status_spans("⠋", "4.2s".to_string(), false, false, " · +1 queued");

    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content.as_ref(), "⠋");
    assert_eq!(spans[1].content.as_ref(), " 4.2s");
    assert_eq!(spans[2].content.as_ref(), " · +1 queued");
}

#[test]
fn streaming_status_spans_keep_spinner_after_message_end_while_finalizing() {
    let spans = streaming_status_spans("⠋", "finalizing".to_string(), true, false, "");

    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content.as_ref(), "⠋");
    assert_eq!(spans[1].content.as_ref(), " finalizing");
}

#[test]
fn push_queued_suffix_appends_only_when_present() {
    let mut spans: Vec<Span<'static>> = Vec::new();
    push_queued_suffix(&mut spans, "");
    assert!(spans.is_empty(), "empty suffix should add no span");

    push_queued_suffix(&mut spans, " · +2 queued");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), " · +2 queued");
    assert_eq!(spans[0].style.fg, Some(queued_color()));
}

#[test]
fn display_connection_type_uses_reader_friendly_labels() {
    assert_eq!(display_connection_type("https/sse"), "https");
    assert_eq!(
        display_connection_type("websocket/persistent-fresh"),
        "websocket"
    );
    assert_eq!(
        display_connection_type("websocket/persistent-reuse"),
        "existing websocket"
    );
}

#[test]
fn normalize_status_detail_uses_reader_friendly_labels() {
    assert_eq!(
        normalize_status_detail("fresh websocket").as_deref(),
        Some("opening websocket")
    );
    assert_eq!(
        normalize_status_detail("reusing websocket").as_deref(),
        Some("using existing websocket")
    );
    assert_eq!(
        normalize_status_detail("websocket healthcheck").as_deref(),
        Some("verifying websocket")
    );
    assert_eq!(
        normalize_status_detail("https fallback").as_deref(),
        Some("using https fallback")
    );
}

#[test]
fn collect_transport_context_labels_dedupes_overlapping_transport_text() {
    assert_eq!(
        collect_transport_context_labels(
            normalize_status_detail("reusing websocket"),
            Some(display_connection_type("websocket/persistent-reuse")),
            Some("OpenRouter".to_string())
        ),
        vec![
            "using existing websocket".to_string(),
            "via OpenRouter".to_string()
        ]
    );

    assert_eq!(
        collect_transport_context_labels(
            normalize_status_detail("https fallback"),
            Some(display_connection_type("https/sse")),
            None,
        ),
        vec!["using https fallback".to_string()]
    );
}

#[test]
fn composer_mode_detects_shell_input_before_commands() {
    assert_eq!(
        composer_mode(" ! cargo test ", false),
        ComposerMode::ShellLocal
    );
    assert_eq!(
        composer_mode("! cargo test", true),
        ComposerMode::ShellRemote
    );
    assert_eq!(composer_mode(" /help", false), ComposerMode::SlashCommand);
    assert_eq!(composer_mode("hello", false), ComposerMode::Chat);
}

#[test]
fn shell_mode_hint_reflects_execution_target() {
    assert_eq!(
        shell_mode_hint(ComposerMode::ShellLocal),
        Some("  shell mode · Enter runs locally")
    );
    assert_eq!(
        shell_mode_hint(ComposerMode::ShellRemote),
        Some("  shell mode · Enter runs on server")
    );
    assert_eq!(shell_mode_hint(ComposerMode::Chat), None);
}

#[test]
fn shell_mode_color_is_distinct() {
    assert_eq!(shell_mode_color(), rgb(110, 214, 151));
}

#[test]
fn normalize_repaint_sensitive_notice_text_drops_warning_variation_selector() {
    assert_eq!(
        normalize_repaint_sensitive_notice_text("⚠️ File activity: read lines 1-9"),
        "⚠ File activity: read lines 1-9"
    );
    assert_eq!(
        normalize_repaint_sensitive_notice_text("all clear"),
        "all clear"
    );
}
