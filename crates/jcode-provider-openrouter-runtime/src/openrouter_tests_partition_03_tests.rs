#[test]
fn named_provider_config_deserializes_nested_extra_body_toml() {
    // Verifies the exact `config.toml` shape documented in the README:
    // a nested `[providers.<name>.extra_body.chat_template_kwargs]` table
    // round-trips into the `serde_json::Value` field correctly.
    let toml_str = r#"
type = "openai-compatible"
base_url = "https://integrate.api.nvidia.com/v1"
api_key_env = "NVIDIA_API_KEY"
default_model = "deepseek-ai/deepseek-v4-flash"

[extra_body.chat_template_kwargs]
thinking = true
reasoning_effort = "high"
"#;
    let profile: jcode_base::config::NamedProviderConfig =
        toml::from_str(toml_str).expect("parse named provider toml");
    let extra = profile.extra_body.as_ref().expect("extra_body present");
    let kwargs = extra
        .get("chat_template_kwargs")
        .and_then(|v| v.as_object())
        .expect("chat_template_kwargs object");
    assert_eq!(kwargs.get("thinking"), Some(&serde_json::json!(true)));
    assert_eq!(
        kwargs.get("reasoning_effort"),
        Some(&serde_json::json!("high"))
    );

    // And the resolver hands it back unchanged when no env override is set.
    let _lock = ENV_LOCK.lock();
    let _guard = EnvVarGuard::remove("JCODE_OPENAI_EXTRA_BODY");
    let resolved =
        OpenRouterProvider::resolve_extra_body(profile.extra_body.as_ref(), "nonexistent.env")
            .expect("resolved extra body");
    assert_eq!(
        resolved
            .get("chat_template_kwargs")
            .and_then(|v| v.get("reasoning_effort")),
        Some(&serde_json::json!("high"))
    );
}

// ============================================================================
// Mid-stream retry rollback (issue #338 gap #3)
// ============================================================================

/// Fake SSE server: the first connection streams partial output then drops the
/// socket mid-stream (transport fault); the second connection streams a clean,
/// complete response.
fn spawn_midstream_fault_then_complete_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake provider server");
    let addr = listener.local_addr().expect("fake provider addr");

    std::thread::spawn(move || {
        // Connection 1: partial output, then abrupt close (no [DONE]).
        {
            let (mut stream, _) = listener.accept().expect("accept first request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");
            let mut request = vec![0u8; 65536];
            let _ = stream.read(&mut request);
            let body = "data: {\"choices\":[{\"delta\":{\"content\":\"partial answer that must not duplicate\"}}]}\n\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{}\r\n",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write partial response");
            stream.flush().expect("flush partial response");
            // Drop without terminating the chunked encoding: the client sees
            // an unexpected EOF mid-stream (transient transport fault).
            drop(stream);
        }

        // Connection 2 (the retry): clean complete response.
        {
            let (mut stream, _) = listener.accept().expect("accept retry request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");
            let mut request = vec![0u8; 65536];
            let _ = stream.read(&mut request);
            let body = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"final answer\"},\"finish_reason\":\"stop\"}]}\n\n",
                "data: [DONE]\n\n",
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write retry response");
        }
    });

    format!("http://{addr}/v1")
}

