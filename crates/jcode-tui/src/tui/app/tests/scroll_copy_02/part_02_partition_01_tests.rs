#[test]
fn test_mouse_click_in_wrapped_input_moves_cursor_to_second_visual_line() {
    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    // Keep first-run suggestions from replacing the composer under test.
    app.push_display_message(DisplayMessage::assistant("seed transcript"));
    app.diagram_mode = crate::config::DiagramDisplayMode::None;
    app.diagram_pane_enabled = false;
    app.input = "abcdefghij".to_string();
    app.cursor_pos = 0;
    app.set_centered(false);
    app.session.short_name = Some("test".to_string());

    let backend = ratatui::backend::TestBackend::new(11, 16);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    render_and_snap(&app, &mut terminal);

    let layout = crate::tui::ui::last_layout_snapshot().expect("layout snapshot");
    let input_area = layout.input_area.expect("input area");

    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: input_area.x + 4,
        row: input_area.y + 1,
        modifiers: KeyModifiers::empty(),
    });

    // The idle composer no longer reserves space for the old send-mode glyph,
    // so this 11-column input wraps after eight characters. Column four on the
    // second visual line is one character into that segment.
    assert_eq!(app.cursor_pos, 9);
}

/// End-to-end: a real left-click on an inline image's label line maps the
/// screen point back through a recorded `ChatFrame` snapshot to the image id and
/// cycles its expand level. This exercises the full click path
/// (`handle_mouse_event` -> `try_cycle_image_expand_at` ->
/// `inline_image_expand_target_from_screen` -> `cycle_image_expand`), not just
/// the isolated helpers.
#[test]
fn test_click_on_inline_image_label_line_cycles_level() {
    use crate::tui::ui::inline_image_ui::{
        AllFit, ImageExpandLevel, InlineImageItem, build_section,
    };
    use jcode_tui_messages::PreparedChatFrame;

    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    const IMAGE_ID: u64 = 0xFEED;
    let chat_width: u16 = 80;

    // Build a real inline-image section: a `shot.png … hide` label line
    // followed by Fit-rendered placeholder rows with a scanned `image_regions`
    // entry.
    let items = vec![InlineImageItem {
        id: IMAGE_ID,
        width: 600,
        height: 400,
        label: "shot.png".to_string(),
        uses_text_fallback: true,
    }];
    let section = build_section(&items, chat_width, 40, false, true, &AllFit);

    // Locate the label line (the one carrying the image label); the whole line
    // is the click target now that the expand badge is gone.
    let label_line = section
        .wrapped_plain_lines
        .iter()
        .position(|line| line.contains("shot.png"))
        .expect("section should contain the image label line");

    // Even with the terminal fallback note attached below the image, the Fit
    // region must remain exactly one line below the label. This adjacency is how
    // `inline_image_id_for_label_line` maps a click back to the image.
    assert!(
        section
            .image_regions
            .iter()
            .any(|r| r.hash == IMAGE_ID && r.abs_line_idx == label_line + 1),
        "expected a Fit image region anchored under the label line"
    );

    let prepared =
        std::sync::Arc::new(PreparedChatFrame::from_single(std::sync::Arc::new(section)));
    let visible_end = prepared.wrapped_plain_line_count();
    let content_area = Rect::new(0, 0, chat_width, visible_end as u16 + 1);

    crate::tui::ui::clear_copy_viewport_snapshot();
    crate::tui::ui::record_copy_viewport_frame_snapshot_for_test(
        prepared,
        0,
        visible_end,
        content_area,
        &vec![0u16; visible_end],
    );

    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Fit,
        "image should start at Fit"
    );

    // Click the label line (button up is what fires the cycle).
    let handled = app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: content_area.x + 2,
        row: content_area.y + label_line as u16,
        modifiers: KeyModifiers::empty(),
    });
    assert!(!handled, "handled click should request an immediate redraw");
    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Large,
        "first label click should expand Fit -> Large"
    );
    assert_eq!(app.status_notice(), Some("Image size: large".to_string()));

    // Large and Full have identical geometry for this landscape image, so the
    // redundant Full state is skipped and the next click returns to Fit.
    let click_label = |app: &mut App| {
        app.handle_mouse_event(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: content_area.x + 2,
            row: content_area.y + label_line as u16,
            modifiers: KeyModifiers::empty(),
        });
    };
    click_label(&mut app);
    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Fit,
        "second click should skip duplicate Full geometry and return to Fit"
    );
}

