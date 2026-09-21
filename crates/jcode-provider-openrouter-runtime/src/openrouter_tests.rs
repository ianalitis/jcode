use super::*;
use bytes::Bytes;
use futures::StreamExt;
use jcode_provider_openrouter::stream::OpenRouterStream;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

pub(crate) struct SharedEnvLock;

pub(crate) static ENV_LOCK: SharedEnvLock = SharedEnvLock;

impl SharedEnvLock {
    /// Acquire the process-global test env lock.
    ///
    /// This recovers from a poisoned mutex (`into_inner`) instead of
    /// propagating the `PoisonError`. The env guard only protects shared
    /// process env state, so a panic in one test must not cascade into a
    /// flood of unrelated `PoisonError` failures across every other test
    /// that takes this lock.
    pub(crate) fn lock(&self) -> jcode_base::storage::TestEnvGuard {
        jcode_base::storage::lock_test_env()
    }
}

pub(crate) struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    pub(crate) fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        jcode_base::env::set_var(key, value);
        Self { key, previous }
    }

    pub(crate) fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        jcode_base::env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            jcode_base::env::set_var(self.key, previous);
        } else {
            jcode_base::env::remove_var(self.key);
        }
    }
}

fn test_config_dir(temp: &TempDir) -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        temp.path().join("Library").join("Application Support")
    }
    #[cfg(target_os = "windows")]
    {
        temp.path().join("AppData").join("Roaming")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        temp.path().to_path_buf()
    }
}

fn write_test_api_key(temp: &TempDir, env_file: &str, env_key: &str, value: &str) {
    // Write where the provider actually reads. `app_config_dir()` prefers
    // `JCODE_HOME`, which `scripts/dev_cargo.sh` sets for test-state isolation,
    // so the `HOME`/`XDG_CONFIG_HOME` override these tests set is not
    // sufficient on its own. Tests pin `JCODE_HOME` at their temp dir with
    // `pin_test_jcode_home` so the resolved directory stays private per test.
    let config_dir = test_app_config_dir(temp);
    std::fs::create_dir_all(&config_dir).expect("create test config dir");
    std::fs::write(config_dir.join(env_file), format!("{env_key}={value}\n"))
        .expect("write test api key");
}

/// The config directory the provider resolves for a test temp dir. Mirrors
/// `write_test_api_key`, so tests that write other config markers land beside
/// the keys instead of in a directory nothing reads.
fn test_app_config_dir(temp: &TempDir) -> std::path::PathBuf {
    jcode_base::storage::app_config_dir().unwrap_or_else(|_| test_config_dir(temp).join("jcode"))
}

/// Pin `JCODE_HOME` at the test's temp dir so config resolution is private and
/// identical whether or not the harness sets `JCODE_HOME` for isolation.
fn pin_test_jcode_home(temp: &TempDir) -> EnvVarGuard {
    EnvVarGuard::set("JCODE_HOME", temp.path())
}

