#[test]
fn render_todo_quality_gate_retry_shows_only_changed_goal_fields() {
    let todos = vec![crate::todo::TodoItem {
        id: "render".to_string(),
        content: "Render the entire unchanged todo plan".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("todo rendering".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(92)),
        ..Default::default()
    }];
    let before = crate::todo::TodoGoal {
        group: Some("todo rendering".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(90)),
        feedback_loop: Some("Inspect one frame".to_string()),
        feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::Indirect),
        feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::Narrow),
        ..Default::default()
    };
    let after = crate::todo::TodoGoal {
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(98)),
        feedback_loop: Some(
            "Render before and after fixtures and assert unchanged fields are absent".to_string(),
        ),
        feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::Representative),
        feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::MainPaths),
        ..before.clone()
    };
    let updates = vec![crate::todo::TodoGoalChange {
        before: Some(before),
        after: Some(after.clone()),
        fields: vec![
            crate::todo::TodoGoalField::ClosedFeedbackLoop,
            crate::todo::TodoGoalField::FeedbackLoop,
            crate::todo::TodoGoalField::FeedbackLoopRelevance,
            crate::todo::TodoGoalField::FeedbackLoopCoverage,
        ],
    }];
    let content = format!(
        "{}\n\nGoals:\n{}\n\nGoal updates:\n{}\n\n{}",
        serde_json::to_string_pretty(&todos).unwrap(),
        serde_json::to_string_pretty(&vec![after]).unwrap(),
        serde_json::to_string_pretty(&updates).unwrap(),
        crate::todo::TODO_CLOSED_FEEDBACK_LOOP_CONTINUATION_MESSAGE,
    );
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content,
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("1 todos".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_todo_update".to_string(),
            name: "todo".to_string(),
            input: serde_json::Value::Null,
            intent: Some("Refine the todo feedback loop".to_string()),
            thought_signature: None,
        }),
    };

    let plain = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("todo rendering  updated"), "{plain}");
    assert!(
        plain.contains("Closed feedback loop strong → closed"),
        "{plain}"
    );
    assert!(
        plain.contains("Feedback-loop relevance indirect → representative"),
        "{plain}"
    );
    assert!(
        plain.contains("Feedback-loop coverage narrow → main_paths"),
        "{plain}"
    );
    assert!(!plain.contains("Feedback ·"), "{plain}");
    assert!(
        !plain.contains("assert unchanged fields are absent"),
        "{plain}"
    );
    assert!(
        !plain.contains("Render the entire unchanged todo plan"),
        "{plain}"
    );
    assert!(!plain.contains("Alignment score"), "{plain}");
    assert!(!plain.contains("Keep the todo card concise"), "{plain}");
    assert!(!plain.contains("See current work at a glance"), "{plain}");
}

#[test]
fn render_goal_update_size_is_bounded_when_narrative_evidence_is_long() {
    let long_text = "verbose evidence ".repeat(600);
    let goal = crate::todo::TodoGoal {
        group: Some("compact assessment".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::Strong),
        delivery_state: Some(crate::todo::DeliveryState::Integrated),
        autonomy: Some(crate::todo::Autonomy::NecessaryFollowthrough),
        iteration_maturity: Some(crate::todo::IterationMaturity::OutcomeReached),
        feedback_loop: Some(long_text.clone()),
        stopping_evidence: Some(long_text),
        ..Default::default()
    };
    let update = crate::todo::TodoGoalChange {
        before: None,
        after: Some(goal),
        fields: vec![
            crate::todo::TodoGoalField::ClosedFeedbackLoop,
            crate::todo::TodoGoalField::DeliveryState,
            crate::todo::TodoGoalField::Autonomy,
            crate::todo::TodoGoalField::IterationMaturity,
            crate::todo::TodoGoalField::FeedbackLoop,
            crate::todo::TodoGoalField::StoppingEvidence,
        ],
    };

    let lines = render_todo_goal_updates(&[update], 95);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(lines.len(), 5, "narrative text must not add rows:\n{plain}");
    assert!(!plain.contains("Feedback"), "{plain}");
    assert!(!plain.contains("Stopping evidence"), "{plain}");
    assert!(!plain.contains("verbose evidence"), "{plain}");
}

