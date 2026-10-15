use super::{
    PersistedSwarmMutationResponse, SwarmMutationRuntime, begin_or_join_in_flight, begin_or_replay,
    finish_request, request_key,
};
use super::{PersistedSwarmMutationState, SWARM_MUTATION_DIR};
use crate::protocol::ServerEvent;
use crate::server::durable_state::{now_unix_ms, state_path};

struct RuntimeEnvGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
    prev_runtime: Option<std::ffi::OsString>,
    prev_home: Option<std::ffi::OsString>,
}

impl RuntimeEnvGuard {
    fn new() -> (Self, tempfile::TempDir) {
        let guard = crate::storage::lock_test_env();
        let temp = tempfile::TempDir::new().expect("create runtime dir");
        let prev_runtime = std::env::var_os("JCODE_RUNTIME_DIR");
        let prev_home = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_RUNTIME_DIR", temp.path());
        crate::env::set_var("JCODE_HOME", temp.path().join("home"));
        (
            Self {
                _guard: guard,
                prev_runtime,
                prev_home,
            },
            temp,
        )
    }
}

impl Drop for RuntimeEnvGuard {
    fn drop(&mut self) {
        if let Some(prev_home) = self.prev_home.take() {
            crate::env::set_var("JCODE_HOME", prev_home);
        } else {
            crate::env::remove_var("JCODE_HOME");
        }
        if let Some(prev_runtime) = self.prev_runtime.take() {
            crate::env::set_var("JCODE_RUNTIME_DIR", prev_runtime);
        } else {
            crate::env::remove_var("JCODE_RUNTIME_DIR");
        }
    }
}

#[tokio::test]
async fn swarm_mutation_replays_persisted_spawn_response() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let runtime = SwarmMutationRuntime::default();
    let (client_tx, mut client_rx) = tokio::sync::mpsc::unbounded_channel();
    let key = request_key(
        "coord",
        "spawn",
        &[
            "swarm-1".to_string(),
            "/repo".to_string(),
            "hello".to_string(),
        ],
    );

    let state = begin_or_replay(&runtime, &key, "spawn", "coord", 1, &client_tx)
        .await
        .expect("first request should start execution");
    finish_request(
        &runtime,
        &state,
        PersistedSwarmMutationResponse::Spawn {
            new_session_id: "child-1".to_string(),
        },
    )
    .await;

    let (retry_tx, mut retry_rx) = tokio::sync::mpsc::unbounded_channel();
    let replay = begin_or_replay(&runtime, &key, "spawn", "coord", 2, &retry_tx).await;
    assert!(replay.is_none(), "retry should replay persisted response");

    match client_rx.recv().await.expect("initial response") {
        ServerEvent::CommSpawnResponse { new_session_id, .. } => {
            assert_eq!(new_session_id, "child-1")
        }
        other => panic!("expected spawn response, got {other:?}"),
    }

    match retry_rx.recv().await.expect("replayed response") {
        ServerEvent::CommSpawnResponse {
            id, new_session_id, ..
        } => {
            assert_eq!(id, 2);
            assert_eq!(new_session_id, "child-1");
        }
        other => panic!("expected spawn replay, got {other:?}"),
    }
}

#[tokio::test]
async fn swarm_mutation_concurrent_duplicates_share_final_done_response() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let runtime = SwarmMutationRuntime::default();
    let key = request_key(
        "coord",
        "assign_task",
        &[
            "swarm-1".to_string(),
            "worker-1".to_string(),
            "task-1".to_string(),
            "extra".to_string(),
        ],
    );
    let (first_tx, mut first_rx) = tokio::sync::mpsc::unbounded_channel();
    let (retry_tx, mut retry_rx) = tokio::sync::mpsc::unbounded_channel();

    let state = begin_or_replay(&runtime, &key, "assign_task", "coord", 1, &first_tx)
        .await
        .expect("first request should start execution");
    let replay = begin_or_replay(&runtime, &key, "assign_task", "coord", 2, &retry_tx).await;
    assert!(
        replay.is_none(),
        "second in-flight duplicate should wait for original completion"
    );

    finish_request(&runtime, &state, PersistedSwarmMutationResponse::Done).await;

    match first_rx.recv().await.expect("first response") {
        ServerEvent::Done { id } => assert_eq!(id, 1),
        other => panic!("expected done, got {other:?}"),
    }
    match retry_rx.recv().await.expect("retry response") {
        ServerEvent::Done { id } => assert_eq!(id, 2),
        other => panic!("expected done, got {other:?}"),
    }
}

