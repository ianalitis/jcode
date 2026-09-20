#[test]
fn startup_check_imported_transcripts_do_not_count_as_history() {
    with_temp_jcode_home(|| {
        // Imported Codex/Claude transcripts exist on genuinely fresh installs
        // that chose to import history; they must not suppress onboarding.
        let sessions_dir = crate::storage::jcode_dir()
            .expect("jcode dir")
            .join("sessions");
        std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");
        for i in 0..20 {
            std::fs::write(
                sessions_dir.join(format!("imported_codex_{i:02}.json")),
                "{}",
            )
            .expect("write imported file");
        }

        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.onboarding_startup_checked = false;

        app.maybe_begin_onboarding_flow_on_startup();

        assert!(app.onboarding_startup_checked);
        assert!(
            app.onboarding_flow.is_some(),
            "imported transcripts alone should still onboard a fresh install"
        );
    });
}

// ---------------------------------------------------------------------------
// Liveness: a first-run user can never be permanently stranded.
//
// The dangerous failure mode is a phase whose only exit depends on an external
// async event (a `LoginCompleted` bus message) that might never arrive. These
// tests prove that from every reachable phase there is *always* a forward path
// using only inputs the user is guaranteed to have: a key press, or the passage
// of time via the tick watchdog. No test here depends on an async event firing.
// ---------------------------------------------------------------------------

/// A phase is a "safe resting/exit state" if the user is no longer trapped by
/// the guided flow: onboarding finished (`None`/`Done`), they reached a ready
/// surface (`Suggestions`/`StartChoice`), or an interactive picker overlay is
/// open for them to act in.
fn onboarding_state_is_escapable(app: &App) -> bool {
    use crate::tui::app::onboarding_flow::OnboardingPhase;
    if app.inline_interactive_state.is_some() || app.session_picker_overlay.is_some() {
        return true;
    }
    match app.onboarding_phase() {
        None => true, // flow finished / inactive
        Some(OnboardingPhase::Suggestions) => true,
        Some(OnboardingPhase::StartChoice { .. }) => true,
        Some(OnboardingPhase::Done) => true,
        _ => false,
    }
}

#[test]
fn liveness_every_login_phase_has_a_single_keypress_exit() {
    use crate::tui::app::onboarding_flow::OnboardingPhase;
    with_temp_jcode_home(|| {
        // Each interactive Login-family phase must leave itself after exactly one
        // decisive key, with no dependence on an async event. We use the "skip /
        // decline" key, which is always synchronous (it never spawns an import).
        let cases: Vec<(&str, OnboardingPhase, KeyCode)> = vec![
            // OpenAI prompt: "n" declines and finishes onboarding immediately.
            (
                "LoginOpenAi",
                OnboardingPhase::LoginOpenAi {
                    yes_highlighted: true,
                },
                KeyCode::Char('n'),
            ),
            // Recovery fallback: Enter opens the provider picker overlay.
            (
                "Login{import:None}",
                OnboardingPhase::Login { import: None },
                KeyCode::Enter,
            ),
        ];
        for (label, phase, key) in cases {
            let mut app = create_test_app();
            app.onboarding_flow = None;
            app.begin_onboarding_flow_at_login();
            if let Some(flow) = app.onboarding_flow.as_mut() {
                flow.phase = phase;
            }
            assert!(
                !onboarding_state_is_escapable(&app),
                "{label}: precondition - should start trapped in the flow"
            );
            let consumed = app.handle_onboarding_continue_prompt_key(key);
            assert!(consumed, "{label}: the exit key must be consumed");
            assert!(
                onboarding_state_is_escapable(&app),
                "{label}: one key press must reach an escapable state"
            );
        }
    });
}

#[test]
fn liveness_import_review_decline_all_then_enter_escapes() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::{ImportReview, OnboardingPhase};
    with_temp_jcode_home(|| {
        // The import list is the richest interactive phase. Declining every login
        // ("n") then committing (Enter) must never spawn an async import (so it
        // can't hang) and must land on the recovery screen, from which a final
        // Enter opens the provider picker. Whole path is synchronous.
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        let mut review = ImportReview::new(vec![
            ExternalAuthReviewCandidate::fixture("OpenAI/Codex", "Codex auth.json"),
            ExternalAuthReviewCandidate::fixture("Claude", "Claude Code"),
        ])
        .unwrap();
        // Start in choose mode: this liveness path declines each login row.
        review.enter_choose_mode();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login {
                import: Some(review),
            };
        }
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Char('n')));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Down));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Char('n')));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        // No async import was spawned (declined all), so we are not stuck on the
        // progress screen; we are on the recovery screen.
        assert!(app.onboarding_import_in_progress.is_none());
        assert!(matches!(
            app.onboarding_phase(),
            Some(OnboardingPhase::Login { import: None })
        ));
        // Final Enter opens the provider picker -> escapable.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        assert!(
            onboarding_state_is_escapable(&app),
            "recovery screen + Enter must open the provider picker"
        );
    });
}