#[test]
fn render_todo_plan_update_card_shows_only_changed_intent_fields() {
    let todos = vec![crate::todo::TodoItem {
        id: "render".to_string(),
        content: "Render the entire unchanged todo plan".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(92)),
        ..Default::default()
    }];
    let before = crate::todo::TodoPlan {
        user_intention: Some("Ship the plan-level intent gate".to_string()),
        understands_user_intent: Some(crate::todo::IntentUnderstanding::from_legacy_score(80)),
        ..Default::default()
    };
    let after = crate::todo::TodoPlan {
        understands_user_intent: Some(crate::todo::IntentUnderstanding::from_legacy_score(97)),
        ..before.clone()
    };
    let update = crate::todo::TodoPlanChange {
        before: Some(before),
        after: Some(after.clone()),
        fields: vec![crate::todo::TodoPlanField::UnderstandsUserIntent],
    };
    let content = format!(
        "{}\n\nPlan:\n{}\n\nPlan updates:\n{}",
        serde_json::to_string_pretty(&todos).unwrap(),
        serde_json::to_string_pretty(&after).unwrap(),
        serde_json::to_string_pretty(&update).unwrap(),
    );
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content,
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("1 todos".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_plan_update".to_string(),
            name: "todo".to_string(),
            input: serde_json::Value::Null,
            intent: Some("Reassess the user's intent".to_string()),
            thought_signature: None,
        }),
    };

    let plain = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("Plan  updated"), "{plain}");
    assert!(
        plain.contains("Understands user intent partial → clear"),
        "{plain}"
    );
    // Unchanged fields and the full plan stay out of the refinement card.
    assert!(!plain.contains("User intention"), "{plain}");
    assert!(
        !plain.contains("Render the entire unchanged todo plan"),
        "{plain}"
    );
}

#[test]
fn parse_todo_tool_output_accepts_timestamp_only_header() {
    let todos = vec![crate::todo::TodoItem {
        id: "timed".to_string(),
        content: "Render the restored todo".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        ..Default::default()
    }];
    let content = format!(
        "[2026-07-13T19:51:50.261Z] [todo] {}",
        serde_json::to_string(&todos).unwrap()
    );

    let parsed = parse_todo_tool_output(&content).expect("timestamped todo payload");
    assert_eq!(parsed.todos.len(), 1);
    assert_eq!(parsed.todos[0].id, todos[0].id);
    assert_eq!(parsed.todos[0].content, todos[0].content);
    assert!(parsed.goals.is_empty());
    assert!(parsed.goal_updates.is_empty());
}

