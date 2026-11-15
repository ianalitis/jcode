use super::*;

pub(super) struct IsolatedConcurrencyEnv {
    _home: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl IsolatedConcurrencyEnv {
    pub(super) fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let previous = ["JCODE_HOME", "JCODE_NO_TELEMETRY", "JCODE_RUNTIME_DIR"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        crate::env::set_var("JCODE_HOME", home.path());
        crate::env::set_var("JCODE_NO_TELEMETRY", "1");
        crate::env::set_var("JCODE_RUNTIME_DIR", home.path().join("runtime"));
        Self {
            _home: home,
            previous,
        }
    }
}

impl Drop for IsolatedConcurrencyEnv {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            match value {
                Some(value) => crate::env::set_var(key, value),
                None => crate::env::remove_var(key),
            }
        }
    }
}

async fn restore_for_concurrency_test(
    target_id: &str,
    source_id: &str,
    source: &Arc<Mutex<Agent>>,
    provider: &Arc<dyn Provider>,
    registry: &Registry,
    sessions: &crate::server::SessionAgents,
    retain_source_peer: bool,
) -> Result<Arc<Mutex<Agent>>> {
    let mut source = Arc::clone(source);
    let mut provider = Arc::clone(provider);
    let mut registry = registry.clone();
    let mut client_selfdev = false;
    let mut client_session_id = source_id.to_owned();
    let (stream, _peer) = crate::transport::stream_pair()?;
    let (_, writer) = stream.into_split();
    let writer = Arc::new(Mutex::new(writer));
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();
    let (swarm_event_tx, _swarm_event_rx) = broadcast::channel(8);
    let now = Instant::now();
    let connections = Arc::new(RwLock::new(HashMap::from([(
        "concurrency-test-connection".to_owned(),
        ClientConnectionInfo {
            client_id: "concurrency-test-connection".to_owned(),
            session_id: client_session_id.clone(),
            client_instance_id: Some("concurrency-test-instance".into()),
            debug_client_id: None,
            connected_at: now,
            last_seen: now,
            is_processing: false,
            current_tool_name: None,
            terminal_env: Vec::new(),
            supports_user_response: false,
            disconnect_tx: mpsc::unbounded_channel().0,
        },
    )])));
    let members = Arc::new(RwLock::new(HashMap::new()));
    let swarms = Arc::new(RwLock::new(HashMap::new()));
    let source_swarm = format!("session:{source_id}");
    if retain_source_peer {
        let mut peer = connections.read().await["concurrency-test-connection"].clone();
        peer.client_id = "retained-peer".into();
        peer.client_instance_id = Some("retained-peer-instance".into());
        connections
            .write()
            .await
            .insert(peer.client_id.clone(), peer);
        let mut member = test_swarm_member(source_id, "ready");
        member.swarm_id = Some(source_swarm.clone());
        members.write().await.insert(source_id.to_owned(), member);
        swarms
            .write()
            .await
            .insert(source_swarm.clone(), HashSet::from([source_id.to_owned()]));
    }
    let result = handle_resume_session(
        1,
        target_id.to_owned(),
        None,
        Some("concurrency-test-instance"),
        retain_source_peer,
        None,
        retain_source_peer,
        &mut client_selfdev,
        &mut client_session_id,
        "concurrency-test-connection",
        &mut source,
        &mut provider,
        &mut registry,
        sessions,
        &Arc::new(RwLock::new(HashMap::new())),
        &Arc::new(RwLock::new(HashMap::new())),
        &connections,
        &Arc::new(RwLock::new(ClientDebugState::default())),
        &members,
        &swarms,
        &FileTouchService::new(),
        &Arc::new(RwLock::new(HashMap::new())),
        &Arc::new(RwLock::new(HashMap::new())),
        &Arc::new(RwLock::new(HashMap::new())),
        &Arc::new(RwLock::new(HashMap::new())),
        &Arc::new(RwLock::new(1)),
        &writer,
        "test-server",
        "test",
        &client_event_tx,
        &Arc::new(crate::mcp::SharedMcpPool::from_default_config()),
        &Arc::new(RwLock::new(VecDeque::new())),
        &Arc::new(std::sync::atomic::AtomicU64::new(0)),
        &swarm_event_tx,
        false,
        false,
    )
    .await;
    if retain_source_peer {
        assert_eq!(connections.read().await.len(), 2);
        assert_eq!(
            connections.read().await["retained-peer"].session_id,
            source_id
        );
        assert_eq!(
            members.read().await[source_id].swarm_id.as_ref(),
            Some(&source_swarm)
        );
        assert!(swarms.read().await[&source_swarm].contains(source_id));
    }
    while let Ok(event) = client_event_rx.try_recv() {
        if let ServerEvent::Error { message, .. } = event {
            assert_eq!(
                target_id, "missing-concurrency-target",
                "resume failed: {message}"
            );
        }
    }
    result
}

