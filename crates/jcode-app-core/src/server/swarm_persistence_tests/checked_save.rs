use super::*;

#[test]
fn checked_swarm_save_core_written_saves_exact_candidate() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);
    let mut plan = VersionedPlan::new();
    plan.version = 7;
    plan.participants.insert("worker".to_string());
    plan.items.push(crate::plan::PlanItem {
        content: "candidate".to_string(),
        status: "queued".to_string(),
        priority: "high".to_string(),
        id: "task".to_string(),
        subsystem: None,
        file_scope: Vec::new(),
        blocked_by: Vec::new(),
        assigned_to: Some("worker".to_string()),
    });
    plan.task_progress.insert(
        "task".to_string(),
        SwarmTaskProgress {
            assigned_session_id: Some("worker".to_string()),
            assigned_at_unix_ms: Some(123),
            ..SwarmTaskProgress::default()
        },
    );
    // A new primary and an equal-version replacement are both writable.
    for content in ["candidate", "same-version update"] {
        plan.items[0].content = content.to_string();
        assert_eq!(
            persist_swarm_state_checked("checked-written", Some(&plan), Some("coord"), &[])
                .unwrap(),
            SwarmSaveOutcome::Written,
        );
        let saved: PersistedSwarmState =
            serde_json::from_slice(&std::fs::read(state_path("checked-written")).unwrap()).unwrap();
        assert_eq!(saved.coordinator_session_id.as_deref(), Some("coord"));
        assert_eq!(
            serde_json::to_value(saved.plan.unwrap()).unwrap(),
            serde_json::to_value(to_persisted_plan(&plan)).unwrap(),
        );
    }
}

#[test]
fn checked_swarm_save_core_skips_newer_plan() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);
    let mut plan = VersionedPlan::new();
    plan.version = 100;
    persist_swarm_state("checked-stale", Some(&plan), Some("coord"), &[]);
    let before = std::fs::read(state_path("checked-stale")).unwrap();
    plan.version = 8;
    assert_eq!(
        persist_swarm_state_checked("checked-stale", Some(&plan), Some("other"), &[]).unwrap(),
        SwarmSaveOutcome::SkippedStale,
    );
    assert_eq!(std::fs::read(state_path("checked-stale")).unwrap(), before);
    let _: () = persist_swarm_state("checked-stale", Some(&plan), Some("other"), &[]);
    assert_eq!(std::fs::read(state_path("checked-stale")).unwrap(), before);
}

#[test]
fn checked_swarm_save_core_skips_empty_and_legacy_dissolves() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);
    let path = state_path("checked-empty");
    assert_eq!(
        persist_swarm_state_checked("checked-empty", None, None, &[]).unwrap(),
        SwarmSaveOutcome::SkippedEmpty,
    );
    assert!(!path.exists());
    persist_swarm_state("checked-empty", None, Some("coord"), &[]);
    persist_swarm_state("checked-empty", None, Some("next"), &[]);
    let before = std::fs::read(&path).unwrap();
    let backup = std::fs::read(path.with_extension("bak")).unwrap();
    assert_eq!(
        persist_swarm_state_checked("checked-empty", None, None, &[]).unwrap(),
        SwarmSaveOutcome::SkippedEmpty,
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(path.with_extension("bak")).unwrap(), backup);
    let _: () = persist_swarm_state("checked-empty", None, None, &[]);
    assert!(!path.exists());
    assert!(!path.with_extension("bak").exists());
}

#[test]
fn checked_swarm_save_core_reports_io_error_and_legacy_is_best_effort() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);
    std::fs::create_dir_all(storage::durable_state_dir()).unwrap();
    std::fs::write(state_dir(), b"not a directory").unwrap();
    let plan = VersionedPlan::new();
    assert!(persist_swarm_state_checked("checked-io", Some(&plan), Some("coord"), &[]).is_err());
    let _: () = persist_swarm_state("checked-io", Some(&plan), Some("coord"), &[]);
    assert_eq!(std::fs::read(state_dir()).unwrap(), b"not a directory");
}