#[test]
fn unbiased_visual_prompt_retry_renders_complete_feedback_change() {
    const PROMPT: &str = "can you make a pelican riding a bike animation in html and vanillia js ";
    const INITIAL_FEEDBACK: &str = "Open the page in a browser, inspect runtime errors, and verify animation state changes over time.";
    const REVISED_FEEDBACK: &str = "Serve the files locally, load them in a real browser at desktop and mobile viewport sizes, assert zero console/page errors, sample wheel and scenery transforms at two timestamps to prove motion, and exercise pause plus speed controls to confirm state changes.";
    const REVISED_OBJECTIVE: &str = "Deliver a responsive standalone animation whose pelican visibly pedals a moving bicycle through a layered seaside scene at 60fps where supported, with working pause/resume and three-speed controls, accessible labels, no external runtime dependencies, and zero browser console errors.";

    // Keep the eval input neutral. The visual verification strategy must come
    // from the model's todo refinement, not from criteria planted in the prompt.
    for biased_term in [
        "feedback loop",
        "browser",
        "console",
        "viewport",
        "screenshot",
        "visual quality",
    ] {
        assert!(!PROMPT.to_ascii_lowercase().contains(biased_term));
    }

    let todos = vec![crate::todo::TodoItem {
        id: "implement".to_string(),
        content: "Implement the illustrated pelican bicycle scene and responsive styling"
            .to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("pelican-bike-animation".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(90)),
        ..Default::default()
    }];
    let render = |goal: crate::todo::TodoGoal,
                  intention: &str,
                  continuation: Option<&str>,
                  tool_data: Option<crate::message::ToolCall>| {
        let plan = crate::todo::TodoPlan {
            user_intention: Some(intention.to_string()),
            understands_user_intent: Some(crate::todo::IntentUnderstanding::from_legacy_score(96)),
            ..Default::default()
        };
        let mut content = format!(
            "[todo] [tool timing: start=2026-07-13T19:51:50.261Z finish=2026-07-13T19:51:50.265Z duration=4ms] {}\n\nPlan:\n{}\n\nGoals:\n{}",
            serde_json::to_string_pretty(&todos).unwrap(),
            serde_json::to_string_pretty(&plan).unwrap(),
            serde_json::to_string_pretty(&vec![goal]).unwrap()
        );
        if let Some(continuation) = continuation {
            content.push_str("\n\n");
            content.push_str(continuation);
        }
        let msg = DisplayMessage {
            role: "tool".to_string(),
            content,
            tool_calls: Vec::new(),
            duration_secs: None,
            title: Some("1 todos".to_string()),
            tool_data,
        };
        render_tool_message(&msg, 72, crate::config::DiffDisplayMode::Off)
            .iter()
            .map(extract_line_text)
            .collect::<Vec<_>>()
            .join("\n")
    };

    let initial = render(
        crate::todo::TodoGoal {
            group: Some("pelican-bike-animation".to_string()),
            closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(90)),
            feedback_loop: Some(INITIAL_FEEDBACK.to_string()),
            ..Default::default()
        },
        "Make a pelican riding a bike animation that clearly works in a browser",
        Some(crate::todo::TODO_CLOSED_FEEDBACK_LOOP_CONTINUATION_MESSAGE),
        Some(crate::message::ToolCall {
            id: "call_initial_todo".to_string(),
            name: "todo".to_string(),
            input: serde_json::Value::Null,
            intent: Some("Track implementation and browser verification".to_string()),
            thought_signature: None,
        }),
    );
    assert!(initial.contains("pelican-bike-animation"), "{initial}");
    assert!(initial.contains("Closed feedback loop strong"), "{initial}");
    assert!(!without_whitespace(&initial).contains(&without_whitespace(INITIAL_FEEDBACK)));

    // Simulate a restored/mirrored result whose ToolCall association was lost.
    // The structured result must still render as the same complete todo card.
    let revised = render(
        crate::todo::TodoGoal {
            group: Some("pelican-bike-animation".to_string()),
            closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(98)),
            feedback_loop: Some(REVISED_FEEDBACK.to_string()),
            ..Default::default()
        },
        REVISED_OBJECTIVE,
        None,
        None,
    );
    let compact_revised = without_whitespace(&revised);
    assert!(revised.contains("pelican-bike-animation"), "{revised}");
    assert!(
        revised.contains("Relevance missing · Coverage missing · Traceability missing"),
        "{revised}"
    );
    assert!(!revised.contains("Closed feedback loop closed"));
    assert!(!compact_revised.contains(&without_whitespace(REVISED_FEEDBACK)));
    assert!(revised.contains("● Implement"), "{revised}");
}

