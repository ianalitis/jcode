//! `self_compact` and `view_context`: agent-callable context management tools.
//!
//! `self_compact({note_to_self})` validates and stores a note byte-for-byte in a
//! process-global session-keyed map, then asks the agent to start manual
//! compaction. `Agent::request_self_compaction` (agent/compaction.rs) drains the
//! map and keeps the note pending; when compaction completes, the note is
//! appended to the session verbatim as a user-role message prefixed by exactly
//! one line `[self_compact note_to_self]`.
//!
//! `view_context()` reports live context usage as JSON using the session's
//! compaction manager (`stats_with` over the persisted session messages).

use super::{Tool, ToolContext, ToolOutput, WeakRegistry};
use crate::session::Session;
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Maximum accepted `note_to_self` size in bytes.
pub const SELF_COMPACT_NOTE_MAX_BYTES: usize = 24_000;

/// Exactly one line prefixed to the verbatim note when it is appended to the
/// session after compaction completes.
pub const SELF_COMPACT_NOTE_PREFIX: &str = "[self_compact note_to_self]";

/// Prefix identifying a pending self-compact request in the session-keyed map.
pub const SELF_COMPACT_REQUEST_MARKER: &str = "SELF_COMPACT_REQUESTED";

/// Session-keyed pending notes, drained by the owning agent.
///
/// Tools run without access to agent state, so this map is the seam: the tool
/// inserts under `ctx.session_id`, and `Agent::request_self_compaction` (which
/// knows its own session id) drains it. Byte-for-byte storage is the contract.
fn pending_self_compact_notes() -> &'static Mutex<HashMap<String, String>> {
    static NOTES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    NOTES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Store a note for `session_id`, byte-for-byte. Any previous pending note for
/// the session is replaced (an agent only ever has one handoff in flight).
pub fn store_pending_self_compact_note(session_id: &str, note: &str) {
    pending_self_compact_notes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(session_id.to_string(), note.to_string());
}

/// Take the pending note for `session_id`, if any.
#[cfg_attr(not(test), allow(dead_code))]
pub fn take_pending_self_compact_note(session_id: &str) -> Option<String> {
    pending_self_compact_notes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(session_id)
}

#[derive(Debug, Deserialize)]
struct SelfCompactInput {
    note_to_self: String,
}

pub struct SelfCompactTool;

impl Default for SelfCompactTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfCompactTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SelfCompactTool {
    fn name(&self) -> &str {
        "self_compact"
    }

    fn description(&self) -> &str {
        "Save a verbatim note to yourself and compact the conversation now."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intent": super::intent_schema_property(),
                "note_to_self": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": SELF_COMPACT_NOTE_MAX_BYTES,
                    "description": "Returned to you verbatim after compaction. Max 24000 bytes."
                }
            },
            "required": ["note_to_self"]
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let params: SelfCompactInput = serde_json::from_value(input)?;

        if params.note_to_self.trim().is_empty() {
            anyhow::bail!(
                "self_compact: note_to_self must not be empty or whitespace-only (limit {} bytes)",
                SELF_COMPACT_NOTE_MAX_BYTES
            );
        }
        if params.note_to_self.len() > SELF_COMPACT_NOTE_MAX_BYTES {
            anyhow::bail!(
                "self_compact: note_to_self is {} bytes, exceeding the {} byte limit",
                params.note_to_self.len(),
                SELF_COMPACT_NOTE_MAX_BYTES
            );
        }

        // Byte-for-byte storage is the contract; trim never touches the note.
        store_pending_self_compact_note(&ctx.session_id, &params.note_to_self);

        // From here the note is durable in this process. Compaction may still
        // fail to start (lock held, usage too low, provider without compaction);
        // the pending note survives so the agent can retry, and the round trip
        // happens on the eventual successful compaction.
        Ok(ToolOutput::new(format!(
            "{}\n{}\n\n{}",
            SELF_COMPACT_REQUEST_MARKER, params.note_to_self, SELF_COMPACT_REQUEST_MARKER
        )))
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ViewContextInput {}

pub struct ViewContextTool {
    registry: WeakRegistry,
}

impl ViewContextTool {
    pub(super) fn new(registry: WeakRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for ViewContextTool {
    fn name(&self) -> &str {
        "view_context"
    }

    fn description(&self) -> &str {
        "Report context-window usage as JSON (tokens, percent, compaction state)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intent": super::intent_schema_property()
            }
        })
    }

    async fn execute(&self, _input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let registry = self
            .registry
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("view_context: tool registry is gone"))?;

        let session_messages: Option<Vec<crate::message::Message>> =
            match Session::load(&ctx.session_id) {
                Ok(session) => Some(
                    session
                        .messages
                        .iter()
                        .map(|message| message.to_message())
                        .collect(),
                ),
                Err(err) => {
                    crate::logging::warn(&format!(
                        "[tool:view_context] failed to load session history for session {}: {}",
                        ctx.session_id, err
                    ));
                    None
                }
            };

        let compaction = registry.compaction();
        let (used_tokens, used_percent, has_summary, is_compacting, active_messages) = {
            let manager = compaction.read().await;
            match session_messages.as_deref() {
                Some(messages) => {
                    let stats = manager.stats_with(messages);
                    (
                        stats.effective_tokens,
                        stats.context_usage * 100.0,
                        stats.has_summary,
                        stats.is_compacting,
                        stats.active_messages,
                    )
                }
                None => {
                    let stats = manager.stats();
                    (
                        stats.effective_tokens,
                        stats.context_usage * 100.0,
                        stats.has_summary,
                        stats.is_compacting,
                        stats.active_messages,
                    )
                }
            }
        };

        let compaction_mode = compaction.read().await.mode().as_str().to_string();

        let budget_tokens = compaction.read().await.token_budget();

        let payload = json!({
            "used_tokens": used_tokens,
            "budget_tokens": budget_tokens,
            "used_percent": used_percent,
            "has_summary": has_summary,
            "is_compacting": is_compacting,
            "compaction_mode": compaction_mode,
            "active_messages": active_messages,
        });
        Ok(ToolOutput::new(payload.to_string()).with_title("Context usage"))
    }
}

#[cfg(test)]
mod tests {
    include!("self_compact_tests.rs");
}
