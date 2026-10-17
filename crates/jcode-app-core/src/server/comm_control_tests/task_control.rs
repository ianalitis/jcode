/// Exercise the actual consumer, including its detached private runner. The
/// current-thread runtime is declared after the environment guards, so even a
/// failed assertion cancels all inert fixture tasks before restoring the host
/// environment. No production runner handle or test hook is required.
fn check_requeue_start_admission(action: &str, status: &str, stage: &str, change: &str) {
    let (_env, temp) = RuntimeEnvGuard::new();
    let _isolated_env = DetachedTaskEnvGuard(
        ["JCODE_HOME", "JCODE_NO_TELEMETRY", "JCODE_HOOKS_DISABLED"]
            .map(|key| (key, std::env::var_os(key)))
            .into(),
    );
    crate::env::set_var("JCODE_HOME", temp.path().join("home"));
    crate::env::set_var("JCODE_NO_TELEMETRY", "1");
    crate::env::set_var("JCODE_HOOKS_DISABLED", "1");
    let calls = Arc::new(AtomicU64::new(0));
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let provider = Arc::new(TerminalBarrierProvider {
        calls: Arc::clone(&calls),
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
        fail: false,
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap();
    // Unconstrained polling keeps cooperative-budget yields from masquerading
    // as one of the explicit lock/ack barriers in the manually polled request.
    runtime.block_on(tokio::task::unconstrained(async {
        let swarm_id = "requeue-start-admission";
        let worker = "worker";
        let mut agent = Agent::new_with_initial_working_dir(
            provider.clone(), Registry::empty(), temp.path().to_str(),
        );
        agent.set_memory_enabled(false);
        let sessions = Arc::new(RwLock::new(HashMap::from([(
            worker.to_string(), Arc::new(Mutex::new(agent)),
        )])));
        let interrupts = Arc::new(RwLock::new(HashMap::new()));
        let connections = Arc::new(RwLock::new(HashMap::new()));
        let (member_tx, mut member_rx) = mpsc::unbounded_channel();
        let mut actor = owned_member(worker, swarm_id, "ready", "coord");
        actor.event_tx = member_tx.clone();
        let mut coordinator = member("coord", swarm_id, "ready");
        coordinator.role = "coordinator".to_string();
        coordinator.event_tx = member_tx;
        let members = Arc::new(RwLock::new(HashMap::from([
            (worker.to_string(), actor), ("coord".to_string(), coordinator),
        ])));
        let swarms = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(), HashSet::from([worker.to_string(), "coord".to_string()]),
        )])));
        let coordinators = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(), "coord".to_string(),
        )])));
        let mut plan = VersionedPlan::new();
        plan.version = 7;
        plan.mode = "light".to_string();
        plan.participants = HashSet::from([worker.to_string(), "coord".to_string()]);
        let mut item = plan_item("task", status, "high", &["upstream"]);
        item.assigned_to = Some(worker.to_string());
        plan.items = vec![item, plan_item("upstream", "done", "low", &[])];
        plan.node_meta.insert("upstream".to_string(), crate::plan::NodeMeta {
            artifact_json: Some(serde_json::json!({"findings": "original evidence"}).to_string()),
            ..Default::default()
        });
        plan.task_progress.insert("task".to_string(), crate::server::SwarmTaskProgress {
            assigned_session_id: Some(worker.to_string()),
            started_at_unix_ms: Some(2_000),
            last_heartbeat_unix_ms: Some(3_000),
            last_detail: Some("historical detail".to_string()),
            heartbeat_count: Some(7),
            checkpoint_count: Some(2),
            checkpoint_summary: Some("historical checkpoint".to_string()),
            no_artifact_requeues: Some(1),
            dead_assignee_reclaims: Some(2),
            ..Default::default()
        });
        let plans = Arc::new(RwLock::new(HashMap::from([(swarm_id.to_string(), plan)])));
        let history = Arc::new(RwLock::new(VecDeque::new()));
        let counter = Arc::new(AtomicU64::new(7));
        let (event_tx, mut event_rx) = broadcast::channel(64);
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let mutation_runtime = SwarmMutationRuntime::default();
        let snapshot_path = crate::storage::durable_state_dir()
            .join("swarm").join(format!("{swarm_id}.json"));
        crate::server::swarm_persistence::persist_swarm_state(
            swarm_id, Some(&plans.read().await[swarm_id]), Some("coord"), &[],
        );
        assert!(snapshot_path.is_file());

        // All earlier coordinator/snapshot reads are uncontended. One manual
        // poll therefore reaches sessions.read() only AFTER taking the selected
        // task snapshot, without scheduling any detached child.
        let selection_guard = sessions.write().await;
        let selected_content = super::task_snapshot_for(swarm_id, "task", &plans)
            .await.unwrap().content;
        let mut request = Box::pin(handle_comm_task_control(
            901, "coord".to_string(), action.to_string(), "task".to_string(),
            Some(worker.to_string()), None, &client_tx, &sessions, &interrupts,
            &connections, &members, &swarms, &plans, &coordinators, &history,
            &counter, &event_tx, &mutation_runtime,
        ));
        if stage != "initial" {
            assert!(futures::poll!(request.as_mut()).is_pending(), "selection barrier");
        }
        drop(selection_guard);
        let mut request_done = false;
        if stage == "spawn" {
            // tokio::spawn never synchronously polls its child. On this private
            // current-thread runtime, the runner cannot start during this poll.
            // The old handler returns now, the fixed handler must wait for ack.
            request_done = futures::poll!(request.as_mut()).is_ready();
            let locked = plans.read().await;
            let queued = &locked[swarm_id];
            assert_eq!(queued.items[0].status, "queued", "must reach queued publication");
            assert_eq!(queued.task_progress["task"].started_at_unix_ms, Some(2_000));
            assert_eq!(queued.task_progress["task"].heartbeat_count, Some(7));
            assert_eq!(queued.task_progress["task"].checkpoint_count, Some(2));
            assert_eq!(queued.task_progress["task"].last_detail.as_deref(), Some("historical detail"));
            let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&snapshot_path).unwrap()).unwrap();
            assert_eq!(saved["plan"]["items"][0]["status"], "queued");
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
            if change == "ack" {
                assert!(!request_done, "handler returned before detached admission");
                assert!(client_rx.try_recv().is_err(), "no running response before admission");
            }
        }

        {
            let mut locked = plans.write().await;
            if change == "missing_plan" {
                locked.remove(swarm_id);
            } else {
                let plan = locked.get_mut(swarm_id).unwrap();
                match change {
                    "missing_task" => plan.items.retain(|item| item.id != "task"),
                    "missing_progress" => { plan.task_progress.remove("task"); }
                    "item_owner" => plan.items[0].assigned_to = Some("successor".to_string()),
                    "item_owner_absent" => plan.items[0].assigned_to = None,
                    "progress_owner" => plan.task_progress.get_mut("task").unwrap().assigned_session_id = Some("successor".to_string()),
                    "progress_owner_absent" => plan.task_progress.get_mut("task").unwrap().assigned_session_id = None,
                    "raw_content" => plan.items[0].content.push_str(" changed scope"),
                    "deep_content" => plan.mode = "deep".to_string(),
                    "hydrated_content" => plan.node_meta.get_mut("upstream").unwrap().artifact_json = Some(serde_json::json!({"findings": "replacement evidence"}).to_string()),
                    "status" => plan.items[0].status = "done".to_string(),
                    "heartbeat" => plan.task_progress.get_mut("task").unwrap().heartbeat_count = Some(42),
                    "unrelated_version" => plan.version += 1,
                    "valid" | "ack" | "save_error" | "newer_disk" => {}
                    other => panic!("unknown mutation: {other}"),
                }
            }
        }
        if matches!(change, "raw_content" | "deep_content" | "hydrated_content") {
            let current = super::task_snapshot_for(swarm_id, "task", &plans).await.unwrap();
            assert_ne!(current.content, selected_content, "content race must be effective");
        }
        let mut displaced_snapshot = None;
        if change == "save_error" {
            // Move only this fixture's swarm directory aside, preserving the
            // committed queued bytes, then make its original parent unwritable.
            let saved_dir = temp.path().join("saved-swarm");
            std::fs::rename(snapshot_path.parent().unwrap(), &saved_dir).unwrap();
            std::fs::write(snapshot_path.parent().unwrap(), b"not a directory").unwrap();
            let saved_path = saved_dir.join(format!("{swarm_id}.json"));
            displaced_snapshot = Some((saved_path.clone(), std::fs::read(saved_path).unwrap()));
        } else if change == "newer_disk" {
            let mut newer = plans.read().await[swarm_id].clone();
            newer.version = 100;
            crate::server::swarm_persistence::persist_swarm_state(
                swarm_id, Some(&newer), Some("coord"), &[],
            );
        }
        let before_plan = format!("{:?}", *plans.read().await);
        let before_members = members.read().await.clone();
        let before_disk = std::fs::read(&snapshot_path).ok();
        if !request_done {
            tokio::time::timeout(Duration::from_secs(10), request.as_mut())
                .await.expect("bounded real task-control request");
        }
        drop(request);
        let response = tokio::time::timeout(Duration::from_secs(1), client_rx.recv())
            .await.expect("bounded response").expect("response channel");
        let admitted = matches!(change, "valid" | "ack" | "heartbeat" | "unrelated_version");
        if admitted {
            // Positive controls observe real admission and actor completion,
            // rather than treating the old optimistic response as evidence.
            tokio::time::timeout(Duration::from_secs(10), entered.notified())
                .await.expect("inert provider entered");
            assert!(matches!(response, ServerEvent::CommTaskControlResponse { status, .. } if status == "running"));
            assert_eq!(plans.read().await[swarm_id].items[0].status, "running");
            let running: serde_json::Value = serde_json::from_slice(&std::fs::read(&snapshot_path).unwrap()).unwrap();
            assert_eq!(running["plan"]["items"][0]["status"], "running");
            if change == "heartbeat" {
                assert!(plans.read().await[swarm_id].task_progress["task"].heartbeat_count.unwrap_or(0) >= 43, "concurrent heartbeat history must survive admission");
            }
            tokio::time::timeout(Duration::from_secs(10), async {
                while let Some(event) = member_rx.recv().await {
                    if matches!(event, ServerEvent::SwarmStatus { members }
                        if members.iter().any(|member| member.session_id == worker && member.status == "running")) {
                        return;
                    }
                }
                panic!("member channel closed before running publication");
            }).await.expect("debounced running publication");
            release.notify_one();
            tokio::time::timeout(Duration::from_secs(10), async {
                while let Some(event) = member_rx.recv().await {
                    if matches!(event, ServerEvent::SwarmStatus { members }
                        if members.iter().any(|member| member.session_id == worker && member.status == "completed")) {
                        return;
                    }
                }
                panic!("member channel closed before completion publication");
            }).await.expect("debounced completed publication");
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
            let locked = plans.read().await;
            let plan = &locked[swarm_id];
            assert_eq!(plan.items[0].status, "done");
            assert_eq!(plan.task_progress["task"].no_artifact_requeues, Some(1));
            assert_eq!(plan.task_progress["task"].dead_assignee_reclaims, Some(2));
        } else {
            // Event-driven bounded observation covers the real 75ms two-member
            // debounce. No sleeps, polling loops, or immediate-try_recv proof.
            let mut publications = Vec::new();
            let _ = tokio::time::timeout(Duration::from_millis(250), async {
                while let Some(event) = member_rx.recv().await {
                    publications.push(event);
                }
            }).await;
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0, "rejected request reached provider: {stage}/{change}");
            assert_eq!(format!("{:?}", *plans.read().await), before_plan, "rejection must preserve current or legitimately queued state");
            assert_eq!(format!("{:?}", *members.read().await), format!("{before_members:?}"), "rejected admission published member state");
            assert_eq!(std::fs::read(&snapshot_path).ok(), before_disk, "rejection overwrote durable state");
            if let Some((path, bytes)) = displaced_snapshot {
                assert_eq!(std::fs::read(path).unwrap(), bytes);
            }
            assert!(event_rx.try_recv().is_err(), "rejected request emitted actor event");
            assert!(history.read().await.is_empty());
            assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 7);
            if stage == "spawn" && status != "queued" {
                assert!(publications.iter().all(|event| matches!(event,
                    ServerEvent::SwarmPlan { reason, .. } if reason.as_deref() == Some("task_resume")
                )), "only the legitimately committed queued transition may publish: {publications:?}");
            } else {
                assert!(publications.is_empty(), "rejected selection published: {publications:?}");
            }
            assert!(matches!(response, ServerEvent::Error { id: 901, .. }), "rejection falsely reported running: {response:?}");
        }
    }));
    // The production handler intentionally detached its private child. This
    // runtime is its cancellation owner, including on request/assertion failure.
    // On normal exit, verify no runner/agent still retains the inert provider.
    drop(runtime);
    assert_eq!(
        Arc::strong_count(&provider),
        1,
        "fixture leaked its runner's provider"
    );
    assert_eq!(
        Arc::strong_count(&calls),
        2,
        "fixture leaked a provider fork"
    );
}