#[tokio::test]
async fn cold_resume_preserves_shared_source_and_persists_target_membership() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let source = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
    let source_id = source.lock().await.session_id().to_owned();
    let mut saved = crate::session::Session::create(None, Some("Shared source resume".into()));
    saved.save()?;
    let target_id = saved.id.clone();
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::from([(source_id.clone(), source.clone())]),
    ));
    let resumed = restore_for_concurrency_test(
        &target_id, &source_id, &source, &provider, &registry, &sessions, true,
    )
    .await?;
    assert!(!Arc::ptr_eq(&resumed, &source));
    assert!(Arc::ptr_eq(&sessions.read().await[&source_id], &source));
    assert!(Arc::ptr_eq(&sessions.read().await[&target_id], &resumed));
    assert_eq!(source.lock().await.session_id(), source_id);
    assert!(source.lock().await.has_concurrency_tracking());
    assert!(source.lock().await.claim_server_turn().is_ok());
    let target_swarm = format!("session:{target_id}");
    let path = crate::server::swarm_persistence::state_path(&target_swarm);
    let persisted: crate::server::swarm_persistence::PersistedSwarmState =
        serde_json::from_slice(&std::fs::read(path)?)?;
    assert_eq!(persisted.swarm_id, target_swarm);
    assert_eq!(persisted.members.len(), 1);
    assert_eq!(persisted.members[0].record.session_id, target_id);
    Ok(())
}

#[tokio::test]
async fn same_session_resume_refreshes_canonical_owner_and_preserves_attached_peer() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    for retain_peer in [false, true] {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let registry = Registry::new(provider.clone()).await;
        let source = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
        let mut returned = source.lock().await.session_for_split().clone();
        let id = returned.id.clone();
        returned.migration_epoch = 1;
        returned.system_prompt = Some("returned-current-marker".into());
        returned.save_prepared()?;
        crate::storage::write_session_lease(&crate::storage::SessionLease {
            session_id: id.clone(),
            epoch: 1,
            away_host: None,
            repo_root: None,
            last_sync_snapshot: None,
            last_sync_head: None,
            updated_at: "test".into(),
        })?;
        assert!(source.lock().await.session_copy_is_stale());
        assert!(source.lock().await.ensure_session_lease().is_err());
        let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
            HashMap::from([(id.clone(), source.clone())]),
        ));
        let restored = restore_for_concurrency_test(
            &id,
            &id,
            &source,
            &provider,
            &registry,
            &sessions,
            retain_peer,
        )
        .await?;
        // Refresh the canonical owner and all its viewers, never create two owners.
        assert!(Arc::ptr_eq(&restored, &source));
        {
            let restored_guard = restored.lock().await;
            assert!(restored_guard.ensure_session_lease().is_ok());
            assert!(restored_guard.has_concurrency_tracking());
            assert_eq!(
                restored_guard.session_for_split().system_prompt.as_deref(),
                Some("returned-current-marker")
            );
        }
        assert!(Arc::ptr_eq(&sessions.read().await[&id], &restored));
    }
    Ok(())
}

#[tokio::test]
async fn same_session_resume_preserves_busy_or_unloadable_owner() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    for busy in [true, false] {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let registry = Registry::new(provider.clone()).await;
        let source = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
        let guard = source.lock().await;
        let id = guard.session_id().to_owned();
        crate::storage::write_session_lease(&crate::storage::SessionLease {
            session_id: id.clone(),
            epoch: 1,
            away_host: None,
            repo_root: None,
            last_sync_snapshot: None,
            last_sync_head: None,
            updated_at: "test".into(),
        })?;
        assert!(guard.session_copy_is_stale());
        // No transcript exists: an unlocked stale owner must survive load failure.
        let held = if busy {
            Some(guard)
        } else {
            drop(guard);
            None
        };
        let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
            HashMap::from([(id.clone(), source.clone())]),
        ));
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            restore_for_concurrency_test(&id, &id, &source, &provider, &registry, &sessions, false),
        )
        .await?;
        if busy {
            assert!(Arc::ptr_eq(&result?, &source));
        } else {
            assert!(
                result.is_err(),
                "missing migrated transcript must reject refresh"
            );
        }
        assert!(Arc::ptr_eq(&sessions.read().await[&id], &source));
        drop(held);
        assert!(source.lock().await.has_concurrency_tracking());
        assert!(source.lock().await.session_copy_is_stale());
    }
    Ok(())
}

