use super::*;
use crate::server::client_disconnect_cleanup::detach_client_attachment;
use crate::server::session_connections::{
    reconcile_user_response, remove_connection_and_reconcile,
};
use std::time::Duration;

type Connections = Arc<RwLock<HashMap<String, ClientConnectionInfo>>>;

fn connection(id: &str, session_id: &str) -> ClientConnectionInfo {
    ClientConnectionInfo {
        client_id: id.into(),
        session_id: session_id.into(),
        supports_user_response: false,
        client_instance_id: None,
        debug_client_id: None,
        connected_at: Instant::now(),
        last_seen: Instant::now(),
        is_processing: false,
        current_tool_name: None,
        terminal_env: Vec::new(),
        disconnect_tx: mpsc::unbounded_channel().0,
    }
}

fn agent(id: &str, registry: Registry) -> Arc<Mutex<Agent>> {
    Arc::new(Mutex::new(build_test_agent_with_id(
        Arc::new(MockProvider),
        registry,
        id,
        Vec::new(),
    )))
}

async fn capability(registry: &Registry) -> bool {
    registry
        .tool_names()
        .await
        .iter()
        .any(|name| name == "user_response")
}

async fn deferred_tasks_finished(connections: &Connections) {
    // Each deferred task owns one map Arc. Waiting for their release verifies
    // completion even when the expected final state equals the initial state.
    tokio::time::timeout(Duration::from_secs(2), async {
        while Arc::strong_count(connections) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("deferred recomputations must finish");
}

#[tokio::test(flavor = "current_thread")]
async fn user_response_busy_snapshots_use_final_aggregate() {
    let _env = crate::storage::lock_test_env();
    for final_enabled in [false, true] {
        let registry = Registry::empty();
        let agent = agent("busy_snapshots", registry.clone());
        agent
            .lock()
            .await
            .set_desktop_response_capability(!final_enabled)
            .await;
        let sessions = Arc::new(RwLock::new(HashMap::from([(
            "busy_snapshots".into(),
            agent.clone(),
        )])));
        let connections = Arc::new(RwLock::new(HashMap::from([
            ("desktop".into(), connection("desktop", "busy_snapshots")),
            ("observer".into(), connection("observer", "busy_snapshots")),
        ])));
        let busy = agent.lock().await;
        for enabled in [true, false, true, false, final_enabled] {
            connections
                .write()
                .await
                .get_mut("desktop")
                .unwrap()
                .supports_user_response = enabled;
            tokio::time::timeout(
                Duration::from_secs(1),
                reconcile_user_response("busy_snapshots", &connections, &sessions),
            )
            .await
            .expect("snapshots must not wait for the running turn");
            // Poll each spawned task while the mutex is still held, putting
            // multiple deferred updates in the mutex's FIFO waiter queue.
            tokio::task::yield_now().await;
        }
        assert!(
            connections.try_write().is_ok(),
            "no connection lock across the agent wait"
        );
        assert!(
            sessions.try_write().is_ok(),
            "no session lock across the agent wait"
        );
        drop(busy);
        deferred_tasks_finished(&connections).await;
        assert_eq!(capability(&registry).await, final_enabled);
    }
}

#[tokio::test]
async fn user_response_busy_rebind_and_duplicate_detach_refresh_actual_sessions() {
    let _env = crate::storage::lock_test_env();
    let registry_a = Registry::empty();
    let registry_b = Registry::empty();
    let a = agent("busy_a", registry_a.clone());
    let b = agent("busy_b", registry_b.clone());
    let sessions = Arc::new(RwLock::new(HashMap::from([
        ("busy_a".into(), a.clone()),
        ("busy_b".into(), b.clone()),
    ])));
    let connections = Arc::new(RwLock::new(HashMap::from([
        ("desktop".into(), connection("desktop", "busy_a")),
        ("observer_a".into(), connection("observer_a", "busy_a")),
        ("observer_b".into(), connection("observer_b", "busy_b")),
    ])));
    connections
        .write()
        .await
        .get_mut("desktop")
        .unwrap()
        .supports_user_response = true;
    reconcile_user_response("busy_a", &connections, &sessions).await;
    assert!(capability(&registry_a).await);
    let busy_a = a.lock().await;
    let busy_b = b.lock().await;
    tokio::time::timeout(Duration::from_secs(1), async {
        for enabled in [false, true] {
            connections
                .write()
                .await
                .get_mut("desktop")
                .unwrap()
                .supports_user_response = enabled;
            reconcile_user_response("busy_a", &connections, &sessions).await;
        }
        let claimed =
            claim_live_target_agent("busy_b", "desktop", None, &a, &sessions, &connections)
                .await
                .expect("busy target is attachable");
        assert!(Arc::ptr_eq(&claimed, &b));
        assert_eq!(connections.read().await["desktop"].session_id, "busy_b");
        let debug = Arc::new(RwLock::new(ClientDebugState::default()));
        let members = Arc::new(RwLock::new(HashMap::new()));
        // Stale lifecycle ID must not prevent refreshing the removed record's B.
        for _ in 0..2 {
            detach_client_attachment(
                "busy_a",
                "desktop",
                "debug",
                &connections,
                &sessions,
                &debug,
                &members,
            )
            .await;
        }
    })
    .await
    .expect("rebind and detach must not await either running agent");
    assert!(connections.try_write().is_ok());
    assert!(sessions.try_write().is_ok());
    drop(busy_b);
    drop(busy_a);
    deferred_tasks_finished(&connections).await;
    assert!(!capability(&registry_a).await);
    assert!(!capability(&registry_b).await);
    assert_eq!(
        connections.read().await.len(),
        2,
        "observers remain attached"
    );
}

#[tokio::test]
async fn user_response_takeover_removes_actual_rebound_session_not_conflict_snapshot() {
    let _env = crate::storage::lock_test_env();
    let registry_a = Registry::empty();
    let registry_b = Registry::empty();
    let a = agent("takeover_a", registry_a.clone());
    let b = agent("takeover_b", registry_b.clone());
    let sessions = Arc::new(RwLock::new(HashMap::from([
        ("takeover_a".into(), a.clone()),
        ("takeover_b".into(), b),
    ])));
    let connections = Arc::new(RwLock::new(HashMap::from([
        ("desktop".into(), connection("desktop", "takeover_a")),
        ("observer_a".into(), connection("observer_a", "takeover_a")),
        ("observer_b".into(), connection("observer_b", "takeover_b")),
    ])));
    connections
        .write()
        .await
        .get_mut("desktop")
        .unwrap()
        .supports_user_response = true;
    reconcile_user_response("takeover_a", &connections, &sessions).await;
    let conflict = connections.read().await["desktop"].clone();
    assert_eq!(conflict.session_id, "takeover_a");
    claim_live_target_agent("takeover_b", "desktop", None, &a, &sessions, &connections)
        .await
        .expect("conflicting client moved while takeover was awaiting");
    assert!(!capability(&registry_a).await);
    assert!(capability(&registry_b).await);
    let removed = remove_connection_and_reconcile(&conflict.client_id, &connections, &sessions)
        .await
        .expect("remove current connection, not the earlier snapshot");
    assert_eq!(removed.session_id, "takeover_b");
    assert!(
        !capability(&registry_b).await,
        "observer keeps B alive but cannot keep the tool"
    );
    assert!(!capability(&registry_a).await);
    assert!(
        remove_connection_and_reconcile(&conflict.client_id, &connections, &sessions)
            .await
            .is_none()
    );
    assert_eq!(connections.read().await.len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn user_response_deferred_map_contention_releases_agent_before_waiting() {
    let _env = crate::storage::lock_test_env();
    for block_connections in [true, false] {
        let registry = Registry::empty();
        let agent = agent("map_contention", registry.clone());
        let sessions = Arc::new(RwLock::new(HashMap::from([(
            "map_contention".into(),
            agent.clone(),
        )])));
        let connections = Arc::new(RwLock::new(HashMap::from([(
            "desktop".into(),
            connection("desktop", "map_contention"),
        )])));
        connections
            .write()
            .await
            .get_mut("desktop")
            .unwrap()
            .supports_user_response = true;
        let busy = agent.lock().await;
        reconcile_user_response("map_contention", &connections, &sessions).await;
        tokio::task::yield_now().await;
        let connections_guard = if block_connections {
            Some(connections.write().await)
        } else {
            None
        };
        let sessions_guard = if block_connections {
            None
        } else {
            Some(sessions.write().await)
        };
        drop(busy);
        tokio::task::yield_now().await;
        assert!(
            agent.try_lock().is_ok(),
            "deferred task must not hold Agent awaiting a map"
        );
        drop(connections_guard);
        drop(sessions_guard);
        deferred_tasks_finished(&connections).await;
        assert!(capability(&registry).await);
    }
}

#[tokio::test]
async fn user_response_deferred_re_resolves_removed_replaced_and_rekeyed_agents() {
    let _env = crate::storage::lock_test_env();
    for change in ["removed", "replaced", "rekeyed", "mismatched"] {
        let registry = Registry::empty();
        let old = agent("old", registry.clone());
        let sessions = Arc::new(RwLock::new(HashMap::from([("old".into(), old.clone())])));
        let connections = Arc::new(RwLock::new(HashMap::from([(
            "desktop".into(),
            connection("desktop", "old"),
        )])));
        connections
            .write()
            .await
            .get_mut("desktop")
            .unwrap()
            .supports_user_response = true;
        let mut busy = old.lock().await;
        reconcile_user_response("old", &connections, &sessions).await;
        let replacement_registry = Registry::empty();
        match change {
            "removed" => {
                sessions.write().await.remove("old");
            }
            "replaced" => {
                sessions
                    .write()
                    .await
                    .insert("old".into(), agent("old", replacement_registry.clone()));
            }
            "rekeyed" | "mismatched" => {
                // Clear/restore can replace the Agent while retaining its Arc.
                *busy = build_test_agent_with_id(
                    Arc::new(MockProvider),
                    registry.clone(),
                    "new",
                    Vec::new(),
                );
                if change == "rekeyed" {
                    sessions.write().await.remove("old");
                    sessions.write().await.insert("new".into(), old.clone());
                    {
                        let mut connections = connections.write().await;
                        let desktop = connections.get_mut("desktop").unwrap();
                        desktop.session_id = "new".into();
                        desktop.supports_user_response = false;
                    }
                    reconcile_user_response("new", &connections, &sessions).await;
                }
            }
            _ => unreachable!(),
        }
        drop(busy);
        deferred_tasks_finished(&connections).await;
        assert!(
            !capability(&registry).await,
            "stale Arc must not be enabled after {change}"
        );
        assert_eq!(
            capability(&replacement_registry).await,
            change == "replaced"
        );
    }
}
