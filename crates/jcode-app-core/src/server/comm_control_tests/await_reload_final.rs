#[tokio::test]
async fn await_members_returns_persisted_final_response_after_reload_retry() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-d";
    let requester = "req";
    let key = crate::server::await_members_state::request_key(
        requester,
        swarm_id,
        &[],
        &["completed".to_string()],
        None,
    );
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    crate::server::await_members_state::save_state(
        &crate::server::await_members_state::PersistedAwaitMembersState {
            key,
            session_id: requester.to_string(),
            swarm_id: swarm_id.to_string(),
            target_status: vec!["completed".to_string()],
            requested_ids: vec![],
            mode: None,
            created_at_unix_ms: now_ms,
            deadline_unix_ms: now_ms + 60_000,
            background: false,
            notify: false,
            wake: false,
            final_response: Some(
                crate::server::await_members_state::PersistedAwaitMembersResult {
                    completed: true,
                    members: vec![crate::protocol::AwaitedMemberStatus {
                        session_id: "peer-1".to_string(),
                        friendly_name: Some("peer-1".to_string()),
                        status: "completed".to_string(),
                        done: true,
                        completion_report: None,
                    }],
                    summary: "All 1 members are done: peer-1".to_string(),
                    resolved_at_unix_ms: now_ms,
                },
            ),
        },
    );

    let await_runtime = AwaitMembersRuntime::default();
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let swarm_members = Arc::new(RwLock::new(HashMap::from([(
        requester.to_string(),
        member(requester, swarm_id, "ready"),
    )])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string()]),
    )])));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);

    handle_comm_await_members(
        1,
        requester.to_string(),
        vec!["completed".to_string()],
        vec![],
        None,
        Some(60),
        false,
        false,
        false,
        CommAwaitMembersContext {
            client_event_tx: &client_tx,
            swarm_members: &swarm_members,
            swarms_by_id: &swarms_by_id,
            swarm_event_tx: &swarm_event_tx,
            await_members_runtime: &await_runtime,
        },
    )
    .await;

    match client_rx.recv().await.expect("response should arrive") {
        ServerEvent::CommAwaitMembersResponse {
            completed,
            summary,
            members,
            ..
        } => {
            assert!(completed);
            assert_eq!(summary, "All 1 members are done: peer-1");
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].session_id, "peer-1");
        }
        other => panic!("expected CommAwaitMembersResponse, got {other:?}"),
    }
}

