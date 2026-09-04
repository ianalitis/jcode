//! Regression coverage for the 400 "unexpected `tool_use_id` found in
//! `tool_result` blocks" rejection.
//!
//! Production shape (session_clover_1785560899476): a long-running `bash` tool
//! was still executing when a scheduled-task wakeup drove a new turn. The
//! missing tool-output repair saw an assistant `tool_use` with no result yet
//! and inserted a synthetic placeholder result; the real tool output landed
//! ~28s later and was appended as a second `tool_result` for the same id.
//! After same-role merging, the duplicate sits in a user message whose
//! preceding assistant turn no longer contains that `tool_use`, so Anthropic
//! rejects every subsequent request and the session is permanently wedged.

use super::*;
use jcode_message_types::{ContentBlock, Message, Role};

fn text_msg(role: Role, text: &str) -> Message {
    Message {
        role,
        content: vec![ContentBlock::Text {
            text: text.to_string(),
            cache_control: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }
}

fn tool_use(id: &str) -> Message {
    Message {
        role: Role::Assistant,
        content: vec![ContentBlock::ToolUse {
            id: id.to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
            thought_signature: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }
}

fn tool_result(id: &str, content: &str, is_error: Option<bool>) -> Message {
    Message {
        role: Role::User,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: id.to_string(),
            content: content.to_string(),
            is_error,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }
}

/// Every tool_use_id may appear at most once across all tool_result blocks.
fn assert_unique_tool_results(messages: &[ApiMessage]) {
    let mut seen = std::collections::HashSet::new();
    for msg in messages {
        for block in &msg.content {
            if let ApiContentBlock::ToolResult { tool_use_id, .. } = block {
                assert!(
                    seen.insert(tool_use_id.clone()),
                    "duplicate tool_result for {tool_use_id}"
                );
            }
        }
    }
}

fn text_blocks(messages: &[ApiMessage]) -> Vec<&str> {
    messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            ApiContentBlock::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn placeholder_then_real_output_keeps_only_real_as_typed_result() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        tool_result("toolu_1", TOOL_OUTPUT_MISSING_TEXT, Some(true)),
        text_msg(Role::User, "[Scheduled task] wakeup"),
        tool_result("toolu_1", "real output", None),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);

    let kept: Vec<&str> = formatted
        .iter()
        .flat_map(|m| &m.content)
        .filter_map(|b| match b {
            ApiContentBlock::ToolResult {
                content: ToolResultContent::Text(text),
                ..
            } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(kept, vec!["real output"], "real output must win");

    let text = text_blocks(&formatted).join("\n");
    assert!(
        text.contains(TOOL_OUTPUT_MISSING_TEXT),
        "the displaced placeholder must remain visible as ordinary text"
    );
    assert!(text.contains("[Scheduled task] wakeup"));
}

#[test]
fn merged_user_turn_places_tool_results_before_text() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        text_msg(Role::User, "[Scheduled task] wakeup"),
        tool_result("toolu_1", "real output", None),
    ];

    let formatted = format_messages(&messages, false);
    let result_turn = formatted.get(2).expect("tool result turn");
    assert!(
        matches!(
            result_turn.content.first(),
            Some(ApiContentBlock::ToolResult { .. })
        ),
        "tool_result blocks must lead the user turn after same-role merging"
    );
}

#[test]
fn real_output_then_placeholder_keeps_the_real_output() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        tool_result("toolu_1", "real output", None),
        tool_result("toolu_1", TOOL_OUTPUT_MISSING_TEXT, Some(true)),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);
    assert!(
        formatted.iter().flat_map(|m| &m.content).any(|b| matches!(
            b,
            ApiContentBlock::ToolResult { content: ToolResultContent::Text(t), .. } if t == "real output"
        ))
    );
}

#[test]
fn duplicate_real_outputs_keep_the_first() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        tool_result("toolu_1", "first", None),
        tool_result("toolu_1", "second", None),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);
    assert!(formatted.iter().flat_map(|m| &m.content).any(|b| matches!(
        b,
        ApiContentBlock::ToolResult { content: ToolResultContent::Text(t), .. } if t == "first"
    )));
    assert!(
        text_blocks(&formatted)
            .iter()
            .any(|text| text.contains("second"))
    );
}