/// Kitty reports mouse motion at pixel granularity, so a physically plain
/// click usually arrives as Down -> Drag(same cell) -> Up. The same-cell Drag
/// must NOT start a selection drag; the release must still fall through to the
/// label-line click handler. Regression test for "click does nothing on
/// kitty".
#[test]
fn test_kitty_jitter_click_on_image_label_still_cycles_level() {
    use crate::tui::ui::inline_image_ui::{
        AllFit, ImageExpandLevel, InlineImageItem, build_section,
    };
    use jcode_tui_messages::PreparedChatFrame;

    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    const IMAGE_ID: u64 = 0xF00D;
    let chat_width: u16 = 80;
    let items = vec![InlineImageItem {
        id: IMAGE_ID,
        width: 600,
        height: 400,
        label: "shot.png".to_string(),
        uses_text_fallback: false,
    }];
    let section = build_section(&items, chat_width, 40, false, true, &AllFit);
    let label_line = section
        .wrapped_plain_lines
        .iter()
        .position(|line| line.contains("shot.png"))
        .expect("section should contain the image label line");
    let badge_col: u16 = 2;

    let prepared =
        std::sync::Arc::new(PreparedChatFrame::from_single(std::sync::Arc::new(section)));
    let visible_end = prepared.wrapped_plain_line_count();
    let content_area = Rect::new(0, 0, chat_width, visible_end as u16 + 1);

    crate::tui::ui::clear_copy_viewport_snapshot();
    crate::tui::ui::record_copy_viewport_frame_snapshot_for_test(
        prepared,
        0,
        visible_end,
        content_area,
        &vec![0u16; visible_end],
    );

    let (col, row) = (
        content_area.x + badge_col,
        content_area.y + label_line as u16,
    );
    let inject = |app: &mut App, kind: MouseEventKind| {
        app.handle_mouse_event(MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        });
    };

    // Down, same-cell Drag (kitty pixel jitter), Up: must count as a click.
    inject(&mut app, MouseEventKind::Down(MouseButton::Left));
    inject(&mut app, MouseEventKind::Drag(MouseButton::Left));
    inject(&mut app, MouseEventKind::Up(MouseButton::Left));

    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Large,
        "jitter click (down + same-cell drag + up) must still cycle the badge"
    );

    // A real drag to a DIFFERENT cell must still start a selection, not click.
    inject(&mut app, MouseEventKind::Down(MouseButton::Left));
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: col.saturating_sub(4),
        row,
        modifiers: KeyModifiers::empty(),
    });
    inject(&mut app, MouseEventKind::Up(MouseButton::Left));
    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Large,
        "a real drag ending on the badge must not fire the click handler"
    );
}

/// 1x1 transparent PNG: a real image header so the inline-image pipeline decodes
/// dimensions and assigns a stable id, exactly like a `read`-tool screenshot.
const REPRO_TINY_PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

