#[test]
fn post_auth_frontier_promotion_covers_bedrock_and_gemini() {
    // Bedrock: a newer Opus 5 (vendor-prefixed + dated) auto-promotes over the
    // curated Opus 4 baseline, and never falls back to the year-old 3.5.
    let activation = activation_for_provider_id("bedrock");
    let routes = vec![
        route(
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "AWS Bedrock",
            "bedrock",
            true,
        ),
        route(
            "anthropic.claude-opus-4-20250514-v1:0",
            "AWS Bedrock",
            "bedrock",
            true,
        ),
        route(
            "anthropic.claude-opus-5-20260101-v1:0",
            "AWS Bedrock",
            "bedrock",
            true,
        ),
    ];
    assert_eq!(
        provider_model_to_select_after_auth(&activation, None, &routes).as_deref(),
        Some("anthropic.claude-opus-5-20260101-v1:0"),
        "a newer Bedrock Opus must auto-promote over the curated Opus 4"
    );

    // Gemini: a newer pro auto-promotes; a newer flash never displaces it.
    let activation = activation_for_provider_id("gemini");
    let routes = vec![
        route(
            "gemini-2.5-flash",
            "Google Gemini",
            "code-assist-oauth",
            true,
        ),
        route(
            "gemini-3-pro-preview",
            "Google Gemini",
            "code-assist-oauth",
            true,
        ),
        route(
            "gemini-4-pro-preview",
            "Google Gemini",
            "code-assist-oauth",
            true,
        ),
        route(
            "gemini-9-flash-preview",
            "Google Gemini",
            "code-assist-oauth",
            true,
        ),
    ];
    assert_eq!(
        provider_model_to_select_after_auth(&activation, None, &routes).as_deref(),
        Some("gemini-4-pro-preview"),
        "the newest Gemini *pro* must win; a higher-numbered flash must not"
    );
}

#[test]
fn frontier_version_parsing_and_compare() {
    let fams = &[
        FrontierFamily {
            prefix: "claude-opus",
            flagship_token: None,
        },
        FrontierFamily {
            prefix: "gpt",
            flagship_token: None,
        },
    ];
    // Clean flagship ids parse with a version vector.
    let opus = parse_frontier_model("claude-opus-4-8", fams).expect("opus parses");
    assert_eq!(opus.family, "claude-opus");
    assert_eq!(opus.version, vec![4, 8]);
    let gpt = parse_frontier_model("gpt-5.5", fams).expect("gpt parses");
    assert_eq!(gpt.family, "gpt");
    assert_eq!(gpt.version, vec![5, 5]);
    // Dated id parses on the canonical base.
    assert_eq!(
        parse_frontier_model("claude-opus-4-9-20260101", fams)
            .expect("dated opus parses")
            .version,
        vec![4, 9]
    );
    // Specialized/cheap tiers and other families are rejected.
    assert!(parse_frontier_model("claude-haiku-5", fams).is_none());
    assert!(parse_frontier_model("gpt-6-mini", fams).is_none());
    assert!(parse_frontier_model("gpt-5-codex", fams).is_none());
    assert!(parse_frontier_model("claude-sonnet-5", fams).is_none());
    // Version comparison is component-wise with zero-padding.
    assert_eq!(version_cmp(&[4, 8], &[4, 9]), std::cmp::Ordering::Less);
    assert_eq!(version_cmp(&[5], &[5, 1]), std::cmp::Ordering::Less);
    assert_eq!(version_cmp(&[6], &[5, 9]), std::cmp::Ordering::Greater);
    assert_eq!(version_cmp(&[5, 5], &[5, 5]), std::cmp::Ordering::Equal);

    // Bedrock vendor-prefixed/versioned ids normalize to the bare Claude
    // family and parse as flagship.
    let bedrock = parse_frontier_model(
        "us.anthropic.claude-opus-4-20250514-v1:0",
        &[FrontierFamily {
            prefix: "claude-opus",
            flagship_token: None,
        }],
    )
    .expect("bedrock opus parses");
    assert_eq!(bedrock.version, vec![4]);

    // Gemini flagship token: `pro` is required and `flash`/`lite` are rejected.
    let gem_fams = &[FrontierFamily {
        prefix: "gemini",
        flagship_token: Some("pro"),
    }];
    let gpro = parse_frontier_model("gemini-3-pro-preview", gem_fams).expect("gemini pro");
    assert_eq!(gpro.family, "gemini");
    assert_eq!(gpro.version, vec![3]);
    assert_eq!(
        parse_frontier_model("gemini-3.1-pro", gem_fams)
            .expect("gemini 3.1 pro")
            .version,
        vec![3, 1]
    );
    assert!(
        parse_frontier_model("gemini-3-flash", gem_fams).is_none(),
        "gemini flash is the cheap tier and must not be a frontier flagship"
    );
    assert!(
        parse_frontier_model("gemini-2.5-flash-lite", gem_fams).is_none(),
        "gemini flash-lite must be rejected"
    );
}

