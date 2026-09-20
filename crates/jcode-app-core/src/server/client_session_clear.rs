#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_clear_session(
    id: u64,
    client_selfdev: bool,
    client_session_id: &mut String,
    client_connection_id: &str,
    agent: &Arc<Mutex<Agent>>,
    provider: &Arc<dyn Provider>,
    registry: &Registry,
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
    let (preserve_debug, working_dir) = {
        let agent_guard = agent.lock().await;
        (
            agent_guard.is_debug(),
            agent_guard.working_dir().map(str::to_string),
        )
    };

    {
        let mut agent_guard = agent.lock().await;
        agent_guard.mark_closed();
    }

    let mut new_agent = Agent::new_with_initial_working_dir(
        Arc::clone(provider),
        registry.clone(),
        working_dir.as_deref(),
    );
    let new_id = new_agent.session_id().to_string();

    if client_selfdev {
        new_agent.set_canary("self-dev");
    }
    if preserve_debug {
        new_agent.set_debug(true);
    }

    // `/clear` replaces the live session with a fresh one. Persist it now so a
    // resume-by-id (or the session menu) finds the replacement instead of
    // treating it as missing.
    new_agent.persist_session_for_resume_best_effort("session clear replacement");

    let mut agent_guard = agent.lock().await;
    *agent_guard = new_agent;
    drop(agent_guard);

    {
        let mut sessions_guard = sessions.write().await;
        sessions_guard.remove(client_session_id);
        sessions_guard.insert(new_id.clone(), Arc::clone(agent));
    }
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
        signals.remove(client_session_id);
        signals.insert(new_id.clone(), agent_guard.graceful_shutdown_signal());
        drop(signals);
        remove_background_tool_signal(client_session_id);
        register_background_tool_signal(&new_id, agent_guard.background_tool_signal());
    }
    remove_session_interrupt_queue(soft_interrupt_queues, client_session_id).await;

    // `/clear` creates a genuinely fresh session. Do not migrate the old
    // session's swarm membership or plan participation to the replacement:
    // doing so lets a subsequent plan snapshot repopulate the cleared UI.
    let (swarm_id_for_update, swarm_enabled, friendly_name) = {
        let mut members = swarm_members.write().await;
        match members.remove(client_session_id) {
            Some(member) => (member.swarm_id, member.swarm_enabled, member.friendly_name),
            None => (None, false, None),
        }
    };
    if let Some(ref swarm_id) = swarm_id_for_update {
        let mut swarms = swarms_by_id.write().await;
        if let Some(swarm) = swarms.get_mut(swarm_id) {
            swarm.remove(client_session_id);
            if swarm.is_empty() {
                swarms.remove(swarm_id);
            }
        }
    }
    file_touch.clear_session(client_session_id).await;
    remove_session_channel_subscriptions(
        client_session_id,
        channel_subscriptions,
        channel_subscriptions_by_session,
    )
    .await;
    // The connection remains subscribed across `/clear`, so there is no later
    // subscribe request to register the replacement session. Register it as a
    // fresh root while deliberately leaving the old swarm and plan behind.
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
    if let Some(ref swarm_id) = swarm_id_for_update {
        remove_plan_participant(swarm_id, client_session_id, swarm_plans).await;
    }

    *client_session_id = new_id.clone();
    {
        let mut connections = client_connections.write().await;
        if let Some(info) = connections.get_mut(client_connection_id) {
            info.session_id = new_id.clone();
            info.last_seen = Instant::now();
        }
    }
    let _ = client_event_tx.send(ServerEvent::SessionId { session_id: new_id });
    let _ = client_event_tx.send(ServerEvent::Done { id });
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
