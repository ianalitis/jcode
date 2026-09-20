//! Trusted product-path caller for one frozen attempt (P3 of
//! `docs/HARNESS_LOOP_ARCHITECTURE.md`).
//!
//! Turns a [`FrozenAttempt`] plus a trusted expected request body into exactly
//! one guarded send through
//! [`OpenRouterProvider::complete_single_send_with_expected_final_request`],
//! bounded by the attempt's deadline and a caller-supplied cancellation
//! signal, and produces a harness-generated [`Receipt`] whose digests come
//! from the bytes actually observed. The worker never writes the receipt.
//!
//! Spend: a [`LocalLedger`] reserves the attempt's `max_micro_usd` before the
//! send and settles it afterwards. A crash or ambiguous outcome leaves the
//! reservation held until an explicit reconcile. This bounds what this
//! harness *initiates*; it is not an account-side cap (operator decision D2).

use crate::OpenRouterProvider;
use chrono::Utc;
use futures::StreamExt;
use jcode_attempt_types::{
    FrozenAttempt, Receipt, ReceiptKind, RouteClass, Usage, validate_receipt_for_gate,
};
use jcode_message_types::{Message, StreamEvent, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

pub use jcode_attempt_types::{LedgerError, LocalLedger, Reservation, ReservationState};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

// ---------------------------------------------------------------------------
// Attempt outcome
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AttemptOutcome {
    /// Stream ended normally. Text is the concatenated deltas.
    Completed { text: String },
    /// Provider or transport reported an error. Exactly one send happened
    /// (or zero if the guard rejected before send).
    Failed { message: String, sent: bool },
    /// Deadline elapsed mid-stream. Exposure retained as ambiguous.
    DeadlineExceeded,
    /// Caller cancelled mid-stream. Exposure retained as ambiguous.
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptResult {
    pub outcome: AttemptOutcome,
    pub receipt: Receipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerError {
    NotMeteredRoute(RouteClass),
    ModelMismatch {
        frozen: String,
        provider: String,
    },
    Ledger(LedgerError),
    /// The receipt the caller generated failed its own validator. Should be
    /// unreachable; surfaced rather than swallowed.
    ReceiptInvalid(String),
}

impl std::fmt::Display for CallerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallerError::NotMeteredRoute(r) => {
                write!(f, "attempt caller only serves metered routes, got {r:?}")
            }
            CallerError::ModelMismatch { frozen, provider } => write!(
                f,
                "frozen model `{frozen}` does not match provider model `{provider}`"
            ),
            CallerError::Ledger(e) => write!(f, "{e}"),
            CallerError::ReceiptInvalid(e) => write!(f, "generated receipt invalid: {e}"),
        }
    }
}

impl std::error::Error for CallerError {}

/// A cancellation signal the caller polls before each stream read. The
/// provider issues its single HTTP send on a spawned task, so cancellation
/// cannot guarantee zero sends once the call has started; it guarantees no
/// further consumption and an `Ambiguous` reservation.
pub type CancelSignal = Arc<std::sync::atomic::AtomicBool>;

