use crate::provider::{
    ActiveProvider, ModelRoute, MultiProvider, OPENAI_COMPATIBLE_PROFILE_CATALOG_SOFT_REFRESH_SECS,
    Provider, ProviderRegistry, build_openrouter_auto_route, explicit_model_provider_prefix,
    is_listable_model_name,
};
use std::sync::Arc;

pub(in crate::provider) fn openai_compatible_profile_catalog_cache_is_stale(
    cached_at: u64,
    now: u64,
) -> bool {
    now.saturating_sub(cached_at) >= OPENAI_COMPATIBLE_PROFILE_CATALOG_SOFT_REFRESH_SECS
}

pub(crate) fn cached_live_models_for_openai_compatible_profile(
    resolved: &crate::provider_catalog::ResolvedOpenAiCompatibleProfile,
) -> Option<(Vec<String>, bool)> {
    let cache = jcode_provider_openrouter::load_disk_cache_entry_for_namespace(&resolved.id)?;
    let cache_is_stale = jcode_provider_openrouter::current_unix_secs()
        .map(|now| openai_compatible_profile_catalog_cache_is_stale(cache.cached_at, now))
        .unwrap_or(false);
    let source_api_base = cache
        .source_api_base
        .as_deref()
        .and_then(crate::provider_catalog::normalize_api_base)?;
    let expected_api_base = crate::provider_catalog::normalize_api_base(&resolved.api_base)?;
    if source_api_base != expected_api_base {
        return None;
    }

    let models = cache
        .models
        .into_iter()
        .map(|model| model.id.trim().to_string())
        .filter(|model| !model.is_empty())
        .collect::<Vec<_>>();
    if models.is_empty() {
        None
    } else {
        Some((models, cache_is_stale))
    }
}

pub(in crate::provider) fn direct_openai_compatible_profile_routes(
    profile: crate::provider_catalog::OpenAiCompatibleProfile,
) -> Vec<ModelRoute> {
    let resolved = crate::provider_catalog::resolve_openai_compatible_profile(profile);
    let static_models = crate::provider_catalog::openai_compatible_profile_static_models(profile);
    // Pure read: the catalog scheduler owns refresh cadence, so rendering
    // routes cannot fan out HTTP requests.
    let (mut models, from_live_catalog) = if let Some((models, _cache_is_stale)) =
        cached_live_models_for_openai_compatible_profile(&resolved)
    {
        (models, true)
    } else {
        let mut models = static_models;
        if models.is_empty()
            && let Some(default_model) = resolved.default_model.as_ref()
            && !default_model.trim().is_empty()
        {
            models.push(default_model.trim().to_string());
        }
        (models, false)
    };

    let provider = resolved.display_name.clone();
    let api_method = format!("openai-compatible:{}", resolved.id);
    let detail = if from_live_catalog {
        resolved.api_base.clone()
    } else if resolved.api_base.trim().is_empty() {
        "fallback: static provider model list".to_string()
    } else {
        format!(
            "{}; fallback: static provider model list",
            resolved.api_base
        )
    };

    let mut routes = Vec::new();
    for model in models.drain(..) {
        if !is_listable_model_name(&model)
            || !crate::provider_catalog::openai_compatible_profile_model_supports_chat(
                &resolved.id,
                &model,
            )
            || routes.iter().any(|route: &ModelRoute| route.model == model)
        {
            continue;
        }

        routes.push(ModelRoute {
            model,
            provider: provider.clone(),
            api_method: api_method.clone(),
            available: true,
            detail: detail.clone(),
            usage: None,
            cheapness: None,
        });
    }

    routes
}

pub(in crate::provider) fn standard_openrouter_profile_configured() -> bool {
    crate::provider_catalog::load_env_value_from_env_or_config(
        "OPENROUTER_API_KEY",
        "openrouter.env",
    )
    .is_some()
}

pub(in crate::provider) fn configured_standard_openrouter_profile_routes() -> Vec<ModelRoute> {
    let Some(cache) = jcode_provider_openrouter::load_disk_cache_entry_for_namespace("openrouter")
    else {
        return Vec::new();
    };

    let source_matches_openrouter = cache
        .source_api_base
        .as_deref()
        .and_then(crate::provider_catalog::normalize_api_base)
        .map(|base| base.contains("openrouter.ai"))
        .unwrap_or(false);
    if !source_matches_openrouter {
        return Vec::new();
    }

    let available = standard_openrouter_profile_configured();
    cache
        .models
        .into_iter()
        .map(|model| model.id.trim().to_string())
        .filter(|model| is_listable_model_name(model))
        .map(|model| build_openrouter_auto_route(&model, available, String::new()))
        .collect()
}

