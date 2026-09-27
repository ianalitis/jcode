fn questions() -> Map<String, Value> {
    json!({"m0": {"type": "noul", "instructions": "Is this memory relevant?", "criteria": {"true": "Useful", "false": "Irrelevant"}}})
        .as_object().unwrap().clone()
}

fn response() -> Value {
    json!({"answers": {"m0": {"type": "noul", "noul": 0.91}}, "usage": {"input_tokens": 123, "output_tokens": 4}})
}

fn browser_questions() -> Map<String, Value> {
    json!({"action": {"type": "choice", "instructions": "Choose the next browser action", "criteria": {"click": "Click the button", "stop": "Return control"}}})
        .as_object().unwrap().clone()
}

#[test]
fn voice_routes_independently_to_subscription_or_typesafe_only() {
    let selector = JevPurpose::Voice
        .selector_with(
            |key| {
                assert_eq!(key, VOICE_PROVIDER_ENV);
                Err(std::env::VarError::NotPresent)
            },
            || panic!("voice must ignore memory configuration"),
        )
        .unwrap();
    assert_eq!(selector, "auto");
    assert_eq!(
        JevPurpose::Voice
            .selector_with(
                |key| {
                    assert_eq!(key, VOICE_PROVIDER_ENV);
                    Ok("typesafe".into())
                },
                || panic!()
            )
            .unwrap(),
        "typesafe"
    );
    for (available, expected) in [
        ("JCODE_API_KEY", JevProvider::Jcode),
        ("TYPESAFE_API_KEY", JevProvider::TypeSafe),
    ] {
        let (provider, _) = resolve_voice_with(&selector, |env, _| {
            assert!(matches!(env, "JCODE_API_KEY" | "TYPESAFE_API_KEY"));
            (env == available).then(|| "test-key".into())
        })
        .unwrap();
        assert_eq!(provider, expected);
    }
    assert_eq!(
        resolve_voice_with("auto", |_, _| Some("present".into()))
            .unwrap()
            .0,
        JevProvider::Jcode
    );
    let (provider, _) = resolve_voice_with("typesafe", |env, file| {
        assert_eq!((env, file), ("TYPESAFE_API_KEY", "typesafe.env"));
        Some("typesafe-test-key".into())
    })
    .unwrap();
    assert_eq!(provider, JevProvider::TypeSafe);
    assert_eq!(provider.model(), "jev-latest");
    assert_eq!(
        provider.endpoint("").unwrap(),
        "https://api.typesafe.ai/v1/systemone"
    );
    for selector in ["auto", "typesafe", "jcode"] {
        let error = resolve_voice_with(selector, |env, _| {
            assert!(matches!(env, "JCODE_API_KEY" | "TYPESAFE_API_KEY"));
            // Other credentials cannot make this route available.
            (env == "OPENROUTER_API_KEY" || env == "AIMLAPI_API_KEY")
                .then(|| "other-account-key".into())
        })
        .unwrap_err();
        assert!(!error.to_string().contains("OPENROUTER_API_KEY"));
        assert!(!error.to_string().contains("AIMLAPI_API_KEY"));
    }
    for selector in ["openrouter", "aimlapi", ""] {
        assert!(
            resolve_voice_with(selector, |_, _| panic!(
                "invalid voice route must not load keys"
            ))
            .is_err()
        );
    }
    assert_eq!(JevPurpose::Voice.capability(), "memory_jev");
}

#[test]
fn purpose_selectors_are_independent_and_browser_defaults_to_subscription_first() {
    let env = |key: &str| match key {
        PROVIDER_ENV => Ok("typesafe".into()),
        BROWSER_PROVIDER_ENV => Ok("openrouter".into()),
        _ => panic!("unexpected configuration lookup"),
    };
    assert_eq!(
        JevPurpose::Memory.selector_with(env, || panic!()).unwrap(),
        "typesafe"
    );
    assert_eq!(
        JevPurpose::Browser.selector_with(env, || panic!()).unwrap(),
        "openrouter"
    );
    let selector = JevPurpose::Browser
        .selector_with(
            |key| {
                assert_eq!(key, BROWSER_PROVIDER_ENV);
                Err(std::env::VarError::NotPresent)
            },
            || panic!("browser must not consult memory config"),
        )
        .unwrap();
    assert_eq!(selector, "auto");
    assert_eq!(
        resolve_with(&selector, |_, _| Some("present".into()))
            .unwrap()
            .0,
        JevProvider::Jcode
    );
    assert_eq!(
        JevPurpose::Memory
            .selector_with(
                |key| {
                    assert_eq!(key, PROVIDER_ENV);
                    Err(std::env::VarError::NotPresent)
                },
                || "aimlapi".into(),
            )
            .unwrap(),
        "aimlapi"
    );
    for purpose in [JevPurpose::Memory, JevPurpose::Browser] {
        assert!(
            purpose
                .selector_with(
                    |_| Err(std::env::VarError::NotUnicode("invalid".into())),
                    || panic!("invalid override must not use defaults"),
                )
                .is_err()
        );
    }
}

