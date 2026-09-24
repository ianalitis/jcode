use super::*;
use jcode_provider_openrouter::stream::OpenRouterStream;

pub(super) fn chat_completions_url(api_base: &str) -> String {
    format!("{api_base}/chat/completions")
}

fn local_endpoint_troubleshooting_hint(api_base: &str, model: &str) -> &'static str {
    let lower = api_base.to_ascii_lowercase();
    if lower.contains("localhost:11434") || lower.contains("127.0.0.1:11434") {
        return "Ollama hint: make sure `ollama serve` is running, the model is installed with `ollama pull <model>`, and run jcode with an installed model, for example `jcode --provider ollama --model llama3.2 run 'hello'`. If replies ignore earlier turns, Ollama is truncating the prompt to its serving context: restart it with a larger window, e.g. `OLLAMA_CONTEXT_LENGTH=65536 ollama serve`.";
    }

    if lower.contains("localhost:1234") || lower.contains("127.0.0.1:1234") {
        return "LM Studio hint: start the Local Server in LM Studio, load a chat model, and run jcode with the exact model id shown by LM Studio's /v1/models endpoint.";
    }

    if lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("[::1]") {
        return "Local endpoint hint: make sure the server is running, the base URL includes /v1, the selected model is loaded, and the server supports streaming POST /chat/completions.";
    }

    let _ = model;
    "Hint: check network connectivity, DNS/TLS, that the base URL includes the API version (usually /v1), and that the model exists on the provider."
}

// ============================================================================
// SSE Stream Parser
// ============================================================================

