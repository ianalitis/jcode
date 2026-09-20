#[tokio::test(flavor = "current_thread")]
async fn opencode_zen_live_opt_in_tool_call_smoke() {
    if !env_truthy("JCODE_OPENCODE_ZEN_LIVE_TOOL_TEST") {
        eprintln!(
            "skipping live OpenCode Zen tool-call smoke; set JCODE_OPENCODE_ZEN_LIVE_TOOL_TEST=1 and provide OPENCODE_API_KEY"
        );
        return;
    }
    let api_key = live_opencode_zen_api_key().expect(
            "JCODE_OPENCODE_ZEN_LIVE_TOOL_TEST=1 requires OPENCODE_API_KEY or JCODE_AUTH_LIFECYCLE_OPENCODE_API_KEY",
        );
    let model = std::env::var("JCODE_OPENCODE_ZEN_LIVE_TOOL_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "kimi-k2.6".to_string());

    let stream_smoke = env_truthy("JCODE_OPENCODE_ZEN_LIVE_STREAM_TEST")
        || env_truthy("JCODE_AUTH_LIFECYCLE_STREAM_SMOKE");
    let mut stages = vec![
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::AUTH_CREDENTIAL_LOADED,
        )
        .with_evidence(
            "auth_source",
            serde_json::json!(api_key.auth.source.clone()),
        )
        .with_evidence("env_key", serde_json::json!(api_key.auth.env_key.clone())),
        cost_quota_safety_stage(true),
    ];
    let models_result = fetch_live_openai_compatible_models(
        jcode_base::provider_catalog::OPENCODE_PROFILE,
        &api_key.secret,
    )
    .await;
    let models = match models_result {
        Ok(models) => models,
        Err(error) => {
            stages.push(jcode_base::live_tests::LiveVerificationStage::failed(
                jcode_base::live_tests::checkpoints::MODEL_CATALOG_LIVE_ENDPOINT,
                error.to_string(),
            ));
            let capabilities = covered_stage_names(&stages);
            let event = live_event(
                "opencode_zen_live_opt_in_tool_call_smoke",
                jcode_base::provider_catalog::OPENCODE_PROFILE,
                api_key.auth.clone(),
                Some(&model),
                capabilities,
                jcode_base::live_tests::LiveVerificationResult::Failed,
            )
            .with_stages(stages);
            append_live_event(&event);
            panic!("live OpenCode Zen model catalog: {error:?}");
        }
    };
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::MODEL_CATALOG_LIVE_ENDPOINT,
        )
        .with_evidence(
            "models",
            jcode_base::live_tests::concise_model_sample(&models, 16),
        )
        .with_evidence(
            "requested_model_present",
            serde_json::json!(models.contains(&model)),
        ),
    );
    assert!(
        models.iter().any(|candidate| candidate == &model),
        "live OpenCode Zen catalog did not include requested test model `{model}`"
    );

    let driver = AuthLifecycleDriver::new().expect("driver");
    let mut spec = AuthLifecycleSpec::openai_compatible_fixture(
        jcode_base::provider_catalog::OPENCODE_PROFILE,
        AuthLifecycleAuthPath::RemoteTuiPasteApiKey,
    );
    spec.api_key = api_key.secret.clone();
    spec.catalog_models_after_auth = models.clone();
    spec.selected_model_override = Some(model.clone());
    let result = driver
        .run_openai_compatible_fixture(&spec)
        .expect("live OpenCode Zen auth lifecycle fixture");
    result.assert_success(&spec);
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::AUTH_UX_KEY_ENTRY,
        )
        .with_evidence("auth_path", serde_json::json!("remote_tui_paste_api_key"))
        .with_evidence("simulated_in_sandbox", serde_json::json!(true))
        .with_evidence("transcript_order_verified", serde_json::json!(true)),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::CREDENTIAL_PERSISTENCE,
        )
        .with_evidence(
            "credential_location",
            serde_json::json!(result.credential_location.clone()),
        )
        .with_evidence("sandboxed", serde_json::json!(true)),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::CATALOG_HOT_RELOAD_CURRENT_SESSION,
        )
        .with_evidence("transcript_markers_verified", serde_json::json!(true))
        .with_evidence(
            "catalog_route_count",
            serde_json::json!(result.catalog_routes.len()),
        ),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::PICKER_LIVE_MODELS,
        )
        .with_evidence("selected_model", serde_json::json!(model.clone()))
        .with_evidence(
            "picker_entries",
            serde_json::json!(result.picker.provider_entries.clone()),
        ),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::PICKER_FALLBACK_LABELING,
        )
        .with_evidence("fallback_routes_present", serde_json::json!(false))
        .with_evidence(
            "all_picker_entries_from_live_catalog",
            serde_json::json!(true),
        ),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::MODEL_SWITCH_ROUTE,
        )
        .with_evidence(
            "switch_request",
            serde_json::json!(result.picker.switch_request.clone()),
        )
        .with_evidence(
            "switch_route_api_method",
            serde_json::json!(result.picker.switch_route_api_method.clone()),
        ),
    );

    if stream_smoke {
        match run_live_openai_compatible_stream_smoke(
            jcode_base::provider_catalog::OPENCODE_PROFILE,
            &api_key.secret,
            &model,
        )
        .await
        {
            Ok(stage) => stages.push(stage),
            Err(error) => {
                stages.push(jcode_base::live_tests::LiveVerificationStage::failed(
                    jcode_base::live_tests::checkpoints::STREAMING_CHAT_COMPLETION,
                    error.to_string(),
                ));
                let capabilities = covered_stage_names(&stages);
                let event = live_event(
                    "opencode_zen_live_opt_in_tool_call_smoke",
                    jcode_base::provider_catalog::OPENCODE_PROFILE,
                    api_key.auth.clone(),
                    Some(&model),
                    capabilities,
                    jcode_base::live_tests::LiveVerificationResult::Failed,
                )
                .with_stages(stages);
                append_live_event(&event);
                panic!("live OpenCode Zen stream smoke: {error:?}");
            }
        }
    }

    match run_live_openai_compatible_tool_smoke(
        jcode_base::provider_catalog::OPENCODE_PROFILE,
        &api_key.secret,
        &model,
    )
    .await
    {
        Ok(stage) => stages.push(stage),
        Err(error) => {
            stages.push(jcode_base::live_tests::LiveVerificationStage::failed(
                jcode_base::live_tests::checkpoints::TOOL_CALL_PARSE,
                error.to_string(),
            ));
            let capabilities = covered_stage_names(&stages);
            let event = live_event(
                "opencode_zen_live_opt_in_tool_call_smoke",
                jcode_base::provider_catalog::OPENCODE_PROFILE,
                api_key.auth.clone(),
                Some(&model),
                capabilities,
                jcode_base::live_tests::LiveVerificationResult::Failed,
            )
            .with_stages(stages);
            append_live_event(&event);
            panic!("live OpenCode Zen tool-call smoke: {error:?}");
        }
    }

    let capabilities = covered_stage_names(&stages);
    let event = live_event(
        "opencode_zen_live_opt_in_tool_call_smoke",
        jcode_base::provider_catalog::OPENCODE_PROFILE,
        api_key.auth.clone(),
        Some(&model),
        capabilities,
        jcode_base::live_tests::LiveVerificationResult::Passed,
    )
    .with_stages(stages)
    .with_metadata("default_model", serde_json::json!("kimi-k2.6"));
    append_live_event(&event);
}