#[tokio::test]
async fn swarm_mutation_finish_waits_for_sync_before_persisting_final() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let runtime = SwarmMutationRuntime::default();
    let key = request_key("coord", "assign_task", &["finish-lock".to_string()]);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let state = begin_or_replay(&runtime, &key, "assign_task", "coord", 1, &tx)
        .await
        .expect("first request should start execution");
    assert!(
        begin_or_replay(&runtime, &key, "assign_task", "coord", 2, &tx)
            .await
            .is_none()
    );
    let path = state_path(SWARM_MUTATION_DIR, &key);
    let backup = path.with_extension("bak");
    let primary_before = std::fs::read(&path).unwrap();
    let backup_before = std::fs::read(&backup).map_err(|err| err.kind());
    let pending: PersistedSwarmMutationState = serde_json::from_slice(&primary_before).unwrap();
    assert!(pending.final_response.is_none());

    let sync = runtime.sync.write().await;
    assert!(sync.active_keys.contains(&key));
    assert_eq!(sync.waiters[&key].len(), 2);
    let mut finish = std::pin::pin!(finish_request(
        &runtime,
        &state,
        PersistedSwarmMutationResponse::Done
    ));
    // Poll exactly once while admission's lock is held, without scheduler timing.
    let polled = std::future::poll_fn(|cx| {
        std::task::Poll::Ready(std::future::Future::poll(finish.as_mut(), cx))
    })
    .await;
    assert!(polled.is_pending());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        primary_before,
        "final persistence must wait for the admission lock"
    );
    assert_eq!(
        std::fs::read(&backup).map_err(|err| err.kind()),
        backup_before
    );
    assert!(rx.try_recv().is_err());
    drop(sync);

    finish.await;
    for expected_id in [1, 2] {
        match rx.try_recv().expect("waiter must receive final response") {
            ServerEvent::Done { id } => assert_eq!(id, expected_id),
            other => panic!("expected done, got {other:?}"),
        }
    }
    let finalized: PersistedSwarmMutationState =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        finalized.final_response,
        Some(PersistedSwarmMutationResponse::Done)
    );
    assert!(
        begin_or_replay(&runtime, &key, "assign_task", "coord", 3, &tx)
            .await
            .is_none()
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        ServerEvent::Done { id: 3 }
    ));
    assert!(rx.try_recv().is_err());
    let sync = runtime.sync.read().await;
    assert!(sync.active_keys.is_empty());
    assert!(sync.waiters.is_empty());
}

/// Regression: a control-driven mutation (retry/reassign/replace/salvage)
/// must re-dispatch even when an identical mutation persisted a success
/// within the final-state TTL. Replaying the stale success would turn the
/// coordinator's deliberate retry into a silent no-op whenever the worker
/// failed in under the TTL.
#[tokio::test]
async fn control_driven_begin_re_dispatches_despite_persisted_final_state() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let runtime = SwarmMutationRuntime::default();
    let key = request_key(
        "coord",
        "assign_task",
        &[
            "swarm-1".to_string(),
            "worker-1".to_string(),
            "task-1".to_string(),
            "Retry this assignment.".to_string(),
        ],
    );

    let (first_tx, mut first_rx) = tokio::sync::mpsc::unbounded_channel();
    let state = begin_or_join_in_flight(&runtime, &key, "assign_task", "coord", 1, &first_tx)
        .await
        .expect("first attempt should start execution");
    finish_request(
        &runtime,
        &state,
        PersistedSwarmMutationResponse::AssignTask {
            task_id: "task-1".to_string(),
            target_session: "worker-1".to_string(),
        },
    )
    .await;
    match first_rx.recv().await.expect("first response") {
        ServerEvent::CommAssignTaskResponse { id, .. } => assert_eq!(id, 1),
        other => panic!("expected assign response, got {other:?}"),
    }

    // Identical retry within the final-state TTL: must start a fresh attempt.
    let (retry_tx, _retry_rx) = tokio::sync::mpsc::unbounded_channel();
    let retry_state = begin_or_join_in_flight(&runtime, &key, "assign_task", "coord", 2, &retry_tx)
        .await
        .expect("control-driven retry must re-dispatch instead of replaying the stale success");
    assert!(
        retry_state.final_response.is_none(),
        "fresh attempt must not carry the previous attempt's final response"
    );
}

