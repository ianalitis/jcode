use anyhow::{Context, ensure};

use jcode_base::auth::lifecycle::{
    AuthActivationRequest, AuthActivationResult, AuthCatalogInvariantReport, activate_auth_change,
    provider_model_to_select_after_auth, validate_catalog_invariants,
};
use jcode_base::auth::test_sandbox::AuthTestSandbox;
use jcode_base::protocol::{
    AuthChanged, AuthCredentialSource, AuthMethod, CatalogNamespace, RuntimeProviderKey,
};
use jcode_base::provider::ModelRoute;
use jcode_base::provider_catalog::OpenAiCompatibleProfile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthLifecycleAuthPath {
    TuiPasteApiKey,
    RemoteTuiPasteApiKey,
    CliLogin,
    EnvFilePreseeded,
    ProcessEnvPreseeded,
}

impl AuthLifecycleAuthPath {
    fn auth_method(self) -> AuthMethod {
        match self {
            Self::TuiPasteApiKey => AuthMethod::TuiPasteApiKey,
            Self::RemoteTuiPasteApiKey => AuthMethod::RemoteTuiPasteApiKey,
            Self::CliLogin => AuthMethod::CliLogin,
            Self::EnvFilePreseeded => AuthMethod::EnvFilePreseeded,
            Self::ProcessEnvPreseeded => AuthMethod::ProcessEnvPreseeded,
        }
    }

    fn credential_source(self) -> AuthCredentialSource {
        match self {
            Self::TuiPasteApiKey
            | Self::RemoteTuiPasteApiKey
            | Self::CliLogin
            | Self::EnvFilePreseeded => AuthCredentialSource::ApiKeyFile,
            Self::ProcessEnvPreseeded => AuthCredentialSource::ProcessEnv,
        }
    }

