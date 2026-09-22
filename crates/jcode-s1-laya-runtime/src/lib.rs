//! The W3 local decision arm: a laya checkpoint behind the W1 decision contract.
//!
//! This crate is the *process boundary* and nothing else. The child
//! (`python/laya_arm_child.py`) owns the forward pass; this side owns the
//! contract, the wall-clock bound, the environment the child is allowed to see,
//! and the fail-closed policy. A child that crashes, times out, answers
//! unparseably, or violates the contract is an **abstention with a recorded
//! reason**, never a default-allow.
//!
//! Shape: one short-lived child per batch. The child is started on the first
//! decision and reused for the rest of the batch, so a 10-fixture run pays one
//! model load rather than ten, and the process ends when the arm is dropped or
//! [`LayaArm::finish`] is called. There is no daemon, no sidecar and no resident
//! server, matching the packet's "no second scheduler" constraint.
//!
//! Two properties this crate asserts rather than trusts:
//!
//! * **No credentials reach the child.** It is spawned with a cleared
//!   environment plus a short allowlist, and with the Hub's offline switches set
//!   by the parent, so "zero network" does not depend on the child honouring a
//!   flag. The interpreter still needs `HOME` to find a warm cache.
//! * **A result the contract rejects is not passed on.** The child's answer is
//!   validated before it is returned; a violation becomes a typed error, and the
//!   count of those is reported next to the scorecard instead of being folded
//!   into it.

use jcode_s1_eval::{
    DecisionArm, DecisionFixture, DecisionRequest, DecisionResult, DecisionScorecard,
    score_decisions, validate_decision,
};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The proposed ceiling from the W3 plan: peak RSS for one child, under the
/// local lane's 20 GB resident budget on a 36 GB machine.
pub const DEFAULT_MAX_RSS_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// How long a model load may take before the child is killed.
pub const DEFAULT_LOAD_TIMEOUT_SECS: u64 = 600;
/// How long one decision may take once the child is ready.
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 120;

/// The child inherits only these. Everything else is dropped, including every
/// credential the parent may hold and every proxy variable that could carry
/// traffic out.
pub const INHERITED_ENV: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "TERM", "USER"];

/// Variables set by the parent so the child cannot reach the network even if it
/// ignored its own `--offline` flag.
pub const FORCED_ENV: &[(&str, &str)] = &[
    ("HF_HUB_OFFLINE", "1"),
    ("TRANSFORMERS_OFFLINE", "1"),
    ("HF_HUB_DISABLE_TELEMETRY", "1"),
    ("DO_NOT_TRACK", "1"),
];

/// Names that appear in a child's environment even when the parent passes none,
/// because the OS or the interpreter's startup code sets them. Measured, not
/// assumed: an empty environment still yields exactly these two on this host.
/// They carry no credential and no reachability, so the allowlist test tolerates
/// them while still failing on anything else.
pub const OS_INJECTED_ENV: &[&str] = &["__CF_USER_TEXT_ENCODING", "LC_CTYPE"];

/// Whether an environment variable name looks like it carries a credential or a
/// way out to the network. The boundary test fails on any such name regardless
/// of the allowlist, so widening the allowlist later cannot quietly re-admit one.
pub fn env_name_looks_like_a_credential(name: &str) -> bool {
    const SHAPES: &[&str] = &["TOKEN", "KEY", "SECRET", "PASSWORD", "CREDENTIAL", "PROXY"];
    let upper = name.to_uppercase();
    SHAPES.iter().any(|shape| upper.contains(shape))
}

/// How many child stderr lines are kept for diagnostics.
const STDERR_KEEP: usize = 40;

