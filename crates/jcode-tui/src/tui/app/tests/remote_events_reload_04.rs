#[test]
fn test_remote_fast_status_tracks_history_tier_including_explicit_off() {
    with_temp_jcode_home(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let mut app = create_test_app();
        let mut remote = crate::tui::backend::RemoteConnection::dummy();
        app.is_remote = true;
        app.remote_session_id = Some("session_fast_status".to_string());
        remote.mark_history_loaded();

        // Bootstrap, catalog refresh, explicit off, then on again. An absent
        // tier is authoritative Standard, not an invitation to use the saved
        // default or preserve a stale priority badge.
        for tier in [Some("priority"), Some("priority"), None, Some("priority")] {
            let history = serde_json::from_value(serde_json::json!({
                "type": "history",
                "id": 1,
                "session_id": "session_fast_status",
                "messages": [],
                "provider_name": "OpenAI",
                "provider_model": "gpt-future-model",
                "service_tier": tier
            }))
            .expect("history fixture");
            app.handle_server_event(history, &mut remote);
            assert_eq!(app.remote_service_tier.as_deref(), tier);
            assert_eq!(
                crate::tui::TuiState::info_widget_data(&app)
                    .service_tier
                    .as_deref(),
                tier
            );

            app.input = "/fast status".to_string();
            rt.block_on(app.handle_remote_key(KeyCode::Enter, KeyModifiers::empty(), &mut remote))
                .expect("fast status");
            let status = &app
                .display_messages()
                .last()
                .expect("status message")
                .content;
            let expected = if tier.is_some() {
                "Fast mode is on."
            } else {
                "Fast mode is off."
            };
            assert!(status.starts_with(expected), "{status}");
        }
    });
}

#[test]
fn test_remote_debug_frame_commands_request_a_fresh_draw() {
    for command in [
        "frame",
        "frame-normalized",
        "screen",
        "screen-json",
        "screen-json-normalized",
        "layout",
        "margins",
        "widgets",
        "info-widgets",
        "render-stats",
        "render-order",
        "anomalies",
        "theme",
    ] {
        assert!(
            remote::debug_command_needs_current_frame(command),
            "{command} should capture the current frame"
        );
    }

    for command in ["state", "picker", "memory", "draw-stats", "overlay:on"] {
        assert!(
            !remote::debug_command_needs_current_frame(command),
            "{command} should not force a draw"
        );
    }
}

#[test]
fn test_remote_error_without_retry_recovers_pending_followups() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "retry me".to_string(),
        images: vec![],
        is_system: false,
        system_reminder: None,
        auto_retry: false,
        retry_attempts: 0,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;
    app.current_message_id = Some(10);
    app.interleave_message = Some("unsent interleave".to_string());
    app.pending_soft_interrupts = vec!["acked interleave".to_string()];
    app.pending_soft_interrupt_requests = vec![(88, "acked interleave".to_string())];
    app.queued_messages.push("queued later".to_string());

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 10,
            message: "provider failed hard".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    assert!(app.rate_limit_pending_message.is_none());
    assert!(app.interleave_message.is_none());
    assert_eq!(
        app.queued_messages(),
        &["unsent interleave", "queued later"]
    );
    assert_eq!(app.pending_soft_interrupts, vec!["acked interleave"]);
    assert_eq!(
        app.pending_soft_interrupt_requests,
        vec![(88, "acked interleave".to_string())]
    );

    rt.block_on(remote::process_remote_followups(&mut app, &mut remote));

    assert!(app.pending_soft_interrupts.is_empty());
    assert!(app.pending_soft_interrupt_requests.is_empty());
    assert!(app.queued_messages().is_empty());
    assert!(app.is_processing);
    assert!(matches!(app.status, ProcessingStatus::Sending));

    let last = app
        .display_messages()
        .last()
        .expect("missing error message");
    assert_eq!(last.role, "user");
    assert_eq!(last.content, "queued later");
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "error" && m.content == "provider failed hard")
    );
}

