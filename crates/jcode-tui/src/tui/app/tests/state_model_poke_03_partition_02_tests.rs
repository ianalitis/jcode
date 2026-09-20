#[test]
fn test_local_model_picker_openrouter_bare_openai_route_uses_openai_catalog_prefix() {
    let (mut app, set_model_calls) = create_openrouter_spec_capture_test_app();
    app.open_model_picker();
    wait_for_model_picker_load(&mut app);

    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("model picker should be open");
    let model_idx = picker
        .entries
        .iter()
        .position(|entry| entry.name == "gpt-5.4 (high)")
        .expect("openrouter-backed OpenAI effort entry should be in picker");
    let filtered_pos = picker
        .filtered
        .iter()
        .position(|&i| i == model_idx)
        .expect("entry should be in filtered list");

    app.inline_interactive_state.as_mut().unwrap().selected = filtered_pos;
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("model picker selection should succeed");

    assert_eq!(
        set_model_calls.lock().unwrap().as_slice(),
        ["openai/gpt-5.4@OpenAI"]
    );
}

#[test]
fn test_agent_model_picker_openrouter_bare_openai_route_saves_openai_catalog_prefix() {
    let (mut app, _set_model_calls) = create_openrouter_spec_capture_test_app();

    app.open_agent_model_picker(crate::tui::AgentModelTarget::Swarm);

    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("agent model picker should be open");
    let model_idx = picker
        .entries
        .iter()
        .position(|entry| entry.name == "gpt-5.4 (high)")
        .expect("openrouter-backed OpenAI effort entry should be in picker");
    let filtered_pos = picker
        .filtered
        .iter()
        .position(|&i| i == model_idx)
        .expect("entry should be in filtered list");

    app.inline_interactive_state.as_mut().unwrap().selected = filtered_pos;
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("agent model picker selection should succeed");

    let last = app.display_messages.last().expect("display message");
    assert_eq!(last.role, "system");
    assert!(
        last.content.contains("openai/gpt-5.4@OpenAI"),
        "message should show normalized saved spec, got: {}",
        last.content
    );
}

#[test]
fn test_local_model_picker_render_shows_antigravity_models_exactly_as_user_sees_them() {
    let mut app = create_antigravity_picker_test_app();
    app.display_messages = vec![DisplayMessage::system("seed render state")];
    app.bump_display_messages_version();
    app.open_model_picker();
    wait_for_model_picker_load(&mut app);

    let render_filtered = |app: &mut App, filter: &str| {
        let picker = app
            .inline_interactive_state
            .as_mut()
            .expect("model picker should be open");
        picker.filter = filter.to_string();
        App::apply_inline_interactive_filter(picker);
        let _render_lock = scroll_render_test_lock();
        let backend = ratatui::backend::TestBackend::new(90, 14);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        render_and_snap(app, &mut terminal)
    };
    let claude_text = render_filtered(&mut app, "claude-sonnet-4-6");
    let gpt_text = render_filtered(&mut app, "gpt-oss-120b-medium");

    assert!(
        claude_text.contains("▸ Claude Sonnet 4.6") && claude_text.contains("↑↓ choose"),
        "rendered /model suggestions should show the selected row and navigation, got:
{}",
        claude_text
    );
    assert!(
        claude_text.contains("Claude Sonnet 4.6"),
        "rendered /model view should show the Antigravity Claude row, got:
{}",
        claude_text
    );
    assert!(
        gpt_text.contains("gpt-oss-120b-medium"),
        "rendered /model view should show the Antigravity GPT row, got:
{}",
        gpt_text
    );
    assert!(
        claude_text.contains("Antigravity") && gpt_text.contains("Antigravity"),
        "rendered /model view should show the Antigravity provider column, got:
Claude:
{}
GPT:
{}",
        claude_text,
        gpt_text
    );
    assert!(
        claude_text.contains("cli") && gpt_text.contains("cli"),
        "rendered /model view should show the route transport column, got:
Claude:
{}
GPT:
{}",
        claude_text,
        gpt_text
    );
}