/// Where the child lives, and how it is run.
#[derive(Debug, Clone)]
pub struct LayaArmConfig {
    /// The interpreter that has `laya` + `torch` installed. Defaults to
    /// `$JCODE_LAYA_PYTHON`, then `python3`.
    pub python: PathBuf,
    /// The child script. Defaults to this crate's bundled copy.
    pub script: PathBuf,
    /// Local checkpoint directory, or a Hub id for a warm cache.
    pub model: String,
    /// `None` lets laya choose (MPS on this host).
    pub device: Option<String>,
    pub load_timeout: Duration,
    pub request_timeout: Duration,
    pub max_rss_bytes: u64,
    /// Extra child arguments, after the standard ones. Tests use this to select
    /// a stub behaviour; an operator can use it for a `--device` variant.
    pub extra_args: Vec<String>,
    /// Extra environment for the child, used by tests to point at a stub.
    pub extra_env: Vec<(String, String)>,
}

impl Default for LayaArmConfig {
    fn default() -> Self {
        Self {
            python: PathBuf::from("python3"),
            script: default_script_path(),
            model: "convaiinnovations/laya".to_string(),
            device: None,
            load_timeout: Duration::from_secs(DEFAULT_LOAD_TIMEOUT_SECS),
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
            max_rss_bytes: DEFAULT_MAX_RSS_BYTES,
            extra_args: Vec::new(),
            extra_env: Vec::new(),
        }
    }
}

impl LayaArmConfig {
    /// Configuration from the environment, so a receipt can name what ran.
    ///
    /// * `JCODE_LAYA_PYTHON` - interpreter with the arm's dependencies
    /// * `JCODE_LAYA_SCRIPT` - override the child script (tests)
    /// * `JCODE_LAYA_MODEL` - checkpoint directory or Hub id
    /// * `JCODE_LAYA_DEVICE` - `cpu`, `mps`, or unset for laya's choice
    /// * `JCODE_LAYA_MAX_RSS_MB` - footprint ceiling for one child
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(value) = std::env::var("JCODE_LAYA_PYTHON") {
            config.python = PathBuf::from(value);
        }
        if let Ok(value) = std::env::var("JCODE_LAYA_SCRIPT") {
            config.script = PathBuf::from(value);
        }
        if let Ok(value) = std::env::var("JCODE_LAYA_MODEL") {
            config.model = value;
        }
        if let Ok(value) = std::env::var("JCODE_LAYA_DEVICE")
            && !value.trim().is_empty()
        {
            config.device = Some(value);
        }
        if let Ok(value) = std::env::var("JCODE_LAYA_MAX_RSS_MB")
            && let Ok(megabytes) = value.trim().parse::<u64>()
        {
            config.max_rss_bytes = megabytes.saturating_mul(1024 * 1024);
        }
        config
    }

    /// The child command, with the environment it is allowed to see.
    pub fn child_command(&self) -> Command {
        let mut command = Command::new(&self.python);
        command
            .arg(&self.script)
            .arg("--model")
            .arg(&self.model)
            .arg("--offline");
        if let Some(device) = &self.device {
            command.arg("--device").arg(device);
        }
        for argument in &self.extra_args {
            command.arg(argument);
        }
        command.env_clear();
        for name in INHERITED_ENV {
            if let Ok(value) = std::env::var(name) {
                command.env(name, value);
            }
        }
        for (name, value) in FORCED_ENV {
            command.env(name, value);
        }
        for (name, value) in &self.extra_env {
            command.env(name, value);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

/// The bundled child script's path.
pub fn default_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("python")
        .join("laya_arm_child.py")
}

/// What the child reported about its own load.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadyInfo {
    pub device: String,
    pub model: String,
    pub load_ms: u64,
}

/// The child's own end-of-batch accounting. `peak_rss_bytes` is its high-water
/// RSS as measured by the process that owns the memory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChildSummary {
    pub requests: usize,
    pub errors: usize,
    pub wall_ms: u64,
    pub load_ms: u64,
    pub input_tokens: u64,
    pub peak_rss_bytes: u64,
    pub device: String,
    pub model: String,
}

