use super::*;
use std::sync::Mutex;

#[derive(Clone)]
struct MutableModelProvider {
    model: Arc<Mutex<String>>,
}

#[async_trait]
impl Provider for MutableModelProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        system: &str,
        resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        TestProvider
            .complete(messages, tools, system, resume_session_id)
            .await
    }

    fn name(&self) -> &str {
        "mutable-model-test"
    }

    fn model(&self) -> String {
        self.model.lock().unwrap().clone()
    }

    fn set_model(&self, model: &str) -> Result<()> {
        *self.model.lock().unwrap() = model.to_string();
        Ok(())
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

#[tokio::test]
async fn restore_agent_session_if_requested_reapplies_explicit_model() {
    let _guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME"]);
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());

    let original_provider: Arc<dyn Provider> = Arc::new(MutableModelProvider {
        model: Arc::new(Mutex::new("session-model".to_string())),
    });
    let registry = Registry::new(original_provider.clone()).await;
    let mut original = crate::agent::Agent::new(original_provider, registry);
    original
        .set_model("session-model")
        .expect("set session model");
    original
        .run_once_capture("seed explicit resume model test")
        .await
        .expect("seed session");
    let session_id = original.session_id().to_string();
    original.mark_closed();

    let resumed_provider: Arc<dyn Provider> = Arc::new(MutableModelProvider {
        model: Arc::new(Mutex::new("cli-model".to_string())),
    });
    let registry = Registry::new(resumed_provider.clone()).await;
    let mut resumed = crate::agent::Agent::new(resumed_provider.clone(), registry);

    restore_agent_session_if_requested(&mut resumed, Some(&session_id), Some("cli-model"))
        .expect("restore with explicit model");

    assert_eq!(resumed_provider.model(), "cli-model");
    let persisted = crate::session::Session::load(&session_id).expect("load resumed session");
    assert_eq!(persisted.model.as_deref(), Some("cli-model"));
}