#[test]
fn visually_appealing_prompt_batched_retry_renders_complete_todo_card() {
    // This fixture is only the first todo retry emitted after the
    // closed feedback loop continuation. The eval stops here and deliberately does
    // not depend on the model implementing or completing the visual task.
    const PROMPT: &str =
        "make the most visually appealing pelican on a bike animation with html and vanillia js";
    const FEEDBACK: &str = "At each iteration, render at 1440x900 and 390x844, capture screenshots, and score five checks: scene fills viewport without clipping, focal subject is centered, at least six distinct motion layers run smoothly, controls respond, and no console errors occur. Refine until all checks pass.";
    const OBJECTIVE: &str = "Deliver a single-page vanilla HTML/CSS/JS animation whose pelican cyclist remains legible and visually balanced at desktop and mobile sizes, includes six or more coordinated motion layers, supports interactive speed controls, and runs with zero console errors.";

    assert!(!PROMPT.contains("1440x900"));
    assert!(!PROMPT.contains("screenshot"));
    assert!(!PROMPT.contains("console"));
    assert!(!PROMPT.contains("feedback"));

    let todos = vec![crate::todo::TodoItem {
        id: "inspect".to_string(),
        content: "Inspect the starter project and determine the page structure".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("pelican-bike".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(95)),
        ..Default::default()
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("pelican-bike".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(98)),
        feedback_loop: Some(FEEDBACK.to_string()),
        feedback_loop_traceability: Some(crate::todo::FeedbackLoopTraceability::Complete),
        ..Default::default()
    }];
    let plan = crate::todo::TodoPlan {
        user_intention: Some(OBJECTIVE.to_string()),
        understands_user_intent: Some(crate::todo::IntentUnderstanding::from_legacy_score(97)),
        ..Default::default()
    };
    let todo_output = format!(
        "{}\n\nPlan:\n{}\n\nGoals:\n{}",
        serde_json::to_string_pretty(&todos).unwrap(),
        serde_json::to_string_pretty(&plan).unwrap(),
        serde_json::to_string_pretty(&goals).unwrap()
    );
    let content = format!(
        "--- [1] todo ---\n{todo_output}\n\n--- [2] ls ---\n./\n\n0 files, 0 directories\n\nCompleted: 2 succeeded, 0 failed"
    );
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content,
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_batch".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "intent": "Inspect starter files and strengthen measurable visual goals",
                "tool_calls": [
                    {
                        "tool": "todo",
                        "intent": "Make the visual outcome objectively verifiable",
                        "todos": todos,
                        "goals": goals
                    },
                    { "tool": "ls", "path": "." }
                ]
            }),
            intent: Some(
                "Inspect starter files and strengthen measurable visual goals".to_string(),
            ),
            thought_signature: None,
        }),
    };

    let rendered = render_tool_message(&msg, 84, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    let compact = without_whitespace(&rendered);

    assert!(rendered.contains("✓ todo"), "{rendered}");
    assert!(rendered.contains("pelican-bike"), "{rendered}");
    assert!(
        compact.contains(&without_whitespace(OBJECTIVE)),
        "batched todo plan intention was truncated:\n{rendered}"
    );
    // Compact transcript cards show the goal's quality assessments rather than
    // repeating its potentially long feedback-loop prose. The full prose remains
    // available in the serialized todo payload and the todos side panel.
    assert!(rendered.contains("Relevance missing · Coverage missing"));
    assert!(!compact.contains(&without_whitespace(FEEDBACK)));
    let goal_details = rendered
        .split_once("pelican-bike")
        .map(|(_, details)| details)
        .and_then(|details| details.split("● Inspect").next())
        .expect("todo item should follow the batched goal details");
    assert!(
        !goal_details.contains('…'),
        "batched todo goal details must not truncate:\n{rendered}"
    );
}

#[test]
fn render_ownership_gated_todo_result_keeps_the_full_card() {
    let todos = vec![crate::todo::TodoItem {
        id: "ship".to_string(),
        content: "Deliver the complete workflow".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
        group: Some("ship outcome".to_string()),
        confidence: Some(crate::todo::ConfidenceState::from_legacy_score(95)),
        ..Default::default()
    }];
    let goals = vec![crate::todo::TodoGoal {
        group: Some("ship outcome".to_string()),
        closed_feedback_loop: Some(crate::todo::FeedbackLoopState::from_legacy_score(100)),
        feedback_loop: Some("Run the complete workflow".to_string()),
        feedback_loop_relevance: Some(crate::todo::FeedbackLoopRelevance::Representative),
        feedback_loop_coverage: Some(crate::todo::FeedbackLoopCoverage::MainPaths),
        feedback_loop_traceability: Some(crate::todo::FeedbackLoopTraceability::Complete),
        delivery_state: Some(crate::todo::DeliveryState::from_legacy_score(80)),
        ..Default::default()
    }];
    let content = format!(
        "{}\n\nGoals:\n{}\n\n{}",
        serde_json::to_string_pretty(&todos).unwrap(),
        serde_json::to_string_pretty(&goals).unwrap(),
        crate::todo::TODO_OWNERSHIP_CONTINUATION_MESSAGE
    );
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content,
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("1 todos".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_todo_ownership".to_string(),
            name: "todo".to_string(),
            input: serde_json::json!({ "todos": todos, "goals": goals }),
            intent: Some("Complete the full user outcome".to_string()),
            thought_signature: None,
        }),
    };

    let plain = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("ship outcome  ●"), "{plain}");
    assert!(plain.contains("Deliver the complete workflow"), "{plain}");
    assert!(
        plain.contains("✓ All quality gates passing · Delivery workflow_validated"),
        "{plain}"
    );
    assert!(!plain.contains("todo 1 items"), "{plain}");
}

