#[test]
fn test_help_topic_suggestions_are_contextual() {
    let app = create_test_app();
    let suggestions = app.get_suggestions_for("/help fi");
    assert_eq!(
        suggestions.first().map(|(cmd, _)| cmd.as_str()),
        Some("/help fix")
    );
}

#[test]
fn test_help_topic_suggestions_include_catchup_topics() {
    let app = create_test_app();

    let suggestions = app.get_suggestions_for("/help cat");
    assert!(suggestions.iter().any(|(cmd, _)| cmd == "/help catchup"));

    let suggestions = app.get_suggestions_for("/help bac");
    assert!(suggestions.iter().any(|(cmd, _)| cmd == "/help back"));
}

#[test]
fn test_context_command_reports_session_context_snapshot() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.memory_enabled = true;
        app.swarm_enabled = true;
        app.queue_mode = true;
        app.active_skill = Some("debug".to_string());
        app.queued_messages.push("queued follow-up".to_string());
        app.pending_images
            .push(("image/png".to_string(), "abc".to_string()));
        app.side_panel = crate::side_panel::SidePanelSnapshot {
            focus_revision: 0,
            focused_page_id: Some("goals".to_string()),
            pages: vec![crate::side_panel::SidePanelPage {
                pdf_data: None,
                id: "goals".to_string(),
                title: "Goals".to_string(),
                file_path: "".to_string(),
                format: crate::side_panel::SidePanelPageFormat::Markdown,
                source: crate::side_panel::SidePanelPageSource::Managed,
                content: "goal details".to_string(),
                updated_at_ms: 0,
            }],
        };
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "one".to_string(),
                content: "Inspect context summary".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: Some(crate::todo::ConfidenceState::from_legacy_score(77)),
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        app.input = "/context".to_string();
        app.submit_input();

        let msg = app
            .display_messages()
            .last()
            .expect("missing context report");
        assert_eq!(msg.title.as_deref(), Some("Context"));
        assert!(msg.content.contains("Session Context"));
        assert!(msg.content.contains("Prompt / Context Composition"));
        assert!(msg.content.contains("Compaction"));
        assert!(msg.content.contains("Session State"));
        assert!(msg.content.contains("Todos"));
        assert!(msg.content.contains("Side Panel"));
        assert!(msg.content.contains("Inspect context summary"));
        assert!(msg.content.contains("[pending|high|confidence plausible]"));
        assert!(msg.content.contains("active skill: debug"));
        assert!(msg.content.contains("queue mode: on"));
    });
}

#[test]
fn test_nested_command_suggestions_filter_partial_suffixes() {
    let app = create_test_app();

    let suggestions = app.get_suggestions_for("/config ed");
    assert_eq!(
        suggestions.first().map(|(cmd, _)| cmd.as_str()),
        Some("/config edit")
    );

    let suggestions = app.get_suggestions_for("/alignment ce");
    assert_eq!(
        suggestions.first().map(|(cmd, _)| cmd.as_str()),
        Some("/alignment centered")
    );

    let suggestions = app.get_suggestions_for("/compact mo se");
    assert_eq!(
        suggestions.first().map(|(cmd, _)| cmd.as_str()),
        Some("/compact mode semantic")
    );

    let suggestions = app.get_suggestions_for("/memory st");
    assert_eq!(
        suggestions.first().map(|(cmd, _)| cmd.as_str()),
        Some("/memory status")
    );

    let suggestions = app.get_suggestions_for("/improve st");
    assert!(
        suggestions.iter().any(|(cmd, _)| cmd == "/improve status"),
        "expected /improve status suggestion"
    );

    let suggestions = app.get_suggestions_for("/refactor st");
    assert!(
        suggestions.iter().any(|(cmd, _)| cmd == "/refactor status"),
        "expected /refactor status suggestion"
    );
}

#[test]
fn test_autocomplete_adds_space_for_nested_argument_commands() {
    let mut app = create_test_app();
    app.input = "/goals sh".to_string();
    app.cursor_pos = app.input.len();

    assert!(app.autocomplete());
    assert_eq!(app.input(), "/goals show ");
}

