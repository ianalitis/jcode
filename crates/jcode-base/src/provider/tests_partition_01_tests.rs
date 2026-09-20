#[test]
fn standard_openrouter_catalog_refresh_fires_when_named_profile_owns_slot() {
    with_clean_provider_test_env(|| {
        let runtime = enter_test_runtime();
        runtime.block_on(async {
            crate::provider_catalog::save_env_value_to_env_file(
                "OPENROUTER_API_KEY",
                "openrouter.env",
                Some("sk-test-openrouter"),
            )
            .expect("save openrouter key");
            // Simulate an active named profile (e.g. NVIDIA NIM) occupying the
            // shared OpenRouter/OpenAI-compatible slot: it sets the runtime env
            // vars to point at a non-openrouter.ai endpoint. The standard
            // OpenRouter catalog refresh must STILL fire so `/model` can list
            // openrouter.ai models (issue #292). Cache is missing -> not fresh.
            crate::env::set_var(
                "JCODE_OPENROUTER_API_BASE",
                "https://integrate.api.nvidia.com/v1",
            );
            crate::env::set_var("JCODE_OPENROUTER_CACHE_NAMESPACE", "mynvidia");

            // Other tests in this process may already have attempted (or be
            // running) an `openrouter` catalog refresh; clear the process-wide
            // backoff/in-flight tracker or this assertion is flaky under
            // parallel test execution.
            jcode_provider_openrouter_runtime::reset_profile_catalog_refresh_tracker_for_tests();

            assert!(
                openrouter::maybe_schedule_standard_openrouter_catalog_refresh(
                    "unit test named profile owns slot"
                ),
                "standard OpenRouter refresh must fire even when a named profile sets JCODE_OPENROUTER_* env"
            );
        });
    });
}

/// Parameterized test stand-in for provider runtimes that live downstream
/// (jcode-provider-{gemini,cursor,antigravity}-runtime) and therefore cannot
/// be constructed from base tests. Mirrors each runtime's catalog surface
/// (static model list plus `ModelRoute`s) so routing/fallback tests stay
/// meaningful.
struct StubExternalRuntime {
    name: &'static str,
    provider_label: &'static str,
    api_method: &'static str,
    models: &'static [&'static str],
    model: std::sync::RwLock<String>,
    credential_mode: std::sync::RwLock<jcode_provider_core::CredentialMode>,
}

impl StubExternalRuntime {
    fn new(
        name: &'static str,
        provider_label: &'static str,
        api_method: &'static str,
        models: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            provider_label,
            api_method,
            models,
            model: std::sync::RwLock::new(models[0].to_string()),
            credential_mode: std::sync::RwLock::new(jcode_provider_core::CredentialMode::Auto),
        }
    }

    fn cursor() -> Self {
        Self::new("cursor", "Cursor", "cursor", cursor::AVAILABLE_MODELS)
    }

    fn antigravity() -> Self {
        Self::new(
            "antigravity",
            "Antigravity",
            "https",
            antigravity::AVAILABLE_MODELS,
        )
    }

    fn copilot() -> Self {
        Self::new(
            "copilot",
            "GitHub Copilot",
            "copilot",
            copilot::FALLBACK_MODELS,
        )
    }

    fn anthropic() -> Self {
        Self::new(
            "anthropic",
            "Anthropic",
            "https",
            anthropic::AVAILABLE_MODELS,
        )
    }

    fn openai() -> Self {
        Self::new("openai", "OpenAI", "https", ALL_OPENAI_MODELS)
    }
}

#[async_trait::async_trait]
impl Provider for StubExternalRuntime {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> anyhow::Result<EventStream> {
        anyhow::bail!("stub {} runtime does not stream", self.name)
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn model(&self) -> String {
        self.model
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
    fn set_model(&self, model: &str) -> anyhow::Result<()> {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            anyhow::bail!("{} model cannot be empty", self.provider_label);
        }
        // Mirror the real runtimes' family validation: the registry is
        // process-global, so hot-init can hand this stub to tests that expect
        // cross-provider models to be rejected (e.g. a Claude model under a
        // forced-OpenAI selection).
        if !self.models.contains(&trimmed) {
            anyhow::bail!(
                "Unsupported {} model '{}'. Use /model to choose from the models available to your account.",
                self.provider_label,
                trimmed,
            );
        }
        *self
            .model
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = trimmed.to_string();
        Ok(())
    }
    fn available_models(&self) -> Vec<&'static str> {
        self.models.to_vec()
    }
    fn available_models_display(&self) -> Vec<String> {
        self.models.iter().map(|model| model.to_string()).collect()
    }
    fn available_models_for_switching(&self) -> Vec<String> {
        self.available_models_display()
    }
    fn model_routes(&self) -> Vec<ModelRoute> {
        self.available_models_display()
            .into_iter()
            .map(|model| ModelRoute {
                model,
                provider: self.provider_label.to_string(),
                api_method: self.api_method.to_string(),
                available: true,
                detail: String::new(),
                usage: None,
                cheapness: None,
            })
            .collect()
    }
    fn credential_mode(&self) -> jcode_provider_core::CredentialMode {
        *self
            .credential_mode
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    fn set_credential_mode(&self, mode: jcode_provider_core::CredentialMode) -> anyhow::Result<()> {
        *self
            .credential_mode
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = mode;
        Ok(())
    }
    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(StubExternalRuntime::new(
            self.name,
            self.provider_label,
            self.api_method,
            self.models,
        ))
    }
}

