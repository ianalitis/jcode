use futures::stream;

struct DefaultEnvelopeProvider;

#[async_trait]
impl Provider for DefaultEnvelopeProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        Ok(Box::pin(stream::empty()))
    }

    fn name(&self) -> &str {
        "default-envelope-test"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self)
    }
}

#[tokio::test]
async fn provider_default_rejects_a_metered_budget_instead_of_ignoring_it() {
    let envelope = SpawnExecutionEnvelope::new(
        Some(100),
        Some(30),
        Some(jcode_attempt_types::DataClass::Public),
        None,
    );

    let result = DefaultEnvelopeProvider
        .complete_with_spawn_envelope(&[], &[], "", None, &envelope)
        .await;
    let error = match result {
        Ok(_) => panic!("providers must opt into metered budget enforcement"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("does not implement metered spawn reservation and settlement")
    );
}

#[tokio::test]
async fn provider_default_rejects_a_zero_spawn_deadline() {
    let envelope = SpawnExecutionEnvelope::new(
        None,
        Some(0),
        Some(jcode_attempt_types::DataClass::Private),
        None,
    );

    let result = DefaultEnvelopeProvider
        .complete_with_spawn_envelope(&[], &[], "", None, &envelope)
        .await;
    let error = match result {
        Ok(_) => panic!("zero deadlines must fail closed"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("deadline_secs must be greater than zero")
    );
}

#[tokio::test]
async fn spawn_deadline_is_whole_child_and_does_not_reset_per_provider_call() {
    let envelope = SpawnExecutionEnvelope::new(
        None,
        Some(1),
        Some(jcode_attempt_types::DataClass::Private),
        None,
    );
    let _stream = DefaultEnvelopeProvider
        .complete_with_spawn_envelope(&[], &[], "", None, &envelope)
        .await
        .expect("first call is within the child deadline");
    tokio::time::sleep(Duration::from_millis(1_100)).await;

    let result = DefaultEnvelopeProvider
        .complete_with_spawn_envelope(&[], &[], "", None, &envelope)
        .await;

    assert!(result.is_err(), "expired child deadline must not reset");
}

#[tokio::test]
async fn shared_no_budget_contract_accepts_an_unbilled_spawn() {
    // Included-subscription and local routes carry a spawn envelope for the
    // deadline but no per-token budget. The shared contract must serve them
    // instead of demanding a metered reservation, which app-core never supplies
    // for a route with nothing to cap.
    let envelope = SpawnExecutionEnvelope::new(
        None,
        Some(30),
        Some(jcode_attempt_types::DataClass::Private),
        None,
    );

    let stream = crate::complete_with_spawn_envelope_without_budget(
        &DefaultEnvelopeProvider,
        &[],
        &[],
        "",
        None,
        &envelope,
    )
    .await
    .expect("an unbilled spawn needs no metered budget");

    drop(stream);
}
