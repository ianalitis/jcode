use super::*;

fn extract_line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn leading_spaces(text: &str) -> usize {
    text.chars().take_while(|c| *c == ' ').count()
}

fn system_glyph_env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn render_system_message_forces_system_color_on_all_spans() {
    let msg = DisplayMessage::system("**Reload complete** - continuing.");

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);

    assert!(!lines.is_empty(), "expected rendered system message lines");
    for line in lines {
        for span in line.spans {
            assert_eq!(span.style.fg, Some(system_message_color()));
        }
    }
}

#[test]
fn render_cold_cache_warning_is_always_one_width_bounded_line() {
    let saved = crate::tui::markdown::center_code_blocks();
    let msg = DisplayMessage::system(
        "🧊 Prompt cache went cold · next turn may resend ~96K tok · /cache extends",
    );

    for centered in [false, true] {
        crate::tui::markdown::set_center_code_blocks(centered);
        for width in [80_u16, 50, 30] {
            let lines = render_system_message(&msg, width, crate::config::DiffDisplayMode::Off);
            assert_eq!(
                lines.len(),
                1,
                "cold-cache notice wrapped at width {width} (centered={centered}): {lines:?}"
            );
            let text = extract_line_text(&lines[0]);
            assert!(
                !text.contains('\n'),
                "cold-cache notice contains a newline: {text:?}"
            );
            assert!(
                lines[0].width() <= width as usize,
                "cold-cache notice width {} exceeds {width}: {text:?}",
                lines[0].width()
            );
            assert!(
                text.contains("Prompt cache went cold"),
                "cold-cache identity was truncated away: {text:?}"
            );
            if width < 80 {
                assert!(
                    text.ends_with('…'),
                    "narrow cold-cache notice should end in an ellipsis: {text:?}"
                );
            }
            for span in &lines[0].spans {
                assert_eq!(span.style.fg, Some(system_message_color()));
            }
        }
    }

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_launch_hotkeys_keeps_both_shortcuts_visible() {
    let saved = crate::tui::markdown::center_code_blocks();
    let content = "Hotkeys: Super+; → jcode · Super+' → home";
    let msg = DisplayMessage::system(content).with_title("Launch hotkeys");

    for centered in [false, true] {
        crate::tui::markdown::set_center_code_blocks(centered);
        for width in [80_u16, 50] {
            let lines = render_system_message(&msg, width, crate::config::DiffDisplayMode::Off);
            assert_eq!(lines.len(), 1);
            assert_eq!(extract_line_text(&lines[0]).trim(), content);
            assert!(lines[0].width() <= width as usize);
        }
    }

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_compact_launch_and_divergence_notices_as_one_line() {
    let saved = crate::tui::markdown::center_code_blocks();
    let notices = [
        DisplayMessage::system(
            "Configured Jcode launch hotkeys (niri):\nSuper+; → jcode (/home/user/project)\n\nBound system-wide.",
        )
        .with_title("Launch hotkeys"),
        DisplayMessage::system(
            "Update diverged. Press Ctrl+Y to let a jcode agent merge local and upstream (or run `git pull` / `git rebase` yourself).",
        )
        .with_title("Update"),
    ];

    for centered in [false, true] {
        crate::tui::markdown::set_center_code_blocks(centered);
        for msg in &notices {
            for width in [80_u16, 50, 30] {
                let lines = render_system_message(msg, width, crate::config::DiffDisplayMode::Off);
                assert_eq!(
                    lines.len(),
                    1,
                    "compact notice wrapped at width {width} (centered={centered}): {lines:?}"
                );
                let text = extract_line_text(&lines[0]);
                assert!(!text.contains('\n'), "notice contains a newline: {text:?}");
                assert!(
                    lines[0].width() <= width as usize,
                    "notice width {} exceeds {width}: {text:?}",
                    lines[0].width()
                );
                if width < 80 {
                    assert!(
                        text.ends_with('…'),
                        "narrow compact notice should end in an ellipsis: {text:?}"
                    );
                }
            }
        }
    }

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_system_message_renders_markdown_formatting() {
    let msg = DisplayMessage::system(
        "**bold** and `code` and # heading\n- bullet item\n[link](http://example.com)",
    );

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    // System messages now render markdown: the inline markers are consumed and
    // the underlying text survives. Bold/code markers should no longer appear
    // literally, while the text content and a bullet glyph remain.
    assert!(plain.contains("bold"), "keeps bold text: {plain:?}");
    assert!(
        !plain.contains("**bold**"),
        "strips bold markers: {plain:?}"
    );
    assert!(plain.contains("code"), "keeps code text: {plain:?}");
    assert!(plain.contains("heading"), "keeps heading text: {plain:?}");
    assert!(
        plain.contains("bullet item"),
        "keeps bullet text: {plain:?}"
    );
    // The link text renders without the raw markdown link syntax.
    assert!(plain.contains("link"), "keeps link text: {plain:?}");
    assert!(
        !plain.contains("[link](http://example.com)"),
        "strips raw link syntax: {plain:?}"
    );

    // Color is still forced to the system color over every span.
    for line in &lines {
        for span in &line.spans {
            assert_eq!(span.style.fg, Some(system_message_color()));
        }
    }
}

#[test]
fn render_system_message_preserves_indentation_and_newlines() {
    let msg = DisplayMessage::system("Header line\n  indented detail\n\nNext block");

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let rendered = lines.iter().map(extract_line_text).collect::<Vec<_>>();

    // Centered mode may add uniform left padding; compare relative structure.
    assert_eq!(rendered.len(), 4, "got: {rendered:?}");
    assert!(
        rendered[0].trim_end().ends_with("Header line"),
        "got: {rendered:?}"
    );
    assert!(
        rendered[1].trim_end().ends_with("indented detail"),
        "got: {rendered:?}"
    );
    assert!(
        rendered[2].trim().is_empty(),
        "blank line preserved, got: {rendered:?}"
    );
    assert!(
        rendered[3].trim_end().ends_with("Next block"),
        "got: {rendered:?}"
    );

    // The detail line keeps exactly two more leading spaces than the header.
    assert_eq!(
        leading_spaces(&rendered[1]),
        leading_spaces(&rendered[0]) + 2,
        "indentation should be preserved, got: {rendered:?}"
    );
}

#[test]
fn render_plaintext_lines_hang_indents_wrapped_continuations() {
    // An indented line longer than the wrap width keeps its indent on the wrap.
    let lines = render_plaintext_lines("  alpha beta gamma delta", 12);
    let rendered = lines.iter().map(extract_line_text).collect::<Vec<_>>();

    assert!(rendered.len() >= 2, "expected wrapping, got: {rendered:?}");
    for line in &rendered {
        assert!(
            line.is_empty() || line.starts_with("  "),
            "continuation lines should keep indent, got: {rendered:?}"
        );
        assert!(line.width() <= 12, "line too wide: {line:?}");
    }
}

#[test]
fn render_system_message_centered_mode_left_aligns_with_padding() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage::system("Reload complete - continuing.");

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);

    assert!(!lines.is_empty(), "expected rendered system message lines");
    for line in &lines {
        assert_eq!(
            line.alignment,
            Some(ratatui::layout::Alignment::Left),
            "centered system lines should be left-aligned with padding"
        );
        assert!(
            line.spans
                .first()
                .is_some_and(|span| span.content.starts_with(' ')),
            "centered system lines should start with padding"
        );
    }
    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_system_message_uses_width_stable_titles_on_kitty() {
    let _guard = system_glyph_env_lock();
    let prev_term_program = std::env::var("TERM_PROGRAM").ok();
    let prev_term = std::env::var("TERM").ok();
    crate::env::set_var("TERM_PROGRAM", "kitty");
    crate::env::set_var("TERM", "xterm-kitty");

    let msg = DisplayMessage::system(
        "⚡ Connection lost - retrying (attempt 2, 7s) - connection reset by server",
    )
    .with_title("Connection");

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("reconnecting"));
    assert!(!plain.contains("⚡ reconnecting"));

    match prev_term_program {
        Some(value) => crate::env::set_var("TERM_PROGRAM", value),
        None => crate::env::remove_var("TERM_PROGRAM"),
    }
    match prev_term {
        Some(value) => crate::env::set_var("TERM", value),
        None => crate::env::remove_var("TERM"),
    }
}

#[test]
fn render_background_task_message_uses_box_and_truncates_preview_lines() {
    let msg = DisplayMessage::background_task(
        "**Background task** `bg123` · `bash` · ✓ completed · 7.1s · exit 0\n\n```text\nline 1\nline 2\nline 3\nline 4\nline 5\n```\n\n_Full output:_ `bg action=\"output\" task_id=\"bg123\"`",
    );

    let lines = render_background_task_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("✓ bg bash completed · bg123"));
    assert!(plain.contains("exit 0 · 7.1s"));
    assert!(plain.contains("line 1"));
    assert!(plain.contains("… +1 more line"));
    assert!(!plain.contains("task bg123 · bash"));
    assert!(!plain.contains("Preview"));
    assert!(!plain.contains("Full output"));
    assert!(!plain.contains("bg action=\"output\" task_id=\"bg123\""));
}

