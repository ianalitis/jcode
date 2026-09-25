#[test]
fn cerebras_profile_exposes_live_chat_models_before_catalog_refresh() {
    assert_eq!(
        jcode_provider_metadata::CEREBRAS_PROFILE.default_model,
        Some("gpt-oss-120b")
    );

    let models = jcode_base::provider_catalog::openai_compatible_profile_static_models(
        jcode_provider_metadata::CEREBRAS_PROFILE,
    );

    assert!(
        !models.iter().any(|model| model == "qwen-3-coder-480b"),
        "old Cerebras default is no longer returned by the live /models catalog"
    );
    assert!(models.iter().any(|model| model == "gpt-oss-120b"));
    assert!(models.iter().any(|model| model == "zai-glm-4.7"));
    assert!(
        !models
            .iter()
            .any(|model| model == "qwen-3-235b-a22b-instruct-2507")
    );
    assert!(!models.iter().any(|model| model == "llama3.1-8b"));
}

#[test]
fn openai_compatible_profiles_with_unverified_live_catalogs_have_static_fallbacks() {
    let cases = [
        (jcode_provider_metadata::OPENCODE_PROFILE, "minimax-m2.7"),
        (jcode_provider_metadata::OPENCODE_GO_PROFILE, "kimi-k2.5"),
        (jcode_provider_metadata::ZAI_PROFILE, "glm-4.7"),
        (
            jcode_provider_metadata::AI302_PROFILE,
            "qwen3-235b-a22b-instruct-2507",
        ),
        (jcode_provider_metadata::BASETEN_PROFILE, "zai-org/GLM-4.7"),
        (jcode_provider_metadata::CORTECS_PROFILE, "kimi-k2.5"),
        (jcode_provider_metadata::KIMI_PROFILE, "kimi-for-coding"),
        (jcode_provider_metadata::FIRMWARE_PROFILE, "kimi-k2.5"),
        (
            jcode_provider_metadata::HUGGING_FACE_PROFILE,
            "Qwen/Qwen3-Coder-480B-A35B-Instruct",
        ),
        (jcode_provider_metadata::MOONSHOT_PROFILE, "kimi-k2.5"),
        (
            jcode_provider_metadata::NEBIUS_PROFILE,
            "openai/gpt-oss-120b",
        ),
        (
            jcode_provider_metadata::SCALEWAY_PROFILE,
            "qwen3-coder-30b-a3b-instruct",
        ),
        (
            jcode_provider_metadata::STACKIT_PROFILE,
            "openai/gpt-oss-120b",
        ),
        (jcode_provider_metadata::PERPLEXITY_PROFILE, "sonar"),
        (
            jcode_provider_metadata::DEEPINFRA_PROFILE,
            "moonshotai/Kimi-K2-Instruct",
        ),
        (
            jcode_provider_metadata::FIREWORKS_PROFILE,
            "accounts/fireworks/routers/kimi-k2p5-turbo",
        ),
        (jcode_provider_metadata::XIAOMI_MIMO_PROFILE, "mimo-v2.5"),
        (jcode_provider_metadata::META_MUSE_PROFILE, "muse-spark-1.2"),
        (
            jcode_provider_metadata::ALIBABA_CODING_PLAN_PROFILE,
            "qwen3-coder-plus",
        ),
    ];

    for (profile, expected_model) in cases {
        let models = jcode_base::provider_catalog::openai_compatible_profile_static_models(profile);
        assert!(
            models.iter().any(|model| model == expected_model),
            "{} should expose static fallback model {expected_model}; got {models:?}",
            profile.id
        );
    }
}

#[test]
fn profiles_use_endpoint_default_max_tokens() {
    let _lock = ENV_LOCK.lock();
    let _override = EnvVarGuard::remove("JCODE_OPENROUTER_MAX_TOKENS");

    assert_eq!(
        OpenRouterProvider::configured_max_tokens(Some("comtegra")),
        None
    );
    assert_eq!(
        OpenRouterProvider::configured_max_tokens(Some("deepseek")),
        None
    );
    assert_eq!(
        OpenRouterProvider::configured_max_tokens(Some("celeris")),
        None
    );
}

#[test]
fn max_tokens_env_overrides_profile_default() {
    let _lock = ENV_LOCK.lock();
    let _override = EnvVarGuard::set("JCODE_OPENROUTER_MAX_TOKENS", "4096");

    assert_eq!(
        OpenRouterProvider::configured_max_tokens(Some("comtegra")),
        Some(4096)
    );
}

#[test]
fn test_configured_api_base_accepts_https() {
    let _lock = ENV_LOCK.lock();
    let prev = std::env::var("JCODE_OPENROUTER_API_BASE").ok();
    jcode_base::env::set_var(
        "JCODE_OPENROUTER_API_BASE",
        "https://api.groq.com/openai/v1/",
    );
    assert_eq!(configured_api_base(), "https://api.groq.com/openai/v1");
    if let Some(value) = prev {
        jcode_base::env::set_var("JCODE_OPENROUTER_API_BASE", value);
    } else {
        jcode_base::env::remove_var("JCODE_OPENROUTER_API_BASE");
    }
}

