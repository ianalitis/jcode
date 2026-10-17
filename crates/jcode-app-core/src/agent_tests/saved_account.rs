use super::*;

struct BoundSavedAccountProvider;

#[tokio::test]
async fn user_response_capability_updates_locked_tool_surface() {
    let mut agent = Agent::new(Arc::new(BoundSavedAccountProvider), Registry::empty());
    assert!(
        !agent
            .tool_definitions()
            .await
            .iter()
            .any(|d| d.name == "user_response")
    );
    assert!(agent.locked_tools.is_some());
    agent.set_desktop_response_capability(true).await;
    assert!(agent.locked_tools.is_none());
    assert!(
        agent
            .tool_definitions()
            .await
            .iter()
            .any(|d| d.name == "user_response")
    );
    agent.set_desktop_response_capability(true).await;
    assert!(
        agent.locked_tools.is_some(),
        "unchanged capability preserves the cache"
    );
    agent.set_desktop_response_capability(false).await;
    assert!(agent.locked_tools.is_none());
    assert!(
        !agent
            .tool_definitions()
            .await
            .iter()
            .any(|d| d.name == "user_response")
    );
}

#[async_trait]
impl Provider for BoundSavedAccountProvider {
    async fn complete(
        &self,
        _: &[Message],
        _: &[ToolDefinition],
        _: &str,
        _: Option<&str>,
    ) -> Result<EventStream> {
        panic!("metadata test must not send a request")
    }
    fn name(&self) -> &str {
        "OpenAI"
    }
    fn oauth_account_label(&self) -> Option<String> {
        Some("work".to_string())
    }
    fn active_resolved_credential(&self) -> Option<jcode_provider_core::ResolvedCredential> {
        Some(jcode_provider_core::ResolvedCredential::Oauth)
    }
    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self)
    }
}

#[tokio::test]
async fn saved_account_observed_binding_is_persisted() {
    let _guard = crate::storage::lock_test_env();
    let home = tempfile::tempdir().unwrap();
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            match &self.0 {
                Some(home) => crate::env::set_var("JCODE_HOME", home),
                None => crate::env::remove_var("JCODE_HOME"),
            }
            crate::config::Config::invalidate_cache();
        }
    }
    let _restore = RestoreHome(std::env::var_os("JCODE_HOME"));
    crate::env::set_var("JCODE_HOME", home.path());
    crate::config::Config::invalidate_cache();
    let provider: Arc<dyn Provider> = Arc::new(BoundSavedAccountProvider);
    let mut agent = Agent::new(provider, Registry::empty());
    agent.session.route_api_method = None;
    agent.session.save_prepared().unwrap();
    agent.sync_bound_account_route();
    assert_eq!(
        agent.session.route_api_method.as_deref(),
        Some("openai-oauth@work")
    );
    let saved = crate::session::Session::load(&agent.session.id).unwrap();
    assert_eq!(saved.route_api_method.as_deref(), Some("openai-oauth@work"));
    assert_eq!(saved.provider_key.as_deref(), Some("openai-oauth"));
    agent.sync_bound_account_route();
    assert_eq!(agent.session.route_api_method, saved.route_api_method);
    crate::provider::clear_openai_provider_unavailability_for_account_label(Some("work"));
    crate::provider::clear_openai_provider_unavailability_for_account_label(Some("personal"));
    agent.record_failed_account_route("429 rate limit reached");
    assert!(
        crate::provider::provider_unavailability_detail_for_account_label("openai", Some("work"))
            .is_some()
    );
    assert!(
        crate::provider::provider_unavailability_detail_for_account_label(
            "openai",
            Some("personal")
        )
        .is_none()
    );
    crate::provider::clear_openai_provider_unavailability_for_account_label(Some("work"));
    agent.session.route_api_method = Some("openai-oauth@personal".to_string());
    agent.sync_bound_account_route();
    assert_eq!(
        agent.session.route_api_method.as_deref(),
        Some("openai-oauth@personal")
    );
    assert!(
        agent
            .ensure_session_lease()
            .unwrap_err()
            .to_string()
            .contains("personal")
    );
}

#[tokio::test]
async fn saved_account_restore_mismatch_blocks_model_turn() {
    let _guard = crate::storage::lock_test_env();
    let home = tempfile::tempdir().unwrap();
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            match &self.0 {
                Some(home) => crate::env::set_var("JCODE_HOME", home),
                None => crate::env::remove_var("JCODE_HOME"),
            }
            crate::config::Config::invalidate_cache();
        }
    }
    let _restore = RestoreHome(std::env::var_os("JCODE_HOME"));
    crate::env::set_var("JCODE_HOME", home.path());
    crate::config::Config::invalidate_cache();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let mut agent = Agent::new(provider, Registry::empty());
    agent.session.route_api_method = Some("openai-oauth@work".into());
    let error = agent.ensure_session_lease().unwrap_err().to_string();
    assert!(error.contains("Saved account 'work' could not be restored"));
    agent.session.route_api_method = None;
    agent.session.provider_key = Some("openai-oauth@personal".into());
    assert!(
        agent
            .ensure_session_lease()
            .unwrap_err()
            .to_string()
            .contains("personal")
    );
    agent.session.provider_key = Some("openai-oauth".into());
    assert!(agent.ensure_session_lease().is_ok());
}
