use super::*;
use tempfile::TempDir;

fn save_gemini_validation(success: bool, summary: &str) {
    crate::auth::validation::save(
        "gemini",
        crate::auth::validation::ProviderValidationRecord {
            checked_at_ms: chrono::Utc::now().timestamp_millis(),
            success,
            provider_smoke_ok: Some(success),
            tool_smoke_ok: None,
            summary: summary.to_string(),
        },
    )
    .expect("save Gemini validation");
    crate::auth::AuthStatus::invalidate_cache();
}

fn has_gemini_route(provider: &dyn crate::provider::Provider) -> bool {
    provider
        .model_routes()
        .iter()
        .any(|route| route.provider == "Gemini")
}

#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::await_holding_lock,
    reason = "test env locks intentionally stay held across provider detection to isolate process-global auth env"
)]
async fn auto_omits_gemini_after_individual_client_retirement() {
    const RETIREMENT: &str = "provider_smoke: This client is no longer supported for Gemini Code Assist for individuals.";
    let _guard = lock_env();
    let _env_guard = crate::storage::lock_test_env();
    let dir = TempDir::new().expect("temp dir");
    let keys = [
        "JCODE_HOME",
        "JCODE_NON_INTERACTIVE",
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "OPENROUTER_API_KEY",
        "JCODE_RELOAD_AUTH_STATUS",
        "JCODE_RUNTIME_PROVIDER",
        "JCODE_ACTIVE_PROVIDER",
        "JCODE_INITIAL_PROVIDER_EXPLICIT",
        "JCODE_GEMINI_FORCE_OAUTH",
    ];
    let saved: Vec<(&str, Option<String>)> = keys
        .iter()
        .map(|key| (*key, std::env::var(key).ok()))
        .collect();

    crate::env::set_var("JCODE_HOME", dir.path());
    crate::env::set_var("JCODE_NON_INTERACTIVE", "1");
    for key in keys.iter().skip(2) {
        crate::env::remove_var(key);
    }
    crate::auth::gemini::save_tokens(&crate::auth::gemini::GeminiTokens {
        access_token: "test-access-token".to_string(),
        refresh_token: "test-refresh-token".to_string(),
        expires_at: i64::MAX,
        email: None,
    })
    .expect("save Gemini OAuth tokens");
    save_gemini_validation(false, RETIREMENT);
    crate::config::invalidate_config_cache();

    let unavailable = detect_auto_provider_flags().await;
    assert_eq!(unavailable.auth_status.gemini, auth::AuthState::Available);
    assert!(!unavailable.has_gemini);
    assert!(!maybe_enable_gemini_auth_for_auto(false).expect("supplemental Gemini detection"));
    assert!(
        !has_gemini_route(&crate::provider::MultiProvider::new_fast()),
        "default automatic MultiProvider construction must honor the compatibility block"
    );

    crate::env::set_var("OPENROUTER_API_KEY", "test-openrouter-key");
    crate::auth::AuthStatus::invalidate_cache();
    let auto_without_gemini = init_provider_for_validation(&ProviderChoice::Auto, None)
        .await
        .expect("another provider should keep auto initialization available");
    assert!(!has_gemini_route(auto_without_gemini.as_ref()));
    auto_without_gemini.on_auth_changed();
    assert!(!has_gemini_route(auto_without_gemini.as_ref()));
    assert!(!has_gemini_route(
        auto_without_gemini.fork_for_new_session().as_ref()
    ));

    crate::env::set_var("GEMINI_API_KEY", "test-api-key");
    crate::env::set_var("JCODE_GEMINI_FORCE_OAUTH", "1");
    crate::auth::AuthStatus::invalidate_cache();
    assert!(
        !detect_auto_provider_flags().await.has_gemini,
        "an API key must not bypass the block when the runtime is forced to OAuth"
    );
    crate::auth::gemini::clear_tokens().expect("clear Gemini OAuth tokens");
    crate::auth::AuthStatus::invalidate_cache();
    assert!(
        !has_gemini_route(&crate::provider::MultiProvider::new_fast()),
        "forced OAuth must not register Gemini from an API key alone"
    );
    crate::env::remove_var("JCODE_GEMINI_FORCE_OAUTH");
    crate::auth::AuthStatus::invalidate_cache();
    assert!(detect_auto_provider_flags().await.has_gemini);
    assert!(has_gemini_route(&crate::provider::MultiProvider::new_fast()));
    let auto_with_api_key = init_provider_for_validation(&ProviderChoice::Auto, None)
        .await
        .expect("Gemini API key should remain auto-routable");
    assert!(has_gemini_route(auto_with_api_key.as_ref()));
    crate::env::remove_var("GEMINI_API_KEY");
    crate::auth::gemini::save_tokens(&crate::auth::gemini::GeminiTokens {
        access_token: "test-access-token".to_string(),
        refresh_token: "test-refresh-token".to_string(),
        expires_at: i64::MAX,
        email: None,
    })
    .expect("restore Gemini OAuth tokens");

    save_gemini_validation(true, "provider_smoke: AUTH_TEST_OK");
    assert!(detect_auto_provider_flags().await.has_gemini);
    let auto_after_recovery = init_provider_for_validation(&ProviderChoice::Auto, None)
        .await
        .expect("successful Gemini validation should restore auto routing");
    assert!(has_gemini_route(auto_after_recovery.as_ref()));
    assert!(has_gemini_route(
        auto_after_recovery.fork_for_new_session().as_ref()
    ));
    save_gemini_validation(false, RETIREMENT);
    auto_after_recovery.on_auth_changed();
    assert!(
        !has_gemini_route(auto_after_recovery.as_ref()),
        "auth refresh must remove a now-ineligible Gemini route from Auto"
    );

    for (key, value) in saved {
        if let Some(value) = value {
            crate::env::set_var(key, value);
        } else {
            crate::env::remove_var(key);
        }
    }
    crate::config::invalidate_config_cache();
    crate::auth::AuthStatus::invalidate_cache();
}