#[test]
fn test_remote_error_with_retryable_pending_schedules_retry() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "retry me".to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: true,
        retry_attempts: 0,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 11,
            message: "provider failed hard".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    let pending = app
        .rate_limit_pending_message
        .as_ref()
        .expect("retryable continuation should remain pending");
    assert!(pending.auto_retry);
    assert_eq!(pending.retry_attempts, 1);
    assert!(pending.retry_at.is_some());
    assert!(app.rate_limit_reset.is_some());
    let retry_notice = app
        .display_messages()
        .iter()
        .find(|message| message.title.as_deref() == Some("Connection"))
        .expect("retry should surface a connection status message");
    assert_eq!(retry_notice.role, "system");
    assert!(retry_notice.content.contains("Connection lost - retrying"));
    assert!(
        retry_notice
            .content
            .contains(&format!("attempt 1/{}", App::AUTO_RETRY_MAX_ATTEMPTS))
    );
    assert!(retry_notice.content.contains("Remote request failed"));
}

#[test]
fn test_remote_non_retryable_error_gets_short_auto_poke_retry() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.auto_poke_incomplete_todos = true;
    app.queued_messages
        .push("You have 1 incomplete todo. Continue working, or update the todo tool.".to_string());
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "You have 1 incomplete todo. Continue working, or update the todo tool."
            .to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: true,
        retry_attempts: 0,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 12,
            message: "OpenAI API error 400 Bad Request: {\"error\":{\"message\":\"Invalid 'input[0].encrypted_content': string too long. Expected a string with maximum length 10485760, but got a string with length 11237432 instead.\",\"type\":\"invalid_request_error\",\"code\":\"string_above_max_length\"}}".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    assert!(app.auto_poke_incomplete_todos);
    let pending = app
        .rate_limit_pending_message
        .as_ref()
        .expect("deterministic error should get a short retry budget");
    assert_eq!(pending.retry_attempts, 1);
    assert!(app.rate_limit_reset.is_some());
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("attempt 1/2"))
    );

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 13,
            message: "OpenAI API error 400 Bad Request: {\"error\":{\"type\":\"invalid_request_error\",\"code\":\"string_above_max_length\"}}".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    assert!(app.auto_poke_incomplete_todos);
    let pending = app
        .rate_limit_pending_message
        .as_ref()
        .expect("second deterministic error should still get final retry");
    assert_eq!(pending.retry_attempts, 2);
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("attempt 2/2"))
    );
}

#[test]
fn test_remote_non_retryable_error_stops_auto_poke_after_short_retry_budget() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.auto_poke_incomplete_todos = true;
    app.queued_messages
        .push("You have 1 incomplete todo. Continue working, or update the todo tool.".to_string());
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "You have 1 incomplete todo. Continue working, or update the todo tool."
            .to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: true,
        retry_attempts: 2,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 14,
            message: "OpenAI API error 400 Bad Request: {\"error\":{\"type\":\"invalid_request_error\",\"code\":\"string_above_max_length\"}}".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    assert!(!app.auto_poke_incomplete_todos);
    assert!(app.queued_messages().is_empty());
    assert!(app.rate_limit_pending_message.is_none());
    assert!(app.rate_limit_reset.is_none());
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("we stopped poking"))
    );
}

#[test]
fn test_remote_fatal_model_endpoint_error_fails_fast_without_retry_budget() {
    // Volcengine Ark coding-plan endpoint returning 404 UnsupportedModel can
    // never succeed on resend, so the recovery/reconnect continuation must NOT
    // burn the auto-retry budget on it (#387).
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.auto_poke_incomplete_todos = true;
    app.queued_messages
        .push("You have 1 incomplete todo. Continue working, or update the todo tool.".to_string());
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "continue".to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: true,
        retry_attempts: 0,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 21,
            message: "OpenAI-compatible chat request failed\n  endpoint: https://ark.cn-beijing.volces.com/api/coding/v3/chat/completions\n  model: volcengine:ark-code-latest\n  auth: ARK_API_KEY\n  status: 404 Not Found\n  response: {\"error\":{\"code\":\"UnsupportedModel\",\"message\":\"The requested model does not support the coding plan feature.\"}}".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    // No retry scheduled: pending cleared immediately, no backoff timer set.
    assert!(app.rate_limit_pending_message.is_none());
    assert!(app.rate_limit_reset.is_none());
    // Auto-poke is stopped, and an actionable hint is shown.
    assert!(!app.auto_poke_incomplete_todos);
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("Not retrying")),
        "expected a fail-fast model/endpoint hint, got: {:?}",
        app.display_messages()
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
    );
    // It must not have produced a "retrying (attempt N/M)" message.
    assert!(
        !app.display_messages()
            .iter()
            .any(|m| m.content.contains("attempt 1/"))
    );
}