#[test]
fn test_login_smoke_model_picker_renders_unstacked_provider_rows() {
    let mut app = create_login_smoke_model_app();
    app.display_messages = vec![DisplayMessage::system("seed render state")];
    app.bump_display_messages_version();
    app.open_model_picker();
    wait_for_model_picker_load(&mut app);

    let render_filtered = |app: &mut App, filter: &str| {
        let picker = app
            .inline_interactive_state
            .as_mut()
            .expect("model picker should be open");
        picker.filter = filter.to_string();
        App::apply_inline_interactive_filter(picker);
        let _render_lock = scroll_render_test_lock();
        let backend = ratatui::backend::TestBackend::new(180, 48);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        render_and_snap(app, &mut terminal)
    };

    // Effort-capable routes now expand into multiple rows. Render focused
    // slices so each provider remains observable without assuming the complete
    // catalog fits in one terminal viewport.
    let openai_text = render_filtered(&mut app, "gpt-5.4");
    let comtegra_text = render_filtered(&mut app, "glm-51-nvfp4");
    let copilot_text = render_filtered(&mut app, "claude-opus-4.6");
    let deepseek_text = render_filtered(&mut app, "deepseek/deepseek-v4-pro");
    let kimi_text = render_filtered(&mut app, "moonshotai/kimi-k2.5");
    let openrouter_openai_text = render_filtered(&mut app, "openai/gpt-5.5");

    assert!(
        openai_text.contains("▸ GPT-5.4") && openai_text.contains("↑↓ choose"),
        "rendered /model suggestions should show the selected row and navigation, got:\n{}",
        openai_text
    );
    assert!(
        openai_text.contains("GPT-5.4")
            && openai_text.contains("OpenAI")
            && openai_text.contains("oauth")
            && openai_text.contains("api key"),
        "OpenAI OAuth and API-key routes should be separately visible, got:\n{}",
        openai_text
    );
    let glm_row = comtegra_text
        .lines()
        .find(|line| line.contains("glm-51-nvfp4"))
        .unwrap_or("");
    assert!(
        glm_row.contains("Comtegra GPU Cloud")
            && glm_row.contains("api key")
            && !glm_row.contains("copilot"),
        "Comtegra GLM row should show its provider and API-key method, got row `{}` in:\n{}",
        glm_row,
        comtegra_text
    );
    assert!(
        comtegra_text.contains("glm-51-nvfp4")
            && comtegra_text.contains("Comtegra GPU Cloud")
            && comtegra_text.contains("new"),
        "Comtegra login route should be visible and marked new, got:\n{}",
        comtegra_text
    );
    assert!(
        copilot_text.contains("Claude Opus 4.6") && copilot_text.contains("Copilot"),
        "Copilot route should be visible, got:\n{}",
        copilot_text
    );
    assert!(
        deepseek_text.contains("deepseek/deepseek-v4-pro") && deepseek_text.contains("openrouter"),
        "OpenRouter route should be visible, got:\n{}",
        deepseek_text
    );
    let deepseek_auto_row = deepseek_text
        .lines()
        .find(|line| line.contains("deepseek/deepseek-v4-pro") && line.contains("auto"))
        .unwrap_or("");
    let deepseek_provider_row = deepseek_text
        .lines()
        .find(|line| line.contains("deepseek/deepseek-v4-pro") && line.contains("DeepSeek"))
        .unwrap_or("");
    assert!(
        !deepseek_auto_row.contains('★'),
        "OpenRouter auto route should not carry the recommended marker, got row `{}` in:\n{}",
        deepseek_auto_row,
        deepseek_text
    );
    assert!(
        !deepseek_provider_row.contains('★'),
        "OpenRouter provider-specific routes should not carry the recommended marker, got row `{}` in:\n{}",
        deepseek_provider_row,
        deepseek_text
    );
    let kimi25_row = kimi_text
        .lines()
        .find(|line| line.contains("moonshotai/kimi-k2.5"))
        .unwrap_or("");
    assert!(
        !kimi25_row.contains('★'),
        "Kimi K2.5 should not be recommended, got row `{}` in:\n{}",
        kimi25_row,
        kimi_text
    );
    let openrouter_openai_row = openrouter_openai_text
        .lines()
        .find(|line| line.contains("openai/gpt-5.5"))
        .unwrap_or("");
    assert!(
        openrouter_openai_row.contains("OpenRou")
            && openrouter_openai_row.contains("openrouter")
            && !openrouter_openai_row.contains("api key"),
        "OpenRouter endpoint routes should not look like native OpenAI API-key rows, got row `{}` in:\n{}",
        openrouter_openai_row,
        openrouter_openai_text
    );
    for text in [
        &openai_text,
        &comtegra_text,
        &copilot_text,
        &deepseek_text,
        &kimi_text,
        &openrouter_openai_text,
    ] {
        assert!(
            !text.contains("(2)"),
            "provider routes should not be hidden behind stacked option counts, got:\n{}",
            text
        );
    }
}