#[test]
fn render_background_task_message_strips_ansi_from_existing_preview() {
    let msg = DisplayMessage::background_task(
        "**Background task** `bg123` · `bash` · ✓ completed · 0.1s · exit 0\n\n```text\n\u{1b}[32m✓\u{1b}[39m passes \u{1b}[2m12ms\u{1b}[22m\n```\n\n_Full output:_ `bg action=\"output\" task_id=\"bg123\"`",
    );

    let plain = render_background_task_message(&msg, 80, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.contains("✓ passes 12ms"),
        "rendered preview:\n{plain}"
    );
    assert!(!plain.contains('\u{1b}'));
    assert!(!plain.contains("[32m"));
    assert!(!plain.contains("[2m"));
}

#[test]
fn render_system_message_strips_ansi_from_existing_inline_command_preview() {
    let msg = DisplayMessage::system(
        "Shell command · ✓ exit 0 · 12ms\n\n  cargo test\n\n  \u{1b}[32m✓\u{1b}[39m passes \u{1b}[2m12ms\u{1b}[22m",
    );

    let plain = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.contains("✓ passes 12ms"),
        "rendered preview:\n{plain}"
    );
    assert!(!plain.contains('\u{1b}'));
    assert!(!plain.contains("[32m"));
    assert!(!plain.contains("[2m"));
}