#[test]
fn test_remote_connectivity_error_waits_for_network_without_retry_budget() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.auto_poke_incomplete_todos = true;
    app.queued_messages
        .push("You have 1 incomplete todo. Continue working, or update the todo tool.".to_string());
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "You have 1 incomplete todo. Continue working, or update the todo tool."
            .to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: true,
        retry_attempts: 0,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 15,
            message: "Failed to send OpenAI-compatible chat request\n  endpoint: https://api.groq.com/openai/v1/chat/completions\n  model: llama-3.1-8b-instant\n  auth: GROQ_API_KEY\nHint: check network connectivity, DNS/TLS, and that the base URL includes the API version (usually /v1).: error sending request for url (https://api.groq.com/openai/v1/chat/completions): client error (Connect): dns error: failed to lookup address information: Name or service not known".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    assert!(app.auto_poke_incomplete_todos);
    assert!(!app.queued_messages().is_empty());
    let pending = app
        .rate_limit_pending_message
        .as_ref()
        .expect("offline auto-poke should be held for network recovery");
    assert_eq!(pending.retry_attempts, 0);
    assert!(app.rate_limit_reset.is_some());
    assert!(matches!(
        app.status,
        ProcessingStatus::WaitingForNetwork { .. }
    ));
    assert_eq!(
        app.status_detail.as_deref(),
        Some("offline; waiting for network before retry")
    );
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("Network appears offline"))
    );
    assert!(
        !app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("attempt 1/2"))
    );
}

#[test]
fn test_remote_connectivity_error_without_auto_retry_still_waits_for_network() {
    // Regression: an auto-poke continuation that carries a visible message gets
    // auto_retry=false. A transient DNS failure must still hold the turn for
    // network recovery instead of permanently stopping auto-poke.
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.auto_poke_incomplete_todos = true;
    app.queued_messages
        .push("You have 1 incomplete todo. Continue working, or update the todo tool.".to_string());
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "Continue working on the task.".to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: false,
        retry_attempts: 0,
        retry_at: None,
    });
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 16,
            message: "Failed to send request to Anthropic API: error sending request for url (https://api.anthropic.com/v1/messages): client error (Connect): dns error: failed to lookup address information: Name or service not known".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    // Auto-poke must stay enabled and queued work preserved.
    assert!(app.auto_poke_incomplete_todos);
    assert!(!app.queued_messages().is_empty());
    let pending = app
        .rate_limit_pending_message
        .as_ref()
        .expect("offline turn should be held for network recovery");
    // Promoted to auto_retry so the tick-based resume re-sends it.
    assert!(pending.auto_retry);
    assert_eq!(pending.retry_attempts, 0);
    assert!(app.rate_limit_reset.is_some());
    assert!(matches!(
        app.status,
        ProcessingStatus::WaitingForNetwork { .. }
    ));
    assert!(
        !app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("we stopped poking"))
    );
}

fn openai_oauth_route(model: &str) -> crate::provider::ModelRoute {
    crate::provider::ModelRoute {
        model: model.to_string(),
        provider: "OpenAI".to_string(),
        api_method: "openai-oauth".to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    }
}

fn claude_oauth_route(model: &str) -> crate::provider::ModelRoute {
    crate::provider::ModelRoute {
        model: model.to_string(),
        provider: "Anthropic".to_string(),
        api_method: "claude-oauth".to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    }
}