/// Regression for issue #338 gap #3: a transient transport fault that hits
/// mid-stream, after partial output has already been emitted, must surface a
/// `RetryRollback` before the replayed response so consumers can discard the
/// partial attempt instead of rendering duplicated output.
#[test]
fn midstream_transport_fault_emits_retry_rollback_before_replay() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    rt.block_on(async {
        let api_base = spawn_midstream_fault_then_complete_server();
        let client = reqwest::Client::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<anyhow::Result<StreamEvent>>(64);

        let request = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true,
        });

        super::openrouter_sse_stream::run_stream_with_retries(
            client,
            api_base,
            ProviderAuth::None {
                label: "test".to_string(),
            },
            false,
            new_conversation_id(),
            request,
            tx,
            Arc::new(Mutex::new(None)),
            "test-model".to_string(),
        )
        .await;

        let mut events = Vec::new();
        while let Some(item) = rx.recv().await {
            events.push(item);
        }

        let mut saw_partial = false;
        let mut rollback_after_partial = false;
        let mut final_after_rollback = false;
        let mut duplicate_partial_without_rollback = false;
        for item in &events {
            let Ok(event) = item else {
                panic!("stream surfaced an error instead of retrying: {item:?}");
            };
            match event {
                StreamEvent::TextDelta(text) => {
                    if text.contains("partial answer") {
                        if saw_partial && !rollback_after_partial {
                            duplicate_partial_without_rollback = true;
                        }
                        saw_partial = true;
                    }
                    if text.contains("final answer") {
                        assert!(
                            rollback_after_partial,
                            "replayed response arrived without a RetryRollback after partial output"
                        );
                        final_after_rollback = true;
                    }
                }
                StreamEvent::RetryRollback { .. } => {
                    assert!(
                        saw_partial,
                        "RetryRollback must only be emitted after partial output was streamed"
                    );
                    rollback_after_partial = true;
                }
                _ => {}
            }
        }

        assert!(saw_partial, "first attempt's partial output never arrived");
        assert!(
            rollback_after_partial,
            "no RetryRollback emitted for the mid-stream fault"
        );
        assert!(
            final_after_rollback,
            "retry never delivered the complete response"
        );
        assert!(
            !duplicate_partial_without_rollback,
            "partial output duplicated without an interleaved rollback"
        );
    });
}

/// Issue #352: reasoning effort must follow the *model family*, not just the
/// dedicated `deepseek` profile id. A custom compat endpoint (named profile or
/// generic openai-compatible) serving a DeepSeek model supports `/effort`.
#[test]
fn compat_profile_serving_deepseek_model_supports_reasoning_effort() {
    let provider = make_custom_compatible_provider();

    // Non-DeepSeek model on a custom endpoint: no effort support.
    provider.set_model("some-random-model").unwrap();
    assert!(provider.available_efforts().is_empty());
    assert!(provider.set_reasoning_effort("high").is_err());
    assert_eq!(provider.reasoning_effort(), None);

    // DeepSeek-family model: DeepSeek-style efforts become available.
    provider.set_model("deepseek-v4-flash").unwrap();
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
        .set_reasoning_effort("high")
        .expect("deepseek model on compat endpoint accepts effort");
    assert_eq!(provider.reasoning_effort(), Some("high".to_string()));
}

/// GPT-family reasoning models served by a direct OpenAI-compatible gateway
/// (e.g. OpenCode Zen's `gpt-5.3-codex-spark`) accept the standard OpenAI
/// `reasoning_effort` field, so the effort command must work for them.
#[test]
fn compat_profile_serving_gpt_family_model_supports_reasoning_effort() {
    let provider = make_custom_compatible_provider();

    for model in [
        "gpt-5.3-codex-spark",
        "gpt-5.5",
        "gpt-5.1-codex-mini",
        "o1",
        "o5-mini",
    ] {
        provider.set_model(model).unwrap();
        assert_eq!(
            provider.available_efforts(),
            vec![
                "none",
                "minimal",
                "low",
                "medium",
                "high",
                "xhigh",
                "max",
                "swarm",
                "swarm-deep"
            ],
            "{model} should expose OpenAI effort vocabulary"
        );
        provider
            .set_reasoning_effort("high")
            .unwrap_or_else(|e| panic!("{model} on compat endpoint accepts effort: {e}"));
        assert_eq!(provider.reasoning_effort(), Some("high".to_string()));
        // A direct compatible endpoint receives OpenAI's real max value.
        provider.set_reasoning_effort("max").unwrap();
        assert_eq!(provider.reasoning_effort(), Some("max".to_string()));
    }

    // Explicit config override still wins in the off direction.
    let force_off = OpenRouterProvider {
        reasoning_effort_support: Some(false),
        ..make_custom_compatible_provider()
    };
    force_off.set_model("gpt-5.3-codex-spark").unwrap();
    assert!(force_off.available_efforts().is_empty());
    assert!(force_off.set_reasoning_effort("high").is_err());
}

