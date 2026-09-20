fn swarm_stop_allowed_by_owner(
    req_session_id: &str,
    target_member: &SwarmMember,
    force: bool,
) -> bool {
    force || target_member.report_back_to_session_id.as_deref() == Some(req_session_id)
}

fn clear_persisted_stop_queue(session_id: &str) -> anyhow::Result<()> {
    crate::soft_interrupt_store::clear(session_id)?;
    let remaining = crate::soft_interrupt_store::load(session_id)?;
    anyhow::ensure!(
        remaining.is_empty(),
        "{} queued soft interrupt(s) remain after clear",
        remaining.len()
    );
    Ok(())
}

fn clear_live_stop_queue(queue: &jcode_agent_runtime::SoftInterruptQueue) -> anyhow::Result<()> {
    let mut pending = queue
        .lock()
        .map_err(|_| anyhow::anyhow!("live soft-interrupt queue lock is poisoned"))?;
    pending.clear();
    anyhow::ensure!(
        pending.is_empty(),
        "live soft-interrupt queue remains nonempty"
    );
    Ok(())
}

fn send_stop_error(client_event_tx: &mpsc::UnboundedSender<ServerEvent>, id: u64, message: String) {
    if client_event_tx
        .send(ServerEvent::Error {
            id,
            message,
            retry_after_secs: None,
        })
        .is_err()
    {
        crate::logging::info("Stop error response dropped because the client disconnected");
    }
}