fn isolate_openrouter_autodetect_env() -> Vec<EnvVarGuard> {
    let mut guards = vec![
        EnvVarGuard::remove("JCODE_OPENROUTER_API_BASE"),
        EnvVarGuard::remove("JCODE_OPENROUTER_API_KEY_NAME"),
        EnvVarGuard::remove("JCODE_OPENROUTER_ENV_FILE"),
        EnvVarGuard::remove("JCODE_OPENROUTER_DYNAMIC_BEARER_PROVIDER"),
        EnvVarGuard::remove("JCODE_OPENROUTER_MODEL"),
        EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE"),
        EnvVarGuard::remove("JCODE_OPENROUTER_ALLOW_NO_AUTH"),
        EnvVarGuard::remove("JCODE_OPENROUTER_TRANSPORT_STATE"),
        EnvVarGuard::remove("JCODE_OPENROUTER_PROVIDER_FEATURES"),
        EnvVarGuard::remove("JCODE_OPENROUTER_MODEL_CATALOG"),
        EnvVarGuard::remove("JCODE_OPENROUTER_AUTH_HEADER"),
        EnvVarGuard::remove("JCODE_OPENROUTER_AUTH_HEADER_NAME"),
        EnvVarGuard::remove("JCODE_OPENROUTER_STATIC_MODELS"),
        EnvVarGuard::remove("JCODE_ACTIVE_PROVIDER"),
        EnvVarGuard::remove("JCODE_RUNTIME_PROVIDER"),
        EnvVarGuard::remove("JCODE_NAMED_PROVIDER_PROFILE"),
        EnvVarGuard::remove("JCODE_PROVIDER_PROFILE_NAME"),
        EnvVarGuard::remove("JCODE_PROVIDER_PROFILE_ACTIVE"),
        EnvVarGuard::remove("JCODE_OPENAI_COMPAT_API_BASE"),
        EnvVarGuard::remove("JCODE_OPENAI_COMPAT_API_KEY_NAME"),
        EnvVarGuard::remove("JCODE_OPENAI_COMPAT_ENV_FILE"),
        EnvVarGuard::remove("JCODE_OPENAI_COMPAT_SETUP_URL"),
        EnvVarGuard::remove("JCODE_OPENAI_COMPAT_DEFAULT_MODEL"),
        EnvVarGuard::remove("JCODE_OPENAI_COMPAT_LOCAL_ENABLED"),
    ];
    guards.extend(
        jcode_base::provider_catalog::openai_compatible_profiles()
            .iter()
            .map(|profile| EnvVarGuard::remove(profile.api_key_env)),
    );
    guards
}

#[test]
fn test_has_credentials() {
    let _has_creds = OpenRouterProvider::has_credentials();
}

#[test]
fn openai_compatible_models_endpoint_allows_minimal_model_objects() {
    let parsed = parse_openai_compatible_models_response(
        r#"{
            "object": "list",
            "data": [
                {"id": "glm-51-nvfp4", "object": "model", "created": null, "owned_by": null},
                {"id": "gte-qwen2-7b", "object": "model"}
            ]
        }"#,
    )
    .expect("minimal OpenAI-compatible /models response should parse");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].id, "glm-51-nvfp4");
    assert_eq!(parsed[0].name, "");
}

#[test]
fn openai_compatible_models_endpoint_allows_chutes_numeric_pricing() {
    let parsed = parse_openai_compatible_models_response(
        r#"{
            "object": "list",
            "data": [{
                "id": "Qwen/Qwen3-32B-TEE",
                "root": "Qwen/Qwen3-32B-FP8",
                "price": {
                    "input": {"tao": 0.0002439746644509701, "usd": 0.08},
                    "output": {"tao": 0.0007319239933529102, "usd": 0.24}
                },
                "object": "model",
                "parent": null,
                "created": 1778439139,
                "pricing": {
                    "prompt": 0.08,
                    "completion": 0.24,
                    "input_cache_read": 0.04
                },
                "owned_by": "sglang",
                "context_length": 40960,
                "supported_features": ["json_mode", "tools"]
            }]
        }"#,
    )
    .expect("Chutes /models response with numeric pricing should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "Qwen/Qwen3-32B-TEE");
    assert_eq!(parsed[0].pricing.prompt.as_deref(), Some("0.08"));
    assert_eq!(parsed[0].pricing.completion.as_deref(), Some("0.24"));
    assert_eq!(parsed[0].pricing.input_cache_read.as_deref(), Some("0.04"));
}

#[test]
fn openai_compatible_models_endpoint_allows_together_top_level_array() {
    let parsed = parse_openai_compatible_models_response(
        r#"[
            {
                "id": "Austism/chronos-hermes-13b",
                "object": "model",
                "created": 1692896905,
                "type": "chat",
                "display_name": "Chronos Hermes (13B)",
                "context_length": 2048,
                "pricing": {
                    "input": 0.3,
                    "output": 0.3,
                    "cached_input": 0.2
                }
            }
        ]"#,
    )
    .expect("Together /models top-level array should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "Austism/chronos-hermes-13b");
    assert_eq!(parsed[0].name, "Chronos Hermes (13B)");
    assert_eq!(parsed[0].context_length, Some(2048));
    assert_eq!(parsed[0].pricing.prompt.as_deref(), Some("0.3"));
    assert_eq!(parsed[0].pricing.completion.as_deref(), Some("0.3"));
    assert_eq!(parsed[0].pricing.input_cache_read.as_deref(), Some("0.2"));
}

