/// Regression: a solo deep-mode seeder must be able to complete (and expand) a
/// node it seeded. Seeded nodes are unowned and the assign path refuses
/// self-assignment, so without the handler-level self-claim the seeder's
/// `complete_node` bounced with "does not own node" (observed live 2026-06-30,
/// session_shrimp completing node 'probe').
#[tokio::test]
async fn e2e_solo_seeder_can_complete_its_own_seeded_node() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let mut fx = graph_fixture_named("swarm-self-claim", "coord-sc", "worker-sc").await;
    fx.seed("deep", vec![node_spec("probe", "explore", &[])])
        .await;

    // The seeder completes its own seeded node directly: the handler must
    // auto-claim the unowned queued node instead of rejecting with NotOwner.
    handle_comm_complete_node(
        2,
        fx.coord.clone(),
        "probe".to_string(),
        serde_json::json!({
            "findings": "probe complete",
            "confidence": "high",
            "what_i_did_not_check": ["nothing; probe only"],
        })
        .to_string(),
        &fx.client_tx,
        &fx.swarm_members,
        &fx.swarms_by_id,
        &fx.swarm_plans,
        &fx.swarm_coordinators,
        &fx.event_history,
        &fx.event_counter,
        &fx.swarm_event_tx,
    )
    .await;

    let plans = fx.swarm_plans.read().await;
    let plan = &plans[&fx.swarm_id];
    let probe = plan.items.iter().find(|i| i.id == "probe").unwrap();
    assert_eq!(
        probe.status, "completed",
        "solo seeder must be able to complete its own seeded node"
    );
    assert!(
        plan.node_meta["probe"].artifact_json.is_some(),
        "artifact must be recorded"
    );
}

/// Regression: the self-claim must not let an actor steal a node that is
/// assigned to someone else. The engine's NotOwner check still applies.
#[tokio::test]
async fn e2e_self_claim_does_not_steal_foreign_assignment() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let mut fx = graph_fixture_named("swarm-no-steal", "coord-ns", "worker-ns").await;
    fx.seed("deep", vec![node_spec("task", "explore", &[])])
        .await;
    {
        let mut plans = fx.swarm_plans.write().await;
        let plan = plans.get_mut(&fx.swarm_id).unwrap();
        let item = plan.items.iter_mut().find(|i| i.id == "task").unwrap();
        item.assigned_to = Some(fx.worker.clone());
        item.status = "queued".to_string();
    }

    // The coordinator (not the assignee) tries to complete it -> rejected.
    handle_comm_complete_node(
        2,
        fx.coord.clone(),
        "task".to_string(),
        serde_json::json!({
            "findings": "hijack",
            "confidence": "high",
            "what_i_did_not_check": ["everything"],
        })
        .to_string(),
        &fx.client_tx,
        &fx.swarm_members,
        &fx.swarms_by_id,
        &fx.swarm_plans,
        &fx.swarm_coordinators,
        &fx.event_history,
        &fx.event_counter,
        &fx.swarm_event_tx,
    )
    .await;

    let plans = fx.swarm_plans.read().await;
    let plan = &plans[&fx.swarm_id];
    let task = plan.items.iter().find(|i| i.id == "task").unwrap();
    assert_eq!(
        task.status, "queued",
        "a foreign actor must not complete someone else's assignment"
    );
    assert_eq!(task.assigned_to.as_deref(), Some(fx.worker.as_str()));
}

/// Regression: an assignee whose node was left `queued` (client-attached worker
/// path skips the server-side flip to running) must still be able to
/// complete/expand its own assignment.
#[tokio::test]
async fn e2e_assignee_can_complete_queued_assignment() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let mut fx = graph_fixture_named("swarm-queued-own", "coord-qo", "worker-qo").await;
    fx.seed("deep", vec![node_spec("mine", "explore", &[])])
        .await;
    {
        let mut plans = fx.swarm_plans.write().await;
        let plan = plans.get_mut(&fx.swarm_id).unwrap();
        let item = plan.items.iter_mut().find(|i| i.id == "mine").unwrap();
        // Assigned but never flipped to running (live-client path).
        item.assigned_to = Some(fx.worker.clone());
        item.status = "queued".to_string();
    }

    handle_comm_complete_node(
        2,
        fx.worker.clone(),
        "mine".to_string(),
        serde_json::json!({
            "findings": "did the work",
            "confidence": "high",
            "what_i_did_not_check": ["nothing"],
        })
        .to_string(),
        &fx.client_tx,
        &fx.swarm_members,
        &fx.swarms_by_id,
        &fx.swarm_plans,
        &fx.swarm_coordinators,
        &fx.event_history,
        &fx.event_counter,
        &fx.swarm_event_tx,
    )
    .await;

    let plans = fx.swarm_plans.read().await;
    let plan = &plans[&fx.swarm_id];
    let mine = plan.items.iter().find(|i| i.id == "mine").unwrap();
    assert_eq!(
        mine.status, "completed",
        "the assignee must be able to complete its queued assignment"
    );
}

/// Regression: re-seeding a non-empty deep plan as light is a silent rigor
/// downgrade (drops gates + artifact validation) and must be rejected.
#[tokio::test]
async fn e2e_seed_rejects_light_downgrade_of_nonempty_deep_plan() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let mut fx = graph_fixture_named("swarm-no-downgrade", "coord-nd", "worker-nd").await;
    fx.seed("deep", vec![node_spec("a", "explore", &[])]).await;

    // Attempt the downgrade.
    fx.seed("light", vec![node_spec("b", "explore", &[])]).await;

    let plans = fx.swarm_plans.read().await;
    let plan = &plans[&fx.swarm_id];
    assert_eq!(
        plan.mode, "deep",
        "deep plan must not be downgraded to light"
    );
    assert!(
        plan.items.iter().all(|i| i.id != "b"),
        "the downgrade seed must be rejected wholesale"
    );
    drop(plans);
    let mut saw_downgrade_error = false;
    while let Ok(ev) = fx.client_rx.try_recv() {
        if let ServerEvent::Error { message, .. } = ev
            && message.contains("deep-mode plan")
        {
            saw_downgrade_error = true;
        }
    }
    assert!(saw_downgrade_error, "downgrade must surface a clear error");
}