#[test]
fn render_background_task_message_uses_swarm_flavor_for_swarm_tool() {
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage::background_task(
        "**Background task** `bg777` · `run_plan (6 nodes, deep mode)` (`swarm`) · ✓ completed · 92.4s · exit 0\n\n```text\nSwarm plan reached terminal/blocked state after 9 loop(s). completed=6 blocked=0 cycles=0 active=0 assignments=8\n```\n\n_Full output:_ `bg action=\"output\" task_id=\"bg777\"`",
    );

    let lines = render_background_task_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(plain, "🐝 ✓ run plan · 92.4s");
    assert!(!plain.contains("bg777"));
    assert!(!plain.contains("Swarm plan reached terminal/blocked state"));
}

#[test]
fn render_background_task_progress_message_uses_swarm_flavor_for_swarm_tool() {
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage::background_task(
        "**Background task progress** `bg777` · `run_plan (6 nodes, deep mode)` (`swarm`)\n\n[####--------] 33% · 2/6 nodes · completed 2 · blocked 0 · active 3 · assignments 5 (reported)",
    );

    let lines = render_background_task_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(plain, "🐝 ● run plan · 2/6");
    assert!(!plain.contains("bg777"));
}

#[test]
fn render_background_task_progress_message_uses_box_with_progress_bar() {
    let msg = DisplayMessage::background_task(
        "**Background task progress** `bg123` · `bash`\n\n[#####-------] 42% · Running tests (reported)",
    );

    let lines = render_background_task_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("◌ bg bash · bg123"));
    assert!(plain.contains("█"));
    assert!(plain.contains("░"));
    assert!(plain.contains("42%"));
    assert!(plain.contains("Running tests"));
    assert!(plain.contains("Latest status: bg action=\"status\" task_id=\"bg123\""));
    assert_eq!(
        plain.matches('│').count(),
        4,
        "expected compact progress row plus status hint:\n{plain}"
    );
    assert!(!plain.contains("Latest update"));
    assert!(!plain.contains("Source: reported"));
    assert!(!plain.contains("**Background task progress**"));
}

