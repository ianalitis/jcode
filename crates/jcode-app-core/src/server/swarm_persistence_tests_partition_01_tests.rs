/// Delete-vs-write interleaving between `remove_persisted_swarm_state_for`
/// and a concurrent persist (wiring-audit.bak-resurrection, part b).
///
/// `remove_persisted_swarm_state_for` (server.rs:120) is `load_runtime()
/// .await` followed by an unserialized `remove_swarm_state`. Like the
/// persist inversion race above, `load_runtime` observes the four state
/// maps across multiple await points, so a remover that saw an all-empty
/// (dissolved) runtime can park, lose the race to a swarm re-creation plus
/// persist, then resume and delete the FRESH snapshot the re-creation just
/// wrote. Two failures compound:
///   1. Orphaned live swarm: the recreated swarm (coordinator registered
///      in memory) has no primary snapshot, so a clean restart loses it.
///   2. Zombie resurrection: the persist that the remover clobbered
///      hard-linked the PRE-dissolution snapshot to `.bak`, and
///      `load_runtime_state` reads `.bak` files, so restart restores the
///      stale pre-dissolution state instead.
///
/// Same gate technique as
/// `stale_persist_cannot_regress_newer_plan_version`:
/// park A inside `load_runtime` at the contended `members.read()`, run
/// mutator B's re-creation and persist while A is parked, release A.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn stale_remove_cannot_delete_fresh_snapshot_or_restore_backup() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);

    // The previous incarnation's snapshot is on disk; the swarm has since
    // been dissolved, so the in-memory runtime is empty.
    persist_swarm_state("swarm-del-race", None, Some("coord-stale"), &[]);
    let swarm_state = crate::server::SwarmState::new(
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    );

    // Gate: hold members.write() so remover A parks inside load_runtime at
    // the final members.read(), AFTER it has already observed the
    // dissolved (all-empty) plans/coordinators/swarms_by_id state.
    let gate = swarm_state.members.write().await;

    let a = tokio::spawn({
        let swarm_state = swarm_state.clone();
        async move {
            crate::server::remove_persisted_swarm_state_for("swarm-del-race", &swarm_state).await;
        }
    });
    // Current-thread test runtime: yielding runs A until it parks on the
    // contended members.read().await.
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }

    // Mutator B: the swarm is recreated while A is parked. B registers a
    // new coordinator in memory ...
    {
        let mut coordinators = swarm_state.coordinators.write().await;
        coordinators.insert("swarm-del-race".to_string(), "coord-new".to_string());
    }
    // ... and B's persist half runs to completion (in production this is
    // B's own persist_swarm_state_for on another worker thread, whose
    // uncontended lock reads resolve without suspending). This overwrite
    // also hard-links the stale pre-dissolution snapshot to `.bak`.
    persist_swarm_state("swarm-del-race", None, Some("coord-new"), &[]);
    let on_disk = storage::read_json::<PersistedSwarmState>(&state_path("swarm-del-race"))
        .expect("fresh snapshot");
    assert_eq!(
        on_disk.coordinator_session_id.as_deref(),
        Some("coord-new"),
        "fresh snapshot must be durably on disk before A resumes"
    );

    // Release A: its stale all-empty runtime passes has_any_state(), but the
    // compare-and-delete guard must notice that the durable snapshot changed.
    drop(gate);
    a.await.expect("remove task");

    assert!(
        state_path("swarm-del-race").exists(),
        "a stale remove must not delete a freshly persisted snapshot"
    );
    let loaded = load_runtime_state();
    assert_eq!(
        loaded.coordinators.get("swarm-del-race"),
        Some(&"coord-new".to_string()),
        "restart must restore the fresh incarnation, not its stale backup"
    );
}