/// Configured OpenAI-compatible profiles (NVIDIA NIM, Groq, ...), excluding
/// the active direct profile which contributes through the OpenRouter path.
/// Returns whether any routes were added.
pub(super) fn append_openai_compatible_profile_routes(
    provider: &MultiProvider,
    routes: &mut Vec<ModelRoute>,
) -> bool {
    let active_direct_openai_compatible_api_method = provider
        .openrouter_provider()
        .and_then(|openrouter| openrouter.direct_openai_compatible_route_parts())
        .map(|(_, api_method, _)| api_method);
    let mut added_any = false;
    for profile in crate::provider_catalog::openai_compatible_profiles()
        .iter()
        .copied()
    {
        if !crate::provider_catalog::openai_compatible_profile_is_configured(profile) {
            continue;
        }
        let resolved = crate::provider_catalog::resolve_openai_compatible_profile(profile);
        let api_method = format!("openai-compatible:{}", resolved.id);

        // The active OpenRouter/OpenAI-compatible provider contributes its own
        // live memory/disk catalog below. Do not preempt it with the generic
        // configured-profile path, because its in-memory catalog may be newer
        // than the disk snapshot that this non-active profile path can read.
        if active_direct_openai_compatible_api_method.as_deref() == Some(api_method.as_str()) {
            continue;
        }

        let profile_routes = direct_openai_compatible_profile_routes(profile);
        added_any |= !profile_routes.is_empty();
        routes.extend(profile_routes);
    }

    // User-defined named provider profiles (`[providers.<name>]` in
    // config.toml). Their statically declared `[[providers.<name>.models]]`
    // entries (and `default_model`) must surface in the picker with a route
    // back to that profile, even when the profile is not the active provider
    // (issue #444).
    for (profile_name, profile_config) in &crate::config::config().providers {
        let api_method = format!("openai-compatible:{}", profile_name);
        // The active runtime already contributes this profile's models (with
        // live-catalog freshness) via the OpenRouter slot path.
        if active_direct_openai_compatible_api_method.as_deref() == Some(api_method.as_str()) {
            continue;
        }
        let named_routes = named_provider_profile_routes(profile_name, profile_config);
        added_any |= !named_routes.is_empty();
        routes.extend(named_routes);
    }
    added_any
}