#[tokio::test]
async fn await_members_uses_current_completion_report_after_worker_finishes_again() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-completed-again";
    let requester = "req";
    let peer = "peer-1";
    let target_status = vec!["completed".to_string()];
    let requested_ids = vec![peer.to_string()];
    let key = crate::server::await_members_state::request_key(
        requester,
        swarm_id,
        &requested_ids,
        &target_status,
        None,
    );
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    crate::server::await_members_state::save_state(
        &crate::server::await_members_state::PersistedAwaitMembersState {
            key: key.clone(),
            session_id: requester.to_string(),
            swarm_id: swarm_id.to_string(),
            target_status: target_status.clone(),
            requested_ids: requested_ids.clone(),
            mode: None,
            created_at_unix_ms: now_ms,
            deadline_unix_ms: now_ms + 60_000,
            background: false,
            notify: false,
            wake: false,
            final_response: Some(
                crate::server::await_members_state::PersistedAwaitMembersResult {
                    completed: true,
                    members: vec![crate::protocol::AwaitedMemberStatus {
                        session_id: peer.to_string(),
                        friendly_name: Some(peer.to_string()),
                        status: "completed".to_string(),
                        done: true,
                        completion_report: Some("SOURCE verdict".to_string()),
                    }],
                    summary: "All 1 members are done: peer-1".to_string(),
                    resolved_at_unix_ms: now_ms,
                },
            ),
        },
    );

    let await_runtime = AwaitMembersRuntime::default();
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let mut current_peer = member(peer, swarm_id, "completed");
    current_peer.latest_completion_report = Some("DOC verdict".to_string());
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), member(requester, swarm_id, "ready")),
        (peer.to_string(), current_peer),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), peer.to_string()]),
    )])));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);

    let persisted = crate::server::await_members_state::load_state(&key)
        .expect("the previous completed response must be persisted");
    assert_eq!(persisted.key, key);
    assert_eq!(persisted.session_id, requester);
    assert_eq!(persisted.swarm_id, swarm_id);
    assert_eq!(persisted.target_status, target_status);
    assert_eq!(persisted.requested_ids, requested_ids);
    assert_eq!(persisted.mode, None);
    assert!(!persisted.background && !persisted.notify && !persisted.wake);
    let cached = persisted.final_response.expect("a final response must exist");
    assert!(cached.completed);
    assert_eq!(cached.members.len(), 1);
    assert_eq!(cached.members[0].session_id, peer);
    assert_eq!(cached.members[0].status, "completed");
    assert!(cached.members[0].done);
    assert_eq!(
        cached.members[0].completion_report.as_deref(),
        Some("SOURCE verdict")
    );

    let current = crate::server::comm_await::awaited_member_statuses(
        requester,
        swarm_id,
        &requested_ids,
        &target_status,
        &swarm_members,
        &swarms_by_id,
    )
    .await;
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].session_id, peer);
    assert_eq!(current[0].status, "completed");
    assert!(current[0].done);
    assert_eq!(current[0].completion_report.as_deref(), Some("DOC verdict"));
    assert!(crate::server::comm_await::mode_satisfied(&current, None));

    handle_comm_await_members(
        3,
        requester.to_string(),
        target_status,
        requested_ids,
        None,
        Some(60),
        false,
        false,
        false,
        CommAwaitMembersContext {
            client_event_tx: &client_tx,
            swarm_members: &swarm_members,
            swarms_by_id: &swarms_by_id,
            swarm_event_tx: &swarm_event_tx,
            await_members_runtime: &await_runtime,
        },
    )
    .await;

    match tokio::time::timeout(Duration::from_secs(1), client_rx.recv())
        .await
        .expect("await response should arrive within one second")
        .expect("response should be sent")
    {
        ServerEvent::CommAwaitMembersResponse {
            id,
            completed,
            members,
            background_started,
            ..
        } => {
            assert_eq!(id, 3);
            assert!(completed);
            assert!(!background_started);
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].session_id, peer);
            assert_eq!(members[0].status, "completed");
            assert!(members[0].done);
            assert_eq!(members[0].completion_report.as_deref(), Some("DOC verdict"));
        }
        other => panic!("expected CommAwaitMembersResponse, got {other:?}"),
    }
}

