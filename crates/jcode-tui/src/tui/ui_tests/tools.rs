use super::*;

#[test]
fn test_summarize_apply_patch_input_ignores_begin_marker() {
    let patch = "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-old\n+new\n*** End Patch\n";
    let summary = tools_ui::summarize_apply_patch_input(patch);
    assert_eq!(summary, "src/lib.rs (6 lines)");
}

#[test]
fn test_summarize_apply_patch_input_multiple_files() {
    let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-a\n+b\n*** Update File: b.txt\n@@\n-c\n+d\n*** End Patch\n";
    let summary = tools_ui::summarize_apply_patch_input(patch);
    assert_eq!(summary, "2 files (10 lines)");
}

#[test]
fn test_extract_apply_patch_primary_file() {
    let patch = "*** Begin Patch\n*** Add File: new/file.rs\n+fn main() {}\n*** End Patch\n";
    let file = tools_ui::extract_apply_patch_primary_file(patch);
    assert_eq!(file.as_deref(), Some("new/file.rs"));
}

#[test]
fn test_patch_summaries_preserve_line_counts() {
    let single = "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n";
    assert_eq!(
        tools_ui::summarize_unified_patch_input(single),
        "src/lib.rs (5 lines)"
    );
    let multiple = format!("{single}--- /dev/null\n+++ b/new.rs\n@@ -0,0 +1 @@\n+new\n");
    assert_eq!(
        tools_ui::summarize_unified_patch_input(&multiple),
        "2 files (9 lines)"
    );
    for patch in ["", "@@\n-old\n+new\n"] {
        let expected = format!("({} lines)", patch.lines().count());
        assert_eq!(tools_ui::summarize_unified_patch_input(patch), expected);
        assert_eq!(tools_ui::summarize_apply_patch_input(patch), expected);
    }
}

#[test]
fn test_patch_headers_preserve_line_counts_and_token_severity() {
    let _guard = viewport_snapshot_test_lock();
    for (name, patch) in [
        (
            "patch",
            "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n",
        ),
        (
            "apply_patch",
            "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-old\n+new\n*** End Patch\n",
        ),
    ] {
        let tool = crate::message::ToolCall {
            id: "call_patch_badge".to_string(),
            name: name.to_string(),
            input: serde_json::json!({"patch_text": patch}),
            intent: None,
            thought_signature: None,
        };
        for (tokens, color) in [
            (0, rgb(118, 118, 118)),
            (3_999, rgb(118, 118, 118)),
            (4_000, rgb(214, 184, 92)),
            (11_999, rgb(214, 184, 92)),
            (12_000, rgb(224, 118, 118)),
        ] {
            let output = "x".repeat(tokens * crate::util::APPROX_CHARS_PER_TOKEN);
            let msg = DisplayMessage {
                role: "tool".to_string(),
                content: output.clone(),
                tool_calls: Vec::new(),
                duration_secs: None,
                title: None,
                tool_data: Some(tool.clone()),
            };
            let standalone =
                messages::render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
            let batch = tools_ui::render_batch_subcall_line(
                &tool,
                "✓",
                color,
                50,
                Some(120),
                Some(&output),
            );
            let label = crate::util::format_approx_token_count(tokens);
            for line in [&standalone[0], &batch] {
                let text = line_plain_text(line);
                assert!(
                    text.contains(&format!("({} lines)", patch.lines().count())),
                    "{name}: {text}"
                );
                assert!(text.ends_with(&label), "{name}: {text}");
                let badge = line
                    .spans
                    .iter()
                    .find(|span| span.content == label)
                    .expect("missing token badge");
                assert_eq!(badge.style.fg, Some(color), "{name}: {tokens}");
            }
        }
    }
}