/// FULL end-to-end reproduction of the user's "clicking the image does
/// nothing" report. Unlike `test_click_on_inline_image_label_line_cycles_level`
/// (which records a synthetic `ChatFrame` snapshot directly), this drives the
/// *real* draw: a local App whose session carries a `read`-tool result image,
/// anchored into the transcript body, rendered through `terminal.draw()`, which
/// is what records the live copy-viewport snapshot. We then locate the rendered
/// image label line in the actual frame buffer and inject a real left click,
/// asserting the image size cycles. This exercises the body-anchored image path
/// (`render_images` -> `resolve_anchored_items` -> `anchored_image_lines`), the
/// path actually used in production, not the isolated `build_section` helper.
#[test]
fn test_real_draw_click_on_body_anchored_image_label_cycles_level() {
    use crate::message::{ContentBlock, Role};
    use crate::tui::ui::inline_image_ui::ImageExpandLevel;

    let _render_lock = scroll_render_test_lock();
    let _ = super::helpers::take_copied_image_for_tests();
    let mut app = create_test_app();
    assert!(!app.is_remote, "repro must use the local image render path");

    const TOOL_ID: &str = "read-shot-1";

    // Build a real transcript: user asks, assistant calls `read`, tool result
    // carries the screenshot image. This is exactly what produces a
    // body-anchored inline image with a `RenderedImageAnchor::ToolCall`.
    app.session.add_message(
        Role::User,
        vec![ContentBlock::Text {
            text: "read the screenshot".to_string(),
            cache_control: None,
        }],
    );
    app.session.add_message(
        Role::Assistant,
        vec![ContentBlock::ToolUse {
            id: TOOL_ID.to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"file_path": "shot.png"}),
            thought_signature: None,
        }],
    );
    app.session.add_message(
        Role::User,
        vec![
            ContentBlock::ToolResult {
                tool_use_id: TOOL_ID.to_string(),
                content: "read image".to_string(),
                is_error: None,
            },
            ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: REPRO_TINY_PNG_B64.to_string(),
            },
        ],
    );

    // Mirror the session into the display transcript the body renderer walks.
    app.display_messages = vec![
        DisplayMessage::user("read the screenshot"),
        DisplayMessage::tool(
            "read shot.png",
            crate::message::ToolCall {
                id: TOOL_ID.to_string(),
                name: "read".to_string(),
                input: serde_json::json!({"file_path": "shot.png"}),
                intent: None,
                thought_signature: None,
            },
        ),
    ];
    app.bump_display_messages_version();
    app.invalidate_side_pane_images_signature();
    app.pin_images = true;
    app.inline_images_visible = true;
    app.scroll_offset = 0;
    app.auto_scroll_paused = false;
    app.is_processing = false;
    app.status = ProcessingStatus::Idle;
    app.session.short_name = Some("test".to_string());

    // Sanity: the local render path must actually surface the anchored image.
    let images = <App as crate::tui::TuiState>::side_pane_images(&app);
    assert_eq!(
        images.len(),
        1,
        "session should render exactly one anchored tool image"
    );
    let image_id = {
        let img = &images[0];
        crate::tui::mermaid::inline_image_dims(&img.media_type, &img.data)
            .expect("tiny png should decode")
            .0
    };

    let backend = ratatui::backend::TestBackend::new(80, 40);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");

    // REAL draw: this records the live copy-viewport snapshot used by clicks.
    let rendered = render_and_snap(&app, &mut terminal);
    assert!(
        rendered.contains("shot.png"),
        "image label line must render in the live frame, got:\n{rendered}"
    );

    // Find the label line in the actual buffer: scan rows for the row carrying
    // the image label, then click a cell inside the label text.
    let buf = terminal.backend().buffer();
    let area = *buf.area();
    let mut badge: Option<(u16, u16)> = None;
    'rows: for row in 0..area.height {
        let mut line = String::new();
        for col in 0..area.width {
            line.push_str(buf[(col, row)].symbol());
        }
        // The transcript also shows the tool-call row ("read shot.png"); the
        // image label row is the one that carries the show/hide badge keys.
        if !line.contains("shot.png") || !line.contains("[I]") {
            continue;
        }
        // Click the first cell of the label text (the hit-region is the whole
        // label line, so any cell on the row works).
        for col in 0..area.width {
            if buf[(col, row)].symbol() == "s" {
                badge = Some((col, row));
                break 'rows;
            }
        }
    }
    let (badge_col, badge_row) = badge.expect("image label cell should be visible in the frame");

    assert_eq!(
        app.image_expand_level(image_id),
        ImageExpandLevel::Fit,
        "image should start at Fit before any click"
    );

    // REAL click on the rendered label cell. A terminal delivers a *pair* of
    // events for one physical click: `Down` then `Up`. We must replay both, just
    // like the live event loop, or we silently skip the copy-selection state the
    // `Down` arms (which is exactly what the user's click goes through).
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: badge_col,
        row: badge_row,
        modifiers: KeyModifiers::empty(),
    });
    app.handle_mouse_event(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: badge_col,
        row: badge_row,
        modifiers: KeyModifiers::empty(),
    });

    assert_eq!(
        app.image_expand_level(image_id),
        ImageExpandLevel::Large,
        "clicking the rendered image label must cycle Fit -> Large \
         (this is the exact path the user reported as broken)"
    );
    assert_eq!(
        app.status_notice(),
        Some("Image size: large · Image copied".to_string())
    );
    assert_eq!(
        super::helpers::take_copied_image_for_tests(),
        Some(("image/png".to_string(), REPRO_TINY_PNG_B64.to_string()))
    );
}