/// The motivating scenario: a remote session's OpenAI OAuth session expires
/// (token refresh fails, non-retryable), and a working Claude OAuth route
/// exists. The terminal error should arm a one-keypress fallback offer that
/// carries the failed payload for resend.
#[test]
fn test_remote_auth_error_arms_fallback_offer_with_resend_payload() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.is_remote = true;
    app.remote_provider_name = Some("OpenAI".to_string());
    app.remote_provider_model = Some("gpt-5.5".to_string());
    app.remote_model_options = vec![
        openai_oauth_route("gpt-5.5"),
        openai_oauth_route("gpt-5.4"),
        claude_oauth_route("claude-sonnet-4"),
    ];
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "hi".to_string(),
        images: vec![],
        is_system: false,
        system_reminder: None,
        auto_retry: false,
        retry_attempts: 0,
        retry_at: None,
    });
    app.last_submitted_input = Some("hi".to_string());
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 21,
            message: "OpenAI token refresh failed; run /login to re-authenticate: {\"error\":{\"message\":\"Your session has ended. Please log in again.\",\"type\":\"invalid_request_error\",\"code\":\"refresh_token_invalidated\"}}".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );

    let offer = app
        .pending_fallback_offer
        .as_ref()
        .expect("terminal auth error should arm a fallback offer");
    // A credential failure must not offer a sibling model behind the same
    // broken OpenAI login; it must hop to the working Anthropic route.
    assert_eq!(offer.selection.provider_label, "Anthropic");
    let resend = offer
        .remote_resend
        .as_ref()
        .expect("remote offer should capture the failed payload");
    assert_eq!(resend.content, "hi");
    assert_eq!(resend.raw_input.as_deref(), Some("hi"));
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("Fallback available")),
        "offer message should be shown"
    );
}

/// Accepting a remote fallback offer stages the route switch (SetRoute via the
/// dispatcher) and the payload resend; the ModelChanged confirmation then
/// dispatches the resend through process_remote_followups.
#[test]
fn test_remote_fallback_offer_accept_stages_switch_and_resends() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    app.is_remote = true;
    app.remote_provider_name = Some("OpenAI".to_string());
    app.remote_provider_model = Some("gpt-5.5".to_string());
    app.remote_model_options = vec![
        openai_oauth_route("gpt-5.5"),
        claude_oauth_route("claude-sonnet-4"),
    ];
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "hi".to_string(),
        images: vec![],
        is_system: false,
        system_reminder: None,
        auto_retry: false,
        retry_attempts: 0,
        retry_at: None,
    });
    app.last_submitted_input = Some("hi".to_string());
    app.is_processing = true;
    app.status = ProcessingStatus::Streaming;

    app.handle_server_event(
        crate::protocol::ServerEvent::Error {
            id: 22,
            message: "OpenAI token refresh failed; run /login to re-authenticate: refresh_token_invalidated".to_string(),
            retry_after_secs: None,
        },
        &mut remote,
    );
    assert!(app.pending_fallback_offer.is_some());

    assert!(app.apply_pending_fallback_offer());
    assert!(
        app.pending_route_selection.is_some(),
        "accept should stage a SetRoute request for the remote dispatcher"
    );
    let staged = app
        .pending_fallback_resend
        .as_ref()
        .expect("accept should stage the failed payload for resend");
    assert_eq!(staged.content, "hi");

    // Server confirms the switch.
    app.pending_route_selection = None;
    app.remote_model_switch_in_flight = true;
    app.handle_server_event(
        crate::protocol::ServerEvent::ModelChanged {
            id: 0,
            model: "claude-sonnet-4".to_string(),
            provider_name: Some("Anthropic".to_string()),
            error: None,
            resolved_credential: None,
        },
        &mut remote,
    );
    assert!(!app.remote_model_switch_in_flight);

    // The followup dispatcher resends the failed payload on the new route.
    rt.block_on(remote::process_remote_followups(&mut app, &mut remote));
    assert!(app.pending_fallback_resend.is_none());
    assert!(app.is_processing, "resend should start a new turn");
    assert!(matches!(app.status, ProcessingStatus::Sending));
    let pending = app
        .rate_limit_pending_message
        .as_ref()
        .expect("resend should repopulate the pending retry slot");
    assert_eq!(pending.content, "hi");
}