#[test]
fn test_configured_api_base_rejects_insecure_http_remote() {
    let _lock = ENV_LOCK.lock();
    let prev = std::env::var("JCODE_OPENROUTER_API_BASE").ok();
    jcode_base::env::set_var("JCODE_OPENROUTER_API_BASE", "http://example.com/v1");
    assert_eq!(configured_api_base(), DEFAULT_API_BASE);
    if let Some(value) = prev {
        jcode_base::env::set_var("JCODE_OPENROUTER_API_BASE", value);
    } else {
        jcode_base::env::remove_var("JCODE_OPENROUTER_API_BASE");
    }
}

#[test]
fn autodetects_single_saved_openai_compatible_profile() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp dir");
    let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _jcode_home = pin_test_jcode_home(&temp);

    let opencode = jcode_base::provider_catalog::resolve_openai_compatible_profile(
        jcode_base::provider_catalog::OPENCODE_PROFILE,
    );
    write_test_api_key(
        &temp,
        &opencode.env_file,
        &opencode.api_key_env,
        "test-opencode-key",
    );

    assert_eq!(configured_api_base(), opencode.api_base);
    assert_eq!(configured_api_key_name(), opencode.api_key_env);
    assert_eq!(configured_env_file_name(), opencode.env_file);
    assert!(OpenRouterProvider::has_credentials());
}

#[test]
fn autodetects_single_saved_local_openai_compatible_profile() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp dir");
    let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _jcode_home = pin_test_jcode_home(&temp);

    let lmstudio = jcode_base::provider_catalog::resolve_openai_compatible_profile(
        jcode_base::provider_catalog::LMSTUDIO_PROFILE,
    );
    let config_dir = test_app_config_dir(&temp);
    std::fs::create_dir_all(&config_dir).expect("create test config dir");
    std::fs::write(
        config_dir.join(&lmstudio.env_file),
        format!(
            "{}=1\n",
            jcode_base::provider_catalog::OPENAI_COMPAT_LOCAL_ENABLED_ENV
        ),
    )
    .expect("write local config");

    assert_eq!(configured_api_base(), lmstudio.api_base);
    assert_eq!(configured_api_key_name(), lmstudio.api_key_env);
    assert_eq!(configured_env_file_name(), lmstudio.env_file);
    assert!(configured_allow_no_auth());
    assert!(OpenRouterProvider::has_credentials());
}

#[test]
fn openrouter_transport_state_distinguishes_runtime_identities() {
    let _lock = ENV_LOCK.lock();
    // Isolate the on-disk config/credential lookup the same way the sibling
    // autodetect tests do, so this test does not read whatever provider
    // profile happens to be configured on the host machine.
    let temp = TempDir::new().expect("create temp dir");
    let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _jcode_home = pin_test_jcode_home(&temp);

    assert_eq!(
        OpenRouterTransportState::from_current_env(None),
        OpenRouterTransportState::OpenRouterApiKey
    );
    assert!(OpenRouterTransportState::from_current_env(None).accrues_user_api_key_cost());
    assert!(OpenRouterTransportState::from_current_env(None).is_real_openrouter());

    jcode_base::env::set_var("JCODE_OPENROUTER_TRANSPORT_STATE", "direct-api-key");
    assert_eq!(
        OpenRouterTransportState::from_current_env(None),
        OpenRouterTransportState::DirectApiKey
    );
    jcode_base::env::remove_var("JCODE_OPENROUTER_TRANSPORT_STATE");

    jcode_base::env::set_var("JCODE_RUNTIME_PROVIDER", "openrouter");
    assert_eq!(
        OpenRouterTransportState::from_current_env(Some("openrouter")),
        OpenRouterTransportState::OpenRouterApiKey
    );
    assert!(OpenRouterTransportState::from_current_env(Some("openrouter")).is_real_openrouter());
    jcode_base::env::remove_var("JCODE_RUNTIME_PROVIDER");

    jcode_base::env::set_var("JCODE_RUNTIME_PROVIDER", "jcode");
    assert_eq!(
        OpenRouterTransportState::from_current_env(Some("jcode")),
        OpenRouterTransportState::JcodeSubscription
    );
    assert!(!OpenRouterTransportState::from_current_env(Some("jcode")).accrues_user_api_key_cost());

    jcode_base::env::set_var("JCODE_RUNTIME_PROVIDER", "openai-compatible");
    assert_eq!(
        OpenRouterTransportState::from_current_env(Some("openai-compatible")),
        OpenRouterTransportState::DirectApiKey
    );

    jcode_base::env::set_var("JCODE_OPENROUTER_ALLOW_NO_AUTH", "1");
    assert_eq!(
        OpenRouterTransportState::from_current_env(Some("openai-compatible")),
        OpenRouterTransportState::DirectNoAuth
    );
    assert!(
        !OpenRouterTransportState::from_current_env(Some("openai-compatible"))
            .accrues_user_api_key_cost()
    );

    jcode_base::env::remove_var("JCODE_OPENROUTER_ALLOW_NO_AUTH");
    jcode_base::env::remove_var("JCODE_RUNTIME_PROVIDER");
    jcode_base::env::set_var("JCODE_NAMED_PROVIDER_PROFILE", "my-gateway");
    assert_eq!(
        OpenRouterTransportState::from_current_env(None),
        OpenRouterTransportState::DirectApiKey
    );
}