/// The inline-image placeholder marker row must never reach the terminal as
/// text. It used to be drawn black-on-black and relied on staying invisible,
/// but terminal-side compositing (kitty translucent background + contrast
/// compositing) and selection highlighting can recolor it, leaking raw
/// "IIMG:<hash>:..." into the transcript whenever the image is not painted
/// over it (cold cache after reload, prewarm in flight, no image protocol).
/// The draw path must blank marker rows instead.
#[test]
fn test_real_draw_never_emits_inline_image_marker_text() {
    use crate::message::{ContentBlock, Role};

    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();
    assert!(!app.is_remote, "repro must use the local image render path");

    const TOOL_ID: &str = "read-shot-marker";

    app.session.add_message(
        Role::User,
        vec![ContentBlock::Text {
            text: "read the screenshot".to_string(),
            cache_control: None,
        }],
    );
    app.session.add_message(
        Role::Assistant,
        vec![ContentBlock::ToolUse {
            id: TOOL_ID.to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"file_path": "shot.png"}),
            thought_signature: None,
        }],
    );
    app.session.add_message(
        Role::User,
        vec![
            ContentBlock::ToolResult {
                tool_use_id: TOOL_ID.to_string(),
                content: "read image".to_string(),
                is_error: None,
            },
            ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: REPRO_TINY_PNG_B64.to_string(),
            },
        ],
    );

    app.display_messages = vec![
        DisplayMessage::user("read the screenshot"),
        DisplayMessage::tool(
            "read shot.png",
            crate::message::ToolCall {
                id: TOOL_ID.to_string(),
                name: "read".to_string(),
                input: serde_json::json!({"file_path": "shot.png"}),
                intent: None,
                thought_signature: None,
            },
        ),
    ];
    app.bump_display_messages_version();
    app.invalidate_side_pane_images_signature();
    app.pin_images = true;
    app.inline_images_visible = true;
    app.scroll_offset = 0;
    app.auto_scroll_paused = false;
    app.is_processing = false;
    app.status = ProcessingStatus::Idle;
    app.session.short_name = Some("test".to_string());

    let backend = ratatui::backend::TestBackend::new(80, 40);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    let rendered = render_and_snap(&app, &mut terminal);

    assert!(
        rendered.contains("shot.png"),
        "sanity: the anchored image's label line must render, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("IIMG"),
        "raw inline-image marker text must never be drawn to the terminal, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("MERMAID_IMAGE"),
        "raw mermaid marker text must never be drawn to the terminal, got:\n{rendered}"
    );
}

/// Clicking anywhere on the image body (its placeholder rows) must cycle the
/// expand level, exactly like the label badge. Clicks in the blank area to
/// the RIGHT of a narrow image must not.
#[test]
fn test_click_on_inline_image_body_cycles_level() {
    use crate::tui::ui::inline_image_ui::{
        AllFit, ImageExpandLevel, InlineImageItem, build_section,
    };
    use jcode_tui_messages::PreparedChatFrame;

    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    const IMAGE_ID: u64 = 0xBEEF;
    let chat_width: u16 = 80;

    let items = vec![InlineImageItem {
        id: IMAGE_ID,
        width: 320,
        height: 200,
        label: "shot.png".to_string(),
        uses_text_fallback: false,
    }];
    let section = build_section(&items, chat_width, 40, false, true, &AllFit);
    let region = *section
        .image_regions
        .iter()
        .find(|r| r.hash == IMAGE_ID)
        .expect("section should carry the image region");
    assert!(region.width > 0, "fit regions record their rendered width");
    assert!(
        region.width < chat_width,
        "test image must be narrower than the chat so the right side is blank"
    );

    let prepared =
        std::sync::Arc::new(PreparedChatFrame::from_single(std::sync::Arc::new(section)));
    let visible_end = prepared.wrapped_plain_line_count();
    let content_area = Rect::new(0, 0, chat_width, visible_end as u16 + 1);

    crate::tui::ui::clear_copy_viewport_snapshot();
    crate::tui::ui::record_copy_viewport_frame_snapshot_for_test(
        prepared,
        0,
        visible_end,
        content_area,
        &vec![0u16; visible_end],
    );

    assert_eq!(app.image_expand_level(IMAGE_ID), ImageExpandLevel::Fit);

    // Click in the middle of the image body (a placeholder row, inside the
    // rendered width). Down then Up, like a real terminal click.
    let body_row = content_area.y + region.abs_line_idx as u16 + 1;
    let body_col = content_area.x + region.width / 2;
    let click = |app: &mut App, col: u16, row: u16| {
        app.handle_mouse_event(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        });
        app.handle_mouse_event(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        });
    };
    click(&mut app, body_col, body_row);
    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Large,
        "clicking the image body should expand Fit -> Large"
    );

    // Large and Full resolve to the same geometry for this landscape image, so
    // the click cycle must omit Full rather than showing a duplicate size.
    click(&mut app, body_col, body_row);
    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Fit,
        "second body click should skip duplicate Full geometry and return to Fit"
    );

    // A click in the blank space to the right of the image must stay inert.
    let far_right = content_area.x + chat_width - 2;
    assert!(far_right > content_area.x + region.width);
    click(&mut app, far_right, body_row);
    assert_eq!(
        app.image_expand_level(IMAGE_ID),
        ImageExpandLevel::Fit,
        "clicking blank space beside the image must not cycle it"
    );
}

