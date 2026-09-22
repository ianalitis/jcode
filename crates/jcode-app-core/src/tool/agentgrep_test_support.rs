//! Helpers shared by the agentgrep test modules.
//!
//! `test_ctx` lives here rather than in one of the test files so both can use it
//! without either of them duplicating it or passing the test-size ratchet.

use super::*;

pub(super) fn test_ctx(root: &Path) -> ToolContext {
    ToolContext {
        session_id: "test".to_string(),
        message_id: "test".to_string(),
        tool_call_id: "test".to_string(),
        working_dir: Some(root.to_path_buf()),
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: super::super::ToolExecutionMode::Direct,
    }
}