#[test]
fn render_overnight_message_uses_rounded_progress_card() {
    let card = crate::overnight::OvernightProgressCard {
        run_id: "overnight_1234567890abcdef".to_string(),
        status: "running".to_string(),
        phase: "running".to_string(),
        coordinator_session_id: "session_coord".to_string(),
        coordinator_session_name: "Overnight coordinator".to_string(),
        elapsed_label: "2h 15m".to_string(),
        target_duration_label: "7h".to_string(),
        progress_percent: 32.0,
        target_wake_at: "2026-05-01T15:00:00Z".to_string(),
        time_relation: "target in 4h 45m".to_string(),
        last_activity_label: "4m ago".to_string(),
        next_prompt_label: "handoff mode in 4h 15m or after current turn".to_string(),
        usage_risk: "medium".to_string(),
        usage_confidence: "low".to_string(),
        usage_projection: "projected 48% to 76%".to_string(),
        resources_summary: "RAM 62%, load 2.4/8, battery 80% discharging, disk 52.0 GB free"
            .to_string(),
        latest_event_kind: Some("coordinator_turn_completed".to_string()),
        latest_event_summary: Some("Coordinator turn completed".to_string()),
        task_summary: crate::overnight::OvernightTaskCardSummary {
            total: 4,
            counts: crate::overnight::OvernightTaskStatusCounts {
                completed: 2,
                active: 1,
                blocked: 0,
                deferred: 1,
                failed: 0,
                skipped: 0,
                unknown: 0,
            },
            validated: 2,
            high_risk: 0,
            latest_title: Some("Verify provider reload".to_string()),
            latest_status: Some("active".to_string()),
        },
        active_task_title: Some("Verify provider reload".to_string()),
        review_path: "/tmp/overnight/review.html".to_string(),
        log_path: "/tmp/overnight/run.log".to_string(),
        run_dir: "/tmp/overnight".to_string(),
        completed_at: None,
    };
    let msg = DisplayMessage::overnight(serde_json::to_string(&card).unwrap());

    let lines = render_overnight_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("overnight · running"));
    assert!(plain.contains("█"));
    assert!(plain.contains("░"));
    assert!(plain.contains("32%"));
    assert!(plain.contains("2 complete, 1 active, 0 blocked, 1 deferred"));
    assert!(plain.contains("Verify provider reload"));
    assert!(plain.contains("medium risk"));
    assert!(plain.contains("review.html"));
}

#[test]
fn render_todos_message_shows_grouped_card_with_status_glyphs() {
    fn todo(id: &str, content: &str, status: &str, group: Option<&str>) -> crate::todo::TodoItem {
        crate::todo::TodoItem {
            id: id.to_string(),
            content: content.to_string(),
            status: status.to_string(),
            priority: "high".to_string(),
            group: group.map(str::to_string),
            confidence: Some(crate::todo::ConfidenceState::from_legacy_score(80)),
            completion_confidence: (status == "completed")
                .then_some(crate::todo::ConfidenceState::from_legacy_score(95)),
            confidence_history: Vec::new(),
            blocked_by: Vec::new(),
            assigned_to: None,
        }
    }

    let todos = vec![
        todo("1", "Wire the hotkey", "completed", Some("todo card")),
        todo("2", "Render the card", "in_progress", Some("todo card")),
        todo("3", "Unrelated cleanup", "pending", None),
    ];
    let msg = DisplayMessage::todos(serde_json::to_string(&todos).unwrap());

    let lines = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!plain.contains("Todos"), "{plain}");
    assert!(plain.contains("todo card"), "{plain}");
    assert!(plain.contains("other"), "{plain}");
    let todo_card_header = lines
        .iter()
        .map(extract_line_text)
        .find(|line| line.contains("todo card"))
        .unwrap();
    assert_eq!(todo_card_header.matches('●').count(), 2, "{plain}");
    assert_eq!(todo_card_header.matches('○').count(), 0, "{plain}");
    let other_header = lines
        .iter()
        .map(extract_line_text)
        .find(|line| line.contains("other"))
        .unwrap();
    assert_eq!(other_header.matches('○').count(), 1, "{plain}");
    assert!(plain.contains("✓ Wire the hotkey"), "{plain}");
    assert!(plain.contains("● Render the card"), "{plain}");
    assert!(plain.contains("○ Unrelated cleanup"), "{plain}");
    // Completed items show completion confidence; open ones planning confidence.
    assert!(plain.contains("plausible"), "{plain}");
    assert!(plain.contains("plausible"), "{plain}");
    // Priority remains metadata and is not repeated in the visible item label.
    assert!(!plain.contains("(high)"), "{plain}");
    assert!(
        !plain.contains('╭'),
        "todo card should be borderless:\n{plain}"
    );
    assert!(
        !plain.contains('╰'),
        "todo card should be borderless:\n{plain}"
    );
}