/// Everything that can go wrong across the boundary. Each variant is a distinct
/// fail-closed outcome, not a retry hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayaArmError {
    /// The interpreter or script could not be started.
    Spawn(String),
    /// The child never reported itself ready.
    LoadTimeout { timeout_ms: u64 },
    /// The child died or misbehaved before it was ready.
    LoadFailed(String),
    /// A decision did not arrive inside the wall-clock bound.
    RequestTimeout { request_id: String, timeout_ms: u64 },
    /// The child exited or its stream ended mid-batch.
    ChildGone(String),
    /// A line was not a shape this protocol admits.
    Protocol(String),
    /// The child itself reported it could not answer this request.
    ChildError { request_id: String, message: String },
    /// The child's answer did not satisfy the W1 contract.
    ContractViolation { request_id: String, detail: String },
    /// Measured peak RSS was above the configured ceiling.
    RssCeilingExceeded { peak_bytes: u64, ceiling_bytes: u64 },
}

impl LayaArmError {
    /// A short, stable token for a receipt or a backend label.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Spawn(_) => "spawn",
            Self::LoadTimeout { .. } => "load-timeout",
            Self::LoadFailed(_) => "load-failed",
            Self::RequestTimeout { .. } => "timeout",
            Self::ChildGone(_) => "child-gone",
            Self::Protocol(_) => "protocol",
            Self::ChildError { .. } => "child-error",
            Self::ContractViolation { .. } => "contract-violation",
            Self::RssCeilingExceeded { .. } => "rss-ceiling",
        }
    }
}

impl std::fmt::Display for LayaArmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(detail) => write!(f, "could not start the laya child: {detail}"),
            Self::LoadTimeout { timeout_ms } => {
                write!(f, "the laya child was not ready within {timeout_ms} ms")
            }
            Self::LoadFailed(detail) => write!(f, "the laya child failed to load: {detail}"),
            Self::RequestTimeout {
                request_id,
                timeout_ms,
            } => write!(
                f,
                "decision {request_id} did not arrive within {timeout_ms} ms"
            ),
            Self::ChildGone(detail) => write!(f, "the laya child went away: {detail}"),
            Self::Protocol(detail) => write!(f, "protocol violation: {detail}"),
            Self::ChildError {
                request_id,
                message,
            } => {
                write!(f, "the laya child could not answer {request_id}: {message}")
            }
            Self::ContractViolation { request_id, detail } => {
                write!(
                    f,
                    "the result for {request_id} violates the contract: {detail}"
                )
            }
            Self::RssCeilingExceeded {
                peak_bytes,
                ceiling_bytes,
            } => write!(
                f,
                "peak RSS {peak_bytes} bytes exceeds the ceiling of {ceiling_bytes} bytes"
            ),
        }
    }
}

impl std::error::Error for LayaArmError {}

/// One child process and the channels that talk to it.
#[derive(Debug)]
struct ChildSession {
    child: Child,
    /// `None` once closed, which is how the child is told to summarise and exit.
    stdin: Option<ChildStdin>,
    lines: Receiver<std::io::Result<String>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    pid: u32,
}

impl ChildSession {
    fn spawn(config: &LayaArmConfig) -> Result<Self, LayaArmError> {
        let mut child = config.child_command().spawn().map_err(|error| {
            LayaArmError::Spawn(format!("{}: {error}", config.python.display()))
        })?;
        let pid = child.id();
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LayaArmError::Spawn("no stdin pipe".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LayaArmError::Spawn("no stdout pipe".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| LayaArmError::Spawn("no stderr pipe".to_string()))?;

        // stdout: one JSON object per line, forwarded to the caller.
        let (sender, lines) = std::sync::mpsc::channel::<std::io::Result<String>>();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });

        // stderr: kept for diagnostics. laya reports calibration warnings here,
        // which is evidence a receipt may need to quote.
        let diagnostics = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_KEEP)));
        let sink = Arc::clone(&diagnostics);
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(mut kept) = sink.lock() {
                    if kept.len() == STDERR_KEEP {
                        kept.pop_front();
                    }
                    kept.push_back(line);
                }
            }
        });

        Ok(Self {
            child,
            stdin: Some(stdin),
            lines,
            stderr: diagnostics,
            pid,
        })
    }

    fn send(&mut self, request: &DecisionRequest) -> Result<(), LayaArmError> {
        let line = serde_json::to_string(request)
            .map_err(|error| LayaArmError::Protocol(format!("unserialisable request: {error}")))?;
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| LayaArmError::ChildGone("the child's stdin is closed".to_string()))?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .and_then(|()| stdin.flush())
            .map_err(|error| LayaArmError::ChildGone(format!("write failed: {error}")))
    }

    /// Read lines until one of them is the answer, or the child goes away.
    fn next_line(&self, timeout: Duration) -> Result<String, LayaArmError> {
        match self.lines.recv_timeout(timeout) {
            Ok(Ok(line)) => Ok(line),
            Ok(Err(error)) => Err(LayaArmError::ChildGone(error.to_string())),
            Err(RecvTimeoutError::Timeout) => Err(LayaArmError::RequestTimeout {
                request_id: String::new(),
                timeout_ms: timeout.as_millis() as u64,
            }),
            Err(RecvTimeoutError::Disconnected) => Err(LayaArmError::ChildGone(
                "the child closed its output".to_string(),
            )),
        }
    }

    /// Wait for the ready line, which is the load boundary.
    fn wait_ready(&mut self, timeout: Duration) -> Result<ReadyInfo, LayaArmError> {
        // The ready line is the child's first line by construction, so anything
        // else is a load failure rather than something to wait through.
        let line = self.next_line(timeout).map_err(|error| match error {
            LayaArmError::RequestTimeout { .. } => LayaArmError::LoadTimeout {
                timeout_ms: timeout.as_millis() as u64,
            },
            other => LayaArmError::LoadFailed(other.to_string()),
        })?;
        match parse_line(&line)? {
            ChildLine::Ready(info) => Ok(info),
            ChildLine::Error { error, .. } => Err(LayaArmError::LoadFailed(error)),
            other => Err(LayaArmError::LoadFailed(format!(
                "unexpected line before ready: {other:?}"
            ))),
        }
    }

    /// Dropping the handle sends EOF, which is the child's signal to summarise
    /// and exit.
    fn close_stdin(&mut self) {
        self.stdin = None;
    }

    fn kill(&mut self) {
        // Best effort by design: the child may already have exited. `wait` reaps
        // whatever is left, so the arm never leaves a zombie behind.
        match self.child.kill() {
            Ok(()) => {}
            // Already gone: this is the expected case after a timeout.
            Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {}
            Err(error) => eprintln!("laya child {} could not be killed: {error}", self.pid),
        }
        match self.child.wait() {
            Ok(_) => {}
            Err(error) => eprintln!("laya child {} could not be reaped: {error}", self.pid),
        }
    }

    fn diagnostics(&self) -> Vec<String> {
        match self.stderr.lock() {
            Ok(kept) => kept.iter().cloned().collect(),
            // A poisoned diagnostics buffer is not worth failing a batch over:
            // the stderr lines are advisory and the evidence is the summary.
            Err(poisoned) => poisoned.into_inner().iter().cloned().collect(),
        }
    }
}

impl Drop for ChildSession {
    fn drop(&mut self) {
        self.kill();
    }
}

/// One line from the child.
#[derive(Debug, Clone, PartialEq)]
enum ChildLine {
    Ready(ReadyInfo),
    Result {
        request_id: String,
        result: Box<DecisionResult>,
    },
    Error {
        request_id: String,
        error: String,
    },
    Summary(Box<ChildSummary>),
    /// A kind this side does not know. Tolerated so a newer child can add
    /// diagnostics without breaking an older parent.
    Unknown(String),
}