#[test]
fn does_not_guess_when_multiple_saved_openai_compatible_profiles_exist() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp dir");
    let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _jcode_home = pin_test_jcode_home(&temp);

    let opencode = jcode_base::provider_catalog::resolve_openai_compatible_profile(
        jcode_base::provider_catalog::OPENCODE_PROFILE,
    );
    let chutes = jcode_base::provider_catalog::resolve_openai_compatible_profile(
        jcode_base::provider_catalog::CHUTES_PROFILE,
    );
    write_test_api_key(
        &temp,
        &opencode.env_file,
        &opencode.api_key_env,
        "test-opencode-key",
    );
    write_test_api_key(
        &temp,
        &chutes.env_file,
        &chutes.api_key_env,
        "test-chutes-key",
    );

    assert_eq!(configured_api_base(), DEFAULT_API_BASE);
    assert_eq!(configured_api_key_name(), DEFAULT_API_KEY_NAME);
    assert_eq!(configured_env_file_name(), DEFAULT_ENV_FILE);
    assert!(!OpenRouterProvider::has_credentials());
}

#[test]
fn autodetected_profile_seeds_default_model_and_cache_namespace() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp dir");
    let _xdg = EnvVarGuard::set("XDG_CONFIG_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let _jcode_home = pin_test_jcode_home(&temp);

    let zai = jcode_base::provider_catalog::resolve_openai_compatible_profile(
        jcode_base::provider_catalog::ZAI_PROFILE,
    );
    write_test_api_key(&temp, &zai.env_file, &zai.api_key_env, "test-zai-key");

    let provider = OpenRouterProvider::new().expect("provider");
    assert_eq!(provider.model.blocking_read().clone(), "glm-4.5");
    assert_eq!(
        std::env::var("JCODE_OPENROUTER_CACHE_NAMESPACE")
            .ok()
            .as_deref(),
        Some("zai")
    );
}

#[test]
fn test_parse_model_spec() {
    let (model, provider) = parse_model_spec("anthropic/claude-sonnet-4@Fireworks");
    assert_eq!(model, "anthropic/claude-sonnet-4");
    let provider = provider.expect("provider");
    assert_eq!(provider.name, "Fireworks");
    assert!(!provider.allow_fallbacks);

    let (model, provider) = parse_model_spec("anthropic/claude-sonnet-4@Fireworks!");
    assert_eq!(model, "anthropic/claude-sonnet-4");
    let provider = provider.expect("provider");
    assert_eq!(provider.name, "Fireworks");
    assert!(!provider.allow_fallbacks);

    let (model, provider) = parse_model_spec("moonshotai/kimi-k2.5@moonshot");
    assert_eq!(model, "moonshotai/kimi-k2.5");
    let provider = provider.expect("provider");
    assert_eq!(provider.name, "Moonshot AI");

    let (model, provider) = parse_model_spec("anthropic/claude-sonnet-4@auto");
    assert_eq!(model, "anthropic/claude-sonnet-4");
    assert!(provider.is_none());
}

#[test]
fn fork_preserves_explicit_provider_pin() {
    let provider = make_provider();
    provider
        .set_model("z-ai/glm-5.2@Novita")
        .expect("set explicitly pinned model");

    let fork = provider.fork();

    assert_eq!(fork.model(), "z-ai/glm-5.2");
    assert_eq!(
        fork.explicit_provider_pin_for_current_model().as_deref(),
        Some("Novita")
    );
}

fn make_endpoint(name: &str, throughput: f64, uptime: f64, cache: bool, cost: f64) -> EndpointInfo {
    EndpointInfo {
        provider_name: name.to_string(),
        tag: None,
        pricing: ModelPricing {
            prompt: Some(format!("{:.10}", cost)),
            completion: None,
            input_cache_read: if cache {
                Some("0.00000007".to_string())
            } else {
                None
            },
            input_cache_write: None,
        },
        context_length: None,
        max_completion_tokens: None,
        quantization: None,
        uptime_last_30m: Some(uptime),
        latency_last_30m: None,
        throughput_last_30m: Some(serde_json::json!({"p50": throughput})),
        supports_implicit_caching: Some(cache),
        status: Some(0),
    }
}