#[test]
fn compatible_model_switch_clears_an_effort_invalid_for_the_new_vocabulary() {
    let provider = make_custom_compatible_provider();
    provider.set_model("gpt-5.5").unwrap();
    provider.set_reasoning_effort("minimal").unwrap();
    provider.set_model("deepseek-v4").unwrap();
    assert_eq!(provider.reasoning_effort(), None);
    assert!(
        provider.set_reasoning_effort("minimal").is_err(),
        "DeepSeek must reject rather than silently promote minimal to max"
    );
}

/// Issue #352: named-profile config can override effort support explicitly in
/// both directions.
#[test]
fn named_profile_supports_reasoning_effort_config_override() {
    let force_on = OpenRouterProvider {
        reasoning_effort_support: Some(true),
        ..make_custom_compatible_provider()
    };
    force_on.set_model("not-a-deepseek-model").unwrap();
    assert_eq!(
        force_on.available_efforts(),
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
    force_on
        .set_reasoning_effort("medium")
        .expect("explicit supports_reasoning_effort=true enables effort");
    assert_eq!(force_on.reasoning_effort(), Some("medium".to_string()));

    let force_off = OpenRouterProvider {
        reasoning_effort_support: Some(false),
        ..make_custom_compatible_provider()
    };
    force_off.set_model("deepseek-v4-flash").unwrap();
    assert!(force_off.available_efforts().is_empty());
    assert!(
        force_off.set_reasoning_effort("high").is_err(),
        "explicit supports_reasoning_effort=false suppresses model auto-detection"
    );
}

/// Issue #352: named profiles construct with the user's configured
/// `openai_reasoning_effort` when the profile supports effort, instead of
/// silently ignoring the config.
#[test]
fn named_profile_construction_reads_openai_reasoning_effort_config() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");

    let config = jcode_base::config::NamedProviderConfig {
        base_url: "https://compat.example.test/v1".to_string(),
        api_key: Some("test".to_string()),
        default_model: Some("deepseek-v4".to_string()),
        supports_reasoning_effort: Some(true),
        ..Default::default()
    };

    let provider =
        OpenRouterProvider::new_named_openai_compatible("custom", &config).expect("provider");
    // The config default is only applied when openai_reasoning_effort is set;
    // with no config value the provider starts with no effort but still
    // supports setting one.
    let initial = provider.reasoning_effort();
    let configured = jcode_base::config::config()
        .provider
        .openai_reasoning_effort
        .clone();
    match configured {
        Some(_) => assert!(initial.is_some(), "configured effort must be honored"),
        None => assert_eq!(initial, None),
    }
    provider
        .set_reasoning_effort("max")
        .expect("explicitly-enabled profile accepts effort");
}

#[test]
fn named_profile_can_disable_reasoning_model_name_heuristics() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let config = jcode_base::config::NamedProviderConfig {
        base_url: "https://compat.example.test/v1".to_string(),
        api_key: Some("test".to_string()),
        default_model: Some("gpt-5.5-enterprise".to_string()),
        disable_reasoning_heuristics: true,
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("enterprise", &config).unwrap();
    assert!(provider.available_efforts().is_empty());
    assert!(provider.set_reasoning_effort("high").is_err());
}