#[test]
fn routing_is_a_third_consumer_with_its_own_key() {
    // Each purpose reads exactly one key, and routing reads its own.
    let env = |key: &str| match key {
        PROVIDER_ENV => Ok("typesafe".into()),
        BROWSER_PROVIDER_ENV => Ok("openrouter".into()),
        ROUTING_PROVIDER_ENV => Ok("jcode".into()),
        _ => panic!("unexpected configuration lookup"),
    };
    assert_eq!(
        JevPurpose::Memory.selector_with(env, || panic!()).unwrap(),
        "typesafe"
    );
    assert_eq!(
        JevPurpose::Browser.selector_with(env, || panic!()).unwrap(),
        "openrouter"
    );
    assert_eq!(
        JevPurpose::Routing.selector_with(env, || panic!()).unwrap(),
        "jcode"
    );

    // With only the memory key present, routing and browser both default to
    // auto and never consult the memory default: the switches are separate.
    let only_memory = |key: &str| match key {
        PROVIDER_ENV => Ok("aimlapi".into()),
        _ => Err(std::env::VarError::NotPresent),
    };
    assert_eq!(
        JevPurpose::Memory
            .selector_with(only_memory, || "aimlapi".into())
            .unwrap(),
        "aimlapi"
    );
    assert_eq!(
        JevPurpose::Routing
            .selector_with(only_memory, || panic!(
                "routing must not consult memory config"
            ))
            .unwrap(),
        "auto"
    );
    assert_eq!(
        JevPurpose::Browser
            .selector_with(only_memory, || panic!(
                "browser must not consult memory config"
            ))
            .unwrap(),
        "auto"
    );

    // An unreadable routing override fails closed rather than falling back.
    assert!(
        JevPurpose::Routing
            .selector_with(
                |key| {
                    assert_eq!(key, ROUTING_PROVIDER_ENV);
                    Err(std::env::VarError::NotUnicode("invalid".into()))
                },
                || panic!("invalid override must not use defaults"),
            )
            .is_err()
    );
}

#[test]
fn routing_carries_its_own_capability_and_option_bound() {
    assert_eq!(JevPurpose::Routing.name(), "routing");
    assert_eq!(JevPurpose::Routing.capability(), "routing_jev");
    // The two live consumers keep their names and capabilities.
    assert_eq!(JevPurpose::Memory.capability(), "memory_jev");
    assert_eq!(JevPurpose::Browser.capability(), "browser_jev");

    let criteria = |n: usize| -> Map<String, Value> {
        (0..n)
            .map(|i| (format!("o{i}"), Value::String(format!("Option {i}"))))
            .collect()
    };
    let choice = |n: usize| -> Map<String, Value> {
        json!({"d0": {"type": "choice", "instructions": "Pick one", "criteria": criteria(n)}})
            .as_object()
            .unwrap()
            .clone()
    };
    for (count, accepted) in [(2usize, true), (16, true), (17, false)] {
        assert_eq!(
            request_body_for(
                JevPurpose::Routing,
                JevProvider::OpenRouter,
                json!("state"),
                &choice(count)
            )
            .is_ok(),
            accepted,
            "routing with {count} options"
        );
    }
    // The same 17-option question is still accepted by the memory consumer,
    // whose wire bound stays at 255.
    assert!(
        request_body_for(
            JevPurpose::Memory,
            JevProvider::OpenRouter,
            json!("state"),
            &choice(17)
        )
        .is_ok()
    );
}