#[test]
fn test_model_picker_filter_text_includes_provider_and_method() {
    let entry = crate::tui::PickerEntry {
        name: "glm-51-nvfp4".to_string(),
        options: vec![crate::tui::PickerOption {
            provider: "Comtegra GPU Cloud".to_string(),
            api_method: "openai-compatible:comtegra".to_string(),
            available: true,
            detail: "https://llm.comtegra.cloud/v1".to_string(),
            estimated_reference_cost_micros: None,
        }],
        action: crate::tui::PickerAction::Model,
        selected_option: 0,
        is_current: false,
        is_default: false,
        is_favorite: false,
        recommended: false,
        recommendation_rank: usize::MAX,
        usage_score: 0,
        old: false,
        created_date: None,
        effort: None,
    };

    let filter_text = crate::tui::PickerKind::Model.filter_text(&entry);
    assert!(filter_text.contains("glm-51-nvfp4"));
    assert!(filter_text.contains("Comtegra GPU Cloud"));
    assert!(filter_text.contains("openai-compatible:comtegra"));
}

#[test]
fn test_login_picker_preview_stays_open_and_updates_filter() {
    let mut app = create_test_app();

    for c in "/login za".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }

    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("login picker preview should be open");
    assert!(picker.preview);
    assert_eq!(picker.kind, crate::tui::PickerKind::Login);
    assert_eq!(picker.filter, "za");
    assert!(
        picker
            .filtered
            .iter()
            .any(|&i| picker.entries[i].name == "Z.AI")
    );
    assert_eq!(app.input(), "/login za");
}

#[test]
fn test_login_picker_preview_enter_starts_login_flow() {
    let mut app = create_test_app();

    for c in "/login zai".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    assert!(app.inline_interactive_state.is_none());
    match app.pending_login {
        Some(crate::tui::app::auth::PendingLogin::ApiKeyProfile {
            provider,
            openai_compatible_profile: Some(profile),
            ..
        }) => {
            assert_eq!(provider, "Z.AI");
            assert_eq!(profile.id, crate::provider_catalog::ZAI_PROFILE.id);
        }
        ref other => panic!("unexpected pending login state: {other:?}"),
    }
}

#[test]
fn test_typing_login_auto_inserts_filter_space() {
    let mut app = create_test_app();

    for c in "/login".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }

    // The trailing space arms provider filtering immediately, so the next
    // keystrokes filter the login picker instead of extending the command.
    assert_eq!(app.input(), "/login ");
    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("login picker preview should be open");
    assert!(picker.preview);
    assert_eq!(picker.kind, crate::tui::PickerKind::Login);
    assert_eq!(picker.filter, "");

    // A habitual manually-typed space is swallowed instead of doubling up.
    app.handle_key(KeyCode::Char(' '), KeyModifiers::empty())
        .unwrap();
    assert_eq!(app.input(), "/login ");

    for c in "za".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    assert_eq!(app.input(), "/login za");
    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("login picker preview should stay open");
    assert_eq!(picker.filter, "za");
}

#[test]
fn test_login_preview_enter_without_selection_focuses_picker_instead_of_logging_in() {
    let mut app = create_test_app();

    for c in "/login".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    // No filter and no explicit selection: Enter must not launch the first
    // provider's login flow. It focuses the picker for a deliberate choice.
    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("login picker should stay open after bare Enter");
    assert!(!picker.preview, "picker should be focused (not preview)");
    assert_eq!(picker.kind, crate::tui::PickerKind::Login);
    assert!(app.pending_login.is_none());
    assert_eq!(app.input(), "");
}