fn make_provider() -> OpenRouterProvider {
    OpenRouterProvider {
        client: jcode_provider_core::shared_http_client(),
        model: Arc::new(RwLock::new(DEFAULT_MODEL.to_string())),
        reasoning_effort: Arc::new(RwLock::new(None)),
        api_base: DEFAULT_API_BASE.to_string(),
        auth: ProviderAuth::AuthorizationBearer {
            token: "test".to_string(),
            label: DEFAULT_API_KEY_NAME.to_string(),
        },
        supports_provider_features: true,
        supports_model_catalog: true,
        profile_id: None,
        reasoning_effort_support: None,
        disable_reasoning_heuristics: false,
        static_reasoning_config: HashMap::new(),
        max_tokens: None,
        extra_body: None,
        static_models: Vec::new(),
        static_context_limits: HashMap::new(),
        static_image_input_support: HashMap::new(),
        send_openrouter_headers: true,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        models_cache: Arc::new(RwLock::new(ModelsCache::default())),
        model_catalog_refresh: Arc::new(Mutex::new(ModelCatalogRefreshState::default())),
        endpoint_refresh: Arc::new(Mutex::new(EndpointRefreshTracker::default())),
        provider_routing: Arc::new(RwLock::new(ProviderRouting::default())),
        provider_pin: Arc::new(Mutex::new(None)),
        endpoints_cache: Arc::new(RwLock::new(HashMap::new())),
    }
}

fn make_custom_compatible_provider() -> OpenRouterProvider {
    OpenRouterProvider {
        client: jcode_provider_core::shared_http_client(),
        model: Arc::new(RwLock::new(DEFAULT_MODEL.to_string())),
        reasoning_effort: Arc::new(RwLock::new(None)),
        api_base: "https://compat.example.test/v1".to_string(),
        auth: ProviderAuth::AuthorizationBearer {
            token: "test".to_string(),
            label: "OPENAI_COMPAT_API_KEY".to_string(),
        },
        supports_provider_features: false,
        supports_model_catalog: true,
        profile_id: None,
        reasoning_effort_support: None,
        disable_reasoning_heuristics: false,
        static_reasoning_config: HashMap::new(),
        max_tokens: None,
        extra_body: None,
        static_models: Vec::new(),
        static_context_limits: HashMap::new(),
        static_image_input_support: HashMap::new(),
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        models_cache: Arc::new(RwLock::new(ModelsCache::default())),
        model_catalog_refresh: Arc::new(Mutex::new(ModelCatalogRefreshState::default())),
        endpoint_refresh: Arc::new(Mutex::new(EndpointRefreshTracker::default())),
        provider_routing: Arc::new(RwLock::new(ProviderRouting::default())),
        provider_pin: Arc::new(Mutex::new(None)),
        endpoints_cache: Arc::new(RwLock::new(HashMap::new())),
    }
}

fn spawn_single_response_models_server(body: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake provider server");
    let addr = listener.local_addr().expect("fake provider addr");
    let (request_tx, request_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fake provider request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        let mut request = vec![0u8; 8192];
        let n = stream.read(&mut request).unwrap_or(0);
        let request = String::from_utf8_lossy(&request[..n]).into_owned();
        let _ = request_tx.send(request);

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write fake provider response");
    });

    (format!("http://{addr}/v1"), request_rx)
}

fn spawn_single_response_chat_server() -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake provider server");
    let addr = listener.local_addr().expect("fake provider addr");
    let (request_tx, request_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fake provider request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        let mut request = vec![0u8; 16384];
        let n = stream.read(&mut request).unwrap_or(0);
        let request = String::from_utf8_lossy(&request[..n]).into_owned();
        let _ = request_tx.send(request);

        let body = "data: [DONE]\n\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write fake provider response");
    });

    (format!("http://{addr}/v1"), request_rx)
}

#[test]
fn direct_deepseek_profile_exposes_max_reasoning_effort() {
    let provider = OpenRouterProvider {
        profile_id: Some("deepseek".to_string()),
        supports_provider_features: false,
        ..make_custom_compatible_provider()
    };

    assert_eq!(
        provider.available_efforts(),
        vec![
            "none",
            "low",
            "medium",
            "high",
            "max",
            "swarm",
            "swarm-deep"
        ]
    );
    provider
        .set_reasoning_effort("max")
        .expect("DeepSeek direct profile should accept max effort");
    assert_eq!(provider.reasoning_effort().as_deref(), Some("max"));
}

#[test]
fn direct_zai_profile_exposes_openai_reasoning_effort_ladder() {
    let provider = OpenRouterProvider {
        profile_id: Some("zai".to_string()),
        supports_provider_features: false,
        ..make_custom_compatible_provider()
    };

    assert_eq!(
        provider.available_efforts(),
        jcode_provider_core::OPENAI_SELECTABLE_EFFORTS
    );
    provider
        .set_reasoning_effort("xhigh")
        .expect("Z.AI Coding Plan should accept xhigh effort");
    assert_eq!(provider.reasoning_effort().as_deref(), Some("xhigh"));
}

