pub(super) fn session_was_interrupted_by_reload(agent: &Agent) -> bool {
    let messages = agent.messages();
    let Some(last) = messages.last() else {
        return false;
    };

    last.content.iter().any(|block| match block {
        ContentBlock::Text { text, .. } => {
            text.ends_with("[generation interrupted - server reloading]")
        }
        ContentBlock::ToolResult {
            content, is_error, ..
        } => {
            content == "Reload initiated. Process restarting..."
                || (is_error.unwrap_or(false)
                    && (content.contains("interrupted by server reload")
                        || content.contains("Skipped - server reloading")))
        }
        _ => false,
    })
}

pub(super) fn restored_session_was_interrupted(
    session_id: &str,
    previous_status: &crate::session::SessionStatus,
    agent: &Agent,
) -> bool {
    let last_is_user = agent
        .last_message_role()
        .as_ref()
        .map(|role| *role == crate::message::Role::User)
        .unwrap_or(false);
    let last_is_reload_interrupted = session_was_interrupted_by_reload(agent);
    let closed_pending_user_during_reload =
        matches!(previous_status, crate::session::SessionStatus::Closed)
            && last_is_user
            && crate::server::reload_marker_active(RELOAD_RESTORE_MARKER_MAX_AGE);

    if last_is_user && matches!(previous_status, crate::session::SessionStatus::Active) {
        crate::logging::info(&format!(
            "Session {} was Active with pending user message - treating as interrupted",
            session_id
        ));
    }

    if last_is_reload_interrupted {
        crate::logging::info(&format!(
            "Session {} contains reload interruption markers - will auto-resume",
            session_id
        ));
    }

    if closed_pending_user_during_reload {
        crate::logging::info(&format!(
            "Session {} was Closed with a pending user message during a recent reload - treating as interrupted",
            session_id
        ));
    }

    matches!(
        previous_status,
        crate::session::SessionStatus::Crashed { .. }
    ) || (matches!(previous_status, crate::session::SessionStatus::Active) && last_is_user)
        || last_is_reload_interrupted
        || closed_pending_user_during_reload
}
