#[test]
fn test_copy_selection_drag_near_top_edge_keeps_auto_scrolling() {
    // Regression: holding the cursor *near* (not exactly on) the top boundary
    // row used to fall outside the edge trigger, which disarmed the continuous
    // autoscroll. The drag then only nudged one step per mouse movement and
    // stalled entirely while the cursor was held still. A small browser-style
    // "hot zone" band near each edge keeps the autoscroll armed so the
    // transcript keeps scrolling while the mouse is held anywhere near the edge.
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    let lines = (1..=200)
        .map(|idx| format!("line {idx:03}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.display_messages = vec![DisplayMessage {
        role: "assistant".to_string(),
        content: lines,
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: None,
    }];
    app.bump_display_messages_version();
    app.scroll_offset = 0;
    app.auto_scroll_paused = false;
    app.is_processing = false;
    app.streaming.streaming_text.clear();
    app.status = ProcessingStatus::Idle;

    let backend = ratatui::backend::TestBackend::new(60, 16);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    app.handle_key(KeyCode::Char('y'), KeyModifiers::ALT)
        .unwrap();

    let layout = crate::tui::ui::last_layout_snapshot().expect("layout snapshot");
    let area = layout.messages_area;
    let lower_row = area.y + area.height / 2;
    let col = area.x + 1;
    // One row *inside* the top boundary: this is the spot that used to fail.
    let near_top_row = area.y + 1;
    assert!(
        near_top_row > area.y,
        "test must drag strictly inside the top boundary row"
    );

    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row: lower_row,
        modifiers: KeyModifiers::empty(),
    });

    let before = app.scroll_offset();
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: col,
        row: near_top_row,
        modifiers: KeyModifiers::empty(),
    });
    assert!(
        app.scroll_offset() < before,
        "drag near (not on) the top edge should auto-scroll up (before={before}, after={})",
        app.scroll_offset()
    );

    // The autoscroll must stay armed so holding the cursor still keeps pulling in
    // more transcript on subsequent ticks (the original bug stalled here).
    assert!(
        crate::tui::TuiState::copy_selection_edge_autoscroll_active(&app),
        "holding near the top edge should keep the continuous autoscroll armed"
    );

    let mut prev = app.scroll_offset();
    for _ in 0..5 {
        if prev == 0 {
            break;
        }
        assert!(app.progress_copy_selection_edge_autoscroll());
        assert!(
            app.scroll_offset() < prev,
            "held-still tick near the edge should keep scrolling up (prev={prev}, now={})",
            app.scroll_offset()
        );
        prev = app.scroll_offset();
    }

    // Releasing the mouse stops the continuous autoscroll.
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: col,
        row: near_top_row,
        modifiers: KeyModifiers::empty(),
    });
    assert!(!crate::tui::TuiState::copy_selection_edge_autoscroll_active(&app));
}

#[test]
fn test_copy_selection_drag_to_bottom_edge_when_pinned_does_not_snap_or_autoscroll() {
    // Regression: when the transcript is already pinned to the bottom (the common
    // case), dragging a selection into the bottom edge "hot zone" used to always
    // snap the cursor to the very last visible line and arm a downward autoscroll,
    // even though there is nothing more below to scroll into. That made it
    // impossible to precisely highlight the bottom rows: the selection kept
    // jumping to the end. With nothing to scroll, the edge band must stay inert so
    // the selection lands on the exact line under the cursor.
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    // Tall transcript pinned to the bottom: the bottom rows of the pane are
    // filled with real content, and there is nothing below to scroll into.
    let lines = (1..=200)
        .map(|idx| format!("line {idx:03}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.display_messages = vec![DisplayMessage {
        role: "assistant".to_string(),
        content: lines,
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: None,
    }];
    app.bump_display_messages_version();
    app.scroll_offset = 0;
    app.auto_scroll_paused = false;
    app.is_processing = false;
    app.streaming.streaming_text.clear();
    app.status = ProcessingStatus::Idle;

    let backend = ratatui::backend::TestBackend::new(60, 16);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    app.handle_key(KeyCode::Char('y'), KeyModifiers::ALT)
        .unwrap();

    let (visible_start, visible_end) =
        crate::tui::ui::copy_viewport_visible_range().expect("visible copy range");
    let line_count = crate::tui::ui::copy_viewport_line_count().expect("line count");
    assert_eq!(
        visible_end, line_count,
        "test precondition: view must be pinned to the bottom with no content below"
    );
    assert!(
        visible_start > 0,
        "test precondition: tall transcript must have content scrolled above the view"
    );

    let layout = crate::tui::ui::last_layout_snapshot().expect("layout snapshot");
    let area = layout.messages_area;
    let col = area.x + 1;

    // Pick a real content line near (but not at) the bottom to target.
    let target_line = visible_end.saturating_sub(2);
    assert!(target_line >= visible_start, "need a visible target line");
    let target_row = area.y + (target_line - visible_start) as u16;
    // The bottom edge band covers the last few rows; target_row must sit inside
    // it for this regression to be meaningful.
    let last_row = area.y + area.height - 1;
    assert!(
        target_row >= last_row.saturating_sub(2),
        "target line must fall within the bottom edge hot zone"
    );

    // Anchor higher up in the viewport.
    let anchor_row = area.y + 1;
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row: anchor_row,
        modifiers: KeyModifiers::empty(),
    });
    let before_scroll = app.scroll_offset();

    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: col,
        row: target_row,
        modifiers: KeyModifiers::empty(),
    });

    // No autoscroll should be armed: there is nothing below to pull in.
    assert!(
        !crate::tui::TuiState::copy_selection_edge_autoscroll_active(&app),
        "edge autoscroll must not arm when pinned to the bottom with no content below"
    );
    assert_eq!(
        app.scroll_offset(),
        before_scroll,
        "dragging into the bottom band while pinned must not scroll"
    );

    // The selection end should land on the exact line under the cursor, not snap
    // to the very last line of the transcript.
    let range = app.normalized_copy_selection().expect("normalized range");
    assert_eq!(
        range.end.abs_line, target_line,
        "selection should extend to the line under the cursor, not snap to the last line"
    );

    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: col,
        row: target_row,
        modifiers: KeyModifiers::empty(),
    });
}