/// Ctrl+wheel (how terminals report a trackpad pinch) over an inline image
/// steps its size without wrapping, while Ctrl+wheel elsewhere still scrolls.
#[test]
fn test_ctrl_wheel_over_inline_image_zooms_without_wrapping() {
    use crate::tui::ui::inline_image_ui::{
        AllFit, ImageExpandLevel, InlineImageItem, build_section,
    };
    use jcode_tui_messages::PreparedChatFrame;

    let _render_lock = scroll_render_test_lock();
    let mut app = create_test_app();

    const IMAGE_ID: u64 = 0xB1AC;
    let chat_width: u16 = 80;
    let items = vec![InlineImageItem {
        id: IMAGE_ID,
        width: 320,
        height: 200,
        label: "pinch.png".to_string(),
        uses_text_fallback: false,
    }];
    let section = build_section(&items, chat_width, 40, false, true, &AllFit);
    let region = *section
        .image_regions
        .iter()
        .find(|r| r.hash == IMAGE_ID)
        .expect("section should carry the image region");
    let prepared =
        std::sync::Arc::new(PreparedChatFrame::from_single(std::sync::Arc::new(section)));
    let visible_end = prepared.wrapped_plain_line_count();
    let content_area = Rect::new(0, 0, chat_width, visible_end as u16 + 1);
    crate::tui::ui::clear_copy_viewport_snapshot();
    crate::tui::ui::record_copy_viewport_frame_snapshot_for_test(
        prepared,
        0,
        visible_end,
        content_area,
        &vec![0u16; visible_end],
    );

    let row = content_area.y + region.abs_line_idx as u16 + 1;
    let col = content_area.x + region.width / 2;
    let wheel = |app: &mut App, kind: MouseEventKind, column: u16| {
        app.handle_mouse_event(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::CONTROL,
        })
    };

    wheel(&mut app, MouseEventKind::ScrollUp, col);
    assert_eq!(app.image_expand_level(IMAGE_ID), ImageExpandLevel::Large);
    // Full has the same geometry as Large here, so zooming in further holds.
    wheel(&mut app, MouseEventKind::ScrollUp, col);
    assert_eq!(app.image_expand_level(IMAGE_ID), ImageExpandLevel::Large);
    wheel(&mut app, MouseEventKind::ScrollDown, col);
    assert_eq!(app.image_expand_level(IMAGE_ID), ImageExpandLevel::Fit);
    wheel(&mut app, MouseEventKind::ScrollDown, col);
    assert_eq!(app.image_expand_level(IMAGE_ID), ImageExpandLevel::Fit);

    // Off the image, Ctrl+wheel falls through to chat scrolling.
    wheel(&mut app, MouseEventKind::ScrollUp, chat_width - 2);
    assert_eq!(app.image_expand_level(IMAGE_ID), ImageExpandLevel::Fit);
}

fn create_math_copy_test_app() -> (App, ratatui::Terminal<ratatui::backend::TestBackend>) {
    let mut app = create_test_app();
    app.display_messages = vec![
        DisplayMessage {
            role: "user".to_string(),
            content: "show latex".to_string(),
            tool_calls: vec![],
            duration_secs: None,
            title: None,
            tool_data: None,
        },
        DisplayMessage {
            role: "assistant".to_string(),
            content: "Euler $e^{i\\pi_{3}} + 1 = 0$ done.\n\n$$\n\\frac{\\partial L}{\\partial \\theta_{88}} = \\theta^\\top x\n$$".to_string(),
            tool_calls: vec![],
            duration_secs: None,
            title: None,
            tool_data: None,
        },
    ];
    app.bump_display_messages_version();
    app.scroll_offset = 0;
    app.auto_scroll_paused = false;
    app.is_processing = false;
    app.streaming.streaming_text.clear();
    app.status = ProcessingStatus::Idle;
    app.session.short_name = Some("test".to_string());
    let backend = ratatui::backend::TestBackend::new(100, 30);
    let terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    (app, terminal)
}

