#[test]
fn preview_render_cache_is_reused_across_scroll_and_rebuilt_on_selection_change() {
    // The preview pane caches its fully-wrapped content keyed by a content hash
    // and pane geometry, so scrolling reuses the cache instead of re-rendering
    // and re-wrapping every line. Navigating to another session must invalidate
    // it (different content hash).
    let a = make_session_with_many_turns("cache_a", 60);
    let b = make_session_with_many_turns("cache_b", 60);
    let mut picker = SessionPicker::new(vec![a, b]);
    picker.focus = PaneFocus::Preview;

    let w = 100u16;
    let h = 16u16;
    let render = |picker: &mut SessionPicker| {
        let backend = ratatui::backend::TestBackend::new(w, h);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| picker.render(frame))
            .expect("render picker");
    };

    // First render builds the cache for the selected session.
    render(&mut picker);
    let key_after_build = picker
        .preview_cache
        .as_ref()
        .map(|c| c.key.clone())
        .expect("preview cache built on first render");
    let wrapped_len = picker
        .preview_cache
        .as_ref()
        .map(|c| c.wrapped_lines.len())
        .unwrap();
    assert!(wrapped_len > h as usize, "preview should overflow viewport");

    // Scrolling several times must not change the cache key (content unchanged):
    // the cache is reused and only the scroll offset + visible slice move.
    for _ in 0..5 {
        picker.scroll_preview_up(1);
        render(&mut picker);
        let key_now = picker
            .preview_cache
            .as_ref()
            .map(|c| c.key.clone())
            .unwrap();
        assert!(
            key_now == key_after_build,
            "scrolling must reuse the cached wrapped preview"
        );
    }

    // Navigating to a different session changes the content hash, so the cache
    // is rebuilt for the new selection.
    picker.next();
    render(&mut picker);
    let key_after_nav = picker
        .preview_cache
        .as_ref()
        .map(|c| c.key.clone())
        .unwrap();
    assert!(
        key_after_nav != key_after_build,
        "selecting a different session must invalidate the preview cache"
    );
}

#[test]
fn preview_visible_slice_matches_scroll_position() {
    // The renderer materializes only the visible window of wrapped lines. Confirm
    // that scrolling actually changes what is drawn (i.e. the slice tracks the
    // scroll offset rather than always showing the bottom).
    let session = make_session_with_many_turns("slice", 60);
    let mut picker = SessionPicker::new(vec![session]);
    picker.focus = PaneFocus::Preview;

    let w = 100u16;
    let h = 16u16;
    let render_text = |picker: &mut SessionPicker| -> String { buffer_text(picker, w, h) };

    // First render auto-scrolls to the bottom.
    let bottom = render_text(&mut picker);
    let bottom_scroll = picker.scroll_offset;
    assert!(bottom_scroll > 0, "long preview should be scrolled down");

    // Scroll to the very top; the rendered content must differ from the bottom.
    picker.scroll_preview_up(bottom_scroll);
    let top = render_text(&mut picker);
    assert_eq!(picker.scroll_offset, 0);
    assert_ne!(
        top, bottom,
        "scrolling to the top should render different content than the bottom"
    );
}

#[test]
fn test_reseed_grouped_preserves_selection_and_search() {
    // Build a picker with several sessions, then simulate the user navigating to
    // a specific session and typing a search. A background refresh that reseeds
    // the same data must keep the highlighted session and the active search.
    let sessions: Vec<SessionInfo> = (0..6)
        .map(|i| {
            make_session(
                &format!("session_reseed_{i}"),
                &format!("reseed{i}"),
                false,
                SessionStatus::Closed,
            )
        })
        .collect();
    let mut picker = SessionPicker::new(sessions.clone());

    // Move selection to the third visible session.
    picker.next();
    picker.next();
    let selected_before = picker
        .selected_session()
        .map(|s| s.id.clone())
        .expect("a session should be selected");

    // Activate a search that matches only one session id.
    picker.search_query = "reseed4".to_string();
    picker.search_active = true;
    picker.focus = PaneFocus::Preview;
    picker.rebuild_items();
    let search_selected = picker
        .selected_session()
        .map(|s| s.id.clone())
        .expect("search should leave a selection");

    // Reseed with the same data (as the async refresh would).
    picker.reseed_grouped(Vec::new(), sessions);

    // Search query, search mode, focus, and the matched selection survive.
    assert_eq!(picker.search_query, "reseed4");
    assert!(picker.search_active);
    assert_eq!(picker.focus, PaneFocus::Preview);
    assert_eq!(
        picker.selected_session().map(|s| s.id.clone()),
        Some(search_selected)
    );

    // Clearing the search restores the full list; the originally highlighted
    // session is still resolvable in the reseeded data.
    picker.search_query.clear();
    picker.search_active = false;
    picker.rebuild_items();
    assert!(
        picker
            .visible_session_iter_for_test()
            .any(|s| s.id == selected_before),
        "previously selected session should still be present after reseed"
    );
}

