use super::*;

#[test]
fn agent_drop_removes_its_configured_session_tool_policy() {
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let session = Session::create(None, None);
    let session_id = session.id.clone();
    let agent = Agent::new_with_session(
        provider,
        Registry::empty(),
        session,
        Some(HashSet::from(["bash".to_string()])),
    );

    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&session_id, "bash"),
        Some(true)
    );
    drop(agent);
    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&session_id, "bash"),
        None,
        "dropping the Agent must remove its global policy entry"
    );
}

#[test]
fn stale_agent_drop_preserves_successor_session_tool_policy() {
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let first_session = Session::create(None, None);
    let session_id = first_session.id.clone();
    let first = Agent::new_with_session(
        provider.clone(),
        Registry::empty(),
        first_session,
        Some(HashSet::from(["bash".to_string()])),
    );
    let mut successor_session = Session::create(None, None);
    successor_session.id.clone_from(&session_id);
    let successor = Agent::new_with_session(
        provider,
        Registry::empty(),
        successor_session,
        Some(HashSet::from(["read".to_string()])),
    );

    drop(first);

    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&session_id, "read"),
        Some(true),
        "a stale Agent must not remove its active successor's policy"
    );
    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&session_id, "bash"),
        Some(false),
        "the surviving entry must be the successor's configured policy"
    );
    drop(successor);
    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&session_id, "read"),
        None
    );
}

#[test]
fn agent_clear_moves_tool_policy_registration_to_new_session() {
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let session = Session::create(None, None);
    let previous_session_id = session.id.clone();
    let mut agent = Agent::new_with_session(
        provider,
        Registry::empty(),
        session,
        Some(HashSet::from(["bash".to_string()])),
    );

    agent.clear();
    let new_session_id = agent.session.id.clone();

    assert_ne!(previous_session_id, new_session_id);
    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&previous_session_id, "bash"),
        None,
        "changing sessions must remove the former ID's policy"
    );
    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&new_session_id, "bash"),
        Some(true),
        "the new session must retain the Agent's configured policy"
    );
    drop(agent);
    assert_eq!(
        crate::tool::session_tool_policy_allows_tool_for_test(&new_session_id, "bash"),
        None
    );
}
