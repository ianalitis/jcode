#[tokio::test]
async fn assign_task_without_task_id_picks_highest_priority_runnable_task() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-assign";
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
        (worker.to_string(), member(worker, swarm_id, "ready")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![
                plan_item("done", "completed", "high", &[]),
                plan_item("blocked", "queued", "high", &["high-ready"]),
                plan_item("low-ready", "queued", "low", &["done"]),
                plan_item("high-ready", "queued", "high", &["done"]),
            ],
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

    handle_comm_assign_task(
        77,
        requester.to_string(),
        Some(worker.to_string()),
        None,
        Some("Pick the next task".to_string()),
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

    let response = client_rx.recv().await.expect("response");
    match response {
        ServerEvent::CommAssignTaskResponse {
            id,
            task_id,
            target_session,
        } => {
            assert_eq!(id, 77);
            assert_eq!(task_id, "high-ready");
            assert_eq!(target_session, worker);
        }
        other => panic!("expected CommAssignTaskResponse, got {other:?}"),
    }

    let plans = swarm_plans.read().await;
    let plan = plans.get(swarm_id).expect("plan exists");
    let selected = plan
        .items
        .iter()
        .find(|item| item.id == "high-ready")
        .expect("selected task exists");
    assert_eq!(selected.assigned_to.as_deref(), Some(worker));
    assert_eq!(selected.status, "queued");

    let blocked = plan
        .items
        .iter()
        .find(|item| item.id == "blocked")
        .expect("blocked task exists");
    assert!(
        blocked.assigned_to.is_none(),
        "blocked task should not be auto-assigned"
    );

    let members = swarm_members.read().await;
    let worker_member = members.get(worker).expect("worker member exists");
    assert_eq!(
        worker_member.status, "queued",
        "assigned worker should stop looking completed/ready before async execution starts"
    );
}

#[tokio::test]
async fn assign_task_marks_completed_worker_queued_before_returning() {
    let (_env, _runtime) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-assign-completed-worker";
    let requester = "coord";
    let worker = "worker-completed";
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
        (worker.to_string(), member(worker, swarm_id, "completed")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), worker.to_string()]),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![plan_item("next", "queued", "high", &[])],
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

    handle_comm_assign_task(
        78,
        requester.to_string(),
        Some(worker.to_string()),
        Some("next".to_string()),
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
        ServerEvent::CommAssignTaskResponse {
            id,
            task_id,
            target_session,
        } => {
            assert_eq!(id, 78);
            assert_eq!(task_id, "next");
            assert_eq!(target_session, worker);
        }
        other => panic!("expected CommAssignTaskResponse, got {other:?}"),
    }

    let members = swarm_members.read().await;
    let worker_member = members.get(worker).expect("worker member exists");
    assert_eq!(worker_member.status, "queued");
    assert!(
        worker_member
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("task next")),
        "queued member should include the assigned task in its detail"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn assign_task_pending_save_failure_preserves_plan_member_and_interrupt() {
    let (_env, runtime_dir) = RuntimeEnvGuard::new();
    // The handler's ordinary persistence must stay isolated even in the RED run.
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            if let Some(home) = self.0.take() {
                crate::env::set_var("JCODE_HOME", home);
            } else {
                crate::env::remove_var("JCODE_HOME");
            }
        }
    }
    let _home = RestoreHome(std::env::var_os("JCODE_HOME"));
    crate::env::set_var("JCODE_HOME", runtime_dir.path().join("home"));
    let swarm_id = "swarm-pending-save-failure";
    let requester = "r".repeat(233);
    let worker = "pending-save-worker";
    let key = crate::server::swarm_mutation_state::request_key(
        &requester,
        "assign_task",
        &[
            swarm_id.to_string(),
            worker.to_string(),
            "next".to_string(),
            String::new(),
        ],
    );
    let path = crate::server::durable_state::state_path("jcode-swarm-mutations", &key);
    assert!(path.starts_with(runtime_dir.path()));
    assert_eq!(key.len(), 250);
    assert_eq!(path.file_name().unwrap().len(), 255);
    assert!(path.with_extension("tmp.1.0").file_name().unwrap().len() > 255);
    let probe = runtime_dir
        .path()
        .join("probe")
        .join(path.file_name().unwrap());
    std::fs::create_dir_all(probe.parent().unwrap()).unwrap();
    std::fs::write(&probe, b"primary is writable").unwrap();
    assert_eq!(std::fs::read(&probe).unwrap(), b"primary is writable");
    let error = crate::storage::write_json_fast(&probe, &serde_json::json!({"pending": true}))
        .expect_err("fixture checked writer must fail before primary replacement");
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::ENAMETOOLONG),
        "fixture must fail at temporary file creation, not during setup"
    );
    assert_eq!(std::fs::read(&probe).unwrap(), b"primary is writable");
    assert!(!probe.with_extension("bak").exists());
    assert!(!path.exists());
    assert!(!path.with_extension("bak").exists());

    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    // No Agent or provider is constructed. Register the queue directly so an
    // accidentally admitted assignment has an observable, local-only effect.
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let interrupt_queue = Arc::new(std::sync::Mutex::new(Vec::new()));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::from([(
        worker.to_string(),
        interrupt_queue.clone(),
    )])));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let worker_before = member(worker, swarm_id, "completed");
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.clone(), {
            let mut member = member(&requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), worker_before.clone()),
    ])));
    let participants = HashSet::from([requester.clone(), worker.to_string()]);
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        participants.clone(),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![plan_item("next", "queued", "high", &[])],
            version: 1,
            participants: participants.clone(),
            task_progress: HashMap::new(),
            mode: "light".to_string(),
            node_meta: HashMap::new(),
        },
    )])));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        requester.clone(),
    )])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, mut swarm_event_rx) = broadcast::channel(32);
    let mutation_runtime = SwarmMutationRuntime::default();

    for request_id in [179, 180] {
        handle_comm_assign_task(
            request_id,
            requester.clone(),
            Some(worker.to_string()),
            Some("next".to_string()),
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

        match client_rx
            .try_recv()
            .expect("pending-save admission response")
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
            client_rx.try_recv().is_err(),
            "must not emit assignment success"
        );
        let plans = swarm_plans.read().await;
        let plan = &plans[swarm_id];
        assert_eq!(plan.version, 1);
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].id, "next");
        assert_eq!(plan.items[0].status, "queued");
        assert!(plan.items[0].assigned_to.is_none());
        assert!(plan.task_progress.is_empty());
        assert_eq!(plan.participants, participants);
        assert_eq!(plan.mode, "light");
        assert!(plan.node_meta.is_empty());
        drop(plans);
        let members = swarm_members.read().await;
        let worker_after = &members[worker];
        assert_eq!(worker_after.status, worker_before.status);
        assert_eq!(worker_after.detail, worker_before.detail);
        assert_eq!(worker_after.task_label, worker_before.task_label);
        assert_eq!(
            worker_after.last_status_change,
            worker_before.last_status_change
        );
        drop(members);
        assert!(interrupt_queue.lock().unwrap().is_empty());
        assert!(event_history.read().await.is_empty());
        assert_eq!(event_counter.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert!(swarm_event_rx.try_recv().is_err());
        assert!(!path.exists(), "must not synthesize a final state");
        assert!(!path.with_extension("bak").exists());
    }
}