#[test]
fn test_token_badges_survive_full_terminal_draw() {
    let _guard = viewport_snapshot_test_lock();
    let patch = "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-old\n+new\n*** End Patch\n";
    let mut version = 100;
    for centered_mode in [false, true] {
        for width in [60, 120] {
            for (tokens, color) in [
                (1_900, rgb(118, 118, 118)),
                (4_000, rgb(214, 184, 92)),
                (12_000, rgb(224, 118, 118)),
            ] {
                for batch in [false, true] {
                    let output = "x".repeat(tokens * crate::util::APPROX_CHARS_PER_TOKEN);
                    let (name, input, content) = if batch {
                        (
                            "batch",
                            serde_json::json!({"tool_calls": [{
                                "tool": "apply_patch", "patch_text": patch
                            }]}),
                            format!(
                                "--- [1] apply_patch ---\n{output}\n\nCompleted: 1 succeeded, 0 failed"
                            ),
                        )
                    } else {
                        (
                            "apply_patch",
                            serde_json::json!({"patch_text": patch}),
                            output,
                        )
                    };
                    version += 1;
                    let state = TestState {
                        display_messages: vec![DisplayMessage {
                            role: "tool".to_string(),
                            content,
                            tool_calls: Vec::new(),
                            duration_secs: None,
                            title: None,
                            tool_data: Some(crate::message::ToolCall {
                                id: format!("badge_{version}"),
                                name: name.to_string(),
                                input,
                                intent: None,
                                thought_signature: None,
                            }),
                        }],
                        messages_version: version,
                        centered_mode,
                        suppress_info_widgets: true,
                        ..Default::default()
                    };
                    clear_test_render_state_for_tests();
                    let backend = ratatui::backend::TestBackend::new(width, 24);
                    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
                    terminal.draw(|frame| draw(frame, &state)).expect("draw");
                    let buffer = terminal.backend().buffer();
                    let label = crate::util::format_approx_token_count(tokens);
                    let rows: Vec<String> = (0..24)
                        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
                        .collect();
                    let y = rows
                        .iter()
                        .position(|row| row.contains("apply_patch"))
                        .unwrap_or_else(|| panic!("missing tool row: {rows:#?}"))
                        as u16;
                    let row = &rows[y as usize];
                    assert!(row.contains("(6 lines)"), "{row}");
                    assert!(row.trim_end().ends_with(&label), "{row}");
                    let x = (0..width)
                        .find(|&x| {
                            (x..width)
                                .map(|x| buffer[(x, y)].symbol())
                                .collect::<String>()
                                .starts_with(&label)
                        })
                        .expect("visible token label");
                    for column in x..x + label.len() as u16 {
                        assert_eq!(buffer[(column, y)].fg, color, "{row}");
                    }
                    println!(
                        "width={width} centered={centered_mode} batch={batch} fg={color:?}: {}",
                        row.trim()
                    );
                }
            }
        }
    }
}

#[test]
fn test_batch_subcall_params_supports_flat_and_nested_shapes() {
    let flat = serde_json::json!({
        "tool": "read",
        "file_path": "src/session.rs",
        "offset": 0,
        "limit": 420
    });
    let nested = serde_json::json!({
        "tool": "read",
        "parameters": {
            "file_path": "src/main.rs",
            "offset": 2320,
            "limit": 220
        }
    });

    let flat_params = tools_ui::batch_subcall_params(&flat);
    let nested_params = tools_ui::batch_subcall_params(&nested);

    assert_eq!(flat_params["file_path"], "src/session.rs");
    assert_eq!(flat_params["offset"], 0);
    assert_eq!(flat_params["limit"], 420);

    assert_eq!(nested_params["file_path"], "src/main.rs");
    assert_eq!(nested_params["offset"], 2320);
    assert_eq!(nested_params["limit"], 220);
}

#[test]
fn test_batch_subcall_params_excludes_name_key() {
    let with_name = serde_json::json!({
        "name": "read",
        "file_path": "src/lib.rs",
        "offset": 0,
        "limit": 100
    });
    let params = tools_ui::batch_subcall_params(&with_name);
    assert_eq!(params["file_path"], "src/lib.rs");
    assert_eq!(params["offset"], 0);
    assert!(params.get("name").is_none());
    assert!(params.get("tool").is_none());
}

