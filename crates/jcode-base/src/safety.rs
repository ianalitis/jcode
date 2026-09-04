use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

use crate::storage;

/// Hook invoked to deliver a permission-request notification.
///
/// Args: `(action, description, request_id)`.
type PermissionNotifier = fn(&str, &str, &str);

static PERMISSION_NOTIFIER: OnceLock<PermissionNotifier> = OnceLock::new();

/// Register the permission-request notification dispatcher.
///
/// This inverts the historical `safety -> notifications` dependency: the
/// `notifications` layer (which already depends on `safety` types like
/// [`AmbientTranscript`]) registers its dispatcher here at startup, so
/// `safety` no longer needs to construct a `NotificationDispatcher`.
pub fn register_permission_notifier(notifier: PermissionNotifier) {
    let _ = PERMISSION_NOTIFIER.set(notifier);
}

fn dispatch_permission_notification(action: &str, description: &str, request_id: &str) {
    if let Some(notifier) = PERMISSION_NOTIFIER.get() {
        notifier(action, description, request_id);
    }
}

// ---------------------------------------------------------------------------
// Action classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionTier {
    AutoAllowed,
    RequiresPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Normal,
    High,
}

// ---------------------------------------------------------------------------
// Permission request / result / decision
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub action: String,
    pub description: String,
    pub rationale: String,
    pub urgency: Urgency,
    pub wait: bool,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionResult {
    Approved { message: Option<String> },
    Denied { reason: Option<String> },
    Queued { request_id: String },
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub request_id: String,
    pub approved: bool,
    pub decided_at: DateTime<Utc>,
    pub decided_via: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Action log / transcript
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLog {
    pub action_type: String,
    pub description: String,
    pub tier: ActionTier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptStatus {
    Complete,
    Interrupted,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientTranscript {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    pub status: TranscriptStatus,
    pub provider: String,
    pub model: String,
    pub actions: Vec<ActionLog>,
    pub pending_permissions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub compactions: u32,
    pub memories_modified: u32,
    /// Full conversation transcript (markdown) for email notifications
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
}

// ---------------------------------------------------------------------------
// Tier-1 (auto-allowed) action names
// ---------------------------------------------------------------------------

const AUTO_ALLOWED: &[&str] = &[
    "read",
    "glob",
    "grep",
    "ls",
    "memory",
    "todo",
    "todowrite",
    "todoread",
    "conversation_search",
    "session_search",
    "codesearch",
];

// ---------------------------------------------------------------------------
// SafetySystem
// ---------------------------------------------------------------------------

pub struct SafetySystem {
    queue: Mutex<Vec<PermissionRequest>>,
    history: Mutex<Vec<Decision>>,
    actions: Mutex<Vec<ActionLog>>,
}

impl SafetySystem {
    /// Create a new SafetySystem, loading persisted queue/history from disk.
    pub fn new() -> Self {
        let queue: Vec<PermissionRequest> = queue_path()
            .ok()
            .and_then(|p| storage::read_json(&p).ok())
            .unwrap_or_default();

        let history: Vec<Decision> = history_path()
            .ok()
            .and_then(|p| storage::read_json(&p).ok())
            .unwrap_or_default();

        SafetySystem {
            queue: Mutex::new(queue),
            history: Mutex::new(history),
            actions: Mutex::new(Vec::new()),
        }
    }

    /// Classify an action name into a tier.
    pub fn classify(&self, action: &str) -> ActionTier {
        let lower = action.to_lowercase();
        if AUTO_ALLOWED.iter().any(|&a| a == lower) {
            ActionTier::AutoAllowed
        } else {
            ActionTier::RequiresPermission
        }
    }

    /// Submit a permission request. Returns `Queued` with the request id.
    pub fn request_permission(&self, request: PermissionRequest) -> PermissionResult {
        let request_id = request.id.clone();
        let action = request.action.clone();
        let description = request.description.clone();
        let inserted = with_safety_state_lock(|| {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *queue = load_queue()?;
            if queue.iter().any(|pending| pending.id == request_id) {
                return Ok(false);
            }
            queue.push(request);
            persist_queue(&queue)?;
            Ok(true)
        });
        let inserted = match inserted {
            Ok(inserted) => inserted,
            Err(error) => {
                return PermissionResult::Denied {
                    reason: Some(format!("Could not persist permission request: {error}")),
                };
            }
        };
        if inserted {
            // Send high-priority notification for permission request via the
            // registered dispatcher (inverts the safety -> notifications edge).
            dispatch_permission_notification(&action, &description, &request_id);
        }
        PermissionResult::Queued { request_id }
    }

    /// Consume one persisted approval exactly once.
    ///
    /// The permissions TUI writes decisions from a separate process, so the
    /// read-check-write sequence is protected by the shared safety-state lock.
    pub fn consume_approved_decision(&self, request_id: &str) -> Result<bool> {
        with_safety_state_lock(|| {
            let mut history = self
                .history
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *history = load_history()?;
            let approved = history
                .iter()
                .rev()
                .find(|decision| decision.request_id == request_id)
                .is_some_and(|decision| decision.approved);
            if !approved {
                return Ok(false);
            }

            history.push(Decision {
                request_id: request_id.to_string(),
                approved: false,
                decided_at: Utc::now(),
                decided_via: "tool_execution".to_string(),
                message: Some("Approval consumed by one tool execution.".to_string()),
            });
            persist_history(&history)?;
            Ok(true)
        })
    }

    /// Expire pending permission requests that can no longer be serviced
    /// because their originating session is no longer active.
    pub fn expire_dead_session_requests(&self, via: &str) -> Result<Vec<String>> {
        with_safety_state_lock(|| {
            let mut expired: Vec<(String, String)> = Vec::new();
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *queue = load_queue()?;
            let mut retained: Vec<PermissionRequest> = Vec::with_capacity(queue.len());
            for req in queue.drain(..) {
                if let Some(reason) = stale_request_reason(&req) {
                    expired.push((req.id.clone(), reason));
                } else {
                    retained.push(req);
                }
            }
            *queue = retained;
            persist_queue(&queue)?;

            if expired.is_empty() {
                return Ok(Vec::new());
            }

            let mut history = self
                .history
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *history = load_history()?;
            for (request_id, reason) in &expired {
                history.push(Decision {
                    request_id: request_id.clone(),
                    approved: false,
                    decided_at: Utc::now(),
                    decided_via: via.to_string(),
                    message: Some(format!(
                        "Expired automatically: {}. Original agent is no longer active.",
                        reason
                    )),
                });
            }
            persist_history(&history)?;

            Ok(expired.into_iter().map(|(id, _)| id).collect())
        })
    }

    /// Record a user decision (approve / deny) for a pending request.
    pub fn record_decision(
        &self,
        request_id: &str,
        approved: bool,
        via: &str,
        message: Option<String>,
    ) -> Result<()> {
        with_safety_state_lock(|| {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *queue = load_queue()?;
            if !queue.iter().any(|request| request.id == request_id) {
                anyhow::bail!("permission request is no longer pending");
            }
            queue.retain(|request| request.id != request_id);
            persist_queue(&queue)?;

            let mut history = self
                .history
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *history = load_history()?;
            history.push(Decision {
                request_id: request_id.to_string(),
                approved,
                decided_at: Utc::now(),
                decided_via: via.to_string(),
                message,
            });
            persist_history(&history)
        })
    }

    /// Return all pending permission requests.
    pub fn pending_requests(&self) -> Vec<PermissionRequest> {
        self.queue.lock().map(|q| q.clone()).unwrap_or_default()
    }

    /// Append an action to the in-memory log.
    pub fn log_action(&self, log: ActionLog) {
        if let Ok(mut actions) = self.actions.lock() {
            actions.push(log);
        }
    }

    /// Generate a human-readable summary of logged actions.
    pub fn generate_summary(&self) -> String {
        let actions = self.actions.lock().map(|a| a.clone()).unwrap_or_default();
        let pending = self.pending_requests();

        let mut lines: Vec<String> = Vec::new();

        if actions.is_empty() && pending.is_empty() {
            return "No actions recorded.".to_string();
        }

        // Separate auto vs permission-required
        let auto: Vec<&ActionLog> = actions
            .iter()
            .filter(|a| a.tier == ActionTier::AutoAllowed)
            .collect();
        let perm: Vec<&ActionLog> = actions
            .iter()
            .filter(|a| a.tier == ActionTier::RequiresPermission)
            .collect();

        if !auto.is_empty() {
            lines.push("Done (auto-allowed):".to_string());
            for a in &auto {
                lines.push(format!("- {} — {}", a.action_type, a.description));
            }
        }

        if !perm.is_empty() {
            lines.push(String::new());
            lines.push("Done (with permission):".to_string());
            for a in &perm {
                lines.push(format!("- {} — {}", a.action_type, a.description));
            }
        }

        if !pending.is_empty() {
            lines.push(String::new());
            lines.push("Needs your review:".to_string());
            for r in &pending {
                lines.push(format!(
                    "- [{:?}] {} — {}",
                    r.urgency, r.action, r.description
                ));
            }
        }

        lines.join("\n")
    }

    /// Persist a transcript to ~/.jcode/ambient/transcripts/{timestamp}.json
    pub fn save_transcript(&self, transcript: &AmbientTranscript) -> Result<()> {
        let dir = storage::jcode_dir()?.join("ambient").join("transcripts");
        storage::ensure_dir(&dir)?;

        let filename = transcript.started_at.format("%Y-%m-%d-%H%M%S").to_string();
        let path = dir.join(format!("{}.json", filename));
        storage::write_json(&path, transcript)
    }
}

impl Default for SafetySystem {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

fn queue_path() -> Result<std::path::PathBuf> {
    Ok(storage::jcode_dir()?.join("safety").join("queue.json"))
}

fn history_path() -> Result<std::path::PathBuf> {
    Ok(storage::jcode_dir()?.join("safety").join("history.json"))
}

fn load_queue() -> Result<Vec<PermissionRequest>> {
    let path = queue_path()?;
    if path.exists() {
        storage::read_json(&path)
    } else {
        Ok(Vec::new())
    }
}

fn load_history() -> Result<Vec<Decision>> {
    let path = history_path()?;
    if path.exists() {
        storage::read_json(&path)
    } else {
        Ok(Vec::new())
    }
}

fn persist_queue(queue: &[PermissionRequest]) -> Result<()> {
    let path = queue_path()?;
    storage::write_json(&path, queue)
}

fn persist_history(history: &[Decision]) -> Result<()> {
    let path = history_path()?;
    storage::write_json(&path, history)
}

static SAFETY_STATE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

struct SafetyStateFileLock {
    _file: std::fs::File,
    #[cfg(not(unix))]
    path: std::path::PathBuf,
}

#[cfg(not(unix))]
impl Drop for SafetyStateFileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn acquire_safety_state_file_lock() -> Result<SafetyStateFileLock> {
    use std::fs::OpenOptions;

    let path = storage::jcode_dir()?.join("safety").join("state.lock");
    if let Some(parent) = path.parent() {
        storage::ensure_dir(parent)?;
    }

    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(SafetyStateFileLock { _file: file })
    }

    #[cfg(not(unix))]
    {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        Ok(SafetyStateFileLock { _file: file, path })
    }
}

fn with_safety_state_lock<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let _process_guard = SAFETY_STATE_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _file_guard = acquire_safety_state_file_lock()?;
    operation()
}

// ---------------------------------------------------------------------------
// File-based permission decision (for IMAP poller / external callers)
// ---------------------------------------------------------------------------

/// Record a permission decision by directly manipulating the queue/history JSON files.
/// Used by the IMAP reply poller which doesn't have access to the live SafetySystem instance.
pub fn record_permission_via_file(
    request_id: &str,
    approved: bool,
    via: &str,
    message: Option<String>,
) -> Result<()> {
    with_safety_state_lock(|| {
        let mut queue = load_queue()?;
        if !queue.iter().any(|request| request.id == request_id) {
            anyhow::bail!("permission request is no longer pending");
        }
        queue.retain(|request| request.id != request_id);
        persist_queue(&queue)?;

        let mut history = load_history()?;
        history.push(Decision {
            request_id: request_id.to_string(),
            approved,
            decided_at: Utc::now(),
            decided_via: via.to_string(),
            message,
        });
        persist_history(&history)
    })
}

/// Expire stale permission requests directly via queue/history files.
/// Used by processes that don't hold the live SafetySystem instance.
pub fn expire_stale_permissions_via_file(via: &str) -> Result<Vec<String>> {
    with_safety_state_lock(|| {
        let mut queue = load_queue()?;
        let mut expired: Vec<(String, String)> = Vec::new();
        queue.retain(|req| {
            if let Some(reason) = stale_request_reason(req) {
                expired.push((req.id.clone(), reason));
                false
            } else {
                true
            }
        });
        persist_queue(&queue)?;

        if expired.is_empty() {
            return Ok(Vec::new());
        }

        let mut history = load_history()?;
        for (request_id, reason) in &expired {
            history.push(Decision {
                request_id: request_id.clone(),
                approved: false,
                decided_at: Utc::now(),
                decided_via: via.to_string(),
                message: Some(format!(
                    "Expired automatically: {}. Original agent is no longer active.",
                    reason
                )),
            });
        }
        persist_history(&history)?;

        Ok(expired.into_iter().map(|(id, _)| id).collect())
    })
}

fn stale_request_reason(request: &PermissionRequest) -> Option<String> {
    let session_id = request_session_id(request)?;
    let mut session = match crate::session::Session::load(&session_id) {
        Ok(s) => s,
        Err(_) => return Some(format!("owner session '{}' was not found", session_id)),
    };

    // Refresh crash status based on PID if needed.
    if session.detect_crash() {
        let _ = session.save();
    }

    if session.status == crate::session::SessionStatus::Active {
        None
    } else {
        Some(format!(
            "owner session '{}' is {}",
            session_id,
            session.status.display()
        ))
    }
}

fn request_session_id(request: &PermissionRequest) -> Option<String> {
    let context = request.context.as_ref()?;

    context
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            context
                .get("requester")
                .and_then(|r| r.get("session_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

// ---------------------------------------------------------------------------
// ID generation helper
// ---------------------------------------------------------------------------

/// Generate a unique permission request id: `req_{timestamp}_{random}`
pub fn new_request_id() -> String {
    crate::id::new_id("req")
}

/// Generate a deterministic request id bound to an exact action scope.
pub fn scoped_request_id(scope: &str) -> String {
    let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, scope.as_bytes());
    format!("req_{}", id.simple())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn with_temp_home<F, T>(f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _guard = crate::storage::lock_test_env();
        let prev_home = std::env::var_os("JCODE_HOME");
        let temp = tempfile::TempDir::new().expect("create temp dir");
        crate::env::set_var("JCODE_HOME", temp.path());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        match prev_home {
            Some(value) => crate::env::set_var("JCODE_HOME", value),
            None => crate::env::remove_var("JCODE_HOME"),
        }

        result.unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    }

    #[test]
    fn test_classify_auto_allowed() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            assert_eq!(sys.classify("read"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("glob"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("grep"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("ls"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("memory"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("todo"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("todowrite"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("todoread"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("conversation_search"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("session_search"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("codesearch"), ActionTier::AutoAllowed);
        });
    }

    #[test]
    fn test_classify_requires_permission() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            assert_eq!(sys.classify("bash"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("write"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("edit"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("multiedit"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("patch"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("apply_patch"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("communicate"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("open"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("launch"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("webfetch"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("websearch"), ActionTier::RequiresPermission);
            assert_eq!(sys.classify("unknown_tool"), ActionTier::RequiresPermission);
        });
    }

    #[test]
    fn test_classify_case_insensitive() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            assert_eq!(sys.classify("Read"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("GLOB"), ActionTier::AutoAllowed);
            assert_eq!(sys.classify("Bash"), ActionTier::RequiresPermission);
        });
    }

    #[test]
    fn test_request_permission_returns_queued() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            let baseline = sys.pending_requests().len();
            let req = PermissionRequest {
                id: "req_test_1".to_string(),
                action: "create_pull_request".to_string(),
                description: "Create PR for test fixes".to_string(),
                rationale: "Found failing tests".to_string(),
                urgency: Urgency::Normal,
                wait: false,
                created_at: Utc::now(),
                context: None,
            };

            let result = sys.request_permission(req);
            match result {
                PermissionResult::Queued { request_id } => {
                    assert_eq!(request_id, "req_test_1");
                }
                _ => panic!("Expected Queued result"),
            }

            assert_eq!(sys.pending_requests().len(), baseline + 1);

            let duplicate = PermissionRequest {
                id: "req_test_1".to_string(),
                action: "create_pull_request".to_string(),
                description: "Create PR for test fixes".to_string(),
                rationale: "Found failing tests".to_string(),
                urgency: Urgency::Normal,
                wait: false,
                created_at: Utc::now(),
                context: None,
            };
            sys.request_permission(duplicate);
            assert_eq!(sys.pending_requests().len(), baseline + 1);
        });
    }

    #[test]
    fn test_record_decision_removes_from_queue() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            let baseline = sys.pending_requests().len();
            let req = PermissionRequest {
                id: "req_test_2".to_string(),
                action: "push".to_string(),
                description: "Push to origin".to_string(),
                rationale: "Ready for review".to_string(),
                urgency: Urgency::Low,
                wait: false,
                created_at: Utc::now(),
                context: None,
            };

            sys.request_permission(req);
            assert_eq!(sys.pending_requests().len(), baseline + 1);

            sys.record_decision("req_test_2", true, "tui", Some("looks good".to_string()))
                .unwrap();
            assert_eq!(sys.pending_requests().len(), baseline);
        });
    }

    #[test]
    fn test_log_action_and_summary() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            sys.log_action(ActionLog {
                action_type: "memory_consolidation".to_string(),
                description: "Merged 2 duplicate memories".to_string(),
                tier: ActionTier::AutoAllowed,
                details: None,
                timestamp: Utc::now(),
            });
            sys.log_action(ActionLog {
                action_type: "edit".to_string(),
                description: "Fixed typo in README".to_string(),
                tier: ActionTier::RequiresPermission,
                details: None,
                timestamp: Utc::now(),
            });

            let summary = sys.generate_summary();
            assert!(summary.contains("memory_consolidation"));
            assert!(summary.contains("edit"));
            assert!(summary.contains("Done (auto-allowed)"));
            assert!(summary.contains("Done (with permission)"));
        });
    }

    #[test]
    fn test_empty_summary() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            let summary = sys.generate_summary();
            assert_eq!(summary, "No actions recorded.");
        });
    }

    #[test]
    fn test_new_request_id_format() {
        with_temp_home(|| {
            let id = new_request_id();
            assert!(id.starts_with("req_"));
        });
    }

    #[test]
    fn scoped_request_ids_are_stable_and_scope_bound() {
        let first = scoped_request_id("bash\0session-a\0pwd\0rm -rf target");
        assert_eq!(
            first,
            scoped_request_id("bash\0session-a\0pwd\0rm -rf target")
        );
        assert_ne!(
            first,
            scoped_request_id("bash\0session-b\0pwd\0rm -rf target")
        );
        assert_ne!(
            first,
            scoped_request_id("bash\0session-a\0pwd\0rm -rf other")
        );
        assert_ne!(
            first,
            scoped_request_id("bash\0session-a\0other-pwd\0rm -rf target")
        );
        assert_ne!(
            first,
            scoped_request_id("bash\0session-a\0pwd\0rm -rf target ")
        );
    }

    #[test]
    fn test_record_permission_via_file() {
        with_temp_home(|| {
            let sys = SafetySystem::new();
            let baseline = sys.pending_requests().len();
            let req = PermissionRequest {
                id: "req_file_test".to_string(),
                action: "push".to_string(),
                description: "Push to origin".to_string(),
                rationale: "Ready for review".to_string(),
                urgency: Urgency::Low,
                wait: false,
                created_at: Utc::now(),
                context: None,
            };
            sys.request_permission(req);
            assert_eq!(sys.pending_requests().len(), baseline + 1);

            record_permission_via_file("req_file_test", true, "email_reply", None).unwrap();

            let sys2 = SafetySystem::new();
            let still_pending = sys2
                .pending_requests()
                .iter()
                .any(|r| r.id == "req_file_test");
            assert!(
                !still_pending,
                "request should have been removed from queue"
            );
        });
    }

    #[test]
    fn persisted_approval_is_consumed_once() {
        with_temp_home(|| {
            let request_id = scoped_request_id("one-time approval");
            let sys = SafetySystem::new();
            sys.request_permission(PermissionRequest {
                id: request_id.clone(),
                action: "bash_destructive_command".to_string(),
                description: "Delete fixture".to_string(),
                rationale: "Test one-time approval".to_string(),
                urgency: Urgency::High,
                wait: false,
                created_at: Utc::now(),
                context: None,
            });
            record_permission_via_file(&request_id, true, "permissions_tui", None).unwrap();

            assert!(sys.consume_approved_decision(&request_id).unwrap());
            assert!(!sys.consume_approved_decision(&request_id).unwrap());
        });
    }

    #[test]
    fn concurrent_consumers_cannot_reuse_one_approval() {
        with_temp_home(|| {
            let request_id = scoped_request_id("concurrent one-time approval");
            let sys = SafetySystem::new();
            sys.request_permission(PermissionRequest {
                id: request_id.clone(),
                action: "bash_destructive_command".to_string(),
                description: "Delete fixture".to_string(),
                rationale: "Test concurrent consumption".to_string(),
                urgency: Urgency::High,
                wait: false,
                created_at: Utc::now(),
                context: None,
            });
            record_permission_via_file(&request_id, true, "permissions_tui", None).unwrap();

            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let barrier = barrier.clone();
                    let request_id = request_id.clone();
                    std::thread::spawn(move || {
                        let sys = SafetySystem::new();
                        barrier.wait();
                        sys.consume_approved_decision(&request_id).unwrap()
                    })
                })
                .collect();
            let outcomes: Vec<bool> = handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect();

            assert_eq!(outcomes.iter().filter(|approved| **approved).count(), 1);
            assert_eq!(outcomes.iter().filter(|approved| !**approved).count(), 1);
        });
    }

    #[test]
    fn permission_request_persistence_failure_denies_instead_of_queueing() {
        with_temp_home(|| {
            let blocked_home = storage::jcode_dir().unwrap().join("not-a-directory");
            std::fs::write(&blocked_home, "file blocks directory creation").unwrap();
            crate::env::set_var("JCODE_HOME", blocked_home);

            let result = SafetySystem::new().request_permission(PermissionRequest {
                id: "req_cannot_persist".to_string(),
                action: "bash_destructive_command".to_string(),
                description: "Delete fixture".to_string(),
                rationale: "Test fail-closed behavior".to_string(),
                urgency: Urgency::High,
                wait: false,
                created_at: Utc::now(),
                context: None,
            });

            match result {
                PermissionResult::Denied { reason } => assert!(
                    reason
                        .as_deref()
                        .is_some_and(|text| text.contains("Could not persist"))
                ),
                other => panic!("persistence failure must deny, got {other:?}"),
            }
        });
    }
}