#[test]
fn direct_zai_profile_applies_configured_effort_on_construction_and_model_switch() {
    let configured = jcode_base::config::config()
        .provider
        .openai_reasoning_effort
        .as_deref()
        .and_then(OpenRouterProvider::normalize_openai_reasoning_effort);
    assert_eq!(
        OpenRouterProvider::initial_reasoning_effort(None, Some("zai")),
        configured
    );

    let provider = OpenRouterProvider {
        profile_id: Some("zai".to_string()),
        supports_provider_features: false,
        reasoning_effort: Arc::new(RwLock::new(None)),
        ..make_custom_compatible_provider()
    };
    provider.set_model("glm-5.3-flash").unwrap();
    assert_eq!(provider.reasoning_effort(), configured);
}

#[test]
fn openrouter_profile_exposes_unified_reasoning_effort() {
    let provider = make_provider();

    assert_eq!(
        provider.available_efforts(),
        vec![
            "none",
            "minimal",
            "low",
            "medium",
            "high",
            "xhigh",
            "swarm",
            "swarm-deep"
        ]
    );
    provider
        .set_reasoning_effort("minimal")
        .expect("OpenRouter minimal effort should be accepted");
    assert_eq!(provider.reasoning_effort().as_deref(), Some("minimal"));
    provider
        .set_reasoning_effort("max")
        .expect("OpenRouter max alias should be accepted");
    assert_eq!(provider.reasoning_effort().as_deref(), Some("xhigh"));
}

#[test]
fn openrouter_with_openrouter_profile_id_exposes_unified_reasoning_effort() {
    // The default OpenRouter api base matches the "openrouter" OpenAI-compat
    // doctor profile, so `new()` can assign profile_id = Some("openrouter").
    // That runtime is still real OpenRouter and must keep unified reasoning
    // (regression: /effort failed with "Reasoning effort is not supported").
    let provider = OpenRouterProvider {
        profile_id: Some("openrouter".to_string()),
        ..make_provider()
    };

    assert_eq!(
        provider.available_efforts(),
        vec![
            "none",
            "minimal",
            "low",
            "medium",
            "high",
            "xhigh",
            "swarm",
            "swarm-deep"
        ]
    );
    provider
        .set_reasoning_effort("high")
        .expect("OpenRouter with doctor profile id should accept effort");
    assert_eq!(provider.reasoning_effort().as_deref(), Some("high"));
}

#[test]
fn non_deepseek_compatible_profile_does_not_expose_reasoning_effort() {
    let provider = make_custom_compatible_provider();

    assert!(provider.available_efforts().is_empty());
    let error = provider
        .set_reasoning_effort("max")
        .expect_err("generic compatible profile should not expose DeepSeek effort UX");
    assert!(
        error.to_string().contains("not supported"),
        "unexpected error: {error:?}"
    );
}

#[test]
fn openrouter_chat_request_sends_unified_reasoning_effort() {
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        model: Arc::new(RwLock::new("anthropic/claude-sonnet-4.6".to_string())),
        supports_model_catalog: false,
        ..make_provider()
    };
    provider
        .set_reasoning_effort("high")
        .expect("OpenRouter unified reasoning should accept high effort");

    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "hello".to_string(),
            cache_control: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }];

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let mut stream = provider
            .complete(&messages, &[], "", None)
            .await
            .expect("fake chat request should start");
        while let Some(event) = stream.next().await {
            event.expect("stream event should parse");
        }
    });

    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("capture fake provider request");
    assert!(
        request.contains(r#""reasoning":{"effort":"high"}"#),
        "OpenRouter request should include unified reasoning effort: {request}"
    );
    assert!(
        !request.contains(r#""thinking":{"type":"enabled"}"#),
        "unified reasoning should supersede legacy thinking override: {request}"
    );
}

fn live_openrouter_models() -> Vec<String> {
    std::env::var("JCODE_LIVE_OPENROUTER_MODELS")
        .or_else(|_| std::env::var("JCODE_OPENROUTER_MODEL"))
        .unwrap_or_else(|_| "anthropic/claude-sonnet-4.6".to_string())
        .split([',', '\n'])
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToString::to_string)
        .collect()
}

async fn collect_openrouter_live_smoke_stream(
    mut stream: EventStream,
    timeout: Duration,
) -> Result<(usize, usize, bool)> {
    tokio::time::timeout(timeout, async move {
        let mut text_bytes = 0usize;
        let mut thinking_bytes = 0usize;
        let mut saw_message_end = false;
        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::TextDelta(text) => {
                    text_bytes += text.len();
                }
                StreamEvent::ThinkingDelta(text) => {
                    thinking_bytes += text.len();
                }
                StreamEvent::MessageEnd { .. } => {
                    saw_message_end = true;
                    break;
                }
                StreamEvent::Error { message, .. } => anyhow::bail!(message),
                _ => {}
            }
        }
        Ok((text_bytes, thinking_bytes, saw_message_end))
    })
    .await
    .context("live OpenRouter smoke timed out")?
}