#[test]
fn render_todos_message_shows_goal_scores_without_verbose_feedback() {
    let todos = vec![crate::todo::TodoItem {
        id: "1".to_string(),
        content: "Render the card".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("todo rendering".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(85)),
        completion_confidence: None,
        confidence_history: vec![
            crate::todo::ConfidenceState::from_legacy_score(80),
            crate::todo::ConfidenceState::from_legacy_score(85),
        ],
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("todo rendering".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(95)),
        feedback_loop: Some("Inspect a debug frame".to_string()),
        feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::Representative),
        feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::MainPaths),
        delivery_state: Some(crate::todo::DeliveryState::from_legacy_score(90)),
        ..Default::default()
    }];
    let plan = crate::todo::TodoPlan {
        user_intention: Some("Keep the agent aligned with the user's request".to_string()),
        understands_user_intent: Some(crate::todo::IntentUnderstanding::from_legacy_score(98)),
        ..Default::default()
    };
    let msg = DisplayMessage::todos(
        serde_json::json!({ "todos": todos, "plan": plan, "goals": goals }).to_string(),
    );

    let plain = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    for assessment in ["Closed feedback loop strong", "Delivery workflow_validated"] {
        assert!(plain.contains(assessment), "{plain}");
    }
    assert!(!plain.contains("Relevance representative"), "{plain}");
    assert!(!plain.contains("Coverage main_paths"), "{plain}");
    // Only the plan-level assessment renders above the groups.
    assert!(
        plain.contains("Intent clear: Keep the agent aligned"),
        "{plain}"
    );
    assert!(!plain.contains("Feedback ·"), "{plain}");
    assert!(!plain.contains("Inspect a debug frame"), "{plain}");
    assert!(plain.contains("● Render the card · plausible"), "{plain}");
    assert!(!plain.contains("(high)"), "{plain}");
}