fn parse_line(line: &str) -> Result<ChildLine, LayaArmError> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| LayaArmError::Protocol(format!("not JSON ({error}): {line:.120}")))?;
    let kind = value
        .get("kind")
        .and_then(|kind| kind.as_str())
        .ok_or_else(|| LayaArmError::Protocol(format!("line has no kind: {line:.120}")))?
        .to_string();
    let payload = |field: &str| -> Result<serde_json::Value, LayaArmError> {
        value
            .get(field)
            .cloned()
            .ok_or_else(|| LayaArmError::Protocol(format!("{kind} line has no {field}")))
    };
    match kind.as_str() {
        "ready" => Ok(ChildLine::Ready(ReadyInfo {
            device: string_field(&value, "device"),
            model: string_field(&value, "model"),
            load_ms: u64_field(&value, "load_ms"),
        })),
        "result" => {
            let result: DecisionResult = serde_json::from_value(payload("result")?)
                .map_err(|error| LayaArmError::Protocol(format!("unreadable result: {error}")))?;
            Ok(ChildLine::Result {
                request_id: string_field(&value, "request_id"),
                result: Box::new(result),
            })
        }
        "error" => Ok(ChildLine::Error {
            request_id: string_field(&value, "request_id"),
            error: string_field(&value, "error"),
        }),
        "summary" => Ok(ChildLine::Summary(Box::new(ChildSummary {
            requests: u64_field(&value, "requests") as usize,
            errors: u64_field(&value, "errors") as usize,
            wall_ms: u64_field(&value, "wall_ms"),
            load_ms: u64_field(&value, "load_ms"),
            input_tokens: u64_field(&value, "input_tokens"),
            peak_rss_bytes: u64_field(&value, "peak_rss_bytes"),
            device: string_field(&value, "device"),
            model: string_field(&value, "model"),
        }))),
        other => Ok(ChildLine::Unknown(other.to_string())),
    }
}

fn string_field(value: &serde_json::Value, field: &str) -> String {
    match value.get(field).and_then(|inner| inner.as_str()) {
        Some(text) => text.to_string(),
        // A missing or non-string field is reported as empty rather than
        // defaulted away elsewhere: the caller decides what an empty device or
        // model name means, and the receipt shows it.
        None => String::new(),
    }
}

fn u64_field(value: &serde_json::Value, field: &str) -> u64 {
    match value.get(field).and_then(|inner| inner.as_u64()) {
        Some(number) => number,
        None => 0,
    }
}

/// What the arm has learned about the child so far.
#[derive(Debug, Default)]
struct ArmState {
    session: Option<ChildSession>,
    ready: Option<ReadyInfo>,
    summary: Option<ChildSummary>,
    /// Set once the child is known bad: every later decision fails closed rather
    /// than spawning a second child behind the caller's back.
    poisoned: Option<String>,
    /// Results the parent refused before returning them.
    rejected: usize,
    /// How many children this arm has started. One per batch is the design.
    spawns: usize,
    /// The last child's stderr, kept after the process is gone.
    diagnostics: Vec<String>,
}

/// The laya arm behind [`DecisionArm`].
pub struct LayaArm {
    config: LayaArmConfig,
    label: String,
    state: Mutex<ArmState>,
}

impl LayaArm {
    pub fn new(config: LayaArmConfig) -> Self {
        let label = format!("laya-local({})", config.model);
        Self {
            config,
            label,
            state: Mutex::new(ArmState::default()),
        }
    }

    pub fn from_env() -> Self {
        Self::new(LayaArmConfig::from_env())
    }

    pub fn config(&self) -> &LayaArmConfig {
        &self.config
    }

    /// The arm's state guard.
    ///
    /// A poisoned lock means some caller panicked while holding it. Every field
    /// in [`ArmState`] is independently valid, so the arm recovers the data and
    /// carries on rather than failing every later decision, which is what
    /// turning the poison into an empty state would do.
    fn state(&self) -> std::sync::MutexGuard<'_, ArmState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The load report, once the child has started.
    pub fn ready(&self) -> Option<ReadyInfo> {
        self.state().ready.clone()
    }

    /// The child's end-of-batch accounting, once it has exited.
    pub fn summary(&self) -> Option<ChildSummary> {
        self.state().summary.clone()
    }