#[test]
fn openai_compatible_models_endpoint_allows_models_array_with_name_ids() {
    let parsed = parse_openai_compatible_models_response(
        r#"{
            "models": [{
                "name": "accounts/fireworks/models/example",
                "displayName": "Example Fireworks Model",
                "contextLength": 8192
            }]
        }"#,
    )
    .expect("models array with name-based identifiers should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "accounts/fireworks/models/example");
    assert_eq!(parsed[0].name, "accounts/fireworks/models/example");
    assert_eq!(parsed[0].context_length, Some(8192));
}

#[test]
fn openai_compatible_models_endpoint_reads_llamacpp_meta_n_ctx() {
    // llama.cpp's /v1/models only exposes the context window inside `meta`
    // (issue #447). The `data` entry mirrors llama.cpp's response shape.
    let parsed = parse_openai_compatible_models_response(
        r#"{
            "object": "list",
            "data": [{
                "id": "unsloth/gemma-4-31B-it-UD-Q8_K_XL",
                "object": "model",
                "created": 1783253170,
                "owned_by": "llamacpp",
                "meta": {
                    "vocab_type": 2,
                    "n_vocab": 262144,
                    "n_ctx": 262144,
                    "n_ctx_train": 262144,
                    "n_embd": 5376
                }
            }]
        }"#,
    )
    .expect("llama.cpp /v1/models response should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "unsloth/gemma-4-31B-it-UD-Q8_K_XL");
    assert_eq!(parsed[0].context_length, Some(262144));
}

#[test]
fn named_openai_compatible_provider_sets_catalog_cache_namespace() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let _key = EnvVarGuard::set("TEST_NAMED_COMPAT_KEY", "test-key");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "https://llm.example.com/v1".to_string(),
        api_key_env: Some("TEST_NAMED_COMPAT_KEY".to_string()),
        model_catalog: true,
        default_model: Some("example-model".to_string()),
        ..Default::default()
    };

    let _provider = OpenRouterProvider::new_named_openai_compatible("example-compat", &profile)
        .expect("named profile should initialize");

    assert_eq!(
        std::env::var("JCODE_OPENROUTER_CACHE_NAMESPACE").as_deref(),
        Ok("example-compat")
    );
}

#[test]
fn named_openai_compatible_provider_exposes_static_models_as_routes() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    let _key = EnvVarGuard::set("TEST_NAMED_COMPAT_KEY", "test-key");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "https://llm.example.com/v1".to_string(),
        api_key_env: Some("TEST_NAMED_COMPAT_KEY".to_string()),
        model_catalog: true,
        default_model: Some("glm-51-nvfp4".to_string()),
        models: vec![jcode_base::config::NamedProviderModelConfig {
            id: "glm-51-nvfp4".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("comtegra-test", &profile)
        .expect("named profile should initialize");
    let routes = provider.model_routes();

    assert!(routes.iter().any(|route| {
        route.model == "glm-51-nvfp4"
            && route.api_method == "openai-compatible:comtegra-test"
            && route.available
    }));
}

#[test]
fn direct_openai_compatible_provider_advertises_image_input_support() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "http://localhost:1234/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        default_model: Some("local-vision-model".to_string()),
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("local-compat", &profile)
        .expect("local named profile should initialize without auth");

    assert!(provider.supports_image_input());
}