#[tokio::test]
async fn completion_save_failure_consumed_assignment_blocks_identical_repeat_orphan_pending_admission()
 {
    let (_env, runtime_dir) = RuntimeEnvGuard::new();
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            if let Some(home) = self.0.take() {
                crate::env::set_var("JCODE_HOME", home);
            } else {
                crate::env::remove_var("JCODE_HOME");
            }
        }
    }
    let _home = RestoreHome(std::env::var_os("JCODE_HOME"));
    crate::env::set_var("JCODE_HOME", runtime_dir.path().join("home"));
    let swarm_id = "swarm-final-save";
    let requester = "coord";
    let worker = "final-save-worker";
    let key = crate::server::swarm_mutation_state::request_key(
        requester,
        "assign_task",
        &[
            swarm_id.to_string(),
            worker.to_string(),
            "next".to_string(),
            String::new(),
        ],
    );
    let path = crate::server::durable_state::state_path("jcode-swarm-mutations", &key);
    assert!(path.starts_with(runtime_dir.path()));
    assert!(key.len() < 100);
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    // The registered queue exposes the real dispatch effect without an Agent
    // or provider, exactly as in the pending-save regression above.
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let interrupt_queue = Arc::new(std::sync::Mutex::new(Vec::new()));
    let soft_interrupt_queues = Arc::new(RwLock::new(HashMap::from([(
        worker.to_string(),
        interrupt_queue.clone(),
    )])));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), {
            let mut member = member(requester, swarm_id, "ready");
            member.role = "coordinator".to_string();
            member
        }),
        (worker.to_string(), member(worker, swarm_id, "completed")),
    ])));
    let participants = HashSet::from([requester.to_string(), worker.to_string()]);
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        participants.clone(),
    )])));
    let swarm_plans = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        VersionedPlan {
            items: vec![plan_item("next", "queued", "high", &[])],
            version: 1,
            participants,
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

    // The first sessions.read() follows pending admission, plan mutation, and
    // member updates, but precedes queue dispatch and final persistence.
    let sessions_guard = sessions.write().await;
    let mut assignment = std::pin::pin!(handle_comm_assign_task(
        301,
        requester.to_string(),
        Some(worker.to_string()),
        Some("next".to_string()),
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
    ));
    let polled = std::future::poll_fn(|cx| {
        std::task::Poll::Ready(std::future::Future::poll(assignment.as_mut(), cx))
    })
    .await;
    assert!(
        polled.is_pending(),
        "sessions guard must stop actual dispatch"
    );
    let pending_bytes = std::fs::read(&path).expect("admission must persist before the seam");
    let pending: crate::server::swarm_mutation_state::PersistedSwarmMutationState =
        serde_json::from_slice(&pending_bytes).unwrap();
    assert_eq!(pending.key, key);
    assert_eq!(pending.action, "assign_task");
    assert_eq!(pending.session_id, requester);
    assert!(pending.final_response.is_none());
    {
        let plans = swarm_plans.read().await;
        let plan = &plans[swarm_id];
        assert_eq!(
            plan.version, 2,
            "actual plan effect must precede obstruction"
        );
        assert_eq!(plan.items[0].assigned_to.as_deref(), Some(worker));
        assert_eq!(
            plan.task_progress["next"].assigned_session_id.as_deref(),
            Some(worker)
        );
    }
    assert_eq!(swarm_members.read().await[worker].status, "queued");
    assert!(interrupt_queue.lock().unwrap().is_empty());
    assert!(client_rx.try_recv().is_err());

    let mutation_dir = path.parent().unwrap();
    let retained_dir = runtime_dir.path().join("retained-mutations");
    let retained_path = retained_dir.join(path.file_name().unwrap());
    std::fs::rename(mutation_dir, &retained_dir).unwrap();
    std::fs::write(mutation_dir, b"block final state directory").unwrap();
    assert!(mutation_dir.is_file());
    assert_eq!(std::fs::read(&retained_path).unwrap(), pending_bytes);
    drop(sessions_guard);
    assignment.await;
    assert_eq!(std::fs::read(&retained_path).unwrap(), pending_bytes);
    // Repair before any repeated request, so strict admission read errors
    // cannot be confused with the required same-runtime terminal quarantine.
    std::fs::remove_file(mutation_dir).unwrap();
    std::fs::rename(&retained_dir, mutation_dir).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), pending_bytes);
    let interrupt_content = {
        let queue = interrupt_queue.lock().unwrap();
        assert_eq!(
            queue.len(),
            1,
            "real dispatch must occur despite final-save failure"
        );
        assert!(
            queue[0]
                .content
                .contains("Task assigned to you by coordinator: task next")
        );
        assert!(!queue[0].urgent);
        queue[0].content.clone()
    };
    let worker_after = swarm_members.read().await[worker].clone();
    let history_len = event_history.read().await.len();
    let event_count = event_counter.load(std::sync::atomic::Ordering::Relaxed);
    assert!(history_len > 0, "assignment must have recorded real events");

    let fresh_runtime = SwarmMutationRuntime::default();
    for request_id in [301, 302, 303] {
        if request_id != 301 {
            handle_comm_assign_task(
                request_id,
                requester.to_string(),
                Some(worker.to_string()),
                Some("next".to_string()),
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
                if request_id == 303 {
                    &fresh_runtime
                } else {
                    &mutation_runtime
                },
            )
            .await;
        }
        match client_rx
            .try_recv()
            .expect("completion or pending uncertainty response")
        {
            ServerEvent::Error {
                id,
                message,
                retry_after_secs,
            } => {
                assert_eq!(id, request_id);
                let expected_message = if request_id == 303 {
                    "Swarm mutation pending execution is uncertain; effects may have occurred; no matching execution is active in this runtime; refusing automatic execution; restarting does not establish safety."
                } else {
                    "Swarm mutation completion persistence failed; effects may have occurred; this key is blocked in this runtime; inspect/reconcile rather than retry; restarting does not establish safety."
                };
                assert_eq!(message, expected_message);
                assert_eq!(retry_after_secs, None);
            }
            other => panic!("expected completion or pending uncertainty, got {other:?}"),
        }
        assert!(
            client_rx.try_recv().is_err(),
            "must not also emit assignment success"
        );
        let plans = swarm_plans.read().await;
        let plan = &plans[swarm_id];
        assert_eq!(plan.version, 2, "repeat must not mutate the plan again");
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].assigned_to.as_deref(), Some(worker));
        assert_eq!(plan.items[0].status, "queued");
        assert_eq!(plan.task_progress.len(), 1);
        assert_eq!(
            plan.task_progress["next"].assigned_session_id.as_deref(),
            Some(worker)
        );
        drop(plans);
        let members = swarm_members.read().await;
        let member = &members[worker];
        assert_eq!(member.status, worker_after.status);
        assert_eq!(member.detail, worker_after.detail);
        assert_eq!(member.task_label, worker_after.task_label);
        assert_eq!(member.last_status_change, worker_after.last_status_change);
        drop(members);
        {
            let queue = interrupt_queue.lock().unwrap();
            assert_eq!(queue.len(), 1, "repeat must not enqueue a second interrupt");
            assert_eq!(queue[0].content, interrupt_content);
        }
        assert_eq!(event_history.read().await.len(), history_len);
        assert_eq!(
            event_counter.load(std::sync::atomic::Ordering::Relaxed),
            event_count
        );
        assert_eq!(std::fs::read(&path).unwrap(), pending_bytes);
    }
}