#[test]
fn render_todos_message_shows_user_intention_when_understanding_is_unclear() {
    let long_text = "This deliberately long assessment detail should not consume several rows in a narrow terminal window";
    let todos = vec![crate::todo::TodoItem {
        id: "1".to_string(),
        content: "Keep the task visible".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("responsive card".to_string()),
        confidence: None,
        completion_confidence: None,
        confidence_history: Vec::new(),
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let plan = crate::todo::TodoPlan {
        user_intention: Some(long_text.to_string()),
        understands_user_intent: Some(crate::todo::IntentUnderstanding::Partial),
        ..Default::default()
    };
    let goals = vec![crate::todo::TodoGoal {
        group: Some("responsive card".to_string()),
        feedback_loop: Some(long_text.to_string()),
        ..Default::default()
    }];
    let msg = DisplayMessage::todos(
        serde_json::json!({ "todos": todos, "plan": plan, "goals": goals }).to_string(),
    );

    let narrow = render_todos_message(&msg, 60, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>();
    assert_eq!(
        narrow
            .iter()
            .filter(|line| line.contains("Intent partial:"))
            .count(),
        1
    );
    assert_eq!(
        narrow
            .iter()
            .filter(|line| line.contains("Feedback"))
            .count(),
        0,
        "verbose goal feedback should stay out of inline cards: {}",
        narrow.join("\n")
    );
    assert!(
        narrow
            .iter()
            .any(|line| line.contains("Keep the task visible")),
        "{}",
        narrow.join("\n")
    );

    let wide = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>();
    assert!(
        wide.iter()
            .any(|line| line.contains("Intent partial: This deliberately long assessment detail")),
        "wide={wide:?}"
    );
    assert!(
        wide.iter().any(|line| line.contains('…')),
        "wide intent should remain on one ellipsized line: {wide:?}"
    );
    assert!(
        !narrow
            .iter()
            .any(|line| line.contains("narrow terminal window")),
        "narrow={narrow:?}"
    );
    assert!(
        narrow.iter().any(|line| line.contains('…')),
        "narrow intent should be ellipsized: {narrow:?}"
    );
}

#[test]
fn render_todos_message_uses_readable_semantic_colors() {
    let todos = vec![crate::todo::TodoItem {
        id: "1".to_string(),
        content: "Tune the palette".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("todo rendering".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(85)),
        completion_confidence: None,
        confidence_history: Vec::new(),
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("todo rendering".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(95)),
        feedback_loop: None,
        ..Default::default()
    }];
    let plan = crate::todo::TodoPlan {
        user_intention: Some("Readable metadata".to_string()),
        understands_user_intent: Some(crate::todo::IntentUnderstanding::from_legacy_score(98)),
        ..Default::default()
    };
    let msg = DisplayMessage::todos(
        serde_json::json!({ "todos": todos, "plan": plan, "goals": goals }).to_string(),
    );
    let lines = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let color_for = |text: &str| {
        lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref() == text)
            .and_then(|span| span.style.fg)
    };

    assert_eq!(color_for("todo rendering"), Some(todo_group_color()));
    assert_eq!(color_for("clear"), Some(todo_score_color()));
    assert_eq!(color_for("Readable metadata"), Some(todo_meta_color()));
    assert_eq!(color_for("● "), Some(asap_color()));
    assert_eq!(color_for(" (high)"), None);
    assert_eq!(color_for(" · plausible"), Some(todo_confidence_color()));
    assert_eq!(color_for("strong"), Some(todo_warning_color()));
    assert_eq!(color_for("missing"), Some(todo_failure_color()));
    assert_ne!(todo_meta_color(), dim_color());
}

#[test]
fn render_todos_message_color_codes_every_intent_state() {
    let cases = [
        (
            crate::todo::IntentUnderstanding::Uncertain,
            todo_failure_color(),
        ),
        (
            crate::todo::IntentUnderstanding::Partial,
            todo_warning_color(),
        ),
        (crate::todo::IntentUnderstanding::Clear, todo_score_color()),
        (
            crate::todo::IntentUnderstanding::Complete,
            todo_score_color(),
        ),
    ];

    for (state, expected_color) in cases {
        let state_text = state.as_str().to_string();
        let msg = DisplayMessage::todos(
            serde_json::json!({
                "todos": [],
                "plan": {
                    "user_intention": "Keep intent visible",
                    "understands_user_intent": state,
                },
                "goals": [],
            })
            .to_string(),
        );
        let lines = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off);
        let rendered_color = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref() == state_text)
            .and_then(|span| span.style.fg);

        assert_eq!(
            rendered_color,
            Some(expected_color),
            "intent state {state_text} should keep its semantic color in the todo renderer"
        );
    }
}

#[test]
fn render_todos_message_collapses_passing_quality_gates() {
    let todos = vec![crate::todo::TodoItem {
        id: "1".to_string(),
        content: "Verify the result".to_string(),
        status: "completed".to_string(),
        priority: "high".to_string(),
        group: Some("quality".to_string()),
        confidence: None,
        completion_confidence: Some(crate::todo::ConfidenceState::Validated),
        confidence_history: Vec::new(),
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("quality".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::Closed),
        feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::AcceptanceAligned),
        feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::EdgeAndIntegrationPaths),
        feedback_loop_traceability: Some(crate::todo::FeedbackLoopTraceability::Complete),
        delivery_state: Some(crate::todo::DeliveryState::OutcomeDelivered),
        ..Default::default()
    }];
    let msg =
        DisplayMessage::todos(serde_json::json!({ "todos": todos, "goals": goals }).to_string());
    let lines = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("✓ All quality gates passing"), "{plain}");
    assert!(plain.contains("Delivery outcome_delivered"), "{plain}");
    assert!(!plain.contains("Closed feedback loop closed"), "{plain}");
    assert!(!plain.contains("Relevance acceptance_aligned"), "{plain}");
    assert!(
        !plain.contains("Coverage edge_and_integration_paths"),
        "{plain}"
    );
    let passing = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .find(|span| span.content.as_ref() == "✓ All quality gates passing")
        .and_then(|span| span.style.fg);
    assert_eq!(passing, Some(todo_score_color()));
}