#[test]
fn named_openai_compatible_provider_uses_per_model_image_input_support() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "http://localhost:1234/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        default_model: Some("vision-model".to_string()),
        models: vec![
            jcode_base::config::NamedProviderModelConfig {
                id: "vision-model".to_string(),
                input: vec!["text".to_string(), "image".to_string()],
                ..Default::default()
            },
            jcode_base::config::NamedProviderModelConfig {
                id: "text-model".to_string(),
                input: vec!["text".to_string()],
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("local-compat", &profile)
        .expect("local named profile should initialize without auth");

    assert!(provider.supports_image_input());
    provider.set_model("text-model").expect("switch model");
    assert!(!provider.supports_image_input());
    provider
        .set_model("local-compat:vision-model")
        .expect("switch using qualified model");
    assert!(provider.supports_image_input());
}

#[test]
fn named_openai_compatible_model_with_omitted_input_preserves_image_support() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "http://localhost:1234/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        default_model: Some("text-model".to_string()),
        models: vec![jcode_base::config::NamedProviderModelConfig {
            id: "text-model".to_string(),
            context_window: Some(200_000),
            ..Default::default()
        }],
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("local-compat", &profile)
        .expect("local named profile should initialize without auth");

    assert!(provider.supports_image_input());
}

#[test]
fn named_openai_compatible_model_with_empty_input_preserves_image_support() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");

    let profile = jcode_base::config::NamedProviderConfig {
        base_url: "http://localhost:1234/v1".to_string(),
        auth: jcode_base::config::NamedProviderAuth::None,
        default_model: Some("text-model".to_string()),
        models: vec![jcode_base::config::NamedProviderModelConfig {
            id: "text-model".to_string(),
            reasoning: None,
            reasoning_effort: None,
            input: Vec::new(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let provider = OpenRouterProvider::new_named_openai_compatible("local-compat", &profile)
        .expect("local named profile should initialize without auth");

    assert!(provider.supports_image_input());
}

#[test]
fn direct_deepseek_profile_unknown_model_does_not_advertise_image_input_support() {
    let provider = OpenRouterProvider {
        profile_id: Some("deepseek".to_string()),
        supports_provider_features: false,
        ..make_custom_compatible_provider()
    };

    assert!(!provider.supports_image_input());
}

#[test]
fn deepseek_image_input_capability_matrix() {
    for (profile, model, provider_features, expected) in [
        ("deepseek", "deepseek-flash", false, true),
        ("deepseek", "deepseek-v4-flash", false, true),
        ("deepseek", "deepseek-v4-flash-vision-exp", false, true),
        ("DeepSeek", "DEEPSEEK-FLASH", false, true),
        ("deepseek", "deepseek:deepseek-flash", false, true),
        ("deepseek", "deepseek-v4-pro", false, false),
        ("deepseek", "deepseek-pro", false, false),
        ("deepseek", "deepseek-chat", false, false),
        ("deepseek", "deepseek-reasoner", false, false),
        ("deepseek", "unknown", false, false),
        ("deepseek", "deepseek-v4-flash-free", false, false),
        ("deepseek", "deepseek-flash-future", false, false),
        ("deepseek", "deepseek/deepseek-flash", false, false),
        ("zai", "deepseek-flash", false, false),
        ("zai", "glm-5", false, false),
        ("openrouter", "deepseek-flash", true, false),
        ("custom", "unknown", false, true),
    ] {
        let provider = OpenRouterProvider {
            profile_id: Some(profile.to_string()),
            model: Arc::new(RwLock::new(model.to_string())),
            supports_provider_features: provider_features,
            ..make_custom_compatible_provider()
        };
        assert_eq!(
            provider.supports_image_input(),
            expected,
            "profile={profile}, model={model}"
        );
    }
}

#[test]
fn deepseek_image_input_explicit_model_inputs_take_precedence() {
    let _lock = ENV_LOCK.lock();
    let _namespace = EnvVarGuard::remove("JCODE_OPENROUTER_CACHE_NAMESPACE");
    for profile_id in ["deepseek", "zai"] {
        let profile = jcode_base::config::NamedProviderConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            auth: jcode_base::config::NamedProviderAuth::None,
            default_model: Some("deepseek-flash".to_string()),
            models: vec![
                jcode_base::config::NamedProviderModelConfig {
                    id: "deepseek-flash".to_string(),
                    input: vec!["text".to_string()],
                    ..Default::default()
                },
                jcode_base::config::NamedProviderModelConfig {
                    id: "deepseek-v4-pro".to_string(),
                    input: vec!["text".to_string(), "image".to_string()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let provider = OpenRouterProvider::new_named_openai_compatible(profile_id, &profile)
            .expect("named profile should initialize without auth");
        assert!(
            !provider.supports_image_input(),
            "{profile_id}: explicit text-only Flash"
        );
        provider.set_model("deepseek-v4-pro").unwrap();
        assert!(
            provider.supports_image_input(),
            "{profile_id}: explicit image-capable Pro"
        );
    }
}

#[test]
fn deepseek_image_input_captured_requests_preserve_only_allowed_pixels() {
    let _lock = ENV_LOCK.lock();
    // A real 1x1 PNG, retained byte-for-byte in the outbound data URL.
    let pixels = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aK1cAAAAASUVORK5CYII=";
    let messages = vec![Message {
        role: Role::User,
        content: vec![
            ContentBlock::Text {
                text: "describe this".to_string(),
                cache_control: None,
            },
            ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: pixels.to_string(),
            },
        ],
        timestamp: None,
        tool_duration_ms: None,
    }];
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    for (profile, model, override_support, expected) in [
        ("deepseek", "deepseek-flash", None, true),
        ("deepseek", "deepseek-v4-flash", None, true),
        ("deepseek", "deepseek-v4-flash-vision-exp", None, true),
        ("deepseek", "deepseek-v4-pro", None, false),
        ("deepseek", "unknown", None, false),
        ("zai", "deepseek-flash", None, false),
        ("deepseek", "deepseek-flash", Some(false), false),
        ("deepseek", "deepseek-v4-pro", Some(true), true),
    ] {
        let (api_base, request_rx) = spawn_single_response_chat_server();
        let provider = OpenRouterProvider {
            api_base,
            model: Arc::new(RwLock::new(model.to_string())),
            profile_id: Some(profile.to_string()),
            supports_provider_features: false,
            supports_model_catalog: false,
            static_image_input_support: override_support
                .map(|value| HashMap::from([(model.to_string(), value)]))
                .unwrap_or_default(),
            ..make_custom_compatible_provider()
        };
        rt.block_on(async {
            let mut stream = provider.complete(&messages, &[], "", None).await.unwrap();
            while let Some(event) = stream.next().await {
                event.expect("local fixture stream should succeed");
            }
        });
        let request = request_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let body = parse_captured_request_body(&request);
        assert_eq!(body["model"], model);
        let parts = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|message| message["content"].as_array())
            .flatten()
            .filter(|part| part["type"] == "image_url")
            .collect::<Vec<_>>();
        assert_eq!(
            parts.len(),
            usize::from(expected),
            "{profile}/{model}: {body}"
        );
        if expected {
            assert_eq!(
                parts[0]["image_url"]["url"],
                format!("data:image/png;base64,{pixels}")
            );
            assert!(!request.contains("Image omitted"), "{body}");
        } else {
            assert!(request.contains("Image omitted"), "{body}");
            assert!(!request.contains(pixels), "{body}");
        }
    }
}

#[test]
fn direct_zai_profile_does_not_advertise_image_input_support() {
    let provider = OpenRouterProvider {
        profile_id: Some("zai".to_string()),
        supports_provider_features: false,
        ..make_custom_compatible_provider()
    };

    assert!(!provider.supports_image_input());
}

#[test]
fn direct_deepseek_profile_omits_image_url_parts() {
    let _lock = ENV_LOCK.lock();
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        profile_id: Some("deepseek".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };
    let messages = vec![Message {
        role: Role::User,
        content: vec![
            ContentBlock::Text {
                text: "describe this".to_string(),
                cache_control: None,
            },
            ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: "aW1hZ2U=".to_string(),
            },
        ],
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
    assert!(
        !request.contains(r#""type":"image_url""#),
        "DeepSeek request must not contain unsupported image_url content parts: {request}"
    );
    assert!(
        request.contains("Image omitted"),
        "DeepSeek request should preserve a textual placeholder for omitted images: {request}"
    );
}

/// Extract the JSON request body from a captured raw HTTP request.
fn parse_captured_request_body(request: &str) -> serde_json::Value {
    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or(request);
    serde_json::from_str(body)
        .unwrap_or_else(|err| panic!("captured request body should be JSON ({err}): {body}"))
}

/// Regression for issue #321: when an assistant turn is interrupted mid-thinking
/// on a direct OpenAI-compatible provider that does not support reasoning replay
/// (e.g. DeepSeek), the persisted assistant message contains only a `Reasoning`
/// block. The request builder must not emit an assistant message that has
/// neither `content` nor `tool_calls`, otherwise the provider rejects the whole
/// request with 400 "Invalid assistant message: content or tool_calls must be
/// set" and the session can never recover.
#[test]
fn interrupted_reasoning_only_assistant_message_is_not_sent_empty() {
    let _lock = ENV_LOCK.lock();
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        profile_id: Some("deepseek".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    let messages = vec![
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "do a thing".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        // Assistant turn that was interrupted while only reasoning had streamed,
        // so it carries a Reasoning block but no text or tool calls.
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Reasoning {
                text: "thinking about the request".to_string(),
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "actually do this instead".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
    ];

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
    let api_messages = body
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("request should contain messages array");

    for msg in api_messages {
        if msg.get("role").and_then(|v| v.as_str()) != Some("assistant") {
            continue;
        }
        let has_content = msg
            .get("content")
            .map(|v| !v.is_null() && v.as_str().map(|s| !s.is_empty()).unwrap_or(true))
            .unwrap_or(false);
        let has_tool_calls = msg
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|calls| !calls.is_empty())
            .unwrap_or(false);
        assert!(
            has_content || has_tool_calls,
            "assistant message must carry content or tool_calls (issue #321); got: {msg}"
        );
    }
}

/// Companion to issue #321: when the provider *does* support reasoning replay
/// (e.g. a generic OpenRouter-style endpoint with provider features enabled and
/// thinking on), an interrupted reasoning-only assistant turn should be sent
/// with both a `reasoning_content` field and a valid (empty) `content`, so the
/// turn is preserved without violating the "content or tool_calls" requirement.
#[test]
fn interrupted_reasoning_only_assistant_message_keeps_reasoning_with_content() {
    let _lock = ENV_LOCK.lock();
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        profile_id: None,
        supports_provider_features: true,
        supports_model_catalog: false,
        ..make_custom_compatible_provider()
    };

    let messages = vec![
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "do a thing".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Reasoning {
                text: "thinking about the request".to_string(),
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "actually do this instead".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
    ];

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
    let api_messages = body
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("request should contain messages array");

    let assistant = api_messages
        .iter()
        .find(|msg| msg.get("role").and_then(|v| v.as_str()) == Some("assistant"))
        .expect("request should retain the interrupted assistant turn");

    assert!(
        assistant.get("reasoning_content").is_some(),
        "reasoning-capable provider should keep reasoning_content; got: {assistant}"
    );
    assert!(
        assistant.get("content").is_some(),
        "interrupted reasoning-only assistant turn must still carry content (issue #321); got: {assistant}"
    );
}

/// Regression for issue #322: the dedicated Kimi coding endpoint
/// (`https://api.kimi.com/coding/v1`, model `kimi-for-coding`) enables thinking
/// server-side and rejects any assistant tool-call message that lacks
/// `reasoning_content` with 400 "thinking is enabled but reasoning_content is
/// missing in assistant tool call message". When an assistant turn produced a
/// tool call without an accompanying reasoning block (the common case once the
/// thinking stream is not persisted), the request builder must still attach a
/// `reasoning_content` field to that assistant message so the endpoint accepts
/// the request.
#[test]
fn kimi_for_coding_tool_call_message_includes_reasoning_content() {
    let _lock = ENV_LOCK.lock();
    let _thinking = EnvVarGuard::remove("JCODE_OPENROUTER_THINKING");
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        // The dedicated Kimi coding endpoint is a direct OpenAI-compatible
        // profile (no OpenRouter provider routing features).
        profile_id: Some("kimi".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        model: Arc::new(RwLock::new("kimi-for-coding".to_string())),
        ..make_custom_compatible_provider()
    };

    let messages = vec![
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "list the files".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        // Assistant turn that emitted a tool call but whose hidden reasoning was
        // not persisted (so there is no Reasoning block to replay).
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "call_1".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({"command": "ls"}),
                thought_signature: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "call_1".to_string(),
                content: "a.txt\nb.txt".to_string(),
                is_error: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
    ];

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
    let api_messages = body
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("request should contain messages array");

    let assistant = api_messages
        .iter()
        .find(|msg| {
            msg.get("role").and_then(|v| v.as_str()) == Some("assistant")
                && msg.get("tool_calls").is_some()
        })
        .expect("request should retain the assistant tool-call turn");

    let reasoning = assistant.get("reasoning_content");
    assert!(
        reasoning.is_some_and(|value| value.as_str().is_some_and(|s| !s.is_empty())),
        "Kimi coding endpoint requires reasoning_content on assistant tool-call messages (issue #322); got: {assistant}"
    );
}

/// Regression for issue #815: DeepSeek-family models on direct
/// OpenAI-compatible profiles require the reasoning returned alongside an
/// assistant tool call to be replayed on the next request. These routes do not
/// enable OpenRouter provider features, so model-family detection must unlock
/// the stored `reasoning_content` without adding a top-level thinking config.
#[test]
fn direct_compatible_deepseek_tool_call_replays_reasoning_content() {
    let _lock = ENV_LOCK.lock();
    let _thinking = EnvVarGuard::remove("JCODE_OPENROUTER_THINKING");
    let (api_base, request_rx) = spawn_single_response_chat_server();
    let provider = OpenRouterProvider {
        api_base,
        profile_id: Some("opencode-zen".to_string()),
        supports_provider_features: false,
        supports_model_catalog: false,
        model: Arc::new(RwLock::new("deepseek-v4-flash-free".to_string())),
        ..make_custom_compatible_provider()
    };

    let messages = vec![
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "list the files".to_string(),
                cache_control: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Reasoning {
                    text: "I should inspect the workspace first.".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "ls"}),
                    thought_signature: None,
                },
            ],
            timestamp: None,
            tool_duration_ms: None,
        },
        Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "call_1".to_string(),
                content: "a.txt\nb.txt".to_string(),
                is_error: None,
            }],
            timestamp: None,
            tool_duration_ms: None,
        },
    ];

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
    let assistant = body["messages"]
        .as_array()
        .expect("request should contain messages array")
        .iter()
        .find(|message| {
            message.get("role").and_then(|value| value.as_str()) == Some("assistant")
                && message.get("tool_calls").is_some()
        })
        .expect("request should retain the assistant tool-call turn");

    assert_eq!(
        assistant
            .get("reasoning_content")
            .and_then(|value| value.as_str()),
        Some("I should inspect the workspace first."),
        "direct DeepSeek request must replay stored reasoning_content (issue #815): {assistant}"
    );
    assert!(
        body.get("thinking").is_none(),
        "server-managed thinking must not add OpenRouter's top-level thinking field: {body}"
    );
}

#[test]
fn minimax_profile_exposes_static_models_before_catalog_refresh() {
    let models = jcode_base::provider_catalog::openai_compatible_profile_static_models(
        jcode_provider_metadata::MINIMAX_PROFILE,
    );
    assert!(models.iter().any(|model| model == "MiniMax-M2.7"));
    assert!(models.iter().any(|model| model == "MiniMax-M2.7-highspeed"));
    assert!(models.iter().any(|model| model == "MiniMax-M2"));
}

include!("openrouter_tests_partition_01_tests.rs");
include!("openrouter_tests_partition_02_tests.rs");
include!("openrouter_tests_partition_03_tests.rs");
