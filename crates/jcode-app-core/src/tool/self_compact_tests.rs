// Tests for the self_compact and view_context tools.
// Included as `mod tests` from self_compact.rs.

use super::*;
use crate::compaction::CompactionManager;
use std::sync::Arc;
use tokio::sync::RwLock;

fn tool_ctx(session_id: &str) -> ToolContext {
    ToolContext {
        session_id: session_id.to_string(),
        message_id: "test-message".to_string(),
        tool_call_id: "test-tool-call".to_string(),
        working_dir: None,
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: crate::tool::ToolExecutionMode::Direct,
    }
}

#[test]
fn self_compact_rejects_blank_note() {
    let tool = SelfCompactTool::new();
    for note in ["", "   ", "\n\t \n"] {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(tool.execute(
                serde_json::json!({ "note_to_self": note }),
                tool_ctx("blank-note-session"),
            ));
        let err = format!("{}", result.expect_err("blank note must be rejected"));
        assert!(
            err.contains("24000") && err.contains("empty or whitespace"),
            "error must name the limit and the blank rule, got: {err}"
        );
    }
    assert!(take_pending_self_compact_note("blank-note-session").is_none());
}

#[test]
fn self_compact_rejects_note_over_limit() {
    let tool = SelfCompactTool::new();
    let note = "x".repeat(SELF_COMPACT_NOTE_MAX_BYTES + 1);
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(tool.execute(
            serde_json::json!({ "note_to_self": note }),
            tool_ctx("oversize-note-session"),
        ));
    let err = format!("{}", result.expect_err("oversize note must be rejected"));
    assert!(
        err.contains("24001") && err.contains("24000"),
        "error must name the actual and allowed sizes, got: {err}"
    );
    assert!(take_pending_self_compact_note("oversize-note-session").is_none());
}

#[test]
fn self_compact_accepts_note_at_limit_byte_for_byte() {
    let tool = SelfCompactTool::new();
    // Multibyte characters so byte length != char length, at exactly the cap.
    let note = "é".repeat(SELF_COMPACT_NOTE_MAX_BYTES / 2);
    debug_assert_eq!(note.len(), SELF_COMPACT_NOTE_MAX_BYTES);
    // No trailing filler: any more bytes would push the note past the cap.
    let expected = note.clone();
    let output = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(tool.execute(
            serde_json::json!({ "note_to_self": note }),
            tool_ctx("limit-note-session"),
        ))
        .expect("note at the byte limit must be accepted");
    let stored = take_pending_self_compact_note("limit-note-session")
        .expect("accepted note must be stored for the agent");
    assert_eq!(stored, expected, "stored note must be byte-for-byte");
    assert_eq!(stored.len(), SELF_COMPACT_NOTE_MAX_BYTES);
    assert!(
        output.output.starts_with(SELF_COMPACT_REQUEST_MARKER),
        "tool output must start with the request marker"
    );
    assert!(output.output.contains(&expected));
}

#[tokio::test]
async fn view_context_output_parses_with_required_keys() {
    let _guard = crate::storage::lock_test_env();
    let manager = Arc::new(RwLock::new(CompactionManager::new()));
    // Keep the strong Arcs alive for the whole call so the WeakRegistry can
    // upgrade (mirrors a live Registry owned by an Agent).
    let holder = WeakRegistryForTest::new(manager);
    let tool = ViewContextTool::new(holder.downgrade());
    let output = tool
        .execute(serde_json::json!({}), tool_ctx("view-context-session"))
        .await
        .expect("view_context must succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&output.output).expect("output must be valid JSON");
    for key in [
        "used_tokens",
        "budget_tokens",
        "used_percent",
        "has_summary",
        "is_compacting",
        "compaction_mode",
    ] {
        assert!(parsed.get(key).is_some(), "missing key {key}: {parsed}");
    }
    assert!(parsed["budget_tokens"].is_u64());
    // The mode string mirrors config ([compaction] mode); assert the shape,
    // not the operator's configured value.
    assert!(matches!(
        parsed["compaction_mode"].as_str(),
        Some("reactive") | Some("proactive") | Some("semantic")
    ));
    assert_eq!(parsed["is_compacting"], false);
}

/// Minimal registry stand-in so `view_context` can be exercised without
/// building a full tool registry. Holds the strong Arcs a real Registry holds.
struct WeakRegistryForTest {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
    mcp_policy: Arc<std::sync::RwLock<crate::tool::McpPolicyIndex>>,
    skills: Arc<RwLock<crate::skill::SkillRegistry>>,
    compaction: Arc<RwLock<CompactionManager>>,
}

impl WeakRegistryForTest {
    fn new(compaction: Arc<RwLock<CompactionManager>>) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            mcp_policy: Arc::new(std::sync::RwLock::new(
                crate::tool::McpPolicyIndex::default(),
            )),
            skills: Arc::new(RwLock::new(crate::skill::SkillRegistry::default())),
            compaction,
        }
    }

    fn downgrade(&self) -> WeakRegistry {
        WeakRegistry {
            tools: Arc::downgrade(&self.tools),
            mcp_policy: Arc::clone(&self.mcp_policy),
            skills: Arc::clone(&self.skills),
            compaction: Arc::clone(&self.compaction),
        }
    }
}