/// Concurrent in-flight duplicates of a control-driven mutation still
/// coalesce onto the active execution instead of double-dispatching.
#[tokio::test]
async fn control_driven_begin_coalesces_in_flight_duplicates() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let runtime = SwarmMutationRuntime::default();
    let key = request_key(
        "coord",
        "assign_task",
        &["swarm-1".to_string(), "worker-1".to_string()],
    );

    let (first_tx, mut first_rx) = tokio::sync::mpsc::unbounded_channel();
    let (dup_tx, mut dup_rx) = tokio::sync::mpsc::unbounded_channel();
    let state = begin_or_join_in_flight(&runtime, &key, "assign_task", "coord", 1, &first_tx)
        .await
        .expect("first attempt should start execution");
    let dup = begin_or_join_in_flight(&runtime, &key, "assign_task", "coord", 2, &dup_tx).await;
    assert!(
        dup.is_none(),
        "in-flight duplicate should join the active execution as a waiter"
    );

    finish_request(&runtime, &state, PersistedSwarmMutationResponse::Done).await;
    match first_rx.recv().await.expect("first response") {
        ServerEvent::Done { id } => assert_eq!(id, 1),
        other => panic!("expected done, got {other:?}"),
    }
    match dup_rx.recv().await.expect("duplicate response") {
        ServerEvent::Done { id } => assert_eq!(id, 2),
        other => panic!("expected done, got {other:?}"),
    }
}

async fn admit(
    runtime: &SwarmMutationRuntime,
    key: &str,
    request_id: u64,
    tx: &tokio::sync::mpsc::UnboundedSender<ServerEvent>,
    replay_final: bool,
) -> Option<PersistedSwarmMutationState> {
    if replay_final {
        begin_or_replay(runtime, key, "assign_task", "coord", request_id, tx).await
    } else {
        begin_or_join_in_flight(runtime, key, "assign_task", "coord", request_id, tx).await
    }
}

fn admission_state(key: &str) -> PersistedSwarmMutationState {
    PersistedSwarmMutationState {
        key: key.to_string(),
        action: "assign_task".to_string(),
        session_id: "coord".to_string(),
        created_at_unix_ms: now_unix_ms(),
        final_response: Some(PersistedSwarmMutationResponse::Done),
    }
}

#[derive(Clone, Copy, Debug)]
enum AdmissionFault {
    CorruptPrimary,
    CorruptWithBackup,
    UnreadablePrimary,
    #[cfg(unix)]
    DanglingPrimary,
    OrphanBackup,
    KeyMismatch,
    ActionMismatch,
    SessionMismatch,
}

