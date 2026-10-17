use super::tests::swarm_member;
use super::*;

#[tokio::test]
async fn refresh_swarm_task_staleness_marks_running_tasks_stale_and_heartbeat_revives() {
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        HashSet::from(["worker".to_string()]),
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::new()));
    let now_ms = now_unix_ms();
    let stale_age_ms = super::swarm_task_stale_after().as_millis() as u64 + 5_000;
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        "swarm-1".to_string(),
        VersionedPlan {
            items: vec![PlanItem {
                content: "task".to_string(),
                status: "running".to_string(),
                priority: "medium".to_string(),
                id: "task-1".to_string(),
                subsystem: None,
                file_scope: Vec::new(),
                blocked_by: Vec::new(),
                assigned_to: Some("worker".to_string()),
            }],
            version: 1,
            participants: HashSet::from(["worker".to_string()]),
            task_progress: HashMap::from([(
                "task-1".to_string(),
                crate::server::SwarmTaskProgress {
                    assigned_session_id: Some("worker".to_string()),
                    started_at_unix_ms: Some(now_ms.saturating_sub(stale_age_ms)),
                    last_heartbeat_unix_ms: Some(now_ms.saturating_sub(stale_age_ms)),
                    ..Default::default()
                },
            )]),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let (worker, _worker_rx) = swarm_member("worker", "agent", true);
    swarm_members
        .write()
        .await
        .insert("worker".to_string(), worker);

    refresh_swarm_task_staleness(
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
    )
    .await;

    {
        let plans = swarm_plans.read().await;
        let plan = plans.get("swarm-1").expect("plan");
        assert_eq!(plan.items[0].status, "running_stale");
        assert!(
            plan.task_progress
                .get("task-1")
                .and_then(|progress| progress.stale_since_unix_ms)
                .is_some()
        );
    }

    // A queued event from the displaced worker must not take ownership back
    // or revive the successor's stale assignment.
    let before = {
        let mut plans = swarm_plans.write().await;
        let plan = plans.get_mut("swarm-1").expect("plan");
        plan.items[0].assigned_to = Some("successor".to_string());
        plan.task_progress
            .get_mut("task-1")
            .expect("progress")
            .assigned_session_id = Some("successor".to_string());
        (plan.items.clone(), plan.task_progress.clone(), plan.version)
    };
    let rejected = touch_swarm_task_progress(
        "swarm-1",
        "task-1",
        Some("worker"),
        Some("late old-worker detail".to_string()),
        Some("late old-worker checkpoint".to_string()),
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
    )
    .await;
    assert_eq!(rejected, None);
    {
        let plans = swarm_plans.read().await;
        let plan = plans.get("swarm-1").expect("plan");
        assert_eq!(
            (plan.items.clone(), plan.task_progress.clone(), plan.version),
            before,
            "displaced worker progress must leave the current assignment unchanged"
        );
    }

    let revived = touch_swarm_task_progress(
        "swarm-1",
        "task-1",
        Some("successor"),
        Some("still working".to_string()),
        Some("checkpoint saved".to_string()),
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
    )
    .await;
    assert_eq!(revived, Some(true));

    let plans = swarm_plans.read().await;
    let plan = plans.get("swarm-1").expect("plan");
    assert_eq!(plan.items[0].status, "running");
    let progress = plan.task_progress.get("task-1").expect("progress");
    assert_eq!(
        progress.checkpoint_summary.as_deref(),
        Some("checkpoint saved")
    );
    assert!(progress.stale_since_unix_ms.is_none());
    drop(plans);

    let accepted = touch_swarm_task_progress(
        "swarm-1",
        "task-1",
        Some("successor"),
        None,
        None,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
    )
    .await;
    assert_eq!(
        accepted,
        Some(false),
        "current running owner stays accepted"
    );

    for (status, item_owner, progress_owner, sender) in [
        (
            "queued",
            Some("successor"),
            Some("successor"),
            Some("successor"),
        ),
        (
            "completed",
            Some("successor"),
            Some("successor"),
            Some("successor"),
        ),
        (
            "failed",
            Some("successor"),
            Some("successor"),
            Some("successor"),
        ),
        ("running", None, Some("successor"), Some("successor")),
        (
            "running",
            Some("other"),
            Some("successor"),
            Some("successor"),
        ),
        ("running", Some("successor"), None, Some("successor")),
        (
            "running",
            Some("successor"),
            Some("other"),
            Some("successor"),
        ),
        ("running", Some("successor"), Some("successor"), None),
    ] {
        let before = {
            let mut plans = swarm_plans.write().await;
            let plan = plans.get_mut("swarm-1").expect("plan");
            plan.items[0].status = status.to_string();
            plan.items[0].assigned_to = item_owner.map(str::to_string);
            plan.task_progress
                .get_mut("task-1")
                .expect("progress")
                .assigned_session_id = progress_owner.map(str::to_string);
            (plan.items.clone(), plan.task_progress.clone(), plan.version)
        };
        let rejected = touch_swarm_task_progress(
            "swarm-1",
            "task-1",
            sender,
            Some("rejected detail".to_string()),
            Some("rejected checkpoint".to_string()),
            &swarm_members,
            &swarms_by_id,
            &swarm_plans,
            &swarm_coordinators,
        )
        .await;
        assert_eq!(
            rejected, None,
            "reject {status}/{item_owner:?}/{progress_owner:?}/{sender:?}"
        );
        let plans = swarm_plans.read().await;
        let plan = plans.get("swarm-1").expect("plan");
        assert_eq!(
            (plan.items.clone(), plan.task_progress.clone(), plan.version),
            before
        );
    }

    // Missing progress must not be recreated by a heartbeat.
    {
        let mut plans = swarm_plans.write().await;
        let plan = plans.get_mut("swarm-1").expect("plan");
        plan.task_progress.clear();
    }
    let rejected = touch_swarm_task_progress(
        "swarm-1",
        "task-1",
        Some("successor"),
        None,
        None,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
    )
    .await;
    assert_eq!(rejected, None);
    assert!(swarm_plans.read().await["swarm-1"].task_progress.is_empty());
}
