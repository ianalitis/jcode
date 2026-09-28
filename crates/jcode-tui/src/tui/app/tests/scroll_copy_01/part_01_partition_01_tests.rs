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

/// The session status line stays visible and keeps the model accent during scroll.
#[test]
fn status_line_is_always_pinned_with_pink_model_on_real_app() {
    let _lock = scroll_render_test_lock();
    for width in [120u16, 60] {
        let (mut app, mut terminal) = create_scroll_test_app(width, 30, 0, 36);
        let pink = ratatui::style::Color::Rgb(255, 135, 200);
        let last_row_pink_cells = |terminal: &ratatui::Terminal<ratatui::backend::TestBackend>| {
            let buf = terminal.backend().buffer();
            let y = buf.area.height - 1;
            (0..buf.area.width)
                .filter(|&x| buf[(x, y)].fg == pink && !buf[(x, y)].symbol().trim().is_empty())
                .count()
        };

        let at_rest = render_and_snap(&app, &mut terminal);
        assert!(!at_rest.contains("(overscroll"), "w={width}: {at_rest}");
        assert!(last_row_pink_cells(&terminal) >= 3, "pinned model (w={width}): {at_rest}");

        for kind in [MouseEventKind::ScrollDown, MouseEventKind::ScrollUp, MouseEventKind::ScrollDown] {
            app.handle_mouse_event(MouseEvent {
                kind,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::empty(),
            });
            let frame = render_and_snap(&app, &mut terminal);
            assert!(!frame.contains("(overscroll"), "w={width}: {frame}");
            assert!(last_row_pink_cells(&terminal) >= 3, "pinned after {kind:?} (w={width}): {frame}");
        }
    }
}

#[test]
fn agent_edited_paths_come_from_transcript_edit_tools() {
    let _lock = scroll_render_test_lock();
    let (mut app, _terminal) = create_scroll_test_app(80, 20, 0, 4);
    app.session.working_dir = Some("/repo/crates".to_string());
    let tool = |name: &str, input: serde_json::Value| {
        DisplayMessage::tool(
            "ok",
            crate::message::ToolCall {
                id: name.into(),
                name: name.into(),
                input,
                ..Default::default()
            },
        )
    };
    app.display_messages
        .push(tool("edit", serde_json::json!({"file_path": "a/src/x.rs"})));
    app.display_messages.push(tool(
        "apply_patch",
        serde_json::json!({"patch_text": "*** Begin Patch\n*** Update File: /repo/README.md\n@@\n-a\n+b\n*** End Patch"}),
    ));
    app.display_messages
        .push(tool("read", serde_json::json!({"file_path": "a/src/y.rs"})));
    app.bump_display_messages_version();

    let data = app.info_widget_data();
    let set = &data.agent_edited;
    assert!(set.contains(std::path::Path::new("/repo/crates/a/src/x.rs")), "{set:?}");
    assert!(set.contains(std::path::Path::new("/repo/README.md")), "{set:?}");
    assert!(
        !set.contains(std::path::Path::new("/repo/crates/a/src/y.rs")),
        "reads are not edits"
    );

    let again = app.info_widget_data().agent_edited;
    assert!(std::sync::Arc::ptr_eq(&data.agent_edited, &again));
    app.display_messages
        .push(tool("write", serde_json::json!({"file_path": "/repo/new.rs"})));
    app.bump_display_messages_version();
    assert!(
        app.info_widget_data()
            .agent_edited
            .contains(std::path::Path::new("/repo/new.rs"))
    );
}