#[test]
fn test_batch_subcall_intent_supports_flat_and_nested_shapes() {
    let flat = serde_json::json!({
        "tool": "read",
        "intent": "Inspect flat input",
        "file_path": "src/lib.rs"
    });
    let nested = serde_json::json!({
        "tool": "read",
        "parameters": {
            "intent": "Inspect nested input",
            "file_path": "src/main.rs"
        }
    });

    let flat_params = tools_ui::batch_subcall_params(&flat);
    let nested_params = tools_ui::batch_subcall_params(&nested);

    assert_eq!(
        tools_ui::batch_subcall_intent(&flat, &flat_params).as_deref(),
        Some("Inspect flat input")
    );
    assert_eq!(
        tools_ui::batch_subcall_intent(&nested, &nested_params).as_deref(),
        Some("Inspect nested input")
    );
}

#[test]
fn test_parse_batch_sub_outputs_strips_footer_and_tracks_errors() {
    let content = "--- [1] read ---\n1234\n\n--- [2] grep ---\nError: 12345678\n\nCompleted: 1 succeeded, 1 failed";

    let results = tools_ui::parse_batch_sub_outputs(content);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].content, "1234");
    assert!(!results[0].errored);
    assert_eq!(results[1].content, "Error: 12345678");
    assert!(results[1].errored);
}

#[test]
fn test_parse_batch_sub_outputs_keeps_final_header_without_trailing_newline() {
    let content = "--- [1] read ---\n1234\n\n--- [2] grep ---";

    let results = tools_ui::parse_batch_sub_outputs(content);

    assert_eq!(results.len(), 2, "results={results:?}");
    assert_eq!(results[0].content, "1234");
    assert_eq!(results[1].content, "");
    assert!(!results[1].errored);
}

#[test]
fn test_render_tool_message_batch_flat_subcall_params_include_read_details() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] read ---\nok\n\n--- [2] read ---\nok\n\nCompleted: 2 succeeded, 0 failed"
            .to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_1".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {"tool": "read", "file_path": "src/session.rs", "offset": 0, "limit": 420},
                    {"tool": "read", "file_path": "src/main.rs", "offset": 2320, "limit": 220}
                ]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 3, "rendered={rendered:?}");
    assert!(
        rendered[0].contains("✓ batch 2 calls"),
        "rendered={rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("✓ read src/session.rs:0-420")),
        "missing first read subtool in {rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("✓ read src/main.rs:2320-2540")),
        "missing second read subtool in {rendered:?}"
    );
}

#[test]
fn test_render_tool_message_batch_subcalls_show_individual_token_badges() {
    let msg = DisplayMessage {
            role: "tool".to_string(),
            content:
                "--- [1] read ---\n1234\n\n--- [2] grep ---\n12345678\n\nCompleted: 2 succeeded, 0 failed"
                    .to_string(),
            tool_calls: vec![],
            duration_secs: None,
            title: None,
            tool_data: Some(ToolCall {
                id: "call_batch_tokens".to_string(),
                name: "batch".to_string(),
                input: serde_json::json!({
                    "tool_calls": [
                        {"tool": "read", "file_path": "src/session.rs", "offset": 0, "limit": 1},
                        {"tool": "grep", "pattern": "TODO", "path": "src"}
                    ]
                }),
                intent: None, thought_signature: None, }),
        };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 3, "rendered={rendered:?}");
    assert!(
        rendered[0].contains("✓ batch 2 calls"),
        "rendered={rendered:?}"
    );
    assert!(
        rendered[1].contains("read src/session.rs:0-1") && rendered[1].contains("1 tok"),
        "rendered={rendered:?}"
    );
    assert!(
        rendered[2].contains("grep 'TODO' in src") && rendered[2].contains("2 tok"),
        "rendered={rendered:?}"
    );
}

#[test]
fn test_render_tool_message_batch_first_subcall_token_badge_with_timing_prefix() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "[tool timing: start=2026-05-14T14:10:08.525Z finish=2026-05-14T14:10:08.598Z duration=73ms] --- [1] bash ---\n12345678\n\n--- [2] bash ---\n12345678\n\nCompleted: 2 succeeded, 0 failed"
            .to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_tokens_timing_prefix".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {"tool": "bash", "command": "echo first"},
                    {"tool": "bash", "command": "echo second"}
                ]
            }),
            intent: None, thought_signature: None, }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 3, "rendered={rendered:?}");
    assert!(
        rendered[1].contains("bash $ echo first") && rendered[1].contains("2 tok"),
        "first subcall should keep its token badge despite timing prefix: {rendered:?}"
    );
    assert!(
        rendered[2].contains("bash $ echo second") && rendered[2].contains("2 tok"),
        "rendered={rendered:?}"
    );
}

