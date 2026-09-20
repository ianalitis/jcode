//! Inert compatibility API for a fork without optional data collection.
//!
//! There is no collector, identity store, consent store, queue, HTTP client,
//! background thread, or upload endpoint in this crate. Settings and environment
//! variables cannot enable collection. Existing callers may keep reporting local
//! operational events here; all recording functions discard their arguments.
//! Local session persistence and provider requests belong to other subsystems.

pub use jcode_usage_types::{ErrorCategory, SessionEndReason};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryOptOutSource {
    BuildPolicy,
}

impl TelemetryOptOutSource {
    pub fn as_str(self) -> &'static str {
        "build_policy"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryStatus {
    pub enabled: bool,
    pub content_sharing_enabled: bool,
    pub opt_out_source: Option<TelemetryOptOutSource>,
    pub telemetry_id: Option<String>,
}

pub fn status() -> TelemetryStatus {
    TelemetryStatus {
        enabled: false,
        content_sharing_enabled: false,
        opt_out_source: opt_out_source(),
        telemetry_id: None,
    }
}

pub fn opt_out_source() -> Option<TelemetryOptOutSource> {
    Some(TelemetryOptOutSource::BuildPolicy)
}

pub fn is_enabled() -> bool {
    false
}
pub fn content_sharing_enabled() -> bool {
    false
}

/// Collection is unavailable regardless of the environment.
pub fn opt_out_forced_by_env() -> bool {
    false
}

/// Disabling is idempotent; enabling is rejected without touching existing files.
pub fn set_usage_telemetry_enabled(enabled: bool) -> bool {
    !enabled
}
pub fn set_content_sharing_enabled(enabled: bool) -> bool {
    !enabled
}

/// An inert ownership token. The caller-provided session ID is not a tracking ID
/// and is never persisted, correlated, or uploaded by this API.
#[derive(Debug)]
pub struct ConcurrencySession {
    session_id: String,
}

pub fn begin_concurrency_session(
    session_id: &str,
    _parent_session_id: Option<&str>,
) -> ConcurrencySession {
    ConcurrencySession {
        session_id: session_id.to_owned(),
    }
}

impl ConcurrencySession {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn is_active(&self) -> bool {
        false
    }
    pub fn finish(&mut self) {}
}

#[derive(Debug, Clone)]
pub struct DiscoveryTelemetry<'a> {
    pub request_id: &'a str,
    pub phase: &'a str,
    pub category: Option<&'a str>,
    pub selected_tool: Option<&'a str>,
    pub outcome: &'a str,
    pub failure_reason: Option<&'a str>,
    pub http_status: Option<u16>,
    pub latency_ms: u64,
    pub response_bytes: Option<u64>,
    pub result_count: Option<u32>,
    pub query_present: bool,
    pub reason_present: bool,
    pub benchmark_run: bool,
    pub endpoint: &'a str,
}

/// Pure numeric value used by todo callers, never retained by this crate.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TelemetryScoreSummary {
    pub min: Option<u8>,
    pub mean: Option<f64>,
    pub count: u32,
}

impl TelemetryScoreSummary {
    pub fn from_scores(scores: impl IntoIterator<Item = u8>) -> Self {
        let mut min = None;
        let mut sum = 0_u64;
        let mut count = 0_u32;
        for score in scores {
            let score = score.min(100);
            min = Some(min.map_or(score, |current: u8| current.min(score)));
            sum = sum.saturating_add(u64::from(score));
            count = count.saturating_add(1);
        }
        Self {
            min,
            mean: (count > 0).then(|| sum as f64 / f64::from(count)),
            count,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TodoTelemetryUpdate {
    pub todos_created: u32,
    pub todos_completed: u32,
    pub todos_abandoned: u32,
    pub current_incomplete: u32,
    pub list_size: u32,
    pub groups_completed: u32,
    pub groups_total: u32,
    pub confidence: TelemetryScoreSummary,
    pub completion_confidence: TelemetryScoreSummary,
    pub understands_user_intent: TelemetryScoreSummary,
    pub closed_feedback_loop: TelemetryScoreSummary,
    pub feedback_loop_relevance: TelemetryScoreSummary,
    pub feedback_loop_coverage: TelemetryScoreSummary,
    pub end_to_end_ownership: TelemetryScoreSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoGateKind {
    Ownership,
    ClosedFeedbackLoop,
    FeedbackLoopRelevance,
    FeedbackLoopCoverage,
    FeedbackLoopTraceability,
    Alignment,
    IntentUnderstanding,
    Completion,
    ConfidenceSpike,
}

pub fn record_transcript(
    _provider: &str,
    _model: &str,
    _end_reason: SessionEndReason,
    _messages: Value,
) -> bool {
    false
}
pub fn current_session_correlation_id() -> Option<String> {
    None
}
pub fn current_provider_model() -> Option<(String, String)> {
    None
}
pub fn record_install_if_first_run() {}
pub fn record_upgrade_if_needed() {}
pub fn record_setup_step_once(_step: &'static str) {}
pub fn record_feedback(_text: &str) {}
pub fn record_discovery_event(_data: DiscoveryTelemetry<'_>) {}
pub fn record_todo_update(_update: TodoTelemetryUpdate) {}
pub fn record_command_family(_command: &str) {}
pub fn record_provider_selected(_provider: &str) {}
pub fn record_auth_started(_provider: &str, _method: &str) {}
pub fn record_auth_failed(_provider: &str, _method: &str) {}
pub fn record_auth_failed_reason(_provider: &str, _method: &str, _reason: &str) {}
pub fn record_auth_cancelled(_provider: &str, _method: &str) {}
pub fn record_auth_surface_blocked(_provider: &str, _method: &str) {}
pub fn record_auth_surface_blocked_reason(_provider: &str, _method: &str, _reason: &str) {}
pub fn record_auth_success(_provider: &str, _method: &str) {}
pub fn begin_session(_provider: &str, _model: &str) {}
pub fn begin_session_with_parent(
    _provider: &str,
    _model: &str,
    _parent_session_id: Option<String>,
    _resumed_session: bool,
) {
}
pub fn begin_resumed_session(_provider: &str, _model: &str) {}
pub fn record_turn() {}
pub fn record_assistant_response() {}
pub fn record_memory_injected(_count: usize, _age_ms: u64) {}
pub fn record_tool_call() {}
pub fn record_tool_failure() {}
pub fn record_connection_type(_connection: &str) {}
pub fn record_token_usage(
    _input_tokens: u64,
    _output_tokens: u64,
    _cache_read_input_tokens: Option<u64>,
    _cache_creation_input_tokens: Option<u64>,
) {
}
pub fn record_error(_category: ErrorCategory) {}
pub fn record_provider_switch() {}
pub fn record_model_switch() {}
pub fn record_user_cancelled() {}
pub fn record_todo_gate(_kind: TodoGateKind) {}
pub fn record_tool_execution(_name: &str, _input: &Value, _succeeded: bool, _latency_ms: u64) {}
pub fn end_session(_provider_end: &str, _model_end: &str) {}
pub fn end_session_with_reason(_provider_end: &str, _model_end: &str, _reason: SessionEndReason) {}
pub fn record_crash(_provider_end: &str, _model_end: &str, _reason: SessionEndReason) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_summary_remains_a_pure_numeric_helper() {
        assert_eq!(
            TelemetryScoreSummary::from_scores([]),
            TelemetryScoreSummary::default()
        );
        assert_eq!(
            TelemetryScoreSummary::from_scores([20, 200]),
            TelemetryScoreSummary {
                min: Some(20),
                mean: Some(60.0),
                count: 2,
            }
        );
    }
}