#[test]
fn liveness_stuck_import_is_recovered_by_the_tick_watchdog() {
    use crate::tui::app::onboarding_flow::OnboardingPhase;
    with_temp_jcode_home(|| {
        // Simulate the dangerous state: the import was committed (progress screen
        // showing) but its `LoginCompleted` event never arrived. The flow sits in
        // Login{import:None} with `onboarding_import_in_progress` set. Without the
        // watchdog the user is stranded forever. We backdate the start time past
        // the watchdog window and assert a single tick recovers the flow into the
        // failure-aware recovery screen (which has a guaranteed keypress exit).
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login { import: None };
        }
        // Enter the "importing" wait, backdated so the watchdog fires immediately.
        app.onboarding_import_in_progress =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(120));
        app.onboarding_import_error = None;

        // Precondition: with the import flag set and no error yet, the screen
        // shows progress and offers no keypress exit.
        assert!(app.onboarding_import_in_progress.is_some());

        let changed = app.onboarding_tick();
        assert!(changed, "watchdog tick should change state");
        // Recovered: no longer "importing", and an error is set so the recovery
        // screen explains what happened.
        assert!(
            app.onboarding_import_in_progress.is_none(),
            "watchdog must clear the stuck import progress flag"
        );
        assert!(
            app.onboarding_import_error.is_some(),
            "watchdog recovery must surface a failure reason to the user"
        );
        assert!(matches!(
            app.onboarding_phase(),
            Some(OnboardingPhase::Login { import: None })
        ));
        // And from there a single Enter still reaches the provider picker.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        assert!(onboarding_state_is_escapable(&app));
    });
}

#[test]
fn liveness_esc_always_exits_onboarding_from_every_guided_phase() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::{ImportReview, OnboardingPhase};
    with_temp_jcode_home(|| {
        // The universal escape hatch: from ANY guided pre-ready phase, a single
        // Esc must leave onboarding to the normal screen. This is the strongest
        // liveness guarantee - it doesn't matter how the flow got wedged, Esc
        // always works. We cover every interactive/transient phase, including the
        // async "importing" wait (where Esc must abandon the in-flight import).
        let make_import = || {
            ImportReview::new(vec![ExternalAuthReviewCandidate::fixture(
                "OpenAI/Codex",
                "Codex auth.json",
            )])
            .unwrap()
        };
        let phases: Vec<(&str, OnboardingPhase, bool)> = vec![
            (
                "LoginOpenAi",
                OnboardingPhase::LoginOpenAi {
                    yes_highlighted: true,
                },
                false,
            ),
            (
                "Login{import:Some}",
                OnboardingPhase::Login {
                    import: Some(make_import()),
                },
                false,
            ),
            (
                "Login{import:None} recovery",
                OnboardingPhase::Login { import: None },
                false,
            ),
            // The async "importing" wait: import committed, LoginCompleted not yet
            // arrived. Esc must still bail out cleanly.
            (
                "Login importing wait",
                OnboardingPhase::Login { import: None },
                true,
            ),
            ("ModelSelect", OnboardingPhase::ModelSelect, false),
            (
                "ContinuePrompt",
                OnboardingPhase::ContinuePrompt {
                    cli: ExternalCli::Codex,
                    yes_highlighted: true,
                    shown_at: std::time::Instant::now(),
                },
                false,
            ),
        ];
        for (label, phase, importing) in phases {
            let mut app = create_test_app();
            app.onboarding_flow = None;
            app.begin_onboarding_flow_at_login();
            if let Some(flow) = app.onboarding_flow.as_mut() {
                flow.phase = phase;
            }
            if importing {
                app.onboarding_import_in_progress = Some(std::time::Instant::now());
            }
            assert!(
                !onboarding_state_is_escapable(&app),
                "{label}: precondition - should start trapped in the flow"
            );
            let consumed = app.handle_onboarding_continue_prompt_key(KeyCode::Esc);
            assert!(consumed, "{label}: Esc must be consumed");
            assert!(
                onboarding_state_is_escapable(&app),
                "{label}: Esc must reach an escapable state"
            );
            // Esc must not leave a stale import-progress flag spinning.
            assert!(
                app.onboarding_import_in_progress.is_none(),
                "{label}: Esc must clear any in-flight import progress"
            );
        }
    });
}