fn test_cursor_runtime() -> Arc<dyn Provider> {
    Arc::new(StubExternalRuntime::cursor())
}

fn test_antigravity_runtime() -> Arc<dyn Provider> {
    Arc::new(StubExternalRuntime::antigravity())
}

fn test_copilot_runtime() -> Arc<dyn Provider> {
    Arc::new(StubExternalRuntime::copilot())
}

fn test_anthropic_runtime() -> Arc<StubExternalRuntime> {
    Arc::new(StubExternalRuntime::anthropic())
}

fn test_openai_runtime() -> Arc<StubExternalRuntime> {
    Arc::new(StubExternalRuntime::openai())
}

/// Register the shared external-runtime stubs for every downstream provider
/// slot base can hot-initialize. Called by `with_clean_provider_test_env` so
/// hot-init/startup tests find a runtime the way the real binary does.
fn register_test_external_runtimes() {
    external::register_external_provider(external::ANTHROPIC_RUNTIME, || {
        test_anthropic_runtime() as Arc<dyn Provider>
    });
    external::register_external_provider(external::OPENAI_RUNTIME, || {
        test_openai_runtime() as Arc<dyn Provider>
    });
    external::register_external_provider(external::CURSOR_RUNTIME, test_cursor_runtime);
    external::register_external_provider(external::ANTIGRAVITY_RUNTIME, test_antigravity_runtime);
    external::register_external_provider(external::COPILOT_RUNTIME, test_copilot_runtime);
    // OpenRouter tests exercise the real runtime (profile-scoped catalogs,
    // transport identities), so register the real factory like the binary's
    // composition root does. The dev-dependency cycle is test-only.
    external::register_openrouter_factory(|spec| {
        use external::OpenRouterRuntimeSpec;
        use jcode_provider_openrouter_runtime::OpenRouterProvider;
        let provider: Arc<dyn Provider> = match spec {
            OpenRouterRuntimeSpec::Default => Arc::new(OpenRouterProvider::new()?),
            OpenRouterRuntimeSpec::OpenRouterApiKey => {
                Arc::new(OpenRouterProvider::new_openrouter_api_key_runtime()?)
            }
            OpenRouterRuntimeSpec::CompatibleProfile(profile) => Arc::new(
                OpenRouterProvider::new_openai_compatible_profile_runtime(profile)?,
            ),
            OpenRouterRuntimeSpec::NamedProfile { name, config } => Arc::new(
                OpenRouterProvider::new_named_openai_compatible(&name, &config)?,
            ),
        };
        Ok(provider)
    });
    external::register_profile_catalog_refresh(
        jcode_provider_openrouter_runtime::maybe_schedule_openai_compatible_profile_catalog_refresh,
    );
    external::register_standard_openrouter_catalog_refresh(
        jcode_provider_openrouter_runtime::maybe_schedule_standard_openrouter_catalog_refresh,
    );
}

/// Construct a real OpenRouter/OpenAI-compatible runtime for tests through
/// the registry, mirroring production construction.
fn test_openrouter_runtime() -> anyhow::Result<Arc<dyn Provider>> {
    external::instantiate_openrouter_runtime(external::OpenRouterRuntimeSpec::Default)
}

fn test_multi_provider_with_cursor() -> MultiProvider {
    MultiProvider {
        claude: RwLock::new(None),
        anthropic: RwLock::new(None),
        openai: RwLock::new(None),
        copilot_api: RwLock::new(None),
        antigravity: RwLock::new(None),
        gemini: RwLock::new(None),
        cursor: RwLock::new(Some(test_cursor_runtime())),
        bedrock: RwLock::new(None),
        openrouter: RwLock::new(None),
        openai_compatible_profiles: RwLock::new(std::collections::HashMap::new()),
        active_openai_compatible_profile: RwLock::new(None),
        active: RwLock::new(ActiveProvider::Cursor),
        use_claude_cli: false,
        startup_notices: RwLock::new(Vec::new()),
        initial_provider: None,
        routes_memo: std::sync::Mutex::new(None),
        post_auth_refreshes_pending: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    }
}

struct PrewarmRecordingProvider {
    name: &'static str,
    prewarms: Arc<std::sync::atomic::AtomicUsize>,
    completions: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait::async_trait]
