#[test]
fn quality_tier_ranks_flagship_above_bare_above_cheap() {
    // Flagship-marked ids (max/pro/opus/coder/large/huge-param) -> tier 2.
    assert_eq!(openai_compatible_model_quality_tier("qwen3-max"), 2);
    assert_eq!(openai_compatible_model_quality_tier("claude-opus-4-8"), 2);
    assert_eq!(openai_compatible_model_quality_tier("qwen3-coder-480b"), 2);
    assert_eq!(openai_compatible_model_quality_tier("glm-4.6-pro"), 2);
    assert_eq!(
        openai_compatible_model_quality_tier("llama-3.1-405b-instruct"),
        2
    );

    // Bare frontier ids (no tier marker) -> tier 1.
    assert_eq!(openai_compatible_model_quality_tier("gpt-5.5"), 1);
    assert_eq!(openai_compatible_model_quality_tier("minimax-m2.7"), 1);
    assert_eq!(openai_compatible_model_quality_tier("kimi-k2.5"), 1);
    assert_eq!(openai_compatible_model_quality_tier("glm-4.6"), 1);

    // Cheap/small/fast-marked ids -> tier 0.
    assert_eq!(openai_compatible_model_quality_tier("gpt-5.5-mini"), 0);
    assert_eq!(openai_compatible_model_quality_tier("deepseek-v4-flash"), 0);
    assert_eq!(openai_compatible_model_quality_tier("glm-4.6-air"), 0);
    assert_eq!(
        openai_compatible_model_quality_tier("llama-3.1-8b-instant"),
        0
    );
    assert_eq!(openai_compatible_model_quality_tier("claude-haiku-4-5"), 0);

    // Brand names that merely *contain* a marker substring must NOT trip the
    // whole-token matcher: `minimax` is not `mini`/`max`.
    assert_eq!(openai_compatible_model_quality_tier("minimax-m2.7"), 1);

    // Flagship marker beats a co-occurring size token.
    assert_eq!(
        openai_compatible_model_quality_tier("qwen3-coder-30b-a3b"),
        2
    );
}

#[test]
fn newest_release_picker_prefers_strongest_tier_over_newest_cheap() {
    use jcode_provider_openrouter::ModelInfo;
    let _lock = crate::storage::lock_test_env();
    let _env = EnvGuard::save(&["JCODE_HOME"]);
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());

    let mk = |id: &str, created: u64| ModelInfo {
        id: id.to_string(),
        name: String::new(),
        context_length: None,
        pricing: Default::default(),
        created: Some(created),
    };

    // A heterogeneous proxy catalog (like OpenCode Zen): the NEWEST model is a
    // cheap `*-flash`, but a slightly older flagship-marked model exists. The
    // picker must choose the flagship, not the newest-cheap.
    jcode_provider_openrouter::save_disk_cache_with_source_for_namespace(
        "deepseek",
        &[
            mk("deepseek-v4-flash", 1_900_000_000), // newest, but cheap tier
            mk("deepseek-v4", 1_850_000_000),       // bare frontier
            mk("deepseek-v4-coder", 1_800_000_000), // flagship tier, oldest
        ],
        Some("https://api.deepseek.com"),
    );

    assert_eq!(
        newest_released_model_for_openai_compatible_profile("deepseek").as_deref(),
        Some("deepseek-v4-coder"),
        "a flagship-marked model must win over a newer cheap/flash sibling"
    );
}

#[test]
fn newest_release_picker_uses_recency_within_a_tier() {
    use jcode_provider_openrouter::ModelInfo;
    let _lock = crate::storage::lock_test_env();
    let _env = EnvGuard::save(&["JCODE_HOME"]);
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());

    let mk = |id: &str, created: u64| ModelInfo {
        id: id.to_string(),
        name: String::new(),
        context_length: None,
        pricing: Default::default(),
        created: Some(created),
    };

    // All same (bare frontier) tier: recency decides.
    jcode_provider_openrouter::save_disk_cache_with_source_for_namespace(
        "deepseek",
        &[
            mk("deepseek-v3", 1_700_000_000),
            mk("deepseek-v4", 1_900_000_000), // newest within the same tier
            mk("deepseek-v3.1", 1_800_000_000),
        ],
        Some("https://api.deepseek.com"),
    );

    assert_eq!(
        newest_released_model_for_openai_compatible_profile("deepseek").as_deref(),
        Some("deepseek-v4"),
        "within one quality tier the newest release should win"
    );
}