/// A failed route switch must drop the staged resend instead of firing it on
/// the old (broken) route.
#[test]
fn test_remote_fallback_resend_dropped_when_switch_fails() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();
    remote.mark_history_loaded();

    app.is_remote = true;
    app.pending_fallback_resend = Some(crate::tui::app::FallbackResendPayload {
        content: "hi".to_string(),
        images: vec![],
        is_system: false,
        auto_retry: false,
        system_reminder: None,
        raw_input: Some("hi".to_string()),
    });
    app.remote_model_switch_in_flight = true;

    app.handle_server_event(
        crate::protocol::ServerEvent::ModelChanged {
            id: 0,
            model: "claude-sonnet-4".to_string(),
            provider_name: None,
            error: Some("switch failed".to_string()),
            resolved_credential: None,
        },
        &mut remote,
    );

    assert!(app.pending_fallback_resend.is_none());
    assert_eq!(
        app.input, "hi",
        "prompt should be restored to the input box"
    );

    rt.block_on(remote::process_remote_followups(&mut app, &mut remote));
    assert!(!app.is_processing, "no resend should fire");
}

#[test]
fn test_schedule_pending_remote_retry_respects_retry_limit() {
    let mut app = create_test_app();
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "retry me".to_string(),
        images: vec![],
        is_system: true,
        system_reminder: None,
        auto_retry: true,
        retry_attempts: App::AUTO_RETRY_MAX_ATTEMPTS,
        retry_at: None,
    });

    assert!(!app.schedule_pending_remote_retry("⚠ failed."));
    assert!(app.rate_limit_pending_message.is_none());
    assert!(app.rate_limit_reset.is_none());
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "error" && m.content.contains("Auto-retry limit reached"))
    );
}

/// A provider guardrail refusal (e.g. Anthropic stop_reason "refusal") should
/// arm a one-keypress reroute offer to claude-opus-4-8, carrying the refused
/// payload so accepting the offer resends it on the stronger route.
#[test]
fn test_provider_guardrail_event_offers_opus_reroute_with_resend_payload() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.is_remote = true;
    app.remote_provider_name = Some("OpenAI".to_string());
    app.remote_provider_model = Some("gpt-5.5".to_string());
    app.remote_model_options = vec![
        openai_oauth_route("gpt-5.5"),
        claude_oauth_route("claude-sonnet-4"),
        claude_oauth_route("claude-opus-4-8"),
    ];
    app.rate_limit_pending_message = Some(PendingRemoteMessage {
        content: "please help".to_string(),
        images: vec![],
        is_system: false,
        system_reminder: None,
        auto_retry: false,
        retry_attempts: 0,
        retry_at: None,
    });
    app.last_submitted_input = Some("please help".to_string());

    app.handle_server_event(
        crate::protocol::ServerEvent::ProviderGuardrail {
            stop_reason: Some("refusal".to_string()),
            message: "Provider guardrail stopped the response (stop_reason: refusal). The model declined to answer this request.".to_string(),
        },
        &mut remote,
    );

    let offer = app
        .pending_fallback_offer
        .as_ref()
        .expect("guardrail event should arm a reroute offer");
    assert_eq!(offer.selection.model, "claude-opus-4-8");
    assert_eq!(offer.selection.provider_label, "Anthropic");
    let resend = offer
        .remote_resend
        .as_ref()
        .expect("offer should capture the refused payload for resend");
    assert_eq!(resend.content, "please help");
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("Reroute available")),
        "reroute offer message should be shown"
    );
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("[guardrail]")),
        "guardrail notice itself should still be shown"
    );
}