#[test]
fn test_goals_show_suggestions_include_goal_ids() {
    let _guard = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("repo");
    std::fs::create_dir_all(&project).expect("project dir");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp.path());

    let goal = crate::goal::create_goal(
        crate::goal::GoalCreateInput {
            title: "Ship mobile MVP".to_string(),
            scope: crate::goal::GoalScope::Project,
            ..crate::goal::GoalCreateInput::default()
        },
        Some(&project),
    )
    .expect("create goal");

    let mut app = create_test_app();
    app.session.working_dir = Some(project.display().to_string());

    let suggestions = app.get_suggestions_for("/goals show ");
    assert!(
        suggestions
            .iter()
            .any(|(cmd, _)| cmd == &format!("/goals show {}", goal.id))
    );

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

fn configure_test_remote_models(app: &mut App) {
    app.is_remote = true;
    app.remote_provider_model = Some("gpt-5.3-codex".to_string());
    app.remote_available_entries = vec!["gpt-5.3-codex".to_string(), "gpt-5.2-codex".to_string()];
}

fn configure_test_remote_models_with_openai_recommendations(app: &mut App) {
    app.is_remote = true;
    app.remote_provider_model = Some("gpt-5.2".to_string());
    app.remote_available_entries = vec![
        "gpt-5.2".to_string(),
        "gpt-5.5".to_string(),
        "gpt-5.4".to_string(),
        "gpt-5.4-pro".to_string(),
        "gpt-5.3-codex-spark".to_string(),
        "gpt-5.3-codex".to_string(),
        "claude-opus-4-8".to_string(),
    ];
    app.remote_model_options = app
        .remote_available_entries
        .iter()
        .filter(|model| model.as_str() != "claude-opus-4-8")
        .cloned()
        .map(|model| crate::provider::ModelRoute {
            model,
            provider: "OpenAI".to_string(),
            api_method: "openai-oauth".to_string(),
            available: true,
            detail: String::new(),
            usage: None,
            cheapness: None,
        })
        .collect();
    app.remote_model_options.push(crate::provider::ModelRoute {
        model: "claude-opus-4-8".to_string(),
        provider: "Anthropic".to_string(),
        api_method: "claude-oauth".to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    });
    app.remote_model_options.push(crate::provider::ModelRoute {
        model: "claude-opus-4-8".to_string(),
        provider: "Anthropic".to_string(),
        api_method: "claude-api".to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    });
}

fn configure_test_remote_openrouter_provider_routes(app: &mut App) {
    app.is_remote = true;
    app.remote_provider_name = Some("openrouter".to_string());
    app.remote_provider_model = Some("anthropic/claude-sonnet-4".to_string());
    app.remote_available_entries = vec!["anthropic/claude-sonnet-4".to_string()];
    app.remote_model_options = vec![
        crate::provider::ModelRoute {
            model: "anthropic/claude-sonnet-4".to_string(),
            provider: "auto".to_string(),
            api_method: "openrouter".to_string(),
            available: true,
            detail: "→ Fireworks".to_string(),
            usage: None,
            cheapness: None,
        },
        crate::provider::ModelRoute {
            model: "anthropic/claude-sonnet-4".to_string(),
            provider: "Fireworks".to_string(),
            api_method: "openrouter".to_string(),
            available: true,
            detail: String::new(),
            usage: None,
            cheapness: None,
        },
        crate::provider::ModelRoute {
            model: "anthropic/claude-sonnet-4".to_string(),
            provider: "OpenAI".to_string(),
            api_method: "openrouter".to_string(),
            available: true,
            detail: String::new(),
            usage: None,
            cheapness: None,
        },
    ];
}

#[test]
fn test_model_picker_preview_filter_parsing() {
    assert_eq!(
        App::model_picker_preview_filter("/model"),
        Some(String::new())
    );
    assert_eq!(
        App::model_picker_preview_filter("/model   gpt-5"),
        Some("gpt-5".to_string())
    );
    assert_eq!(
        App::model_picker_preview_filter("   /models codex"),
        Some("codex".to_string())
    );
    assert_eq!(App::model_picker_preview_filter("/modelx"), None);
    assert_eq!(App::model_picker_preview_filter("hello /model"), None);
}

#[test]
fn test_login_picker_preview_filter_parsing() {
    assert_eq!(
        App::login_picker_preview_filter("/login"),
        Some(String::new())
    );
    assert_eq!(
        App::login_picker_preview_filter("/login   zai"),
        Some("zai".to_string())
    );
    assert_eq!(App::login_picker_preview_filter("/loginx"), None);
    assert_eq!(App::login_picker_preview_filter("hello /login"), None);
}

