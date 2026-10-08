#[test]
fn test_handle_key_escape_clears_input() {
    let mut app = create_test_app();

    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('e'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('s'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input(), "test");

    app.handle_key(KeyCode::Esc, KeyModifiers::empty()).unwrap();

    assert!(app.input().is_empty());
    assert_eq!(app.cursor_pos(), 0);
    assert_eq!(
        app.status_notice(),
        Some("Input cleared - Ctrl+Z to restore".to_string())
    );
}

#[test]
fn test_handle_key_ctrl_z_restores_escaped_input() {
    let mut app = create_test_app();

    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('e'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('s'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Esc, KeyModifiers::empty()).unwrap();

    app.handle_key(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .unwrap();

    assert_eq!(app.input(), "test");
    assert_eq!(app.cursor_pos(), 4);
    assert_eq!(app.status_notice(), Some("↶ Input restored".to_string()));
}

#[test]
fn test_handle_key_ctrl_z_undoes_typing() {
    let mut app = create_test_app();

    app.handle_key(KeyCode::Char('a'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('b'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('c'), KeyModifiers::empty())
        .unwrap();

    app.handle_key(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.input(), "");
    assert_eq!(app.cursor_pos(), 0);

    app.handle_key(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.input(), "");
    assert_eq!(app.cursor_pos(), 0);
}

#[test]
fn test_handle_key_ctrl_u_clears_input() {
    let mut app = create_test_app();

    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('e'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('s'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();

    app.handle_key(KeyCode::Char('u'), KeyModifiers::CONTROL)
        .unwrap();

    assert!(app.input().is_empty());
    assert_eq!(app.cursor_pos(), 0);
}

#[test]
fn test_submit_input_adds_message() {
    let mut app = create_test_app();

    // Type and submit
    app.handle_key(KeyCode::Char('h'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('i'), KeyModifiers::empty())
        .unwrap();
    app.submit_input();

    // Check message was added to display
    assert_eq!(app.display_messages().len(), 1);
    assert_eq!(app.display_messages()[0].role, "user");
    assert_eq!(app.display_messages()[0].content, "hi");

    // Check processing state
    assert!(app.is_processing());
    assert!(app.pending_turn);
    assert!(app.session_save_pending);
    assert!(matches!(app.status(), ProcessingStatus::Sending));
    assert!(app.elapsed().is_some());

    // Input should be cleared
    assert!(app.input().is_empty());
}

#[test]
fn test_submit_input_commits_pending_streaming_assistant_text_before_user_message() {
    let mut app = create_test_app();
    app.display_messages.push(DisplayMessage::tool(
        "file contents",
        crate::message::ToolCall {
            id: "tool_read".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
            intent: None,
            thought_signature: None,
        },
    ));
    app.bump_display_messages_version();
    app.streaming.streaming_text = "Here is the final paragraph".to_string();
    // Mirror the real streaming caller: append any paced chunk the buffer reveals.
    // The paced StreamBuffer may reveal part of the text immediately, so commit
    // (below) must still flush the remainder.
    let ops = app.stream_buffer.push_text(" that was still buffered.");
    app.apply_stream_ops(ops);

    app.input = "follow up".to_string();
    app.cursor_pos = app.input.len();
    app.submit_input();

    assert_eq!(app.display_messages().len(), 3);
    assert_eq!(app.display_messages()[0].role, "tool");
    assert_eq!(app.display_messages()[1].role, "assistant");
    assert_eq!(
        app.display_messages()[1].content,
        "Here is the final paragraph that was still buffered."
    );
    assert_eq!(app.display_messages()[2].role, "user");
    assert_eq!(app.display_messages()[2].content, "follow up");
    assert!(app.streaming_text().is_empty());
    assert!(app.stream_buffer.is_empty());
}

#[test]
fn test_queue_message_while_processing() {
    let mut app = create_test_app();
    app.queue_mode = true;

    // Simulate processing state
    app.is_processing = true;

    // Type a message
    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('e'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('s'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('t'), KeyModifiers::empty())
        .unwrap();

    // Press Enter should queue, not submit
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.queued_count(), 1);
    assert!(app.input().is_empty());

    // Queued messages are stored in queued_messages, not display_messages
    assert_eq!(app.queued_messages()[0], "test");
    assert!(app.display_messages().is_empty());
}

#[test]
fn test_ctrl_tab_toggles_queue_mode() {
    let mut app = create_test_app();

    assert!(!app.queue_mode);

    app.handle_key(KeyCode::Char('t'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(app.queue_mode);

    app.handle_key(KeyCode::Char('t'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(!app.queue_mode);
}

#[test]
fn test_auto_poke_starts_enabled_by_default() {
    // The default comes from config, which a concurrent test may be
    // rewriting under its own temp home; take the env lock and a clean home.
    with_temp_jcode_home(|| {
        let app = create_test_app();

        assert!(app.auto_poke_incomplete_todos);
    });
}

#[test]
fn test_ctrl_p_toggles_auto_poke_locally() {
    with_temp_jcode_home(ctrl_p_toggles_auto_poke_locally);
}

fn ctrl_p_toggles_auto_poke_locally() {
    let mut app = create_test_app();

    assert!(app.auto_poke_incomplete_todos);

    app.handle_key(KeyCode::Char('p'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(!app.auto_poke_incomplete_todos);
    assert_eq!(app.status_notice(), Some("Poke: OFF".to_string()));

    app.handle_key(KeyCode::Char('p'), KeyModifiers::CONTROL)
        .unwrap();
    assert!(app.auto_poke_incomplete_todos);
    assert_eq!(app.status_notice(), Some("Poke: ON".to_string()));
    assert!(app.display_messages().iter().any(|msg| {
        msg.content
            .contains("Auto-poke enabled. Nothing unfinished right now")
    }));
}

#[test]
fn test_transfer_command_queues_pause_while_processing_locally() {
    let mut app = create_test_app();
    app.is_processing = true;

    super::commands::handle_transfer_command_local(&mut app);

    assert!(app.pending_transfer_request);
    let pause_message = super::commands::transfer_pause_message();
    assert_eq!(
        app.interleave_message.as_deref(),
        Some(pause_message.as_str())
    );
    assert_eq!(
        app.status_notice(),
        Some("Transfer queued after current turn".to_string())
    );
}

#[test]
fn test_create_transfer_session_from_parent_copies_todos_and_uses_compacted_context_only() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.session.working_dir = Some("/tmp".to_string());
        app.session.model = Some("test-model".to_string());
        app.session.provider_key = Some("test-provider".to_string());
        app.session.system_prompt = Some("custom transfer prompt".to_string());
        app.session.messages.push(crate::session::StoredMessage {
            id: "msg-1".to_string(),
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "full transcript should not be copied".to_string(),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
        let transfer_compaction = crate::session::StoredCompactionState {
            summary_text: "Compacted handoff summary".to_string(),
            openai_encrypted_content: None,
            covers_up_to_turn: 1,
            original_turn_count: 1,
            compacted_count: 0,
        };
        crate::todo::save_todos(
            &app.session.id,
            &[crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "Carry this forward".to_string(),
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

        let (child_id, _) = super::commands::create_transfer_session_from_parent(
            &app.session.id,
            &app.session,
            Some(transfer_compaction.clone()),
        )
        .expect("create transfer session");
        let child = crate::session::Session::load(&child_id).expect("load child session");
        let child_todos = crate::todo::load_todos(&child_id).expect("load child todos");

        assert_eq!(child.parent_id.as_deref(), Some(app.session.id.as_str()));
        assert!(child.messages.is_empty());
        assert_eq!(child.compaction, Some(transfer_compaction));
        assert_eq!(child.model.as_deref(), Some("test-model"));
        assert_eq!(child.provider_key.as_deref(), Some("test-provider"));
        assert_eq!(child.system_prompt, app.session.system_prompt);
        assert_eq!(child.working_dir.as_deref(), Some("/tmp"));
        assert_eq!(child_todos.len(), 1);
        assert_eq!(child_todos[0].content, "Carry this forward");
        for prompt in [None, Some("")] {
            app.session.system_prompt = prompt.map(str::to_string);
            let (child_id, _) = super::commands::create_transfer_session_from_parent(
                &app.session.id,
                &app.session,
                None,
            )
            .expect("create transfer session with optional prompt");
            let child = crate::session::Session::load(&child_id).expect("load child session");
            assert_eq!(child.system_prompt, app.session.system_prompt);
        }
    });
}

#[test]
fn test_shift_enter_inserts_newline() {
    let mut app = create_test_app();
    app.is_processing = true;

    app.handle_key(KeyCode::Char('h'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::SHIFT).unwrap();
    app.handle_key(KeyCode::Char('i'), KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input(), "h\ni");
    assert_eq!(app.queued_count(), 0);
    assert_eq!(app.interleave_message.as_deref(), None);
}

#[test]
fn test_alt_enter_inserts_newline() {
    let mut app = create_test_app();
    app.is_processing = true;

    app.handle_key(KeyCode::Char('h'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::ALT).unwrap();
    app.handle_key(KeyCode::Char('i'), KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input(), "h\ni");
    assert_eq!(app.queued_count(), 0);
    assert_eq!(app.interleave_message.as_deref(), None);
}
#[test]
fn test_ctrl_enter_opposite_send_mode() {
    let mut app = create_test_app();
    app.is_processing = true;

    // Default immediate mode: Ctrl+Enter should queue
    app.handle_key(KeyCode::Char('h'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('i'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::CONTROL)
        .unwrap();

    assert_eq!(app.queued_count(), 1);
    assert_eq!(app.interleave_message.as_deref(), None);
    assert!(app.input().is_empty());

    // Queue mode: Ctrl+Enter should interleave (sets interleave_message, not queued)
    app.queue_mode = true;
    app.handle_key(KeyCode::Char('y'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('o'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::CONTROL)
        .unwrap();

    // Interleave now sets interleave_message instead of adding to queue
    assert_eq!(app.queued_count(), 1); // Still just "hi" in queue
    assert_eq!(app.interleave_message.as_deref(), Some("yo")); // "yo" is for interleave
}

#[test]
fn test_cmd_enter_opposite_send_mode() {
    let mut app = create_test_app();
    app.is_processing = true;

    // Default immediate mode: Cmd+Enter should queue, matching Ctrl+Enter
    app.handle_key(KeyCode::Char('h'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('i'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::SUPER).unwrap();

    assert_eq!(app.queued_count(), 1);
    assert_eq!(app.interleave_message.as_deref(), None);
    assert!(app.input().is_empty());

    // Queue mode: Cmd+Enter should interleave (sets interleave_message, not queued)
    app.queue_mode = true;
    app.handle_key(KeyCode::Char('y'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('o'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::SUPER).unwrap();

    assert_eq!(app.queued_count(), 1); // Still just "hi" in queue
    assert_eq!(app.interleave_message.as_deref(), Some("yo")); // "yo" is for interleave
}

#[test]
fn test_typing_during_processing() {
    let mut app = create_test_app();
    app.is_processing = true;

    // Should still be able to type
    app.handle_key(KeyCode::Char('a'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('b'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('c'), KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.input(), "abc");
}

#[test]
fn test_ctrl_c_requests_cancel_while_processing() {
    let mut app = create_test_app();
    app.is_processing = true;
    app.interleave_message = Some("queued interrupt".to_string());
    app.pending_soft_interrupts
        .push("pending soft interrupt".to_string());

    app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL)
        .unwrap();

    assert!(app.cancel_requested);
    assert!(app.interleave_message.is_none());
    assert!(app.pending_soft_interrupts.is_empty());
    assert_eq!(app.status_notice(), Some("Interrupting...".to_string()));
}

#[test]
fn test_escape_interrupt_disables_auto_poke_while_processing() {
    let mut app = create_test_app();
    app.is_processing = true;
    app.auto_poke_incomplete_todos = true;
    app.queued_messages
        .push(super::commands::build_poke_message(&[
            crate::todo::TodoItem {
                group: None,
                id: "todo-1".to_string(),
                content: "keep going".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                blocked_by: Vec::new(),
                assigned_to: None,
                confidence: None,
                completion_confidence: None,
                confidence_history: Vec::new(),
            },
        ]));

    app.handle_key(KeyCode::Esc, KeyModifiers::empty()).unwrap();

    assert!(app.cancel_requested);
    assert!(!app.auto_poke_incomplete_todos);
    assert!(app.queued_messages.is_empty());
    assert_eq!(
        app.status_notice(),
        Some("Interrupting... Auto-poke OFF".to_string())
    );
}

#[test]
fn test_ctrl_c_still_arms_quit_when_idle() {
    let mut app = create_test_app();

    app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL)
        .unwrap();

    assert!(!app.cancel_requested);
    assert!(app.quit_pending.is_some());
    assert_eq!(
        app.status_notice(),
        Some("Press Ctrl+C again to quit".to_string())
    );
}

#[test]
fn test_ctrl_x_cuts_entire_input_line_to_clipboard() {
    let mut app = create_test_app();
    app.input = "hello world".to_string();
    app.cursor_pos = 5;

    let copied = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let copied_for_closure = copied.clone();

    let cut = super::input::cut_input_line_to_clipboard_with(&mut app, |text| {
        *copied_for_closure.lock().unwrap() = text.to_string();
        true
    });

    assert!(cut);
    assert_eq!(&*copied.lock().unwrap(), "hello world");
    assert!(app.input().is_empty());
    assert_eq!(app.cursor_pos(), 0);
    assert_eq!(app.status_notice(), Some("✂ Cut input line".to_string()));

    app.handle_key(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .unwrap();
    assert_eq!(app.input(), "hello world");
    assert_eq!(app.cursor_pos(), 5);
}

#[test]
fn test_ctrl_x_preserves_input_when_clipboard_copy_fails() {
    let mut app = create_test_app();
    app.input = "hello world".to_string();
    app.cursor_pos = 5;

    let cut = super::input::cut_input_line_to_clipboard_with(&mut app, |_text| false);

    assert!(!cut);
    assert_eq!(app.input(), "hello world");
    assert_eq!(app.cursor_pos(), 5);
    assert_eq!(
        app.status_notice(),
        Some("Failed to copy input line".to_string())
    );
}

#[test]
fn test_ctrl_a_keeps_home_behavior_when_input_present() {
    let mut app = create_test_app();
    app.input = "hello world".to_string();
    app.cursor_pos = app.input.len();

    app.handle_key(KeyCode::Char('a'), KeyModifiers::CONTROL)
        .unwrap();

    assert_eq!(app.input(), "hello world");
    assert_eq!(app.cursor_pos(), 0);
}

#[test]
fn test_retrieve_pending_message_edits_queued_message() {
    let mut app = create_test_app();
    app.queue_mode = true;
    app.is_processing = true;

    // Type and queue a message
    app.handle_key(KeyCode::Char('h'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('e'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('l'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('l'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Char('o'), KeyModifiers::empty())
        .unwrap();
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    assert_eq!(app.queued_count(), 1);
    assert!(app.input().is_empty());

    app.handle_key(KeyCode::Up, KeyModifiers::CONTROL).unwrap();

    assert_eq!(app.queued_count(), 0);
    assert_eq!(app.input(), "hello");
    assert_eq!(app.cursor_pos(), 5); // Cursor at end
}

#[test]
fn test_retrieve_pending_message_with_alt_and_super_up() {
    // Ctrl+Up, Alt(Option)+Up and Cmd(Super)+Up must all recall a queued message
    // so the gesture works regardless of which modifier the terminal forwards.
    // Alt+Up only recalls when the speed-tier keys are unbound.
    for modifier in [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SUPER,
    ] {
        let mut app = create_test_app();
        app.speed_switch_keys = crate::tui::keybind::SpeedSwitchKeys::default();
        app.queue_mode = true;
        app.is_processing = true;

        for c in "hello".chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
                .unwrap();
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::empty())
            .unwrap();

        assert_eq!(app.queued_count(), 1, "modifier {modifier:?}");
        assert!(app.input().is_empty(), "modifier {modifier:?}");

        app.handle_key(KeyCode::Up, modifier).unwrap();

        assert_eq!(app.queued_count(), 0, "modifier {modifier:?}");
        assert_eq!(app.input(), "hello", "modifier {modifier:?}");
        assert_eq!(app.cursor_pos(), 5, "modifier {modifier:?}");
    }
}

#[test]
fn test_alt_up_down_cycle_speed_tier_by_default() {
    use crate::tui::TuiState as _;
    let mut app = create_test_app();
    app.speed_switch_keys = crate::tui::keybind::SpeedSwitchKeys {
        increase: Some(crate::tui::keybind::KeyBinding {
            code: KeyCode::Up,
            modifiers: KeyModifiers::ALT,
        }),
        decrease: Some(crate::tui::keybind::KeyBinding {
            code: KeyCode::Down,
            modifiers: KeyModifiers::ALT,
        }),
    };
    app.set_input_for_test("draft");

    app.handle_key(KeyCode::Up, KeyModifiers::ALT).unwrap();

    // The speed key is consumed: no history recall, draft untouched, and a
    // speed notice (or an availability notice for the mock provider) shows.
    assert_eq!(app.input(), "draft");
    let notice = app.status_notice().unwrap_or_default();
    assert!(notice.starts_with("Speed"), "unexpected notice: {notice}");

    app.handle_key(KeyCode::Down, KeyModifiers::ALT).unwrap();
    assert_eq!(app.input(), "draft");
    let notice = app.status_notice().unwrap_or_default();
    assert!(notice.starts_with("Speed"), "unexpected notice: {notice}");
}

#[test]
fn test_local_speed_cycle_walks_standard_fast_ultrafast_and_clamps() {
    use crate::tui::TuiState as _;
    use std::sync::{Arc as StdArc, Mutex as StdMutex};

    #[derive(Clone)]
    struct UltraMock(StdArc<StdMutex<Option<String>>>);

    #[async_trait::async_trait]
    impl Provider for UltraMock {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[crate::message::ToolDefinition],
            _system: &str,
            _resume_session_id: Option<&str>,
        ) -> Result<crate::provider::EventStream> {
            unimplemented!("UltraMock")
        }
        fn name(&self) -> &str {
            "openai"
        }
        fn model(&self) -> String {
            "gpt-6-astra".to_string()
        }
        fn fork(&self) -> Arc<dyn Provider> {
            Arc::new(self.clone())
        }
        fn service_tier(&self) -> Option<String> {
            self.0.lock().unwrap().clone()
        }
        fn set_service_tier(&self, tier: &str) -> anyhow::Result<()> {
            *self.0.lock().unwrap() = match tier {
                "off" => None,
                other => Some(other.to_string()),
            };
            Ok(())
        }
    }

    let tier = StdArc::new(StdMutex::new(None));
    let provider: Arc<dyn Provider> = Arc::new(UltraMock(tier.clone()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let registry = rt.block_on(crate::tool::Registry::new(provider.clone()));
    let mut app = App::new_for_test_harness(provider, registry);
    app.speed_switch_keys = crate::tui::keybind::SpeedSwitchKeys {
        increase: Some(crate::tui::keybind::KeyBinding {
            code: KeyCode::Up,
            modifiers: KeyModifiers::ALT,
        }),
        decrease: Some(crate::tui::keybind::KeyBinding {
            code: KeyCode::Down,
            modifiers: KeyModifiers::ALT,
        }),
    };

    let press = |app: &mut App, code| {
        app.handle_key(code, KeyModifiers::ALT).unwrap();
        (
            tier.lock().unwrap().clone(),
            app.status_notice().unwrap_or_default(),
        )
    };

    assert_eq!(
        press(&mut app, KeyCode::Up),
        (Some("priority".into()), "Speed: Fast ○●○".into())
    );
    assert_eq!(
        press(&mut app, KeyCode::Up),
        (Some("ultrafast".into()), "Speed: Ultrafast ○○●".into())
    );
    assert_eq!(
        press(&mut app, KeyCode::Up),
        (
            Some("ultrafast".into()),
            "Speed: Ultrafast ○○● (already at max)".into()
        )
    );
    assert_eq!(
        press(&mut app, KeyCode::Down),
        (Some("priority".into()), "Speed: Fast ○●○".into())
    );
    assert_eq!(
        press(&mut app, KeyCode::Down),
        (None, "Speed: Standard ●○○".into())
    );
    assert_eq!(
        press(&mut app, KeyCode::Down),
        (None, "Speed: Standard ●○○ (already at min)".into())
    );
}

#[test]
fn test_remote_alt_up_sends_next_speed_tier_over_the_wire() {
    use tokio::io::AsyncBufReadExt;

    let rt = tokio::runtime::Runtime::new().unwrap();
    for (current, expected) in [(None, "priority"), (Some("priority"), "ultrafast")] {
        let mut app = create_test_app();
        app.is_remote = true;
        app.remote_provider_name = Some("openai".to_string());
        app.remote_provider_model = Some("gpt-6-astra".to_string());
        app.remote_service_tier = current.map(str::to_string);
        app.speed_switch_keys = crate::tui::keybind::SpeedSwitchKeys {
            increase: Some(crate::tui::keybind::KeyBinding {
                code: KeyCode::Up,
                modifiers: KeyModifiers::ALT,
            }),
            decrease: None,
        };
        rt.block_on(async {
            let mut remote = crate::tui::backend::RemoteConnection::dummy();
            let peer = remote.take_dummy_peer().unwrap();
            let mut reader = tokio::io::BufReader::new(peer);
            app.handle_remote_key(KeyCode::Up, KeyModifiers::ALT, &mut remote)
                .await
                .unwrap();
            let mut line = String::new();
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                reader.read_line(&mut line),
            )
            .await
            .expect("speed key must send a wire request")
            .unwrap();
            let request: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(request["type"], "set_service_tier", "request: {line}");
            assert_eq!(request["service_tier"], expected, "request: {line}");
        });
    }
}

#[test]
fn test_retrieve_pending_message_prefers_pending_interleave_for_editing() {
    let mut app = create_test_app();
    app.is_processing = true;
    app.queue_mode = false; // Enter=interleave, Ctrl+Enter=queue

    for c in "urgent".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    for c in "later".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .unwrap();
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::CONTROL)
        .unwrap();

    assert_eq!(app.interleave_message.as_deref(), Some("urgent"));
    assert_eq!(app.queued_count(), 1);

    app.retrieve_pending_message_for_edit();

    assert_eq!(app.input(), "urgent\n\nlater");
    assert_eq!(app.interleave_message.as_deref(), None);
    assert_eq!(app.queued_count(), 0);
}

#[test]
fn test_send_action_modes() {
    let mut app = create_test_app();
    app.is_processing = true;
    app.queue_mode = false;

    assert_eq!(app.send_action(false), SendAction::Interleave);
    assert_eq!(app.send_action(true), SendAction::Queue);

    app.queue_mode = true;
    assert_eq!(app.send_action(false), SendAction::Queue);
    assert_eq!(app.send_action(true), SendAction::Interleave);

    app.is_processing = false;
    assert_eq!(app.send_action(false), SendAction::Submit);
}

#[test]
fn test_interleave_submission_preserves_pending_images() {
    let mut app = create_test_app();
    app.is_processing = true;
    app.queue_mode = false;
    app.input = "[image 1] describe this".to_string();
    app.cursor_pos = app.input.len();
    let images = vec![("image/png".to_string(), "ZmFrZQ==".to_string())];
    app.pending_images = images.clone();

    assert!(input::handle_enter(&mut app));

    assert_eq!(
        app.interleave_message.as_deref(),
        Some("[image 1] describe this")
    );
    assert_eq!(app.interleave_images, images);
    assert!(app.pending_images.is_empty());
}

#[test]
fn test_send_action_submits_bang_commands_while_processing() {
    let mut app = create_test_app();
    app.is_processing = true;
    app.input = "!pwd".to_string();

    assert_eq!(app.send_action(false), SendAction::Submit);
    assert_eq!(app.send_action(true), SendAction::Submit);
}

#[test]
fn test_handle_input_shell_completed_renders_markdown_blocks() {
    let mut app = create_test_app();
    let event = BusEvent::InputShellCompleted(InputShellCompleted {
        session_id: app.session.id.clone(),
        result: crate::message::InputShellResult {
            command: "ls -la".to_string(),
            cwd: Some("/tmp/project".to_string()),
            output: "Cargo.toml\nsrc\n".to_string(),
            exit_code: Some(0),
            duration_ms: 42,
            truncated: false,
            failed_to_start: false,
        },
    });

    super::local::handle_bus_event(&mut app, Ok(event));

    let rendered = app.display_messages().last().expect("shell result message");
    assert_eq!(rendered.role, "system");
    assert!(rendered.content.contains("Shell command"));
    assert!(rendered.content.contains("ls -la"));
    assert!(rendered.content.contains("Cargo.toml"));
    assert_eq!(
        app.status_notice(),
        Some("Shell command completed".to_string())
    );
}

/// Regression for issue #427: selecting an effort-variant model row (e.g.
/// "gpt-5.5 (high)") in the remote model picker must stage the chosen effort
/// alongside the pending model switch. Previously only the model spec was
/// staged, so the server kept its configured default effort (low) and the
/// session silently ran gpt-5.5 at low effort.
#[test]
fn test_model_picker_effort_variant_selection_stages_effort_in_remote_mode() {
    let mut app = create_test_app();
    configure_test_remote_models_with_openai_recommendations(&mut app);

    app.open_model_picker();

    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("model picker should be open");

    let entry_idx = picker
        .entries
        .iter()
        .position(|m| m.name == "gpt-5.5 (high)")
        .expect("gpt-5.5 (high) should be in picker");
    assert_eq!(
        picker.entries[entry_idx].effort.as_deref(),
        Some("high"),
        "effort variant rows must carry their effort"
    );

    let filtered_pos = picker
        .filtered
        .iter()
        .position(|&i| i == entry_idx)
        .expect("gpt-5.5 (high) should be in filtered list");
    app.inline_interactive_state.as_mut().unwrap().selected = filtered_pos;

    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    assert!(
        app.inline_interactive_state.is_none(),
        "picker should close"
    );
    assert!(
        app.pending_route_selection.is_some(),
        "model switch should be staged for the remote dispatcher"
    );
    assert_eq!(
        app.pending_reasoning_effort.as_deref(),
        Some("high"),
        "the picked effort variant must be staged so it reaches the server (issue #427)"
    );
}

#[test]
fn test_model_picker_effort_variants_follow_each_route_vocabulary() {
    let mut app = create_test_app();
    configure_test_remote_models_with_openai_recommendations(&mut app);
    app.remote_model_options.push(crate::provider::ModelRoute {
        model: "gpt-5.5".to_string(),
        provider: "OpenRouter".to_string(),
        api_method: "openrouter".to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    });

    app.open_model_picker();
    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("model picker should be open");
    let has_route_effort = |api_method: &str, effort: &str| {
        picker.entries.iter().any(|entry| {
            entry.name.starts_with("gpt-5.5 (")
                && entry.effort.as_deref() == Some(effort)
                && entry
                    .options
                    .first()
                    .is_some_and(|route| route.api_method == api_method)
        })
    };

    assert!(has_route_effort("openai-oauth", "max"));
    assert!(has_route_effort("openai-oauth", "minimal"));
    assert!(has_route_effort("openrouter", "xhigh"));
    assert!(has_route_effort("openrouter", "minimal"));
    assert!(
        !has_route_effort("openrouter", "max"),
        "OpenRouter must not advertise max as a distinct rung because it aliases xhigh"
    );
}

/// Plain model rows (no effort suffix) must not stage a reasoning effort.
/// Routes whose runtime cannot apply a reasoning effort (e.g. Copilot) get
/// plain rows even for models that have an effort ladder elsewhere.
#[test]
fn test_model_picker_plain_selection_stages_no_effort_in_remote_mode() {
    let mut app = create_test_app();
    configure_test_remote_models_with_openai_recommendations(&mut app);
    // A Copilot-backed route cannot apply per-request reasoning effort, so it
    // must render as a plain row (issue #458 route gating).
    app.remote_model_options.push(crate::provider::ModelRoute {
        model: "claude-opus-4-8".to_string(),
        provider: "Copilot".to_string(),
        api_method: "copilot".to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    });

    app.open_model_picker();

    let picker = app
        .inline_interactive_state
        .as_ref()
        .expect("model picker should be open");

    let entry_idx = picker
        .entries
        .iter()
        .position(|m| m.name == "claude-opus-4-8" && m.effort.is_none())
        .expect("claude-opus-4-8 should be in picker without an effort variant");

    let filtered_pos = picker
        .filtered
        .iter()
        .position(|&i| i == entry_idx)
        .expect("claude-opus-4-8 should be in filtered list");
    app.inline_interactive_state.as_mut().unwrap().selected = filtered_pos;

    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .unwrap();

    assert!(
        app.inline_interactive_state.is_none(),
        "picker should close"
    );
    assert!(
        app.pending_reasoning_effort.is_none(),
        "plain rows must not override the server's effort"
    );
}

#[test]
fn test_model_switch_notice_omits_placeholder_route_details() {
    // Selecting a model whose chosen row is a placeholder ("remote-catalog")
    // must not advertise a bogus provider/method or the "refreshing route
    // details…" text; those describe a catalog still being refreshed.
    with_temp_jcode_home(|| {
        let model = "placeholder-only-model";
        let mut app = create_test_app();
        app.is_remote = true;
        app.remote_provider_name = Some("Some Server".to_string());
        app.remote_provider_model = Some("other-model".to_string());
        app.remote_available_entries = vec![model.to_string()];
        app.remote_model_options = vec![crate::provider::ModelRoute {
            model: model.to_string(),
            provider: "Some Server".to_string(),
            api_method: "remote-catalog".to_string(),
            available: true,
            detail: "refreshing route details…".to_string(),
            usage: None,
            cheapness: None,
        }];

        app.open_model_picker();
        // Placeholder-only entries also get real routes synthesized, so locate
        // the entry/option pair that is still the placeholder and pick it.
        let (entry_idx, option_idx) = {
            let picker = app
                .inline_interactive_state
                .as_ref()
                .expect("model picker should be open");
            picker
                .entries
                .iter()
                .enumerate()
                .find_map(|(entry_idx, entry)| {
                    entry
                        .options
                        .iter()
                        .position(|route| route.api_method == "remote-catalog")
                        .map(|option_idx| (entry_idx, option_idx))
                })
                .expect("a placeholder route option should be present")
        };
        let filtered_pos = app
            .inline_interactive_state
            .as_ref()
            .unwrap()
            .filtered
            .iter()
            .position(|&i| i == entry_idx)
            .expect("placeholder entry should be in the filtered list");
        {
            let picker = app.inline_interactive_state.as_mut().unwrap();
            picker.selected = filtered_pos;
            picker.entries[entry_idx].selected_option = option_idx;
        }

        app.handle_key(KeyCode::Enter, KeyModifiers::empty())
            .unwrap();

        let notice = app
            .status_notice
            .as_ref()
            .map(|(text, _)| text.clone())
            .expect("a model switch notice should be set");
        assert!(!notice.contains("remote-catalog"), "got {notice}");
        assert!(!notice.contains("refreshing route details"), "got {notice}");
        assert!(notice.starts_with("Model → "), "got {notice}");
        assert!(!notice.contains(" via "), "got {notice}");
    });
}

#[test]
fn test_favorite_hotkey_does_not_confirm_remote_placeholder_without_matching_favorite() {
    with_temp_jcode_home(|| {
        let model = "placeholder-favorite-hotkey-model";
        let mut app = create_test_app();
        app.is_remote = true;
        app.remote_provider_name = Some("Some Server".to_string());
        app.remote_provider_model = Some(model.to_string());
        app.remote_available_entries = vec![model.to_string()];
        app.remote_model_options = vec![crate::provider::ModelRoute {
            model: model.to_string(),
            provider: "Some Server".to_string(),
            api_method: "remote-catalog".to_string(),
            available: true,
            detail: "refreshing route details…".to_string(),
            usage: None,
            cheapness: None,
        }];

        app.cycle_model_favorite_hotkey();

        assert!(app.inline_interactive_state.is_some());
        assert!(app.pending_model_switch.is_none());
    });
}

#[test]
fn test_catalog_update_rebuilds_open_model_picker_with_real_routes() {
    // A picker opened while the catalog is still names-only shows placeholder
    // rows. When the detailed catalog lands, the open picker must be rebuilt
    // rather than left stale until the user reopens it.
    with_temp_jcode_home(|| {
        let model = "gpt-5.5";
        let mut app = create_test_app();
        app.is_remote = true;
        app.remote_provider_name = Some("OpenAI".to_string());
        app.remote_provider_model = Some(model.to_string());
        app.remote_available_entries = vec![model.to_string()];
        app.remote_model_options = vec![crate::provider::ModelRoute {
            model: model.to_string(),
            provider: "OpenAI".to_string(),
            api_method: "remote-catalog".to_string(),
            available: true,
            detail: "refreshing route details…".to_string(),
            usage: None,
            cheapness: None,
        }];

        app.open_model_picker();
        assert!(app.inline_interactive_state.is_some());

        app.remote_model_options = vec![crate::provider::ModelRoute {
            model: model.to_string(),
            provider: "OpenAI".to_string(),
            api_method: "openai-api".to_string(),
            available: true,
            detail: String::new(),
            usage: None,
            cheapness: None,
        }];
        app.invalidate_model_picker_cache();
        app.refresh_open_model_picker_after_catalog_update();

        let picker = app
            .inline_interactive_state
            .as_ref()
            .expect("picker should still be open after the catalog update");
        assert!(
            picker
                .entries
                .iter()
                .any(|entry| entry.name.starts_with(model)),
            "rebuilt picker should still list the model"
        );
        assert!(
            picker.entries.iter().all(|entry| {
                entry
                    .options
                    .iter()
                    .all(|route| route.api_method != "remote-catalog")
            }),
            "rebuilt picker should not keep placeholder rows"
        );
        assert!(
            picker.entries.iter().any(|entry| {
                entry
                    .options
                    .iter()
                    .any(|route| route.api_method == "openai-api")
            }),
            "rebuilt picker should expose the real route"
        );
    });
}
