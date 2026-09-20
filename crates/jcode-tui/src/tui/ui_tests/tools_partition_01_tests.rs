#[test]
fn test_common_tool_summaries_keep_full_text_when_row_budget_fits() {
    let cases = vec![
        (
            ToolCall {
                id: "read-wide".to_string(),
                name: "read".to_string(),
                input: serde_json::json!({
                    "file_path": "src/tui/ui_messages.rs",
                    "offset": 120,
                    "limit": 40
                }),
                intent: None,
                thought_signature: None,
            },
            "src/tui/ui_messages.rs:120-160",
        ),
        (
            ToolCall {
                id: "grep-wide".to_string(),
                name: "grep".to_string(),
                input: serde_json::json!({
                    "pattern": "render_batch_subcall_line",
                    "path": "src/tui"
                }),
                intent: None,
                thought_signature: None,
            },
            "'render_batch_subcall_line' in src/tui",
        ),
        (
            ToolCall {
                id: "glob-wide".to_string(),
                name: "glob".to_string(),
                input: serde_json::json!({
                    "pattern": "src/tui/**/*.rs"
                }),
                intent: None,
                thought_signature: None,
            },
            "'src/tui/**/*.rs'",
        ),
        (
            ToolCall {
                id: "webfetch-wide".to_string(),
                name: "webfetch".to_string(),
                input: serde_json::json!({
                    "url": "https://example.com/docs/api/reference"
                }),
                intent: None,
                thought_signature: None,
            },
            "https://example.com/docs/api/reference",
        ),
        (
            ToolCall {
                id: "open-wide".to_string(),
                name: "open".to_string(),
                input: serde_json::json!({
                    "action": "open",
                    "target": "src/tui/ui.rs"
                }),
                intent: None,
                thought_signature: None,
            },
            "open src/tui/ui.rs",
        ),
        (
            ToolCall {
                id: "memory-wide".to_string(),
                name: "memory".to_string(),
                input: serde_json::json!({
                    "action": "recall",
                    "query": "tool summary truncation"
                }),
                intent: None,
                thought_signature: None,
            },
            "recall 'tool summary truncation'",
        ),
        (
            ToolCall {
                id: "codesearch-wide".to_string(),
                name: "codesearch".to_string(),
                input: serde_json::json!({
                    "query": "rust unicode width truncation examples"
                }),
                intent: None,
                thought_signature: None,
            },
            "'rust unicode width truncation examples'",
        ),
        (
            ToolCall {
                id: "debug-wide".to_string(),
                name: "debug_socket".to_string(),
                input: serde_json::json!({
                    "command": "tester:list"
                }),
                intent: None,
                thought_signature: None,
            },
            "tester:list",
        ),
    ];

    for (tool, expected) in cases {
        let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
        assert_eq!(summary, expected, "tool={tool:?} summary={summary:?}");
        assert!(!summary.contains('…'), "tool={tool:?} summary={summary:?}");
    }
}

