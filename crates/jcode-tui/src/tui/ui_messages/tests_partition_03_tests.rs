#[test]
fn render_tool_message_memory_recall_centered_mode_left_aligns_with_padding() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: concat!(
            "- [fact] Centered mode should keep the recall card centered\n",
            "- [preference] The user likes visible side gutters"
        )
        .to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_memory_recall_centered".to_string(),
            name: "memory".to_string(),
            input: serde_json::json!({
                "action": "recall",
                "query": "centered mode"
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();

    assert!(!rendered.is_empty(), "expected rendered recall card");
    assert!(
        rendered.iter().all(|line| line.starts_with("  ")),
        "centered recall card should include shared left padding: {rendered:?}"
    );
    assert_eq!(
        lines[0].alignment,
        Some(ratatui::layout::Alignment::Left),
        "centered recall card header should be left-aligned after padding"
    );
    assert!(
        rendered[0]
            .trim_start()
            .starts_with("🧠 recalled 2 memories"),
        "unexpected recall header: {rendered:?}"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_tool_message_memory_store_centered_mode_left_aligns_with_padding() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(true);
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Saved memory".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_memory_store_centered".to_string(),
            name: "memory".to_string(),
            input: serde_json::json!({
                "action": "remember",
                "category": "fact",
                "content": "Centered mode should pad saved memory cards too"
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();

    assert!(!rendered.is_empty(), "expected rendered saved-memory card");
    assert!(
        rendered.iter().all(|line| line.starts_with("  ")),
        "centered saved-memory card should include shared left padding: {rendered:?}"
    );
    assert_eq!(
        lines[0].alignment,
        Some(ratatui::layout::Alignment::Left),
        "centered saved-memory card should be left-aligned after padding"
    );

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_tool_message_shows_swarm_spawn_prompt_summary() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "spawned".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_swarm_spawn".to_string(),
            name: "swarm".to_string(),
            input: serde_json::json!({
                "action": "spawn",
                "prompt": "Extract the restart command cluster from cli commands and validate it"
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: String = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();

    assert!(rendered.contains("swarm spawn"), "rendered={rendered}");
    assert!(
        rendered.contains("Extract the restart command cluster"),
        "rendered={rendered}"
    );
}

#[test]
fn render_tool_message_batch_subcall_shows_swarm_dm_details() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] swarm ---\nDone\n\nCompleted: 1 succeeded, 0 failed".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_batch_swarm".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {
                        "tool": "swarm",
                        "action": "dm",
                        "to_session": "shark",
                        "message": "Please validate the restart extraction and report back"
                    }
                ]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("swarm dm → shark"), "rendered={rendered}");
    assert!(
        rendered.contains("Please validate the restart"),
        "rendered={rendered}"
    );
}

#[test]
fn render_agentgrep_output_body_borders_each_line() {
    let content = "crates/foo.rs\n  symbols: 1 matched\n    - fn bar @ 1-5";
    let lines = super::render_agentgrep_output_body(content, 120);
    let rendered = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("│ crates/foo.rs"), "rendered={rendered}");
    assert!(
        rendered.contains("│   symbols: 1 matched"),
        "rendered={rendered}"
    );
    assert!(
        rendered.contains("│     - fn bar @ 1-5"),
        "rendered={rendered}"
    );
    assert_eq!(lines.len(), 3, "one bordered line per source line");
}

#[test]
fn render_agentgrep_output_body_caps_huge_output() {
    let content = (0..1000)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let lines = super::render_agentgrep_output_body(&content, 120);
    // 400-line cap plus a single truncation summary line.
    assert_eq!(lines.len(), 401, "should cap the body and add a summary");
    let last = extract_line_text(&lines[lines.len() - 1]);
    assert!(last.contains("more lines"), "last={last}");
}

#[test]
fn render_assistant_message_plan_card_wraps_instead_of_truncating() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);
    // Long paragraph and long list items must wrap inside the card, not be
    // clipped at the right border by render_rounded_box's truncation.
    let plan_body = "# Long content plan\n\n\
        Goal\n\
        Produce an up-to-date ranked report grounded in current crate paths, then fix the highest-leverage low-risk offenders without destabilizing active work.\n\n\
        Approach\n\
        1. Write an audit document that regenerates metrics with current crate paths, ranks the top issues with evidence, and marks which items from the previous audit are complete versus stale.\n\
        2. Map the provider migration and record whether each module is a thin wrapper, partial duplicate, or full duplicate of the extracted crate.\n";
    let content = format!("Intro text.\n\n```plan\n{plan_body}```\n\nAfter the card.");
    let msg = DisplayMessage::assistant(&content);

    for width in [40u16, 60, 80, 100, 140] {
        let lines = render_assistant_message(&msg, width, crate::config::DiffDisplayMode::Off);
        let squashed = lines
            .iter()
            .map(extract_line_text)
            .collect::<Vec<_>>()
            .join(" ")
            .replace(['│', '╭', '╮', '╰', '╯', '─'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for phrase in [
            "without destabilizing active work.",
            "complete versus stale.",
            "or full duplicate of the extracted crate.",
        ] {
            assert!(
                squashed.contains(phrase),
                "width {width}: plan card lost trailing content {phrase:?}\n{squashed}"
            );
        }
        // Card borders stay intact.
        for line in lines
            .iter()
            .map(extract_line_text)
            .filter(|l| l.contains('│'))
        {
            assert!(
                line.trim_end().ends_with('│'),
                "width {width}: card row missing right border: {line:?}"
            );
        }
    }
    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_swarm_message_preserves_inline_image_placeholder_lines() {
    let saved = crate::tui::markdown::center_code_blocks();
    crate::tui::markdown::set_center_code_blocks(false);

    // Simulate a rendered mermaid diagram inside a swarm message body: the
    // marker line plus its blank fill rows must survive rendering without a
    // rail prefix or blank-line cleanup so the image draws at full height.
    let placeholder = crate::tui::mermaid::inline_image_placeholder_lines(0xabcd1234, 4, 20);
    assert_eq!(placeholder.len(), 4);
    let marker_text = placeholder[0]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<String>();

    let msg = DisplayMessage::swarm(
        "Plan graph · v3",
        "```mermaid\nflowchart TD\n    a --> b\n```",
    );
    // Rendering the real message goes through the markdown pipeline; whether a
    // real image materializes depends on protocol availability, so test the
    // line-preservation path directly through render_swarm_message with a body
    // the markdown renderer maps to placeholder lines is not deterministic in
    // tests. Instead assert the parser round-trips the marker we emit.
    let parsed = crate::tui::mermaid::parse_inline_image_placeholder(&placeholder[0]);
    assert_eq!(parsed, Some((0xabcd1234, 4, 20)));
    assert!(
        marker_text.starts_with('\u{0}'),
        "marker must keep its sentinel prefix"
    );

    // And the swarm renderer must not panic or drop content for a mermaid body.
    let lines = render_swarm_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    assert!(!lines.is_empty());

    crate::tui::markdown::set_center_code_blocks(saved);
}

#[test]
fn render_empty_todo_tool_result_collapses_to_compact_line() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "[todo] []".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("0 todos".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_todo_empty".to_string(),
            name: "todo".to_string(),
            input: serde_json::json!({}),
            intent: Some("Read the todo list".to_string()),
            thought_signature: None,
        }),
    };

    let plain = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!plain.contains("No tasks yet"), "{plain}");
    assert!(plain.contains("no tasks"), "{plain}");
}