async fn assert_admission_rejects(fault: AdmissionFault, replay_final: bool) {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    // Ownership must be checked for pending and final records, before either TTL.
    for expired in [false, true] {
        for finalized in [false, true] {
            let key = request_key(
                "coord",
                "assign_task",
                &[format!("{fault:?}-{expired}-{finalized}")],
            );
            let path = state_path(SWARM_MUTATION_DIR, &key);
            let backup = path.with_extension("bak");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut state = admission_state(&key);
            if expired {
                state.created_at_unix_ms = 0;
            }
            if !finalized {
                state.final_response = None;
            }
            match fault {
                AdmissionFault::KeyMismatch => state.key = "other-key".to_string(),
                AdmissionFault::ActionMismatch => state.action = "spawn".to_string(),
                AdmissionFault::SessionMismatch => state.session_id = "other-session".to_string(),
                _ => {}
            }
            let bytes = serde_json::to_vec(&state).unwrap();
            match fault {
                AdmissionFault::CorruptPrimary => std::fs::write(&path, b"{broken").unwrap(),
                AdmissionFault::CorruptWithBackup => {
                    std::fs::write(&path, b"{broken").unwrap();
                    std::fs::write(&backup, &bytes).unwrap();
                }
                AdmissionFault::UnreadablePrimary => {
                    std::fs::create_dir(&path).unwrap();
                    std::fs::write(path.join("evidence"), b"preserve me").unwrap();
                }
                #[cfg(unix)]
                AdmissionFault::DanglingPrimary => {
                    std::os::unix::fs::symlink(path.with_extension("missing"), &path).unwrap();
                }
                AdmissionFault::OrphanBackup => std::fs::write(&backup, &bytes).unwrap(),
                _ => {
                    std::fs::write(&path, &bytes).unwrap();
                    std::fs::write(&backup, b"preserve backup evidence").unwrap();
                }
            }
            let primary_before = std::fs::read(&path).map_err(|err| err.kind());
            let backup_before = std::fs::read(&backup).map_err(|err| err.kind());
            let runtime = SwarmMutationRuntime::default();
            // A repeated rejection must neither acquire a claim nor join a poisoned waiter.
            for request_id in [71, 72] {
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                let grant = admit(&runtime, &key, request_id, &tx, replay_final).await;
                assert!(
                    grant.is_none(),
                    "invalid state granted execution: {fault:?}"
                );
                match rx.try_recv().expect("invalid state must emit an error") {
                    ServerEvent::Error { id, message, .. } => {
                        assert_eq!(id, request_id);
                        assert!(!message.is_empty());
                    }
                    other => panic!("expected admission error for {fault:?}, got {other:?}"),
                }
                assert!(
                    rx.try_recv().is_err(),
                    "only one error response is expected"
                );
                let sync = runtime.sync.read().await;
                assert!(
                    sync.active_keys.is_empty(),
                    "rejection acquired an active claim"
                );
                assert!(sync.waiters.is_empty(), "rejection registered a waiter");
                assert_eq!(
                    std::fs::read(&path).map_err(|err| err.kind()),
                    primary_before
                );
                assert_eq!(
                    std::fs::read(&backup).map_err(|err| err.kind()),
                    backup_before
                );
                #[cfg(unix)]
                if matches!(fault, AdmissionFault::DanglingPrimary) {
                    let target = path.with_extension("missing");
                    assert!(std::fs::symlink_metadata(&path).unwrap().is_symlink());
                    assert_eq!(std::fs::read_link(&path).unwrap(), target);
                    assert_eq!(
                        std::fs::symlink_metadata(&target).unwrap_err().kind(),
                        std::io::ErrorKind::NotFound
                    );
                    assert_eq!(
                        std::fs::symlink_metadata(&backup).unwrap_err().kind(),
                        std::io::ErrorKind::NotFound
                    );
                }
                if matches!(fault, AdmissionFault::UnreadablePrimary) {
                    assert!(path.is_dir());
                    assert_eq!(
                        std::fs::read(path.join("evidence")).unwrap(),
                        b"preserve me"
                    );
                }
            }
        }
    }
}

macro_rules! admission_rejection_tests {
    ($replay:ident, $control:ident, $fault:ident) => {
        #[tokio::test]
        async fn $replay() {
            assert_admission_rejects(AdmissionFault::$fault, true).await;
        }
        #[tokio::test]
        async fn $control() {
            assert_admission_rejects(AdmissionFault::$fault, false).await;
        }
    };
}

admission_rejection_tests!(
    durable_mutation_admission_corrupt_primary_replay,
    durable_mutation_admission_corrupt_primary_control,
    CorruptPrimary
);
admission_rejection_tests!(
    durable_mutation_admission_corrupt_with_backup_replay,
    durable_mutation_admission_corrupt_with_backup_control,
    CorruptWithBackup
);
admission_rejection_tests!(
    durable_mutation_admission_unreadable_primary_replay,
    durable_mutation_admission_unreadable_primary_control,
    UnreadablePrimary
);
admission_rejection_tests!(
    durable_mutation_admission_orphan_backup_replay,
    durable_mutation_admission_orphan_backup_control,
    OrphanBackup
);
#[cfg(unix)]
admission_rejection_tests!(
    durable_mutation_admission_dangling_primary_replay,
    durable_mutation_admission_dangling_primary_control,
    DanglingPrimary
);
admission_rejection_tests!(
    durable_mutation_admission_key_mismatch_replay,
    durable_mutation_admission_key_mismatch_control,
    KeyMismatch
);
admission_rejection_tests!(
    durable_mutation_admission_action_mismatch_replay,
    durable_mutation_admission_action_mismatch_control,
    ActionMismatch
);
admission_rejection_tests!(
    durable_mutation_admission_session_mismatch_replay,
    durable_mutation_admission_session_mismatch_control,
    SessionMismatch
);