#[test]
fn test_copy_selection_drag_below_last_line_fully_selects_last_line() {
    // Dragging *past* the last content line (into the empty area below the
    // chat pane) should fully select that last line through its end, just like
    // native terminal and browser selection. The chat pane is sized to its
    // content, so a downward drag that overshoots reports a row at/below the
    // bottom boundary that maps to no line at all; that used to silently drop
    // the extension so the bottom line could never be fully highlighted.
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    let lines = (1..=6)
        .map(|idx| format!("line {idx:03}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.display_messages = vec![DisplayMessage {
        role: "assistant".to_string(),
        content: lines,
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: None,
    }];
    app.bump_display_messages_version();
    app.scroll_offset = 0;
    app.auto_scroll_paused = false;
    app.is_processing = false;
    app.streaming.streaming_text.clear();
    app.status = ProcessingStatus::Idle;

    // Tall terminal so there is empty space below the content-sized chat pane.
    let backend = ratatui::backend::TestBackend::new(60, 20);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    app.handle_key(KeyCode::Char('y'), KeyModifiers::ALT)
        .unwrap();

    let (visible_start, visible_end) =
        crate::tui::ui::copy_viewport_visible_range().expect("visible copy range");
    let line_count = crate::tui::ui::copy_viewport_line_count().expect("line count");
    assert_eq!(visible_end, line_count, "view must be pinned to the bottom");

    // The last line that maps to a real screen point.
    let last_line = (visible_start..visible_end)
        .rev()
        .find(|&ln| {
            crate::tui::ui::copy_viewport_line_text(ln)
                .map(|t| unicode_width::UnicodeWidthStr::width(t.as_str()) > 0)
                .unwrap_or(false)
        })
        .expect("a non-empty visible content line");
    let last_text = crate::tui::ui::copy_viewport_line_text(last_line).unwrap_or_default();
    let last_width = unicode_width::UnicodeWidthStr::width(last_text.as_str());

    let layout = crate::tui::ui::last_layout_snapshot().expect("layout snapshot");
    let area = layout.messages_area;

    // Anchor on a valid cell at the START of the last content line.
    let last_content_row = area.y + (last_line - visible_start) as u16;
    let anchor_x = (area.x..area.x + area.width)
        .find(|&x| {
            crate::tui::ui::copy_viewport_point_from_screen(x, last_content_row)
                .map(|p| p.abs_line == last_line)
                .unwrap_or(false)
        })
        .expect("a screen column mapping to the last content line");
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: anchor_x,
        row: last_content_row,
        modifiers: KeyModifiers::empty(),
    });

    // Drag straight down, past the bottom of the pane, with the cursor x landing
    // partway through (not at the end of) the last line. Even so the whole last
    // line should be selected, because we have overshot it vertically.
    let mid_x = anchor_x + 1;
    let below_row = (area.y + area.height + 2).min(terminal.backend().size().unwrap().height - 1);
    assert!(
        below_row > last_content_row,
        "test must drag strictly below the last content row"
    );
    let before_scroll = app.scroll_offset();
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: mid_x,
        row: below_row,
        modifiers: KeyModifiers::empty(),
    });

    // No autoscroll (nothing below), and no scroll movement.
    assert!(
        !crate::tui::TuiState::copy_selection_edge_autoscroll_active(&app),
        "edge autoscroll must not arm dragging past the last line"
    );
    assert_eq!(app.scroll_offset(), before_scroll, "must not scroll");

    // The selection should now extend through the END of the last line.
    let range = app.normalized_copy_selection().expect("normalized range");
    assert_eq!(
        range.end.abs_line, last_line,
        "selection should extend to the last content line"
    );
    assert_eq!(
        range.end.column, last_width,
        "selection should cover the full last line (through its end)"
    );
    let selected = app
        .current_copy_selection_text()
        .expect("expected selection text");
    assert!(
        selected.contains(last_text.trim_end()),
        "selection should include the full last line text: got {selected:?}"
    );

    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: mid_x,
        row: below_row,
        modifiers: KeyModifiers::empty(),
    });
}