#[tokio::test(flavor = "current_thread")]
async fn issue_driven_openai_compatible_live_target_smoke() {
    let Some(provider_id) = std::env::var("JCODE_ISSUE_DRIVEN_LIVE_PROVIDER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        eprintln!(
            "skipping issue-driven live provider test; set JCODE_ISSUE_DRIVEN_LIVE_PROVIDER to an OpenAI-compatible profile id"
        );
        return;
    };
    let profile = jcode_base::provider_catalog::openai_compatible_profile_by_id(&provider_id)
        .unwrap_or_else(|| panic!("unknown OpenAI-compatible profile id: {provider_id}"));
    let resolved = jcode_base::provider_catalog::resolve_openai_compatible_profile(profile);
    let api_key = live_openai_compatible_api_key(profile).unwrap_or_else(|| {
        panic!(
            "JCODE_ISSUE_DRIVEN_LIVE_PROVIDER={} requires {} or {}",
            provider_id, resolved.api_key_env, resolved.env_file
        )
    });
    let requested_model = std::env::var("JCODE_ISSUE_DRIVEN_LIVE_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| resolved.default_model.clone());
    let spend_smoke = env_truthy("JCODE_ISSUE_DRIVEN_LIVE_COMPLETION_SMOKE");
    let stream_smoke = env_truthy("JCODE_ISSUE_DRIVEN_LIVE_STREAM_SMOKE");

    let mut stages = vec![
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::AUTH_CREDENTIAL_LOADED,
        )
        .with_evidence(
            "auth_source",
            serde_json::json!(api_key.auth.source.clone()),
        )
        .with_evidence("env_key", serde_json::json!(api_key.auth.env_key.clone())),
        cost_quota_safety_stage(spend_smoke || stream_smoke),
    ];

    let models_result = fetch_live_openai_compatible_models(profile, &api_key.secret).await;
    let models = match models_result {
        Ok(models) => models,
        Err(error) => {
            stages.push(jcode_base::live_tests::LiveVerificationStage::failed(
                jcode_base::live_tests::checkpoints::MODEL_CATALOG_LIVE_ENDPOINT,
                error.to_string(),
            ));
            let capabilities = covered_stage_names(&stages);
            let event = live_event(
                "issue_driven_openai_compatible_live_target_smoke",
                profile,
                api_key.auth.clone(),
                requested_model.as_deref(),
                capabilities,
                jcode_base::live_tests::LiveVerificationResult::Failed,
            )
            .with_stages(stages);
            append_live_event(&event);
            panic!("live {} model catalog: {error:?}", resolved.display_name);
        }
    };
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::MODEL_CATALOG_LIVE_ENDPOINT,
        )
        .with_evidence(
            "models",
            jcode_base::live_tests::concise_model_sample(&models, 16),
        ),
    );

    let selected = requested_model
        .filter(|requested| models.iter().any(|model| model == requested))
        .or_else(|| models.first().cloned())
        .expect("live catalog should have at least one model");
    assert!(
        models.iter().any(|model| model == &selected),
        "live {} catalog did not include selected issue target model `{selected}`",
        resolved.display_name
    );

    let driver = AuthLifecycleDriver::new().expect("driver");
    let mut spec = AuthLifecycleSpec::openai_compatible_fixture(
        profile,
        AuthLifecycleAuthPath::RemoteTuiPasteApiKey,
    );
    spec.api_key = api_key.secret.clone();
    spec.catalog_models_after_auth = models.clone();
    spec.selected_model_override = Some(selected.clone());
    let result = driver
        .run_openai_compatible_fixture(&spec)
        .expect("issue-driven live auth lifecycle fixture");
    result.assert_success(&spec);

    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::AUTH_UX_KEY_ENTRY,
        )
        .with_evidence("auth_path", serde_json::json!("remote_tui_paste_api_key"))
        .with_evidence("simulated_in_sandbox", serde_json::json!(true))
        .with_evidence("transcript_order_verified", serde_json::json!(true)),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::CREDENTIAL_PERSISTENCE,
        )
        .with_evidence(
            "credential_location",
            serde_json::json!(result.credential_location.clone()),
        )
        .with_evidence("sandboxed", serde_json::json!(true)),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::CATALOG_HOT_RELOAD_CURRENT_SESSION,
        )
        .with_evidence("transcript_markers_verified", serde_json::json!(true))
        .with_evidence(
            "catalog_route_count",
            serde_json::json!(result.catalog_routes.len()),
        ),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::PICKER_LIVE_MODELS,
        )
        .with_evidence("selected_model", serde_json::json!(selected.clone()))
        .with_evidence(
            "picker_entries",
            serde_json::json!(result.picker.provider_entries.clone()),
        ),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::PICKER_FALLBACK_LABELING,
        )
        .with_evidence("fallback_routes_present", serde_json::json!(false))
        .with_evidence(
            "all_picker_entries_from_live_catalog",
            serde_json::json!(true),
        ),
    );
    stages.push(
        jcode_base::live_tests::LiveVerificationStage::passed(
            jcode_base::live_tests::checkpoints::MODEL_SWITCH_ROUTE,
        )
        .with_evidence(
            "switch_request",
            serde_json::json!(result.picker.switch_request.clone()),
        )
        .with_evidence(
            "switch_route_api_method",
            serde_json::json!(result.picker.switch_route_api_method.clone()),
        ),
    );

    if spend_smoke {
        match run_live_openai_compatible_smoke(profile, &api_key.secret, &selected).await {
            Ok(stage) => stages.push(stage),
            Err(error) => {
                stages.push(jcode_base::live_tests::LiveVerificationStage::failed(
                    jcode_base::live_tests::checkpoints::NON_STREAMING_CHAT_COMPLETION,
                    error.to_string(),
                ));
                let capabilities = covered_stage_names(&stages);
                let event = live_event(
                    "issue_driven_openai_compatible_live_target_smoke",
                    profile,
                    api_key.auth.clone(),
                    Some(&selected),
                    capabilities,
                    jcode_base::live_tests::LiveVerificationResult::Failed,
                )
                .with_stages(stages);
                append_live_event(&event);
                panic!("live {} smoke completion: {error:?}", resolved.display_name);
            }
        }
    }

    if stream_smoke {
        match run_live_openai_compatible_stream_smoke(profile, &api_key.secret, &selected).await {
            Ok(stage) => stages.push(stage),
            Err(error) => {
                stages.push(jcode_base::live_tests::LiveVerificationStage::failed(
                    jcode_base::live_tests::checkpoints::STREAMING_CHAT_COMPLETION,
                    error.to_string(),
                ));
                let capabilities = covered_stage_names(&stages);
                let event = live_event(
                    "issue_driven_openai_compatible_live_target_smoke",
                    profile,
                    api_key.auth.clone(),
                    Some(&selected),
                    capabilities,
                    jcode_base::live_tests::LiveVerificationResult::Failed,
                )
                .with_stages(stages);
                append_live_event(&event);
                panic!("live {} stream smoke: {error:?}", resolved.display_name);
            }
        }
    }

    let capabilities = covered_stage_names(&stages);
    let event = live_event(
        "issue_driven_openai_compatible_live_target_smoke",
        profile,
        api_key.auth.clone(),
        Some(&selected),
        capabilities,
        jcode_base::live_tests::LiveVerificationResult::Passed,
    )
    .with_stages(stages)
    .with_metadata("issue_driven_provider", serde_json::json!(provider_id));
    append_live_event(&event);
}

