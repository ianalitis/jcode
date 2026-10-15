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
async fn headless_parent_is_set_before_concurrency_tracking_begins() {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedTelemetryEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let child = Agent::new_with_parent_and_initial_working_dir(
        provider.clone(),
        registry,
        None,
        Some("coordinator-session".to_owned()),
    );
    assert_eq!(
        child.session.parent_id.as_deref(),
        Some("coordinator-session")
    );
    assert!(format!("{:?}", child.concurrency_session).contains("child: true"));
    let registry = Registry::new(provider.clone()).await;
    let root = Agent::new_with_parent_and_initial_working_dir(provider, registry, None, None);
    assert!(root.session.parent_id.is_none());
    assert!(format!("{:?}", root.concurrency_session).contains("child: false"));
}

#[tokio::test]
async fn created_session_preserves_identity_and_uses_create_initialization() {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedTelemetryEnv::new();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut session = Session::create(Some("coordinator-session".to_owned()), None);
    let id = session.id.clone();
    let working_dir = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    session.working_dir = Some(working_dir.clone());
    session.model = Some("not-a-model-to-restore".to_owned());

    let agent = Agent::new_with_created_session(provider, registry, session, true);

    assert_eq!(agent.session.id, id);
    assert_eq!(
        agent.session.parent_id.as_deref(),
        Some("coordinator-session")
    );
    assert_eq!(
        agent.session.working_dir.as_deref(),
        Some(working_dir.as_str())
    );
    assert_eq!(
        agent.session.model.as_deref(),
        Some(agent.provider_model().as_str())
    );
    assert!(
        !agent.session.messages.is_empty(),
        "create seeds initial session context"
    );
    assert!(format!("{:?}", agent.concurrency_session).contains("child: true"));
}