#[tokio::test]
#[ignore = "live smoke: requires OPENROUTER_API_KEY or configured OpenRouter credentials"]
async fn live_openrouter_unified_reasoning_smoke() -> Result<()> {
    let _env_lock = ENV_LOCK.lock();
    let Some(token) = OpenRouterProvider::get_api_key() else {
        eprintln!(
            "skipping live OpenRouter smoke: OPENROUTER_API_KEY or configured OpenRouter credentials not found"
        );
        return Ok(());
    };

    let models = live_openrouter_models();
    let effort = std::env::var("JCODE_LIVE_OPENROUTER_REASONING_EFFORT")
        .unwrap_or_else(|_| "low".to_string());
    let max_tokens = std::env::var("JCODE_LIVE_OPENROUTER_MAX_TOKENS")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(1024);

    for model in models {
        let provider = OpenRouterProvider {
            auth: ProviderAuth::AuthorizationBearer {
                token: token.clone(),
                label: configured_api_key_name(),
            },
            model: Arc::new(RwLock::new(model.clone())),
            max_tokens: Some(max_tokens),
            ..make_provider()
        };
        provider.set_reasoning_effort(&effort)?;

        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Live smoke test: answer exactly OK.".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        }];

        let stream = provider
            .complete(
                &messages,
                &[],
                "You are a live provider smoke test. Keep the answer tiny.",
                None,
            )
            .await
            .with_context(|| format!("starting live OpenRouter stream for {model}"))?;
        let (text_bytes, thinking_bytes, saw_message_end) =
            collect_openrouter_live_smoke_stream(stream, Duration::from_secs(90))
                .await
                .with_context(|| format!("collecting live OpenRouter stream for {model}"))?;

        eprintln!(
            "live OpenRouter reasoning smoke passed: model={model}, effort={effort}, text_bytes={text_bytes}, thinking_bytes={thinking_bytes}, message_end={saw_message_end}"
        );
        assert!(
            text_bytes > 0 || thinking_bytes > 0,
            "live OpenRouter response for {model} contained neither text nor thinking deltas"
        );
    }

    Ok(())
}

#[test]
fn direct_deepseek_chat_request_sends_reasoning_effort() {
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        model: Arc::new(RwLock::new("deepseek-v4-pro".to_string())),
        profile_id: Some("deepseek".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        ..make_custom_compatible_provider()
    };
    provider
        .set_reasoning_effort("max")
        .expect("DeepSeek direct profile should accept max effort");

    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "hello".to_string(),
            cache_control: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }];

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let mut stream = provider
            .complete(&messages, &[], "", None)
            .await
            .expect("fake chat request should start");
        while let Some(event) = stream.next().await {
            event.expect("stream event should parse");
        }
    });

    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("capture fake provider request");
    assert!(
        request.starts_with("POST /v1/chat/completions "),
        "unexpected chat request: {request}"
    );
    assert!(
        request.contains(r#""model":"deepseek-v4-pro""#),
        "request should contain model: {request}"
    );
    assert!(
        request.contains(r#""reasoning_effort":"max""#),
        "DeepSeek request should include max reasoning effort: {request}"
    );
}

#[test]
fn direct_openai_compatible_chat_request_preserves_max_reasoning_effort() {
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        model: Arc::new(RwLock::new("gpt-5.5".to_string())),
        supports_provider_features: false,
        supports_model_catalog: false,
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        ..make_custom_compatible_provider()
    };
    provider
        .set_reasoning_effort("max")
        .expect("direct OpenAI-compatible profile should accept max effort");

    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "hello".to_string(),
            cache_control: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }];
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let mut stream = provider
            .complete(&messages, &[], "", None)
            .await
            .expect("fake chat request should start");
        while let Some(event) = stream.next().await {
            event.expect("stream event should parse");
        }
    });

    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("capture fake provider request");
    assert!(
        request.contains(r#""reasoning_effort":"max""#),
        "direct compatible request must preserve OpenAI max: {request}"
    );
}