#[test]
fn test_agents_command_opens_agent_picker() {
    let mut app = create_test_app();
    app.input = "/agents".to_string();

    app.submit_input();

    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("/agents should open the agent picker");
    assert!(
        picker
            .entries
            .iter()
            .any(|entry| entry.name == "Code review")
    );
    assert!(picker.entries.iter().any(|entry| matches!(
        entry.action,
        crate::tui::PickerAction::AgentTarget(crate::tui::AgentModelTarget::Swarm)
    )));
    let swarm_entry = picker
        .entries
        .iter()
        .find(|entry| {
            matches!(
                entry.action,
                crate::tui::PickerAction::AgentTarget(crate::tui::AgentModelTarget::Swarm)
            )
        })
        .expect("swarm entry");
    assert!(swarm_entry.options[0].detail.contains("/swarm-prompt"));
}

#[test]
fn test_agents_command_suggestions_include_targets() {
    let app = create_test_app();
    let suggestions = app.get_suggestions_for("/agents re");
    assert!(suggestions.iter().any(|(cmd, _)| cmd == "/agents review"));
}

#[test]
fn test_swarm_prompt_command_is_discoverable_in_suggestions_and_help() {
    let app = create_test_app();
    let suggestions = app.get_suggestions_for("/swarm-pro");
    assert!(
        suggestions
            .iter()
            .any(|(command, _)| command == "/swarm-prompt")
    );

    let help = app
        .command_help("swarm-prompt")
        .expect("/swarm-prompt should have detailed help");
    assert!(help.contains("/swarm-prompt"));
    assert!(help.contains(".jcode/swarm-prompt.md"));
    assert!(help.contains("Restart or reload Jcode"));
}

#[test]
fn test_agents_picker_uses_provider_default_when_inherited_model_is_unknown() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.open_agents_picker();

        let picker = app
            .inline_interactive_state
            .as_ref()
            .expect("/agents should open the agent picker");
        let swarm_entry = picker
            .entries
            .iter()
            .find(|entry| {
                matches!(
                    entry.action,
                    crate::tui::PickerAction::AgentTarget(crate::tui::AgentModelTarget::Swarm)
                )
            })
            .expect("swarm entry should exist");

        assert_eq!(swarm_entry.options[0].provider, "provider default");
    });
}

#[test]
fn test_agent_model_picker_inherit_row_uses_provider_default_when_inherited_model_is_unknown() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        configure_test_remote_models(&mut app);
        app.open_agent_model_picker(crate::tui::AgentModelTarget::Swarm);

        let picker = app
            .inline_interactive_state
            .as_ref()
            .expect("agent model picker should open");
        let inherit_entry = picker.entries.first().expect("inherit row should exist");

        assert_eq!(inherit_entry.name, "inherit (provider default)");
        assert!(matches!(
            inherit_entry.action,
            crate::tui::PickerAction::AgentModelChoice {
                target: crate::tui::AgentModelTarget::Swarm,
                clear_override: true,
            }
        ));
    });
}

