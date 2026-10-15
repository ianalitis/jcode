use crate::protocol::ServerEvent;
use crate::server::durable_state::{
    elapsed_exceeds, hashed_request_key, now_unix_ms, save_json_state, state_path,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};

const SWARM_MUTATION_DIR: &str = "jcode-swarm-mutations";
const FINAL_STATE_TTL: Duration = Duration::from_secs(30);
const PENDING_STATE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum PersistedSwarmMutationResponse {
    Done,
    AssignTask {
        task_id: String,
        target_session: String,
    },
    Error {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_secs: Option<u64>,
    },
    Spawn {
        new_session_id: String,
    },
}

impl PersistedSwarmMutationResponse {
    fn into_server_event(self, id: u64, session_id: &str) -> ServerEvent {
        match self {
            Self::Done => ServerEvent::Done { id },
            Self::AssignTask {
                task_id,
                target_session,
            } => ServerEvent::CommAssignTaskResponse {
                id,
                task_id,
                target_session,
            },
            Self::Error {
                message,
                retry_after_secs,
            } => ServerEvent::Error {
                id,
                message,
                retry_after_secs,
            },
            Self::Spawn { new_session_id } => ServerEvent::CommSpawnResponse {
                id,
                session_id: session_id.to_string(),
                new_session_id,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedSwarmMutationState {
    pub key: String,
    pub action: String,
    pub session_id: String,
    pub created_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_response: Option<PersistedSwarmMutationResponse>,
}

#[derive(Clone)]
struct SwarmMutationWaiter {
    request_id: u64,
    client_event_tx: mpsc::UnboundedSender<ServerEvent>,
}

/// In-memory coordination for in-flight swarm mutations.
///
/// A single lock guards both the active-key set and the waiter map so the
/// persisted-final check, waiter registration, and active-claim in
/// [`begin_or_replay`] form one atomic step with respect to
/// [`finish_request`] draining waiters and releasing the claim. Splitting
/// these across separate locks allowed a TOCTOU where a duplicate request
/// could observe "no final state" before the original finished, then
/// register itself after the finisher had already drained waiters and
/// cleared the active claim, re-executing the whole mutation (double
/// assign/spawn).
#[derive(Default)]
struct SwarmMutationSync {
    active_keys: HashSet<String>,
    waiters: HashMap<String, Vec<SwarmMutationWaiter>>,
}

#[derive(Clone, Default)]
pub(crate) struct SwarmMutationRuntime {
    sync: Arc<RwLock<SwarmMutationSync>>,
}

fn is_stale(state: &PersistedSwarmMutationState) -> bool {
    if state.final_response.is_some() {
        elapsed_exceeds(state.created_at_unix_ms, FINAL_STATE_TTL)
    } else {
        elapsed_exceeds(state.created_at_unix_ms, PENDING_STATE_TTL)
    }
}

pub(super) fn request_key(session_id: &str, action: &str, components: &[String]) -> String {
    hashed_request_key(session_id, action, components)
}

fn load_state_strict(
    key: &str,
    action: &str,
    session_id: &str,
) -> Result<Option<PersistedSwarmMutationState>, String> {
    let path = state_path(SWARM_MUTATION_DIR, key);
    // Admission must not recover a potentially stale backup or mistake corrupt
    // or unreadable evidence for a new request. Storage replaces .json with .bak.
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // A dangling primary symlink is evidence, not genuine absence.
            match std::fs::symlink_metadata(&path) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Ok(_) => return Err("primary state exists but cannot be read".to_string()),
                Err(err) => return Err(format!("cannot inspect primary state: {err}")),
            }
            return match std::fs::symlink_metadata(path.with_extension("bak")) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Ok(_) => Err("primary state is missing but backup evidence exists".to_string()),
                Err(err) => Err(format!("cannot inspect state backup: {err}")),
            };
        }
        Err(err) => return Err(format!("cannot read persisted state: {err}")),
    };
    let state: PersistedSwarmMutationState = serde_json::from_slice(&bytes)
        .map_err(|err| format!("cannot decode persisted state: {err}"))?;
    if state.key != key || state.action != action || state.session_id != session_id {
        return Err("persisted key, action, or session does not match the request".to_string());
    }
    Ok(Some(state))
}

pub(super) fn save_state(state: &PersistedSwarmMutationState) {
    save_json_state(
        SWARM_MUTATION_DIR,
        &state.key,
        state,
        "swarm mutation state",
    )
}

pub(super) fn persist_final_response(
    state: &PersistedSwarmMutationState,
    response: PersistedSwarmMutationResponse,
) -> PersistedSwarmMutationState {
    let mut next = state.clone();
    next.final_response = Some(response);
    save_state(&next);
    next
}

pub(super) async fn begin_or_replay(
    runtime: &SwarmMutationRuntime,
    key: &str,
    action: &str,
    session_id: &str,
    request_id: u64,
    client_event_tx: &mpsc::UnboundedSender<ServerEvent>,
) -> Option<PersistedSwarmMutationState> {
    begin_with_mode(
        runtime,
        key,
        action,
        session_id,
        request_id,
        client_event_tx,
        true,
    )
    .await
}