#[test]
fn named_profile_model_reasoning_overrides_capability_and_default_effort() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let config = jcode_base::config::NamedProviderConfig {
        base_url: "https://compat.example.test/v1".to_string(),
        api_key: Some("test".to_string()),
        default_model: Some("reasoning-custom".to_string()),
        disable_reasoning_heuristics: true,
        models: vec![
            jcode_base::config::NamedProviderModelConfig {
                id: "reasoning-custom".to_string(),
                reasoning: Some(true),
                reasoning_effort: Some("high".to_string()),
                ..Default::default()
            },
            jcode_base::config::NamedProviderModelConfig {
                id: "gpt-5-disabled".to_string(),
                reasoning: Some(false),
                ..Default::default()
            },
            jcode_base::config::NamedProviderModelConfig {
                id: "reasoning-mini".to_string(),
                reasoning: Some(true),
                reasoning_effort: Some("low".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("custom", &config).unwrap();
    assert_eq!(provider.reasoning_effort(), Some("high".to_string()));
    assert!(provider.available_efforts().contains(&"xhigh"));

    provider.set_model("gpt-5-disabled").unwrap();
    assert!(provider.available_efforts().is_empty());
    assert_eq!(provider.reasoning_effort(), None);

    provider.set_model("reasoning-mini").unwrap();
    assert_eq!(provider.reasoning_effort(), Some("low".to_string()));
}

/// Regression: when the shared interactive server boots an `OpenRouterProvider`
/// without binding `profile_id` (the deferred-auth bootstrap path used by the
/// TUI server), a session-routing `<name>:` prefix for a *user-defined* named
/// provider profile (`[providers.<name>]` in config.toml) must still be
/// stripped before the model id reaches the upstream API. Without this, a
/// resumed/new TUI session sends e.g. `cline:cline-pass/qwen3.7-max` verbatim
/// and the gateway rejects it with 404 model_not_found, even though headless
/// `jcode run` (which binds profile_id in-process) works fine.
#[test]
fn user_named_profile_prefix_is_stripped_even_without_profile_id() {
    let _lock = ENV_LOCK.lock();
    let temp = TempDir::new().expect("create temp home");
    let jcode_home = temp.path().join("jcode-home");
    let _jcode_home = EnvVarGuard::set("JCODE_HOME", &jcode_home);
    let _home = EnvVarGuard::set("HOME", temp.path());
    let _appdata = EnvVarGuard::set("APPDATA", temp.path().join("AppData").join("Roaming"));
    let _env = isolate_openrouter_autodetect_env();
    let (api_base, request_rx) = spawn_single_response_chat_server();

    std::fs::create_dir_all(&jcode_home).expect("create test config dir");
    std::fs::write(
        jcode_home.join("config.toml"),
        r#"
[provider]
default_provider = "cline"

[providers.cline]
type = "openai-compatible"
base_url = "https://api.cline.bot/api/v1"
api_key_env = "TEST_CLINE_KEY"
default_model = "cline-pass/qwen3.7-max"
model_catalog = false
"#,
    )
    .expect("write test config");
    jcode_base::config::invalidate_config_cache();

    // Simulate the shared-server provider slot: a generic OpenAI-compatible
    // provider with NO profile_id bound (deferred-auth bootstrap path).
    let provider = OpenRouterProvider {
        api_base,
        profile_id: None,
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    // Session restore / default-model routing hands the provider a
    // `<name>:<model>` spec for the user profile.
    provider
        .set_model("cline:cline-pass/qwen3.7-max")
        .expect("set prefixed model");

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
            if event.is_err() {
                break;
            }
        }
    });

    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("capture fake provider request");
    let body = parse_captured_request_body(&request);
    assert_eq!(
        body.get("model").and_then(|v| v.as_str()),
        Some("cline-pass/qwen3.7-max"),
        "user-defined named profile prefix must be stripped from the outbound model id; got: {request}"
    );

    jcode_base::config::invalidate_config_cache();
}
include!("openrouter_stream_options_tests.rs");

/// A named OpenAI-compatible profile keeps the stable machine-facing
/// `Provider::name()` and surfaces its identity through `display_name()`.
///
/// Issue #691 proposed returning `profile_id` from `name()`. That would regress
/// the contract documented on the trait and settled in #329: billing, routing,
/// and provider-class matching key off `name()`, so it must stay constant for a
/// provider class, while user-visible labels come from `display_name()`. This
/// pins both halves so the split cannot be undone by accident.
#[test]
fn named_openai_compatible_provider_keeps_stable_name_and_profile_display_name() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "https://llm.example.com/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        default_model: Some("example-model".to_string()),
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("example-compat", &profile)
        .expect("named profile should initialize");

    // Machine-facing identity: stable per provider class.
    assert_eq!(
        Provider::name(&provider),
        "openrouter",
        "billing/routing key off name(); it must not become the profile id"
    );
    // User-facing identity: the profile the user configured.
    assert_eq!(provider.runtime_display_name(), "example-compat");
    assert_eq!(Provider::display_name(&provider), "example-compat");
}

