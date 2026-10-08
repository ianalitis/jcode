use super::{
    OpenRouterProvider, endpoints_fingerprint, load_endpoints_disk_cache, models_fingerprint,
    normalize_api_base,
};
use std::collections::HashSet;

impl OpenRouterProvider {
    pub(crate) fn filter_profile_chat_supported_models(&self, models: Vec<String>) -> Vec<String> {
        let Some(profile_id) = self.profile_id.as_deref() else {
            return models;
        };

        models
            .into_iter()
            .filter(|model| {
                jcode_base::provider_catalog::openai_compatible_profile_model_supports_chat(
                    profile_id, model,
                )
            })
            .collect()
    }

    fn model_disk_cache_source_matches(
        &self,
        cache_entry: &jcode_provider_openrouter::DiskCache,
    ) -> bool {
        let Some(source_api_base) = cache_entry
            .source_api_base
            .as_deref()
            .and_then(normalize_api_base)
        else {
            // Legacy cache files did not record which endpoint produced the
            // catalog. They are acceptable for real OpenRouter catalogs, but
            // not for direct OpenAI-compatible profiles: a process-wide cache
            // namespace can leave an OpenRouter catalog under a profile such as
            // `chutes`, which then makes every picker row look like that direct
            // provider.
            return self.supports_provider_features;
        };

        source_api_base == self.api_base
    }

    pub(super) fn cached_model_catalog_fingerprint(&self) -> String {
        if let Ok(cache) = self.models_cache.try_read()
            && cache.fetched
        {
            return models_fingerprint(&cache.models);
        }
        if let Some(cache_entry) = self.load_usable_model_disk_cache_entry() {
            return models_fingerprint(&cache_entry.models);
        }
        String::new()
    }

    pub(crate) fn cached_live_model_ids_for_display(&self) -> Option<HashSet<String>> {
        if let Ok(cache) = self.models_cache.try_read()
            && cache.fetched
            && !cache.models.is_empty()
        {
            return Some(cache.models.iter().map(|model| model.id.clone()).collect());
        }

        self.load_usable_model_disk_cache_entry().and_then(|entry| {
            if entry.models.is_empty() {
                None
            } else {
                Some(entry.models.into_iter().map(|model| model.id).collect())
            }
        })
    }

    pub(super) fn cached_endpoints_fingerprint(&self, model: &str) -> String {
        if let Some(endpoints) = load_endpoints_disk_cache(model) {
            return endpoints_fingerprint(&endpoints);
        }
        if let Ok(cache) = self.endpoints_cache.try_read()
            && let Some((_, endpoints)) = cache.get(model)
        {
            return endpoints_fingerprint(endpoints);
        }
        String::new()
    }
}

impl OpenRouterProvider {
    /// The disk-cache namespace this provider's *foreground* catalog reads and
    /// writes should use.
    ///
    /// Every `new_named_openai_compatible()` constructor sets the process-global
    /// `JCODE_OPENROUTER_CACHE_NAMESPACE` env var, so with several named
    /// profiles in one process the last one constructed wins and all profiles
    /// collide on a single `<last-profile>_models.json`. The background refresh
    /// path already passes an explicit namespace; the foreground paths did not.
    /// See issue #607.
    ///
    /// Standard/direct OpenRouter and built-in profiles keep the existing
    /// env-var-driven `cache_path()` semantics.
    pub(crate) fn foreground_cache_namespace(&self) -> Option<String> {
        self.is_user_named_profile()
            .then(|| self.profile_id.clone())
            .flatten()
    }

    /// The disk cache entry usable for this provider, i.e. its own namespace
    /// (#607) and a `source_api_base` that matches this endpoint.
    pub(crate) fn load_usable_model_disk_cache_entry(
        &self,
    ) -> Option<jcode_provider_openrouter::DiskCache> {
        self.load_disk_cache_entry_for_this_profile()
            .filter(|entry| self.model_disk_cache_source_matches(entry))
    }