#[test]
fn render_todos_message_wraps_goal_scores_at_narrow_widths() {
    let todos = vec![crate::todo::TodoItem {
        id: "1".to_string(),
        content: "Render the card".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("todo rendering".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(85)),
        completion_confidence: None,
        confidence_history: Vec::new(),
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("todo rendering".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(95)),
        feedback_loop: None,
        delivery_state: Some(crate::todo::DeliveryState::from_legacy_score(90)),
        ..Default::default()
    }];
    let msg =
        DisplayMessage::todos(serde_json::json!({ "todos": todos, "goals": goals }).to_string());

    let lines = render_todos_message(&msg, 40, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("Closed feedback loop strong"), "{plain}");
    assert!(plain.contains("Delivery workflow_validated"), "{plain}");
    assert!(
        lines.iter().all(|line| line.width() <= 38),
        "card exceeded its 38-column content budget: {plain}"
    );
}

#[test]
fn render_todos_message_empty_list_shows_placeholder() {
    let msg = DisplayMessage::todos("[]");
    let plain = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!plain.contains("Todos"), "{plain}");
    assert!(plain.contains("No tasks yet"), "{plain}");
}

#[test]
fn render_todos_message_bad_payload_falls_back_to_system() {
    let msg = DisplayMessage::todos("not json");
    let lines = render_todos_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    assert!(!lines.is_empty());
}

#[test]
fn render_todo_tool_result_uses_borderless_card_with_goal_scores() {
    let todos = vec![crate::todo::TodoItem {
        id: "render".to_string(),
        content: "Render the todo result".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("todo rendering".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(92)),
        completion_confidence: None,
        confidence_history: vec![
            crate::todo::ConfidenceState::from_legacy_score(85),
            crate::todo::ConfidenceState::from_legacy_score(92),
        ],
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("todo rendering".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(95)),
        feedback_loop: Some("Inspect the rendered frame".to_string()),
        delivery_state: Some(crate::todo::DeliveryState::from_legacy_score(92)),
        ..Default::default()
    }];
    let content = format!(
        "[todo] [tool timing: start=2026-07-13T19:51:50.261Z finish=2026-07-13T19:51:50.265Z duration=4ms] {}\n\nGoals:\n{}\n\n{}",
        serde_json::to_string_pretty(&todos).unwrap(),
        serde_json::to_string_pretty(&goals).unwrap(),
        crate::todo::TODO_CLOSED_FEEDBACK_LOOP_CONTINUATION_MESSAGE
    );
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content,
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("1 todos".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_todo".to_string(),
            name: "todo".to_string(),
            input: serde_json::json!({ "todos": todos, "goals": goals }),
            intent: Some("Track todo card work".to_string()),
            thought_signature: None,
        }),
    };

    let plain = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!plain.contains("Todos"), "{plain}");
    assert!(plain.contains("todo rendering  ●"), "{plain}");
    assert!(plain.contains("Closed feedback loop strong"), "{plain}");
    assert!(plain.contains("Relevance missing"), "{plain}");
    assert!(plain.contains("Coverage missing"), "{plain}");
    assert!(plain.contains("Delivery workflow_validated"), "{plain}");
    assert!(
        plain.contains("● Render the todo result · plausible"),
        "{plain}"
    );
    assert!(!plain.contains("(high)"), "{plain}");
    assert!(
        !plain.contains('╭'),
        "todo tool result should be borderless:\n{plain}"
    );
    assert!(
        !plain.contains("todo 1 items"),
        "generic tool row leaked:\n{plain}"
    );
}

include!("tests_partition_01_tests.rs");
include!("tests_partition_02_tests.rs");
include!("tests_partition_03_tests.rs");