/// The reroute offer must prefer native Anthropic auth over aggregator routes
/// that also expose claude-opus-4-8.
#[test]
fn test_guardrail_reroute_prefers_native_anthropic_route() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.is_remote = true;
    app.remote_provider_name = Some("OpenAI".to_string());
    app.remote_provider_model = Some("gpt-5.5".to_string());
    app.remote_model_options = vec![
        openai_oauth_route("gpt-5.5"),
        crate::provider::ModelRoute {
            model: "claude-opus-4-8".to_string(),
            provider: "OpenRouter".to_string(),
            api_method: "openrouter".to_string(),
            available: true,
            detail: String::new(),
            usage: None,
            cheapness: None,
        },
        claude_oauth_route("claude-opus-4-8"),
    ];

    app.handle_server_event(
        crate::protocol::ServerEvent::ProviderGuardrail {
            stop_reason: Some("refusal".to_string()),
            message: "refused".to_string(),
        },
        &mut remote,
    );

    let offer = app
        .pending_fallback_offer
        .as_ref()
        .expect("guardrail event should arm a reroute offer");
    assert_eq!(offer.selection.provider_label, "Anthropic");
    assert_eq!(offer.selection.api_method, "claude-oauth");
}

/// No reroute offer when the session is already on claude-opus-4-8: there is
/// nothing stronger to hop to, so only the guardrail notice should appear.
#[test]
fn test_guardrail_reroute_not_offered_when_already_on_opus() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.is_remote = true;
    app.remote_provider_name = Some("Anthropic".to_string());
    app.remote_provider_model = Some("claude-opus-4-8".to_string());
    app.remote_model_options = vec![
        claude_oauth_route("claude-opus-4-8"),
        claude_oauth_route("claude-sonnet-4"),
    ];

    app.handle_server_event(
        crate::protocol::ServerEvent::ProviderGuardrail {
            stop_reason: Some("refusal".to_string()),
            message: "refused".to_string(),
        },
        &mut remote,
    );

    assert!(
        app.pending_fallback_offer.is_none(),
        "no reroute offer when already on the reroute target"
    );
    assert!(
        app.display_messages()
            .iter()
            .any(|m| m.role == "system" && m.content.contains("[guardrail]")),
        "guardrail notice should still be shown"
    );
}

/// No reroute offer when no claude-opus-4-8 route exists in the catalog.
#[test]
fn test_guardrail_reroute_not_offered_without_opus_route() {
    let mut app = create_test_app();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    let mut remote = crate::tui::backend::RemoteConnection::dummy();

    app.is_remote = true;
    app.remote_provider_name = Some("OpenAI".to_string());
    app.remote_provider_model = Some("gpt-5.5".to_string());
    app.remote_model_options = vec![openai_oauth_route("gpt-5.5"), openai_oauth_route("gpt-5.4")];

    app.handle_server_event(
        crate::protocol::ServerEvent::ProviderGuardrail {
            stop_reason: Some("refusal".to_string()),
            message: "refused".to_string(),
        },
        &mut remote,
    );

    assert!(app.pending_fallback_offer.is_none());
}

#[test]
fn test_info_widget_data_includes_connection_type() {
    let mut app = create_test_app();
    app.connection_type = Some("https".to_string());
    let data = crate::tui::TuiState::info_widget_data(&app);
    assert_eq!(data.connection_type.as_deref(), Some("https"));
}