    /// How many children this arm has started. One per batch is the design; a
    /// test asserts it and a caller can too.
    pub fn spawns(&self) -> usize {
        self.state().spawns
    }

    /// How many results the contract rejected before they reached a caller.
    pub fn rejected(&self) -> usize {
        self.state().rejected
    }

    /// The child's stderr, kept for diagnostics.
    pub fn diagnostics(&self) -> Vec<String> {
        self.state().diagnostics.clone()
    }

    /// The child's pid while it is alive, so a measurement can attribute RSS to
    /// the process it actually belongs to.
    pub fn child_pid(&self) -> Option<u32> {
        self.state().session.as_ref().map(|session| session.pid)
    }

    /// One decision, with the failure visible. Validates the child's answer
    /// against the contract before returning it.
    pub fn decide_or_error(
        &self,
        request: &DecisionRequest,
    ) -> Result<DecisionResult, LayaArmError> {
        let mut state = self.state();

        if let Some(reason) = &state.poisoned {
            return Err(LayaArmError::ChildGone(reason.clone()));
        }

        if state.session.is_none() {
            let mut session = ChildSession::spawn(&self.config)?;
            state.spawns += 1;
            match session.wait_ready(self.config.load_timeout) {
                Ok(ready) => {
                    state.ready = Some(ready);
                }
                Err(error) => {
                    let diagnostics = session.diagnostics();
                    session.kill();
                    let detail = if diagnostics.is_empty() {
                        error.to_string()
                    } else {
                        format!("{error} (child stderr: {})", diagnostics.join(" | "))
                    };
                    state.diagnostics = diagnostics;
                    state.poisoned = Some(detail.clone());
                    return Err(match error {
                        LayaArmError::LoadTimeout { timeout_ms } => {
                            LayaArmError::LoadTimeout { timeout_ms }
                        }
                        _ => LayaArmError::LoadFailed(detail),
                    });
                }
            }
            state.session = Some(session);
        }

        // The session is moved out for the duration of the request so the state
        // behind it can still be updated without holding two borrows of it.
        let Some(mut session) = state.session.take() else {
            return Err(LayaArmError::Protocol(
                "the child session disappeared while starting it".to_string(),
            ));
        };
        let timeout = self.config.request_timeout;
        if let Err(error) = session.send(request) {
            let detail = error.to_string();
            state.diagnostics = session.diagnostics();
            session.kill();
            state.poisoned = Some(detail.clone());
            return Err(LayaArmError::ChildGone(detail));
        }

        let mut early_summary = None;
        let outcome = loop {
            match session.next_line(timeout) {
                Ok(line) => match parse_line(&line) {
                    Ok(ChildLine::Result { request_id, result }) => {
                        if request_id != request.id {
                            break Err(LayaArmError::Protocol(format!(
                                "expected a result for {} but got one for {request_id}",
                                request.id
                            )));
                        }
                        break Ok(*result);
                    }
                    Ok(ChildLine::Error { request_id, error }) => {
                        break Err(LayaArmError::ChildError {
                            request_id,
                            message: error,
                        });
                    }
                    Ok(ChildLine::Summary(summary)) => {
                        // The child exited before answering: keep its accounting
                        // and treat this as a gone child.
                        early_summary = Some(*summary);
                        break Err(LayaArmError::ChildGone(
                            "the child summarised before answering".to_string(),
                        ));
                    }
                    Ok(ChildLine::Unknown(_)) => continue,
                    Ok(ChildLine::Ready(_)) => continue,
                    Err(error) => break Err(error),
                },
                Err(error) => break Err(error),
            }
        };

        if let Some(summary) = early_summary {
            state.summary = Some(summary);
        }

        match outcome {
            Ok(result) => {
                state.session = Some(session);
                match validate_decision(request, &result) {
                    Ok(()) => Ok(result),
                    Err(violation) => {
                        state.rejected += 1;
                        Err(LayaArmError::ContractViolation {
                            request_id: request.id.clone(),
                            detail: violation.to_string(),
                        })
                    }
                }
            }
            Err(error) => {
                // A timeout or a dead stream leaves the child unusable, so it is
                // killed now rather than left holding memory for a caller that
                // will never get an answer from it.
                let fatal = matches!(
                    error,
                    LayaArmError::RequestTimeout { .. }
                        | LayaArmError::ChildGone(_)
                        | LayaArmError::Protocol(_)
                );
                if fatal {
                    let detail = error.to_string();
                    state.diagnostics = session.diagnostics();
                    session.kill();
                    state.poisoned = Some(detail);
                } else {
                    state.session = Some(session);
                }
                Err(error)
            }
        }
    }