macro_rules! requeue_start_admission_cases {
    ($($name:ident => ($action:literal, $status:literal, $stage:literal, $change:literal)),+ $(,)?) => {
        $(#[test] fn $name() { check_requeue_start_admission($action, $status, $stage, $change); })+
    };
}

requeue_start_admission_cases! {
    requeue_start_admission_valid_start => ("start", "queued", "selection", "valid"),
    requeue_start_admission_valid_wake => ("wake", "queued", "selection", "valid"),
    requeue_start_admission_valid_resume => ("resume", "queued", "selection", "valid"),
    requeue_start_admission_valid_stale_resume => ("resume", "running_stale", "selection", "valid"),
    requeue_start_admission_valid_running_resume => ("resume", "running", "selection", "valid"),
    requeue_start_admission_heartbeat_not_authority => ("resume", "running_stale", "selection", "heartbeat"),
    requeue_start_admission_unrelated_version_not_authority => ("resume", "running_stale", "selection", "unrelated_version"),
    requeue_start_admission_start_waits_for_ack => ("start", "queued", "spawn", "ack"),
    requeue_start_admission_wake_waits_for_ack => ("wake", "queued", "spawn", "ack"),
    requeue_start_admission_resume_waits_for_ack => ("resume", "queued", "spawn", "ack"),
    requeue_start_admission_initial_missing_plan => ("wake", "queued", "initial", "missing_plan"),
    requeue_start_admission_initial_missing_task => ("wake", "queued", "initial", "missing_task"),
    requeue_start_admission_initial_missing_progress => ("wake", "queued", "initial", "missing_progress"),
    requeue_start_admission_initial_stale_missing_progress => ("resume", "running_stale", "initial", "missing_progress"),
    requeue_start_admission_queued_missing_plan => ("wake", "queued", "selection", "missing_plan"),
    requeue_start_admission_queued_missing_task => ("wake", "queued", "selection", "missing_task"),
    requeue_start_admission_queued_missing_progress => ("wake", "queued", "selection", "missing_progress"),
    requeue_start_admission_queued_item_owner => ("wake", "queued", "selection", "item_owner"),
    requeue_start_admission_queued_item_owner_absent => ("wake", "queued", "selection", "item_owner_absent"),
    requeue_start_admission_queued_progress_owner => ("wake", "queued", "selection", "progress_owner"),
    requeue_start_admission_queued_progress_owner_absent => ("wake", "queued", "selection", "progress_owner_absent"),
    requeue_start_admission_queued_raw_content => ("wake", "queued", "selection", "raw_content"),
    requeue_start_admission_queued_deep_content => ("wake", "queued", "selection", "deep_content"),
    requeue_start_admission_queued_hydrated_content => ("wake", "queued", "selection", "hydrated_content"),
    requeue_start_admission_queued_status => ("wake", "queued", "selection", "status"),
    requeue_start_admission_queued_save_error => ("wake", "queued", "selection", "save_error"),
    requeue_start_admission_queued_newer_disk => ("wake", "queued", "selection", "newer_disk"),
    requeue_start_admission_stale_missing_plan => ("resume", "running_stale", "selection", "missing_plan"),
    requeue_start_admission_stale_missing_task => ("resume", "running_stale", "selection", "missing_task"),
    requeue_start_admission_stale_missing_progress => ("resume", "running_stale", "selection", "missing_progress"),
    requeue_start_admission_stale_item_owner => ("resume", "running_stale", "selection", "item_owner"),
    requeue_start_admission_stale_item_owner_absent => ("resume", "running_stale", "selection", "item_owner_absent"),
    requeue_start_admission_stale_progress_owner => ("resume", "running_stale", "selection", "progress_owner"),
    requeue_start_admission_stale_progress_owner_absent => ("resume", "running_stale", "selection", "progress_owner_absent"),
    requeue_start_admission_stale_raw_content => ("resume", "running_stale", "selection", "raw_content"),
    requeue_start_admission_stale_deep_content => ("resume", "running_stale", "selection", "deep_content"),
    requeue_start_admission_stale_hydrated_content => ("resume", "running_stale", "selection", "hydrated_content"),
    requeue_start_admission_stale_status => ("resume", "running_stale", "selection", "status"),
    requeue_start_admission_stale_save_error => ("resume", "running_stale", "selection", "save_error"),
    requeue_start_admission_stale_newer_disk => ("resume", "running_stale", "selection", "newer_disk"),
    requeue_start_admission_spawn_queued_missing_plan => ("wake", "queued", "spawn", "missing_plan"),
    requeue_start_admission_spawn_queued_missing_task => ("wake", "queued", "spawn", "missing_task"),
    requeue_start_admission_spawn_queued_missing_progress => ("wake", "queued", "spawn", "missing_progress"),
    requeue_start_admission_spawn_queued_item_owner => ("wake", "queued", "spawn", "item_owner"),
    requeue_start_admission_spawn_queued_progress_owner => ("wake", "queued", "spawn", "progress_owner"),
    requeue_start_admission_spawn_queued_raw_content => ("wake", "queued", "spawn", "raw_content"),
    requeue_start_admission_spawn_queued_deep_content => ("wake", "queued", "spawn", "deep_content"),
    requeue_start_admission_spawn_queued_hydrated_content => ("wake", "queued", "spawn", "hydrated_content"),
    requeue_start_admission_spawn_queued_status => ("wake", "queued", "spawn", "status"),
    requeue_start_admission_spawn_queued_save_error => ("wake", "queued", "spawn", "save_error"),
    requeue_start_admission_spawn_queued_newer_disk => ("wake", "queued", "spawn", "newer_disk"),
    requeue_start_admission_spawn_stale_missing_plan => ("resume", "running_stale", "spawn", "missing_plan"),
    requeue_start_admission_spawn_stale_missing_task => ("resume", "running_stale", "spawn", "missing_task"),
    requeue_start_admission_spawn_stale_missing_progress => ("resume", "running_stale", "spawn", "missing_progress"),
    requeue_start_admission_spawn_stale_item_owner => ("resume", "running_stale", "spawn", "item_owner"),
    requeue_start_admission_spawn_stale_progress_owner => ("resume", "running_stale", "spawn", "progress_owner"),
    requeue_start_admission_spawn_stale_raw_content => ("resume", "running_stale", "spawn", "raw_content"),
    requeue_start_admission_spawn_stale_deep_content => ("resume", "running_stale", "spawn", "deep_content"),
    requeue_start_admission_spawn_stale_hydrated_content => ("resume", "running_stale", "spawn", "hydrated_content"),
    requeue_start_admission_spawn_stale_status => ("resume", "running_stale", "spawn", "status"),
    requeue_start_admission_spawn_stale_save_error => ("resume", "running_stale", "spawn", "save_error"),
    requeue_start_admission_spawn_stale_newer_disk => ("resume", "running_stale", "spawn", "newer_disk"),
}

#[tokio::test]
async fn task_control_wake_returns_structured_response_with_plan_summary() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-task-control";
    let requester = "coord";
    let worker = "worker";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let worker_agent = test_agent().await;
    let sessions = Arc::new(RwLock::new(HashMap::from([(
        worker.to_string(),
        worker_agent,
    )])));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "ready")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let mut assigned = plan_item("active-task", "queued", "high", &[]);
    assigned.assigned_to = Some(worker.to_string());
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![assigned, plan_item("next", "queued", "high", &[])],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::from([(
                "active-task".to_string(),
                crate::server::SwarmTaskProgress {
                    assigned_session_id: Some(worker.to_string()),
                    ..Default::default()
                },
            )]),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();

    handle_comm_task_control(
        101,
        requester.to_string(),
        "wake".to_string(),
        "active-task".to_string(),
        Some(worker.to_string()),
        Some("continue".to_string()),
        &client_tx,
        &sessions,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("response") {
        ServerEvent::CommTaskControlResponse {
            id,
            action,
            task_id,
            target_session,
            status,
            summary,
        } => {
            assert_eq!(id, 101);
            assert_eq!(action, "wake");
            assert_eq!(task_id, "active-task");
            assert_eq!(target_session.as_deref(), Some(worker));
            assert_eq!(status, "running");
            assert_eq!(summary.item_count, 2);
            assert!(summary.ready_ids.contains(&"next".to_string()));
        }
        other => panic!("expected CommTaskControlResponse, got {other:?}"),
    }
}

