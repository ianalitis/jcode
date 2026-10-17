//! Issue #1650: resuming a session must not attribute it to the temporary
//! connection's swarm, and a persisted member must keep its own swarm.

use super::*;

fn member_in(session_id: &str, swarm_id: &str, owner: Option<&str>) -> SwarmMember {
    SwarmMember {
        swarm_id: Some(swarm_id.to_string()),
        report_back_to_session_id: owner.map(str::to_string),
        ..test_swarm_member(session_id, "stopped")
    }
}

#[tokio::test]
async fn persisted_resume_preserves_source_member_children_and_sender_isolation() {
    for existing_target in [false, true] {
        let source = "session_source";
        let target = "session_target";
        let source_swarm = format!("session:{source}");
        let target_swarm = format!("session:{target}");
        let (source_tx, mut source_rx) = mpsc::unbounded_channel();
        let (incoming_tx, mut incoming_rx) = mpsc::unbounded_channel();
        let (target_tx, mut target_rx) = mpsc::unbounded_channel();
        let mut source_member = member_in(source, &source_swarm, None);
        source_member.event_tx = source_tx.clone();
        source_member.event_txs = HashMap::from([("peer".into(), source_tx)]);
        let members = Arc::new(RwLock::new(HashMap::from([
            (source.into(), source_member),
            (
                "child".into(),
                member_in("child", &source_swarm, Some(source)),
            ),
        ])));
        let swarms = Arc::new(RwLock::new(HashMap::from([(
            source_swarm.clone(),
            HashSet::from([source.into(), "child".into()]),
        )])));
        if existing_target {
            let mut persisted = member_in(target, "session:stale_foreign_root", None);
            persisted.event_tx = target_tx.clone();
            persisted.event_txs = HashMap::from([("target-peer".into(), target_tx)]);
            members.write().await.insert(target.into(), persisted);
            swarms.write().await.insert(
                "session:stale_foreign_root".into(),
                HashSet::from([target.into()]),
            );
        }
        let outcome = rename_swarm_member_session(
            source,
            target,
            &members,
            &swarms,
            Some(("incoming", &incoming_tx)),
        )
        .await;
        assert_eq!(outcome.new_swarm_id.as_deref(), Some(target_swarm.as_str()));
        assert_eq!(outcome.healed_swarm_id.is_some(), existing_target);
        let guard = members.read().await;
        assert_eq!(
            guard[source].swarm_id.as_deref(),
            Some(source_swarm.as_str())
        );
        assert_eq!(
            guard["child"].report_back_to_session_id.as_deref(),
            Some(source)
        );
        assert_eq!(guard[source].event_txs.len(), 1);
        assert!(guard[source].event_txs.contains_key("peer"));
        assert_eq!(
            guard[target].event_txs.len(),
            if existing_target { 2 } else { 1 }
        );
        assert!(!guard[target].event_txs.contains_key("peer"));
        drop(guard);
        assert_eq!(
            swarms.read().await[&source_swarm],
            HashSet::from([source.into(), "child".into()])
        );
        crate::server::fanout_session_event(&members, target, ServerEvent::Done { id: 77 }).await;
        assert!(matches!(
            incoming_rx.try_recv(),
            Ok(ServerEvent::Done { id: 77 })
        ));
        assert!(source_rx.try_recv().is_err());
        if existing_target {
            assert!(matches!(
                target_rx.try_recv(),
                Ok(ServerEvent::Done { id: 77 })
            ));
        }
        crate::server::fanout_session_event(&members, source, ServerEvent::Done { id: 78 }).await;
        assert!(matches!(
            source_rx.try_recv(),
            Ok(ServerEvent::Done { id: 78 })
        ));
        assert!(incoming_rx.try_recv().is_err());
    }
}