/// Exercise the production status collector and a real App frame together.
#[test]
fn changes_widget_end_to_end_on_real_git_repo() {
    use std::process::Command;
    let _lock = scroll_render_test_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&root)
            .output()
            .expect("git");
        assert!(output.status.success(), "git {args:?}: {output:?}");
    };
    let write = |rel: &str, bytes: &[u8]| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    let sleep = || std::thread::sleep(std::time::Duration::from_millis(20));

    git(&["init", "-q", "-b", "main"]);
    write("src/lib.rs", b"a\nb\nc\nd\n");
    write("src/old_name.rs", b"x\ny\n");
    write("gone.txt", b"1\n2\n3\n");
    write("logo.bin", &[0, 1, 2, 3]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "init"]);
    write("logo.bin", &[0, 9, 9, 9, 9]);
    sleep();
    std::fs::remove_file(root.join("gone.txt")).unwrap();
    git(&["mv", "src/old_name.rs", "src/new_name.rs"]);
    sleep();
    write("nested/dir/new.rs", b"1\n2\n3\n");
    sleep();
    write("src/lib.rs", b"a\nB\nc\nd\ne\nf\n");

    let info = crate::tui::app::helpers::gather_git_info_in(Some(&root)).expect("repo");
    let find = |name: &str| info.dirty_files.iter().find(|f| f.path.ends_with(name)).cloned();
    let lib = find("src/lib.rs").expect("modified");
    assert_eq!((lib.status, lib.added, lib.removed), ('M', Some(3), Some(1)));
    let untracked = find("nested/dir/new.rs").expect("untracked");
    assert_eq!((untracked.status, untracked.added, untracked.removed), ('?', Some(3), Some(0)));
    let deleted = find("gone.txt").expect("deleted");
    assert_eq!((deleted.status, deleted.added, deleted.removed), ('D', Some(0), Some(3)));
    let renamed = find("new_name.rs").expect("renamed");
    assert_eq!(renamed.status, 'R');
    let binary = find("logo.bin").expect("binary");
    assert_eq!((binary.added, binary.removed), (None, None));
    assert_eq!(info.dirty_total, 5);
    assert_eq!(info.added_total, 6 + renamed.added.unwrap_or(0));
    assert_eq!(info.removed_total, 4 + renamed.removed.unwrap_or(0));
    assert_eq!(info.dirty_files[0].path, "src/lib.rs");
    assert_eq!(info.dirty_files.last().unwrap().path, "gone.txt");
    assert_eq!(info.repo_root.as_deref(), Some(root.as_path()));

    crate::tui::app::helpers::seed_git_info_cache_for_tests(Some(info));
    crate::tui::info_widget::clear_widget_placements_for_tests();
    let (mut app, mut terminal) = create_scroll_test_app(140, 40, 0, 0);
    app.session.working_dir = Some(root.join("src").to_string_lossy().into_owned());
    app.display_messages.push(DisplayMessage::tool(
        "ok",
        crate::message::ToolCall {
            id: "e1".into(),
            name: "edit".into(),
            input: serde_json::json!({"file_path": "lib.rs"}),
            ..Default::default()
        },
    ));
    app.bump_display_messages_version();
    let mut frame = String::new();
    for _ in 0..3 {
        frame = render_and_snap(&app, &mut terminal);
    }
    crate::tui::app::helpers::seed_git_info_cache_for_tests(None);
    let row = |name: &str| {
        frame
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("{name:?} missing:\n{frame}"))
    };
    assert!(row("lib.rs").contains("M● src/lib.rs"), "{frame}");
    assert!(row("new_name.rs").contains("R  new_name.rs"), "{frame}");
    assert!(row("gone.txt").contains("D  gone.txt"), "{frame}");
    assert!(row("lib.rs").contains("+3 −1"), "{frame}");
    assert!(row("new.rs").contains("?  new.rs"), "{frame}");
    assert!(row("new.rs").contains("+3 −0"), "{frame}");
    assert!(!row("logo.bin").contains('+'), "{frame}");
    assert!(frame.contains("● edited by agent"), "{frame}");

    let info = crate::tui::app::helpers::gather_git_info_in(Some(&root)).expect("repo");
    crate::tui::app::helpers::seed_git_info_cache_for_tests(Some(info));
    crate::tui::info_widget::clear_widget_placements_for_tests();
    let (app, mut terminal) = create_scroll_test_app(140, 40, 0, 0);
    let mut without_edits = String::new();
    for _ in 0..3 {
        without_edits = render_and_snap(&app, &mut terminal);
    }
    crate::tui::app::helpers::seed_git_info_cache_for_tests(None);
    assert!(without_edits.contains("src/lib.rs"), "{without_edits}");
    assert!(!without_edits.contains("edited by agent"), "{without_edits}");
    assert!(!without_edits.contains('●'), "{without_edits}");
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
