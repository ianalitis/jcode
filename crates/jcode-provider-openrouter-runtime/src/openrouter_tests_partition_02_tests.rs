#[test]
fn direct_openai_compatible_static_models_are_marked_as_fallback_before_live_catalog() {
    let provider = OpenRouterProvider {
        supports_provider_features: false,
        supports_model_catalog: true,
        profile_id: Some("opencode".to_string()),
        static_models: vec!["minimax-m2.7".to_string()],
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        ..make_custom_compatible_provider()
    };

    let routes = provider.model_routes();
    let route = routes
        .iter()
        .find(|route| route.model == "minimax-m2.7")
        .expect("static fallback route should be present before live catalog fetch");

    assert!(
        route
            .detail
            .contains("fallback: static provider model list"),
        "fallback routes should be clearly labeled in the model picker: {route:?}"
    );
}

#[test]
fn cerebras_live_catalog_models_are_selectable_on_explicit_switch() {
    let provider = OpenRouterProvider {
        supports_provider_features: false,
        supports_model_catalog: true,
        profile_id: Some("cerebras".to_string()),
        static_models: vec!["gpt-oss-120b".to_string()],
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        ..make_custom_compatible_provider()
    };

    provider
        .set_model("zai-glm-4.7")
        .expect("live Cerebras model should be selectable");
    assert_eq!(provider.model(), "zai-glm-4.7");
    provider
        .set_model("gpt-oss-120b")
        .expect("default Cerebras model should remain selectable");
    assert_eq!(provider.model(), "gpt-oss-120b");
}

#[test]
fn direct_deepseek_profile_uses_static_1m_context_when_catalog_is_absent() {
    let _lock = ENV_LOCK.lock();
    let _base = EnvVarGuard::set("JCODE_OPENROUTER_API_BASE", "https://api.deepseek.com");
    let _key_name = EnvVarGuard::set("JCODE_OPENROUTER_API_KEY_NAME", "DEEPSEEK_API_KEY");
    let _api_key = EnvVarGuard::set("DEEPSEEK_API_KEY", "test");
    let _namespace = EnvVarGuard::set("JCODE_OPENROUTER_CACHE_NAMESPACE", "deepseek");
    let _model = EnvVarGuard::set("JCODE_OPENROUTER_MODEL", "deepseek-v4-flash");
    let _catalog = EnvVarGuard::set("JCODE_OPENROUTER_MODEL_CATALOG", "0");

    let provider = OpenRouterProvider::new().expect("provider");

    assert_eq!(provider.context_window(), 1_000_000);
}

#[test]
fn explicit_cached_context_window_precedes_zai_family_fallback() {
    let model = "glm-5.3-issue-1087";
    jcode_base::provider::populate_context_limits(HashMap::from([(model.to_string(), 1_000_000)]));
    let provider = OpenRouterProvider {
        model: Arc::new(RwLock::new(model.to_string())),
        profile_id: Some("zai".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    assert_eq!(
        jcode_base::provider_catalog::openai_compatible_profile_context_limit("zai", model),
        Some(200_000),
        "the regression requires a conflicting static family guess"
    );
    assert_eq!(provider.context_window(), 1_000_000);
}

#[test]
fn named_openai_compatible_model_context_window_overrides_default() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let mut config = jcode_base::config::NamedProviderConfig {
        base_url: "https://compat.example.test/v1".to_string(),
        api_key: Some("test".to_string()),
        default_model: Some("custom-long-context".to_string()),
        models: vec![jcode_base::config::NamedProviderModelConfig {
            id: "custom-long-context".to_string(),
            context_window: Some(512_000),
            reasoning: None,
            reasoning_effort: None,
            input: Vec::new(),
        }],
        ..Default::default()
    };
    config.model_catalog = false;

    let provider =
        OpenRouterProvider::new_named_openai_compatible("custom", &config).expect("provider");

    assert_eq!(provider.context_window(), 512_000);
}

#[test]
fn named_profile_context_window_survives_provider_qualified_model() {
    // Regression for #403: if the runtime model transiently carries the
    // session-routing `<profile>:<model>` prefix, context_window() must still
    // resolve the configured per-model context_window rather than falling
    // through to the (large) provider default and over-budgeting the request.
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let mut config = jcode_base::config::NamedProviderConfig {
        base_url: "http://10.15.15.53:8080/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        default_model: Some("qwen3.6-35b-a2000-128k".to_string()),
        models: vec![jcode_base::config::NamedProviderModelConfig {
            id: "qwen3.6-35b-a2000-128k".to_string(),
            context_window: Some(131_072),
            reasoning: None,
            reasoning_effort: None,
            input: Vec::new(),
        }],
        ..Default::default()
    };
    config.model_catalog = false;
    config.requires_api_key = Some(false);

    let provider = OpenRouterProvider::new_named_openai_compatible("cachyai-a2000", &config)
        .expect("provider");

    // Simulate the poisoned/qualified runtime model that #403 reported.
    {
        let mut model = provider.model.try_write().expect("model lock");
        *model = "cachyai-a2000:qwen3.6-35b-a2000-128k".to_string();
    }

    assert_eq!(provider.context_window(), 131_072);
}

#[test]
fn named_openai_compatible_loads_api_key_from_env_file() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp dir");
    let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let _api_key = EnvVarGuard::remove("CUSTOM_API_KEY");
    write_test_api_key(&temp, "custom.env", "CUSTOM_API_KEY", "from-env-file");

    let config = jcode_base::config::NamedProviderConfig {
        base_url: "https://compat.example.test/v1".to_string(),
        api_key_env: Some("CUSTOM_API_KEY".to_string()),
        env_file: Some("custom.env".to_string()),
        default_model: Some("custom-model".to_string()),
        ..Default::default()
    };

    OpenRouterProvider::new_named_openai_compatible("custom", &config)
        .expect("provider should load key from env file");
}

