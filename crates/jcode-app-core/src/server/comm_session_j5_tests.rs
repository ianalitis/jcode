use super::{SwarmSpawnSelection, validate_spawn_execution_envelope};
use crate::provider::SpawnExecutionEnvelope;
use jcode_attempt_types::{DataClass, RouteClass, RouterPolicy};

fn selection(model: &str, provider_key: &str, api_method: &str) -> SwarmSpawnSelection {
    SwarmSpawnSelection {
        model: Some(model.to_string()),
        provider_key: Some(provider_key.to_string()),
        route_api_method: Some(api_method.to_string()),
        declared_route_class: None,
    }
}

#[test]
fn openrouter_spawn_without_budget_is_refused_before_visible_or_headless_launch() {
    let route = selection(
        "deepseek/deepseek-v4-flash-0731",
        "openrouter",
        "openrouter",
    );

    let error = validate_spawn_execution_envelope(&route, None)
        .expect_err("metered routes require a bounded envelope at common admission");

    assert!(
        error
            .to_string()
            .contains("metered spawn requires max_micro_usd")
    );
}

#[test]
fn concrete_openrouter_spawn_is_refused_without_enforceable_request_bounds() {
    let route = selection(
        "deepseek/deepseek-v4-flash-0731",
        "openrouter",
        "openrouter",
    );
    let envelope = SpawnExecutionEnvelope::new(Some(100), Some(30), Some(DataClass::Public), None);

    let error = validate_spawn_execution_envelope(&route, Some(&envelope))
        .expect_err("reservation accounting alone is not a per-request hard cap");

    assert!(
        error
            .to_string()
            .contains("no enforceable per-request input, output, and pricing bounds")
    );
}

#[test]
fn unsupported_metered_provider_is_refused_instead_of_using_trait_default() {
    let route = selection("gpt-5.6", "openai-api-key", "openai-api-key");
    let envelope = SpawnExecutionEnvelope::new(Some(100), Some(30), Some(DataClass::Public), None);

    let error = validate_spawn_execution_envelope(&route, Some(&envelope))
        .expect_err("unenforced API-key routes must fail closed");

    assert!(
        error
            .to_string()
            .contains("does not support spawn reservation and settlement")
    );
}

#[test]
fn included_subscription_spawn_does_not_require_a_metered_budget() {
    let route = selection("gpt-5.6-terra", "openai-oauth", "openai-oauth");

    assert_eq!(
        validate_spawn_execution_envelope(&route, None).unwrap(),
        RouteClass::IncludedSubscription
    );
}

#[test]
fn verified_included_subscription_compatible_profile_spawn_needs_no_metered_budget() {
    // OpenCode Go is a fixed-fee subscription with included usage, so a worker
    // pinned to its OpenAI-compatible endpoint is an included-subscription
    // route, not a per-token metered one. Both the explicit api_method and the
    // route-less provider-key fallback must classify identically.
    for route in [
        selection("deepseek-v4.1-flash", "opencode-go", "openai-compatible"),
        selection(
            "deepseek-v4.1-flash",
            "opencode-go",
            "openai-compatible:opencode-go",
        ),
        {
            let mut route = selection("deepseek-v4.1-flash", "opencode-go", "");
            route.route_api_method = None;
            route
        },
    ] {
        assert_eq!(
            validate_spawn_execution_envelope(&route, None).unwrap(),
            RouteClass::IncludedSubscription,
            "{route:?}"
        );
    }
}

#[test]
fn unverified_compatible_profiles_stay_fail_closed_metered() {
    // OpenCode Zen is a prepaid balance and the direct provider keys are
    // per-token metered, so neither may inherit the subscription route class.
    for route in [
        selection("minimax-m2.7", "opencode", "openai-compatible"),
        selection("deepseek-chat", "deepseek", "openai-compatible"),
        selection("glm-4.5", "zai", "openai-compatible"),
    ] {
        let error = validate_spawn_execution_envelope(&route, None)
            .expect_err("unverified compatible profiles must stay metered");
        assert!(
            error
                .to_string()
                .contains("metered spawn requires max_micro_usd"),
            "{route:?}: {error}"
        );
    }
}

#[test]
fn dynamic_router_spawn_is_refused_without_enforceable_request_pricing_bounds() {
    let route = selection("openrouter/auto-beta", "openrouter", "openrouter");
    let envelope = SpawnExecutionEnvelope::new(
        Some(100),
        Some(30),
        Some(DataClass::Public),
        Some(RouterPolicy {
            excluded_models: vec!["openai/*".to_string(), "anthropic/*".to_string()],
            cost_tier: Some("low".to_string()),
        }),
    );

    let error = validate_spawn_execution_envelope(&route, Some(&envelope))
        .expect_err("dynamic pricing cannot promise the local reservation as a hard cap");

    assert!(error.to_string().contains("dynamic router pricing"));
}

#[test]
fn undeclared_provider_key_fails_closed_unless_the_provider_declares_local() {
    let mut route = selection("unknown-model", "test", "");
    route.route_api_method = None;
    let error = validate_spawn_execution_envelope(&route, None)
        .expect_err("unclassifiable provider keys must not launch");
    assert!(
        error
            .to_string()
            .contains("cannot establish spawn route billing for `test`")
    );

    route.declared_route_class = Some(RouteClass::Local);
    assert_eq!(
        validate_spawn_execution_envelope(&route, None).unwrap(),
        RouteClass::Local
    );

    // A provider may not declare its way onto a subscription or metered class.
    route.declared_route_class = Some(RouteClass::IncludedSubscription);
    assert!(validate_spawn_execution_envelope(&route, None).is_err());
    route.declared_route_class = Some(RouteClass::MeteredRemote);
    assert!(validate_spawn_execution_envelope(&route, None).is_err());
}