#[tokio::test]
async fn failed_server_resume_keeps_original_concurrency_owner() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let source = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
    let source_id = source.lock().await.session_id().to_owned();
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::from([(source_id.clone(), source.clone())]),
    ));
    let restored = restore_for_concurrency_test(
        "missing-concurrency-target",
        &source_id,
        &source,
        &provider,
        &registry,
        &sessions,
        false,
    )
    .await?;
    assert!(Arc::ptr_eq(&restored, &source));
    let source = source.lock().await;
    assert_eq!(source.session_id(), source_id);
    assert!(
        source.has_concurrency_tracking(),
        "failed resume must not close the original logical owner"
    );
    Ok(())
}

#[tokio::test]
async fn persisted_restore_reuses_target_published_while_waiting_for_reservation() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let source = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
    let source_id = source.lock().await.session_id().to_owned();
    let target = Arc::new(Mutex::new(Agent::new(
        provider.clone(),
        Registry::new(provider.clone()).await,
    )));
    let target_id = target.lock().await.session_id().to_owned();
    target
        .lock()
        .await
        .session_for_split()
        .clone()
        .save_prepared()?;
    let before = std::fs::read(crate::session::session_path(&target_id)?)?;
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::from([(source_id.clone(), source.clone())]),
    ));
    let reservation = sessions.reserve(&target_id).await;
    let restore = restore_for_concurrency_test(
        &target_id, &source_id, &source, &provider, &registry, &sessions, true,
    );
    tokio::pin!(restore);
    // A competing owner publishes while resume waits on the same reservation.
    assert!(futures::poll!(&mut restore).is_pending());
    reservation.publish(target.clone()).await?;
    drop(reservation);
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), restore).await??;
    assert!(Arc::ptr_eq(&result, &target));
    assert!(Arc::ptr_eq(&sessions.read().await[&target_id], &target));
    assert!(Arc::ptr_eq(&sessions.read().await[&source_id], &source));
    assert_eq!(source.lock().await.session_id(), source_id);
    assert_eq!(
        before,
        std::fs::read(crate::session::session_path(&target_id)?)?
    );
    Ok(())
}

#[tokio::test]
async fn viewer_attach_reuses_live_owner_without_tracking_placeholder() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let live = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
    let live_id = live.lock().await.session_id().to_owned();
    let placeholder = Arc::new(Mutex::new(Agent::new_provisional_with_initial_working_dir(
        provider.clone(),
        registry.clone(),
        None,
    )));
    let placeholder_id = placeholder.lock().await.session_id().to_owned();
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::from([
            (live_id.clone(), live.clone()),
            (placeholder_id.clone(), placeholder.clone()),
        ]),
    ));
    assert!(!placeholder.lock().await.has_concurrency_tracking());
    let attached = restore_for_concurrency_test(
        &live_id,
        &placeholder_id,
        &placeholder,
        &provider,
        &registry,
        &sessions,
        false,
    )
    .await?;
    assert!(Arc::ptr_eq(&attached, &live));
    assert!(live.lock().await.has_concurrency_tracking());
    assert!(
        !placeholder.lock().await.has_concurrency_tracking(),
        "a viewer placeholder must never publish a join"
    );
    Ok(())
}