async fn acquire_terminal_agent_guard(
    agent: Option<Arc<Mutex<Agent>>>,
    session_control: &SessionControlHandle,
) -> Result<Option<tokio::sync::OwnedMutexGuard<Agent>>, ()> {
    let Some(agent) = agent else {
        return Ok(None);
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            // A live wake can reserve the Agent before its run_turn signal is
            // registered. Re-fire so that late registration cannot miss stop.
            session_control.request_cancel();
            if let Ok(agent) = Arc::clone(&agent).try_lock_owned() {
                return agent;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .map(Some)
    .map_err(|_| ())
}

async fn resolve_stop_target_session(
    swarm_id: &str,
    target: &str,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
) -> std::result::Result<String, String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("target_session is required.".to_string());
    }

    let members = swarm_members.read().await;
    if members
        .get(target)
        .is_some_and(|member| member.swarm_id.as_deref() == Some(swarm_id))
    {
        return Ok(target.to_string());
    }

    let mut matches = members
        .iter()
        .filter(|(_, member)| member.swarm_id.as_deref() == Some(swarm_id))
        .filter(|(session_id, member)| {
            member.friendly_name.as_deref() == Some(target)
                || session_id.starts_with(target)
                || session_id.ends_with(target)
        })
        .map(|(session_id, member)| {
            (
                session_id.clone(),
                member
                    .friendly_name
                    .as_deref()
                    .unwrap_or(session_id)
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| a.0.cmp(&b.0));

    match matches.len() {
        0 => Err(format!(
            "Unknown swarm session '{target}'. Use an exact session ID, unique friendly name, or unique session ID prefix/suffix."
        )),
        1 => Ok(matches.remove(0).0),
        _ => Err(format!(
            "Ambiguous swarm session '{target}' matched: {}. Use an exact session ID.",
            matches
                .iter()
                .map(|(session_id, friendly)| format!("{friendly} [{session_id}]"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_comm_stop(
    id: u64,
    req_session_id: String,
    target_session: String,
    force: bool,
    client_event_tx: &mpsc::UnboundedSender<ServerEvent>,
    sessions: &SessionAgents,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
    swarms_by_id: &Arc<RwLock<HashMap<String, HashSet<String>>>>,
    swarm_coordinators: &Arc<RwLock<HashMap<String, String>>>,
    swarm_plans: &Arc<RwLock<HashMap<String, VersionedPlan>>>,
    channel_subscriptions: &ChannelSubscriptions,
    channel_subscriptions_by_session: &ChannelSubscriptions,
    event_history: &Arc<RwLock<std::collections::VecDeque<SwarmEvent>>>,
    event_counter: &Arc<std::sync::atomic::AtomicU64>,
    swarm_event_tx: &broadcast::Sender<SwarmEvent>,
    soft_interrupt_queues: &SessionInterruptQueues,
    swarm_mutation_runtime: &SwarmMutationRuntime,
) {
    // Stopping is authorized per-target by ownership (the requester is the
    // target's spawner or a transitive ancestor) rather than by the swarm-level
    // coordinator slot, so that any parent can stop agents in its own subtree.
    // We only require the requester to be a member of a swarm here; the concrete
    // permission check happens below via `stop_allowed`.
    let swarm_id = {
        let members = swarm_members.read().await;
        members
            .get(&req_session_id)
            .and_then(|member| member.swarm_id.clone())
    };
    let Some(swarm_id) = swarm_id else {
        send_stop_error(client_event_tx, id, "Not in a swarm.".to_string());
        return;
    };

    let target_session =
        match resolve_stop_target_session(&swarm_id, &target_session, swarm_members).await {
            Ok(target_session) => target_session,
            Err(message) => {
                send_stop_error(client_event_tx, id, message);
                return;
            }
        };

    let stop_allowed = {
        let members = swarm_members.read().await;
        members
            .get(&target_session)
            .map(|member| {
                swarm_stop_allowed_by_owner(&req_session_id, member, force)
                    || (!force
                        && super::swarm_is_self_or_ancestor(
                            &members,
                            &req_session_id,
                            &target_session,
                        ))
            })
            .unwrap_or(false)
    };
    if !stop_allowed {
        send_stop_error(
            client_event_tx,
            id,
            format!(
                "Refusing to stop session '{target_session}' because it was not spawned by this coordinator. Pass force=true to stop a non-owned/user-created swarm session explicitly."
            ),
        );
        return;
    }

    fanout_session_event(
        swarm_members,
        &target_session,
        ServerEvent::SessionCloseRequested {
            reason: format!("Stopped by coordinator {req_session_id}"),
        },
    )
    .await;

    let mutation_key = request_key(&req_session_id, "stop", &[swarm_id, target_session.clone()]);
    let Some(mutation_state) = begin_or_join_in_flight(
        swarm_mutation_runtime,
        &mutation_key,
        "stop",
        &req_session_id,
        id,
        client_event_tx,
    )
    .await
    else {
        return;
    };

    update_member_status(
        &target_session,
        "stopping",
        Some(format!("Cancellation requested by {req_session_id}")),
        swarm_members,
        swarms_by_id,
        Some(event_history),
        Some(event_counter),
        Some(swarm_event_tx),
    )
    .await;

    // Flip the queue lifecycle gate before clearing. Existing producers drain
    // first; later live or persisted producers are refused until explicit resume.
    begin_session_interrupt_stop(&target_session).await;
    let live_agent = sessions.read().await.get(&target_session).cloned();
    let soft_interrupt_queue = soft_interrupt_queues
        .read()
        .await
        .get(&target_session)
        .cloned()
        .unwrap_or_else(|| Arc::new(StdMutex::new(Vec::new())));
    let session_control = SessionControlHandle::cancel_only(
        &target_session,
        soft_interrupt_queue.clone(),
        InterruptSignal::new(),
    );
    if let Err(error) = clear_live_stop_queue(&soft_interrupt_queue) {
        crate::logging::warn(&format!(
            "Initial live soft-interrupt clear failed while stopping session {}: {}",
            target_session, error
        ));
    }
    if let Err(error) = clear_persisted_stop_queue(&target_session) {
        crate::logging::warn(&format!(
            "Initial persisted soft-interrupt clear failed while stopping session {}: {}",
            target_session, error
        ));
    }

    let mut terminal_agent = match acquire_terminal_agent_guard(live_agent, &session_control).await
    {
        Ok(agent) => agent,
        Err(()) => {
            finish_request(
                swarm_mutation_runtime,
                &mutation_state,
                PersistedSwarmMutationResponse::Error {
                    message: format!(
                        "Cancellation was requested for session '{target_session}', but its active turn did not stop within 5 seconds. The target remains in stopping state and may be stopped again; do not schedule replacement work yet."
                    ),
                    retry_after_secs: None,
                },
            )
            .await;
            return;
        }
    };

    // The stopping gate is still set and every earlier producer has drained, so
    // these verified clears are stable through routing removal and Done.
    if let Err(error) = clear_live_stop_queue(&soft_interrupt_queue) {
        finish_request(
            swarm_mutation_runtime,
            &mutation_state,
            PersistedSwarmMutationResponse::Error {
                message: format!(
                    "Session '{target_session}' quiesced, but queued live work could not be cleared: {error}. The target remains retryable in stopping state."
                ),
                retry_after_secs: None,
            },
        )
        .await;
        return;
    }
    if let Err(error) = clear_persisted_stop_queue(&target_session) {
        finish_request(
            swarm_mutation_runtime,
            &mutation_state,
            PersistedSwarmMutationResponse::Error {
                message: format!(
                    "Session '{target_session}' quiesced, but persisted queued work could not be cleared: {error}. The target remains retryable in stopping state."
                ),
                retry_after_secs: None,
            },
        )
        .await;
        return;
    }

    let extraction = terminal_agent.as_mut().and_then(|agent| {
        agent.mark_closed();
        agent.memory_enabled().then(|| {
            (
                agent.build_transcript_for_extraction(),
                target_session.clone(),
                agent.working_dir().map(str::to_string),
            )
        })
    });

    let removed_live_agent = super::remove_session_entry(sessions, &target_session)
        .await
        .is_some();
    remove_session_interrupt_queue(soft_interrupt_queues, &target_session).await;
    remove_background_tool_signal(&target_session);

    let (removed_swarm_id, removed_name) = {
        let mut members = swarm_members.write().await;
        if let Some(member) = members.remove(&target_session) {
            (member.swarm_id, member.friendly_name)
        } else {
            (None, None)
        }
    };
    if let Some(ref swarm_id) = removed_swarm_id {
        record_swarm_event(
            event_history,
            event_counter,
            swarm_event_tx,
            target_session.clone(),
            removed_name.clone(),
            Some(swarm_id.clone()),
            SwarmEventType::MemberChange {
                action: "left".to_string(),
            },
        )
        .await;
        remove_session_from_swarm(
            &target_session,
            swarm_id,
            swarm_members,
            swarms_by_id,
            swarm_coordinators,
            swarm_plans,
        )
        .await;
    }
    remove_session_channel_subscriptions(
        &target_session,
        channel_subscriptions,
        channel_subscriptions_by_session,
    )
    .await;

    if let Some((transcript, session_id, working_dir)) = extraction {
        crate::memory_agent::trigger_final_extraction_with_dir(transcript, session_id, working_dir);
    }

    let response = if removed_live_agent || removed_swarm_id.is_some() {
        PersistedSwarmMutationResponse::Done
    } else {
        PersistedSwarmMutationResponse::Error {
            message: format!("Unknown session '{target_session}'"),
            retry_after_secs: None,
        }
    };
    // Retain terminal_agent across response persistence and delivery so no local
    // turn or tool dispatch can reacquire the Agent before Done is observable.
    if matches!(response, PersistedSwarmMutationResponse::Done) {
        complete_session_interrupt_stop(&target_session);
    }
    finish_request(swarm_mutation_runtime, &mutation_state, response).await;
    drop(terminal_agent);
}