#[test]
fn openai_compatible_model_catalog_refresh_calls_models_endpoint_and_updates_display() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _namespace = EnvVarGuard::set(
        "JCODE_OPENROUTER_CACHE_NAMESPACE",
        "test-openai-compatible-flow",
    );
    let (api_base, request_rx) = spawn_single_response_models_server(
        r#"{
            "object": "list",
            "data": [
                {"id": "live-login-flow-model", "object": "model", "context_length": 131072}
            ]
        }"#,
    );
    let provider = OpenRouterProvider {
        api_base,
        model: Arc::new(RwLock::new("live-login-flow-model".to_string())),
        auth: ProviderAuth::AuthorizationBearer {
            token: "sk-live-catalog".to_string(),
            label: "OPENAI_COMPAT_API_KEY".to_string(),
        },
        supports_provider_features: false,
        supports_model_catalog: true,
        profile_id: None,
        reasoning_effort_support: None,
        static_models: vec!["static-login-flow-fallback".to_string()],
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        ..make_custom_compatible_provider()
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let fetched = rt
        .block_on(provider.refresh_models())
        .expect("refresh fake model catalog");
    assert_eq!(fetched[0].id, "live-login-flow-model");
    assert_eq!(provider.context_window(), 131_072);

    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("capture fake provider request");
    assert!(
        request.starts_with("GET /v1/models "),
        "unexpected catalog request: {request}"
    );
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-live-catalog"),
        "catalog request should include saved API key auth header: {request}"
    );
    assert!(
        request.to_ascii_lowercase().contains("user-agent: jcode/"),
        "catalog requests must include a User-Agent because providers like Cerebras reject bare HTTP clients: {request}"
    );

    let display = provider.available_models_display();
    assert!(display.iter().any(|model| model == "live-login-flow-model"));
    assert!(
        display
            .iter()
            .any(|model| model == "static-login-flow-fallback"),
        "static fallback/default models should remain visible alongside live catalog models: {display:?}"
    );

    let fresh_provider = OpenRouterProvider {
        api_base: provider.api_base.clone(),
        model: Arc::new(RwLock::new("live-login-flow-model".to_string())),
        auth: provider.auth.clone(),
        supports_provider_features: false,
        supports_model_catalog: true,
        profile_id: None,
        reasoning_effort_support: None,
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        ..make_custom_compatible_provider()
    };
    assert_eq!(fresh_provider.context_window(), 131_072);
}

#[test]
fn built_in_openai_compatible_static_models_drop_out_after_live_catalog() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _namespace = EnvVarGuard::set(
        "JCODE_OPENROUTER_CACHE_NAMESPACE",
        "test-cerebras-live-catalog-filters-static-fallback",
    );
    let (api_base, _request_rx) = spawn_single_response_models_server(
        r#"{
            "object": "list",
            "data": [
                {"id": "qwen-3-235b-a22b-instruct-2507", "object": "model"},
                {"id": "zai-glm-4.7", "object": "model"},
                {"id": "gpt-oss-120b", "object": "model"}
            ]
        }"#,
    );
    let provider = OpenRouterProvider {
        api_base,
        auth: ProviderAuth::AuthorizationBearer {
            token: "sk-live-catalog".to_string(),
            label: "CEREBRAS_API_KEY".to_string(),
        },
        supports_provider_features: false,
        supports_model_catalog: true,
        profile_id: Some("cerebras".to_string()),
        static_models: vec!["gpt-oss-120b".to_string(), "zai-glm-4.7".to_string()],
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        ..make_custom_compatible_provider()
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(provider.refresh_models())
        .expect("refresh fake model catalog");

    let display = provider.available_models_display();
    assert!(display.iter().any(|model| model == "gpt-oss-120b"));
    assert!(display.iter().any(|model| model == "zai-glm-4.7"));
    assert!(
        display
            .iter()
            .any(|model| model == "qwen-3-235b-a22b-instruct-2507"),
        "live catalog chat-capable models should remain visible: {display:?}"
    );
}

#[test]
fn configured_swarm_root_effort_covers_all_wire_formats() {
    let unified = make_provider();
    let deepseek = OpenRouterProvider {
        profile_id: Some("deepseek".into()),
        ..make_custom_compatible_provider()
    };
    let openai = OpenRouterProvider {
        profile_id: Some("zai".into()),
        ..make_custom_compatible_provider()
    };
    for mode in ["swarm", "swarm-deep"] {
        for (provider, strict, field, max) in [
            (&unified, false, "reasoning", "xhigh"),
            (&deepseek, false, "reasoning_effort", "max"),
            (&openai, false, "reasoning_effort", "max"),
            (&openai, true, "reasoning_effort", "xhigh"),
        ] {
            provider.set_reasoning_effort(mode).unwrap();
            for (resolved, expected) in [("low", "low"), ("medium", "medium"), ("max", max)] {
                let mut request = serde_json::json!({});
                assert!(provider.apply_resolved_reasoning_effort(&mut request, resolved, strict));
                let wire = if field == "reasoning" {
                    &request[field]["effort"]
                } else {
                    &request[field]
                };
                assert_eq!(wire, expected);
                assert_eq!(provider.reasoning_effort().as_deref(), Some(mode));
            }
            let mut request = serde_json::json!({});
            assert_eq!(
                provider.apply_resolved_reasoning_effort(&mut request, "none", strict),
                field == "reasoning"
            );
            if field == "reasoning" {
                assert_eq!(request[field]["effort"], "none");
            } else {
                assert!(request.get(field).is_none());
            }
        }
    }
    for (effort, expected) in [("minimal", "low"), ("xhigh", "high")] {
        let mut request = serde_json::json!({});
        assert!(deepseek.apply_resolved_reasoning_effort(&mut request, effort, false));
        assert_eq!(request["reasoning_effort"], expected);
    }
}