/// Picker routes for one user-defined named provider profile from config.
///
/// Text-capable static models plus the profile's `default_model` are offered;
/// models declared image-only via `input = ["image"]` are excluded.
pub(super) fn named_provider_profile_routes(
    profile_name: &str,
    profile_config: &crate::config::NamedProviderConfig,
) -> Vec<ModelRoute> {
    let mut models: Vec<String> = profile_config
        .models
        .iter()
        .filter(|model| {
            // `input` empty means unspecified (assume text-capable).
            model.input.is_empty() || model.input.iter().any(|input| input == "text")
        })
        .map(|model| model.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    if models.is_empty()
        && let Some(default_model) = profile_config
            .default_model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
    {
        models.push(default_model.to_string());
    }

    let api_method = format!("openai-compatible:{}", profile_name);
    let detail = if profile_config.base_url.trim().is_empty() {
        "configured provider profile".to_string()
    } else {
        profile_config.base_url.trim().to_string()
    };

    let mut routes: Vec<ModelRoute> = Vec::new();
    for model in models {
        if !is_listable_model_name(&model) || routes.iter().any(|route| route.model == model) {
            continue;
        }
        routes.push(ModelRoute {
            model,
            provider: profile_name.to_string(),
            api_method: api_method.clone(),
            available: true,
            detail: detail.clone(),
            usage: None,
            cheapness: None,
        });
    }
    routes
}

/// Route for `model` when it belongs to a user-defined `[providers.<name>]`
/// profile from config.toml.
///
/// Built-in OpenAI-compatible profiles are handled above; without this, a
/// bare model id from a custom profile (e.g. a local MLX server) matches no
/// known provider and falls through to the Copilot heuristic, which then
/// labels it `Copilot` and builds a `copilot:<model>` id that no runtime can
/// resolve (issue #694).
pub(super) fn named_provider_profile_route_for_model(model: &str) -> Option<ModelRoute> {
    named_provider_profile_route_for_model_in(model, &crate::config::config().providers)
}

pub(super) fn named_provider_profile_route_for_model_in(
    model: &str,
    providers: &std::collections::BTreeMap<String, crate::config::NamedProviderConfig>,
) -> Option<ModelRoute> {
    let model = model.trim();
    if model.is_empty() {
        return None;
    }
    for (profile_name, profile_config) in providers {
        if !named_provider_profile_routes(profile_name, profile_config)
            .iter()
            .any(|route| route.model == model)
        {
            continue;
        }
        let detail = if profile_config.base_url.trim().is_empty() {
            "configured provider profile".to_string()
        } else {
            profile_config.base_url.trim().to_string()
        };
        return Some(ModelRoute {
            model: model.to_string(),
            provider: profile_name.clone(),
            api_method: format!("openai-compatible:{}", profile_name),
            available: true,
            detail,
            usage: None,
            cheapness: None,
        });
    }
    None
}

impl MultiProvider {
    pub(in crate::provider) fn openai_compatible_model_prefix(
        model: &str,
    ) -> Option<(crate::provider_catalog::OpenAiCompatibleProfile, &str)> {
        let (prefix, rest) = model.split_once(':')?;
        if explicit_model_provider_prefix(model).is_some() {
            return None;
        }
        let rest = rest.trim();
        if rest.is_empty() {
            return None;
        }

        let profile = crate::provider_catalog::openai_compatible_profile_by_id(prefix)?;
        Some((profile, rest))
    }

    /// Find the configured OpenAI-compatible profile that serves a bare model
    /// id, using the live route catalog as the source of truth.
    ///
    /// Route specs from the picker carry a `<profile>:<model>` prefix, but
    /// hand-typed `/model <id>` and saved sessions can carry the bare id. The
    /// active profile wins when several profiles serve the same id, so a
    /// re-select of the current model never silently hops endpoints.
    pub(in crate::provider) fn openai_compatible_profile_owning_model(
        &self,
        model: &str,
    ) -> Option<crate::provider_catalog::OpenAiCompatibleProfile> {
        let model = model.trim();
        if model.is_empty() {
            return None;
        }

        let active_profile_id = ProviderRegistry::new(self).active_compatible_profile_id();
        let mut fallback: Option<String> = None;
        for route in self.fresh_routes_memo_entry().routes {
            if !route.available || route.model != model {
                continue;
            }
            let Some(profile_id) = route
                .api_method
                .strip_prefix("openai-compatible:")
                .map(str::trim)
                .filter(|profile_id| !profile_id.is_empty())
            else {
                continue;
            };
            if active_profile_id.as_deref() == Some(profile_id) {
                fallback = Some(profile_id.to_string());
                break;
            }
            if fallback.is_none() {
                fallback = Some(profile_id.to_string());
            }
        }

        crate::provider_catalog::openai_compatible_profile_by_id(&fallback?)
    }

    /// Return the active direct OpenAI-compatible runtime when its own catalog
    /// serves `model`. Bare model switches must stay on that runtime rather than
    /// rebinding the shared slot to native OpenRouter.
    pub(in crate::provider) fn active_openai_compatible_profile_serving_model(
        &self,
        model: &str,
    ) -> Option<Arc<dyn Provider>> {
        if self.active_provider() != ActiveProvider::OpenRouter {
            return None;
        }
        let provider = self.active_openrouter_execution_provider()?;
        if provider.supports_provider_routing_features() {
            return None;
        }
        let (_, api_method, _) = provider.direct_openai_compatible_route_parts()?;
        self.fresh_routes_memo_entry()
            .routes
            .iter()
            .any(|route| route.available && route.model == model && route.api_method == api_method)
            .then_some(provider)
    }

    /// Parse a `<name>:<model>` spec whose prefix is a user-defined named
    /// provider profile from config (`[providers.<name>]`). Built-in provider
    /// prefixes and catalog profile ids take precedence and never reach here.
    pub(in crate::provider) fn named_provider_profile_model_prefix(
        model: &str,
    ) -> Option<(String, String)> {
        let (prefix, rest) = model.split_once(':')?;
        if explicit_model_provider_prefix(model).is_some()
            || Self::openai_compatible_model_prefix(model).is_some()
        {
            return None;
        }
        let prefix = prefix.trim();
        let rest = rest.trim();
        if prefix.is_empty() || rest.is_empty() {
            return None;
        }
        crate::config::config()
            .providers
            .contains_key(prefix)
            .then(|| (prefix.to_string(), rest.to_string()))
    }
}