    fn shows_paste_prompt(self) -> bool {
        matches!(self, Self::TuiPasteApiKey | Self::RemoteTuiPasteApiKey)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AuthLifecycleSpec {
    pub provider_id: &'static str,
    pub provider_label: &'static str,
    pub profile: OpenAiCompatibleProfile,
    pub auth_path: AuthLifecycleAuthPath,
    pub api_key: String,
    pub catalog_models_after_auth: Vec<String>,
    pub selected_model_override: Option<String>,
    pub current_runtime_provider_name: &'static str,
}

impl AuthLifecycleSpec {
    pub(crate) fn cerebras_fixture(auth_path: AuthLifecycleAuthPath) -> Self {
        let mut spec = Self::openai_compatible_fixture(
            jcode_base::provider_catalog::CEREBRAS_PROFILE,
            auth_path,
        );
        spec.catalog_models_after_auth = vec![
            "qwen-3-235b-a22b-instruct-2507".to_string(),
            "llama3.1-8b".to_string(),
        ];
        spec.selected_model_override = None;
        spec
    }

    pub(crate) fn openai_compatible_fixture(
        profile: OpenAiCompatibleProfile,
        auth_path: AuthLifecycleAuthPath,
    ) -> Self {
        let default_model = profile.default_model.unwrap_or("fixture-model");
        let mut catalog_models_after_auth = vec![default_model.to_string()];
        catalog_models_after_auth.push(format!("{}-alternate-fixture-model", profile.id));
        Self {
            provider_id: profile.id,
            provider_label: profile.display_name,
            profile,
            auth_path,
            api_key: format!("test-{}-key", profile.id),
            catalog_models_after_auth,
            selected_model_override: profile
                .default_model
                .is_none()
                .then(|| default_model.to_string()),
            current_runtime_provider_name: "mock-auth",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PickerSnapshot {
    pub selected_model: Option<String>,
    pub provider_entries: Vec<String>,
    pub switch_target: Option<String>,
    pub switch_request: Option<String>,
    pub switch_route_provider: Option<String>,
    pub switch_route_api_method: Option<String>,
}

impl PickerSnapshot {
    fn build(
        spec: &AuthLifecycleSpec,
        activation: &AuthActivationResult,
        selected_model: Option<&str>,
        routes: &[ModelRoute],
    ) -> Self {
        let provider_routes = routes
            .iter()
            .filter(|route| route.available && route_matches_spec(route, spec))
            .collect::<Vec<_>>();
        let provider_entries = provider_routes
            .iter()
            .map(|route| route.model.clone())
            .collect::<Vec<_>>();
        let selected_model = selected_model
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(ToString::to_string);
        let switch_target = provider_entries
            .iter()
            .find(|model| Some(model.as_str()) != selected_model.as_deref())
            .or_else(|| provider_entries.first())
            .cloned();
        let switch_request = switch_target.as_deref().map(|model| {
            activation.model_switch_request(spec.current_runtime_provider_name, model)
        });
        let switch_route = switch_target.as_ref().and_then(|target| {
            provider_routes
                .iter()
                .find(|route| route.model == *target)
                .copied()
        });

        Self {
            selected_model,
            provider_entries,
            switch_target,
            switch_request,
            switch_route_provider: switch_route.map(|route| route.provider.clone()),
            switch_route_api_method: switch_route.map(|route| route.api_method.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AuthLifecycleResult {
    pub activation: AuthActivationResult,
    pub transcript: Vec<String>,
    pub catalog_report: AuthCatalogInvariantReport,
    pub picker: PickerSnapshot,
    pub catalog_routes: Vec<ModelRoute>,
    pub credential_location: Option<String>,
}

impl AuthLifecycleResult {
    pub(crate) fn assert_success(&self, spec: &AuthLifecycleSpec) {
        let transcript = self.transcript_text();
        assert!(self.catalog_report.ok(), "{}", self.failure_report(spec));
        assert_eq!(
            self.activation.provider_id.as_deref(),
            Some(spec.provider_id)
        );
        assert_eq!(
            self.activation.provider_label.as_deref(),
            Some(spec.provider_label)
        );
        assert_eq!(
            self.activation.expected_runtime.as_deref(),
            Some("openai-compatible")
        );
        assert_eq!(
            self.activation.expected_catalog_namespace.as_deref(),
            Some(spec.provider_id)
        );
        assert!(
            transcript.contains(&format!("{} catalog changed", spec.provider_label)),
            "{}",
            self.failure_report(spec)
        );
        // Anti-confusion guard: the transcript must never attribute the new
        // access to a different provider. Skip the marker matching the provider
        // under test: OpenRouter/OpenAI-labelled compat profiles legitimately
        // produce their own catalog-changed line.
        for other_label in ["OpenAI", "OpenRouter"] {
            if spec.provider_label == other_label {
                continue;
            }
            assert!(
                !transcript.contains(&format!("{other_label} catalog changed")),
                "{}",
                self.failure_report(spec)
            );
        }
        self.assert_transcript_order(spec);
        for forbidden in [
            "Auth Model Catalog Warning",
            "did not switch models",
            "contained no selectable",
            "Login: failed",
            "failed",
            "Unable to sign in",
            "Saved the API key and fetched the model catalog, but",
        ] {
            assert!(
                !transcript.contains(forbidden),
                "happy auth lifecycle transcript contained forbidden degraded-success marker `{forbidden}`:\n{}",
                self.failure_report(spec)
            );
        }
        assert!(
            !self.picker.provider_entries.is_empty(),
            "{}",
            self.failure_report(spec)
        );
        assert!(
            self.picker
                .selected_model
                .as_ref()
                .is_some_and(|selected| self
                    .picker
                    .provider_entries
                    .iter()
                    .any(|entry| entry == selected)),
            "{}",
            self.failure_report(spec)
        );
        assert!(
            self.picker.switch_target.is_some(),
            "{}",
            self.failure_report(spec)
        );
        assert!(
            self.picker
                .switch_request
                .as_ref()
                .is_some_and(|request| request.starts_with(&format!("{}:", spec.provider_id))),
            "{}",
            self.failure_report(spec)
        );
        assert!(
            self.picker
                .switch_route_api_method
                .as_deref()
                .is_some_and(|api_method| api_method
                    .eq_ignore_ascii_case(&format!("openai-compatible:{}", spec.provider_id))
                    || api_method.eq_ignore_ascii_case(spec.provider_id)),
            "{}",
            self.failure_report(spec)
        );
        let matching_routes = self
            .catalog_routes
            .iter()
            .filter(|route| route.available && route_matches_spec(route, spec))
            .collect::<Vec<_>>();
        assert!(
            matching_routes.iter().all(|route| spec
                .catalog_models_after_auth
                .iter()
                .any(|model| model == &route.model)),
            "happy auth lifecycle advertised provider routes that were not returned by the live catalog:\n{}",
            self.failure_report(spec)
        );
        assert!(
            self.picker.provider_entries.iter().all(|entry| spec
                .catalog_models_after_auth
                .iter()
                .any(|model| model == entry)),
            "happy auth lifecycle picker included models that were not returned by the live catalog:\n{}",
            self.failure_report(spec)
        );
        assert!(
            matching_routes
                .iter()
                .all(|route| route.detail.contains("live-catalog")),
            "happy auth lifecycle must be backed by live/provider catalog routes, not static fallback routes:\n{}",
            self.failure_report(spec)
        );
        assert!(
            matching_routes.iter().all(|route| !route
                .detail
                .to_ascii_lowercase()
                .contains("static fallback")),
            "happy auth lifecycle accepted a static fallback route:\n{}",
            self.failure_report(spec)
        );
    }

    fn assert_transcript_order(&self, spec: &AuthLifecycleSpec) {
        let transcript = self.transcript_text();
        let saved_or_detected = if spec.auth_path.shows_paste_prompt() {
            format!("**{} API key saved.**", spec.provider_label)
        } else {
            format!("**{} credentials detected.**", spec.provider_label)
        };
        let markers = [saved_or_detected.as_str(), "**Model ready:**"];
        let mut previous = None;
        for marker in markers {
            let first = transcript.find(marker).unwrap_or_else(|| {
                panic!(
                    "happy auth lifecycle transcript is missing `{marker}`:\n{}",
                    self.failure_report(spec)
                )
            });
            let last = transcript.rfind(marker).expect("marker found above");
            assert_eq!(
                first,
                last,
                "happy auth lifecycle transcript contained duplicate `{marker}`:\n{}",
                self.failure_report(spec)
            );
            if let Some(previous) = previous {
                assert!(
                    previous < first,
                    "happy auth lifecycle transcript marker `{marker}` was out of order:\n{}",
                    self.failure_report(spec)
                );
            }
            previous = Some(first);
        }
    }

    pub(crate) fn transcript_text(&self) -> String {
        self.transcript.join("\n\n")
    }

    pub(crate) fn failure_report(&self, spec: &AuthLifecycleSpec) -> String {
        let warning = self
            .catalog_report
            .warning_message()
            .unwrap_or_else(|| "none".to_string());
        let route_sample = self
            .catalog_routes
            .iter()
            .take(8)
            .map(|route| {
                format!(
                    "{} via {} provider={} available={}",
                    route.model, route.api_method, route.provider, route.available
                )
            })
            .collect::<Vec<_>>()
            .join("\n  ");
        format!(
            "auth lifecycle failed for {} via {:?}\ncredential: {:?}\nactivation: {:?}\ncatalog invariant: {:?}\nwarning: {}\npicker: {:?}\nroutes:\n  {}\ntranscript:\n{}",
            spec.provider_label,
            spec.auth_path,
            self.credential_location,
            self.activation,
            self.catalog_report,
            warning,
            self.picker,
            route_sample,
            self.transcript_text()
        )
    }
}

pub(crate) struct AuthLifecycleDriver {
    sandbox: AuthTestSandbox,
}

impl AuthLifecycleDriver {
    pub(crate) fn new() -> anyhow::Result<Self> {
        Ok(Self {
            sandbox: AuthTestSandbox::new()?,
        })
    }

    pub(crate) fn run_openai_compatible_fixture(
        &self,
        spec: &AuthLifecycleSpec,
    ) -> anyhow::Result<AuthLifecycleResult> {
        let resolved =
            jcode_base::provider_catalog::resolve_openai_compatible_profile(spec.profile);
        ensure!(
            resolved.id == spec.provider_id,
            "spec provider id {} did not match profile {}",
            spec.provider_id,
            resolved.id
        );

        let credential_location = self.apply_credentials(spec, &resolved)?;
        let auth = AuthChanged {
            provider: jcode_base::protocol::AuthProviderId::new(spec.provider_id),
            credential_source: Some(spec.auth_path.credential_source()),
            auth_method: Some(spec.auth_path.auth_method()),
            expected_runtime: Some(RuntimeProviderKey::new("openai-compatible")),
            expected_catalog_namespace: Some(CatalogNamespace::new(spec.provider_id)),
        };
        let activation = activate_auth_change(&AuthActivationRequest::new(None, Some(auth)));
        let catalog_routes = self.catalog_routes_for_spec(spec);
        // The session's model immediately after activation: an explicit
        // fixture override, else whatever runtime activation selected (the
        // profile's static default model).
        let session_model = spec
            .selected_model_override
            .clone()
            .or_else(|| activation.activated_model.clone());
        // Mirror the server's post-auth re-selection
        // (`handle_notify_auth_changed`): once the live catalog lands, jcode
        // switches to an accessible model returned by the live catalog when
        // the current model has no matching provider route (e.g. a static
        // profile default the live catalog no longer serves).
        let selected_model = provider_model_to_select_after_auth(
            &activation,
            session_model.as_deref(),
            &catalog_routes,
        )
        .or(session_model);
        let catalog_report =
            validate_catalog_invariants(&activation, selected_model.as_deref(), &catalog_routes);
        let picker = PickerSnapshot::build(
            spec,
            &activation,
            selected_model.as_deref(),
            &catalog_routes,
        );
        let transcript = self.user_visible_transcript(
            spec,
            &resolved,
            selected_model.as_deref(),
            catalog_report.warning_message().as_deref(),
        );

        Ok(AuthLifecycleResult {
            activation,
            transcript,
            catalog_report,
            picker,
            catalog_routes,
            credential_location,
        })
    }

    fn apply_credentials(
        &self,
        spec: &AuthLifecycleSpec,
        resolved: &jcode_base::provider_catalog::ResolvedOpenAiCompatibleProfile,
    ) -> anyhow::Result<Option<String>> {
        match spec.auth_path {
            AuthLifecycleAuthPath::TuiPasteApiKey
            | AuthLifecycleAuthPath::RemoteTuiPasteApiKey
            | AuthLifecycleAuthPath::CliLogin
            | AuthLifecycleAuthPath::EnvFilePreseeded => {
                let path = self
                    .sandbox
                    .write_openai_compatible_api_key(spec.profile, &spec.api_key)
                    .with_context(|| format!("write {} API key file", spec.provider_label))?;
                Ok(Some(path.display().to_string()))
            }
            AuthLifecycleAuthPath::ProcessEnvPreseeded => {
                jcode_base::env::set_var(&resolved.api_key_env, &spec.api_key);
                jcode_base::auth::AuthStatus::invalidate_cache();
                Ok(Some(format!("process env {}", resolved.api_key_env)))
            }
        }
    }

    fn catalog_routes_for_spec(&self, spec: &AuthLifecycleSpec) -> Vec<ModelRoute> {
        spec.catalog_models_after_auth
            .iter()
            .map(|model| ModelRoute {
                model: model.clone(),
                provider: spec.provider_label.to_string(),
                api_method: format!("openai-compatible:{}", spec.provider_id),
                available: true,
                detail: "fixture live-catalog route".to_string(),
                usage: None,
                cheapness: None,
            })
            .collect()
    }

    fn user_visible_transcript(
        &self,
        spec: &AuthLifecycleSpec,
        resolved: &jcode_base::provider_catalog::ResolvedOpenAiCompatibleProfile,
        selected_model: Option<&str>,
        warning: Option<&str>,
    ) -> Vec<String> {
        let mut transcript = Vec::new();
        if spec.auth_path.shows_paste_prompt() {
            transcript.push(format!(
                "**{} API Key**\n\nSetup docs: {}\nStored variable: `{}`\nEndpoint: `{}`\nSuggested default model: `{}`\n\n**Paste your API key below** (it will be saved securely), or type `/cancel` to abort.",
                spec.provider_label,
                resolved.setup_url,
                resolved.api_key_env,
                resolved.api_base,
                resolved.default_model.as_deref().unwrap_or("none")
            ));
            transcript.push(format!(
                "**{} API key saved.**\n\nStored at `{}`.\nFetching models now. Jcode will switch to an accessible model returned by the live catalog and show the catalog diff when discovery finishes.",
                spec.provider_label,
                self.sandbox.env_file_path(&resolved.env_file).display()
            ));
        } else {
            transcript.push(format!(
                "**{} credentials detected.**\n\nCredential source: {:?}. Fetching models now.",
                spec.provider_label,
                spec.auth_path.credential_source()
            ));
        }
        let catalog_status = if warning.is_some() {
            format!(
                "{} catalog changed; some routes missing. Use `/model`.",
                spec.provider_label
            )
        } else {
            format!("{} catalog changed. Use `/model`.", spec.provider_label)
        };
        transcript.push(format!(
            "**Model ready:** `{}`\n{catalog_status}",
            selected_model.unwrap_or("none"),
        ));
        transcript
    }
}

fn route_matches_spec(route: &ModelRoute, spec: &AuthLifecycleSpec) -> bool {
    route
        .api_method
        .eq_ignore_ascii_case(&format!("openai-compatible:{}", spec.provider_id))
        || route.api_method.eq_ignore_ascii_case(spec.provider_id)
}

// The live provider probes were moved to `live_provider_probes` so they compile
// into the shipping binary (used by `provider_e2e`). Re-export for the tests
// below that reference them via `super::*`.
#[cfg(test)]
pub(crate) use crate::live_provider_probes::{
    fetch_live_openai_compatible_models, run_live_openai_compatible_smoke,
    run_live_openai_compatible_stream_smoke, run_live_openai_compatible_tool_smoke,
};

#[cfg(test)]
mod tests {
    include!("lifecycle_driver_tests_body_tests.rs");
}