/// Issue #1167: OpenCode Go/Zen require a stable per-conversation
/// `x-opencode-session` header; other OpenAI-compatible hosts must not get it.
#[test]
fn opencode_session_header_only_for_opencode_hosts() {
    assert!(is_opencode_api_base("https://opencode.ai/zen/go/v1"));
    assert!(is_opencode_api_base("https://opencode.ai/zen/v1"));
    assert!(is_opencode_api_base("https://api.opencode.ai/v1"));
    assert!(!is_opencode_api_base("https://openrouter.ai/api/v1"));
    assert!(!is_opencode_api_base("https://api.deepseek.com/v1"));
    assert!(!is_opencode_api_base("not a url"));

    let client = reqwest::Client::new();
    let req = apply_opencode_session_header(
        client.post("https://opencode.ai/zen/go/v1/chat/completions"),
        "https://opencode.ai/zen/go/v1",
        "conv-123",
    )
    .build()
    .unwrap();
    assert_eq!(
        req.headers()
            .get(OPENCODE_SESSION_HEADER)
            .and_then(|v| v.to_str().ok()),
        Some("conv-123")
    );

    let req = apply_opencode_session_header(
        client.post("https://openrouter.ai/api/v1/chat/completions"),
        "https://openrouter.ai/api/v1",
        "conv-123",
    )
    .build()
    .unwrap();
    assert!(req.headers().get(OPENCODE_SESSION_HEADER).is_none());
}

#[test]
fn opencode_session_ids_are_uuids_and_unique() {
    let a = new_conversation_id();
    let b = new_conversation_id();
    assert_ne!(a, b);
    assert!(uuid::Uuid::parse_str(&a).is_ok());
}

/// Wire-level check for issue #1167: a real `chat/completions` request whose
/// api_base host is `opencode.ai` carries `x-opencode-session`, and a
/// request to another host does not. The DNS override points the hostname at
/// a local listener, so the full stream path (including retries) is exercised.
fn spawn_header_capturing_server() -> (std::net::SocketAddr, std::sync::mpsc::Receiver<String>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut buf = vec![0u8; 65536];
        let n = stream.read(&mut buf).unwrap_or(0);
        let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });
    (addr, rx)
}

fn captured_request_for_host(host: &str, conversation_id: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let (addr, rx) = spawn_header_capturing_server();
        let client = reqwest::Client::builder()
            .resolve(host, addr)
            .build()
            .expect("client");
        let api_base = format!("http://{host}:{}/zen/go/v1", addr.port());
        let (tx, mut events) = tokio::sync::mpsc::channel::<anyhow::Result<StreamEvent>>(64);
        super::openrouter_sse_stream::run_stream_with_retries(
            client,
            api_base,
            ProviderAuth::None {
                label: "test".to_string(),
            },
            false,
            conversation_id.to_string(),
            serde_json::json!({"model": "m", "messages": [], "stream": true}),
            tx,
            Arc::new(Mutex::new(None)),
            "m".to_string(),
        )
        .await;
        while events.recv().await.is_some() {}
        rx.recv_timeout(Duration::from_secs(5))
            .expect("server captured request")
    })
}

#[test]
fn opencode_session_header_is_sent_on_the_wire_only_to_opencode_hosts() {
    let raw = captured_request_for_host("opencode.ai", "conv-wire-1167").to_ascii_lowercase();
    assert!(
        raw.contains("x-opencode-session: conv-wire-1167"),
        "opencode.ai request lacked the header:\n{raw}"
    );

    let raw = captured_request_for_host("example.test", "conv-wire-1167").to_ascii_lowercase();
    assert!(
        !raw.contains("x-opencode-session"),
        "non-opencode host received the header:\n{raw}"
    );
}