#[test]
fn import_failure_reason_is_cleaned_and_capitalized() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::{ImportReview, OnboardingPhase};
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        // Must be in the Login phase for the failure handler to apply.
        let review = ImportReview::new(vec![ExternalAuthReviewCandidate::fixture(
            "Cursor", "Cursor",
        )])
        .unwrap();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login {
                import: Some(review),
            };
        }
        // A multi-line markdown failure message with marker noise and a
        // lowercase first word, mimicking the importer's render_markdown output.
        let raw =
            "**Logins imported**\n\nthe token has expired\n- \u{2715} Cursor (from Cursor): bad";
        app.onboarding_handle_login_failed(Some(raw.to_string()));
        let shown = app
            .onboarding_import_error
            .as_deref()
            .expect("failure reason should be recorded");
        // Markdown bold headers and the "Logins imported" line are stripped; the
        // first meaningful line is kept, marker trimmed, first letter uppercased.
        assert!(!shown.contains("**"), "markdown bold stripped: {shown}");
        assert!(
            !shown.contains('\u{2715}'),
            "marker glyph stripped: {shown}"
        );
        assert!(
            shown.starts_with("The token has expired"),
            "first meaningful line kept + capitalized: {shown}"
        );
    });
}

#[test]
fn import_failure_h_key_prepares_agent_repair_brief() {
    use crate::tui::app::onboarding_flow::OnboardingPhase;
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login { import: None };
        }
        // Simulate a failed import that recorded a reason.
        app.onboarding_import_error = Some("the saved credential was rejected".to_string());
        app.onboarding_import_failed_provider = Some("openai".to_string());
        let before = app.display_messages.len();

        // H on the failure screen prepares the agent repair brief.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Char('H')));

        // A brief was pushed into the transcript with the agent-runnable
        // commands and the failure reason, so it works even without a clipboard.
        assert!(app.display_messages.len() > before, "brief message pushed");
        let brief = app
            .display_messages
            .iter()
            .rev()
            .find(|m| m.content.contains("Agent repair brief"))
            .map(|m| m.content.clone())
            .expect("repair brief message");
        assert!(
            brief.contains("jcode auth-test --provider openai --json"),
            "{brief}"
        );
        assert!(brief.contains("--api-key-stdin"), "{brief}");
        assert!(
            brief.contains("the saved credential was rejected"),
            "{brief}"
        );
        // The brief was also persisted to a stable path a helper agent can read.
        let brief_path =
            crate::tui::app::onboarding_repair::repair_brief_path().expect("repair brief path");
        assert!(
            brief_path.exists(),
            "brief file should be written: {brief_path:?}"
        );
        let on_disk = std::fs::read_to_string(&brief_path).expect("read brief file");
        assert!(
            on_disk.contains("jcode auth-test --provider openai --json"),
            "{on_disk}"
        );
        assert!(
            brief.contains(&brief_path.display().to_string()),
            "brief cites its own path"
        );
        // Staying on the recovery screen, Enter still opens the provider picker.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        assert!(app.inline_interactive_state.is_some());
    });
}

#[test]
fn import_failure_h_key_is_inert_without_a_recorded_error() {
    use crate::tui::app::onboarding_flow::OnboardingPhase;
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login { import: None };
        }
        // Recovery screen reached by declining all (no error reason recorded):
        // H must NOT be intercepted, so normal input handling can use it.
        app.onboarding_import_error = None;
        assert!(!app.handle_onboarding_continue_prompt_key(KeyCode::Char('H')));
    });
}