#[tokio::test]
async fn resume_keeps_persisted_member_swarm_and_drops_temporary_swarm() {
    let temp = "session_temp_1";
    let resumed = "session_resumed_1";
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (
            temp.to_string(),
            member_in(temp, &format!("session:{temp}"), None),
        ),
        (
            resumed.to_string(),
            member_in(resumed, &format!("session:{resumed}"), None),
        ),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([
        (format!("session:{temp}"), HashSet::from([temp.to_string()])),
        (
            format!("session:{resumed}"),
            HashSet::from([resumed.to_string()]),
        ),
    ])));

    let rename =
        rename_swarm_member_session(temp, resumed, &swarm_members, &swarms_by_id, None).await;

    assert_eq!(rename.old_swarm_id, Some(format!("session:{temp}")));
    assert_eq!(rename.new_swarm_id, Some(format!("session:{resumed}")));
    let members = swarm_members.read().await;
    assert!(!members.contains_key(temp));
    let member = members.get(resumed).expect("resumed member");
    assert_eq!(member.swarm_id, Some(format!("session:{resumed}")));
    assert_eq!(member.status, "ready");
    drop(members);
    let swarms = swarms_by_id.read().await;
    assert!(
        !swarms.contains_key(&format!("session:{temp}")),
        "the temporary swarm must not survive naming the resumed session"
    );
    assert_eq!(
        swarms.get(&format!("session:{resumed}")),
        Some(&HashSet::from([resumed.to_string()]))
    );
}

#[tokio::test]
async fn resume_without_persisted_member_rekeys_temporary_session_swarm() {
    let temp = "session_temp_2";
    let resumed = "session_resumed_2";
    let swarm_members = Arc::new(RwLock::new(HashMap::from([(
        temp.to_string(),
        member_in(temp, &format!("session:{temp}"), None),
    )])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
        format!("session:{temp}"),
        HashSet::from([temp.to_string()]),
    )])));

    rename_swarm_member_session(temp, resumed, &swarm_members, &swarms_by_id, None).await;

    let members = swarm_members.read().await;
    assert_eq!(
        members.get(resumed).and_then(|m| m.swarm_id.clone()),
        Some(format!("session:{resumed}"))
    );
    drop(members);
    let swarms = swarms_by_id.read().await;
    assert_eq!(swarms.len(), 1);
    assert_eq!(
        swarms.get(&format!("session:{resumed}")),
        Some(&HashSet::from([resumed.to_string()]))
    );
}

#[tokio::test]
async fn resume_heals_root_member_that_claims_a_foreign_session_swarm() {
    // State written by older builds: the resumed root session's own record
    // names a dead session's swarm, and several stale swarms list it.
    let temp = "session_temp_3";
    let resumed = "session_blowfish";
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (
            temp.to_string(),
            member_in(temp, &format!("session:{temp}"), None),
        ),
        (
            resumed.to_string(),
            member_in(resumed, "session:session_rooster", None),
        ),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([
        (format!("session:{temp}"), HashSet::from([temp.to_string()])),
        (
            "session:session_rooster".to_string(),
            HashSet::from([resumed.to_string()]),
        ),
        (
            "session:session_ox".to_string(),
            HashSet::from([resumed.to_string()]),
        ),
    ])));

    let rename =
        rename_swarm_member_session(temp, resumed, &swarm_members, &swarms_by_id, None).await;

    assert_eq!(
        rename.healed_swarm_id.as_deref(),
        Some("session:session_rooster")
    );
    assert_eq!(rename.new_swarm_id, Some(format!("session:{resumed}")));
    let swarms = swarms_by_id.read().await;
    assert_eq!(swarms.len(), 1, "stale swarms still list it: {swarms:?}");
    assert!(swarms.contains_key(&format!("session:{resumed}")));
}

#[tokio::test]
async fn resume_keeps_spawned_worker_in_its_coordinator_swarm() {
    let temp = "session_temp_4";
    let worker = "session_worker";
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (
            temp.to_string(),
            member_in(temp, &format!("session:{temp}"), None),
        ),
        (
            worker.to_string(),
            member_in(worker, "session:session_coord", Some("session_coord")),
        ),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([
        (format!("session:{temp}"), HashSet::from([temp.to_string()])),
        (
            "session:session_coord".to_string(),
            HashSet::from(["session_coord".to_string(), worker.to_string()]),
        ),
    ])));

    let rename =
        rename_swarm_member_session(temp, worker, &swarm_members, &swarms_by_id, None).await;

    assert_eq!(rename.healed_swarm_id, None);
    assert_eq!(
        rename.new_swarm_id.as_deref(),
        Some("session:session_coord")
    );
    let members = swarm_members.read().await;
    let member = members.get(worker).expect("worker");
    assert_eq!(
        member.report_back_to_session_id.as_deref(),
        Some("session_coord")
    );
}
