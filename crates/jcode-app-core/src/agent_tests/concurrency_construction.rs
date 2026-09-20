use super::*;

struct IsolatedTelemetryEnv {
    _home: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl IsolatedTelemetryEnv {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let previous = ["JCODE_HOME", "JCODE_NO_TELEMETRY"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        crate::env::set_var("JCODE_HOME", home.path());
        crate::env::set_var("JCODE_NO_TELEMETRY", "1");
        Self {
            _home: home,
            previous,
        }
    }
}

impl Drop for IsolatedTelemetryEnv {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            match value {
                Some(value) => crate::env::set_var(key, value),
                None => crate::env::remove_var(key),
            }
        }
    }
}

#[tokio::test]
async fn provisional_connection_does_not_track_until_logical_ownership_commits() {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedTelemetryEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut agent = Agent::new_provisional_with_initial_working_dir(provider, registry, None);
    assert!(
        !agent.has_concurrency_tracking(),
        "a viewer placeholder is not a live logical session"
    );
    agent.activate_concurrency_tracking();
    assert!(
        agent.has_concurrency_tracking(),
        "idle committed sessions must count before their first turn"
    );
    let first_guard = format!("{:?}", agent.concurrency_session);
    agent.activate_concurrency_tracking();
    assert_eq!(
        format!("{:?}", agent.concurrency_session),
        first_guard,
        "repeated subscribe must not create another incarnation"
    );
    agent.mark_closed();
    assert!(!agent.has_concurrency_tracking());
}

#[tokio::test]
async fn headless_parent_is_preserved_without_concurrency_collection() {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedTelemetryEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let child = Agent::new_with_parent_and_initial_working_dir(
        provider.clone(),
        registry,
        None,
        Some("coordinator-session".to_owned()),
        None,
        None,
    );
    assert_eq!(
        child.session.parent_id.as_deref(),
        Some("coordinator-session")
    );
    assert!(!child.concurrency_session.as_ref().unwrap().is_active());
    let registry = Registry::new(provider.clone()).await;
    let root =
        Agent::new_with_parent_and_initial_working_dir(provider, registry, None, None, None, None);
    assert!(root.session.parent_id.is_none());
    assert!(!root.concurrency_session.as_ref().unwrap().is_active());
}

#[tokio::test]
async fn spawn_construction_persists_the_declared_tool_allowlist() {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedTelemetryEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;

    let declared = vec!["read".to_string(), "ls".to_string()];
    let worker = Agent::new_with_parent_and_initial_working_dir(
        provider.clone(),
        registry,
        None,
        Some("coordinator-session".to_owned()),
        Some(declared.as_slice()),
        None,
    );

    assert_eq!(
        worker.session.spawn_allowed_tools.as_deref(),
        Some(declared.as_slice()),
        "the declared allowlist must be persisted with the session"
    );

    // No allowlist means no restriction to restore, and it stays unset.
    let registry = Registry::new(provider.clone()).await;
    let unrestricted =
        Agent::new_with_parent_and_initial_working_dir(provider, registry, None, None, None, None);
    assert!(unrestricted.session.spawn_allowed_tools.is_none());
}

#[test]
fn restored_allowlist_is_narrowed_against_the_current_config() {
    let _lock = crate::storage::lock_test_env();
    let mut session = crate::session::Session::create(None, None);

    assert!(
        crate::server::restored_spawn_allowed_tools(&session).is_none(),
        "a session without a spawn allowlist keeps the configured selection"
    );

    // An empty declared allowlist stays an empty restriction: config can never
    // re-widen a worker that was admitted with no tools.
    session.spawn_allowed_tools = Some(Vec::new());
    assert_eq!(
        crate::server::restored_spawn_allowed_tools(&session),
        Some(std::collections::HashSet::new())
    );

    // A declared name is preserved only where the current config still permits
    // it, so config tightening also applies to restored workers.
    session.spawn_allowed_tools = Some(vec!["read".to_string()]);
    let restored = crate::server::restored_spawn_allowed_tools(&session)
        .expect("declared allowlist yields a restriction");
    assert!(
        restored
            .iter()
            .all(|name| name == "read" || name.is_empty()),
        "{restored:?}"
    );
}