#[test]
fn normalize_model_for_preference_strips_hosted_prefixes_and_suffixes() {
    assert_eq!(
        normalize_model_for_preference("us.anthropic.claude-opus-4-20250514-v1:0"),
        "claude-opus-4"
    );
    assert_eq!(
        normalize_model_for_preference("anthropic.claude-3-5-sonnet-20241022-v2:0"),
        "claude-3-5-sonnet"
    );
    assert_eq!(
        normalize_model_for_preference("models/gemini-3-pro-preview"),
        "gemini-3-pro"
    );
    assert_eq!(
        normalize_model_for_preference("accounts/fireworks/models/qwen3-coder"),
        "qwen3-coder"
    );
    // Non-hosted ids are unchanged apart from canonicalization.
    assert_eq!(
        normalize_model_for_preference("claude-haiku-4-5-20251001"),
        "claude-haiku-4-5"
    );
    assert_eq!(normalize_model_for_preference("gpt-5.5"), "gpt-5.5");
}

/// The set of canonical provider ids whose post-login fallback must apply a
/// curated flagship-first order. These are the providers that expose
/// Claude/OpenAI models under their bare canonical ids and report no
/// `activated_model`, so a "cheap model first" catalog would otherwise
/// auto-select the wrong default. Kept here as the single source of truth
/// the exhaustive walk asserts against.
const RANKED_PROVIDER_IDS: &[&str] = &[
    "claude",
    "claude-api",
    "openai",
    "openai-api",
    "copilot",
    "cursor",
    "bedrock",
    "azure-openai",
    "gemini",
    "antigravity",
];

fn activation_for_provider_id(provider_id: &str) -> AuthActivationResult {
    AuthActivationResult {
        provider_id: Some(provider_id.to_string()),
        provider_label: provider_display_label(Some(provider_id)),
        activated_model: None,
        expected_runtime: None,
        expected_catalog_namespace: None,
    }
}

/// Exhaustive walk: every login provider descriptor is classified as ranked
/// (curated flagship order) or unranked (catalog order), and the
/// classification must exactly match RANKED_PROVIDER_IDS. This is the guard
/// that catches a newly added provider that proxies Claude/OpenAI models but
/// forgets to opt into the flagship-first fallback.
#[test]
fn post_auth_model_selection_classifies_every_login_provider() {
    let mut ranked_seen: std::collections::BTreeSet<String> = Default::default();
    for descriptor in crate::provider_catalog::login_providers() {
        let Some(provider_id) = normalized_auth_provider_id(Some(descriptor.id)) else {
            // AutoImport / non-runtime descriptors have no activation id.
            continue;
        };
        let activation = activation_for_provider_id(provider_id);
        let ranked = !provider_preferred_model_orders(&activation).is_empty();
        let expected = RANKED_PROVIDER_IDS.contains(&provider_id);
        assert_eq!(
            ranked, expected,
            "login provider `{}` (id `{}`) classified ranked={ranked}, expected {expected}; \
                 if this is a new Claude/OpenAI-proxying provider add it to \
                 provider_preferred_model_orders + RANKED_PROVIDER_IDS, otherwise leave it unranked",
            descriptor.id, provider_id
        );
        if ranked {
            ranked_seen.insert(provider_id.to_string());
        }
    }
    let expected_ranked: std::collections::BTreeSet<String> = RANKED_PROVIDER_IDS
        .iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(
        ranked_seen, expected_ranked,
        "the ranked providers reachable from the login catalog drifted from RANKED_PROVIDER_IDS"
    );
}

