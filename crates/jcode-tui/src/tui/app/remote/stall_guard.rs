use super::{
    App, DisplayMessage, ProcessingStatus, RemoteConnection,
    recover_undelivered_queued_continuation,
};
use std::time::{Duration, Instant};

const QUEUED_FOLLOWUP_STARVATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Recover the "👉 Auto-poking: N incomplete todos" + spinner-forever state
/// where no request is actually in flight.
///
/// `schedule_auto_poke_followup_if_needed` pushes the continuation onto
/// `queued_messages` and sets `pending_queued_dispatch`. The event loop clears
/// that flag and calls `process_remote_followups`, which returns early WITHOUT
/// sending whenever one of its gates is closed (history not loaded, an earlier
/// pending prompt/split/transfer branch returning first, or `is_processing`
/// still true from a turn whose terminal event was dropped). The flag is
/// already consumed by then, and nothing re-arms it: the follow-up sits in
/// `queued_messages`, `App::is_processing()` keeps reporting true because the
/// queue is non-empty, and the spinner spins while the model is idle.
///
/// `detect_and_cancel_stall` does not cover this: it only runs while
/// `app.is_processing`, which is false in this variant. So track how long a
/// queued follow-up has been idle-but-undispatched and re-arm the dispatch past
/// the timeout, logging it so a recurrence is diagnosable from logs alone.
pub(super) fn detect_starved_queued_followup(app: &mut App) -> bool {
    let starved_candidate =
        !app.is_processing && !app.pending_queued_dispatch && app.has_queued_followups();
    if !starved_candidate {
        app.queued_followup_starved_since = None;
        return false;
    }
    let since = *app
        .queued_followup_starved_since
        .get_or_insert_with(Instant::now);
    let idle_for = since.elapsed();
    if idle_for < QUEUED_FOLLOWUP_STARVATION_TIMEOUT {
        return false;
    }
    crate::logging::warn(&format!(
        "QUEUED_FOLLOWUP_STARVED queued_messages={} hidden_reminders={} interleave={} idle_for_secs={} re-arming dispatch",
        app.queued_messages.len(),
        app.hidden_queued_system_messages.len(),
        app.interleave_message.is_some(),
        idle_for.as_secs(),
    ));
    app.queued_followup_starved_since = None;
    app.pending_queued_dispatch = true;
    true
}

/// Client-side stall budget before the TUI cancels an in-flight turn.
///
/// The server relays provider events over the local socket; when the upstream
/// model reasons silently, no events cross the socket, so a hardcoded short
/// watchdog cannot distinguish a dead connection from a healthy long think
/// (issue #434). Derive it from `[provider] stream_idle_timeout_secs`, scaled by
/// the largest reasoning-effort multiplier because effort is invisible here,
/// plus grace so the server-side idle timeout (which produces a visible error
/// event) always fires first. Never below 2 minutes.
fn stall_timeout() -> Duration {
    const MIN_STALL_TIMEOUT: Duration = Duration::from_secs(2 * 60);
    const GRACE: Duration = Duration::from_secs(30);
    let provider_idle = crate::provider::max_stream_idle_timeout();
    provider_idle.saturating_add(GRACE).max(MIN_STALL_TIMEOUT)
}

/// Human-readable stall duration for user-facing stall messages, e.g.
/// "2 minutes", "3.5 minutes", or "90 seconds".
fn format_stall_duration(timeout: Duration) -> String {
    let secs = timeout.as_secs();
    if secs < 120 {
        format!("{} seconds", secs)
    } else if secs.is_multiple_of(60) {
        format!("{} minutes", secs / 60)
    } else {
        format!("{:.1} minutes", secs as f64 / 60.0)
    }
}

pub(super) async fn detect_and_cancel_stall(app: &mut App, remote: &mut RemoteConnection) {
    let stall_timeout = stall_timeout();
    let is_running_tool = matches!(app.status, ProcessingStatus::RunningTool(_));
    if app.is_processing && !is_running_tool {
        let stalled = app
            .last_stream_activity
            .map(|t| t.elapsed() > stall_timeout)
            .unwrap_or_else(|| {
                app.processing_started
                    .map(|t| t.elapsed() > stall_timeout)
                    .unwrap_or(false)
            });
        if stalled {
            if let Some(snapshot) = app.remote_resume_activity.clone() {
                let elapsed = app
                    .last_stream_activity
                    .map(|t| t.elapsed())
                    .or(app.processing_started.map(|t| t.elapsed()));
                crate::logging::warn(&format!(
                    "Protocol stall guard: resumed session {} is still marked processing by history snapshot (tool={:?}, snapshot_age={:?}) but no corroborating live events arrived after {:?}; deferring client-side cancel",
                    snapshot.session_id,
                    snapshot.current_tool_name,
                    snapshot.observed_at.elapsed(),
                    elapsed
                ));
                app.last_stream_activity = Some(Instant::now());
                app.status = match snapshot.current_tool_name {
                    Some(tool_name) => ProcessingStatus::RunningTool(tool_name),
                    None => ProcessingStatus::Thinking(Instant::now()),
                };
                return;
            }
            crate::logging::warn(&format!(
                "Stream stall detected: no server events for {:?}, cancelling",
                app.last_stream_activity
                    .map(|t| t.elapsed())
                    .or(app.processing_started.map(|t| t.elapsed()))
            ));
            let _ = remote.cancel_with_reason("stall_guard").await;
            app.is_processing = false;
            app.clear_visible_turn_started();
            app.status = ProcessingStatus::Idle;
            app.current_message_id = None;
            app.processing_started = None;
            app.last_stream_activity = None;
            if !app.streaming.streaming_text.is_empty() {
                let content = app.take_streaming_text();
                let content = app.collapse_reasoning_for_commit(content);
                if !content.trim().is_empty() {
                    app.push_display_message(DisplayMessage {
                        role: "assistant".to_string(),
                        content,
                        tool_calls: vec![],
                        duration_secs: None,
                        title: None,
                        tool_data: None,
                    });
                }
            }
            let stall_desc = format_stall_duration(stall_timeout);
            if !app.schedule_pending_remote_retry(&format!(
                "⚠ Stream stalled (no response for {stall_desc}). Processing cancelled.",
            )) {
                // Keep a dispatched-but-unfinished queued follow-up on the
                // queue instead of silently dropping it (issue #391).
                let recovered = recover_undelivered_queued_continuation(app, "stream stall");
                app.clear_pending_remote_retry();
                if recovered {
                    app.push_display_message(DisplayMessage::system(format!(
                        "⚠ Stream stalled (no response for {stall_desc}). Processing cancelled. Your queued follow-up stays queued.",
                    )));
                } else {
                    app.push_display_message(DisplayMessage::system(format!(
                        "⚠ Stream stalled (no response for {stall_desc}). Processing cancelled. You can resend your message. Raise `[provider] stream_idle_timeout_secs` in config.toml if your model thinks silently for longer.",
                    )));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    include!("../remote_stall_guard_tests_body_tests.rs");
}