/// Sets process env vars and restores them on drop. Hold the shared
/// test-env lock for the guard's lifetime.
struct EnvGuard(Vec<(&'static str, Option<String>)>);

impl EnvGuard {
    fn set(values: &[(&'static str, &str)]) -> Self {
        let mut previous = Vec::new();
        for (key, value) in values {
            previous.push((*key, std::env::var(key).ok()));
            crate::env::set_var(key, value);
        }
        Self(previous)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.0.drain(..) {
            match value {
                Some(value) => crate::env::set_var(key, value),
                None => crate::env::remove_var(key),
            }
        }
    }
}

#[test]
fn a_routed_decision_resolves_to_its_own_provider_key() {
    let _lock = crate::storage::lock_test_env();
    let _env = EnvGuard::set(&[
        ("JCODE_MEMORY_JEV_PROVIDER", "aimlapi"),
        ("JCODE_BROWSER_JEV_PROVIDER", "openrouter"),
        ("JCODE_ROUTING_JEV_PROVIDER", "typesafe"),
        ("AIMLAPI_API_KEY", "aimlapi_test_only_never_a_real_key"),
        (
            "OPENROUTER_API_KEY",
            "openrouter_test_only_never_a_real_key",
        ),
        ("TYPESAFE_API_KEY", "typesafe_test_only_never_a_real_key"),
    ]);

    let routing = JevClient::for_routing().expect("routing resolves its own key");
    assert_eq!(routing.provider_name(), "typesafe");
    assert_eq!(routing.model_id(), "jev-latest");

    // The two live consumers still read their own keys, unchanged.
    assert_eq!(JevClient::new().expect("memory").provider_name(), "aimlapi");
    assert_eq!(
        JevClient::for_browser().expect("browser").provider_name(),
        "openrouter"
    );
}

#[test]
fn browser_choice_contract_is_distinct_from_memory_noul() {
    let valid = browser_questions();
    for provider in [
        JevProvider::Jcode,
        JevProvider::OpenRouter,
        JevProvider::TypeSafe,
        JevProvider::Aimlapi,
    ] {
        assert!(request_body_for(JevPurpose::Browser, provider, json!("page"), &valid).is_ok());
        assert!(
            request_body_for(JevPurpose::Browser, provider, json!("page"), &questions()).is_err()
        );
        for invalid in [
            json!({}),
            json!({"pick": valid["action"]}),
            json!({"action": valid["action"], "extra": valid["action"]}),
            json!({"action": {"type": "noul", "instructions": "Pick", "criteria": {"a": "A", "b": "B"}}}),
            json!({"action": {"type": "choice", "instructions": {}, "criteria": {"a": "A", "b": "B"}}}),
            json!({"action": {"type": "choice", "instructions": "  ", "criteria": {"a": "A", "b": "B"}}}),
            json!({"action": {"type": "choice", "instructions": "Pick", "criteria": {"a": "A"}}}),
            json!({"action": {"type": "choice", "instructions": "Pick", "criteria": {"a": "A", "b": null}}}),
        ] {
            assert!(
                request_body_for(
                    JevPurpose::Browser,
                    provider,
                    json!("page"),
                    invalid.as_object().unwrap()
                )
                .is_err(),
                "{invalid}"
            );
        }
        for count in [255, 256] {
            let mut q = valid.clone();
            q.get_mut("action").unwrap()["criteria"] = Value::Object(
                (0..count)
                    .map(|i| (i.to_string(), json!("option")))
                    .collect(),
            );
            assert_eq!(
                request_body_for(JevPurpose::Browser, provider, json!("page"), &q).is_ok(),
                count == 255
            );
        }
    }
    assert!(request_body(JevProvider::Jcode, json!("state"), &valid).is_err());
    assert!(request_body(JevProvider::Jcode, json!("state"), &questions()).is_ok());
}

#[test]
fn resolver_keeps_provider_credentials_and_endpoints_isolated() {
    for (selector, expected, key, file, endpoint, model) in [
        (
            "openrouter",
            JevProvider::OpenRouter,
            "OPENROUTER_API_KEY",
            "openrouter.env",
            "https://openrouter.ai/api/alpha/decisions",
            "typesafe/jev-1.13",
        ),
        (
            "typesafe",
            JevProvider::TypeSafe,
            "TYPESAFE_API_KEY",
            "typesafe.env",
            "https://api.typesafe.ai/v1/systemone",
            "jev-latest",
        ),
        (
            "aimlapi",
            JevProvider::Aimlapi,
            "AIMLAPI_API_KEY",
            "aimlapi.env",
            "https://api.aimlapi.com/v1/decisions",
            "typesafe/jev",
        ),
        (
            "jcode",
            JevProvider::Jcode,
            "JCODE_API_KEY",
            "jcode-subscription.env",
            "https://api.jcode.sh/v1/decisions",
            "typesafe/jev-1.13",
        ),
    ] {
        let (provider, secret) = resolve_with(selector, |env, env_file| {
            assert_eq!((env, env_file), (key, file));
            Some(format!("test-{selector}"))
        })
        .unwrap();
        assert_eq!(provider, expected);
        assert_eq!(secret, format!("test-{selector}"));
        assert_eq!(
            provider.endpoint("https://api.jcode.sh/v1/").unwrap(),
            endpoint
        );
        assert_eq!(provider.model(), model);
    }
}

#[test]
fn explicit_missing_provider_never_uses_another_key() {
    let mut lookups = Vec::new();
    let error = resolve_with("typesafe", |env, _| {
        lookups.push(env.to_string());
        (env != "TYPESAFE_API_KEY").then(|| "other-provider-secret".into())
    })
    .err()
    .unwrap();
    assert_eq!(lookups, ["TYPESAFE_API_KEY"]);
    assert!(!error.to_string().contains("other-provider-secret"));
    assert!(
        error
            .to_string()
            .contains("TYPESAFE_API_KEY (typesafe.env)")
    );
    assert!(!error.to_string().contains("OPENROUTER_API_KEY"));
}

#[test]
fn auto_prefers_included_subscription_without_shared_slot() {
    for available in [
        "OPENROUTER_API_KEY",
        "TYPESAFE_API_KEY",
        "AIMLAPI_API_KEY",
        "JCODE_API_KEY",
    ] {
        let (provider, _) = resolve_with("auto", |env, _| {
            assert!(!env.starts_with("JCODE_OPENROUTER"));
            (env == available).then(|| "test-secret".into())
        })
        .unwrap();
        assert_eq!(provider.credentials().0, available);
    }
    assert_eq!(
        resolve_with("auto", |_, _| Some("all-present".into()))
            .unwrap()
            .0,
        JevProvider::Jcode
    );
    // Typesafe direct wins over resellers of the same model.
    assert_eq!(
        resolve_with("auto", |env, _| (env != "JCODE_API_KEY")
            .then(|| "k".into()))
        .unwrap()
        .0,
        JevProvider::TypeSafe
    );
    // Deliberate BYOK remains available even when a Jcode login is present.
    assert_eq!(
        resolve_with("openrouter", |_, _| Some("all-present".into()))
            .unwrap()
            .0,
        JevProvider::OpenRouter
    );
    assert!(resolve_with("auto", |_, _| None).is_err());
    assert!(
        resolve_with("untrusted-selector", |_, _| panic!(
            "must not load any credential"
        ))
        .is_err()
    );
    assert!(
        resolve_with("", |_, _| panic!(
            "empty override must not choose another account"
        ))
        .is_err()
    );
}

#[test]
fn empty_and_malformed_credentials_are_rejected_without_echoing() {
    assert!(resolve_with("openrouter", |_, _| Some(" \"\" ".into())).is_err());
    let secret = "sensitive\r\nheader";
    let error = resolve_with("openrouter", |_, _| Some(secret.into()))
        .err()
        .unwrap();
    assert!(!format!("{error:#}").contains("sensitive"));
}

#[test]
fn gateway_requires_secure_trusted_base() {
    for base in [
        "https://api.jcode.sh/v1",
        "https://custom.example/v1/",
        "http://127.0.0.1:4444/v1",
        "http://[::1]:4444/v1",
        "http://localhost/v1",
    ] {
        assert!(trusted_gateway_base(base).is_ok(), "{base}");
    }
    for base in [
        "http://remote.example/v1",
        "https://user:secret@example.com/v1",
        "https://example.com/v1?key=secret",
        "https://example.com/v1#secret",
        "file:///secret",
        "not-a-url",
    ] {
        assert!(trusted_gateway_base(base).is_err());
    }
}

#[test]
fn request_uses_decisions_not_chat_and_preserves_provider_models() {
    for provider in [
        JevProvider::OpenRouter,
        JevProvider::TypeSafe,
        JevProvider::Aimlapi,
        JevProvider::Jcode,
    ] {
        let body: Value = serde_json::from_slice(
            &request_body(provider, json!({"memory": "example"}), &questions()).unwrap(),
        )
        .unwrap();
        assert_eq!(body["model"], provider.model());
        assert_eq!(body["questions"], Value::Object(questions()));
        assert!(body.get("messages").is_none());
    }
}

#[test]
fn request_bounds_and_gateway_contract_are_checked_before_network() {
    assert!(
        request_body(
            JevProvider::OpenRouter,
            json!("x".repeat(MAX_REQUEST_BYTES)),
            &questions()
        )
        .is_err()
    );
    assert!(request_body(JevProvider::OpenRouter, Value::Null, &questions()).is_err());
    assert!(request_body(JevProvider::OpenRouter, json!("state"), &Map::new()).is_err());
    let many = (0..25)
        .map(|i| (format!("m{i}"), questions()["m0"].clone()))
        .collect();
    assert!(request_body(JevProvider::Jcode, json!("state"), &many).is_err());
    let choice = json!({"pick": {"type": "choice", "instructions": "Pick", "criteria": {"a": "A", "b": "B"}}}).as_object().unwrap().clone();
    assert!(request_body(JevProvider::OpenRouter, json!("state"), &choice).is_ok());
    assert!(request_body(JevProvider::Jcode, json!("state"), &choice).is_err());
    let mut missing_criteria = questions();
    missing_criteria
        .get_mut("m0")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("criteria");
    assert!(request_body(JevProvider::Jcode, json!("state"), &missing_criteria).is_err());
}

#[test]
fn answers_must_match_questions_and_have_valid_noul_probabilities() {
    assert!(validate_answers(&response(), &questions()).is_ok());
    for value in [
        json!({}),
        json!({"answers": {}}),
        json!({"answers": {"other": {"type": "noul", "noul": 0.9}}}),
        json!({"answers": {"m0": {"type": "choice", "noul": 0.9}}}),
        json!({"answers": {"m0": {"type": "noul", "noul": 1.1}}}),
    ] {
        assert!(validate_answers(&value, &questions()).is_err());
    }
}

fn score_questions() -> Map<String, Value> {
    json!({"s0": {"type": "score", "instructions": "Rate this memory", "criteria": ["Irrelevant", "Somewhat relevant", "Relevant"]}})
        .as_object().unwrap().clone()
}

#[test]
fn choice_answers_must_name_a_declared_option() {
    // The browser path offers action IDs as the question's criteria keys and
    // requires the answer to name one of them, so the declared keys are the
    // authoritative option set here too.
    for choice in ["click", "stop"] {
        let value =
            json!({"answers": {"action": {"type": "choice", "choice": choice, "confidence": 0.9}}});
        assert!(
            validate_answers(&value, &browser_questions()).is_ok(),
            "{choice}"
        );
    }
    for answer in [
        json!({"type": "choice", "choice": "unoffered"}),
        json!({"type": "choice", "choice": {"id": "click"}}),
        json!({"type": "choice", "choice": 0}),
        json!({"type": "choice", "choice": null}),
        json!({"type": "choice"}),
    ] {
        let value = json!({"answers": {"action": answer}});
        assert!(
            validate_answers(&value, &browser_questions()).is_err(),
            "accepted {answer}"
        );
    }
}

#[test]
fn score_answers_must_stay_within_declared_levels() {
    for score in [json!(0), json!(1), json!(2), json!(3)] {
        let value = json!({"answers": {"s0": {"type": "score", "score": score}}});
        assert!(
            validate_answers(&value, &score_questions()).is_ok(),
            "rejected {score}"
        );
    }
    for score in [json!(4), json!(-1), json!(1e9), json!("2"), json!(null)] {
        let value = json!({"answers": {"s0": {"type": "score", "score": score}}});
        assert!(
            validate_answers(&value, &score_questions()).is_err(),
            "accepted {score}"
        );
    }
}

#[test]
fn valid_answers_and_noul_handling_are_unchanged() {
    assert!(validate_answers(&response(), &questions()).is_ok());
    let browser = json!({"answers": {"action": {"type": "choice", "choice": "click", "confidence": 0.9, "probabilities": {"click": 0.9, "stop": 0.1}}}});
    assert!(validate_answers(&browser, &browser_questions()).is_ok());
    for noul in [0.0, 1.0, 0.5] {
        let value = json!({"answers": {"m0": {"type": "noul", "noul": noul}}});
        assert!(validate_answers(&value, &questions()).is_ok(), "{noul}");
    }
    for noul in [json!(1.1), json!(-0.1), json!("0.9"), json!(null)] {
        let value = json!({"answers": {"m0": {"type": "noul", "noul": noul}}});
        assert!(
            validate_answers(&value, &questions()).is_err(),
            "accepted {noul}"
        );
    }
}
