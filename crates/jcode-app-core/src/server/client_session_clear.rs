//! Connection-local clear keeps other clients bound to their existing session.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::server) async fn handle_clear_session(
    id: u64,
    client_selfdev: bool,
    client_session_id: &mut String,
    client_connection_id: &str,
    agent: &mut Arc<Mutex<Agent>>,
    provider: &mut Arc<dyn Provider>,
    registry: &mut Registry,
    sessions: &SessionAgents,
    shutdown_signals: &Arc<RwLock<HashMap<String, InterruptSignal>>>,
    soft_interrupt_queues: &SessionInterruptQueues,
    client_connections: &Arc<RwLock<HashMap<String, ClientConnectionInfo>>>,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
    swarms_by_id: &Arc<RwLock<HashMap<String, HashSet<String>>>>,
    file_touch: &FileTouchService,
    channel_subscriptions: &ChannelSubscriptions,
    channel_subscriptions_by_session: &ChannelSubscriptions,
    swarm_plans: &Arc<RwLock<HashMap<String, VersionedPlan>>>,
    swarm_coordinators: &Arc<RwLock<HashMap<String, String>>>,
    mcp_pool: &Arc<crate::mcp::SharedMcpPool>,
    event_history: &Arc<RwLock<std::collections::VecDeque<SwarmEvent>>>,
    event_counter: &Arc<std::sync::atomic::AtomicU64>,
    swarm_event_tx: &broadcast::Sender<SwarmEvent>,
    client_event_tx: &mpsc::UnboundedSender<ServerEvent>,
) {
    let clear_start = Instant::now();
    let old_session_id = client_session_id.clone();
    crate::logging::event_info(
        "SESSION_LIFECYCLE",
        vec![
            ("phase", "clear_start".to_string()),
            ("request_id", id.to_string()),
            ("session_id", old_session_id.clone()),
            ("client_connection_id", client_connection_id.to_string()),
            ("client_selfdev", client_selfdev.to_string()),
        ],
    );
    let old_agent = Arc::clone(agent);
    let (preserve_debug, working_dir, new_provider) = {
        let agent_guard = agent.lock().await;
        (
            agent_guard.is_debug(),
            agent_guard.working_dir().map(str::to_string),
            agent_guard.provider_fork(),
        )
    };
    // Registry::clone shares the tools map. Fresh bindings isolate both tools
    // and mutable provider state while peers keep the old session unchanged.
    let new_registry = Registry::new(Arc::clone(&new_provider)).await;
    let mut new_agent = Agent::new_with_initial_working_dir(
        Arc::clone(&new_provider),
        new_registry.clone(),
        working_dir.as_deref(),
    );
    let new_id = new_agent.session_id().to_string();
    if client_selfdev {
        new_agent.set_canary("self-dev");
    }
    if preserve_debug {
        new_agent.set_debug(true);
    }
    {
        // Canonical order serializes with resume/disconnect. No Agent await
        // while holding maps, and no preflight singleton/sharedness snapshot.
        let mut connections = client_connections.write().await;
        let mut sessions_guard = sessions.write().await;
        let owns_source = connections
            .get(client_connection_id)
            .is_some_and(|info| info.session_id == old_session_id)
            && sessions_guard
                .get(&old_session_id)
                .is_some_and(|source| Arc::ptr_eq(source, &old_agent))
            && old_agent
                .try_lock()
                .is_ok_and(|source| source.session_id() == old_session_id);
        if !owns_source {
            if client_event_tx
                .send(ServerEvent::Error {
                    id,
                    message:
                        "Session changed or became busy while preparing clear; retry the request."
                            .into(),
                    retry_after_secs: None,
                })
                .is_err()
            {
                crate::logging::debug("Clear error response receiver disconnected");
            }
            return;
        }
        *agent = Arc::new(Mutex::new(new_agent));
        *provider = new_provider;
        *registry = new_registry;
        *client_session_id = new_id.clone();
        sessions_guard.insert(new_id.clone(), Arc::clone(agent));
        if let Some(info) = connections.get_mut(client_connection_id) {
            info.session_id = new_id.clone();
            info.last_seen = Instant::now();
        }
    }
    reconcile_user_response(&old_session_id, client_connections, sessions).await;
    reconcile_user_response(&new_id, client_connections, sessions).await;
    crate::runtime_memory_log::emit_event(
        crate::runtime_memory_log::RuntimeMemoryLogEvent::new(
            "session_cleared",
            "session_replaced_with_fresh_agent",
        )
        .with_session_id(new_id.clone())
        .force_attribution(),
    );
    {
        let agent_guard = agent.lock().await;
        register_session_interrupt_queue(
            soft_interrupt_queues,
            &new_id,
            agent_guard.soft_interrupt_queue(),
        )
        .await;
        let mut signals = shutdown_signals.write().await;
        signals.insert(new_id.clone(), agent_guard.graceful_shutdown_signal());
        drop(signals);
        register_background_tool_signal(&new_id, agent_guard.background_tool_signal());
    }
    let (swarm_id_for_update, swarm_enabled, friendly_name) = {
        let members = swarm_members.read().await;
        match members.get(&old_session_id) {
            Some(member) => (
                member.swarm_id.clone(),
                member.swarm_enabled,
                member.friendly_name.clone(),
            ),
            None => (None, false, None),
        }
    };
    // Unregister only this sender. Existing cleanup preserves claimed source
    // state, or closes/removes it if the last peer left during preparation.
    cleanup_detached_source_session_if_unused(
        &old_session_id,
        client_connection_id,
        &old_agent,
        sessions,
        shutdown_signals,
        soft_interrupt_queues,
        client_connections,
        swarm_members,
        swarms_by_id,
        file_touch,
        channel_subscriptions,
        channel_subscriptions_by_session,
        swarm_plans,
        swarm_coordinators,
    )
    .await;
    // Fresh root, without inheriting the old session's swarm/plan participation.
    ensure_client_swarm_member(
        &new_id,
        client_connection_id,
        &friendly_name,
        client_event_tx,
        agent,
        swarm_enabled,
        swarm_members,
        swarms_by_id,
        event_history,
        event_counter,
        swarm_event_tx,
    )
    .await;
    update_member_status(
        &new_id,
        "ready",
        None,
        swarm_members,
        swarms_by_id,
        Some(event_history),
        Some(event_counter),
        Some(swarm_event_tx),
    )
    .await;
    // Clear needs no later Subscribe, so rehydrate the new registry now.
    if client_selfdev {
        registry.register_selfdev_tools().await;
    }
    registry
        .register_mcp_tools_for_dir(
            Some(client_event_tx.clone()),
            Some(Arc::clone(mcp_pool)),
            Some(new_id.clone()),
            working_dir.map(PathBuf::from),
        )
        .await;
    if client_event_tx
        .send(ServerEvent::SessionId { session_id: new_id })
        .is_err()
        || client_event_tx.send(ServerEvent::Done { id }).is_err()
    {
        crate::logging::debug("Clear completion receiver disconnected");
        return;
    }
    crate::logging::event_info(
        "SESSION_LIFECYCLE",
        vec![
            ("phase", "clear_done".to_string()),
            ("request_id", id.to_string()),
            ("old_session_id", old_session_id),
            ("new_session_id", client_session_id.clone()),
            ("client_connection_id", client_connection_id.to_string()),
            ("preserve_debug", preserve_debug.to_string()),
            (
                "swarm_id_updated",
                swarm_id_for_update.is_some().to_string(),
            ),
            ("elapsed_ms", clear_start.elapsed().as_millis().to_string()),
        ],
    );
}