#[test]
fn test_login_preview_enter_after_navigation_starts_selected_login() {
    let mut app = create_test_app();

    for c in "/login".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    // Explicit navigation makes the selection deliberate, so Enter activates.
    // Navigate to the Anthropic API key row (an offline api-key prompt flow).
    app.handle_key(KeyCode::Down, KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Down, KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    assert!(
        app.inline_interactive_state.is_none(),
        "picker should close after selecting a provider"
    );
    assert!(
        app.pending_login.is_some(),
        "selected provider login flow should start"
    );
}

#[test]
fn test_subagent_model_command_sets_and_resets_session_preference() {
    let mut app = create_test_app();

    assert!(super::commands::handle_session_command(
        &mut app,
        "/subagent-model gpt-5.4"
    ));
    assert_eq!(app.session.subagent_model.as_deref(), Some("gpt-5.4"));

    assert!(super::commands::handle_session_command(
        &mut app,
        "/subagent-model inherit"
    ));
    assert_eq!(app.session.subagent_model, None);
}

#[test]
fn test_autoreview_command_toggles_session_preference() {
    let mut app = create_test_app();

    assert!(super::commands::handle_session_command(
        &mut app,
        "/autoreview on"
    ));
    assert_eq!(app.session.autoreview_enabled, Some(true));
    assert!(app.autoreview_enabled);

    assert!(super::commands::handle_session_command(
        &mut app,
        "/autoreview off"
    ));
    assert_eq!(app.session.autoreview_enabled, Some(false));
    assert!(!app.autoreview_enabled);
}

#[test]
fn test_autojudge_command_toggles_session_preference() {
    let mut app = create_test_app();

    assert!(super::commands::handle_session_command(
        &mut app,
        "/autojudge on"
    ));
    assert_eq!(app.session.autojudge_enabled, Some(true));
    assert!(app.autojudge_enabled);

    assert!(super::commands::handle_session_command(
        &mut app,
        "/autojudge off"
    ));
    assert_eq!(app.session.autojudge_enabled, Some(false));
    assert!(!app.autojudge_enabled);
}

#[test]
fn test_transcript_path_command_reports_current_session_file() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        let expected = crate::session::session_path(&app.session.id).expect("session path");

        assert!(super::commands::handle_session_command(
            &mut app,
            "/transcript path"
        ));

        assert!(app.display_messages().iter().any(|msg| {
            msg.content.contains("Transcript file:")
                && msg.content.contains(&expected.display().to_string())
        }));
    });
}

#[test]
fn test_poke_arms_auto_poke_until_todos_are_done() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Finish the remaining task".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        assert!(super::commands::handle_session_command(&mut app, "/poke"));

        assert!(app.auto_poke_incomplete_todos);
        assert!(app.pending_turn);
        assert!(app.display_messages().iter().any(|msg| {
            msg.content
                .contains("1 incomplete todo. We poked the agent")
                && msg.content.contains("/poke off")
        }));
    });
}

#[test]
fn test_poke_status_reports_current_state() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Finish the remaining task".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        assert!(super::commands::handle_session_command(
            &mut app,
            "/poke status"
        ));
        assert!(
            app.display_messages()
                .iter()
                .any(|msg| { msg.content.contains("Auto-poke: ON. 1 incomplete todo.") })
        );

        app.auto_poke_incomplete_todos = true;
        app.is_processing = true;
        app.queued_messages
            .push(super::commands::build_poke_message(
                &super::commands::incomplete_poke_todos(&app),
            ));
        app.hidden_queued_system_messages.push(
            "All todos are done. Todo confidence summary:\n- Weighted completion confidence: 80%."
                .to_string(),
        );

        assert!(super::commands::handle_session_command(
            &mut app,
            "/poke status"
        ));
        assert!(app.display_messages().iter().any(|msg| {
            msg.content.contains("Auto-poke: ON. 1 incomplete todo.")
                && msg.content.contains("A follow-up poke is queued.")
                && msg.content.contains("A turn is currently running.")
        }));
    });
}

#[test]
fn test_poke_off_disarms_and_clears_queued_followup() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Keep going".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        app.auto_poke_incomplete_todos = true;
        app.pending_queued_dispatch = true;
        app.queued_messages
            .push(super::commands::build_poke_message(
                &super::commands::incomplete_poke_todos(&app),
            ));
        app.hidden_queued_system_messages.push(
            "All todos are done. Todo confidence summary:\n- Weighted completion confidence: 80%."
                .to_string(),
        );

        assert!(super::commands::handle_session_command(
            &mut app,
            "/poke off"
        ));

        assert!(!app.auto_poke_incomplete_todos);
        assert!(!app.pending_queued_dispatch);
        assert!(app.queued_messages().is_empty());
        assert!(app.hidden_queued_system_messages.is_empty());
        assert_eq!(app.status_notice(), Some("Poke: OFF".to_string()));
        assert!(app.display_messages().iter().any(|msg| {
            msg.content.contains("Auto-poke disabled.")
                && msg.content.contains("Cleared 2 queued poke follow-ups")
        }));
    });
}