    /// Load this provider's own model disk cache, ignoring the process-global
    /// namespace env var for user-named profiles (#607).
    pub(crate) fn load_disk_cache_entry_for_this_profile(
        &self,
    ) -> Option<jcode_provider_openrouter::DiskCache> {
        match self.foreground_cache_namespace() {
            Some(namespace) => {
                jcode_provider_openrouter::load_disk_cache_entry_for_namespace(&namespace)
            }
            None => jcode_provider_openrouter::load_disk_cache_entry(),
        }
    }

    /// True when this instance was built from a user-declared
    /// `[providers.<name>]` profile in config.toml rather than a built-in
    /// OpenAI-compatible profile (Cerebras, NVIDIA NIM, ...).
    pub(crate) fn is_user_named_profile(&self) -> bool {
        let Some(id) = self.profile_id.as_deref() else {
            return false;
        };
        match jcode_base::provider_catalog::openai_compatible_profile_by_id(id) {
            // A `[providers.<name>]` block that shadows a built-in profile name
            // but points somewhere else is still a user-declared endpoint, so
            // its explicit model list must be preserved.
            Some(builtin) => normalize_api_base(builtin.api_base)
                .is_some_and(|builtin_base| builtin_base != self.api_base),
            None => true,
        }
    }

    pub(crate) fn should_merge_static_models_with_live_catalog(&self) -> bool {
        // Built-in OpenAI-compatible provider profiles use `static_models` as a
        // startup/pre-catalog fallback so `/model` is useful immediately after
        // login. Once a live `/models` catalog has been fetched, the live catalog
        // is more authoritative for access control. Keeping built-in fallback
        // entries after a successful fetch can advertise preview/stale models that
        // the provider rejects at chat time, which is especially confusing for
        // direct providers such as Cerebras.
        //
        // Preserve static models for OpenRouter itself and for custom/named
        // profiles, where the user supplied the list explicitly and there may be
        // no provider-side catalog contract. A profile id that is not in the
        // built-in OpenAI-compatible catalog comes from a user-declared
        // `[providers.<name>]` block in config.toml, so its
        // `[[providers.<name>.models]]` entries must survive background
        // `/models` catalog refreshes (issue #579).
        self.supports_provider_features || self.profile_id.is_none() || self.is_user_named_profile()
    }

    /// Whether the model catalog declares `image` as an accepted input modality
    /// for this model.
    ///
    /// Memory decides whenever it holds an opinion about the model, so a freshly
    /// fetched catalog is never overruled by a stale copy on disk. Only when
    /// memory is silent does the persisted catalog answer, and that fallback is
    /// what makes the first request after startup behave: the in-memory cache is
    /// initialised empty while the catalog is normally already on disk from the
    /// previous run, so memory alone would keep clamping images until the first
    /// refresh completed.
    ///
    /// `supports_image_input` is a sync trait method, so the in-memory read uses
    /// `try_read` rather than awaiting the tokio lock; a busy lock defers to the
    /// disk copy instead of blocking.
    pub(crate) fn catalog_declares_image_input(&self, model_id: &str) -> bool {
        if !self.supports_model_catalog {
            return false;
        }
        if let Ok(cache) = self.models_cache.try_read()
            && let Some(model) = cache
                .models
                .iter()
                .find(|model| model.id.trim().eq_ignore_ascii_case(model_id))
        {
            return declares_image_input(model);
        }
        self.load_usable_model_disk_cache_entry()
            .is_some_and(|entry| {
                entry
                    .models
                    .iter()
                    .find(|model| model.id.trim().eq_ignore_ascii_case(model_id))
                    .is_some_and(declares_image_input)
            })
    }
}

/// Whether one catalog entry declares `image` as an accepted input modality.
fn declares_image_input(model: &jcode_provider_openrouter::ModelInfo) -> bool {
    model
        .input
        .iter()
        .any(|modality| modality.eq_ignore_ascii_case("image"))
}