#[test]
fn checked_swarm_save_core_preserves_legacy_primary_recovery_semantics() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);
    std::fs::create_dir_all(state_dir()).unwrap();
    let mut candidate = VersionedPlan::new();
    candidate.version = 7;
    let mut newer = candidate.clone();
    newer.version = 100;
    // Missing primary ignores backup. Corrupt primary recovers a readable
    // backup (including copying it to primary) before the version comparison.
    // Without readable evidence, both APIs keep the legacy best-effort write.
    // This deliberately does not claim strict primary authority.
    for (label, corrupt, backup) in [
        ("missing", false, false),
        ("corrupt", true, false),
        ("missing-backup", false, true),
        ("corrupt-backup", true, true),
    ] {
        for checked in [true, false] {
            let swarm_id = format!("checked-{label}-{checked}");
            let path = state_path(&swarm_id);
            if corrupt {
                std::fs::write(&path, b"{broken").unwrap();
            }
            if backup {
                let state = PersistedSwarmState {
                    swarm_id: swarm_id.clone(),
                    plan: Some(to_persisted_plan(&newer)),
                    coordinator_session_id: Some("old".to_string()),
                    members: Vec::new(),
                    updated_at_unix_ms: now_unix_ms(),
                };
                std::fs::write(
                    path.with_extension("bak"),
                    serde_json::to_vec(&state).unwrap(),
                )
                .unwrap();
            }
            let recovered_newer = corrupt && backup;
            if checked {
                assert_eq!(
                    persist_swarm_state_checked(&swarm_id, Some(&candidate), Some("coord"), &[])
                        .unwrap(),
                    if recovered_newer {
                        SwarmSaveOutcome::SkippedStale
                    } else {
                        SwarmSaveOutcome::Written
                    },
                    "{label}",
                );
            } else {
                let _: () = persist_swarm_state(&swarm_id, Some(&candidate), Some("coord"), &[]);
            }
            let saved: PersistedSwarmState =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(
                saved.plan.unwrap().version,
                if recovered_newer { 100 } else { 7 }
            );
            assert_eq!(
                saved.coordinator_session_id.as_deref(),
                Some(if recovered_newer { "old" } else { "coord" })
            );
        }
    }
}

/// A persist that captured an older plan must not overwrite a newer durable
/// plan when the calls complete out of order.
///
/// This test parks persist A inside `load_runtime` at `members.read()`
/// (after A has already cloned the v5 plan) behind a held `members.write()`
/// gate, then performs mutator B's work while A is parked: bump the
/// in-memory plan to v6 and run B's persist half (`persist_swarm_state` with
/// the v6 runtime, exactly what B's unblocked `persist_swarm_state_for` does
/// on another worker thread, where its uncontended lock reads resolve
/// without suspending). v6 is then durably on disk. Releasing A regresses
/// the durable snapshot back to v5.
///
/// Post-restart impact: `Server::new` seeds `SwarmState` from
/// `load_runtime_state()` and `recover_headless_sessions_on_startup`
/// (server.rs:584-918) drives recovery from that state, so a regressed
/// snapshot silently restores the older plan: work completed between v5 and
/// v6 flips back to queued/running_stale and newer node_meta artifacts are
/// lost.
///
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn stale_persist_cannot_regress_newer_plan_version() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let _env = test_env(&dir);

    let mut plan = VersionedPlan::new();
    plan.version = 5;
    plan.items = vec![crate::plan::PlanItem {
        content: "task one".to_string(),
        status: "queued".to_string(),
        priority: "medium".to_string(),
        id: "t1".to_string(),
        subsystem: None,
        file_scope: Vec::new(),
        blocked_by: Vec::new(),
        assigned_to: None,
    }];
    let swarm_state = crate::server::SwarmState::new(
        HashMap::new(),
        HashMap::new(),
        HashMap::from([("swarm-race".to_string(), plan)]),
        HashMap::new(),
    );

    // Gate: hold members.write() so persist A parks inside load_runtime at
    // the final members.read(), AFTER it has already cloned the v5 plan
    // under plans.read().
    let gate = swarm_state.members.write().await;

    let a = tokio::spawn({
        let swarm_state = swarm_state.clone();
        async move {
            crate::server::persist_swarm_state_for("swarm-race", &swarm_state).await;
        }
    });
    // Current-thread test runtime: yielding runs A until it parks on the
    // contended members.read().await, past its v5 plan clone.
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }

    // Mutator B: bump the in-memory plan to v6 ...
    let v6_plan = {
        let mut plans = swarm_state.plans.write().await;
        let plan = plans.get_mut("swarm-race").expect("plan");
        plan.version = 6;
        plan.clone()
    };
    // ... and B's persist half runs to completion while A is parked. In
    // production this is B's own persist_swarm_state_for on another worker
    // thread: nothing gates B on A (there is no per-swarm persist lock), so
    // B's load_runtime observes v6 and its synchronous persist_swarm_state
    // lands v6 on disk before A's task is polled again.
    persist_swarm_state("swarm-race", Some(&v6_plan), None, &[]);
    assert_eq!(
        load_runtime_state()
            .plans
            .get("swarm-race")
            .expect("v6 snapshot")
            .version,
        6,
        "v6 must be durably on disk before A resumes"
    );

    // Release A: it resumes with its stale v5 runtime snapshot. The durable
    // version guard must reject that write.
    drop(gate);
    a.await.expect("persist task");

    let primary = storage::read_json::<PersistedSwarmState>(&state_path("swarm-race"))
        .expect("primary snapshot");
    assert_eq!(
        primary.plan.expect("plan").version,
        6,
        "a stale persist must not regress the durable plan version"
    );
}
