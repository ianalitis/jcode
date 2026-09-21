//! Provider prompt-cache retention policy.

use super::{anthropic, openai};

/// Get the prompt cache TTL in seconds for a given provider name.
/// Returns None if the provider doesn't support prompt caching or TTL is unknown.
pub fn cache_ttl_for_provider(provider: &str) -> Option<u64> {
    cache_ttl_for_provider_model(provider, None)
}

/// Whether a reported cache lifetime is an estimate rather than a hard expiry.
/// OpenAI documents typical, maximum, or minimum lifetimes depending on model.
/// The generic OpenRouter/subscription/Gemini values are also only heuristics.
pub fn cache_ttl_is_estimate(provider: &str) -> bool {
    jcode_provider_core::AuthRoute::parse(provider)
        .is_some_and(|route| route.provider == jcode_provider_core::DualAuthProvider::OpenAI)
        || matches!(
            provider.trim().to_ascii_lowercase().as_str(),
            "openrouter" | "jcode subscription" | "gemini"
        )
}

/// Get the prompt cache TTL in seconds for a given provider/model pair.
///
/// This is a cache-retention estimate, not a guaranteed expiry or cache hit.
/// It depends on the auth route, model and configured request retention.
pub fn cache_ttl_for_provider_model(provider: &str, model: Option<&str>) -> Option<u64> {
    if let Some(route) = jcode_provider_core::AuthRoute::parse(provider)
        && route.provider == jcode_provider_core::DualAuthProvider::OpenAI
    {
        // Codex OAuth omits API retention controls. A generic runtime name does
        // not identify the credential mode, so don't claim an API-only lifetime.
        return (route.mode == jcode_provider_core::AuthMode::ApiKey).then(|| {
            openai::prompt_cache_ttl_for_model(
                model,
                openai::prompt_cache_retention_from_env().as_deref(),
            )
        });
    }
    match provider.trim().to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => Some(if anthropic::is_cache_ttl_1h() {
            60 * 60
        } else {
            300
        }),
        "openrouter" => Some(300),
        "jcode subscription" => Some(300),
        "gemini" => Some(300),
        "copilot" => None,
        "cursor" => None,
        "antigravity" => None,
        _ => None,
    }
}