#[test]
fn test_poke_queues_when_turn_is_in_progress() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Finish the remaining task".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        app.is_processing = true;

        assert!(super::commands::handle_session_command(&mut app, "/poke"));

        assert!(app.auto_poke_incomplete_todos);
        assert!(app.is_processing);
        assert!(!app.cancel_requested);
        assert!(!app.pending_turn);
        assert_eq!(
            app.status_notice(),
            Some("Poke queued after current turn".to_string())
        );
        assert!(app.queued_messages().is_empty());
        assert!(app.display_messages().iter().any(|msg| {
            msg.content
                .contains("Poke queued. We'll re-check for unfinished todos after this turn")
        }));

        crate::todo::save_todos(
            &app.session.id,
            &[
                crate::todo::TodoItem {
                    group: None,
                    id: "todo-1".to_string(),
                    content: "Finish the remaining task".to_string(),
                    status: "pending".to_string(),
                    priority: "high".to_string(),
                    blocked_by: Vec::new(),
                    assigned_to: None,
                    confidence: None,
                    completion_confidence: None,
                    confidence_history: Vec::new(),
                },
                crate::todo::TodoItem {
                    group: None,
                    id: "todo-2".to_string(),
                    content: "Pick up the newly discovered task".to_string(),
                    status: "pending".to_string(),
                    priority: "medium".to_string(),
                    blocked_by: Vec::new(),
                    assigned_to: None,
                    confidence: None,
                    completion_confidence: None,
                    confidence_history: Vec::new(),
                },
            ],
        )
        .expect("save updated todos");

        super::local::finish_turn(&mut app);

        assert!(app.pending_queued_dispatch);
        assert_eq!(app.queued_messages().len(), 1);
        assert!(app.queued_messages()[0].contains("You have 2 incomplete todos"));
        assert!(!app.queued_messages()[0].contains("Pick up the newly discovered task"));
        assert!(!app.queued_messages()[0].contains("/poke off"));
    });
}

#[test]
fn test_btw_forks_even_when_turn_is_in_progress() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.is_processing = true;

        assert!(super::commands::handle_session_command(
            &mut app,
            "/btw should this fork context?"
        ));

        assert!(app.is_processing, "parent turn should keep running");
        assert!(app.queued_messages().is_empty());
        assert!(app.hidden_queued_system_messages.is_empty());
        assert!(app.display_messages().iter().any(|msg| {
            msg.content.contains("created for the next prompt")
                || msg.content.contains("Next prompt launched in")
        }));
    });
}

#[test]
fn test_finish_turn_auto_pokes_again_when_todos_remain() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Keep going".to_string(),
                status: "in_progress".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            }],
        )
        .expect("save todos");

        app.auto_poke_incomplete_todos = true;
        app.is_processing = true;
        super::local::finish_turn(&mut app);

        assert!(app.pending_queued_dispatch);
        assert_eq!(app.queued_messages().len(), 1);
        assert!(app.queued_messages()[0].contains("Continue working, or update the todo tool."));
    });
}