#[tokio::test]
async fn concurrent_cold_resumes_share_one_restored_runtime() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut persisted = crate::session::Session::create(None, Some("Cold resume race".into()));
    persisted.save()?;
    let target_id = persisted.id.clone();
    let first = Arc::new(Mutex::new(Agent::new_provisional_with_initial_working_dir(
        provider.clone(),
        registry.clone(),
        None,
    )));
    let second = Arc::new(Mutex::new(Agent::new_provisional_with_initial_working_dir(
        provider.clone(),
        registry.clone(),
        None,
    )));
    let first_id = first.lock().await.session_id().to_owned();
    let second_id = second.lock().await.session_id().to_owned();
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::from([
            (first_id.clone(), first.clone()),
            (second_id.clone(), second.clone()),
        ]),
    ));
    // Force both requests to enter before either can restore. Without the
    // target gate each passes live lookup and waits on its own source mutex.
    let first_guard = first.lock().await;
    let second_guard = second.lock().await;
    let first_resume = restore_for_concurrency_test(
        &target_id, &first_id, &first, &provider, &registry, &sessions, false,
    );
    let second_resume = restore_for_concurrency_test(
        &target_id, &second_id, &second, &provider, &registry, &sessions, false,
    );
    tokio::pin!(first_resume, second_resume);
    assert!(futures::poll!(&mut first_resume).is_pending());
    assert!(futures::poll!(&mut second_resume).is_pending());
    drop(first_guard);
    drop(second_guard);
    let (first_result, second_result) =
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(first_resume, second_resume)
        })
        .await
        .expect("concurrent resumes must not deadlock");
    let (first_result, second_result) = (first_result?, second_result?);
    assert!(Arc::ptr_eq(&first_result, &second_result));
    assert!(Arc::ptr_eq(
        &first_result,
        sessions.read().await.get(&target_id).unwrap()
    ));
    assert!(!Arc::ptr_eq(&first_result, &first));
    assert!(!Arc::ptr_eq(&first_result, &second));
    assert!(first_result.lock().await.has_concurrency_tracking());
    let first = first.lock().await;
    let second = second.lock().await;
    assert_eq!(first.session_id(), first_id);
    assert_eq!(second.session_id(), second_id);
    assert!(!first.has_concurrency_tracking());
    assert!(!second.has_concurrency_tracking());
    Ok(())
}

#[tokio::test]
async fn same_session_reattach_preserves_runtime_owner() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedConcurrencyEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);
    let registry = Registry::new(provider.clone()).await;
    let agent = Arc::new(Mutex::new(Agent::new(provider.clone(), registry.clone())));
    let id = agent.lock().await.session_id().to_owned();
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::from([(id.clone(), agent.clone())]),
    ));
    let attached = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        restore_for_concurrency_test(&id, &id, &agent, &provider, &registry, &sessions, false),
    )
    .await
    .expect("same-session reattach must not deadlock")?;
    assert!(Arc::ptr_eq(&attached, &agent));
    assert!(Arc::ptr_eq(sessions.read().await.get(&id).unwrap(), &agent));
    assert!(agent.lock().await.has_concurrency_tracking());
    Ok(())
}

#[tokio::test]
async fn resume_reservations_are_scoped_and_release_on_drop() {
    let sessions = crate::server::session_manager::SessionManager::new();
    let other_sessions = crate::server::session_manager::SessionManager::new();
    let held = sessions.reserve("target").await;
    let same = sessions.reserve("target");
    tokio::pin!(same);
    assert!(futures::poll!(&mut same).is_pending());
    let other_target = sessions.reserve("other");
    tokio::pin!(other_target);
    assert!(futures::poll!(&mut other_target).is_ready());
    let other_manager = other_sessions.reserve("target");
    tokio::pin!(other_manager);
    assert!(futures::poll!(&mut other_manager).is_ready());
    drop(held);
    assert!(
        futures::poll!(&mut same).is_ready(),
        "dropping a reservation must release its gate"
    );
}

#[tokio::test]
async fn comm_status_releases_member_guard_before_reading_connections() {
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        ("caller".to_string(), test_swarm_member("caller", "ready")),
        ("target".to_string(), test_swarm_member("target", "ready")),
    ])));
    let client_connections = Arc::new(RwLock::new(HashMap::<String, ClientConnectionInfo>::new()));
    let sessions = Arc::new(crate::server::session_manager::SessionManager::from_agents(
        HashMap::new(),
    ));
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    // Disconnect cleanup holds client_connections.write, then needs swarm_members.write.
    let cleanup_connections = client_connections.write().await;
    let status = tokio::spawn({
        let (swarm_members, client_connections) =
            (Arc::clone(&swarm_members), Arc::clone(&client_connections));
        async move {
            crate::server::comm_sync::handle_comm_status(
                1,
                "caller".to_string(),
                "target".to_string(),
                &sessions,
                &swarm_members,
                &client_connections,
                &FileTouchService::new(),
                &event_tx,
            )
            .await;
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let _members = tokio::time::timeout(std::time::Duration::from_secs(2), swarm_members.write())
        .await
        .expect("comm status must not hold swarm_members while waiting on client_connections");
    drop(_members);
    drop(cleanup_connections);
    status.await.unwrap();
    assert!(matches!(
        event_rx.recv().await,
        Some(ServerEvent::CommStatusResponse { .. })
    ));
}