#[test]
fn test_debug_socket_summary_hides_transient_missing_input() {
    let tool = ToolCall {
        id: "debug-start".to_string(),
        name: "debug_socket".to_string(),
        input: serde_json::Value::Null,
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(summary, "");
}

#[test]
fn test_tool_summary_browser_open_shows_url() {
    let tool = ToolCall {
        id: "browser-open".to_string(),
        name: "browser".to_string(),
        input: serde_json::json!({
            "action": "open",
            "url": "https://example.com/docs/reference/browser-tool"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(
        summary,
        "open https://example.com/docs/reference/browser-tool"
    );
}

#[test]
fn test_tool_summary_browser_type_hides_typed_text() {
    let tool = ToolCall {
        id: "browser-type".to_string(),
        name: "browser".to_string(),
        input: serde_json::json!({
            "action": "type",
            "selector": "#password",
            "text": "super-secret-value"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(summary, "type #password (18 chars)");
    assert!(
        !summary.contains("super-secret-value"),
        "summary={summary:?}"
    );
}

#[test]
fn test_tool_summary_browser_type_without_selector_still_hides_text() {
    let tool = ToolCall {
        id: "browser-type-no-selector".to_string(),
        name: "browser".to_string(),
        input: serde_json::json!({
            "action": "type",
            "text": "secret-token-123"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(summary, "type (16 chars)");
    assert!(!summary.contains("secret-token-123"), "summary={summary:?}");
}

#[test]
fn test_tool_summary_browser_eval_truncates_script() {
    let tool = ToolCall {
        id: "browser-eval".to_string(),
        name: "browser".to_string(),
        input: serde_json::json!({
            "action": "eval",
            "script": "return window.__APP_STATE__?.reallyLongNestedValue?.items?.map(item => item.name).join(', ')"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(34));
    assert!(summary.starts_with("eval "), "summary={summary:?}");
    assert!(summary.contains('…'), "summary={summary:?}");
    assert!(unicode_width::UnicodeWidthStr::width(summary.as_str()) <= 34);
}

#[test]
fn test_tool_summary_agentgrep_smart_uses_terms_subject_relation() {
    let tool = ToolCall {
        id: "agentgrep-smart-terms".to_string(),
        name: "agentgrep".to_string(),
        input: serde_json::json!({
            "mode": "smart",
            "terms": ["subject:agentgrep", "relation:build_args", "path:src/tool"]
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(summary, "smart agentgrep:build_args");
}

#[test]
fn test_tool_summary_agentgrep_smart_uses_query_subject_relation() {
    let tool = ToolCall {
        id: "agentgrep-smart-query".to_string(),
        name: "agentgrep".to_string(),
        input: serde_json::json!({
            "mode": "smart",
            "query": "subject:agentgrep relation:build_args path:src/tool"
        }),
        intent: None,
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(summary, "smart agentgrep:build_args");
}

#[test]
fn test_tool_summary_bg_infers_wait_from_intent_when_action_missing() {
    let tool = ToolCall {
        id: "bg-intent-only".to_string(),
        name: "bg".to_string(),
        input: serde_json::json!({
            "intent": "Wait for library tests",
            "latest": true
        }),
        intent: Some("Wait for library tests".to_string()),
        thought_signature: None,
    };

    let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
    assert_eq!(summary, "wait");
}

#[test]
fn test_render_tool_message_batch_rows_do_not_soft_wrap_on_narrow_width() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] read ---\nok\n\nCompleted: 1 succeeded, 0 failed".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_batch_narrow".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {
                        "tool": "read",
                        "file_path": "src/tui/really/long/nested/location/ui_messages.rs",
                        "offset": 120,
                        "limit": 40
                    }
                ]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 32, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert_eq!(rendered.len(), 2, "rendered={rendered:?}");
    assert!(
        rendered.iter().all(|line| line.width() <= 31),
        "rendered={rendered:?}"
    );
    assert!(rendered[1].contains('…'), "rendered={rendered:?}");
    assert!(rendered[1].contains("tok"), "rendered={rendered:?}");
}

#[test]
fn test_render_tool_message_keeps_token_badge_when_intent_is_truncated() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "ok".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_long_intent".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "cargo test --package jcode --lib tui::ui::tests::very_long_test_name -- --nocapture"
            }),
            intent: Some(
                "Inspect and validate the extremely long wrapping behavior for tool rows"
                    .to_string(),
            ),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 48, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert!(!rendered.is_empty(), "rendered={rendered:?}");
    assert!(rendered[0].width() <= 47, "rendered={rendered:?}");
    assert!(rendered[0].contains('…'), "rendered={rendered:?}");
    assert!(rendered[0].contains("tok"), "rendered={rendered:?}");
}

/// With an intent present, the bash command preview must never spill onto a
/// second `$ ...` line. It renders inline when it fits and is dropped when it
/// does not.
#[test]
fn test_render_tool_message_with_intent_never_adds_second_command_line() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "ok".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_intent_no_wrap".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "set -euo pipefail; python -c 'import modal' && echo ready"
            }),
            intent: Some("Launch exactly one paid Opus canary".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 60, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert!(!rendered.is_empty(), "rendered={rendered:?}");
    assert_eq!(
        rendered.len(),
        1,
        "Bash output is hidden by default: {rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .all(|line| !line.trim_start().starts_with('$')),
        "rendered={rendered:?}"
    );
}

#[test]
fn test_render_tool_message_keeps_bash_command_visible_when_row_is_narrow() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "2\n".to_string(),
        tool_calls: vec![],
        duration_secs: None,
        title: None,
        tool_data: Some(ToolCall {
            id: "call_narrow_bash".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "grep -rn \"unwrap()\" src/ --include=\"*.rs\" | wc -l"
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 18, crate::config::DiffDisplayMode::Off);
    let rendered: Vec<String> = lines.iter().map(extract_line_text).collect();

    assert!(
        rendered.iter().any(|line| line.contains("bash")),
        "rendered={rendered:?}"
    );
    assert!(
        rendered.iter().any(|line| line.contains('$')),
        "narrow bash tool rows should include a command preview: {rendered:?}"
    );
}

/// Regression for https://github.com/1jehuang/jcode/issues/284:
/// While a tool call is still streaming, its arguments arrive separately and
/// `input` is `null` (or an empty object) for many render frames. The summary
/// must not show "action missing" / "command missing" placeholders in that
/// window; it should be empty so only the tool name renders.
#[test]
fn test_action_tools_hide_missing_placeholder_for_streaming_input() {
    let action_tools = [
        "bg",
        "swarm",
        "initiative",
        "selfdev",
        "side_panel",
        "memory",
    ];
    let transient_inputs = [serde_json::Value::Null, serde_json::json!({})];

    for name in action_tools {
        for input in &transient_inputs {
            let tool = ToolCall {
                id: format!("{name}-streaming"),
                name: name.to_string(),
                input: input.clone(),
                intent: None,
                thought_signature: None,
            };

            let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
            assert!(
                !summary.contains("missing"),
                "tool={name} input={input} summary={summary:?}"
            );
            assert_eq!(
                summary, "",
                "transient streaming input should yield an empty summary: tool={name} input={input}"
            );
        }
    }
}

/// Even when a tool call carries a populated, valid input object, a missing
/// `action` field must degrade to the tool name rather than the alarming
/// "action missing" placeholder.
#[test]
fn test_action_tools_degrade_to_tool_name_when_action_absent() {
    let cases = [
        ("bg", serde_json::json!({ "task_id": "abc" })),
        ("swarm", serde_json::json!({ "to_session": "worker-1" })),
        ("initiative", serde_json::json!({ "id": "plan-1" })),
        ("memory", serde_json::json!({ "query": "notes" })),
    ];

    for (name, input) in cases {
        let tool = ToolCall {
            id: format!("{name}-no-action"),
            name: name.to_string(),
            input,
            intent: None,
            thought_signature: None,
        };

        let summary = tools_ui::get_tool_summary_with_budget(&tool, 50, Some(200));
        assert!(
            !summary.contains("missing"),
            "tool={name} summary={summary:?}"
        );
    }
}

/// The live activity line should surface the model-provided `intent` for any
/// tool (including swarm) ahead of the technical summary when tool call
/// details are enabled.
#[test]
fn test_activity_detail_prefers_intent_and_appends_summary() {
    tools_ui::tests_tool_call_details_override::set(true);
    let tool = ToolCall {
        id: "swarm-1".to_string(),
        name: "swarm".to_string(),
        input: serde_json::json!({
            "intent": "Spin up a worker for the parser fix",
            "action": "spawn",
            "prompt": "Fix the parser bug in crates/parser"
        }),
        intent: Some("Spin up a worker for the parser fix".to_string()),
        thought_signature: None,
    };

    let detail = tools_ui::get_tool_activity_detail(&tool);
    assert!(
        detail.starts_with("Spin up a worker for the parser fix"),
        "intent should lead the activity detail: {detail:?}"
    );
    assert!(
        detail.contains("spawn"),
        "technical summary should still appear: {detail:?}"
    );
    tools_ui::tests_tool_call_details_override::set(false);
}

/// When the `ToolCall.intent` field is not populated yet (e.g. streamed input
/// parsed but intent refresh missed), fall back to the raw `intent` input key.
#[test]
fn test_activity_detail_falls_back_to_input_intent_field() {
    let tool = ToolCall {
        id: "swarm-2".to_string(),
        name: "swarm".to_string(),
        input: serde_json::json!({
            "intent": "Check on worker progress",
            "action": "status",
            "target_session": "worker-1"
        }),
        intent: None,
        thought_signature: None,
    };

    let detail = tools_ui::get_tool_activity_detail(&tool);
    assert!(
        detail.starts_with("Check on worker progress"),
        "input intent should be used when the field is unset: {detail:?}"
    );
}

/// Without an intent, the activity detail matches the plain technical summary.
#[test]
fn test_activity_detail_without_intent_matches_summary() {
    let tool = ToolCall {
        id: "swarm-3".to_string(),
        name: "swarm".to_string(),
        input: serde_json::json!({ "action": "dm", "to_session": "worker-1", "message": "hello" }),
        intent: None,
        thought_signature: None,
    };

    let detail = tools_ui::get_tool_activity_detail(&tool);
    let summary = tools_ui::get_tool_summary(&tool);
    assert_eq!(detail, summary);
    assert!(!detail.is_empty());
}
