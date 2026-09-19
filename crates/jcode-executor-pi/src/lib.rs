//! No-tools Pi RPC executor adapter (P4 of
//! `docs/HARNESS_LOOP_ARCHITECTURE.md`).
//!
//! Runs an already-admitted [`FrozenAttempt`] on a **tool-disabled**
//! `pi --mode rpc` process and returns a harness-generated [`Receipt`]. The
//! adapter never enables tools: it always passes `--no-tools`, which is the
//! safe default under every D1 containment option. A tool-enabled Pi must not
//! be built here until an OS-enforced sandbox is chosen.
//!
//! The receipt is written by this adapter from bytes it observed; pi is the
//! worker and cannot forge it. As with `attempt_caller`, this bounds what the
//! harness initiates, not the account (D2).

mod protocol;

pub use protocol::PiProtocol;

use chrono::Utc;
use jcode_attempt_types::{FrozenAttempt, Receipt, ReceiptKind, Usage, validate_receipt_for_gate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Polled cancellation signal, same shape as the single-send caller's.
pub type CancelSignal = Arc<AtomicBool>;

/// How to launch pi. `provider`/`model` are pi-native identifiers; for a route
/// promoted to pi, `model` must equal the attempt's frozen `model_exact`.
#[derive(Debug, Clone)]
pub struct PiConfig {
    pub binary: PathBuf,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PiOutcome {
    Completed { text: String },
    Failed { message: String },
    DeadlineExceeded,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiResult {
    pub outcome: PiOutcome,
    pub receipt: Receipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PiError {
    ModelMismatch { frozen: String, configured: String },
    Spawn(String),
    Protocol(String),
    Io(String),
    ReceiptInvalid(String),
}

impl std::fmt::Display for PiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PiError::ModelMismatch { frozen, configured } => write!(
                f,
                "frozen model `{frozen}` does not match configured pi model `{configured}`"
            ),
            PiError::Spawn(e) => write!(f, "failed to spawn pi: {e}"),
            PiError::Protocol(e) => write!(f, "pi protocol error: {e}"),
            PiError::Io(e) => write!(f, "pi adapter I/O error: {e}"),
            PiError::ReceiptInvalid(e) => write!(f, "generated receipt invalid: {e}"),
        }
    }
}

impl std::error::Error for PiError {}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Build the pi argv for an admitted attempt. Always `--no-tools`.
pub fn build_argv(config: &PiConfig, attempt: &FrozenAttempt) -> Result<Vec<String>, PiError> {
    let record = attempt.record();
    if let Some(model) = &config.model
        && model != &record.model_exact
    {
        return Err(PiError::ModelMismatch {
            frozen: record.model_exact.clone(),
            configured: model.clone(),
        });
    }
    let mut argv = vec![
        config.binary.to_string_lossy().into_owned(),
        "--mode".into(),
        "rpc".into(),
        "--no-session".into(),
        "--no-tools".into(),
    ];
    if let Some(provider) = &config.provider {
        argv.push("--provider".into());
        argv.push(provider.clone());
    }
    if let Some(model) = &config.model {
        argv.push("--model".into());
        argv.push(model.clone());
    }
    Ok(argv)
}

fn temp_stderr_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "jcode-pi-stderr-{}-{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Run one tool-disabled pi attempt and return its outcome plus a receipt.
pub async fn run_pi_attempt(
    config: &PiConfig,
    attempt: &FrozenAttempt,
    prompt: &str,
    deadline: Duration,
    cancel: Option<CancelSignal>,
) -> Result<PiResult, PiError> {
    let record = attempt.record();
    let argv = build_argv(config, attempt)?;
    let argv_hash = sha256_hex(argv.join("\0").as_bytes());
    let started = Utc::now();

    let stderr_path = temp_stderr_path();
    let stderr_file =
        std::fs::File::create(&stderr_path).map_err(|e| PiError::Io(e.to_string()))?;
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(&config.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file))
        // The adapter is the trusted side; record telemetry as off so the
        // receipt is checkable even though pi sends none itself.
        .env("DO_NOT_TRACK", "1")
        .env("PI_SKIP_VERSION_CHECK", "1")
        .env("JCODE_NO_TELEMETRY", "1")
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(|e| PiError::Spawn(e.to_string()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| PiError::Spawn("no stdin pipe".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PiError::Spawn("no stdout pipe".into()))?;
    let mut lines = BufReader::new(stdout).lines();

    let prompt_line = serde_json::json!({ "type": "prompt", "message": prompt }).to_string();
    stdin
        .write_all(prompt_line.as_bytes())
        .await
        .map_err(|e| PiError::Io(e.to_string()))?;
    stdin
        .write_all(b"\n")
        .await
        .map_err(|e| PiError::Io(e.to_string()))?;
    stdin
        .flush()
        .await
        .map_err(|e| PiError::Io(e.to_string()))?;

    let deadline_at = tokio::time::Instant::now() + deadline;
    let mut proto = PiProtocol::default();
    let mut outcome: Option<PiOutcome> = None;

    loop {
        if cancel
            .as_ref()
            .is_some_and(|signal| signal.load(Ordering::Relaxed))
        {
            outcome = Some(PiOutcome::Cancelled);
            break;
        }
        let remaining = deadline_at.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            outcome = Some(PiOutcome::DeadlineExceeded);
            break;
        }
        match tokio::time::timeout(remaining, lines.next_line()).await {
            Err(_) => {
                outcome = Some(PiOutcome::DeadlineExceeded);
                break;
            }
            Ok(Err(err)) => {
                outcome = Some(PiOutcome::Failed {
                    message: err.to_string(),
                });
                break;
            }
            Ok(Ok(None)) => {
                if proto.settled {
                    break;
                }
                outcome = Some(PiOutcome::Failed {
                    message: "pi exited before agent_settled".into(),
                });
                break;
            }
            Ok(Ok(Some(line))) => {
                if let Err(err) = proto.ingest_line(&line) {
                    outcome = Some(PiOutcome::Failed { message: err });
                    break;
                }
                if let Some(err) = proto.error.clone() {
                    outcome = Some(PiOutcome::Failed { message: err });
                    break;
                }
                if proto.settled {
                    break;
                }
            }
        }
    }

    // Best-effort stats request so the receipt can cite cost when pi reports it.
    if outcome.is_none() && proto.settled {
        let stats = serde_json::json!({ "type": "get_session_stats" }).to_string();
        let wrote = async {
            stdin.write_all(stats.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await
        }
        .await
        .is_ok();
        if wrote {
            let stats_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                let remaining =
                    stats_deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match tokio::time::timeout(remaining, lines.next_line()).await {
                    Err(_) | Ok(Err(_)) | Ok(Ok(None)) => break,
                    Ok(Ok(Some(line))) => {
                        let _ = proto.ingest_line(&line);
                        if proto.stats_seen {
                            break;
                        }
                    }
                }
            }
        }
    }

    let _ = stdin.shutdown().await;
    drop(stdin);
    let _ = child.start_kill();
    let _ = child.wait().await;

    let stderr_bytes = std::fs::read(&stderr_path).unwrap_or_default();
    let _ = std::fs::remove_file(&stderr_path);

    let outcome = outcome.unwrap_or_else(|| {
        if let Some(err) = proto.error.clone() {
            PiOutcome::Failed { message: err }
        } else {
            PiOutcome::Completed {
                text: proto.final_text(),
            }
        }
    });

    let finished = Utc::now();
    let text = match &outcome {
        PiOutcome::Completed { text } => text.clone(),
        _ => proto.final_text(),
    };
    let exit_code = match &outcome {
        PiOutcome::Completed { .. } => 0,
        PiOutcome::Failed { .. } => 1,
        PiOutcome::DeadlineExceeded => 124,
        PiOutcome::Cancelled => 130,
    };
    let usage: Usage = proto.usage();
    let receipt = Receipt {
        attempt_id: attempt.attempt_id().to_string(),
        kind: ReceiptKind::ModelCall,
        cmd: argv.join(" "),
        argv_hash,
        cwd: config.cwd.to_string_lossy().into_owned(),
        exit_code: Some(exit_code),
        stdout_sha256: sha256_hex(text.as_bytes()),
        stderr_sha256: sha256_hex(&stderr_bytes),
        started,
        finished,
        binary_id: format!(
            "pi:{}",
            config
                .model
                .as_deref()
                .unwrap_or(record.model_exact.as_str())
        ),
        usage: Some(usage),
        effective_telemetry: [("DO_NOT_TRACK".to_string(), "1".to_string())]
            .into_iter()
            .collect(),
        task_type: None,
    };
    validate_receipt_for_gate(&receipt, attempt)
        .map_err(|e| PiError::ReceiptInvalid(e.to_string()))?;
    Ok(PiResult { outcome, receipt })
}

#[cfg(all(test, unix))]
#[path = "tests.rs"]
mod tests;
