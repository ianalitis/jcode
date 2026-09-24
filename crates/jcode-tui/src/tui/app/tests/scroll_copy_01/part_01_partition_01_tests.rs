#[test]
fn test_remote_typing_scroll_lock_can_be_toggled_back_off() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.scroll_offset = 7;
    app.auto_scroll_paused = true;

    rt.block_on(app.handle_remote_key(KeyCode::Char('s'), KeyModifiers::ALT, &mut remote))
        .unwrap();
    rt.block_on(app.handle_remote_key(KeyCode::Char('s'), KeyModifiers::ALT, &mut remote))
        .unwrap();
    app.handle_remote_char_input('x');

    assert_eq!(app.scroll_offset, 0);
    assert!(
        !app.auto_scroll_paused,
        "typing should resume following chat bottom after disabling the lock"
    );
}

#[test]
fn test_should_allow_reconnect_takeover_only_after_successful_attach() {
    let mut app = create_test_app();
    let state = super::remote::RemoteRunState {
        reconnect_attempts: 1,
        ..Default::default()
    };

    app.resume_session_id = Some("ses_resume_only".to_string());
    assert!(!super::remote::should_allow_reconnect_takeover(
        &app,
        &state,
        app.resume_session_id.as_deref(),
    ));

    app.remote_session_id = Some("ses_other".to_string());
    assert!(!super::remote::should_allow_reconnect_takeover(
        &app,
        &state,
        app.resume_session_id.as_deref(),
    ));

    app.remote_session_id = Some("ses_resume_only".to_string());
    assert!(super::remote::should_allow_reconnect_takeover(
        &app,
        &state,
        app.resume_session_id.as_deref(),
    ));
    assert!(!super::remote::should_allow_reconnect_takeover(
        &app,
        &super::remote::RemoteRunState::default(),
        app.resume_session_id.as_deref(),
    ));
    assert!(!super::remote::should_allow_reconnect_takeover(
        &app, &state, None,
    ));
}

#[test]
fn test_reconnect_target_prefers_remote_session_id() {
    let mut app = create_test_app();
    app.resume_session_id = Some("ses_resume_idle".to_string());
    app.remote_session_id = Some("ses_remote_active".to_string());

    assert_eq!(
        app.reconnect_target_session_id().as_deref(),
        Some("ses_remote_active")
    );
}

#[test]
fn test_reconnect_target_uses_resume_when_remote_missing() {
    let mut app = create_test_app();
    app.resume_session_id = Some("ses_resume_only".to_string());
    app.remote_session_id = None;

    assert_eq!(
        app.reconnect_target_session_id().as_deref(),
        Some("ses_resume_only")
    );
}

#[test]
fn test_reconnect_target_does_not_consume_resume_session_id() {
    let mut app = create_test_app();
    app.resume_session_id = Some("ses_resume_persistent".to_string());
    app.remote_session_id = None;

    let first = app.reconnect_target_session_id();
    let second = app.reconnect_target_session_id();

    assert_eq!(first.as_deref(), Some("ses_resume_persistent"));
    assert_eq!(second.as_deref(), Some("ses_resume_persistent"));
    assert_eq!(
        app.resume_session_id.as_deref(),
        Some("ses_resume_persistent")
    );
}