#[expect(
    clippy::too_many_arguments,
    reason = "stream helpers thread transport, auth, request, event channel, and pin state explicitly"
)]
pub(super) async fn run_stream_with_retries(
    client: Client,
    api_base: String,
    auth: ProviderAuth,
    send_openrouter_headers: bool,
    conversation_id: String,
    request: Value,
    tx: mpsc::Sender<Result<StreamEvent>>,
    provider_pin: Arc<Mutex<Option<ProviderPin>>>,
    model: String,
) {
    let mut last_error = None;
    let mut next_retry_delay = None;
    let config = jcode_base::config::config();
    let max_retries = config.provider.max_retries.max(1);
    let retry_backoff_cap =
        std::time::Duration::from_secs(config.provider.retry_backoff_cap_secs.max(1));

    for attempt in 0..max_retries {
        if attempt > 0 {
            let delay = jcode_provider_core::retry_after::retry_delay(
                attempt,
                RETRY_BASE_DELAY_MS,
                next_retry_delay.take(),
            )
            .min(retry_backoff_cap);
            tokio::time::sleep(delay).await;
            jcode_base::logging::info(&format!(
                "Retrying API request using {} (attempt {}/{})",
                auth.label(),
                attempt + 1,
                max_retries
            ));
        }

        jcode_base::logging::info(&format!(
            "API stream attempt {}/{} over HTTPS transport (model: {}, endpoint: {}, auth: {})",
            attempt + 1,
            max_retries,
            model,
            api_base,
            auth.label()
        ));

        // Track whether this attempt streams replay-visible output so a
        // mid-stream transport fault can roll the partial output back on the
        // consumer before the retry replays the response from the top.
        let (attempt_tx, attempt_guard) =
            jcode_provider_core::attempt_tracker::track_attempt_output(tx.clone());

        // Retries use a fresh unpooled client: the fault that broke attempt N
        // (e.g. TLS BadRecordMac from a corrupting middlebox) may also have
        // poisoned other idle pooled connections opened through the same path,
        // so reusing the shared pool can fail identically. A fresh client
        // guarantees a brand-new TCP+TLS connection.
        let attempt_client = if attempt == 0 {
            client.clone()
        } else {
            jcode_provider_core::fresh_transport_client()
        };

        match stream_response(
            attempt_client,
            api_base.clone(),
            chat_completions_url(&api_base),
            auth.clone(),
            send_openrouter_headers,
            &conversation_id,
            request.clone(),
            attempt_tx,
            Arc::clone(&provider_pin),
            model.clone(),
        )
        .await
        {
            Ok(()) => {
                let _ = attempt_guard.finish().await;
                return;
            }
            Err(e) => {
                let saw_output = attempt_guard.finish().await;
                // Full anyhow chain ({:#}) so a `.context(...)`-wrapped transport
                // cause (e.g. TLS BadRecordMac) is visible to the classifier.
                let error_str = format!("{e:#}").to_lowercase();
                if is_retryable_error(&error_str) && attempt + 1 < max_retries {
                    if saw_output {
                        // Partial output already reached the consumer; tell it
                        // to discard the partial attempt so the retried
                        // response replays cleanly instead of duplicating.
                        jcode_base::logging::warn(&format!(
                            "Transient API error after partial output; rolling back partial attempt and retrying: {}",
                            e
                        ));
                        let _ = tx
                            .send(Ok(StreamEvent::RetryRollback {
                                attempt: attempt + 2,
                                max: max_retries,
                            }))
                            .await;
                    } else {
                        jcode_base::logging::info(&format!(
                            "Transient API error, will retry: {}",
                            e
                        ));
                    }
                    next_retry_delay = jcode_provider_core::retry_after::retry_after_from_error(&e);
                    last_error = Some(e);
                    continue;
                }

                let _ = tx.send(Err(e)).await;
                return;
            }
        }
    }

    if let Some(e) = last_error {
        let _ = tx
            .send(Err(anyhow::anyhow!(
                "Failed after {} retries: {}",
                max_retries,
                e
            )))
            .await;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "single-send threads its constrained transport and immutable request state explicitly"
)]
pub(super) async fn run_stream_once(
    client: Client,
    api_base: String,
    destination: String,
    auth: ProviderAuth,
    send_openrouter_headers: bool,
    conversation_id: String,
    request: Value,
    tx: mpsc::Sender<Result<StreamEvent>>,
    provider_pin: Arc<Mutex<Option<ProviderPin>>>,
    model: String,
) {
    if stream_response(
        client,
        api_base,
        destination,
        auth,
        send_openrouter_headers,
        &conversation_id,
        request,
        tx.clone(),
        provider_pin,
        model,
    )
    .await
    .is_err()
    {
        let _ = tx
            .send(Err(anyhow::anyhow!(
                "constrained single-send OpenRouter request failed"
            )))
            .await;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "stream helpers thread transport, auth, request, event channel, and pin state explicitly"
)]
async fn stream_response(
    client: Client,
    api_base: String,
    url: String,
    auth: ProviderAuth,
    send_openrouter_headers: bool,
    conversation_id: &str,
    request: Value,
    tx: mpsc::Sender<Result<StreamEvent>>,
    provider_pin: Arc<Mutex<Option<ProviderPin>>>,
    model: String,
) -> Result<()> {
    use jcode_message_types::ConnectionPhase;
    let _ = tx
        .send(Ok(StreamEvent::ConnectionPhase {
            phase: ConnectionPhase::SendingRequest,
        }))
        .await;
    let connect_start = std::time::Instant::now();
    let stream_idle_timeout = jcode_base::provider::stream_idle_timeout();

    let mut req = apply_kimi_coding_agent_headers(
        auth.apply(
            client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept-Encoding", "identity"),
        )
        .await?,
        &api_base,
        Some(&model),
    );

    if send_openrouter_headers {
        // Keep the response metadata needed for routing/cost receipts, but do
        // not send optional application-attribution headers to the provider.
        req = req.header("X-OpenRouter-Metadata", "enabled");
    }
    req = apply_opencode_session_header(req, &api_base, conversation_id);

    let response = jcode_provider_core::transport::send_with_initial_response_timeout(
        req.json(&request),
        stream_idle_timeout,
    )
    .await
    .with_context(|| {
        let hint = local_endpoint_troubleshooting_hint(&api_base, &model);
        format!(
            "Failed to send OpenAI-compatible chat request\n  endpoint: {}\n  model: {}\n  auth: {}\n{}",
            url,
            model,
            auth.label(),
            hint
        )
    })?;

    let connect_ms = connect_start.elapsed().as_millis();
    jcode_base::logging::info(&format!(
        "HTTP connection established in {}ms (status={})",
        connect_ms,
        response.status()
    ));

    if !response.status().is_success() {
        let status = response.status();
        let retry_after = jcode_provider_core::retry_after::retry_after(response.headers());
        let body = jcode_base::util::http_error_body(response, "HTTP error").await;
        // A spent quota is not a connectivity problem, and telling the user to
        // check DNS sends them after the wrong thing. The generic hint stays
        // for everything it actually describes.
        let hint = if status.as_u16() == 429 && rate_limit_window_is_exhausted(&body) {
            "Hint: this provider's usage allowance is spent for the current \
             window, so retrying will not help until it resets. Switch provider \
             or model (`--provider`/`--model`, or `/model` in a session), or \
             wait for the window named in the response above."
        } else {
            local_endpoint_troubleshooting_hint(&api_base, &model)
        };
        return Err(jcode_provider_core::retry_after::error_with_retry_after(
            format!(
                "OpenAI-compatible chat request failed\n  endpoint: {}\n  model: {}\n  auth: {}\n  status: {}\n  response: {}\n{}",
                url,
                model,
                auth.label(),
                status,
                body,
                hint
            ),
            retry_after,
        ));
    }

    let _ = tx
        .send(Ok(StreamEvent::ConnectionPhase {
            phase: ConnectionPhase::WaitingForResponse,
        }))
        .await;

    let mut stream = OpenRouterStream::new(response.bytes_stream(), model.clone(), provider_pin);

    // Idle timeout between streamed chunks. Configurable so slow reasoning
    // models (e.g. DeepSeek) that think silently for minutes before emitting
    // tokens don't trip a premature timeout (issue #196). Resolved from
    // `[provider] stream_idle_timeout_secs` / `JCODE_STREAM_IDLE_TIMEOUT_SECS`,
    // defaulting to 180s. Shared with the native provider paths (issue #434).
    let idle_timeout_secs = stream_idle_timeout.as_secs();

    loop {
        let event = match tokio::time::timeout(stream_idle_timeout, stream.next()).await {
            Ok(Some(Ok(event))) => event,
            Ok(Some(Err(e))) => anyhow::bail!(
                "OpenAI-compatible stream error\n  endpoint: {}\n  model: {}\n  auth: {}\n  error: {}",
                url,
                model,
                auth.label(),
                e
            ),
            Ok(None) => break, // stream ended normally
            Err(_) => {
                jcode_base::logging::warn(&format!(
                    "OpenRouter SSE stream timed out (no data for {}s)",
                    idle_timeout_secs
                ));
                anyhow::bail!(
                    "OpenAI-compatible stream timeout\n  endpoint: {}\n  model: {}\n  auth: {}\n  timeout: no data received for {} seconds\n{}",
                    url,
                    model,
                    auth.label(),
                    idle_timeout_secs,
                    local_endpoint_troubleshooting_hint(&api_base, &model)
                );
            }
        };
        if tx.send(Ok(event)).await.is_err() {
            return Ok(());
        }
    }

    Ok(())
}