#[test]
fn import_summary_defaults_to_continue_and_enter_imports_all() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::ImportReview;

    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        let mut review = ImportReview::new(vec![
            ExternalAuthReviewCandidate::fixture("OpenAI/Codex", "Codex auth.json"),
            ExternalAuthReviewCandidate::fixture("Claude", "Claude Code"),
        ])
        .unwrap();
        // Import tests explicitly select Continue because the product default is
        // now Jcode subscription.
        assert!(!review.choosing);
        review.focus_summary_pill(crate::tui::app::onboarding_flow::SummaryPill::Continue);
        assert!(review.continue_focused);
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login {
                import: Some(review),
            };
        }
        // Enter on the preselected Continue commits the whole import: the list
        // clears. On a live runtime the async import is marked in-flight; the
        // test harness has no tokio runtime, so the graceful fallback lands on
        // the recovery screen instead (never a panic, never a stuck screen).
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        assert!(matches!(
            app.onboarding_phase(),
            Some(OnboardingPhase::Login { import: None })
        ));
        assert!(
            app.onboarding_import_in_progress.is_some() || app.onboarding_import_error.is_some(),
            "Continue must either start the import or fail it gracefully"
        );
    });
}

#[test]
fn import_continue_reaches_ready_quality_first_openai_model() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::ImportReview;

    with_temp_jcode_home(|| {
        let legacy_auth = crate::auth::codex::legacy_auth_file_path().expect("legacy auth path");
        std::fs::create_dir_all(legacy_auth.parent().expect("legacy auth parent"))
            .expect("create legacy auth dir");
        std::fs::write(legacy_auth, r#"{"OPENAI_API_KEY":"sk-onboarding-test"}"#)
            .expect("seed importable Codex key");
        crate::auth::AuthStatus::invalidate_cache();

        // App construction performs synchronous runtime-backed setup, so build
        // it before entering the async test runtime to avoid nested `block_on`.
        let mut app = quality_first_openai_test_app();
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        runtime.block_on(async {
            app.onboarding_flow = None;
            app.begin_onboarding_flow_at_login();
            let mut review = ImportReview::new(vec![ExternalAuthReviewCandidate::fixture(
                "OpenAI/Codex",
                "Codex auth.json",
            )])
            .unwrap();
            review.focus_summary_pill(crate::tui::app::onboarding_flow::SummaryPill::Continue);
            if let Some(flow) = app.onboarding_flow.as_mut() {
                flow.phase = OnboardingPhase::Login {
                    import: Some(review),
                };
            }

            let mut bus_rx = crate::bus::Bus::global().subscribe();
            assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));

            let login = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                loop {
                    if let Ok(crate::bus::BusEvent::LoginCompleted(login)) = bus_rx.recv().await {
                        break login;
                    }
                }
            })
            .await
            .expect("import completion event");
            assert!(
                login.success,
                "Continue should complete the approved import: {}",
                login.message
            );
            assert_eq!(
                login.provider, "openai-api",
                "import completion must preserve the concrete provider route"
            );
            assert!(
                app.onboarding_should_prefer_strongest_model(),
                "first-run import without explicit defaults should use global ranking"
            );

            app.handle_login_completed(login);
            assert!(matches!(
                app.onboarding_phase(),
                Some(OnboardingPhase::StartChoice { .. })
            ));
            let (model, provider_key) =
                tokio::time::timeout(std::time::Duration::from_secs(4), async {
                    loop {
                        match bus_rx.recv().await {
                            Ok(crate::bus::BusEvent::ProviderModelActivated {
                                model,
                                provider_key,
                                ..
                            }) => break (model, provider_key),
                            Ok(crate::bus::BusEvent::AuthCatalogRefreshReady) => {
                                app.finish_auth_catalog_refresh();
                            }
                            _ => {}
                        }
                    }
                })
                .await
                .expect("strongest model activation event");

            assert_eq!(model, jcode_provider_core::DEFAULT_OPENAI_MODEL);
            assert_eq!(provider_key.as_deref(), Some("openai-api"));
        });
    });
}