#[test]
fn test_finish_turn_auto_poke_queues_confidence_summary_when_todos_done() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[
                crate::todo::TodoItem {
                    group: None,
                    id: "todo-1".to_string(),
                    content: "Finish risky provider path".to_string(),
                    status: "completed".to_string(),
                    priority: "high".to_string(),
                    blocked_by: Vec::new(),
                    assigned_to: None,
                    confidence: Some(crate::todo::ConfidenceState::from_legacy_score(70)),
                    completion_confidence: Some(crate::todo::ConfidenceState::from_legacy_score(
                        80,
                    )),
                    confidence_history: Vec::new(),
                },
                crate::todo::TodoItem {
                    group: None,
                    id: "todo-2".to_string(),
                    content: "Document straightforward behavior".to_string(),
                    status: "completed".to_string(),
                    priority: "medium".to_string(),
                    blocked_by: Vec::new(),
                    assigned_to: None,
                    confidence: Some(crate::todo::ConfidenceState::from_legacy_score(90)),
                    completion_confidence: Some(crate::todo::ConfidenceState::from_legacy_score(
                        95,
                    )),
                    confidence_history: Vec::new(),
                },
            ],
        )
        .expect("save todos");

        crate::todo::save_goals(
            &app.session.id,
            &[crate::todo::TodoGoal {
                delivery_state: Some(crate::todo::DeliveryState::WorkflowValidated),
                autonomy: Some(crate::todo::Autonomy::NecessaryFollowthrough),
                iteration_maturity: Some(crate::todo::IterationMaturity::OutcomeReached),
                closed_feedback_loop: Some(crate::todo::FeedbackLoopState::Closed),
                feedback_loop: Some("verify completed work".to_string()),
                feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::Representative),
                feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::MainPaths),
                feedback_loop_traceability: Some(crate::todo::FeedbackLoopTraceability::Complete),
                ..Default::default()
            }],
        )
        .expect("save goal delivery state");

        app.auto_poke_incomplete_todos = true;
        app.is_processing = true;
        super::local::finish_turn(&mut app);

        assert!(app.auto_poke_incomplete_todos);
        assert!(app.pending_queued_dispatch);
        assert_eq!(app.queued_messages.len(), 1);
        let summary = app.queued_messages[0].clone();
        let summary = &summary;
        assert!(super::commands::is_poke_message(summary));
        assert!(super::commands::is_todo_confidence_summary_message(summary));
        assert!(summary.starts_with(crate::todo::TODO_COMPLETION_CONTINUATION_MESSAGE));
        // The continuation self-identifies as an automated follow-up so the model
        // does not mistake it for a user message, but never discloses private
        // calibration details.
        assert!(summary.starts_with("[auto]"));
        assert!(!summary.to_ascii_lowercase().contains("threshold"));
        // The model is told exactly which completed todos to recheck.
        assert!(summary.contains("Finish risky provider path"));
        assert!(summary.contains("Document straightforward behavior"));
        assert!(
            app.display_messages()
                .iter()
                .any(|msg| msg.content.contains("Double-checking confidence"))
        );

        // Dispatching the follow-up does not disarm the gate. If the model
        // finishes another turn without improving completion confidence, the
        // same validation follow-up is queued again.
        app.queued_messages.clear();
        app.pending_queued_dispatch = false;
        app.is_processing = true;
        super::local::finish_turn(&mut app);
        assert!(app.auto_poke_incomplete_todos);
        assert!(app.pending_queued_dispatch);
        assert_eq!(app.queued_messages.len(), 1);

        // Once the model records sufficient completion confidence through the
        // todo tool, the next completion check requests one clean final answer.
        let mut validated = crate::todo::load_todos(&app.session.id).expect("load todos");
        for todo in &mut validated {
            todo.completion_confidence = Some(crate::todo::ConfidenceState::from_legacy_score(100));
            todo.confidence_history = match todo.id.as_str() {
                "todo-1" => vec![
                    crate::todo::ConfidenceState::Speculative,
                    crate::todo::ConfidenceState::Plausible,
                    crate::todo::ConfidenceState::Validated,
                    crate::todo::ConfidenceState::Verified,
                ],
                _ => vec![
                    crate::todo::ConfidenceState::Validated,
                    crate::todo::ConfidenceState::Verified,
                ],
            };
        }
        crate::todo::save_todos(&app.session.id, &validated).expect("save validated todos");
        app.queued_messages.clear();
        app.pending_queued_dispatch = false;
        app.is_processing = true;
        super::local::finish_turn(&mut app);
        // Auto-poke is default-on, so a completed cycle re-arms for the next
        // batch of work rather than silently switching the feature off.
        assert_eq!(app.auto_poke_incomplete_todos, app.auto_poke_default_on);
        assert!(app.pending_queued_dispatch);
        assert_eq!(
            app.queued_messages,
            vec![crate::todo::TODO_FINAL_RESPONSE_CONTINUATION_MESSAGE.to_string()]
        );
        assert!(app.hidden_queued_system_messages.is_empty());
        assert!(app.display_messages().iter().any(|msg| {
            msg.content
                .contains("All todos done. Completion confidence: verified.")
        }));

        // The final-answer turn itself must not enqueue another final-answer
        // turn, otherwise a successfully completed cycle loops forever.
        app.queued_messages.clear();
        app.pending_queued_dispatch = false;
        app.is_processing = true;
        super::local::finish_turn(&mut app);
        assert!(!app.pending_queued_dispatch);
        assert!(app.queued_messages.is_empty());
    });
}