    /// Close the child's stdin and collect its summary, then hold the measured
    /// peak RSS against the configured ceiling.
    pub fn finish(&self) -> Result<Option<ChildSummary>, LayaArmError> {
        let ceiling = self.config.max_rss_bytes;
        let mut state = self.state();
        if let Some(summary) = state.summary.clone() {
            return check_ceiling(ceiling, summary).map(Some);
        }
        let Some(mut session) = state.session.take() else {
            return Ok(None);
        };
        session.close_stdin();

        // Read whatever is left: the summary, then EOF.
        let deadline = std::time::Instant::now() + self.config.request_timeout;
        let mut summary = None;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match session.next_line(remaining) {
                Ok(line) => match parse_line(&line) {
                    Ok(ChildLine::Summary(reported)) => {
                        summary = Some(*reported);
                        break;
                    }
                    Ok(_) => continue,
                    Err(_) => continue,
                },
                Err(_) => break,
            }
        }
        let diagnostics = session.diagnostics();
        session.kill();
        state.diagnostics = diagnostics;
        state.summary = summary.clone();
        match summary {
            Some(reported) => check_ceiling(ceiling, reported).map(Some),
            None => Ok(None),
        }
    }
}

fn check_ceiling(ceiling: u64, summary: ChildSummary) -> Result<ChildSummary, LayaArmError> {
    if summary.peak_rss_bytes > ceiling {
        return Err(LayaArmError::RssCeilingExceeded {
            peak_bytes: summary.peak_rss_bytes,
            ceiling_bytes: ceiling,
        });
    }
    Ok(summary)
}

impl DecisionArm for LayaArm {
    fn name(&self) -> &str {
        &self.label
    }

    /// The trait shape has no error channel, so a failure becomes a visible
    /// abstention whose `backend` names the reason. Callers that need the typed
    /// error use [`LayaArm::decide_or_error`].
    fn decide(&self, request: &DecisionRequest) -> DecisionResult {
        match self.decide_or_error(request) {
            Ok(result) => result,
            Err(error) => DecisionResult {
                request_id: request.id.clone(),
                prompt_sha256: request.prompt_sha256.clone(),
                abstain: true,
                backend: Some(format!("{}:error({})", self.label, error.code())),
                ..Default::default()
            },
        }
    }
}

/// The result of one measured batch.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchReport {
    pub scorecard: DecisionScorecard,
    pub ready: Option<ReadyInfo>,
    pub summary: Option<ChildSummary>,
    /// Children started for this batch. One is the design.
    pub spawns: usize,
    /// Results the contract refused before they reached the scorecard.
    pub rejected: usize,
    pub diagnostics: Vec<String>,
}

/// Score the arm over a fixture set in one child, and collect its accounting.
pub fn run_batch(
    config: &LayaArmConfig,
    fixtures: &[DecisionFixture],
) -> Result<BatchReport, LayaArmError> {
    let arm = LayaArm::new(config.clone());
    let scorecard = score_decisions(&arm, fixtures);
    let summary = arm.finish()?;
    Ok(BatchReport {
        scorecard,
        ready: arm.ready(),
        summary,
        spawns: arm.spawns(),
        rejected: arm.rejected(),
        diagnostics: arm.diagnostics(),
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