#[tokio::test]
async fn await_members_cached_final_live_snapshot_compatibility() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    for (mode, statuses, cached_completed, background, live_summary) in [
        (
            "any",
            &["completed", "queued"][..],
            true,
            false,
            Some("Matched 1 member: peer-1"),
        ),
        (
            "any",
            &["completed", "unknown"][..],
            true,
            false,
            Some("Matched 2 members: peer-1, peer-2"),
        ),
        ("any", &["queued", "unknown"][..], true, false, None),
        ("all", &["completed", "unknown"][..], true, false, None),
        ("all", &["unknown"][..], true, false, None),
        ("any", &["unknown"][..], true, false, None),
        ("all", &["unknown"][..], false, false, None),
        ("any", &["unknown"][..], false, false, None),
        ("all", &[][..], false, false, None),
        ("any", &[][..], false, false, None),
        (
            "all",
            &["completed"][..],
            false,
            false,
            Some("All 1 members are done: peer-1"),
        ),
        (
            "any",
            &["completed", "queued"][..],
            false,
            false,
            Some("Matched 1 member: peer-1"),
        ),
        (
            "all",
            &["completed"][..],
            true,
            true,
            Some("All 1 members are done: peer-1"),
        ),
    ] {
        let case = format!("{mode} {statuses:?} cached={cached_completed} background={background}");
        let swarm_id = "swarm-cache-compatibility";
        let requester = "req";
        let target_status = vec!["completed".to_string()];
        let requested_ids: Vec<String> = (0..statuses.len())
            .map(|index| format!("peer-{}", index + 1))
            .collect();
        let key = crate::server::await_members_state::request_key(
            requester,
            swarm_id,
            &requested_ids,
            &target_status,
            Some(mode),
        );
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let cached_summary = if cached_completed {
            "All 1 members are done: cached SOURCE"
        } else {
            "Timed out. Still waiting on: cached SOURCE (queued)"
        };
        crate::server::await_members_state::save_state(
            &crate::server::await_members_state::PersistedAwaitMembersState {
                key: key.clone(),
                session_id: requester.to_string(),
                swarm_id: swarm_id.to_string(),
                target_status: target_status.clone(),
                requested_ids: requested_ids.clone(),
                mode: Some(mode.to_string()),
                created_at_unix_ms: now_ms,
                deadline_unix_ms: now_ms + 60_000,
                background,
                notify: false,
                wake: false,
                final_response: Some(
                    crate::server::await_members_state::PersistedAwaitMembersResult {
                        completed: cached_completed,
                        members: vec![crate::protocol::AwaitedMemberStatus {
                            session_id: "peer-1".to_string(),
                            friendly_name: Some("peer-1".to_string()),
                            status: if cached_completed {
                                "completed"
                            } else {
                                "queued"
                            }
                            .to_string(),
                            done: cached_completed,
                            completion_report: Some("SOURCE verdict".to_string()),
                        }],
                        summary: cached_summary.to_string(),
                        resolved_at_unix_ms: now_ms,
                    },
                ),
            },
        );

        let mut live_members =
            HashMap::from([(requester.to_string(), member(requester, swarm_id, "ready"))]);
        for (peer, status) in requested_ids.iter().zip(statuses) {
            if *status != "unknown" {
                let mut current_peer = member(peer, swarm_id, status);
                if *status == "completed" {
                    current_peer.latest_completion_report = Some("DOC verdict".to_string());
                }
                live_members.insert(peer.clone(), current_peer);
            }
        }
        let swarm_ids = live_members.keys().cloned().collect::<HashSet<_>>();
        let swarm_members = Arc::new(RwLock::new(live_members));
        let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(),
            swarm_ids,
        )])));
        let await_runtime = AwaitMembersRuntime::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);
        handle_comm_await_members(
            4,
            requester.to_string(),
            target_status,
            requested_ids.clone(),
            Some(mode.to_string()),
            Some(60),
            background,
            false,
            false,
            CommAwaitMembersContext {
                client_event_tx: &client_tx,
                swarm_members: &swarm_members,
                swarms_by_id: &swarms_by_id,
                swarm_event_tx: &swarm_event_tx,
                await_members_runtime: &await_runtime,
            },
        )
        .await;

        match client_rx.try_recv().expect("response must be inline") {
            ServerEvent::CommAwaitMembersResponse {
                id,
                completed,
                members,
                summary,
                background_started,
            } => {
                assert_eq!(id, 4, "{case}");
                assert!(!background_started, "{case}");
                assert_eq!(
                    completed,
                    live_summary.is_some() || cached_completed,
                    "{case}"
                );
                assert_eq!(summary, live_summary.unwrap_or(cached_summary), "{case}");
                if live_summary.is_some() {
                    assert_eq!(members.len(), statuses.len(), "{case}");
                    for (index, current) in members.iter().enumerate() {
                        assert_eq!(current.session_id, requested_ids[index], "{case}");
                        assert_eq!(current.status, statuses[index], "{case}");
                        assert_eq!(current.done, statuses[index] != "queued", "{case}");
                        assert_eq!(
                            current.completion_report.as_deref(),
                            (statuses[index] == "completed").then_some("DOC verdict"),
                            "{case}"
                        );
                    }
                } else {
                    assert_eq!(members.len(), 1, "{case}");
                    assert_eq!(members[0].session_id, "peer-1", "{case}");
                    assert_eq!(members[0].done, cached_completed, "{case}");
                    assert_eq!(
                        members[0].status,
                        if cached_completed { "completed" } else { "queued" },
                        "{case}"
                    );
                    assert_eq!(
                        members[0].completion_report.as_deref(),
                        Some("SOURCE verdict"),
                        "{case}"
                    );
                }
            }
            other => panic!("expected CommAwaitMembersResponse for {case}, got {other:?}"),
        }
        // Fresh inline replies must not refresh the durable cached result.
        let cached = crate::server::await_members_state::load_state(&key)
            .expect("cached state must remain")
            .final_response
            .expect("cached final must remain");
        assert_eq!(cached.completed, cached_completed, "{case}");
        assert_eq!(cached.summary, cached_summary, "{case}");
        assert_eq!(cached.resolved_at_unix_ms, now_ms, "{case}");
        assert_eq!(cached.members.len(), 1, "{case}");
        assert_eq!(
            cached.members[0].completion_report.as_deref(),
            Some("SOURCE verdict"),
            "{case}"
        );
    }
}