#[tokio::test]
async fn task_control_resume_without_task_id_uses_unique_target_assignment() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-task-control-target";
    let requester = "coord";
    let worker = "worker";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let worker_agent = test_agent().await;
    let sessions = Arc::new(RwLock::new(HashMap::from([(
        worker.to_string(),
        worker_agent,
    )])));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "stopped")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let mut assigned = plan_item("resume-me", "queued", "high", &[]);
    assigned.assigned_to = Some(worker.to_string());
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![assigned],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::from([(
                "resume-me".to_string(),
                crate::server::SwarmTaskProgress {
                    assigned_session_id: Some(worker.to_string()),
                    ..Default::default()
                },
            )]),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();

    handle_comm_task_control(
        102,
        requester.to_string(),
        "resume".to_string(),
        String::new(),
        Some(worker.to_string()),
        None,
        &client_tx,
        &sessions,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("response") {
        ServerEvent::CommTaskControlResponse {
            id,
            action,
            task_id,
            target_session,
            status,
            ..
        } => {
            assert_eq!(id, 102);
            assert_eq!(action, "resume");
            assert_eq!(task_id, "resume-me");
            assert_eq!(target_session.as_deref(), Some(worker));
            assert_eq!(status, "running");
        }
        other => panic!("expected CommTaskControlResponse, got {other:?}"),
    }
}

