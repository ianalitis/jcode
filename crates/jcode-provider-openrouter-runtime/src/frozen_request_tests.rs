use super::openrouter_provider_impl::{
    merge_extra_body_and_validate, validate_expected_final_request,
};
use super::*;
use serde_json::{Value, json};

fn admitted_final_request() -> Value {
    json!({
        "model": "approved/model",
        "provider": {
            "order": ["approved-provider"],
            "only": ["approved-provider"],
            "allow_fallbacks": false,
            "data_collection": "deny",
            "max_price": {
                "prompt": "0.10",
                "completion": "0.20"
            }
        },
        "messages": [{"role": "user", "content": "approved prompt"}],
        "max_tokens": 64,
        "stream": true,
        "reasoning": {"effort": "high"},
        "usage": {"include": true}
    })
}

fn assert_frozen_request_rejected(label: &str, actual: Value) {
    let error = validate_expected_final_request(&admitted_final_request(), &actual)
        .expect_err("hostile final payload must be rejected");
    assert!(
        error.to_string().contains("trusted expected request"),
        "{label}: {error:#}"
    );
}

fn synthetic_provider(extra_body: Option<serde_json::Map<String, Value>>) -> OpenRouterProvider {
    OpenRouterProvider {
        client: jcode_provider_core::shared_http_client(),
        model: Arc::new(RwLock::new("approved/model".to_string())),
        reasoning_effort: Arc::new(RwLock::new(None)),
        api_base: "https://fixture.invalid/v1".to_string(),
        auth: ProviderAuth::None {
            label: "synthetic no-auth fixture".to_string(),
        },
        supports_provider_features: false,
        supports_model_catalog: false,
        profile_id: Some("synthetic".to_string()),
        reasoning_effort_support: Some(false),
        disable_reasoning_heuristics: true,
        static_reasoning_config: HashMap::new(),
        max_tokens: None,
        extra_body,
        static_models: Vec::new(),
        static_context_limits: HashMap::new(),
        static_image_input_support: HashMap::new(),
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        extra_headers: ExtraHeaders::default(),
        models_cache: Arc::new(RwLock::new(ModelsCache::default())),
        model_catalog_refresh: Arc::new(Mutex::new(ModelCatalogRefreshState::default())),
        provider_routing: Arc::new(RwLock::new(ProviderRouting::default())),
        provider_pin: Arc::new(Mutex::new(None)),
        endpoints_cache: Arc::new(RwLock::new(HashMap::new())),
        endpoint_refresh: Arc::new(Mutex::new(EndpointRefreshTracker::default())),
    }
}

#[test]
fn frozen_request_accepts_only_the_exact_approved_final_payload() {
    let expected = admitted_final_request();
    validate_expected_final_request(&expected, &expected)
        .expect("the exact approved final payload must pass");
}

#[test]
fn frozen_request_rejects_hostile_final_payload_changes() {
    let mut cases = Vec::new();

    let mut actual = admitted_final_request();
    actual["model"] = json!("attacker/model");
    cases.push(("model changed", actual));

    let mut actual = admitted_final_request();
    actual.as_object_mut().unwrap().remove("provider");
    cases.push(("provider removed", actual));

    let mut actual = admitted_final_request();
    actual["provider"] = json!({"only": ["attacker-provider"]});
    cases.push(("provider replaced", actual));

    let mut actual = admitted_final_request();
    actual["provider"]["allow_fallbacks"] = json!(true);
    cases.push(("fallback policy changed", actual));

    let mut actual = admitted_final_request();
    actual["provider"]["data_collection"] = json!("allow");
    cases.push(("privacy policy changed", actual));

    let mut actual = admitted_final_request();
    actual["provider"]["only"] = json!(["attacker-provider"]);
    cases.push(("provider allowlist changed", actual));

    let mut actual = admitted_final_request();
    actual["provider"]["max_price"] = json!(0.01);
    cases.push(("price object replaced by scalar", actual));

    let mut actual = admitted_final_request();
    actual["max_tokens"] = json!(65);
    cases.push(("output cap increased", actual));

    let mut actual = admitted_final_request();
    actual.as_object_mut().unwrap().remove("max_tokens");
    cases.push(("output cap removed", actual));

    let mut actual = admitted_final_request();
    actual["messages"][0]["content"] = json!("hostile prompt");
    cases.push(("messages changed", actual));

    let mut actual = admitted_final_request();
    actual["tools"] = json!([{"type": "function", "function": {"name": "shell"}}]);
    cases.push(("tool added", actual));

    let mut actual = admitted_final_request();
    actual["plugins"] = json!([{"id": "web"}]);
    cases.push(("plugin added", actual));

    let mut actual = admitted_final_request();
    actual["reasoning"]["effort"] = json!("max");
    cases.push(("reasoning changed", actual));

    let mut actual = admitted_final_request();
    actual.as_object_mut().unwrap().remove("usage");
    cases.push(("accounting request removed", actual));

    for (label, actual) in cases {
        assert_frozen_request_rejected(label, actual);
    }
}

#[test]
fn frozen_request_no_policy_preserves_ordinary_extra_body_overrides() {
    let mut request = json!({"model": "base", "stream": true});
    let extra = serde_json::Map::from_iter([
        ("model".to_string(), json!("compatible-override")),
        ("temperature".to_string(), json!(0.25)),
    ]);

    merge_extra_body_and_validate(&mut request, Some(&extra), None)
        .expect("ordinary extra_body overrides must remain compatible without a policy");

    assert_eq!(request["model"], "compatible-override");
    assert_eq!(request["temperature"], 0.25);
}

#[test]
fn frozen_request_real_complete_rejects_post_merge_override_before_transport_spawn() {
    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "approved prompt".to_string(),
            cache_control: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }];
    let provider = synthetic_provider(Some(serde_json::Map::from_iter([(
        "model".to_string(),
        json!("attacker/model"),
    )])));
    let api_messages = jcode_provider_openrouter::request::build_chat_messages(
        &messages,
        "",
        false,
        false,
        provider.supports_image_input(),
    );
    let expected = json!({
        "model": "approved/model",
        "messages": api_messages,
        "stream": true,
        "stream_options": {"include_usage": true}
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let result = runtime.block_on(provider.complete_with_expected_final_request(
        expected,
        &messages,
        &[],
        "",
        None,
    ));
    let error = match result {
        Ok(_) => panic!("post-merge hostile override returned a stream"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("trusted expected request"));
}