#[test]
fn test_remote_tui_state_prefers_cached_model_during_brief_connecting_phase() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("create temp home");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let session_id = "session_otter_123";
    let mut session = crate::session::Session::create_with_id(
        session_id.to_string(),
        None,
        Some("remote cached model".to_string()),
    );
    session.model = Some("gpt-5.4".to_string());
    session.save().expect("save remote session");

    let app = App::new_for_remote(Some(session_id.to_string()));

    assert_eq!(crate::tui::TuiState::provider_model(&app), "gpt-5.4");
    assert_eq!(crate::tui::TuiState::provider_name(&app), "openai");
    assert_eq!(
        crate::tui::TuiState::session_display_name(&app).as_deref(),
        Some("otter")
    );

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[test]
fn test_remote_tui_state_falls_back_to_cached_model_after_startup_phase_clears() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("create temp home");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let session_id = "session_otter_124";
    let mut session = crate::session::Session::create_with_id(
        session_id.to_string(),
        None,
        Some("remote cached model".to_string()),
    );
    session.model = Some("gpt-5.4".to_string());
    session.save().expect("save remote session");

    let mut app = App::new_for_remote(Some(session_id.to_string()));
    app.clear_remote_startup_phase();

    assert_eq!(crate::tui::TuiState::provider_model(&app), "gpt-5.4");
    assert_eq!(crate::tui::TuiState::provider_name(&app), "openai");

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[test]
fn test_new_for_remote_uses_startup_stub_without_loading_full_transcript() {
    let _guard = crate::storage::lock_test_env();
    let temp_home = tempfile::TempDir::new().expect("create temp home");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp_home.path());

    let session_id = "session_otter_stub_125";
    let mut session = crate::session::Session::create_with_id(
        session_id.to_string(),
        None,
        Some("remote cached model".to_string()),
    );
    session.model = Some("gpt-5.4".to_string());
    session.append_stored_message(crate::session::StoredMessage {
        id: "msg-startup-stub".to_string(),
        role: crate::message::Role::User,
        content: vec![crate::message::ContentBlock::Text {
            text: "hello from persisted history".to_string(),
            cache_control: None,
        }],
        display_role: None,
        timestamp: None,
        tool_duration_ms: None,
        token_usage: None,
    });
    session.save().expect("save remote session");

    let app = App::new_for_remote(Some(session_id.to_string()));
    assert_eq!(app.session_id(), session_id);
    assert_eq!(app.display_messages().len(), 1);
    assert_eq!(
        app.display_messages()[0].content,
        "hello from persisted history"
    );
    // The remote client renders persisted history into `display_messages`,
    // then calls `strip_transcript_for_remote_client()` to release the backing
    // transcript (the server is the source of truth for the live transcript).
    // So the stripped `session.messages` is expected to be empty here.
    assert_eq!(app.session.messages.len(), 0);
    assert_eq!(app.remote_session_id.as_deref(), Some(session_id));
    assert_eq!(crate::tui::TuiState::provider_model(&app), "gpt-5.4");

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

/// Run `f` with the shared env lock held and `JCODE_MODEL`/`JCODE_PROVIDER`
/// neutralized. Remote header tests assert on startup-phase labels, which are
/// only shown when no model hint is known; a sibling test that persisted a
/// config `default_model` (shared test home) or exported `JCODE_MODEL` would
/// otherwise leak a concrete model into the header.
fn with_neutral_remote_model_hints<T>(f: impl FnOnce() -> T) -> T {
    let _guard = crate::storage::lock_test_env();
    let prev_model = std::env::var_os("JCODE_MODEL");
    let prev_provider = std::env::var_os("JCODE_PROVIDER");
    // "unknown" is sanitized to no-hint and, unlike removing the var, also
    // suppresses the config default_model fallback.
    crate::env::set_var("JCODE_MODEL", "unknown");
    crate::env::remove_var("JCODE_PROVIDER");

    let result = f();

    if let Some(prev_model) = prev_model {
        crate::env::set_var("JCODE_MODEL", prev_model);
    } else {
        crate::env::remove_var("JCODE_MODEL");
    }
    if let Some(prev_provider) = prev_provider {
        crate::env::set_var("JCODE_PROVIDER", prev_provider);
    } else {
        crate::env::remove_var("JCODE_PROVIDER");
    }
    result
}

#[test]
fn test_remote_tui_state_shows_connected_after_startup_phase_clears_without_model() {
    with_neutral_remote_model_hints(|| {
        let mut app = App::new_for_remote(None);
        app.remote_session_id = Some("session_connected_123".to_string());
        app.clear_remote_startup_phase();

        assert_eq!(crate::tui::TuiState::provider_model(&app), "connected");
        assert_eq!(crate::tui::TuiState::provider_name(&app), "");
    });
}

include!("remote_events_reload_04_partition_01_tests.rs");
include!("remote_events_reload_04_partition_02_tests.rs");