#[test]
fn test_panel_image_preview_click_render_dismiss_and_restore() {
    let _lock = scroll_render_test_lock();
    struct ResetImageMode;
    impl Drop for ResetImageMode {
        fn drop(&mut self) {
            crate::tui::mermaid::set_video_export_mode(false);
        }
    }
    crate::tui::mermaid::set_video_export_mode(true);
    let _reset = ResetImageMode;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preview.png");
    ::image::RgbaImage::from_pixel(800, 400, ::image::Rgba([0, 80, 255, 255]))
        .save(&path)
        .unwrap();
    let mut app = create_test_app();
    app.diff_mode = crate::config::DiffDisplayMode::Inline;
    app.side_panel = crate::side_panel::SidePanelSnapshot {
        focus_revision: 0,
        focused_page_id: Some("preview".into()),
        pages: vec![crate::side_panel::SidePanelPage {
            pdf_data: None,
            id: "preview".into(),
            title: "Preview fixture".into(),
            file_path: "".into(),
            format: crate::side_panel::SidePanelPageFormat::Markdown,
            source: crate::side_panel::SidePanelPageSource::Managed,
            content: format!(
                "# Preview fixture\n\n![Image]({})\n\nAfter image",
                path.display()
            ),
            updated_at_ms: 1,
        }],
    };
    app.input = "keep my draft".into();
    app.cursor_pos = app.input.len();
    app.diff_pane_auto_scroll = false;
    app.diff_pane_scroll = 2;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    let text = render_and_snap(&app, &mut terminal);
    assert!(text.contains("Preview fixture"), "{text}");
    let (x, y, hash) = (0..40)
        .find_map(|y| {
            (0..120).find_map(|x| {
                crate::tui::ui::panel_image_preview::image_at(x, y).map(|hash| (x, y, hash))
            })
        })
        .expect("rendered image should have a click target");
    let mouse = |kind, column, row| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    };
    // Click targets exclude the text header and chat.
    assert_eq!(crate::tui::ui::panel_image_preview::image_at(0, 0), None);
    // Dragging across image placeholder rows remains a selection, not a click.
    app.handle_mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
    app.handle_mouse_event(mouse(MouseEventKind::Drag(MouseButton::Left), x, y + 1));
    app.handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), x, y + 1));
    assert_eq!(app.panel_image_preview, None);
    app.handle_mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
    assert_eq!(app.panel_image_preview, None, "open on release, not press");
    app.handle_mouse_event(mouse(MouseEventKind::Drag(MouseButton::Left), x, y));
    app.handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), x, y));
    assert_eq!(app.panel_image_preview, Some(hash));
    let text = render_and_snap(&app, &mut terminal);
    assert!(text.contains("Image preview"), "{text}");
    assert!(text.contains("Click or Esc to close"), "{text}");
    assert!(
        !text.contains("Preview fixture"),
        "preview replaces the split layout"
    );
    assert_eq!(crate::tui::ui::panel_image_preview::image_at(x, y), None);
    // Modal input must not edit the prompt or scroll the panel behind it.
    app.handle_key(KeyCode::Char('x'), KeyModifiers::empty())
        .unwrap();
    app.handle_mouse_event(mouse(MouseEventKind::ScrollDown, x, y));
    assert_eq!(app.input, "keep my draft");
    assert_eq!(app.diff_pane_scroll, 2);
    app.handle_key(KeyCode::Esc, KeyModifiers::empty()).unwrap();
    assert_eq!(app.panel_image_preview, None);
    assert_eq!(app.diff_pane_scroll, 2);
    let text = render_and_snap(&app, &mut terminal);
    assert!(text.contains("Preview fixture"));
    // Reopening and clicking anywhere closes without clicking through.
    app.handle_mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
    app.handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), x, y));
    assert_eq!(app.panel_image_preview, Some(hash));
    app.handle_mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
    assert_eq!(app.panel_image_preview, Some(hash));
    app.handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), 0, 0));
    assert_eq!(app.panel_image_preview, None);
    assert_eq!(app.input, "keep my draft");
    // Hiding the panel must remove its old clickable regions.
    app.side_panel = Default::default();
    render_and_snap(&app, &mut terminal);
    assert_eq!(crate::tui::ui::panel_image_preview::image_at(x, y), None);
}

#[test]
fn test_panel_image_preview_missing_image_and_tiny_terminal_are_dismissible() {
    let _lock = scroll_render_test_lock();
    let mut app = create_test_app();
    app.panel_image_preview = Some(u64::MAX - 123);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(90, 15)).unwrap();
    assert!(render_and_snap(&app, &mut terminal).contains("Image is no longer available"));
    let mut tiny = ratatui::Terminal::new(ratatui::backend::TestBackend::new(1, 1)).unwrap();
    render_and_snap(&app, &mut tiny);
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();
    assert_eq!(app.panel_image_preview, None);
}

#[test]
fn test_panel_image_preview_remote_keys_and_session_reset() {
    let mut app = create_test_app();
    app.input = "unsent draft".into();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    for close_key in [KeyCode::Esc, KeyCode::Enter, KeyCode::Char('q')] {
        app.panel_image_preview = Some(42);
        rt.block_on(app.handle_remote_key(KeyCode::Char('x'), KeyModifiers::empty(), &mut remote))
            .unwrap();
        assert_eq!(app.input, "unsent draft");
        assert_eq!(app.panel_image_preview, Some(42));
        rt.block_on(app.handle_remote_key(close_key, KeyModifiers::empty(), &mut remote))
            .unwrap();
        assert_eq!(app.panel_image_preview, None);
        assert_eq!(app.input, "unsent draft");
    }
    app.panel_image_preview = Some(42);
    crate::tui::app::commands_review::clear_side_panel_for_new_session(&mut app);
    assert_eq!(app.panel_image_preview, None);
}