#[test]
fn render_background_task_messages_prefer_display_name() {
    let completion = DisplayMessage::background_task(
        "**Background task** `bg123` · `Run integration tests` (`bash`) · ✓ completed · 7.1s · exit 0\n\n_No output captured._\n\n_Full output:_ `bg action=\"output\" task_id=\"bg123\"`",
    );
    let completion_plain =
        render_background_task_message(&completion, 100, crate::config::DiffDisplayMode::Off)
            .iter()
            .map(extract_line_text)
            .collect::<Vec<_>>()
            .join("\n");
    assert!(completion_plain.contains("✓ bg Run integration tests completed · bg123"));

    let progress = DisplayMessage::background_task(
        "**Background task progress** `bg123` · `Run integration tests` (`bash`)\n\n[#####-------] 42% · Running tests (reported)",
    );
    let progress_plain =
        render_background_task_message(&progress, 100, crate::config::DiffDisplayMode::Off)
            .iter()
            .map(extract_line_text)
            .collect::<Vec<_>>()
            .join("\n");
    assert!(progress_plain.contains("◌ bg Run integration tests · bg123"));
}

#[test]
fn render_system_message_uses_scheduled_task_card() {
    let msg = DisplayMessage::system(
        "[Scheduled task]\nA scheduled task for this session is now due.\n\nTask: Follow up on the scheduler test\nWorking directory: /home/jeremy/jcode\nRelevant files: src/tui/ui_messages.rs\nBranch: master\n\nBackground: Verify the scheduled task card styling\nSuccess criteria: The due task renders clearly\nScheduled by session: session_test",
    );

    let lines = render_system_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains(width_stable_system_title(
        "⏰ scheduled task due",
        "scheduled task due"
    )));
    assert!(plain.contains("This scheduled task is now active in this session."));
    assert!(plain.contains("Follow up on the scheduler test"));
    assert!(plain.contains("Verify the scheduled task card styling"));
    assert!(!plain.contains("[Scheduled task]"));
    assert!(!plain.contains("A scheduled task for this session is now due."));
}

#[test]
fn render_tool_message_uses_scheduled_card() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Scheduled task 'Follow up on the scheduler test' for in 1m (id: sched_abc123)\nWorking directory: /home/jeremy/jcode\nRelevant files: src/tui/ui_messages.rs\nTarget: resume session session_test".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("scheduled: Follow up on the scheduler test".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_schedule_card".to_string(),
            name: "schedule".to_string(),
            input: serde_json::json!({
                "task": "Follow up on the scheduler test",
                "wake_in_minutes": 1,
                "target": "resume"
            }),
            intent: None, thought_signature: None, }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains(width_stable_system_title("⏰ scheduled", "scheduled")));
    assert!(plain.contains("Will run in 1m."));
    assert!(plain.contains("Follow up on the scheduler test"));
    assert!(plain.contains("session session_test"));
    assert!(plain.contains("sched_abc123"));
    assert!(!plain.contains("✓ schedule"));
}

#[test]
fn render_assistant_message_renders_plan_block_as_card() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage::assistant(
        "Here is the plan:\n\n```plan\n# Ship compact mode\n\n## Goal\nAdd a compact message mode.\n\n## Approach\n1. Add config flag\n2. Wire renderer\n```\n\nLet me know if this works.",
    );

    let lines = render_assistant_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    crate::tui::markdown::set_center_code_blocks(saved);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("Here is the plan:"), "plain: {plain}");
    assert!(plain.contains("⛭ Ship compact mode"), "plain: {plain}");
    assert!(plain.contains('╭'), "expected card border: {plain}");
    assert!(plain.contains('╰'), "expected card border: {plain}");
    assert!(plain.contains("Add a compact message mode."));
    assert!(plain.contains("Let me know if this works."));
    assert!(
        !plain.contains("```"),
        "plan fence markers should not render: {plain}"
    );
}

#[test]
fn render_assistant_message_plan_card_survives_unterminated_fence() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage::assistant("```plan\n# Streaming plan\n\n- step one");

    let lines = render_assistant_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    crate::tui::markdown::set_center_code_blocks(saved);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("⛭ Streaming plan"), "plain: {plain}");
    assert!(plain.contains("step one"), "plain: {plain}");
}

#[test]
fn render_assistant_message_plan_card_keeps_nested_fences_inside() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage::assistant(
        "```plan\n# Validation plan\n\n```bash\ncargo test -p jcode-tui\n```\n\nAfter the block.\n```\n\nOutside text.",
    );

    let lines = render_assistant_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    crate::tui::markdown::set_center_code_blocks(saved);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("⛭ Validation plan"), "plain: {plain}");
    assert!(plain.contains("cargo test -p jcode-tui"), "plain: {plain}");
    assert!(plain.contains("After the block."), "plain: {plain}");
    assert!(plain.contains("Outside text."), "plain: {plain}");
    // The nested bash content stays inside the card borders.
    let bash_line = lines
        .iter()
        .map(extract_line_text)
        .find(|line| line.contains("cargo test -p jcode-tui"))
        .expect("missing bash line");
    assert!(
        bash_line.trim_start().starts_with('│'),
        "nested fence content should be inside the card: {bash_line}"
    );
}