#[test]
fn test_copy_selection_select_all_copies_rendered_math_as_latex() {
    let _render_lock = scroll_render_test_lock();
    let (mut app, mut terminal) = create_math_copy_test_app();

    let screen = render_and_snap(&app, &mut terminal);
    // Sanity: the screen shows the Unicode approximation, not the source.
    assert!(screen.contains("┌─ math"), "{screen}");
    assert!(screen.contains('⊤'), "\\top should render as ⊤: {screen}");
    assert!(!screen.contains("\\frac"), "{screen}");

    app.handle_key(KeyCode::Char('y'), KeyModifiers::ALT)
        .unwrap();
    assert!(app.select_all_in_copy_mode());
    let selected = app
        .current_copy_selection_text()
        .expect("expected selected transcript text");
    assert!(
        selected.contains("Euler $e^{i\\pi_{3}} + 1 = 0$ done."),
        "inline math should copy as LaTeX: {selected}"
    );
    assert!(
        selected.contains("$$\n\\frac{\\partial L}{\\partial \\theta_{88}} = \\theta^\\top x\n$$"),
        "display math should copy as LaTeX: {selected}"
    );
    assert!(!selected.contains('⊤'), "{selected}");

    let range = app.normalized_copy_selection().expect("range");
    let (chars, _) = crate::tui::ui::copy_selection_metrics(range).expect("metrics");
    assert_eq!(chars, selected.chars().count());
}

#[test]
fn test_copy_selection_partial_drag_over_inline_math_copies_whole_formula() {
    let _render_lock = scroll_render_test_lock();
    let (mut app, mut terminal) = create_math_copy_test_app();
    render_and_snap(&app, &mut terminal);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::ALT)
        .unwrap();

    let layout = crate::tui::ui::last_layout_snapshot().expect("layout snapshot");
    let (visible_start, visible_end) =
        crate::tui::ui::copy_viewport_visible_range().expect("visible copy range");
    let (line_idx, text) = (visible_start..visible_end)
        .find_map(|abs| {
            let text = crate::tui::ui::copy_viewport_line_text(abs).unwrap_or_default();
            text.contains("Euler").then_some((abs, text))
        })
        .expect("prose line");
    let euler_col = unicode_width::UnicodeWidthStr::width(&text[..text.find("Euler").unwrap()]);
    // Drag from the start of "Euler" to two cells into the formula.
    let start_col = euler_col;
    let end_col = euler_col + "Euler ".len() + 2;
    let row = layout.messages_area.y + (line_idx - visible_start) as u16;
    let screen_x = |target: usize| {
        (layout.messages_area.x..layout.messages_area.x + layout.messages_area.width)
            .find(|&column| {
                crate::tui::ui::copy_viewport_point_from_screen(column, row)
                    .is_some_and(|p| p.abs_line == line_idx && p.column == target)
            })
            .expect("screen x")
    };
    let (sx, ex) = (screen_x(start_col), screen_x(end_col));
    for (kind, column) in [
        (MouseEventKind::Down(MouseButton::Left), sx),
        (MouseEventKind::Drag(MouseButton::Left), ex),
    ] {
        app.handle_mouse_event(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        });
    }
    let selected = app.current_copy_selection_text().expect("selection");
    assert_eq!(selected, "Euler $e^{i\\pi_{3}} + 1 = 0$");
}

#[test]
fn test_unicode_math_copy_badge_copies_latex_to_clipboard() {
    let _render_lock = scroll_render_test_lock();
    let clipboard = CapturedClipboard::new();
    let (mut app, mut terminal) = create_math_copy_test_app();
    let screen = render_and_snap(&app, &mut terminal);
    assert!(
        screen.contains("[S]"),
        "math frame should get a badge: {screen}"
    );

    app.handle_key(KeyCode::Char('S'), KeyModifiers::ALT)
        .unwrap();
    assert_eq!(app.status_notice(), Some("Copied math".to_string()));
    assert_eq!(
        clipboard.text().as_deref(),
        Some("$$\n\\frac{\\partial L}{\\partial \\theta_{88}} = \\theta^\\top x\n$$")
    );
}
