//! Server-side garbage collection of finished and idle swarm members.

use super::*;

const SWARM_TERMINAL_MEMBER_GC_BATCH_SIZE: usize = 64;

pub(super) async fn prune_expired_terminal_swarm_members(
    sessions: &SessionAgents,
    swarm_state: &SwarmState,
    channel_subscriptions: &ChannelSubscriptions,
    channel_subscriptions_by_session: &ChannelSubscriptions,
) -> usize {
    let retention = swarm::swarm_terminal_member_retention();
    let mut candidates = {
        let members = swarm_state.members.read().await;
        expired_terminal_member_ids(&members, retention)
    };
    candidates.truncate(SWARM_TERMINAL_MEMBER_GC_BATCH_SIZE);
    if candidates.is_empty() {
        return 0;
    }

    let live_sessions: HashSet<String> = sessions.read().await.keys().cloned().collect();
    let mut pruned = 0usize;
    for session_id in candidates {
        // A session can be resumed between candidate collection and removal.
        // Never collect a member that currently has a live agent runtime.
        if live_sessions.contains(&session_id) || sessions.read().await.contains_key(&session_id) {
            continue;
        }
        let removed_swarm_id = {
            let mut members = swarm_state.members.write().await;
            let still_expired = members.get(&session_id).is_some_and(|member| {
                swarm::member_status_is_terminal(&member.status)
                    && member.last_status_change.elapsed() >= retention
            });
            if still_expired {
                members
                    .remove(&session_id)
                    .and_then(|member| member.swarm_id)
            } else {
                None
            }
        };
        let Some(swarm_id) = removed_swarm_id else {
            continue;
        };

        remove_session_from_swarm(
            &session_id,
            &swarm_id,
            &swarm_state.members,
            &swarm_state.swarms_by_id,
            &swarm_state.coordinators,
            &swarm_state.plans,
        )
        .await;
        remove_session_channel_subscriptions(
            &session_id,
            channel_subscriptions,
            channel_subscriptions_by_session,
        )
        .await;
        pruned += 1;
    }

    if pruned > 0 {
        crate::logging::info(&format!(
            "Garbage-collected {pruned} expired terminal swarm member(s)"
        ));
    }
    pruned
}

/// Reap spawned swarm workers that finished their work and have sat idle past
/// the reap window: ask any attached client window to close, shut down the
/// server-side agent, and remove the member from swarm state.
///
/// This is the server-side backstop for coordinator `cleanup`: coordinators
/// that get interrupted, replaced, or never call cleanup used to leave every
/// spawned worker running forever (one ~80-150 MB client process each). Only
/// agent-spawned members (`report_back_to_session_id` set) are eligible;
/// user-created sessions are never touched. See
/// [`swarm::idle_spawned_worker_reap_candidates`] for the exact policy.
pub(super) async fn reap_idle_spawned_workers(
    sessions: &SessionAgents,
    swarm_state: &SwarmState,
    channel_subscriptions: &ChannelSubscriptions,
    channel_subscriptions_by_session: &ChannelSubscriptions,
    soft_interrupt_queues: &SessionInterruptQueues,
) -> usize {
    let Some(idle_after) = swarm::swarm_idle_worker_reap_after() else {
        return 0;
    };
    let candidates = {
        let members = swarm_state.members.read().await;
        swarm::idle_spawned_worker_reap_candidates(&members, idle_after)
    };
    if candidates.is_empty() {
        return 0;
    }

    let mut reaped = 0usize;
    for session_id in candidates {
        // Re-validate under the current map: status may have changed between
        // candidate collection and removal (a resumed/reassigned worker).
        let still_reapable = {
            let members = swarm_state.members.read().await;
            members.get(&session_id).is_some_and(|member| {
                member.report_back_to_session_id.is_some()
                    && member.role != "coordinator"
                    && (member.status == "ready"
                        || swarm::member_status_is_terminal(&member.status))
                    && member.last_status_change.elapsed() >= idle_after
            })
        };
        if !still_reapable {
            continue;
        }

        // Ask any attached client (visible spawned window) to close itself.
        let notified_clients = fanout_session_event(
            &swarm_state.members,
            &session_id,
            ServerEvent::SessionCloseRequested {
                reason: format!(
                    "Idle spawned worker reaped after {}s of inactivity",
                    idle_after.as_secs()
                ),
            },
        )
        .await;
        crate::logging::info(&format!(
            "Reaping idle spawned worker {session_id} ({notified_clients} attached client(s) asked to close)"
        ));

        if let Some(agent_arc) = remove_session_entry(sessions, &session_id).await {
            remove_session_interrupt_queue(soft_interrupt_queues, &session_id).await;
            remove_background_tool_signal(&session_id);
            if let Ok(mut agent) = agent_arc.try_lock() {
                agent.mark_closed();
            }
        }

        let removed_swarm_id = {
            let mut members = swarm_state.members.write().await;
            members
                .remove(&session_id)
                .and_then(|member| member.swarm_id)
        };
        if let Some(ref swarm_id) = removed_swarm_id {
            remove_session_from_swarm(
                &session_id,
                swarm_id,
                &swarm_state.members,
                &swarm_state.swarms_by_id,
                &swarm_state.coordinators,
                &swarm_state.plans,
            )
            .await;
        }
        remove_session_channel_subscriptions(
            &session_id,
            channel_subscriptions,
            channel_subscriptions_by_session,
        )
        .await;
        crate::logging::info(&format!(
            "Reaped idle spawned swarm worker {session_id} (idle > {}s)",
            idle_after.as_secs()
        ));
        reaped += 1;
    }
    reaped
}