#[test]
fn test_render_tool_message_batch_last_subcall_keeps_token_badge_without_trailing_newline() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] read ---\n1234\n\n--- [2] grep ---".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_tokens_no_newline".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {"tool": "read", "file_path": "src/session.rs", "offset": 0, "limit": 1},
                    {"tool": "grep", "pattern": "TODO", "path": "src"}
                ]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 3, "rendered={rendered:?}");
    assert!(
        rendered[1].contains("read src/session.rs:0-1") && rendered[1].contains("1 tok"),
        "rendered={rendered:?}"
    );
    assert!(
        rendered[2].contains("grep 'TODO' in src") && rendered[2].contains("0 tok"),
        "rendered={rendered:?}"
    );
}

#[test]
fn test_render_tool_message_batch_partial_failure_shows_all_subcalls() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] read ---
ok

--- [2] agentgrep ---
Error: missing field `mode`

--- [3] grep ---
ok

Completed: 2 succeeded, 1 failed"
            .to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_partial".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {"tool": "read", "file_path": "src/lib.rs"},
                    {"tool": "agentgrep"},
                    {"tool": "grep", "pattern": "TODO", "path": "src"}
                ]
            }),
            intent: Some("Inspect schemas".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert!(
        rendered[0].contains("⚠ batch · Inspect schemas · 2/3 succeeded"),
        "rendered={rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("✗ agentgrep invalid input: missing mode")),
        "failed subcall should be attributed to agentgrep: {rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("✓ read src/lib.rs")),
        "successful read subcall should still be visible: {rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("✓ grep 'TODO' in src")),
        "successful grep subcall should still be visible: {rendered:?}"
    );
}

#[test]
fn test_render_tool_message_batch_all_failed_marks_all_children_failed() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] agentgrep ---\nError: missing field `mode`\n\n--- [2] agentgrep ---\nError: missing field `mode`\n\n--- [3] agentgrep ---\nError: missing field `mode`\n\nCompleted: 0 succeeded, 3 failed"
            .to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_all_failed".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {"tool": "agentgrep"},
                    {"tool": "agentgrep"},
                    {"tool": "agentgrep"}
                ]
            }),
            intent: None, thought_signature: None, }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert!(
        rendered[0].contains("✗ batch 3/3 failed"),
        "rendered={rendered:?}"
    );
    let failed_children = rendered
        .iter()
        .filter(|line| line.contains("✗ agentgrep invalid input: missing mode"))
        .count();
    assert_eq!(failed_children, 3, "rendered={rendered:?}");
    assert!(
        !rendered
            .iter()
            .any(|line| line.contains("✓ agentgrep") || line.contains("agentgrep missing mode")),
        "rendered={rendered:?}"
    );
}