#[test]
fn import_summary_choose_pill_opens_checkbox_list() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::ImportReview;

    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        let review = ImportReview::new(vec![
            ExternalAuthReviewCandidate::fixture("OpenAI/Codex", "Codex auth.json"),
            ExternalAuthReviewCandidate::fixture("Claude", "Claude Code"),
        ])
        .unwrap();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login {
                import: Some(review),
            };
        }
        // Arrow from the default subscription option to the "Import less" pill,
        // then commit it.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Right));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        // Now in choose mode: the checkbox list with the cursor on row 1 and
        // nothing imported yet.
        match app.onboarding_phase() {
            Some(OnboardingPhase::Login {
                import: Some(review),
            }) => {
                assert!(review.choosing);
                assert!(!review.continue_focused);
                assert_eq!(review.cursor, 0);
                assert_eq!(review.checked_count(), 2);
            }
            other => panic!("expected choose-mode import review, got {other:?}"),
        }
        assert!(app.onboarding_import_in_progress.is_none());

        // The welcome snapshot reports choose mode so the renderer switches.
        match app.onboarding_welcome_kind() {
            crate::tui::OnboardingWelcomeKind::Login {
                import: Some(prompt),
                ..
            } => assert!(prompt.choosing),
            other => panic!("expected Login welcome with import prompt, got {other:?}"),
        }
    });
}

#[test]
fn recent_project_review_prompt_is_bounded_read_only_and_requires_approval() {
    let repository = std::path::Path::new("/home/example/projects/demo");
    let prompt = App::onboarding_recent_project_review_prompt(repository);

    assert_eq!(
        prompt,
        "Find the most critical architecture problems in the repository at \"/home/example/projects/demo\". Do not fix them yet, and ask me whether I want them fixed once you find them."
    );
}

#[test]
fn preparing_recent_project_review_finishes_onboarding_and_seeds_the_first_turn() {
    let mut app = onboarding_test_app();
    let repository = app
        .onboarding_recent_project_path()
        .expect("test session should start in a Git repository");
    let expected = App::onboarding_recent_project_review_prompt(&repository);

    assert!(app.onboarding_prepare_recent_project_review());

    assert!(!app.onboarding_flow_active());
    assert_eq!(app.input, expected);
    assert_eq!(app.cursor_pos, app.input.len());
}

#[test]
fn starting_recent_project_review_runs_as_a_visible_local_turn() {
    let mut app = onboarding_test_app();
    let repository = app
        .onboarding_recent_project_path()
        .expect("test session should start in a Git repository");
    let expected = App::onboarding_recent_project_review_prompt(&repository);

    app.onboarding_start_recent_project_review();

    assert!(!app.onboarding_flow_active());
    assert!(app.pending_turn, "local review should start a local turn");
    assert!(app.is_processing, "local review should enter Sending");
    assert!(app.queued_messages.is_empty());
    assert_eq!(
        app.session.messages.last().and_then(|message| {
            message.content.iter().find_map(|block| match block {
                crate::message::ContentBlock::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
        }),
        Some(expected.as_str())
    );
}

#[test]
fn starting_recent_project_review_queues_remote_turn_without_stuck_sending() {
    let mut app = onboarding_test_app();
    app.is_remote = true;
    let repository = app
        .onboarding_recent_project_path()
        .expect("remote session should provide its working directory");
    let expected = App::onboarding_recent_project_review_prompt(&repository);

    app.onboarding_start_recent_project_review();

    assert!(!app.onboarding_flow_active());
    assert!(
        !app.pending_turn,
        "remote review must not set the local pending-turn flag"
    );
    assert!(
        !app.is_processing,
        "remote review must stay idle until the remote queue dispatches"
    );
    assert!(app.input.is_empty());
    assert_eq!(app.queued_messages, vec![expected]);
}

#[test]
fn recent_project_review_falls_back_cleanly_when_no_repo_is_known() {
    let mut app = onboarding_test_app();
    app.is_remote = true;
    app.session.working_dir = dirs::home_dir().map(|path| path.to_string_lossy().into_owned());

    app.onboarding_start_recent_project_review();

    assert!(!app.pending_turn);
    assert!(app.queued_messages.is_empty());
    assert!(matches!(
        app.onboarding_phase(),
        Some(OnboardingPhase::Suggestions)
    ));
    assert!(
        app.status_notice
            .as_ref()
            .is_some_and(|(notice, _)| { notice.contains("No active Git repository found") })
    );
}

#[test]
fn telemetry_and_feedback_commands_never_claim_to_enable_or_send() {
    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        for command in [
            "/telemetry",
            "/telemetry everything",
            "/telemetry usage",
            "/telemetry off",
        ] {
            assert!(crate::tui::app::commands::handle_telemetry_command(
                &mut app, command
            ));
            assert!(
                app.display_messages
                    .last()
                    .unwrap()
                    .content
                    .contains("cannot be enabled")
            );
            assert!(!crate::telemetry::is_enabled());
        }
        assert!(crate::tui::app::commands::handle_feedback_command(
            &mut app,
            "/feedback private payload"
        ));
        let message = &app.display_messages.last().unwrap().content;
        assert!(message.contains("Nothing was sent"));
        assert!(!message.contains("private payload"));
        assert!(!crate::tui::app::commands::handle_feedback_command(
            &mut app,
            "/feedback-other"
        ));
    });
}