#[tokio::test]
async fn durable_mutation_admission_closed_receiver_still_rejects() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    for replay_final in [true, false] {
        let key = request_key("coord", "assign_task", &[format!("closed-{replay_final}")]);
        let path = state_path(SWARM_MUTATION_DIR, &key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{broken").unwrap();
        let runtime = SwarmMutationRuntime::default();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        drop(rx);

        assert!(admit(&runtime, &key, 96, &tx, replay_final).await.is_none());
        let sync = runtime.sync.read().await;
        assert!(sync.active_keys.is_empty());
        assert!(sync.waiters.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    }
}

#[tokio::test]
async fn durable_mutation_admission_replays_after_runtime_recreation() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let key = request_key("coord", "assign_task", &["recreated".to_string()]);
    let runtime = SwarmMutationRuntime::default();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let state = admit(&runtime, &key, 1, &tx, true).await.unwrap();
    finish_request(&runtime, &state, PersistedSwarmMutationResponse::Done).await;
    assert!(matches!(
        rx.try_recv().unwrap(),
        ServerEvent::Done { id: 1 }
    ));
    drop(runtime);
    let path = state_path(SWARM_MUTATION_DIR, &key);
    let before = std::fs::read(&path).unwrap();
    let runtime = SwarmMutationRuntime::default();
    assert!(admit(&runtime, &key, 91, &tx, true).await.is_none());
    assert!(matches!(
        rx.try_recv().unwrap(),
        ServerEvent::Done { id: 91 }
    ));
    let sync = runtime.sync.read().await;
    assert!(sync.active_keys.is_empty());
    assert!(sync.waiters.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
async fn durable_mutation_admission_control_retry_after_runtime_recreation() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let key = request_key("coord", "assign_task", &["control-recreated".to_string()]);
    let runtime = SwarmMutationRuntime::default();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let state = admit(&runtime, &key, 1, &tx, false).await.unwrap();
    finish_request(&runtime, &state, PersistedSwarmMutationResponse::Done).await;
    assert!(matches!(
        rx.try_recv().unwrap(),
        ServerEvent::Done { id: 1 }
    ));
    drop(runtime);
    let runtime = SwarmMutationRuntime::default();
    let retry = admit(&runtime, &key, 92, &tx, false).await.unwrap();
    assert!(retry.final_response.is_none());
    assert!(rx.try_recv().is_err(), "deliberate retry must not replay");
    assert!(admit(&runtime, &key, 93, &tx, false).await.is_none());
    assert!(rx.try_recv().is_err(), "duplicate must wait");
    finish_request(&runtime, &retry, PersistedSwarmMutationResponse::Done).await;
    assert!(matches!(
        rx.try_recv().unwrap(),
        ServerEvent::Done { id: 92 }
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        ServerEvent::Done { id: 93 }
    ));
    let sync = runtime.sync.read().await;
    assert!(sync.active_keys.is_empty());
    assert!(sync.waiters.is_empty());
}

#[tokio::test]
async fn durable_mutation_admission_missing_and_expired_matching_state_start_fresh() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    for replay_final in [true, false] {
        for persisted in [None, Some(false), Some(true)] {
            let key = request_key(
                "coord",
                "assign_task",
                &[format!("{replay_final}-{persisted:?}")],
            );
            if let Some(finalized) = persisted {
                let mut state = admission_state(&key);
                state.created_at_unix_ms = 0;
                if !finalized {
                    state.final_response = None;
                }
                // Exercise ordinary storage's backup as well as the expired primary.
                super::save_state(&state).expect("persist expired fixture state");
                super::save_state(&state).expect("persist expired fixture state again");
            }
            let runtime = SwarmMutationRuntime::default();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let state = admit(&runtime, &key, 94, &tx, replay_final).await.unwrap();
            assert_eq!(state.key, key);
            assert_eq!(state.action, "assign_task");
            assert_eq!(state.session_id, "coord");
            assert!(state.created_at_unix_ms > 0);
            assert!(state.final_response.is_none());
            assert!(rx.try_recv().is_err());
            assert!(admit(&runtime, &key, 95, &tx, replay_final).await.is_none());
            finish_request(&runtime, &state, PersistedSwarmMutationResponse::Done).await;
            assert!(matches!(
                rx.try_recv().unwrap(),
                ServerEvent::Done { id: 94 }
            ));
            assert!(matches!(
                rx.try_recv().unwrap(),
                ServerEvent::Done { id: 95 }
            ));
        }
    }
}

// The primary component fits NAME_MAX, but even the shortest random temporary
// component used by write_json_fast does not. No permissions or timing race.
#[cfg(unix)]
async fn assert_pending_save_failure(
    replay_final: bool,
    persisted: Option<(bool, bool)>, // (expired, finalized)
    closed_receiver: bool,
) {
    let (_env, runtime_dir) = RuntimeEnvGuard::new();
    let requester = "r".repeat(233);
    let key = request_key(&requester, "assign_task", &["pending-save".to_string()]);
    let path = state_path(SWARM_MUTATION_DIR, &key);
    assert!(path.starts_with(runtime_dir.path()));
    assert_eq!(key.len(), 250, "fixture requires a 250-byte request key");
    assert_eq!(path.file_name().unwrap().len(), 255);
    assert!(path.with_extension("tmp.1.0").file_name().unwrap().len() > 255);

    let mut state = admission_state(&key);
    state.session_id = requester.clone();
    state.final_response = None;
    // Prove that the primary is readable/writable independently of the checked
    // writer. Keep this probe separate from the genuinely missing-state case.
    let probe = runtime_dir
        .path()
        .join("probe")
        .join(path.file_name().unwrap());
    std::fs::create_dir_all(probe.parent().unwrap()).unwrap();
    let bytes = serde_json::to_vec(&state).unwrap();
    std::fs::write(&probe, &bytes).expect("fixture primary must be writable");
    assert_eq!(std::fs::read(&probe).unwrap(), bytes);
    let error = crate::storage::write_json_fast(&probe, &state)
        .expect_err("fixture checked writer must fail before primary replacement");
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::ENAMETOOLONG),
        "fixture must fail at temporary file creation, not during setup"
    );
    assert_eq!(std::fs::read(&probe).unwrap(), bytes);
    assert!(!probe.with_extension("bak").exists());

    let backup = path.with_extension("bak");
    if let Some((expired, finalized)) = persisted {
        if expired {
            state.created_at_unix_ms = 0;
        }
        if finalized {
            state.final_response = Some(PersistedSwarmMutationResponse::Done);
        }
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
        std::fs::write(&backup, b"preserve prior backup evidence").unwrap();
    }
    let primary_before = std::fs::read(&path).map_err(|err| err.kind());
    let backup_before = std::fs::read(&backup).map_err(|err| err.kind());
    let runtime = SwarmMutationRuntime::default();
    for request_id in [171, 172] {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut rx = if closed_receiver {
            drop(rx);
            None
        } else {
            Some(rx)
        };
        let grant = if replay_final {
            begin_or_replay(&runtime, &key, "assign_task", &requester, request_id, &tx).await
        } else {
            begin_or_join_in_flight(&runtime, &key, "assign_task", &requester, request_id, &tx)
                .await
        };
        assert!(
            grant.is_none(),
            "failed pending save must not grant execution"
        );
        if let Some(rx) = rx.as_mut() {
            match rx
                .try_recv()
                .expect("failed pending save must emit an error")
            {
                ServerEvent::Error {
                    id,
                    message,
                    retry_after_secs,
                } => {
                    assert_eq!(id, request_id);
                    assert!(!message.is_empty());
                    assert_eq!(retry_after_secs, None);
                }
                other => panic!("expected pending-save admission error, got {other:?}"),
            }
            assert!(
                rx.try_recv().is_err(),
                "rejection must not also emit success"
            );
        }
        let sync = runtime.sync.read().await;
        assert!(
            sync.active_keys.is_empty(),
            "failed pending save leaked a claim"
        );
        assert!(
            sync.waiters.is_empty(),
            "failed pending save leaked a waiter"
        );
        assert_eq!(
            std::fs::read(&path).map_err(|err| err.kind()),
            primary_before
        );
        assert_eq!(
            std::fs::read(&backup).map_err(|err| err.kind()),
            backup_before
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn pending_save_failure_missing_replay() {
    assert_pending_save_failure(true, None, false).await;
}

#[cfg(unix)]
#[tokio::test]
async fn pending_save_failure_missing_control() {
    assert_pending_save_failure(false, None, false).await;
}

#[cfg(unix)]
#[tokio::test]
async fn pending_save_failure_expired_matching_state() {
    for replay_final in [true, false] {
        for finalized in [false, true] {
            assert_pending_save_failure(replay_final, Some((true, finalized)), false).await;
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn pending_save_failure_control_replaces_recent_final() {
    assert_pending_save_failure(false, Some((false, true)), false).await;
}

#[cfg(unix)]
#[tokio::test]
async fn pending_save_failure_closed_receiver() {
    for replay_final in [true, false] {
        assert_pending_save_failure(replay_final, None, true).await;
    }
}

const COMPLETION_SAVE_UNCERTAINTY: &str = "Swarm mutation completion persistence failed; effects may have occurred; this key is blocked in this runtime; inspect/reconcile rather than retry; restarting does not establish safety.";

fn assert_completion_save_uncertainty(event: ServerEvent, expected_id: u64) {
    match event {
        ServerEvent::Error {
            id,
            message,
            retry_after_secs,
        } => {
            assert_eq!(id, expected_id);
            assert_eq!(message, COMPLETION_SAVE_UNCERTAINTY);
            assert_eq!(retry_after_secs, None);
        }
        other => panic!("expected completion-persistence uncertainty, got {other:?}"),
    }
}

async fn assert_completion_save_failure(
    response: PersistedSwarmMutationResponse,
    closed_original: bool,
) {
    let (_env, runtime_dir) = RuntimeEnvGuard::new();
    let runtime = SwarmMutationRuntime::default();
    let key = request_key("coord", "assign_task", &["final-save".to_string()]);
    let path = state_path(SWARM_MUTATION_DIR, &key);
    assert!(path.starts_with(runtime_dir.path()));
    assert!(
        key.len() < 100,
        "final-save fixture must use an ordinary short key"
    );
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (closed_tx, closed_rx) = tokio::sync::mpsc::unbounded_channel();
    drop(closed_rx);
    let original_tx = if closed_original { &closed_tx } else { &tx };
    let state = admit(&runtime, &key, 201, original_tx, true)
        .await
        .expect("actual admission must persist pending state before obstruction");
    assert!(admit(&runtime, &key, 202, &closed_tx, true).await.is_none());
    assert!(admit(&runtime, &key, 203, &tx, false).await.is_none());
    assert!(rx.try_recv().is_err(), "in-flight duplicates must wait");
    {
        let sync = runtime.sync.read().await;
        assert!(sync.active_keys.contains(&key));
        assert_eq!(sync.waiters[&key].len(), 3);
    }
    let pending_bytes = std::fs::read(&path).unwrap();
    let pending: PersistedSwarmMutationState = serde_json::from_slice(&pending_bytes).unwrap();
    assert_eq!(pending.key, key);
    assert_eq!(pending.action, "assign_task");
    assert_eq!(pending.session_id, "coord");
    assert!(pending.final_response.is_none());

    // Move only this isolated mutation directory, preserving the admitted
    // primary. A regular file at its former path makes the real final writer
    // fail before replacement, without permissions, hooks, or timing races.
    let mutation_dir = path.parent().unwrap();
    let retained_dir = runtime_dir.path().join("retained-mutations");
    let retained_path = retained_dir.join(path.file_name().unwrap());
    std::fs::rename(mutation_dir, &retained_dir).unwrap();
    std::fs::write(mutation_dir, b"block final state directory").unwrap();
    assert!(mutation_dir.is_file());
    assert_eq!(std::fs::read(&retained_path).unwrap(), pending_bytes);
    finish_request(&runtime, &state, response).await;
    assert_eq!(std::fs::read(&retained_path).unwrap(), pending_bytes);
    std::fs::remove_file(mutation_dir).unwrap();
    std::fs::rename(&retained_dir, mutation_dir).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), pending_bytes);

    if !closed_original {
        assert_completion_save_uncertainty(rx.try_recv().expect("original response"), 201);
    }
    assert_completion_save_uncertainty(rx.try_recv().expect("live waiter response"), 203);
    assert!(
        rx.try_recv().is_err(),
        "must emit no original success or business error"
    );
    {
        let sync = runtime.sync.read().await;
        assert!(
            sync.active_keys.is_empty(),
            "failed completion leaked its claim"
        );
        assert!(sync.waiters.is_empty(), "failed completion leaked waiters");
    }

    // Repair precedes every duplicate: strict-read failure must not masquerade
    // as runtime quarantine. A cloned runtime shares that terminal block.
    let same_runtime = runtime.clone();
    for finalized in [false, true] {
        for expired in [false, true] {
            let mut evidence = pending.clone();
            if finalized {
                evidence.final_response = Some(PersistedSwarmMutationResponse::Done);
            }
            if expired {
                evidence.created_at_unix_ms = 0;
            }
            let evidence_bytes = serde_json::to_vec(&evidence).unwrap();
            std::fs::write(&path, &evidence_bytes).unwrap();
            for (request_id, replay_final) in [(204, true), (205, false)] {
                assert!(
                    admit(&same_runtime, &key, request_id, &tx, replay_final)
                        .await
                        .is_none(),
                    "uncertain key must not grant execution after repair or TTL expiry"
                );
                assert_completion_save_uncertainty(
                    rx.try_recv().expect("blocked response"),
                    request_id,
                );
                assert!(rx.try_recv().is_err());
                assert_eq!(std::fs::read(&path).unwrap(), evidence_bytes);
                let sync = runtime.sync.read().await;
                assert!(sync.active_keys.is_empty());
                assert!(sync.waiters.is_empty());
            }
        }
    }
    // The fixed response must precede disk inspection, not expose a subsequent
    // unrelated storage error. Closed receivers cannot clear the block either.
    std::fs::write(&path, b"{subsequent-corruption").unwrap();
    for replay_final in [true, false] {
        assert!(
            admit(&runtime, &key, 206, &closed_tx, replay_final)
                .await
                .is_none()
        );
        assert!(
            admit(&runtime, &key, 207, &tx, replay_final)
                .await
                .is_none()
        );
        assert_completion_save_uncertainty(rx.try_recv().expect("blocked before disk read"), 207);
        assert!(rx.try_recv().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{subsequent-corruption");
    }

    // Characterize the intentionally unresolved restart risk: the block is
    // runtime-only, and the preserved pending record still grants a new runtime.
    std::fs::write(&path, &pending_bytes).unwrap();
    let fresh_runtime = SwarmMutationRuntime::default();
    let fresh = admit(&fresh_runtime, &key, 208, &tx, true)
        .await
        .expect("runtime quarantine does not establish restart safety");
    assert!(fresh.final_response.is_none());
    assert!(rx.try_recv().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), pending_bytes);
    assert!(fresh_runtime.sync.read().await.active_keys.contains(&key));
    let sync = runtime.sync.read().await;
    assert!(sync.active_keys.is_empty());
    assert!(sync.waiters.is_empty());
}

#[tokio::test]
async fn completion_save_failure_done_waiters_and_runtime_quarantine() {
    assert_completion_save_failure(PersistedSwarmMutationResponse::Done, false).await;
}

#[tokio::test]
async fn completion_save_failure_assign_task_waiters_and_runtime_quarantine() {
    assert_completion_save_failure(
        PersistedSwarmMutationResponse::AssignTask {
            task_id: "task-1".to_string(),
            target_session: "worker-1".to_string(),
        },
        false,
    )
    .await;
}

#[tokio::test]
async fn completion_save_failure_spawn_waiters_and_runtime_quarantine() {
    assert_completion_save_failure(
        PersistedSwarmMutationResponse::Spawn {
            new_session_id: "child-1".to_string(),
        },
        false,
    )
    .await;
}

#[tokio::test]
async fn completion_save_failure_business_error_is_not_reclassified() {
    assert_completion_save_failure(
        PersistedSwarmMutationResponse::Error {
            message: "No runnable unassigned tasks; No ready or completed swarm agents; Swarm member limit reached; duplicate node id 'task-1'".to_string(),
            retry_after_secs: Some(17),
        },
        false,
    )
    .await;
}

#[tokio::test]
async fn completion_save_failure_closed_original_still_notifies_live_waiter() {
    assert_completion_save_failure(PersistedSwarmMutationResponse::Done, true).await;
}