/// Like [`begin_or_replay`], but never replays a persisted final response.
///
/// Explicit control-driven mutations (retry/reassign/replace/salvage) must
/// re-dispatch even when an identical mutation finished moments ago: a worker
/// that fails within `FINAL_STATE_TTL` would otherwise turn the coordinator's
/// follow-up retry into a silent no-op replay. Concurrent in-flight duplicates
/// still join the active execution as waiters.
pub(super) async fn begin_or_join_in_flight(
    runtime: &SwarmMutationRuntime,
    key: &str,
    action: &str,
    session_id: &str,
    request_id: u64,
    client_event_tx: &mpsc::UnboundedSender<ServerEvent>,
) -> Option<PersistedSwarmMutationState> {
    begin_with_mode(
        runtime,
        key,
        action,
        session_id,
        request_id,
        client_event_tx,
        false,
    )
    .await
}

async fn begin_with_mode(
    runtime: &SwarmMutationRuntime,
    key: &str,
    action: &str,
    session_id: &str,
    request_id: u64,
    client_event_tx: &mpsc::UnboundedSender<ServerEvent>,
    replay_final: bool,
) -> Option<PersistedSwarmMutationState> {
    // Hold the sync lock across the strict read, waiter registration, and
    // active claim. Finish cannot drain/clear between the read and the claim.
    // Reuse this validated snapshot instead of reloading after claiming.
    let mut sync = runtime.sync.write().await;
    let existing = match load_state_strict(key, action, session_id) {
        Ok(state) => state,
        Err(reason) => {
            if let Err(err) = client_event_tx.send(ServerEvent::Error {
                id: request_id,
                message: format!("Cannot admit swarm mutation: {reason}"),
                retry_after_secs: None,
            }) {
                crate::logging::warn(&format!(
                    "Failed to deliver swarm mutation admission rejection for request {request_id}: {err}"
                ));
            }
            return None;
        }
    };
    // Validate ownership before applying either TTL. Replace expired matching
    // state only when admitting an executor, without leaving an orphan backup.
    let existing = existing.filter(|state| !is_stale(state));
    if replay_final
        && let Some(final_response) = existing
            .as_ref()
            .and_then(|state| state.final_response.clone())
    {
        let _ = client_event_tx.send(final_response.into_server_event(request_id, session_id));
        return None;
    }
    sync.waiters
        .entry(key.to_string())
        .or_default()
        .push(SwarmMutationWaiter {
            request_id,
            client_event_tx: client_event_tx.clone(),
        });
    if !sync.active_keys.insert(key.to_string()) {
        return None;
    }

    if replay_final && let Some(existing) = existing {
        return Some(existing);
    }
    // A control-driven mutation is a fresh attempt even after a valid final.
    let state = PersistedSwarmMutationState {
        key: key.to_string(),
        action: action.to_string(),
        session_id: session_id.to_string(),
        created_at_unix_ms: now_unix_ms(),
        final_response: None,
    };
    if let Err(err) = crate::storage::write_json_fast(&state_path(SWARM_MUTATION_DIR, key), &state)
    {
        // Only this newly acquired claim is ours to unwind. Existing executors
        // and loaded pending replay returned above without attempting this save.
        let waiters = sync.waiters.remove(key);
        sync.active_keys.remove(key);
        drop(sync);
        let message = format!("Cannot admit swarm mutation: cannot persist pending state: {err}");
        if let Some(waiters) = waiters {
            for waiter in waiters {
                if let Err(err) = waiter.client_event_tx.send(ServerEvent::Error {
                    id: waiter.request_id,
                    message: message.clone(),
                    retry_after_secs: None,
                }) {
                    crate::logging::warn(&format!(
                        "Failed to deliver swarm mutation admission rejection for request {}: {err}",
                        waiter.request_id
                    ));
                }
            }
        }
        return None;
    }
    Some(state)
}

pub(super) async fn finish_request(
    runtime: &SwarmMutationRuntime,
    state: &PersistedSwarmMutationState,
    response: PersistedSwarmMutationResponse,
) {
    // Serialize persistence with admission so strict reads cannot observe the
    // temporary missing primary during non-Unix backup replacement.
    let mut sync = runtime.sync.write().await;
    // Persist the final response BEFORE draining waiters/releasing the
    // active claim: a duplicate request that misses the drain below must be
    // able to observe the persisted final state and replay it instead of
    // re-executing the mutation.
    let persisted = persist_final_response(state, response.clone());
    let waiters = sync.waiters.remove(&persisted.key).unwrap_or_default();
    sync.active_keys.remove(&persisted.key);
    drop(sync);
    for waiter in waiters {
        let _ = waiter.client_event_tx.send(
            response
                .clone()
                .into_server_event(waiter.request_id, &persisted.session_id),
        );
    }
}

#[cfg(test)]
#[path = "swarm_mutation_state_tests.rs"]
mod swarm_mutation_state_tests;