#[test]
fn direct_deepseek_profile_uses_1m_context_for_listed_models_when_catalog_is_absent() {
    for model in ["deepseek-flash", "deepseek-v4-pro"] {
        let _lock = ENV_LOCK.lock();
        let _base = EnvVarGuard::set("JCODE_OPENROUTER_API_BASE", "https://api.deepseek.com");
        let _key_name = EnvVarGuard::set("JCODE_OPENROUTER_API_KEY_NAME", "DEEPSEEK_API_KEY");
        let _api_key = EnvVarGuard::set("DEEPSEEK_API_KEY", "test");
        let _namespace = EnvVarGuard::set("JCODE_OPENROUTER_CACHE_NAMESPACE", "deepseek");
        let _model = EnvVarGuard::set("JCODE_OPENROUTER_MODEL", model);
        let _catalog = EnvVarGuard::set("JCODE_OPENROUTER_MODEL_CATALOG", "0");

        let provider = OpenRouterProvider::new().expect("provider");

        assert_eq!(provider.context_window(), 1_000_000, "{model}");
    }
}

#[test]
fn grok_build_subscription_request_spoofs_grok_cli_and_uses_oidc_bearer() {
    let _lock = ENV_LOCK.lock();
    let grok_home = TempDir::new().expect("grok home");
    std::fs::write(
        grok_home.path().join("auth.json"),
        format!(
            r#"{{"https://auth.x.ai::{}": {{"key":"oidc-access","auth_mode":"oidc","expires_at":"2999-01-01T00:00:00Z"}}}}"#,
            jcode_base::auth::grok_build::OAUTH_CLIENT_ID
        ),
    )
    .expect("auth.json");
    let _home = EnvVarGuard::set("GROK_HOME", grok_home.path());
    let _deploy = EnvVarGuard::remove("GROK_DEPLOYMENT_KEY");
    let _version = EnvVarGuard::set("JCODE_GROK_CLI_VERSION", "9.8.7");
    let (addr, rx) = spawn_header_capturing_server();
    let _base = EnvVarGuard::set(
        "GROK_CLI_CHAT_PROXY_BASE_URL",
        format!("http://127.0.0.1:{}/v1", addr.port()),
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let raw = rt.block_on(async {
        let provider = OpenRouterProvider::new_grok_build_subscription("grok-4.6");
        assert_eq!(provider.context_window(), 500_000);
        let tools = vec![ToolDefinition {
            name: "bash".to_string(),
            description: "run".to_string(),
            input_schema: serde_json::json!({"type":"object","properties":{"cmd":{"type":"string"}}}),
        }];
        let mut stream = provider
            .complete(&[Message::user("hello")], &tools, "sys", None)
            .await
            .expect("stream");
        while stream.next().await.is_some() {}
        rx.recv_timeout(Duration::from_secs(5))
            .expect("server captured request")
    });
    let lower = raw.to_ascii_lowercase();
    assert!(lower.starts_with("post /v1/chat/completions "), "{raw}");
    for expected in [
        "authorization: bearer oidc-access",
        "user-agent: grok-cli/9.8.7",
        "x-xai-token-auth: xai-grok-cli",
        "x-grok-client-version: 9.8.7",
        "x-grok-client-identifier: grok-shell",
        "x-grok-client-surface: cli",
        "x-grok-model-override: grok-4.6",
        "x-grok-conv-id: ",
        "x-grok-req-id: ",
    ] {
        assert!(lower.contains(expected), "missing `{expected}` in:\n{raw}");
    }
    assert!(!lower.contains("user-agent: jcode"), "{raw}");
    assert!(!lower.contains("http-referer"), "{raw}");
    let body: serde_json::Value =
        serde_json::from_str(raw.split("\r\n\r\n").nth(1).expect("body")).expect("json body");
    assert_eq!(body["model"], "grok-4.6");
    assert_eq!(body["stream"], true);
    assert_eq!(body["tools"][0]["function"]["name"], "bash");
    assert_eq!(body["messages"][0]["role"], "system");
    assert!(body.get("reasoning_effort").is_none());
}