/// Run exactly one guarded send for a frozen attempt.
///
/// `expected_final_request` and `expected_destination` are trusted values
/// from captain-owned state; the seam refuses to send if the provider would
/// build anything else. `messages`, `tools` and `system` must be the inputs
/// from which that expected body was derived.
#[allow(clippy::too_many_arguments)]
pub async fn run_frozen_attempt(
    provider: &OpenRouterProvider,
    attempt: &FrozenAttempt,
    ledger: &LocalLedger,
    expected_final_request: Value,
    expected_destination: &str,
    messages: &[Message],
    tools: &[ToolDefinition],
    system: &str,
    cancel: Option<CancelSignal>,
) -> Result<AttemptResult, CallerError> {
    let record = attempt.record();
    if record.route_class != RouteClass::MeteredRemote {
        return Err(CallerError::NotMeteredRoute(record.route_class));
    }
    let provider_model = provider.model.read().await.clone();
    if provider_model != record.model_exact {
        return Err(CallerError::ModelMismatch {
            frozen: record.model_exact.clone(),
            provider: provider_model,
        });
    }
    ledger
        .reserve(attempt.attempt_id(), record.budget.max_micro_usd)
        .map_err(CallerError::Ledger)?;

    let request_bytes = serde_json::to_vec(&expected_final_request).unwrap_or_default();
    let argv_hash = sha256_hex(&request_bytes);
    let started = Utc::now();
    let deadline_at =
        tokio::time::Instant::now().checked_add(Duration::from_secs(record.deadline_secs));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut usage = Usage::default();
    let mut sent = false;
    let mut served_model: Option<String> = None;
    let mut task_type: Option<String> = None;

    let opened = match deadline_at {
        Some(deadline_at) => {
            tokio::time::timeout_at(
                deadline_at,
                provider.complete_single_send_with_expected_final_request(
                    expected_final_request,
                    expected_destination,
                    messages,
                    tools,
                    system,
                    None,
                ),
            )
            .await
        }
        None => Ok(provider
            .complete_single_send_with_expected_final_request(
                expected_final_request,
                expected_destination,
                messages,
                tools,
                system,
                None,
            )
            .await),
    };
    let outcome = match opened {
        Err(_) => AttemptOutcome::DeadlineExceeded,
        Ok(Err(err)) => {
            // Guard rejected before any send (body/destination mismatch).
            let message = err.to_string();
            stderr.extend_from_slice(message.as_bytes());
            AttemptOutcome::Failed {
                message,
                sent: false,
            }
        }
        Ok(Ok(mut stream)) => {
            sent = true;
            let mut outcome = None;
            loop {
                if cancel
                    .as_ref()
                    .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                {
                    outcome = Some(AttemptOutcome::Cancelled);
                    break;
                }
                if deadline_at.is_some_and(|deadline| tokio::time::Instant::now() >= deadline) {
                    outcome = Some(AttemptOutcome::DeadlineExceeded);
                    break;
                }
                let next = match deadline_at {
                    Some(deadline_at) => tokio::time::timeout_at(deadline_at, stream.next()).await,
                    None => Ok(stream.next().await),
                };
                match next {
                    Err(_) => {
                        outcome = Some(AttemptOutcome::DeadlineExceeded);
                        break;
                    }
                    Ok(None) => break,
                    Ok(Some(Ok(StreamEvent::TextDelta(t)))) => {
                        stdout.extend_from_slice(t.as_bytes());
                    }
                    Ok(Some(Ok(StreamEvent::TokenUsage {
                        input_tokens,
                        output_tokens,
                        ..
                    }))) => {
                        usage.input_tokens = input_tokens.unwrap_or(usage.input_tokens);
                        usage.output_tokens = output_tokens.unwrap_or(usage.output_tokens);
                    }
                    Ok(Some(Ok(StreamEvent::ServedModel {
                        model,
                        micro_usd,
                        task_type: label,
                    }))) => {
                        served_model = Some(model);
                        if micro_usd.is_some() {
                            usage.micro_usd = micro_usd;
                        }
                        if label.is_some() {
                            task_type = label;
                        }
                    }
                    Ok(Some(Ok(StreamEvent::Error { message, .. }))) => {
                        stderr.extend_from_slice(message.as_bytes());
                        outcome = Some(AttemptOutcome::Failed {
                            message,
                            sent: true,
                        });
                        break;
                    }
                    Ok(Some(Ok(_))) => {}
                    Ok(Some(Err(err))) => {
                        let message = err.to_string();
                        stderr.extend_from_slice(message.as_bytes());
                        outcome = Some(AttemptOutcome::Failed {
                            message,
                            sent: true,
                        });
                        break;
                    }
                }
            }
            outcome.unwrap_or_else(|| AttemptOutcome::Completed {
                text: String::from_utf8_lossy(&stdout).into_owned(),
            })
        }
    };

    // Settle the ledger. Unknown outcomes keep exposure.
    let settle = match &outcome {
        AttemptOutcome::Completed { .. } => {
            Some(usage.micro_usd.unwrap_or(record.budget.max_micro_usd))
        }
        AttemptOutcome::Failed { sent: false, .. } => Some(0),
        AttemptOutcome::Failed { sent: true, .. } => None,
        AttemptOutcome::DeadlineExceeded | AttemptOutcome::Cancelled => None,
    };
    match settle {
        Some(amount) => ledger
            .settle(attempt.attempt_id(), amount)
            .map_err(CallerError::Ledger)?,
        None => ledger
            .mark_ambiguous(attempt.attempt_id())
            .map_err(CallerError::Ledger)?,
    }

    let finished = Utc::now();
    let receipt = Receipt {
        attempt_id: attempt.attempt_id().to_string(),
        kind: ReceiptKind::ModelCall,
        cmd: format!("single-send {} {}", record.provider, record.model_exact),
        argv_hash,
        cwd: expected_destination.to_string(),
        exit_code: Some(match &outcome {
            AttemptOutcome::Completed { .. } => 0,
            AttemptOutcome::Failed { sent: false, .. } => 2,
            AttemptOutcome::Failed { sent: true, .. } => 1,
            AttemptOutcome::DeadlineExceeded => 124,
            AttemptOutcome::Cancelled => 130,
        }),
        stdout_sha256: sha256_hex(&stdout),
        stderr_sha256: sha256_hex(&stderr),
        started,
        finished,
        // The receipt names what actually answered. For a dynamic router that
        // is the resolved slug; the gate checks it against the banned families.
        binary_id: format!(
            "{}:{}",
            record.provider,
            served_model.as_deref().unwrap_or(&record.model_exact)
        ),
        usage: if sent { Some(usage) } else { None },
        effective_telemetry: BTreeMap::new(),
        task_type,
    };
    validate_receipt_for_gate(&receipt, attempt)
        .map_err(|e| CallerError::ReceiptInvalid(e.to_string()))?;
    Ok(AttemptResult { outcome, receipt })
}

#[cfg(test)]
#[path = "attempt_caller_tests.rs"]
mod tests;