#[test]
fn split_plan_segments_returns_none_without_plan_block() {
    assert!(split_plan_segments("Just some text\n\n```rust\nfn main() {}\n```").is_none());
    assert!(split_plan_segments("mentions plan but no fence").is_none());
}

#[test]
fn render_assistant_message_truncates_tool_calls_to_single_line() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage {
        role: "assistant".to_string(),
        content: "Done.".to_string(),
        tool_calls: vec![
            "read".to_string(),
            "grep".to_string(),
            "apply_patch".to_string(),
            "batch".to_string(),
        ],
        duration_secs: None,
        title: None,
        tool_data: None,
    };

    let lines = render_assistant_message(&msg, 20, crate::config::DiffDisplayMode::Off);
    assert_eq!(extract_line_text(&lines[1]), "");
    let tool_lines: Vec<String> = lines
        .iter()
        .skip(2)
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();

    assert!(
        tool_lines.len() == 1,
        "expected single-line tool-call summary: {tool_lines:?}"
    );
    assert!(
        tool_lines[0].contains("tools:"),
        "expected tool summary label on first line: {tool_lines:?}"
    );
    assert!(
        tool_lines.iter().all(|line| line.width() <= 20),
        "tool-call summary line should respect available width: {tool_lines:?}"
    );
    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_assistant_message_centers_single_line_tool_summary() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage {
        role: "assistant".to_string(),
        content: "Done.".to_string(),
        tool_calls: vec![
            "read".to_string(),
            "grep".to_string(),
            "apply_patch".to_string(),
            "batch".to_string(),
        ],
        duration_secs: None,
        title: None,
        tool_data: None,
    };

    let lines = render_assistant_message(&msg, 28, crate::config::DiffDisplayMode::Off);
    assert_eq!(extract_line_text(&lines[1]), "");
    let tool_lines: Vec<String> = lines
        .iter()
        .skip(2)
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();

    assert!(
        tool_lines.len() == 1,
        "expected single-line tool-call summary: {tool_lines:?}"
    );
    let first_pad = tool_lines[0].chars().take_while(|c| *c == ' ').count();
    assert!(
        first_pad > 0,
        "tool summary should still be padded/centered as a block: {tool_lines:?}"
    );
    assert!(
        lines
            .iter()
            .skip(2)
            .all(|line| line.alignment == Some(ratatui::layout::Alignment::Left)),
        "centered tool summary should use a shared left-aligned block pad"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_assistant_message_without_body_does_not_add_extra_blank_line_before_tool_summary() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let msg = DisplayMessage {
        role: "assistant".to_string(),
        content: String::new(),
        tool_calls: vec!["read".to_string()],
        duration_secs: None,
        title: None,
        tool_data: None,
    };

    let lines = render_assistant_message(&msg, 28, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 1, "rendered={rendered:?}");
    assert!(rendered[0].contains("tool:"), "rendered={rendered:?}");

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_assistant_message_centered_mode_keeps_markdown_unpadded_for_center_alignment() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage::assistant(
        "streaming-block streaming-block streaming-block streaming-block",
    );

    let lines = render_assistant_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let content_line = lines
        .iter()
        .find(|line| extract_line_text(line).contains("streaming-block"))
        .expect("expected assistant markdown line");

    let first_pad = extract_line_text(content_line)
        .chars()
        .take_while(|c| *c == ' ')
        .count();
    assert_eq!(
        first_pad, 0,
        "centered assistant markdown should not inject left padding: {lines:?}"
    );
    assert_eq!(
        content_line.alignment, None,
        "assistant render should leave centered prose alignment unset for outer centering"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_assistant_message_recenters_structured_markdown_to_actual_width() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage::assistant("- one\n- two");

    let lines = render_assistant_message(&msg, 140, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();
    let bullets: Vec<&String> = rendered.iter().filter(|line| line.contains("• ")).collect();

    assert_eq!(
        bullets.len(),
        2,
        "expected two rendered bullet lines: {rendered:?}"
    );
    let first_pad = leading_spaces(bullets[0]);
    let second_pad = leading_spaces(bullets[1]);
    assert_eq!(
        first_pad, second_pad,
        "simple list should share a block pad: {rendered:?}"
    );
    assert!(
        first_pad > 45,
        "list should be re-centered to the full display width: {rendered:?}"
    );
    assert!(
        bullets
            .iter()
            .all(|line| line[leading_spaces(line)..].starts_with("• ")),
        "bullet markers should remain flush-left within the centered block: {rendered:?}"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_system_message_centered_mode_caps_wrap_width_for_visible_gutters() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage::system(
        "This is a long centered-mode system notification that should keep visible side gutters instead of stretching nearly edge to edge in a wide terminal.",
    );

    let lines = render_system_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();

    assert!(
        rendered.iter().all(|line| line.starts_with("          ")),
        "centered system message should retain visible left padding in wide layouts: {rendered:?}"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_system_message_uses_minimal_inline_style_for_reload_title() {
    let msg = DisplayMessage::system("Reloading server with newer binary...").with_title("Reload");

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !plain.contains('╭'),
        "unexpected reload card border: {plain}"
    );
    assert!(
        !plain.contains('╰'),
        "unexpected reload card border: {plain}"
    );
    assert!(
        !plain.contains("⚡ reload"),
        "unexpected reload card title: {plain}"
    );
    assert!(plain.contains("Reloading server with newer binary"));
}

#[test]
fn render_system_message_uses_connection_card_for_reconnect_status() {
    let msg = DisplayMessage::system(
        "⚡ Connection lost - retrying (attempt 2, 7s) - connection reset by server · resume: jcode --resume koala",
    )
    .with_title("Connection");

    let lines = render_system_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.contains("reconnecting"),
        "expected reconnect card title: {plain}"
    );
    assert!(plain.contains("Retrying · attempt 2 · 7s"));
    assert!(plain.contains("connection reset by server"));
    assert!(plain.contains("jcode --resume koala"));
}

#[test]
fn render_swarm_message_centered_mode_caps_wrap_width_for_long_notifications() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage::swarm(
        "File activity",
        "/home/jeremy/jcode/src/tui/ui_messages.rs - moss just edited this file while you were working nearby, so the notification should still read as centered in wide layouts.",
    );

    let lines = render_swarm_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();
    let first_pad = rendered[0].chars().take_while(|c| *c == ' ').count();

    assert!(
        first_pad >= 8,
        "centered swarm notification should keep a clearly visible left gutter: {rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .all(|line| line.is_empty() || line.starts_with(&" ".repeat(first_pad))),
        "centered swarm notification should share one left pad across wrapped lines: {rendered:?}"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_swarm_message_collapsed_shows_tldr_and_expand_badge_only() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let content = jcode_tui_messages::encode_collapsible_swarm_content(
        "fixed the flaky test",
        "The flaky test was caused by a race in the setup helper.\n\nI rewrote it to use a barrier.",
    );
    let msg = DisplayMessage::swarm("DM from sheep", content);

    let lines = render_swarm_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("fixed the flaky test"), "{plain}");
    assert!(plain.contains(super::SWARM_EXPAND_BADGE), "{plain}");
    assert!(
        !plain.contains("race in the setup helper"),
        "collapsed card must hide the full body: {plain}"
    );
    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_swarm_message_expanded_shows_body_and_collapse_badge() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    let collapsed = jcode_tui_messages::encode_collapsible_swarm_content(
        "fixed the flaky test",
        "The flaky test was caused by a race in the setup helper.",
    );
    let expanded =
        jcode_tui_messages::toggle_collapsible_swarm_content(&collapsed).expect("toggle");
    let msg = DisplayMessage::swarm("DM from sheep", expanded);

    let lines = render_swarm_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("fixed the flaky test"), "{plain}");
    assert!(plain.contains(super::SWARM_COLLAPSE_BADGE), "{plain}");
    assert!(plain.contains("race in the setup helper"), "{plain}");
    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_tool_message_prefers_subagent_title_with_model() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "done".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("Verify subagent model (general · gpt-5.4)".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_1".to_string(),
            name: "subagent".to_string(),
            input: serde_json::json!({
                "description": "Verify subagent model",
                "subagent_type": "general"
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let rendered: String = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();

    assert!(rendered.contains("subagent Verify subagent model (general · gpt-5.4)"));
}