#[test]
fn test_alt_a_copies_chat_viewport_with_context_when_input_empty() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    let lines = (1..=20)
        .map(|idx| format!("line {idx:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.display_messages = vec![DisplayMessage {
        role: "assistant".to_string(),
        content: lines,
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: None,
    }];
    app.bump_display_messages_version();
    app.scroll_offset = 4;
    app.auto_scroll_paused = true;

    let backend = ratatui::backend::TestBackend::new(40, 8);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let handled = super::input::handle_alt_key(&mut app, KeyCode::Char('a'));
    assert!(handled);
    assert!(matches!(
        app.status_notice().as_deref(),
        Some("Copied viewport context")
            | Some("Failed to copy viewport context")
            | Some("Nothing visible to copy")
    ));
}

#[test]
fn test_changelog_overlay_supports_drag_select_and_copy() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.changelog_scroll = Some(0);

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    // The overlay must register a chat-pane copy snapshot so the shared
    // selection machinery can map screen coordinates to text.
    let (visible_start, visible_end) = crate::tui::ui::copy_viewport_visible_range()
        .expect("changelog overlay should register a copy viewport snapshot");
    assert!(visible_end > visible_start);

    // Find a non-empty rendered line to select.
    let (line_idx, line_text) = (visible_start..visible_end)
        .find_map(|abs_line| {
            let text = crate::tui::ui::copy_viewport_line_text(abs_line)?;
            (!text.trim().is_empty()).then_some((abs_line, text))
        })
        .expect("expected a visible non-empty changelog line");

    app.copy_selection_anchor = Some(crate::tui::CopySelectionPoint {
        pane: crate::tui::CopySelectionPane::Chat,
        abs_line: line_idx,
        column: 0,
    });
    app.copy_selection_cursor = Some(crate::tui::CopySelectionPoint {
        pane: crate::tui::CopySelectionPane::Chat,
        abs_line: line_idx,
        column: unicode_width::UnicodeWidthStr::width(line_text.as_str()),
    });

    let selected = app
        .current_copy_selection_text()
        .expect("expected selection text from changelog overlay");
    assert_eq!(selected, line_text);
}

#[test]
fn test_changelog_overlay_mouse_drag_release_copies_text() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.changelog_scroll = Some(0);

    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let (visible_start, visible_end) = crate::tui::ui::copy_viewport_visible_range()
        .expect("changelog overlay should register a copy viewport snapshot");
    let line_idx = (visible_start..visible_end)
        .find(|&abs_line| {
            crate::tui::ui::copy_viewport_line_text(abs_line).is_some_and(|t| !t.trim().is_empty())
        })
        .expect("expected a visible non-empty changelog line");

    // Resolve a screen row for that line via the recorded snapshot.
    let mut found_row = None;
    for row in 0..24u16 {
        for col in 0..80u16 {
            if let Some(point) = crate::tui::ui::copy_point_from_screen(col, row)
                && point.pane == crate::tui::CopySelectionPane::Chat
                && point.abs_line == line_idx
            {
                found_row = Some(row);
                break;
            }
        }
        if found_row.is_some() {
            break;
        }
    }
    let row = found_row.expect("expected a screen row mapping to the changelog line");

    // Press, drag across the line, and release: this should select and attempt a copy.
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 2,
        row,
        modifiers: KeyModifiers::empty(),
    });
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 40,
        row,
        modifiers: KeyModifiers::empty(),
    });
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 40,
        row,
        modifiers: KeyModifiers::empty(),
    });

    // A copy was attempted (success/failure depends on clipboard availability
    // in the test environment, but the selection path must have run).
    assert!(matches!(
        app.status_notice().as_deref(),
        Some("Copied selection")
            | Some("Copied selection · highlight remains visible")
            | Some("Failed to copy selection")
            | Some("Selection is empty")
    ));
}