/// Extract the HTTP status code reported in a formatted provider error string.
///
/// Error strings produced in this module embed the status as `status: <code>`
/// (e.g. `status: 402 Payment Required`). The input may be lowercased before
/// it reaches here, so matching is case-insensitive.
fn parsed_http_status(error_str: &str) -> Option<u16> {
    let lower = error_str.to_ascii_lowercase();
    let idx = lower.find("status:")?;
    let rest = lower[idx + "status:".len()..].trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() == 3 {
        digits.parse().ok()
    } else {
        None
    }
}

/// Whether a 429 body reports an exhausted quota *window* rather than a
/// short-term rate limit.
///
/// A per-second or per-minute limit clears on its own, so backing off is
/// correct. A daily, weekly or monthly allowance does not clear inside any
/// retry budget, so retrying converts an instant, actionable answer ("your
/// weekly quota is spent") into a silent multi-minute hang that ends in a
/// timeout. The user then has no idea why, which is the expensive part.
///
/// Matched on the window words plus the explicit "no quota left" spellings,
/// because the status alone cannot distinguish the two cases and the wording
/// is provider-specific.
fn rate_limit_window_is_exhausted(error_str: &str) -> bool {
    const EXHAUSTED_WINDOWS: &[&str] = &["daily", "weekly", "monthly", "per day", "per week"];
    const EXHAUSTED_QUOTA: &[&str] = &[
        "insufficient_quota",
        "quota exceeded",
        "usage limit exceeded",
        "out of credits",
    ];
    let lowered = error_str.to_ascii_lowercase();
    EXHAUSTED_WINDOWS.iter().any(|w| lowered.contains(w))
        || EXHAUSTED_QUOTA.iter().any(|q| lowered.contains(q))
}