#[test]
fn custom_compatible_provider_preserves_claude_like_model_ids() {
    let provider = make_custom_compatible_provider();

    provider.set_model("claude-opus4.6-thinking").unwrap();

    assert_eq!(provider.model(), "claude-opus4.6-thinking");
}

#[test]
fn custom_compatible_provider_preserves_at_sign_model_ids() {
    let provider = make_custom_compatible_provider();

    provider.set_model("gpt-5.4@OpenAI").unwrap();

    assert_eq!(provider.model(), "gpt-5.4@OpenAI");
}

#[test]
fn named_profile_set_model_strips_own_session_routing_prefix() {
    // Session restore persists `<profile>:<model>`; the standalone provider
    // must normalize its own profile prefix back to the bare model id so the
    // upstream API never sees `tokenrouter:MiniMax-M3` (issues #382/#383/#363).
    let provider = OpenRouterProvider {
        profile_id: Some("tokenrouter".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    provider.set_model("tokenrouter:MiniMax-M3").unwrap();
    assert_eq!(provider.model(), "MiniMax-M3");

    // Bare ids still work unchanged.
    provider.set_model("MiniMax-M3").unwrap();
    assert_eq!(provider.model(), "MiniMax-M3");
}

#[test]
fn named_profile_set_model_strips_other_known_profile_prefix() {
    // A session saved under one built-in OpenAI-compatible profile and
    // reattached under another must still normalize to the bare model id.
    let provider = OpenRouterProvider {
        profile_id: Some("tokenrouter".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    provider.set_model("kimi:kimi-for-coding").unwrap();
    assert_eq!(provider.model(), "kimi-for-coding");
}

#[test]
fn named_profile_set_model_keeps_builtin_routing_prefixes() {
    // Built-in provider routing prefixes must round-trip verbatim so a user can
    // switch the active provider from a saved session.
    let provider = OpenRouterProvider {
        profile_id: Some("tokenrouter".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    for spec in [
        "claude-oauth:claude-opus-4-8",
        "openai-api:gpt-5.4",
        "copilot:gpt-5.4",
    ] {
        provider.set_model(spec).unwrap();
        assert_eq!(provider.model(), spec, "spec {spec} must be preserved");
    }
}

#[test]
fn named_profile_set_model_keeps_unknown_prefix_with_colon() {
    // A `:`-bearing id whose prefix is neither this profile nor a known
    // built-in profile must be preserved verbatim (it may be a real model id).
    let provider = OpenRouterProvider {
        profile_id: Some("tokenrouter".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    provider.set_model("some-vendor:weird-model").unwrap();
    assert_eq!(provider.model(), "some-vendor:weird-model");
}

#[test]
fn openrouter_provider_normalizes_bare_pinned_model_ids() {
    let provider = make_provider();

    provider.set_model("gpt-5.4@OpenAI").unwrap();

    assert_eq!(provider.model(), "openai/gpt-5.4");
}

#[test]
fn test_rank_providers_cache_priority() {
    let endpoints = vec![
        make_endpoint("FastCache", 50.0, 99.0, true, 0.0000002),
        make_endpoint("FasterNoCache", 60.0, 99.0, false, 0.0000001),
    ];

    let ranked = OpenRouterProvider::rank_providers_from_endpoints(&endpoints);
    assert_eq!(ranked.first().map(|s| s.as_str()), Some("FastCache"));
}

#[test]
fn test_rank_providers_speed_priority_among_cache_capable() {
    let endpoints = vec![
        make_endpoint("Fireworks", 120.0, 99.0, true, 0.0000013),
        make_endpoint("Moonshot AI", 80.0, 99.0, true, 0.0000010),
    ];

    let ranked = OpenRouterProvider::rank_providers_from_endpoints(&endpoints);
    assert_eq!(ranked.first().map(|s| s.as_str()), Some("Fireworks"));
}

#[test]
fn test_rank_providers_filters_down_providers() {
    let mut down_ep = make_endpoint("DownProvider", 200.0, 100.0, true, 0.0000001);
    down_ep.status = Some(1); // down
    let endpoints = vec![
        down_ep,
        make_endpoint("UpProvider", 50.0, 99.0, true, 0.0000002),
    ];

    let ranked = OpenRouterProvider::rank_providers_from_endpoints(&endpoints);
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0], "UpProvider");
}

#[test]
fn test_background_refresh_waits_for_soft_ttl() {
    let provider = make_provider();

    assert!(!provider.should_background_refresh_model_catalog(
        MODEL_CATALOG_SOFT_REFRESH_SECS.saturating_sub(1)
    ));
    assert!(provider.should_background_refresh_model_catalog(MODEL_CATALOG_SOFT_REFRESH_SECS));
}

#[test]
fn test_background_refresh_is_throttled_between_attempts() {
    let provider = make_provider();
    assert!(provider.begin_background_model_catalog_refresh());
    assert!(!provider.should_background_refresh_model_catalog(MODEL_CATALOG_SOFT_REFRESH_SECS));

    OpenRouterProvider::finish_background_model_catalog_refresh(&provider.model_catalog_refresh);

    assert!(!provider.should_background_refresh_model_catalog(MODEL_CATALOG_SOFT_REFRESH_SECS));
}

#[test]
fn test_kimi_routing_uses_endpoints_or_fallback() {
    let provider = OpenRouterProvider {
        model: Arc::new(RwLock::new("moonshotai/kimi-k2.5".to_string())),
        ..make_provider()
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let routing = rt.block_on(provider.effective_routing("moonshotai/kimi-k2.5"));
    let order = routing.order.expect("provider order should be set");
    // Should have providers - either from endpoint API or Kimi fallback
    assert!(
        !order.is_empty(),
        "Kimi routing should always produce a provider order"
    );
}

#[test]
fn observed_session_provider_pin_sticks_without_fallbacks() {
    // Simulates the KV-cache stickiness contract: after OpenRouter serves a
    // request for this model from a concrete provider (recorded as an
    // observed pin), every subsequent request must route to that exact same
    // provider with fallbacks disabled so the upstream prompt cache stays warm.
    let model = "anthropic/claude-sonnet-4.6";
    let provider = OpenRouterProvider {
        model: Arc::new(RwLock::new(model.to_string())),
        provider_pin: Arc::new(Mutex::new(Some(ProviderPin {
            model: model.to_string(),
            provider: "anthropic".to_string(),
            source: PinSource::Observed,
            allow_fallbacks: true,
            last_cache_read: None,
        }))),
        ..make_provider()
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let routing = rt.block_on(provider.effective_routing(model));

    assert_eq!(
        routing.order.as_deref(),
        Some(["anthropic".to_string()].as_slice()),
        "observed session provider should be pinned exactly"
    );
    assert!(
        !routing.allow_fallbacks,
        "observed session pin must disable fallbacks to preserve the KV cache"
    );
}

#[test]
fn observed_pin_yields_to_explicit_user_routing_order() {
    // If the user explicitly narrowed routing themselves (base order set),
    // their configured order wins over the auto-observed session pin.
    let model = "anthropic/claude-sonnet-4.6";
    let base = ProviderRouting {
        order: Some(vec!["fireworks".to_string()]),
        ..Default::default()
    };
    let provider = OpenRouterProvider {
        model: Arc::new(RwLock::new(model.to_string())),
        provider_routing: Arc::new(RwLock::new(base)),
        provider_pin: Arc::new(Mutex::new(Some(ProviderPin {
            model: model.to_string(),
            provider: "anthropic".to_string(),
            source: PinSource::Observed,
            allow_fallbacks: true,
            last_cache_read: None,
        }))),
        ..make_provider()
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let routing = rt.block_on(provider.effective_routing(model));

    assert_eq!(
        routing.order.as_deref(),
        Some(["fireworks".to_string()].as_slice()),
        "explicit user routing order should win over an observed session pin"
    );
}

#[test]
fn test_kimi_coding_header_detection_matches_endpoint_and_model() {
    assert!(should_send_kimi_coding_agent_headers(
        "https://api.kimi.com/coding/v1",
        None,
    ));
    assert!(should_send_kimi_coding_agent_headers(
        "https://coding.dashscope.aliyuncs.com/v1",
        None,
    ));
    assert!(should_send_kimi_coding_agent_headers(
        "https://coding-intl.dashscope.aliyuncs.com/v1",
        None,
    ));
    assert!(should_send_kimi_coding_agent_headers(
        "https://api.z.ai/api/coding/paas/v4",
        None,
    ));
    assert!(should_send_kimi_coding_agent_headers(
        "https://example.com/v1",
        Some("kimi-for-coding"),
    ));
    assert!(should_send_kimi_coding_agent_headers(
        "https://openrouter.ai/api/v1",
        Some("moonshotai/kimi-k2.5"),
    ));
    assert!(!should_send_kimi_coding_agent_headers(
        "https://api.openrouter.ai/api/v1",
        Some("anthropic/claude-sonnet-4"),
    ));
}

#[test]
fn test_openrouter_kimi_chat_request_includes_compat_user_agent() {
    let request = apply_kimi_coding_agent_headers(
        Client::new().post("https://openrouter.ai/api/v1/chat/completions"),
        "https://openrouter.ai/api/v1",
        Some("moonshotai/kimi-k2.5"),
    )
    .build()
    .expect("build request");
    assert!(
        request
            .headers()
            .get("User-Agent")
            .and_then(|value| value.to_str().ok())
            == Some(KIMI_CODING_USER_AGENT),
        "Kimi OpenRouter chat request should include compatibility User-Agent"
    );
}

#[test]
fn test_parse_next_event_accepts_compact_sse_data_and_reasoning_content() {
    let bytes = Bytes::from_static(
        b"data:{\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}\n\n",
    );
    let mut stream = OpenRouterStream::new(
        futures::stream::once(async move { Ok::<Bytes, reqwest::Error>(bytes) }),
        "kimi-for-coding".to_string(),
        Arc::new(Mutex::new(None)),
    );

    match futures::executor::block_on(stream.next()) {
        Some(Ok(StreamEvent::ThinkingDelta(text))) => assert_eq!(text, "thinking"),
        other => panic!("expected ThinkingDelta, got {:?}", other),
    }
}

#[test]
fn test_parse_next_event_emits_only_incremental_reasoning_content() {
    let chunks = vec![
        Ok::<Bytes, reqwest::Error>(Bytes::from_static(
            b"data:{\"choices\":[{\"delta\":{\"reasoning_content\":\"Thinking\"}}]}\n\n",
        )),
        Ok::<Bytes, reqwest::Error>(Bytes::from_static(
            b"data:{\"choices\":[{\"delta\":{\"reasoning_content\":\"Thinking more\"}}]}\n\n",
        )),
    ];
    let mut stream = OpenRouterStream::new(
        futures::stream::iter(chunks),
        "moonshotai/kimi-k2.5".to_string(),
        Arc::new(Mutex::new(None)),
    );

    match futures::executor::block_on(stream.next()) {
        Some(Ok(StreamEvent::ThinkingDelta(text))) => assert_eq!(text, "Thinking"),
        other => panic!("expected first ThinkingDelta, got {:?}", other),
    }
    match futures::executor::block_on(stream.next()) {
        Some(Ok(StreamEvent::ThinkingDelta(text))) => assert_eq!(text, " more"),
        other => panic!("expected incremental ThinkingDelta, got {:?}", other),
    }
}

#[test]
fn test_endpoint_detail_string() {
    let ep = EndpointInfo {
        provider_name: "TestProvider".to_string(),
        tag: None,
        pricing: ModelPricing {
            prompt: Some("0.00000045".to_string()),
            completion: Some("0.00000225".to_string()),
            input_cache_read: Some("0.00000007".to_string()),
            input_cache_write: Some("0.00000012".to_string()),
        },
        context_length: Some(131072),
        max_completion_tokens: Some(8192),
        quantization: Some("fp8".to_string()),
        uptime_last_30m: Some(99.5),
        latency_last_30m: Some(serde_json::json!({"p50": 500, "p75": 800})),
        throughput_last_30m: Some(serde_json::json!({"p50": 42, "p75": 55})),
        supports_implicit_caching: Some(true),
        status: Some(0),
    };
    let detail = ep.detail_string();
    assert!(
        detail.contains("$0.45/M"),
        "should contain price: {}",
        detail
    );
    assert!(detail.contains("100%"), "should contain uptime: {}", detail);
    assert!(
        detail.contains("out $2.25/M"),
        "should contain output price: {}",
        detail
    );
    assert!(
        detail.contains("cache write $0.12/M"),
        "should contain cache write price: {}",
        detail
    );
    assert!(
        detail.contains("cache read $0.07/M"),
        "should contain cache read price: {}",
        detail
    );
    assert!(
        detail.contains("500ms p50"),
        "should contain latency: {}",
        detail
    );
    assert!(
        detail.contains("42tps"),
        "should contain throughput: {}",
        detail
    );
    assert!(
        detail.contains("cache on"),
        "should contain cache: {}",
        detail
    );
    assert!(
        detail.contains("fp8"),
        "should contain quantization: {}",
        detail
    );
}

#[test]
fn strict_openai_schema_endpoint_detects_mistral_profile() {
    // Mistral direct profile rejects non-standard reasoning_content/thinking
    // fields with a 422 (issue #261), so it must be flagged strict.
    assert!(OpenRouterProvider::strict_openai_schema_endpoint(
        Some("mistral"),
        "https://api.mistral.ai/v1"
    ));
    assert!(OpenRouterProvider::strict_openai_schema_endpoint(
        Some("MISTRAL"),
        "https://example.com/v1"
    ));
}

#[test]
fn strict_openai_schema_endpoint_detects_mistral_api_base() {
    assert!(OpenRouterProvider::strict_openai_schema_endpoint(
        None,
        "https://api.mistral.ai/v1"
    ));
    assert!(OpenRouterProvider::strict_openai_schema_endpoint(
        Some("custom"),
        "https://API.MISTRAL.AI/v1"
    ));
}

#[test]
fn strict_openai_schema_endpoint_allows_other_providers() {
    assert!(!OpenRouterProvider::strict_openai_schema_endpoint(
        Some("deepseek"),
        "https://api.deepseek.com"
    ));
    assert!(!OpenRouterProvider::strict_openai_schema_endpoint(
        None,
        "https://openrouter.ai/api/v1"
    ));
    assert!(!OpenRouterProvider::strict_openai_schema_endpoint(
        Some("openai"),
        "https://api.openai.com/v1"
    ));
}

#[test]
fn runtime_display_name_for_profile_runtime_instance() {
    // Direct unit coverage of the per-instance resolver used by
    // `Provider::display_name`.
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let jcode_home = temp.path().join("jcode-home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", &jcode_home);
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _key = EnvVarGuard::set("NVIDIA_API_KEY", "nim-test-key");

    let nim = OpenRouterProvider::new_openai_compatible_profile_runtime(
        jcode_base::provider_catalog::NVIDIA_NIM_PROFILE,
    )
    .expect("build nvidia-nim runtime");
    assert_eq!(nim.runtime_display_name(), "NVIDIA NIM");
    assert_eq!(Provider::name(&nim), "openrouter");
}

#[test]
fn jcode_subscription_runtime_has_explicit_display_and_route_identity() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let jcode_home = temp.path().join("jcode-home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", &jcode_home);
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _base = EnvVarGuard::set(
        "JCODE_OPENROUTER_API_BASE",
        jcode_base::subscription_catalog::DEFAULT_JCODE_API_BASE,
    );
    let _key_name = EnvVarGuard::set(
        "JCODE_OPENROUTER_API_KEY_NAME",
        jcode_base::subscription_catalog::JCODE_API_KEY_ENV,
    );
    let _env_file = EnvVarGuard::set(
        "JCODE_OPENROUTER_ENV_FILE",
        jcode_base::subscription_catalog::JCODE_ENV_FILE,
    );
    let _provider_features = EnvVarGuard::set("JCODE_OPENROUTER_PROVIDER_FEATURES", "0");
    let _transport = EnvVarGuard::set("JCODE_OPENROUTER_TRANSPORT_STATE", "jcode-subscription");
    let _key = EnvVarGuard::set(
        jcode_base::subscription_catalog::JCODE_API_KEY_ENV,
        "jcode_test_subscription_key",
    );

    let provider = OpenRouterProvider::new().expect("build jcode subscription runtime");
    assert_eq!(provider.runtime_display_name(), "Jcode Subscription");
    assert_eq!(Provider::display_name(&provider), "Jcode Subscription");
    assert_eq!(Provider::name(&provider), "openrouter");
    assert_eq!(
        provider.direct_openai_compatible_route_parts(),
        Some((
            "Jcode Subscription".to_string(),
            "jcode-subscription".to_string(),
            jcode_base::subscription_catalog::DEFAULT_JCODE_API_BASE.to_string(),
        ))
    );
}

#[test]
fn non_subscription_runtimes_keep_existing_display_and_route_identity() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let jcode_home = temp.path().join("jcode-home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", &jcode_home);
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _openrouter_key = EnvVarGuard::set("OPENROUTER_API_KEY", "openrouter-test-key");

    let openrouter =
        OpenRouterProvider::new_openrouter_api_key_runtime().expect("build OpenRouter runtime");
    assert_eq!(openrouter.runtime_display_name(), "OpenRouter");
    assert_eq!(Provider::display_name(&openrouter), "OpenRouter");
    assert_eq!(openrouter.direct_openai_compatible_route_parts(), None);

    let _base = EnvVarGuard::set("JCODE_OPENROUTER_API_BASE", "https://example.com/v1");
    let _key_name = EnvVarGuard::set("JCODE_OPENROUTER_API_KEY_NAME", "GENERIC_API_KEY");
    let _provider_features = EnvVarGuard::set("JCODE_OPENROUTER_PROVIDER_FEATURES", "0");
    let _transport = EnvVarGuard::set("JCODE_OPENROUTER_TRANSPORT_STATE", "direct-compatible");
    let _generic_key = EnvVarGuard::set("GENERIC_API_KEY", "generic-test-key");

    let compatible = OpenRouterProvider::new().expect("build generic compatible runtime");
    assert_eq!(compatible.runtime_display_name(), "OpenAI-compatible");
    assert_eq!(Provider::display_name(&compatible), "OpenAI-compatible");
    assert_eq!(
        compatible.direct_openai_compatible_route_parts(),
        Some((
            "OpenAI-compatible".to_string(),
            "openai-compatible".to_string(),
            "https://example.com/v1".to_string(),
        ))
    );
}

#[test]
fn custom_endpoint_using_jcode_key_name_is_not_a_subscription_runtime() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let jcode_home = temp.path().join("jcode-home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", &jcode_home);
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _base = EnvVarGuard::set("JCODE_OPENROUTER_API_BASE", "https://example.com/v1");
    let _key_name = EnvVarGuard::set(
        "JCODE_OPENROUTER_API_KEY_NAME",
        jcode_base::subscription_catalog::JCODE_API_KEY_ENV,
    );
    let _provider_features = EnvVarGuard::set("JCODE_OPENROUTER_PROVIDER_FEATURES", "0");
    let _key = EnvVarGuard::set(
        jcode_base::subscription_catalog::JCODE_API_KEY_ENV,
        "custom-endpoint-test-key",
    );

    let provider = OpenRouterProvider::new().expect("build custom endpoint runtime");
    assert_eq!(provider.runtime_display_name(), "OpenAI-compatible");
    assert_eq!(
        provider.direct_openai_compatible_route_parts(),
        Some((
            "OpenAI-compatible".to_string(),
            "openai-compatible".to_string(),
            "https://example.com/v1".to_string(),
        ))
    );
}

#[test]
fn resolve_extra_body_returns_none_when_unset() {
    let _lock = ENV_LOCK.lock();
    let _guard = EnvVarGuard::remove("JCODE_OPENAI_EXTRA_BODY");
    assert!(OpenRouterProvider::resolve_extra_body(None, "nonexistent.env").is_none());
}

#[test]
fn resolve_extra_body_parses_env_json_object() {
    let _lock = ENV_LOCK.lock();
    let _guard = EnvVarGuard::set(
        "JCODE_OPENAI_EXTRA_BODY",
        r#"{"chat_template_kwargs":{"thinking":true,"reasoning_effort":"high"}}"#,
    );
    let extra =
        OpenRouterProvider::resolve_extra_body(None, "nonexistent.env").expect("extra body");
    let kwargs = extra
        .get("chat_template_kwargs")
        .and_then(|v| v.as_object())
        .expect("chat_template_kwargs object");
    assert_eq!(kwargs.get("thinking"), Some(&serde_json::json!(true)));
    assert_eq!(
        kwargs.get("reasoning_effort"),
        Some(&serde_json::json!("high"))
    );
}

#[test]
fn resolve_extra_body_ignores_invalid_env_json() {
    let _lock = ENV_LOCK.lock();
    let _guard = EnvVarGuard::set("JCODE_OPENAI_EXTRA_BODY", "not-json");
    assert!(OpenRouterProvider::resolve_extra_body(None, "nonexistent.env").is_none());
}

#[test]
fn resolve_extra_body_ignores_non_object_env_json() {
    let _lock = ENV_LOCK.lock();
    let _guard = EnvVarGuard::set("JCODE_OPENAI_EXTRA_BODY", "[1,2,3]");
    assert!(OpenRouterProvider::resolve_extra_body(None, "nonexistent.env").is_none());
}

#[test]
fn resolve_extra_body_merges_config_and_env_with_env_override() {
    let _lock = ENV_LOCK.lock();
    let config = serde_json::json!({
        "chat_template_kwargs": {"thinking": false},
        "config_only": 1,
    });
    let _guard = EnvVarGuard::set(
        "JCODE_OPENAI_EXTRA_BODY",
        r#"{"chat_template_kwargs":{"thinking":true},"env_only":2}"#,
    );
    let extra = OpenRouterProvider::resolve_extra_body(Some(&config), "nonexistent.env")
        .expect("merged extra body");
    // Env overrides the colliding key.
    assert_eq!(
        extra
            .get("chat_template_kwargs")
            .and_then(|v| v.get("thinking")),
        Some(&serde_json::json!(true))
    );
    // Non-colliding keys from both sources survive.
    assert_eq!(extra.get("config_only"), Some(&serde_json::json!(1)));
    assert_eq!(extra.get("env_only"), Some(&serde_json::json!(2)));
}

#[test]
fn resolve_extra_body_ignores_non_object_config() {
    let _lock = ENV_LOCK.lock();
    let _guard = EnvVarGuard::remove("JCODE_OPENAI_EXTRA_BODY");
    let config = serde_json::json!("not an object");
    assert!(OpenRouterProvider::resolve_extra_body(Some(&config), "nonexistent.env").is_none());
}

#[test]
fn named_profile_extra_body_threads_into_provider() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let jcode_home = temp.path().join("jcode-home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", &jcode_home);
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _extra_guard = EnvVarGuard::remove("JCODE_OPENAI_EXTRA_BODY");

    let mut profile = jcode_base::config::NamedProviderConfig {
        base_url: "https://integrate.api.nvidia.com/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        requires_api_key: Some(false),
        ..Default::default()
    };
    profile.extra_body = Some(serde_json::json!({
        "chat_template_kwargs": {"thinking": true, "reasoning_effort": "high"}
    }));

    let provider = OpenRouterProvider::new_named_openai_compatible("my-nim", &profile)
        .expect("build named provider");
    let extra = provider.extra_body.as_ref().expect("extra body present");
    assert_eq!(
        extra
            .get("chat_template_kwargs")
            .and_then(|v| v.get("reasoning_effort")),
        Some(&serde_json::json!("high"))
    );
}