#[test]
fn test_tool_summary_gmail_actions() {
    let search = ToolCall {
        id: "call_gmail_search".to_string(),
        name: "gmail".to_string(),
        input: serde_json::json!({
            "action": "search",
            "query": "from:alice subject:invoice",
            "max_results": 5
        }),
        intent: None,
        thought_signature: None,
    };
    let summary = tools_ui::get_tool_summary_with_budget(&search, 50, Some(50));
    assert!(summary.starts_with("search "), "summary={summary:?}");
    assert!(summary.contains("from:alice"), "summary={summary:?}");

    let read = ToolCall {
        id: "call_gmail_read".to_string(),
        name: "gmail".to_string(),
        input: serde_json::json!({
            "action": "read",
            "message_id": "18f2ab34cd56ef78"
        }),
        intent: None,
        thought_signature: None,
    };
    let summary = tools_ui::get_tool_summary_with_budget(&read, 50, Some(50));
    assert!(summary.starts_with("read "), "summary={summary:?}");

    let send = ToolCall {
        id: "call_gmail_send".to_string(),
        name: "gmail".to_string(),
        input: serde_json::json!({
            "action": "send",
            "to": "bob@example.com",
            "subject": "hello"
        }),
        intent: None,
        thought_signature: None,
    };
    let summary = tools_ui::get_tool_summary_with_budget(&send, 50, Some(50));
    assert!(
        summary.contains("send") && summary.contains("bob@example.com"),
        "summary={summary:?}"
    );

    let bare = ToolCall {
        id: "call_gmail_labels".to_string(),
        name: "gmail".to_string(),
        input: serde_json::json!({ "action": "labels" }),
        intent: None,
        thought_signature: None,
    };
    let summary = tools_ui::get_tool_summary_with_budget(&bare, 50, Some(50));
    assert_eq!(summary, "labels");
}

#[test]
fn test_tool_activity_detail_prefixes_intent_for_gmail_and_browser() {
    tools_ui::tests_tool_call_details_override::set(true);
    let gmail = ToolCall {
        id: "call_gmail_intent".to_string(),
        name: "gmail".to_string(),
        input: serde_json::json!({
            "action": "search",
            "query": "is:unread",
            "intent": "Check unread mail"
        }),
        intent: Some("Check unread mail".to_string()),
        thought_signature: None,
    };
    let detail = tools_ui::get_tool_activity_detail(&gmail);
    assert!(detail.starts_with("Check unread mail"), "detail={detail:?}");
    assert!(detail.contains("is:unread"), "detail={detail:?}");

    let browser = ToolCall {
        id: "call_browser_intent".to_string(),
        name: "browser".to_string(),
        input: serde_json::json!({
            "action": "open",
            "url": "https://example.com",
            "intent": "Open docs page"
        }),
        intent: Some("Open docs page".to_string()),
        thought_signature: None,
    };
    let detail = tools_ui::get_tool_activity_detail(&browser);
    assert!(detail.starts_with("Open docs page"), "detail={detail:?}");
    assert!(detail.contains("example.com"), "detail={detail:?}");
    tools_ui::tests_tool_call_details_override::set(false);
}

/// By default (tool_call_details off) the activity detail is the intent alone.
#[test]
fn test_tool_activity_detail_hides_technical_summary_by_default() {
    let gmail = ToolCall {
        id: "call_gmail_intent_only".to_string(),
        name: "gmail".to_string(),
        input: serde_json::json!({
            "action": "search",
            "query": "is:unread",
            "intent": "Check unread mail"
        }),
        intent: Some("Check unread mail".to_string()),
        thought_signature: None,
    };
    let detail = tools_ui::get_tool_activity_detail(&gmail);
    assert_eq!(detail, "Check unread mail");
}

#[test]
fn test_tool_summary_covers_action_shaped_tools_and_fallback() {
    let cases: Vec<(&str, serde_json::Value, &str)> = vec![
        (
            "schedule",
            serde_json::json!({ "action": "create", "task": "check CI status" }),
            "create",
        ),
        (
            "schedule",
            serde_json::json!({ "action": "cancel", "schedule_id": "sched_123" }),
            "cancel",
        ),
        (
            "skill_manage",
            serde_json::json!({ "action": "load", "name": "frontend-design" }),
            "load /frontend-design",
        ),
        (
            "invalid",
            serde_json::json!({ "tool": "bash", "error": "missing command" }),
            "bash: missing command",
        ),
        (
            "integration_tools",
            serde_json::json!({ "category": "databases", "reason": "need a db" }),
            "search databases",
        ),
        (
            "integration_tools",
            serde_json::json!({
                "action": "suggest",
                "category": "payments",
                "suggestion_kind": "known_product",
                "product_name": "Stripe sandbox MCP"
            }),
            "suggest Stripe sandbox MCP",
        ),
        // Unknown/unmatched tools fall back to the action field.
        (
            "request_permission",
            serde_json::json!({ "action": "push", "description": "push commits" }),
            "push",
        ),
    ];
    for (name, input, expected_prefix) in cases {
        let tool = ToolCall {
            id: format!("call_{name}"),
            name: name.to_string(),
            input,
            intent: None,
            thought_signature: None,
        };
        let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(60));
        assert!(
            summary.starts_with(expected_prefix),
            "tool={name} summary={summary:?} expected prefix {expected_prefix:?}"
        );
    }
}