#[tokio::test]
async fn task_control_without_task_id_rejects_ambiguous_target_assignments() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-task-control-ambiguous";
    let requester = "coord";
    let worker = "worker";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "stopped")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let mut first = plan_item("first", "queued", "high", &[]);
    first.assigned_to = Some(worker.to_string());
    let mut second = plan_item("second", "queued", "high", &[]);
    second.assigned_to = Some(worker.to_string());
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![first, second],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::new(),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();

    handle_comm_task_control(
        103,
        requester.to_string(),
        "resume".to_string(),
        String::new(),
        Some(worker.to_string()),
        None,
        &client_tx,
        &sessions,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("response") {
        ServerEvent::Error { id, message, .. } => {
            assert_eq!(id, 103);
            assert!(message.contains("Multiple tasks assigned"), "{message}");
            assert!(message.contains("first"), "{message}");
            assert!(message.contains("second"), "{message}");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

/// Regression: resuming a plain-'running' task whose agent is busy must be
/// rejected BEFORE any plan mutation. The old ordering requeued the live task
/// first (flipping it to 'queued' and rewriting its progress record) and only
/// then discovered the agent was busy, so a rejected resume still mangled the
/// running task's state and run history.
#[tokio::test]
async fn task_control_resume_busy_agent_rejects_without_mutating_plan() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-task-control-busy";
    let requester = "coord";
    let worker = "worker";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let worker_agent = test_agent().await;
    let sessions = Arc::new(RwLock::new(HashMap::from([(
        worker.to_string(),
        Arc::clone(&worker_agent),
    )])));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "running")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let mut assigned = plan_item("busy-task", "running", "high", &[]);
    assigned.assigned_to = Some(worker.to_string());
    let prior_progress = crate::server::SwarmTaskProgress {
        assigned_session_id: Some(worker.to_string()),
        assignment_summary: Some("original assignment".to_string()),
        assigned_at_unix_ms: Some(1_000),
        started_at_unix_ms: Some(2_000),
        last_heartbeat_unix_ms: Some(3_000),
        heartbeat_count: Some(7),
        checkpoint_count: Some(2),
        checkpoint_summary: Some("halfway".to_string()),
        ..crate::server::SwarmTaskProgress::default()
    };
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![assigned],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::from([("busy-task".to_string(), prior_progress.clone())]),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();

    // Hold the agent lock so the resume path sees the worker as busy.
    let _busy_guard = worker_agent.lock().await;

    handle_comm_task_control(
        104,
        requester.to_string(),
        "resume".to_string(),
        "busy-task".to_string(),
        None,
        None,
        &client_tx,
        &sessions,
        &soft_interrupt_queues,
        &client_connections,
        &swarm_members,
        &swarms_by_id,
        &swarm_plans,
        &swarm_coordinators,
        &event_history,
        &event_counter,
        &swarm_event_tx,
        &mutation_runtime,
    )
    .await;

    match client_rx.recv().await.expect("response") {
        ServerEvent::Error { id, message, .. } => {
            assert_eq!(id, 104);
            assert!(message.contains("currently busy"), "{message}");
        }
        other => panic!("expected busy Error, got {other:?}"),
    }

    let plans = swarm_plans.read().await;
    let plan = plans.get(swarm_id).expect("plan exists");
    let item = plan
        .items
        .iter()
        .find(|item| item.id == "busy-task")
        .expect("task exists");
    assert_eq!(
        item.status, "running",
        "rejected resume must not flip a live task back to queued"
    );
    assert_eq!(plan.version, 1, "rejected resume must not bump the plan");
    assert_eq!(
        plan.task_progress.get("busy-task"),
        Some(&prior_progress),
        "rejected resume must not touch the task's progress record"
    );
}

/// Regression: requeueing an existing assignment (resume of a running/stale
/// task) must preserve the prior run's history instead of replacing the
/// progress record. Wiping started_at/heartbeats/checkpoints blinded
/// staleness monitors and salvage flows to everything the previous run did.
#[tokio::test]
async fn requeue_existing_assignment_preserves_prior_progress_history() {
    let (_env, runtime_dir) = RuntimeEnvGuard::new();
    let _isolated_home = DetachedTaskEnvGuard(vec![("JCODE_HOME", std::env::var_os("JCODE_HOME"))]);
    crate::env::set_var("JCODE_HOME", runtime_dir.path().join("home"));
    let swarm_id = "swarm-requeue-preserve";
    let requester = "coord";
    let worker = "worker";
    let mut assigned = plan_item("requeue-me", "running_stale", "high", &[]);
    assigned.assigned_to = Some(worker.to_string());
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![assigned],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::from([(
                "requeue-me".to_string(),
                crate::server::SwarmTaskProgress {
                    assigned_session_id: Some(worker.to_string()),
                    assignment_summary: Some("original assignment".to_string()),
                    assigned_at_unix_ms: Some(1_000),
                    started_at_unix_ms: Some(2_000),
                    last_heartbeat_unix_ms: Some(3_000),
                    last_detail: Some("was working".to_string()),
                    last_checkpoint_unix_ms: Some(3_500),
                    checkpoint_summary: Some("halfway".to_string()),
                    heartbeat_count: Some(7),
                    checkpoint_count: Some(2),
                    stale_since_unix_ms: Some(4_000),
                    completed_at_unix_ms: None,
                    no_artifact_requeues: None,
                    dead_assignee_reclaims: None,
                },
            )]),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));

    let selected = super::task_snapshot_for(swarm_id, "requeue-me", &swarm_plans)
        .await
        .expect("selected task");
    let swarm_state = super::SwarmState {
        members: Arc::new(RwLock::new(HashMap::new())),
        swarms_by_id: Arc::new(RwLock::new(HashMap::new())),
        plans: Arc::clone(&swarm_plans),
        coordinators: Arc::new(RwLock::new(HashMap::new())),
    };
    let result = super::requeue_existing_assignment(
        swarm_id,
        requester,
        worker,
        "requeue-me",
        "resume this work".to_string(),
        &selected,
        &swarm_state,
    )
    .await;
    assert!(result.is_ok(), "requeue should succeed");

    let plans = swarm_plans.read().await;
    let plan = plans.get(swarm_id).expect("plan exists");
    let item = plan
        .items
        .iter()
        .find(|item| item.id == "requeue-me")
        .expect("task exists");
    assert_eq!(item.status, "queued");
    let progress = plan
        .task_progress
        .get("requeue-me")
        .expect("progress exists");
    assert_eq!(
        progress.started_at_unix_ms,
        Some(2_000),
        "prior run's start time must survive the requeue"
    );
    assert_eq!(progress.last_heartbeat_unix_ms, Some(3_000));
    assert_eq!(progress.heartbeat_count, Some(7));
    assert_eq!(progress.checkpoint_count, Some(2));
    assert_eq!(progress.checkpoint_summary.as_deref(), Some("halfway"));
    assert_eq!(progress.last_detail.as_deref(), Some("was working"));
    assert_eq!(
        progress.stale_since_unix_ms, None,
        "requeued task is no longer stale"
    );
    assert_eq!(
        progress.assignment_summary.as_deref(),
        Some("resume this work"),
        "assignment-scoped fields refresh for the new attempt"
    );
    assert!(
        progress.assigned_at_unix_ms.unwrap_or(0) > 1_000,
        "assigned_at refreshes for the new attempt"
    );
}