/// Exhaustive walk: for every ranked provider, an adversarial catalog that
/// lists the cheapest model first must still auto-select the curated
/// flagship after login. This is the direct regression for the live
/// Anthropic API-key login that auto-selected Haiku instead of Opus.
#[test]
fn post_auth_model_selection_picks_flagship_for_every_ranked_provider() {
    // (provider_id, api_method, provider_display, cheap_first_routes, expected flagship)
    let cases: &[(&str, &str, &str, &[&str], &str)] = &[
        (
            "claude",
            "claude-oauth",
            "Anthropic",
            &["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-8"],
            "claude-opus-4-8",
        ),
        (
            "claude-api",
            "claude-api",
            "Anthropic",
            &[
                "claude-haiku-4-5-20251001",
                "claude-sonnet-4-6",
                "claude-opus-4-8",
            ],
            "claude-opus-4-8",
        ),
        (
            "openai",
            "openai-oauth",
            "OpenAI",
            &["gpt-5-nano", "gpt-5.1", "gpt-5.5"],
            "gpt-5.5",
        ),
        (
            "openai-api",
            "openai-api-key",
            "OpenAI",
            &["gpt-5-mini", "gpt-5.1", "gpt-5.5"],
            "gpt-5.5",
        ),
        (
            // Copilot proxies Claude under canonical ids: Opus must beat Haiku.
            "copilot",
            "copilot",
            "Copilot",
            &["claude-haiku-4-5", "gpt-5.5", "claude-opus-4-8"],
            "claude-opus-4-8",
        ),
        (
            // Cursor likewise: an all-OpenAI catalog still picks the flagship.
            "cursor",
            "cursor",
            "Cursor",
            &["gpt-5-nano", "gpt-5.1", "gpt-5.5"],
            "gpt-5.5",
        ),
        (
            // Bedrock lists year-old Claude first; the curated order must
            // still pick Opus 4 over claude-3-5-sonnet. Bedrock ids carry the
            // vendor prefix + version tag, normalized away before ranking.
            "bedrock",
            "bedrock",
            "AWS Bedrock",
            &[
                "anthropic.claude-3-5-sonnet-20241022-v2:0",
                "anthropic.claude-3-5-haiku-20241022-v1:0",
                "anthropic.claude-sonnet-4-20250514-v1:0",
                "anthropic.claude-opus-4-20250514-v1:0",
            ],
            "anthropic.claude-opus-4-20250514-v1:0",
        ),
        (
            // Azure hosts the OpenAI family over the OpenRouter transport.
            "azure-openai",
            "openrouter",
            "Azure OpenAI",
            &["gpt-5-mini", "gpt-5.1", "gpt-5.5"],
            "gpt-5.5",
        ),
        (
            // Gemini's flagship tier is `pro`; a flash-first catalog must
            // still pick the strongest pro model.
            "gemini",
            "code-assist-oauth",
            "Google Gemini",
            &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-3-pro-preview"],
            "gemini-3-pro-preview",
        ),
        (
            // Antigravity also serves Gemini models (https transport).
            "antigravity",
            "https",
            "Antigravity",
            &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-3-pro-preview"],
            "gemini-3-pro-preview",
        ),
    ];

    // Guard: the hand-written cases must cover every ranked provider, or the
    // "for_every_ranked_provider" claim silently rots when a new ranked
    // provider is added without a matching case.
    let covered: std::collections::BTreeSet<&str> =
        cases.iter().map(|(provider_id, ..)| *provider_id).collect();
    let expected_covered: std::collections::BTreeSet<&str> =
        RANKED_PROVIDER_IDS.iter().copied().collect();
    assert_eq!(
        covered, expected_covered,
        "flagship cases drifted from RANKED_PROVIDER_IDS; add a cheap-first case for any \
             newly ranked provider so its flagship selection is actually exercised"
    );

    for (provider_id, api_method, provider_display, models, expected) in cases {
        let activation = activation_for_provider_id(provider_id);
        let routes: Vec<ModelRoute> = models
            .iter()
            .map(|model| route(model, provider_display, api_method, true))
            .collect();
        assert_eq!(
            provider_model_to_select_after_auth(&activation, None, &routes).as_deref(),
            Some(*expected),
            "provider `{provider_id}` should auto-select flagship `{expected}` from a \
                 cheap-first catalog, not the first route `{}`",
            models[0]
        );
    }
}

/// Copilot proxies both families; the cross-family tie-break must prefer the
/// Claude flagship over the OpenAI flagship to mirror jcode's default model.
#[test]
fn post_auth_model_selection_copilot_prefers_claude_family_over_openai() {
    let activation = activation_for_provider_id("copilot");
    let routes = vec![
        route("gpt-5.5", "Copilot", "copilot", true),
        route("claude-opus-4-8", "Copilot", "copilot", true),
    ];
    assert_eq!(
        provider_model_to_select_after_auth(&activation, None, &routes).as_deref(),
        Some("claude-opus-4-8"),
        "copilot tie-break should prefer the Claude flagship family first"
    );
}

#[test]
fn onboarding_frontier_provider_preference_matrix() {
    use crate::auth::{AuthState, AuthStatus, ProviderAuth};

    let none = AuthStatus::default();
    assert_eq!(preferred_frontier_auth_provider(&none), None);

    let openai_api = AuthStatus {
        openai: AuthState::Available,
        openai_has_api_key: true,
        ..AuthStatus::default()
    };
    assert_eq!(
        preferred_frontier_auth_provider(&openai_api),
        Some("openai-api")
    );

    let anthropic_api = AuthStatus {
        anthropic: ProviderAuth {
            state: AuthState::Available,
            has_api_key: true,
            ..ProviderAuth::default()
        },
        ..AuthStatus::default()
    };
    assert_eq!(
        preferred_frontier_auth_provider(&anthropic_api),
        Some("claude-api")
    );

    let both_oauth = AuthStatus {
        openai: AuthState::Available,
        openai_has_oauth: true,
        openai_oauth_state: AuthState::Available,
        anthropic: ProviderAuth {
            state: AuthState::Available,
            has_oauth: true,
            oauth_state: AuthState::Available,
            ..ProviderAuth::default()
        },
        ..AuthStatus::default()
    };
    assert_eq!(
        preferred_frontier_auth_provider(&both_oauth),
        Some("claude"),
        "Claude is the quality-first default when both frontier providers work"
    );

    let openai_api_and_oauth = AuthStatus {
        openai: AuthState::Available,
        openai_has_oauth: true,
        openai_oauth_state: AuthState::Available,
        openai_has_api_key: true,
        ..AuthStatus::default()
    };
    assert_eq!(
        preferred_frontier_auth_provider(&openai_api_and_oauth),
        Some("openai"),
        "OAuth is preferred over an API key within one provider family"
    );
}