fn is_retryable_error(error_str: &str) -> bool {
    // Explicit non-retryable HTTP statuses take precedence over the loose
    // substring heuristics below. These are deterministic client-side failures
    // (auth, billing, malformed request) where retrying is futile and just
    // burns time/credits. 429 (rate limit) is classified explicitly so it does
    // not depend on provider-specific body wording -- except when the body says
    // the exhausted window is longer than any retry budget, which no amount of
    // backing off can outlast.
    match parsed_http_status(error_str) {
        Some(400 | 401 | 402 | 403 | 404 | 405 | 406 | 422) => return false,
        Some(429) => return !rate_limit_window_is_exhausted(error_str),
        _ => {}
    }

    jcode_provider_core::is_transient_transport_error(error_str)
        || error_str.contains("stream error")
        || error_str.contains("eof")
        || error_str.contains("5")
            && (error_str.contains("50")
                || error_str.contains("502")
                || error_str.contains("503")
                || error_str.contains("504")
                || error_str.contains("internal server error"))
        || error_str.contains("overloaded")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_endpoint_hint_mentions_ollama_actions() {
        let hint = local_endpoint_troubleshooting_hint("http://localhost:11434/v1", "llama3.2");
        assert!(hint.contains("ollama serve"));
        assert!(hint.contains("ollama pull"));
        assert!(hint.contains("--provider ollama"));
    }

    #[test]
    fn local_endpoint_hint_mentions_lm_studio_server() {
        let hint = local_endpoint_troubleshooting_hint("http://127.0.0.1:1234/v1", "local-model");
        assert!(hint.contains("LM Studio"));
        assert!(hint.contains("Local Server"));
        assert!(hint.contains("/v1/models"));
    }

    #[test]
    fn parsed_http_status_extracts_code() {
        assert_eq!(
            parsed_http_status("status: 402 payment required"),
            Some(402)
        );
        assert_eq!(parsed_http_status("  status:404 not found"), Some(404));
        assert_eq!(parsed_http_status("no status here"), None);
        // Embedded numbers elsewhere must not be misread as a status.
        assert_eq!(parsed_http_status("you requested 65536 tokens"), None);
    }

    /// A 429 that reports an exhausted *quota window* must not be retried.
    ///
    /// Measured against the live OpenCode Go endpoint: it answers
    /// `GoUsageLimitError` with `"limitName":"weekly"` in about 0.2s. With
    /// `max_retries = 8` and a 30s backoff cap, retrying turned that instant,
    /// actionable answer into a multi-minute silent hang that ended in a
    /// timeout, so the user never learned their weekly quota was spent.
    ///
    /// The distinction is the window, not the status: a per-second or
    /// per-minute limit does clear on its own and is still retried below.
    #[test]
    fn exhausted_quota_window_is_not_retryable() {
        for body in [
            r#"status: 429 too many requests
  response: {"type":"error","error":{"type":"GoUsageLimitError","message":"Go usage limit exceeded"},"metadata":{"workspace":"wrk_01","limitName":"weekly"}}"#,
            r#"status: 429 too many requests
  response: {"error":{"message":"monthly quota exceeded"}}"#,
            r#"status: 429 too many requests
  response: {"error":{"message":"You have exceeded your daily limit"}}"#,
            r#"status: 429 too many requests
  response: {"error":{"message":"insufficient_quota"}}"#,
        ] {
            assert!(
                !is_retryable_error(body),
                "an exhausted quota window must fail fast: {body}"
            );
        }
    }

    /// A short-window rate limit still retries: it clears on its own, which is
    /// exactly what the backoff is for.
    #[test]
    fn short_window_rate_limit_is_still_retryable() {
        for body in [
            "status: 429 too many requests\n  response: {\"error\":{\"message\":\"rate limit exceeded, retry in 2s\"}}",
            "status: 429 too many requests\n  response: {\"error\":{\"message\":\"too many requests per minute\"}}",
            "status: 429 too many requests",
        ] {
            assert!(
                is_retryable_error(body),
                "a transient rate limit must still retry: {body}"
            );
        }
    }

    #[test]
    fn payment_required_is_not_retryable() {
        let err = "openai-compatible chat request failed\n  endpoint: \
            https://openrouter.ai/api/v1/chat/completions\n  model: openai/gpt-5.4\n  \
            auth: openrouter_api_key\n  status: 402 payment required\n  response: \
            {\"error\":{\"message\":\"this request requires more credits, or fewer \
            max_tokens. you requested up to 65536 tokens, but can only afford 34424\"}}";
        assert!(!is_retryable_error(err));
    }

    #[test]
    fn client_errors_are_not_retryable() {
        for status in [400u16, 401, 402, 403, 404, 405, 406, 422] {
            let err = format!("chat request failed\n  status: {status} client error");
            assert!(
                !is_retryable_error(&err),
                "status {status} should not be retryable"
            );
        }
    }

    #[test]
    fn server_errors_remain_retryable() {
        assert!(is_retryable_error(
            "chat request failed\n  status: 503 service unavailable"
        ));
        assert!(is_retryable_error(
            "chat request failed\n  status: 500 internal server error"
        ));
        // Provider overload messages should still be retried.
        assert!(is_retryable_error("overloaded"));
    }

    #[test]
    fn http_429_is_retryable_without_rate_limit_words_in_body() {
        assert!(is_retryable_error(
            "chat request failed\n  status: 429 unknown\n  response: {}"
        ));
    }
}