impl Provider for PrewarmRecordingProvider {
    async fn prewarm(&self, _tools: &[ToolDefinition], _system_static: &str) {
        self.prewarms
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> anyhow::Result<EventStream> {
        self.completions
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        anyhow::bail!("recording provider must not complete")
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self {
            name: self.name,
            prewarms: Arc::clone(&self.prewarms),
            completions: Arc::clone(&self.completions),
        })
    }
}

#[test]
fn prewarm_delegates_only_to_active_provider_without_completing() {
    let runtime = enter_test_runtime();
    runtime.block_on(async {
        let active_prewarms = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inactive_prewarms = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let completions = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let provider = test_multi_provider_with_cursor();
        *provider
            .cursor
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(Arc::new(PrewarmRecordingProvider {
                name: "active",
                prewarms: Arc::clone(&active_prewarms),
                completions: Arc::clone(&completions),
            }));
        *provider
            .openai
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(Arc::new(PrewarmRecordingProvider {
                name: "inactive",
                prewarms: Arc::clone(&inactive_prewarms),
                completions: Arc::clone(&completions),
            }));

        provider.prewarm(&[], "static instructions").await;

        assert_eq!(active_prewarms.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            inactive_prewarms.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(completions.load(std::sync::atomic::Ordering::SeqCst), 0);
    });
}

#[test]
fn new_session_fork_reloads_changed_config_provider_and_model() {
    with_clean_provider_test_env(|| {
        let runtime = enter_test_runtime();
        runtime.block_on(async {
            crate::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test");
            crate::env::set_var("OPENAI_API_KEY", "sk-openai-test");

            crate::config::Config::set_default_model(Some("claude-fable-5"), Some("anthropic-api"))
                .expect("save initial Claude default");
            let template = MultiProvider::new_fast();
            assert_eq!(template.name(), "Claude");
            assert_eq!(template.model(), "claude-fable-5");

            crate::config::Config::set_default_model(Some("gpt-5.5"), Some("openai-api"))
                .expect("save changed OpenAI default");

            let fresh = template.fork_for_new_session();
            assert_eq!(fresh.name(), "OpenAI");
            assert_eq!(fresh.model(), "gpt-5.5");
            assert_eq!(
                fresh.active_resolved_credential(),
                Some(jcode_provider_core::ResolvedCredential::ApiKey)
            );

            // Ordinary forks still preserve the existing session's selection.
            let preserved = template.fork();
            assert_eq!(preserved.name(), "Claude");
            assert_eq!(preserved.model(), "claude-fable-5");
        });
    });
}

include!("tests/auth_refresh.rs");
include!("tests/model_resolution.rs");
include!("tests/issue_534_profile_preservation.rs");
include!("tests/fallback_failover.rs");
include!("tests/catalog_subscription.rs");

/// Rendering the route catalog must never schedule network work.
///
/// Regression guard for the "spawning a session refetches every provider
/// catalog" bug: route building used to schedule a background `/models` fetch
/// for each stale or missing profile cache, so every session attach and picker
/// open fanned out dozens of HTTP requests. Refresh cadence now belongs solely
/// to the background catalog scheduler.
#[test]
fn building_direct_profile_routes_does_not_schedule_catalog_refreshes() {
    with_clean_provider_test_env(|| {
        let runtime = enter_test_runtime();
        runtime.block_on(async {
            crate::provider_catalog::save_env_value_to_env_file(
                "OPENROUTER_API_KEY",
                "openrouter.env",
                Some("sk-test-openrouter"),
            )
            .expect("save openrouter key");

            // Deliberately leave every profile cache missing/stale: under the
            // old behavior this was the worst case that scheduled a refresh
            // for each configured profile.
            jcode_provider_openrouter_runtime::reset_profile_catalog_refresh_tracker_for_tests();

            for profile in crate::provider_catalog::openai_compatible_profiles()
                .iter()
                .copied()
            {
                let _ = super::direct_openai_compatible_profile_routes(profile);
            }

            // If route building had scheduled refreshes, the profile tracker
            // would have recorded attempts, and this direct call for the
            // standard OpenRouter namespace would be throttled/in-flight.
            assert!(
                openrouter::maybe_schedule_standard_openrouter_catalog_refresh(
                    "unit test post-render scheduling"
                ),
                "route building must leave the refresh tracker untouched"
            );
        });
    });
}

/// The scheduler's staleness predicate must treat a missing or mismatched
/// cache as needing a refresh, so the sweeper actually populates cold caches.
#[test]
fn profile_catalog_cache_needs_refresh_for_missing_cache() {
    with_clean_provider_test_env(|| {
        let profile = crate::provider_catalog::openai_compatible_profiles()
            .first()
            .copied()
            .expect("at least one OpenAI-compatible profile is defined");
        assert!(
            super::catalog_scheduler::profile_catalog_cache_needs_refresh(profile),
            "a missing catalog cache must be reported as needing a refresh"
        );
    });
}