/// Regression: an identical coordinator retry issued shortly after a
/// previous retry succeeded must actually re-dispatch the task. The durable
/// mutation layer used to replay the persisted assign_task success for any
/// identical request within the final-state TTL (30s), so a worker that
/// failed quickly made the second retry a silent no-op: the coordinator got
/// a success response while the task stayed failed.
#[tokio::test]
async fn task_control_retry_re_dispatches_after_recent_identical_retry() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-retry-replay";
    let requester = "coord";
    let worker = "worker";
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let worker_agent = test_agent().await;
    let sessions = Arc::new(RwLock::new(HashMap::from([(
        worker.to_string(),
        worker_agent,
    )])));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::new()));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "ready")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let mut assigned = plan_item("flaky-task", "failed", "high", &[]);
    assigned.assigned_to = Some(worker.to_string());
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![assigned],
            version: 1,
            participants: HashSet::from([requester.to_string(), worker.to_string()]),
            task_progress: HashMap::new(),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.to_string(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();

    let run_retry = |id: u64| {
        let requester = requester.to_string();
        let client_tx = client_tx.clone();
        let sessions = Arc::clone(&sessions);
        let soft_interrupt_queues = Arc::clone(&soft_interrupt_queues);
        let client_connections = Arc::clone(&client_connections);
        let swarm_members = Arc::clone(&swarm_members);
        let swarms_by_id = Arc::clone(&swarms_by_id);
        let swarm_plans = Arc::clone(&swarm_plans);
        let swarm_coordinators = Arc::clone(&swarm_coordinators);
        let event_history = Arc::clone(&event_history);
        let event_counter = Arc::clone(&event_counter);
        let swarm_event_tx = swarm_event_tx.clone();
        let mutation_runtime = mutation_runtime.clone();
        async move {
            handle_comm_task_control(
                id,
                requester,
                "retry".to_string(),
                "flaky-task".to_string(),
                None,
                None,
                &client_tx,
                &sessions,
                &soft_interrupt_queues,
                &client_connections,
                &swarm_members,
                &swarms_by_id,
                &swarm_plans,
                &swarm_coordinators,
                &event_history,
                &event_counter,
                &swarm_event_tx,
                &mutation_runtime,
            )
            .await;
        }
    };

    let wait_for_status_leaving_failed = || {
        let swarm_plans = Arc::clone(&swarm_plans);
        async move {
            for _ in 0..200 {
                {
                    let plans = swarm_plans.read().await;
                    let status = plans
                        .get(swarm_id)
                        .and_then(|plan| plan.items.iter().find(|item| item.id == "flaky-task"))
                        .map(|item| item.status.clone())
                        .unwrap_or_default();
                    if status != "failed" {
                        return status;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            "failed".to_string()
        }
    };

    // First retry dispatches the task.
    run_retry(201).await;
    match client_rx.recv().await.expect("first retry response") {
        ServerEvent::CommAssignTaskResponse { id, task_id, .. } => {
            assert_eq!(id, 201);
            assert_eq!(task_id, "flaky-task");
        }
        other => panic!("expected CommAssignTaskResponse, got {other:?}"),
    }
    let status = wait_for_status_leaving_failed().await;
    assert_ne!(status, "failed", "first retry should dispatch the task");

    // Simulate the worker failing quickly, well within the final-state TTL.
    {
        let mut plans = swarm_plans.write().await;
        let plan = plans.get_mut(swarm_id).expect("plan exists");
        let item = plan
            .items
            .iter_mut()
            .find(|item| item.id == "flaky-task")
            .expect("task exists");
        item.status = "failed".to_string();
    }

    // Identical second retry must re-dispatch instead of replaying the
    // persisted success from the first retry.
    run_retry(202).await;
    match client_rx.recv().await.expect("second retry response") {
        ServerEvent::CommAssignTaskResponse { id, task_id, .. } => {
            assert_eq!(id, 202);
            assert_eq!(task_id, "flaky-task");
        }
        other => panic!("expected CommAssignTaskResponse, got {other:?}"),
    }
    let status = wait_for_status_leaving_failed().await;
    assert_ne!(
        status, "failed",
        "second identical retry must actually re-dispatch the task"
    );
}