#[test]
fn test_reseed_grouped_keeps_selection_when_list_changes() {
    // The highlighted session must follow its id even when the refreshed list has
    // a different order / additional sessions (the realistic refresh case).
    let initial: Vec<SessionInfo> = (0..4)
        .map(|i| {
            make_session(
                &format!("session_keep_{i}"),
                &format!("keep{i}"),
                false,
                SessionStatus::Closed,
            )
        })
        .collect();
    let mut picker = SessionPicker::new(initial.clone());
    picker.next(); // select session_keep_1
    let target = picker
        .selected_session()
        .map(|s| s.id.clone())
        .expect("selection");

    // Refreshed list: prepend a brand-new session and keep the rest, changing
    // indices so a naive index-based selection would drift.
    let mut refreshed = vec![make_session(
        "session_keep_new",
        "keepnew",
        false,
        SessionStatus::Closed,
    )];
    refreshed.extend(initial);

    picker.reseed_grouped(Vec::new(), refreshed);

    assert_eq!(
        picker.selected_session().map(|s| s.id.clone()),
        Some(target),
        "selection should follow the session id across a reordered refresh"
    );
}

#[test]
fn test_search_mode_ctrl_j_k_navigate_session_list() {
    let mut newer = make_session("session_newer", "newer", false, SessionStatus::Closed);
    let mut older = make_session("session_older", "older", false, SessionStatus::Closed);
    newer.last_message_time = Utc::now();
    older.last_message_time = Utc::now() - ChronoDuration::minutes(1);
    let mut picker = SessionPicker::new(vec![older, newer]);

    // Enter search mode (both visible sessions still match the empty query).
    picker
        .handle_overlay_key(KeyCode::Char('/'), KeyModifiers::empty())
        .unwrap();
    assert!(picker.search_active);
    let first = picker
        .selected_session()
        .map(|s| s.id.clone())
        .expect("a session is selected on entering search");

    // Ctrl+J moves down the list without typing 'j' into the query.
    picker
        .handle_overlay_key(KeyCode::Char('j'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(
        picker.search_query.is_empty(),
        "Ctrl+J must not type into search"
    );
    let second = picker
        .selected_session()
        .map(|s| s.id.clone())
        .expect("a session is selected after Ctrl+J");
    assert_ne!(first, second, "Ctrl+J should move the selection down");

    // Ctrl+K moves back up to the original selection.
    picker
        .handle_overlay_key(KeyCode::Char('k'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(
        picker.search_query.is_empty(),
        "Ctrl+K must not type into search"
    );
    assert_eq!(
        picker.selected_session().map(|s| s.id.clone()),
        Some(first),
        "Ctrl+K should move the selection back up"
    );
}

#[test]
fn test_search_mode_ctrl_backspace_deletes_word() {
    let mut picker = SessionPicker::new(vec![make_session(
        "session_a",
        "a",
        false,
        SessionStatus::Closed,
    )]);
    picker
        .handle_overlay_key(KeyCode::Char('/'), KeyModifiers::empty())
        .unwrap();
    for c in "hello world".chars() {
        picker
            .handle_overlay_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    assert_eq!(picker.search_query, "hello world");

    // Ctrl+Backspace deletes the trailing word.
    picker
        .handle_overlay_key(KeyCode::Backspace, KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(picker.search_query, "hello ");

    // The \u{8} alias some terminals send for Ctrl+Backspace also deletes a word.
    picker
        .handle_overlay_key(KeyCode::Char('\u{8}'), KeyModifiers::empty())
        .unwrap();
    assert_eq!(picker.search_query, "");

    // Plain Backspace still deletes a single character.
    for c in "abc".chars() {
        picker
            .handle_overlay_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    picker
        .handle_overlay_key(KeyCode::Backspace, KeyModifiers::empty())
        .unwrap();
    assert_eq!(picker.search_query, "ab");
}

#[test]
fn test_search_mode_ctrl_u_clears_query() {
    let mut picker = SessionPicker::new(vec![make_session(
        "session_a",
        "a",
        false,
        SessionStatus::Closed,
    )]);
    picker
        .handle_overlay_key(KeyCode::Char('/'), KeyModifiers::empty())
        .unwrap();
    for c in "needle".chars() {
        picker
            .handle_overlay_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    assert_eq!(picker.search_query, "needle");
    picker
        .handle_overlay_key(KeyCode::Char('u'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(picker.search_query, "");
    assert!(
        picker.search_active,
        "Ctrl+U clears text but stays in search"
    );
}

#[test]
fn test_current_dir_highlight_marks_matching_sessions() {
    let mut same = make_session("same_dir", "same", false, SessionStatus::Closed);
    same.working_dir = Some("/home/jeremy/project".to_string());
    let mut other = make_session("other_dir", "other", false, SessionStatus::Closed);
    other.working_dir = Some("/home/jeremy/elsewhere".to_string());

    let mut picker = SessionPicker::new(vec![same.clone(), other.clone()]);
    // Trailing slash on the current dir should still match (normalization).
    picker.set_current_dir(Some("/home/jeremy/project/".to_string()));

    assert!(picker.session_in_current_dir(&same));
    assert!(!picker.session_in_current_dir(&other));

    let rows = picker.render_session_item_lines(&same, false);
    let text: String = rows.iter().map(line_text).collect::<Vec<_>>().join("\n");
    assert!(
        text.contains("here"),
        "matching session should show the `here` marker: {text}"
    );

    // The marker and directory line should be styled with the same-dir accent
    // green so the highlight is visually distinct, not just present as text.
    let same_dir_color = rgb(120, 200, 140);
    let marker_styled_green = rows.iter().any(|line| {
        line.spans
            .iter()
            .any(|span| span.content.contains("here") && span.style.fg == Some(same_dir_color))
    });
    assert!(
        marker_styled_green,
        "`here` marker should be rendered in the same-dir accent color"
    );

    let other_rows = picker.render_session_item_lines(&other, false);
    let other_text: String = other_rows
        .iter()
        .map(line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !other_text.contains("▸ here"),
        "non-matching session should not show the marker: {other_text}"
    );
}

#[test]
fn test_current_dir_highlight_absent_without_current_dir() {
    let mut session = make_session("s", "s", false, SessionStatus::Closed);
    session.working_dir = Some("/home/jeremy/project".to_string());
    let picker = SessionPicker::new(vec![session.clone()]);
    // No current_dir set: nothing is highlighted.
    assert!(!picker.session_in_current_dir(&session));
}

#[test]
fn highlight_spans_marks_query_occurrences() {
    let base = Style::default().fg(Color::White);
    let tokens = vec!["resume".to_string()];
    let spans = SessionPicker::highlight_spans("Fix the Resume bug", &tokens, base);
    let combined: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(combined, "Fix the Resume bug");

    let highlighted: Vec<&str> = spans
        .iter()
        .filter(|s| s.style.add_modifier.contains(Modifier::BOLD))
        .map(|s| s.content.as_ref())
        .collect();
    assert_eq!(
        highlighted,
        vec!["Resume"],
        "match should be highlighted case-insensitively"
    );
}

#[test]
fn highlight_spans_marks_each_token_independently() {
    // Multi-word queries highlight every token (order independent), matching the
    // AND-token filter semantics.
    let base = Style::default().fg(Color::White);
    let tokens = vec!["resume".to_string(), "bug".to_string()];
    let spans = SessionPicker::highlight_spans("Fix the Resume bug now", &tokens, base);
    let combined: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(combined, "Fix the Resume bug now");
    let highlighted: Vec<&str> = spans
        .iter()
        .filter(|s| s.style.add_modifier.contains(Modifier::BOLD))
        .map(|s| s.content.as_ref())
        .collect();
    assert_eq!(
        highlighted,
        vec!["Resume", "bug"],
        "every token should be highlighted"
    );
}

#[test]
fn highlight_spans_without_query_returns_single_span() {
    let base = Style::default().fg(Color::White);
    let spans = SessionPicker::highlight_spans("hello world", &[], base);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "hello world");
}

#[test]
fn search_highlights_matching_title_in_rendered_rows() {
    let session = make_session("abc", "deploy pipeline", false, SessionStatus::Closed);
    let mut picker = SessionPicker::new(vec![session]);
    // make_session sets title = "Test session"; search a substring of the title.
    picker.search_query = "sess".to_string();
    let rows = picker.render_session_item_lines(picker.all_sessions.first().unwrap(), false);
    let has_highlight = rows[0]
        .spans
        .iter()
        .any(|s| s.content.as_ref() == "sess" && s.style.add_modifier.contains(Modifier::BOLD));
    assert!(
        has_highlight,
        "query substring in title should be highlighted"
    );
}

#[test]
fn search_highlights_match_in_preview_and_scrolls_to_it() {
    // A long transcript where a distinctive term ("flibbertigibbet") appears only
    // in an early message. Searching for it should both highlight the match in the
    // preview pane and scroll the preview to the match rather than to the bottom.
    let mut session = make_session_with_many_turns("long", 60);
    // Inject the unique term near the top of the transcript.
    session.messages_preview[4].content = "the magic flibbertigibbet token".to_string();
    let mut picker = SessionPicker::new(vec![session]);
    picker.focus = PaneFocus::Preview;

    let w = 100u16;
    let h = 16u16;

    // Baseline: no search -> auto-scrolls to bottom.
    let _ = buffer_text(&mut picker, w, h);
    let bottom_scroll = picker.scroll_offset;
    assert!(
        bottom_scroll > 0,
        "long preview should scroll to bottom by default"
    );

    // Now search for the unique early term. Reset auto-scroll like a keystroke would.
    picker.search_query = "flibbertigibbet".to_string();
    picker.auto_scroll_preview = true;
    let text = buffer_text(&mut picker, w, h);

    // The preview should have scrolled to the match (near the top), not the bottom.
    assert!(
        picker.scroll_offset < bottom_scroll,
        "preview should scroll up to the match (got {}, bottom was {})",
        picker.scroll_offset,
        bottom_scroll
    );
    assert!(
        text.contains("flibbertigibbet"),
        "matched term should be visible in the preview after scrolling"
    );

    // The match should be highlighted (bold) in the cached wrapped lines.
    let highlighted = picker
        .preview_cache
        .as_ref()
        .expect("preview cache built")
        .wrapped_lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .any(|s| {
            s.content.to_lowercase().contains("flibbertigibbet")
                && s.style.add_modifier.contains(Modifier::BOLD)
        });
    assert!(
        highlighted,
        "matched term in preview body should be highlighted"
    );
}

#[test]
fn preview_without_search_has_no_highlight_and_scrolls_to_bottom() {
    let session = make_session_with_many_turns("nosrch", 60);
    let mut picker = SessionPicker::new(vec![session]);
    picker.focus = PaneFocus::Preview;
    let _ = buffer_text(&mut picker, 100, 16);
    assert!(
        picker.scroll_offset > 0,
        "should scroll to bottom without search"
    );
    let any_highlight = picker
        .preview_cache
        .as_ref()
        .expect("preview cache built")
        .wrapped_lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .any(|s| {
            s.style.add_modifier.contains(Modifier::BOLD) && s.style.fg == Some(rgb(255, 214, 90))
        });
    assert!(
        !any_highlight,
        "no search means no highlight color in preview"
    );
}