#[test]
fn test_tool_summary_read_supports_start_line_end_line() {
    let tool = ToolCall {
        id: "call_read_range".to_string(),
        name: "read".to_string(),
        input: serde_json::json!({
            "file_path": "src/tool/read.rs",
            "start_line": 10,
            "end_line": 20
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(40));
    assert!(summary.contains("read.rs:10-20"), "summary={summary:?}");
}

#[test]
fn test_render_tool_message_batch_includes_start_end_read_details() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] read ---\nok\n\nCompleted: 1 succeeded, 0 failed".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_range".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {"tool": "read", "file_path": "src/tool/read.rs", "start_line": 10, "end_line": 20}
                ]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 2, "rendered={rendered:?}");
    assert!(
        rendered[0].contains("✓ batch 1 call"),
        "rendered={rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("✓ read src/tool/read.rs:10-20")),
        "missing read subtool in {rendered:?}"
    );
}

#[test]
fn test_tool_summary_path_truncation_keeps_filename_tail() {
    let tool = ToolCall {
        id: "call_read_tail".to_string(),
        name: "read".to_string(),
        input: serde_json::json!({
            "file_path": "src/tui/really/long/nested/location/ui_messages.rs",
            "offset": 120,
            "limit": 40
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(28));

    assert!(summary.contains("ui_messages.rs"), "summary={summary:?}");
    assert!(summary.contains(":120-160"), "summary={summary:?}");
    assert!(summary.contains('…'), "summary={summary:?}");
    assert!(unicode_width::UnicodeWidthStr::width(summary.as_str()) <= 28);
}

#[test]
fn test_tool_summary_grep_truncation_prefers_middle() {
    let tool = ToolCall {
        id: "call_grep_middle".to_string(),
        name: "grep".to_string(),
        input: serde_json::json!({
            "pattern": "prefix_[A-Z0-9]+_important_middle_token_[a-z]+_suffix",
            "path": "src/some/really/long/module"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(34));

    assert!(
        summary.contains("importan") || summary.contains("token"),
        "summary={summary:?}"
    );
    assert!(
        summary.contains("suffix") || summary.contains("module"),
        "summary={summary:?}"
    );
    assert!(summary.contains('…'), "summary={summary:?}");
    assert!(unicode_width::UnicodeWidthStr::width(summary.as_str()) <= 34);
}

#[test]
fn test_tool_summary_bash_truncation_keeps_start_and_end() {
    let tool = ToolCall {
        id: "call_bash_middle".to_string(),
        name: "bash".to_string(),
        input: serde_json::json!({
            "command": "cargo test --package jcode --lib tui::ui::tests::render_tool_message_batch_flat_subcall_params_include_read_details -- --nocapture"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 32, Some(34));

    assert!(summary.starts_with("$ cargo"), "summary={summary:?}");
    assert!(
        summary.contains("nocapture") || summary.contains("read_details"),
        "summary={summary:?}"
    );
    assert!(summary.contains('…'), "summary={summary:?}");
    assert!(unicode_width::UnicodeWidthStr::width(summary.as_str()) <= 34);
}

#[test]
fn test_tool_summary_bash_keeps_full_command_when_width_fits() {
    let tool = ToolCall {
        id: "call_bash_full".to_string(),
        name: "bash".to_string(),
        input: serde_json::json!({
            "command": "cargo test --package jcode --lib tui::ui::tests::render_tool_message_batch_rows_do_not_soft_wrap_on_narrow_width -- --nocapture"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 32, Some(160));

    assert_eq!(
        summary,
        "$ cargo test --package jcode --lib tui::ui::tests::render_tool_message_batch_rows_do_not_soft_wrap_on_narrow_width -- --nocapture"
    );
    assert!(!summary.contains('…'), "summary={summary:?}");
}

#[test]
fn test_render_batch_subcall_line_keeps_full_bash_summary_when_row_fits() {
    let tool = ToolCall {
        id: "batch-1-bash".to_string(),
        name: "bash".to_string(),
        input: serde_json::json!({
            "command": "cargo test --package jcode --lib tui::ui::tests::render_tool_message_batch_rows_do_not_soft_wrap_on_narrow_width -- --nocapture"
        }),
        intent: None,
        thought_signature: None,
    };

    let line =
        tools_ui::render_batch_subcall_line(&tool, "✓", rgb(100, 180, 100), 32, Some(160), None);
    let rendered = extract_line_text(&line);

    assert!(
        rendered.contains("bash $ cargo test --package jcode"),
        "rendered={rendered:?}"
    );
    assert!(rendered.contains("-- --nocapture"), "rendered={rendered:?}");
    assert!(!rendered.contains('…'), "rendered={rendered:?}");
}

#[test]
fn test_render_batch_subcall_line_shows_model_provided_intent() {
    tools_ui::tests_tool_call_details_override::set(true);
    let tool = ToolCall {
        id: "batch-1-read".to_string(),
        name: "read".to_string(),
        input: serde_json::json!({"file_path": "src/tui/ui_messages.rs"}),
        intent: Some("Inspect completed batch rendering".to_string()),
        thought_signature: None,
    };

    let line =
        tools_ui::render_batch_subcall_line(&tool, "✓", rgb(100, 180, 100), 50, Some(120), None);
    let rendered = extract_line_text(&line);

    assert!(
        rendered.contains("read · Inspect completed batch rendering ·"),
        "rendered={rendered:?}"
    );
    assert!(rendered.contains("ui_messages.rs"), "rendered={rendered:?}");
    tools_ui::tests_tool_call_details_override::set(false);
}

/// By default (tool_call_details off) a subcall row with an intent shows only
/// the intent, not the dimmed technical summary.
#[test]
fn test_render_batch_subcall_line_hides_technical_detail_by_default() {
    let tool = ToolCall {
        id: "batch-1-read".to_string(),
        name: "read".to_string(),
        input: serde_json::json!({"file_path": "src/tui/ui_messages.rs"}),
        intent: Some("Inspect completed batch rendering".to_string()),
        thought_signature: None,
    };

    let line =
        tools_ui::render_batch_subcall_line(&tool, "✓", rgb(100, 180, 100), 50, Some(120), None);
    let rendered = extract_line_text(&line);

    assert!(
        rendered.contains("read · Inspect completed batch rendering"),
        "rendered={rendered:?}"
    );
    assert!(
        !rendered.contains("ui_messages.rs"),
        "technical detail should be hidden by default: {rendered:?}"
    );
}

#[test]
fn test_agentgrep_summary_uses_default_grep_mode_query() {
    let tool = ToolCall {
        id: "agentgrep-default-mode".to_string(),
        name: "agentgrep".to_string(),
        input: serde_json::json!({
            "query": "pending_soft_interrupt",
            "path": "src/tui"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(120));

    assert_eq!(summary, "grep 'pending_soft_interrupt'");
}

#[test]
fn test_render_batch_subcall_line_shows_first_subcall_token_badge() {
    let tool = ToolCall {
        id: "agentgrep-default-mode".to_string(),
        name: "agentgrep".to_string(),
        input: serde_json::json!({
            "query": "pending_soft_interrupt",
            "path": "src/tui"
        }),
        intent: None,
        thought_signature: None,
    };

    let line = tools_ui::render_batch_subcall_line(
        &tool,
        "✓",
        rgb(100, 180, 100),
        50,
        Some(120),
        Some("query: pending_soft_interrupt\nmatches: 1 in 1 files\n"),
    );
    let rendered = extract_line_text(&line);

    assert!(
        rendered.contains("agentgrep grep 'pending_soft_interrupt'"),
        "rendered={rendered:?}"
    );
    assert!(rendered.contains("tok"), "rendered={rendered:?}");
}

include!("tools_partition_01_tests.rs");