#[test]
fn configured_swarm_root_effort_reads_real_config() {
    // Run this single test in a child process so changing config cannot race
    // other provider tests or reuse an already-initialized global config cache.
    if std::env::var_os("JCODE_TEST_SWARM_ROOT_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                std::thread::current().name().unwrap(),
                "--nocapture",
            ])
            .env("JCODE_TEST_SWARM_ROOT_CHILD", "1")
            .env("JCODE_SWARM_ROOT_EFFORT", "low")
            .env("JCODE_SWARM_DEEP_ROOT_EFFORT", "none")
            .output()
            .expect("run isolated config test");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    for (mode, expected) in [("swarm", "low"), ("swarm-deep", "none")] {
        let (api_base, request_rx) = spawn_single_response_chat_server();
        let provider = OpenRouterProvider {
            api_base,
            supports_model_catalog: false,
            ..make_provider()
        };
        provider.set_reasoning_effort(mode).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut stream = provider.complete(&[], &[], "test", None).await.unwrap();
            while let Some(event) = stream.next().await {
                event.unwrap();
            }
        });
        let request = request_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            request.contains(&format!(r#""reasoning":{{"effort":"{expected}"}}"#)),
            "{request}"
        );
        assert_eq!(provider.reasoning_effort().as_deref(), Some(mode));
    }
}

#[test]
fn conifer_context_fallback_yields_to_live_and_disk_catalog_without_remapping_aliases() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", temp.path());
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _key = EnvVarGuard::set("CONIFER_API_KEY", "test-conifer-catalog");
    let _namespace = EnvVarGuard::set("JCODE_OPENROUTER_CACHE_NAMESPACE", "test-conifer-1274");
    // Synthetic future catalog deliberately changes latest aliases in both
    // directions and reintroduces the missing Together route with its own limit.
    let (api_base, request_rx) = spawn_single_response_models_server(
        r#"{"data":[
            {"id":"mistral-large-latest","context_window":128000},
            {"id":"mistral-medium-latest","context_window":512000},
            {"id":"mistral-small-latest","context_window":64000},
            {"id":"nemotron-3-ultra-together","context_window":131072}
        ]}"#,
    );
    let make_provider = || {
        let mut provider = OpenRouterProvider::new_openai_compatible_profile_runtime(
            jcode_base::provider_catalog::CONIFER_PROFILE,
        )
        .expect("Conifer provider");
        // Use the real constructor/metadata without sending any vendor requests.
        provider.api_base = api_base.clone();
        provider
    };
    let provider = make_provider();
    for (model, expected) in [
        ("seed-2.0-pro", 256_000),
        ("gemma-4-31b", 128_000),
        ("llama-4-scout", 327_680),
        ("conifer:grok-4.6", 500_000),
        ("mistral-large-latest", 256_000),
        ("mistral-medium-latest", 256_000),
        ("mistral-small-latest", 256_000),
    ] {
        provider.set_model(model).expect("select fallback model");
        assert_eq!(provider.context_window(), expected, "{model}");
    }
    let alias = "nemotron-3-ultra-together";
    assert!(!provider.static_models.iter().any(|model| model == alias));
    provider
        .set_model(&format!("conifer:{alias}"))
        .expect("explicit legacy selection");
    assert_eq!(
        provider.model(),
        alias,
        "never remap to the DeepInfra route"
    );
    assert_eq!(
        provider.context_window(),
        jcode_provider_core::DEFAULT_CONTEXT_LIMIT
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let fetched = rt
        .block_on(provider.refresh_models())
        .expect("refresh Conifer catalog");
    assert!(fetched.iter().any(|model| model.id == alias));
    provider
        .set_model("mistral-large-latest")
        .expect("select another model before checking discovery");
    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("catalog request");
    assert!(request.starts_with("GET /v1/models "));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-conifer-catalog")
    );
    assert!(
        provider
            .available_models_display()
            .iter()
            .any(|model| model == alias)
    );

    let fresh = make_provider();
    assert!(fresh.models_cache.try_read().unwrap().models.is_empty());
    for (model, expected) in [
        ("mistral-large-latest", 128_000),
        ("mistral-medium-latest", 512_000),
        ("mistral-small-latest", 64_000),
        (alias, 131_072),
    ] {
        provider.set_model(model).expect("select live model");
        fresh
            .set_model(model)
            .expect("restore model before in-memory hydration");
        assert_eq!(provider.model(), model);
        assert_eq!(provider.context_window(), expected, "live: {model}");
        assert_eq!(fresh.context_window(), expected, "disk: {model}");
    }
}
