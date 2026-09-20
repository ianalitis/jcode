#[tokio::test]
async fn auth_model_first_prompt_e2e_state_space_is_bounded_by_selection_source() {
    let scenarios = [
        AuthModelE2eScenario {
            name: "auth auto-selects matching route when user does not intervene",
            manual_pick_after_first_snapshot: None,
            prompt_immediately_after_model_pick: false,
            expected_first_prompt_model: "logged-in-model",
        },
        AuthModelE2eScenario {
            name: "manual picker selection during auth refresh wins first prompt",
            manual_pick_after_first_snapshot: Some("user-picked-model"),
            prompt_immediately_after_model_pick: true,
            expected_first_prompt_model: "user-picked-model",
        },
    ];

    for scenario in scenarios {
        let _guard = EnvGuard::save(&[
            "JCODE_OPENROUTER_API_BASE",
            "JCODE_OPENROUTER_API_KEY_NAME",
            "JCODE_OPENROUTER_ENV_FILE",
            "JCODE_OPENROUTER_CACHE_NAMESPACE",
            "JCODE_OPENROUTER_PROVIDER_FEATURES",
            "JCODE_OPENROUTER_TRANSPORT_STATE",
            "JCODE_OPENROUTER_MODEL_CATALOG",
            "JCODE_OPENROUTER_AUTH_HEADER",
            "JCODE_OPENROUTER_DYNAMIC_BEARER_PROVIDER",
            "JCODE_OPENROUTER_MODEL",
            "JCODE_RUNTIME_PROVIDER",
            "JCODE_ACTIVE_PROVIDER",
            "JCODE_INITIAL_PROVIDER_EXPLICIT",
        ]);

        crate::bus::reset_models_updated_publish_state_for_tests();
        let provider_concrete = Arc::new(AuthChangeMockProvider::new());
        provider_concrete
            .state
            .auth_refresh_delay_ms
            .store(80, Ordering::Release);
        *provider_concrete.state.selected_model.write().unwrap() = Some("stale-model".to_string());
        *provider_concrete.state.route_provider.write().unwrap() = "Cerebras".to_string();
        *provider_concrete.state.route_api_method.write().unwrap() =
            "openai-compatible:cerebras".to_string();
        *provider_concrete
            .state
            .expose_selected_model_in_routes
            .write()
            .unwrap() = false;
        let provider: Arc<dyn Provider> = provider_concrete.clone();
        let registry = Registry::empty();
        let agent = Arc::new(Mutex::new(Agent::new(provider.clone(), registry)));
        let session_id = { agent.lock().await.session_id().to_string() };
        let sessions: SessionAgents = Arc::new(RwLock::new(HashMap::from([(
            format!("test-session-{}", scenario.name),
            Arc::clone(&agent),
        )])));
        let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

        let mut auth = crate::protocol::AuthChanged::new("cerebras");
        auth.credential_source = Some(crate::protocol::AuthCredentialSource::ApiKeyFile);
        auth.auth_method = Some(crate::protocol::AuthMethod::RemoteTuiPasteApiKey);
        auth.expected_runtime = Some(crate::protocol::RuntimeProviderKey::new(
            "openai-compatible",
        ));
        auth.expected_catalog_namespace = Some(crate::protocol::CatalogNamespace::new("cerebras"));

        handle_notify_auth_changed(
            148,
            None,
            Some(auth),
            false,
            &provider,
            &provider,
            &sessions,
            session_id.as_str(),
            &agent,
            &client_event_tx,
        )
        .await;

        assert!(
            matches!(
                client_event_rx.recv().await,
                Some(ServerEvent::Done { id: 148 })
            ),
            "{}: expected auth Done",
            scenario.name
        );

        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if matches!(
                    client_event_rx.recv().await,
                    Some(ServerEvent::AvailableModelsUpdated { .. })
                ) {
                    break;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{}: expected immediate auth model snapshot", scenario.name));

        let mut first_prompt_output = None;
        if let Some(model) = scenario.manual_pick_after_first_snapshot {
            handle_set_model(248, model.to_string(), &agent, &client_event_tx).await;
            loop {
                match client_event_rx.recv().await {
                    Some(ServerEvent::ModelChanged {
                        id: 248,
                        model: changed,
                        error,
                        ..
                    }) => {
                        assert_eq!(error, None, "{}: manual model switch failed", scenario.name);
                        assert_eq!(
                            changed, model,
                            "{}: manual model switch mismatch",
                            scenario.name
                        );
                        break;
                    }
                    Some(_) => continue,
                    None => panic!("{}: model switch channel closed", scenario.name),
                }
            }

            if scenario.prompt_immediately_after_model_pick {
                let agent_for_prompt = Arc::clone(&agent);
                let scenario_name = scenario.name;
                first_prompt_output = Some(
                    tokio::time::timeout(std::time::Duration::from_secs(2), async move {
                        let mut agent_guard = agent_for_prompt.lock().await;
                        agent_guard
                            .run_once_capture("first prompt immediately after model selection")
                            .await
                    })
                    .await
                    .unwrap_or_else(|_| panic!("{}: first prompt stalled", scenario_name))
                    .unwrap_or_else(|error| {
                        panic!("{}: first prompt failed: {error:?}", scenario_name)
                    }),
                );
            }
        }

        let final_message = recv_final_catalog_notification(&mut client_event_rx).await;
        assert!(
            final_message.contains(&format!(
                "**Model ready:** `{}`",
                scenario.expected_first_prompt_model
            )),
            "{}: final activity selected wrong model: {}",
            scenario.name,
            final_message
        );

        let first_prompt_output = if let Some(output) = first_prompt_output {
            output
        } else {
            let mut agent_guard = agent.lock().await;
            agent_guard
                .run_once_capture("first prompt after auth/model selection")
                .await
                .unwrap_or_else(|error| panic!("{}: first prompt failed: {error:?}", scenario.name))
        };
        assert!(
            first_prompt_output.contains("ok"),
            "{}: fake provider response not observed: {}",
            scenario.name,
            first_prompt_output
        );
        let completed_models = provider_concrete
            .state
            .complete_models
            .lock()
            .unwrap()
            .clone();
        assert_eq!(
            completed_models.last().map(String::as_str),
            Some(scenario.expected_first_prompt_model),
            "{}: first provider request used wrong model; all completions: {:?}",
            scenario.name,
            completed_models
        );
    }
}

#[tokio::test]
async fn notify_auth_changed_switches_only_current_session_model() {
    let _guard = EnvGuard::save(&[
        "JCODE_OPENROUTER_API_BASE",
        "JCODE_OPENROUTER_API_KEY_NAME",
        "JCODE_OPENROUTER_ENV_FILE",
        "JCODE_OPENROUTER_CACHE_NAMESPACE",
        "JCODE_OPENROUTER_PROVIDER_FEATURES",
        "JCODE_OPENROUTER_TRANSPORT_STATE",
        "JCODE_OPENROUTER_MODEL_CATALOG",
        "JCODE_OPENROUTER_AUTH_HEADER",
        "JCODE_OPENROUTER_DYNAMIC_BEARER_PROVIDER",
        "JCODE_OPENROUTER_MODEL",
        "JCODE_RUNTIME_PROVIDER",
        "JCODE_ACTIVE_PROVIDER",
        "JCODE_INITIAL_PROVIDER_EXPLICIT",
    ]);

    crate::bus::reset_models_updated_publish_state_for_tests();
    let current_provider = Arc::new(AuthChangeMockProvider::new());
    let current_state = Arc::clone(&current_provider.state);
    *current_state.selected_model.write().unwrap() = Some("gpt-5.5".to_string());
    *current_state.route_provider.write().unwrap() = "Groq".to_string();
    *current_state.route_api_method.write().unwrap() = "openai-compatible:groq".to_string();
    let peer_provider = Arc::new(AuthChangeMockProvider::new());
    let peer_state = Arc::clone(&peer_provider.state);
    *peer_state.selected_model.write().unwrap() = Some("gpt-5.5".to_string());
    *peer_state.route_provider.write().unwrap() = "Groq".to_string();
    *peer_state.route_api_method.write().unwrap() = "openai-compatible:groq".to_string();

    let current_provider: Arc<dyn Provider> = current_provider;
    let peer_provider: Arc<dyn Provider> = peer_provider;
    let registry = Registry::empty();
    let current_agent = Arc::new(Mutex::new(Agent::new(
        Arc::clone(&current_provider),
        registry.clone(),
    )));
    let current_session_id = { current_agent.lock().await.session_id().to_string() };
    let peer_agent = Arc::new(Mutex::new(Agent::new(peer_provider, registry)));
    let sessions: SessionAgents = Arc::new(RwLock::new(HashMap::from([
        ("current-session".to_string(), Arc::clone(&current_agent)),
        ("peer-session".to_string(), Arc::clone(&peer_agent)),
    ])));
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    let mut auth = crate::protocol::AuthChanged::new("groq");
    auth.credential_source = Some(crate::protocol::AuthCredentialSource::ApiKeyFile);
    auth.auth_method = Some(crate::protocol::AuthMethod::RemoteTuiPasteApiKey);
    auth.expected_runtime = Some(crate::protocol::RuntimeProviderKey::new(
        "openai-compatible",
    ));
    auth.expected_catalog_namespace = Some(crate::protocol::CatalogNamespace::new("groq"));

    handle_notify_auth_changed(
        47,
        Some("openai".to_string()),
        Some(auth),
        false,
        &current_provider,
        &current_provider,
        &sessions,
        current_session_id.as_str(),
        &current_agent,
        &client_event_tx,
    )
    .await;

    assert!(matches!(
        client_event_rx.recv().await,
        Some(ServerEvent::Done { id: 47 })
    ));

    let expected = "llama-3.1-8b-instant";
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let current = current_state.selected_model.read().unwrap().clone();
        let peer = peer_state.selected_model.read().unwrap().clone();
        let peer_refreshed = *peer_state.logged_in.read().unwrap();
        if current.as_deref() == Some(expected)
            && peer.as_deref() == Some("gpt-5.5")
            && peer_refreshed
        {
            let peer_snapshot = available_models_updated_event(&peer_agent).await;
            let ServerEvent::AvailableModelsUpdated {
                provider_name,
                provider_model,
                available_model_routes,
                ..
            } = peer_snapshot
            else {
                panic!("expected available models snapshot for peer session");
            };
            assert_eq!(provider_name.as_deref(), Some("mock-auth"));
            assert_eq!(provider_model.as_deref(), Some("gpt-5.5"));
            assert!(available_model_routes.iter().any(|route| {
                route.model == "gpt-5.5"
                    && route.provider == "Groq"
                    && route.api_method == "openai-compatible:groq"
            }));
            assert!(
                available_model_routes
                    .iter()
                    .all(|route| route.model != expected),
                "auth-triggered Groq model leaked into peer session routes: {:?}",
                available_model_routes
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    panic!(
        "auth change did not keep model switch session-local: current={:?}, peer={:?}, peer_refreshed={}",
        current_state.selected_model.read().unwrap().clone(),
        peer_state.selected_model.read().unwrap().clone(),
        *peer_state.logged_in.read().unwrap()
    );
}

#[tokio::test]
async fn refresh_models_emits_available_models_updated_after_prefetch() {
    crate::bus::reset_models_updated_publish_state_for_tests();
    let provider: Arc<dyn Provider> = Arc::new(AuthChangeMockProvider::new());
    let registry = Registry::empty();
    let agent = Arc::new(Mutex::new(Agent::new(provider.clone(), registry)));
    let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel();

    handle_refresh_models(7, &provider, &agent, &client_event_tx).await;

    let mut saw_done = false;
    let mut saw_models = None;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let event = tokio::time::timeout(remaining, client_event_rx.recv())
            .await
            .expect("receive server event before timeout");
        match event.expect("channel open") {
            ServerEvent::Done { id } => {
                assert_eq!(id, 7);
                saw_done = true;
            }
            ServerEvent::AvailableModelsUpdated {
                provider_name,
                provider_model,
                available_models,
                available_model_routes,
            } => {
                saw_models = Some((
                    provider_name,
                    provider_model,
                    available_models,
                    available_model_routes,
                ));
                break;
            }
            _ => {}
        }
    }

    assert!(saw_done, "expected immediate Done ack");
    let (provider_name, provider_model, available_models, available_model_routes) =
        saw_models.expect("expected AvailableModelsUpdated event");
    assert_eq!(provider_name.as_deref(), Some("mock-auth"));
    assert_eq!(provider_model.as_deref(), Some("logged-out-model"));
    assert_eq!(available_models, vec!["logged-out-model".to_string()]);
    assert!(available_model_routes.iter().any(|route| {
        route.model == "logged-out-model"
            && route.provider == "MockAuth"
            && route.api_method == "mock-auth"
    }));
}