#[test]
fn test_prompt_jump_ctrl_brackets() {
    let _render_lock = scroll_render_test_lock();
    let (mut app, mut terminal) = create_scroll_test_app(100, 30, 1, 20);

    // Seed max scroll estimates before key handling.
    render_and_snap(&app, &mut terminal);

    assert_eq!(app.scroll_offset, 0);
    assert!(!app.auto_scroll_paused);

    app.handle_key(KeyCode::Char('['), KeyModifiers::CONTROL)
        .unwrap();
    assert!(app.auto_scroll_paused);
    assert!(app.scroll_offset > 0);

    let after_up = app.scroll_offset;
    app.handle_key(KeyCode::Char(']'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(app.scroll_offset <= after_up);
}

// NOTE: test_prompt_jump_ctrl_digits_by_recency was removed because it relied on
// pre-render prompt positions that no longer exist. The render-based version
// test_prompt_jump_ctrl_digit_is_recency_rank_in_app covers this functionality.

#[cfg(target_os = "macos")]
#[test]
fn test_prompt_jump_ctrl_esc_fallback_on_macos() {
    let _render_lock = scroll_render_test_lock();
    let (mut app, mut terminal) = create_scroll_test_app(100, 30, 1, 20);

    render_and_snap(&app, &mut terminal);

    assert_eq!(app.scroll_offset, 0);
    app.handle_key(KeyCode::Esc, KeyModifiers::CONTROL).unwrap();
    assert!(app.auto_scroll_paused);
    assert!(app.scroll_offset > 0);
}

#[test]
fn test_ctrl_digit_side_panel_preset_in_app() {
    let mut app = create_test_app();

    app.handle_key(KeyCode::Char('1'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.diagram_pane_ratio_target, 25);

    app.handle_key(KeyCode::Char('2'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.diagram_pane_ratio_target, 50);

    app.handle_key(KeyCode::Char('3'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.diagram_pane_ratio_target, 75);

    app.handle_key(KeyCode::Char('4'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.diagram_pane_ratio_target, 100);
}

#[test]
fn test_chat_overscroll_reveals_status_line_then_rebounds() {
    let _lock = scroll_render_test_lock();

    let (mut app, mut terminal) = create_scroll_test_app(80, 14, 0, 36);
    // Exercise the elastic reveal explicitly (the default pins the line on).
    app.overscroll_status_mode = crate::config::OverscrollStatusMode::Overscroll;

    // Give the app some context so the overscroll line has a percentage to show.
    app.context_info = crate::prompt::ContextInfo {
        total_chars: 40_000,
        ..Default::default()
    };
    app.context_limit = 200_000;

    // Pinned to the bottom: no overscroll line yet. (The idle status line now
    // renders its own short ▰▱ context bar, so the overscroll-specific
    // affordance to assert on is the `(overscroll x.x)` countdown, not the
    // glyphs alone.)
    let pinned = render_and_snap(&app, &mut terminal);
    assert!(
        !app.chat_overscroll_active(),
        "should start without overscroll"
    );
    assert!(
        !pinned.contains("(overscroll"),
        "overscroll countdown should be hidden while pinned: {pinned:?}"
    );

    // Scroll down at the bottom => overscroll registered, line revealed.
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::empty(),
    });
    assert!(
        app.chat_overscroll_active(),
        "overscroll should be active after scrolling down at the bottom"
    );
    let revealed = render_and_snap(&app, &mut terminal);
    assert!(
        revealed.contains("(overscroll"),
        "overscroll status line should show the countdown affordance: {revealed:?}"
    );

    // Scrolling up cancels the overscroll line immediately.
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::empty(),
    });
    assert!(
        !app.chat_overscroll_active(),
        "scrolling up should cancel the overscroll line"
    );
}

#[test]
fn renderer_publishes_the_prepared_frame_as_geometry() {
    let _lock = scroll_render_test_lock();
    let (app, mut terminal) = create_scroll_test_app(100, 30, 0, 60);
    render_and_snap(&app, &mut terminal);

    // The retained frame *is* the published geometry: its total must agree with
    // the scalar the rest of the code reads, and its section ranges must tile
    // the wrapped row vector with no gaps so an anchor can index into it.
    let frame = crate::tui::ui::last_chat_frame().expect("frame published after a render");
    assert_eq!(
        frame.total_wrapped_lines(),
        crate::tui::ui::last_total_wrapped_lines()
    );
    let mut next_start = 0;
    for section in &frame.sections {
        assert_eq!(
            section.line_start, next_start,
            "section ranges must be contiguous"
        );
        next_start += section.prepared.wrapped_lines.len();
    }
    assert_eq!(next_start, frame.total_wrapped_lines());

    // A narrower window re-lays the frame out: same handle, new ranges.
    let mut narrow = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 30)).unwrap();
    render_and_snap(&app, &mut narrow);
    let narrow_frame = crate::tui::ui::last_chat_frame().expect("frame published after a render");
    assert_eq!(
        narrow_frame.total_wrapped_lines(),
        crate::tui::ui::last_total_wrapped_lines()
    );
    assert!(
        narrow_frame.total_wrapped_lines() > frame.total_wrapped_lines(),
        "narrowing must wrap into more rows: {} vs {}",
        narrow_frame.total_wrapped_lines(),
        frame.total_wrapped_lines()
    );
}

#[test]
fn retained_frame_row_matches_the_rendered_screen() {
    // Integration check across the draw boundary: a consumer outside `draw`
    // resolves a row index against the retained frame, so that row has to be
    // what is actually rendered at the top of the chat viewport.
    let _lock = scroll_render_test_lock();
    let (mut app, mut terminal) = create_scroll_test_app(100, 30, 0, 60);
    app.auto_scroll_paused = false;
    render_and_snap(&app, &mut terminal);
    app.scroll_up(20);
    render_and_snap(&app, &mut terminal);

    let scroll = crate::tui::ui::last_resolved_chat_scroll();
    assert!(scroll > 0, "fixture must be scrolled into history");
    let frame = crate::tui::ui::last_chat_frame().expect("frame published after a render");
    let top_row = frame
        .wrapped_plain_line(scroll)
        .expect("resolved row is in range")
        .trim()
        .to_string();

    let area = crate::tui::ui::last_layout_snapshot()
        .expect("layout snapshot")
        .messages_area;
    let first_chat_line = buffer_to_text(&terminal)
        .lines()
        .skip(area.y as usize)
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();

    assert!(!top_row.is_empty(), "frame row must carry text");
    assert_eq!(
        first_chat_line, top_row,
        "the retained frame's row must be the line rendered at the top of the viewport"
    );
}

/// `Ctrl+5` must reach the recency-rank jump on macOS like it does elsewhere.
///
/// A legacy tty encodes `Ctrl+]` as `0x1D`, which crossterm decodes as
/// `Ctrl+5`. jcode used to rewrite that back to `Ctrl+]` on macOS, which also
/// swallowed the real `Ctrl+5` and made rank 5 the one unreachable entry in the
/// help overlay's documented `Ctrl+5..9` range.
#[cfg(target_os = "macos")]
#[test]
fn test_ctrl_5_is_a_prompt_rank_jump_on_macos() {
    let _render_lock = scroll_render_test_lock();
    let (mut app, mut terminal) = create_scroll_test_app(100, 30, 1, 20);

    render_and_snap(&app, &mut terminal);

    assert_eq!(app.scroll_offset, 0);
    assert!(!app.auto_scroll_paused);

    app.handle_key(KeyCode::Char('5'), KeyModifiers::CONTROL)
        .unwrap();

    // A rank jump scrolls to a prompt and pins the viewport there. The old
    // `Ctrl+]` rewrite instead moved *forward* toward the tail, which from the
    // bottom is a no-op and leaves the offset at 0.
    assert!(
        app.auto_scroll_paused,
        "Ctrl+5 should jump to a prompt, not fall through to a next-prompt scroll"
    );
    assert!(app.scroll_offset > 0);
}