#[test]
fn telemetry_pill_opens_settings_page_and_commits_choice() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::{ImportReview, TelemetryLevel};

    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        let review = ImportReview::new(vec![ExternalAuthReviewCandidate::fixture(
            "OpenAI/Codex",
            "Codex auth.json",
        )])
        .unwrap();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login {
                import: Some(review),
            };
        }

        // Right twice: Subscription -> Import less -> Telemetry settings.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Right));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Right));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));

        // The page reports the fork's permanent no-collection policy.
        match app.onboarding_phase() {
            Some(OnboardingPhase::Login {
                import: Some(review),
            }) => assert_eq!(review.telemetry, Some(TelemetryLevel::Nothing)),
            other => panic!("expected telemetry page open, got {other:?}"),
        }
        // The import countdown is paused while the page is open, so the screen
        // cannot commit the import out from under the user.
        assert!(!app.onboarding_flow.as_ref().unwrap().decision_timed_out());

        // Enter returns to onboarding without enabling collection.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        assert!(!crate::telemetry::is_enabled());
        assert!(!crate::telemetry::content_sharing_enabled());
        // We are back on the summary screen with the import still pending.
        match app.onboarding_phase() {
            Some(OnboardingPhase::Login {
                import: Some(review),
            }) => {
                assert!(review.telemetry.is_none());
                assert!(!review.choosing);
            }
            other => panic!("expected import summary, got {other:?}"),
        }
        assert!(app.onboarding_import_in_progress.is_none());
    });
}

#[test]
fn telemetry_page_send_nothing_disables_telemetry_and_esc_goes_back() {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::ImportReview;

    with_temp_jcode_home(|| {
        let mut app = create_test_app();
        app.onboarding_flow = None;
        app.begin_onboarding_flow_at_login();
        let review = ImportReview::new(vec![ExternalAuthReviewCandidate::fixture(
            "OpenAI/Codex",
            "Codex auth.json",
        )])
        .unwrap();
        if let Some(flow) = app.onboarding_flow.as_mut() {
            flow.phase = OnboardingPhase::Login {
                import: Some(review),
            };
        }

        // t is the direct shortcut onto the telemetry page; Esc returns without
        // changing anything and keeps onboarding active.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Char('t')));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Esc));
        assert!(matches!(
            app.onboarding_phase(),
            Some(OnboardingPhase::Login { import: Some(_) })
        ));
        assert!(!crate::telemetry::is_enabled());

        // Arrow keys cannot select an opt-in. Returning keeps collection off.
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Char('t')));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Down));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Down));
        assert!(app.handle_onboarding_continue_prompt_key(KeyCode::Enter));
        assert!(!crate::telemetry::is_enabled());
        assert!(!crate::telemetry::content_sharing_enabled());
    });
}

#[test]
fn start_choice_prefetches_recent_project_so_enter_does_not_block() {
    let mut app = onboarding_test_app();
    assert!(
        app.onboarding_recent_project_prefetch.is_none(),
        "no prefetch before the start choice is shown"
    );

    app.onboarding_open_start_choice();

    let slot = app
        .onboarding_recent_project_prefetch
        .clone()
        .expect("opening the start choice should warm the recent-project lookup");

    // Wait briefly for the background scan; the action must not depend on it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if slot.lock().expect("prefetch slot").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        slot.lock().expect("prefetch slot").is_some(),
        "prefetch should resolve in the background"
    );

    // Opening the choice twice must not spawn a second scan.
    app.onboarding_open_start_choice();
    assert!(
        std::sync::Arc::ptr_eq(
            &slot,
            app.onboarding_recent_project_prefetch
                .as_ref()
                .expect("prefetch retained")
        ),
        "the warm prefetch should be reused"
    );

    // The resolved path is still the repository the session runs in.
    assert_eq!(
        app.onboarding_recent_project_path(),
        crate::import::repo_ranking::resolve_git_root(std::path::Path::new(
            app.session
                .working_dir
                .as_deref()
                .expect("test session working dir")
        ))
    );
}