#[test]
fn sanitized_id_collisions_keep_one_typed_result() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu.1"),
        tool_result("toolu.1", "first", None),
        tool_result("toolu:1", "second", None),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);
    assert!(
        text_blocks(&formatted)
            .iter()
            .any(|text| text.contains("second")),
        "the colliding result must remain visible as text"
    );
}

#[test]
fn duplicate_text_is_not_folded_into_an_image_tool_result() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        Message {
            role: Role::User,
            content: vec![
                ContentBlock::ToolResult {
                    tool_use_id: "toolu_1".to_string(),
                    content: "real output".to_string(),
                    is_error: None,
                },
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "AA==".to_string(),
                },
                ContentBlock::ToolResult {
                    tool_use_id: "toolu_1".to_string(),
                    content: "duplicate output".to_string(),
                    is_error: None,
                },
            ],
            timestamp: None,
            tool_duration_ms: None,
        },
    ];

    let formatted = format_messages(&messages, false);
    assert!(
        text_blocks(&formatted)
            .iter()
            .any(|text| text.contains("duplicate output")),
        "normalization text must remain a top-level user block"
    );
}

#[test]
fn ordinary_duplicate_prefix_text_stays_after_parallel_results() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        Message {
            role: Role::Assistant,
            content: vec![
                ContentBlock::ToolUse {
                    id: "toolu_1".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({}),
                    thought_signature: None,
                },
                ContentBlock::ToolUse {
                    id: "toolu_2".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({}),
                    thought_signature: None,
                },
            ],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::User,
            content: vec![
                ContentBlock::ToolResult {
                    tool_use_id: "toolu_1".to_string(),
                    content: "first".to_string(),
                    is_error: None,
                },
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "AA==".to_string(),
                },
                ContentBlock::Text {
                    text: "[Duplicate tool result for ordinary user text".to_string(),
                    cache_control: None,
                },
                ContentBlock::ToolResult {
                    tool_use_id: "toolu_2".to_string(),
                    content: "second".to_string(),
                    is_error: None,
                },
            ],
            timestamp: None,
            tool_duration_ms: None,
        },
    ];

    let formatted = format_messages(&messages, false);
    let result_turn = formatted.get(2).expect("parallel result turn");
    assert!(matches!(
        result_turn.content.as_slice(),
        [ApiContentBlock::ToolResult { .. }, ApiContentBlock::ToolResult { .. }, ApiContentBlock::Text { text, .. }]
            if text == "[Duplicate tool result for ordinary user text"
    ));
}

#[test]
fn distinct_tool_ids_are_untouched() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        tool_result("toolu_1", "a", None),
        tool_use("toolu_2"),
        tool_result("toolu_2", "b", None),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);
    let count = formatted
        .iter()
        .flat_map(|m| &m.content)
        .filter(|b| matches!(b, ApiContentBlock::ToolResult { .. }))
        .count();
    assert_eq!(count, 2);
}

#[test]
fn message_with_duplicate_result_keeps_content_and_roles_valid() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        tool_result("toolu_1", "real", None),
        tool_result("toolu_1", TOOL_OUTPUT_MISSING_TEXT, Some(true)),
        text_msg(Role::Assistant, "done"),
        text_msg(Role::User, "next"),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);
    assert!(formatted.iter().all(|m| !m.content.is_empty()));
    let roles: Vec<&str> = formatted.iter().map(|m| m.role.as_str()).collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "user", "assistant", "user"]
    );
    assert!(
        text_blocks(&formatted)
            .iter()
            .any(|text| text.contains(TOOL_OUTPUT_MISSING_TEXT)),
        "normalization must not delete the duplicate block's content"
    );
}

#[test]
fn synthetic_interrupt_placeholder_text_is_also_treated_as_a_placeholder() {
    let messages = vec![
        text_msg(Role::User, "Q"),
        tool_use("toolu_1"),
        tool_result(
            "toolu_1",
            "[Session interrupted before tool execution completed]",
            Some(true),
        ),
        tool_result("toolu_1", "real output", None),
    ];

    let formatted = format_messages(&messages, false);
    assert_unique_tool_results(&formatted);
    assert!(
        formatted.iter().flat_map(|m| &m.content).any(|b| matches!(
            b,
            ApiContentBlock::ToolResult { content: ToolResultContent::Text(t), .. } if t == "real output"
        ))
    );
}