#[test]
fn test_todo_completion_gate_detects_abrupt_confidence_increase() {
    let summary = super::commands::todo_confidence_summary(&[crate::todo::TodoItem {
        status: "completed".to_string(),
        priority: "high".to_string(),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(0)),
        completion_confidence: Some(crate::todo::ConfidenceState::from_legacy_score(100)),
        confidence_history: vec![
            crate::todo::ConfidenceState::from_legacy_score(0),
            crate::todo::ConfidenceState::from_legacy_score(100),
        ],
        ..Default::default()
    }]);

    assert_eq!(summary.completion_average, Some(100));
    assert!(!summary.completion_confidence_needs_validation);
    assert!(summary.confidence_spike_detected);
    assert!(summary.needs_more_work);
}

#[test]
fn test_todo_completion_gate_allows_evidence_backed_confidence_steps() {
    let summary = super::commands::todo_confidence_summary(&[crate::todo::TodoItem {
        status: "completed".to_string(),
        priority: "high".to_string(),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(100)),
        completion_confidence: Some(crate::todo::ConfidenceState::from_legacy_score(100)),
        confidence_history: vec![
            crate::todo::ConfidenceState::Speculative,
            crate::todo::ConfidenceState::Plausible,
            crate::todo::ConfidenceState::Validated,
            crate::todo::ConfidenceState::Verified,
        ],
        ..Default::default()
    }]);

    assert_eq!(summary.completion_average, Some(100));
    assert!(!summary.completion_confidence_needs_validation);
    assert!(!summary.confidence_spike_detected);
    assert!(!summary.needs_more_work);
}

#[test]
fn test_finish_turn_challenges_confidence_spike_once() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                id: "todo-1".to_string(),
                content: "Validate provider result".to_string(),
                status: "completed".to_string(),
                priority: "high".to_string(),
                confidence: Some(crate::todo::ConfidenceState::from_legacy_score(100)),
                completion_confidence: Some(crate::todo::ConfidenceState::from_legacy_score(100)),
                confidence_history: vec![
                    crate::todo::ConfidenceState::from_legacy_score(70),
                    crate::todo::ConfidenceState::from_legacy_score(100),
                ],
                ..Default::default()
            }],
        )
        .expect("save todos");

        crate::todo::save_goals(
            &app.session.id,
            &[crate::todo::TodoGoal {
                delivery_state: Some(crate::todo::DeliveryState::WorkflowValidated),
                autonomy: Some(crate::todo::Autonomy::NecessaryFollowthrough),
                iteration_maturity: Some(crate::todo::IterationMaturity::OutcomeReached),
                feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::Representative),
                feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::MainPaths),
                feedback_loop_traceability: Some(crate::todo::FeedbackLoopTraceability::Complete),
                ..Default::default()
            }],
        )
        .expect("save passing goal");

        app.auto_poke_incomplete_todos = true;
        app.is_processing = true;
        super::local::finish_turn(&mut app);

        assert!(app.auto_poke_incomplete_todos);
        assert!(app.todo_confidence_spike_challenged);
        assert!(app.pending_queued_dispatch);
        assert_eq!(app.queued_messages.len(), 1);
        assert!(
            app.queued_messages[0]
                .starts_with(crate::todo::TODO_CONFIDENCE_SPIKE_CONTINUATION_MESSAGE)
        );
        assert!(
            app.display_messages()
                .iter()
                .any(|msg| { msg.content.contains("Double-checking confidence jumps") })
        );

        app.queued_messages.clear();
        app.pending_queued_dispatch = false;
        app.is_processing = true;
        super::local::finish_turn(&mut app);

        assert!(app.auto_poke_incomplete_todos);
        assert!(app.todo_confidence_spike_challenged);
        assert!(app.pending_queued_dispatch);
        assert_eq!(
            app.queued_messages,
            vec![crate::todo::TODO_FINAL_RESPONSE_CONTINUATION_MESSAGE.to_string()]
        );

        // Finishing the synthetic final-response turn must not challenge the
        // same unchanged confidence history again.
        app.queued_messages.clear();
        app.pending_queued_dispatch = false;
        app.is_processing = true;
        super::local::finish_turn(&mut app);

        assert!(app.auto_poke_incomplete_todos);
        assert!(app.todo_confidence_spike_challenged);
        assert!(!app.pending_queued_dispatch);
        assert!(app.queued_messages.is_empty());
        assert_eq!(
            app.display_messages()
                .iter()
                .filter(|message| message.content.contains("All todos done"))
                .count(),
            1
        );
    });
}