#[tokio::test]
async fn await_members_ignores_persisted_final_when_requested_member_is_queued_again() {
    let (_env, _runtime_dir) = RuntimeEnvGuard::new();
    let swarm_id = "swarm-requeue";
    let requester = "req";
    let peer = "peer-1";
    let target_status = vec!["completed".to_string()];
    let requested_ids = vec![peer.to_string()];
    let key = crate::server::await_members_state::request_key(
        requester,
        swarm_id,
        &requested_ids,
        &target_status,
        None,
    );
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    crate::server::await_members_state::save_state(
        &crate::server::await_members_state::PersistedAwaitMembersState {
            key,
            session_id: requester.to_string(),
            swarm_id: swarm_id.to_string(),
            target_status: target_status.clone(),
            requested_ids: requested_ids.clone(),
            mode: None,
            created_at_unix_ms: now_ms,
            deadline_unix_ms: now_ms + 60_000,
            background: false,
            notify: false,
            wake: false,
            final_response: Some(
                crate::server::await_members_state::PersistedAwaitMembersResult {
                    completed: true,
                    members: vec![crate::protocol::AwaitedMemberStatus {
                        session_id: peer.to_string(),
                        friendly_name: Some(peer.to_string()),
                        status: "completed".to_string(),
                        done: true,
                        completion_report: None,
                    }],
                    summary: "All 1 members are done: peer-1".to_string(),
                    resolved_at_unix_ms: now_ms,
                },
            ),
        },
    );

    let await_runtime = AwaitMembersRuntime::default();
    let (client_tx, mut client_rx) = mpsc::unbounded_channel();
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (requester.to_string(), member(requester, swarm_id, "ready")),
        (peer.to_string(), member(peer, swarm_id, "queued")),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        swarm_id.to_string(),
        HashSet::from([requester.to_string(), peer.to_string()]),
    )])));
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(32);

    handle_comm_await_members(
        2,
        requester.to_string(),
        target_status,
        requested_ids,
        None,
        Some(5),
        false,
        false,
        false,
        CommAwaitMembersContext {
            client_event_tx: &client_tx,
            swarm_members: &swarm_members,
            swarms_by_id: &swarms_by_id,
            swarm_event_tx: &swarm_event_tx,
            await_members_runtime: &await_runtime,
        },
    )
    .await;

    assert!(
        tokio::time::timeout(Duration::from_millis(50), client_rx.recv())
            .await
            .is_err(),
        "stale persisted final response should not be replayed while the worker is queued again"
    );

    {
        let mut members = swarm_members.write().await;
        members.get_mut(peer).expect("peer exists").status = "completed".to_string();
    }
    let _ = swarm_event_tx.send(swarm_event(
        peer,
        swarm_id,
        SwarmEventType::StatusChange {
            old_status: "queued".to_string(),
            new_status: "completed".to_string(),
        },
    ));

    match tokio::time::timeout(Duration::from_secs(1), client_rx.recv())
        .await
        .expect("await response should arrive after completion")
        .expect("response should be sent")
    {
        ServerEvent::CommAwaitMembersResponse {
            completed, members, ..
        } => {
            assert!(completed);
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].session_id, peer);
        }
        other => panic!("expected CommAwaitMembersResponse, got {other:?}"),
    }
}