#[test]
fn fresh_start_sandbox_is_unconfigured_then_tui_key_lifecycle_configures_provider() {
    let driver = AuthLifecycleDriver::new().expect("driver");
    let spec = AuthLifecycleSpec::cerebras_fixture(AuthLifecycleAuthPath::TuiPasteApiKey);
    let resolved = jcode_base::provider_catalog::resolve_openai_compatible_profile(spec.profile);
    let env_file = driver.sandbox.env_file_path(&resolved.env_file);
    let provider = jcode_base::provider_catalog::resolve_login_provider(spec.provider_id)
        .expect("Cerebras login provider descriptor");

    assert!(
        !env_file.exists(),
        "fresh sandbox should not start with a provider env file: {}",
        env_file.display()
    );
    assert_eq!(
        jcode_base::provider_catalog::load_api_key_from_env_or_config(
            &resolved.api_key_env,
            &resolved.env_file,
        ),
        None,
        "fresh sandbox should not inherit credentials from the developer machine"
    );
    assert!(
        !jcode_base::provider_catalog::openai_compatible_profile_is_configured(spec.profile),
        "fresh sandbox should report the provider as unconfigured before setup"
    );
    jcode_base::auth::AuthStatus::invalidate_cache();
    assert_eq!(
        jcode_base::auth::AuthStatus::check_fast().state_for_provider(provider),
        jcode_base::auth::AuthState::NotConfigured
    );

    let result = driver
        .run_openai_compatible_fixture(&spec)
        .expect("fresh-start TUI paste-key lifecycle");

    result.assert_success(&spec);
    assert!(
        env_file.exists(),
        "TUI paste-key lifecycle should create env file"
    );
    assert_eq!(
        jcode_base::provider_catalog::load_api_key_from_env_or_config(
            &resolved.api_key_env,
            &resolved.env_file,
        )
        .as_deref(),
        Some(spec.api_key.as_str())
    );
    assert!(
        result
            .transcript_text()
            .contains("**Cerebras API key saved.**"),
        "fresh-start lifecycle should show the user that the key was saved: {}",
        result.transcript_text()
    );
    jcode_base::auth::AuthStatus::invalidate_cache();
    assert_eq!(
        jcode_base::auth::AuthStatus::check_fast().state_for_provider(provider),
        jcode_base::auth::AuthState::Available
    );
}