/// Exhaustiveness guard: every model shipped in a profile's static catalog must
/// resolve to a concrete context window. Open-weight gateways frequently omit
/// `context_length` from `/v1/models`, so a missing entry here means that model
/// would silently fall back to the generic 200K default. First-party
/// OpenAI/Claude/Gemini ids are resolved by their own providers (not this static
/// table) and are exempted.
#[test]
fn every_static_profile_model_has_a_known_context_limit() {
    use jcode_provider_core::models::context_limit_for_model_with_provider;

    // Ids handled by dedicated first-party providers rather than the
    // OpenAI-compatible static table.
    fn is_first_party(model: &str) -> bool {
        let m = model.to_ascii_lowercase();
        m.starts_with("claude-")
            || m.starts_with("gpt-")
            || m.starts_with("gemini-")
            || m.starts_with("o3")
            || m.starts_with("o4")
    }

    let mut missing: Vec<(String, String)> = Vec::new();
    for profile in jcode_provider_metadata::openai_compatible_profiles()
        .iter()
        .copied()
    {
        for model in openai_compatible_profile_static_models(profile) {
            if is_first_party(&model) {
                continue;
            }

            let via_profile = openai_compatible_profile_context_limit(profile.id, &model);
            let via_global = context_limit_for_model_with_provider(&model, Some("openrouter"));

            if via_profile.is_none() && via_global.is_none() {
                missing.push((profile.id.to_string(), model));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "static profile models without a known context limit (would fall back to the \
         generic default); add them to open_weight_family_context_limit: {missing:?}"
    );
}

#[test]
fn open_weight_family_context_limits_match_published_windows() {
    use jcode_provider_core::models::open_weight_family_context_limit as f;

    // GLM family spelling variants across gateways.
    assert_eq!(f("glm-4.5"), Some(128_000));
    assert_eq!(f("glm-4.7"), Some(200_000));
    assert_eq!(f("zai-org/glm-4.7"), Some(200_000));
    assert_eq!(f("accounts/fireworks/models/glm-4p7"), Some(200_000));
    assert_eq!(f("glm-5"), Some(200_000));
    assert_eq!(f("glm-5.1"), Some(200_000));
    assert_eq!(f("zai-glm-5-1"), Some(200_000));
    assert_eq!(f("glm-5.2"), Some(1_000_000));

    // Other open-weight families.
    assert_eq!(f("kimi-k2.5"), Some(262_144));
    assert_eq!(f("minimax-m2.7"), Some(204_800));
    assert_eq!(f("mimo-v2.5"), Some(262_144));
    assert_eq!(f("muse-spark-1.2"), Some(1_048_576));
    assert_eq!(f("deepseek-v3.2"), Some(163_840));
    assert_eq!(f("deepseek-v4-pro"), Some(1_000_000));
    assert_eq!(f("qwen3-235b-a22b-instruct-2507"), Some(262_144));
    assert_eq!(f("gpt-oss-120b"), Some(131_072));
    assert_eq!(f("llama-3.3-70b-instruct"), Some(131_072));
    assert_eq!(f("sonar-pro"), Some(128_000));

    // Unknown families stay unresolved so the dynamic cache/default can act.
    assert_eq!(f("some-unknown-model"), None);
}

#[test]
fn minimax_default_provider_applies_minimax_api_key_env_not_openrouter() {
    // Regression for #407: `default_provider = "minimax"` (the built-in MiniMax
    // profile) must resolve credentials from the profile's documented
    // MINIMAX_API_KEY / minimax.env, not the generic OPENROUTER_API_KEY /
    // openrouter.env. The earlier bug surfaced as
    // "OPENROUTER_API_KEY not found ..." when applying the configured
    // default_model.
    let _lock = crate::storage::lock_test_env();
    let _guard = EnvGuard::save(&[
        "JCODE_OPENROUTER_API_KEY_NAME",
        "JCODE_OPENROUTER_ENV_FILE",
        "JCODE_OPENROUTER_API_BASE",
        "JCODE_OPENROUTER_CACHE_NAMESPACE",
        "JCODE_PROVIDER_PROFILE_ACTIVE",
        "JCODE_NAMED_PROVIDER_PROFILE",
    ]);
    for v in [
        "JCODE_OPENROUTER_API_KEY_NAME",
        "JCODE_OPENROUTER_ENV_FILE",
        "JCODE_OPENROUTER_API_BASE",
        "JCODE_OPENROUTER_CACHE_NAMESPACE",
        "JCODE_PROVIDER_PROFILE_ACTIVE",
        "JCODE_NAMED_PROVIDER_PROFILE",
    ] {
        crate::env::remove_var(v);
    }

    let selection = resolve_openai_compatible_profile_selection("minimax");
    assert_eq!(
        selection.map(|profile| profile.id),
        Some("minimax"),
        "default_provider=minimax must resolve the built-in MiniMax profile"
    );

    apply_openai_compatible_profile_env(selection);

    assert_eq!(
        std::env::var("JCODE_OPENROUTER_API_KEY_NAME")
            .ok()
            .as_deref(),
        Some("MINIMAX_API_KEY"),
        "MiniMax profile must use MINIMAX_API_KEY, not OPENROUTER_API_KEY"
    );
    assert_eq!(
        std::env::var("JCODE_OPENROUTER_ENV_FILE").ok().as_deref(),
        Some("minimax.env"),
        "MiniMax profile must use minimax.env, not openrouter.env"
    );
}

#[test]
fn novita_static_models_are_available_before_live_catalog_refresh() {
    let models = openai_compatible_profile_static_models(NOVITA_PROFILE);
    assert_eq!(
        models.first().map(String::as_str),
        NOVITA_PROFILE.default_model
    );
    for model in [
        "zai-org/glm-5.3",
        "zai-org/glm-5.3-flash",
        "moonshotai/kimi-k3",
        "deepseek/deepseek-v4-pro-0813",
    ] {
        assert!(models.iter().any(|candidate| candidate == model));
    }
}
