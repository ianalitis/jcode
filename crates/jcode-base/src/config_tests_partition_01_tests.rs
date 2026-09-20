#[test]
fn test_external_auth_source_allowed_for_path_matches_saved_entry() {
    let _guard = crate::storage::lock_test_env();
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("auth.json");
    std::fs::write(&path, "{}\n").expect("write auth file");

    let canonical = std::fs::canonicalize(&path).expect("canonical path");
    let mut cfg = Config::default();
    cfg.auth.trusted_external_source_paths = vec![format!(
        "test_source|{}",
        canonical.to_string_lossy().to_ascii_lowercase()
    )];

    assert!(cfg.external_auth_source_allowed_for_path_config("test_source", &path));
}

#[test]
fn test_external_auth_source_allowed_for_path_ignores_broad_legacy_entry() {
    let _guard = crate::storage::lock_test_env();
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("auth.json");
    std::fs::write(&path, "{}\n").expect("write auth file");

    let mut cfg = Config::default();
    cfg.auth.trusted_external_sources = vec!["test_source".to_string()];

    assert!(!cfg.external_auth_source_allowed_for_path_config("test_source", &path));
}

/// Regression test for issue #349: a removed/unknown `update_channel` value
/// (older configs could contain `"manual"`) must not fail the whole config
/// parse. A hard parse failure during the reload handoff left the reload
/// marker stuck in `starting` and clients re-requested the reload forever.
#[test]
fn unknown_update_channel_value_falls_back_to_stable_instead_of_failing_parse() {
    let cfg: Config = toml::from_str("[features]\nupdate_channel = \"manual\"\n")
        .expect("unknown update_channel must not fail config parse");
    assert_eq!(
        cfg.features.update_channel,
        super::UpdateChannel::Stable,
        "unknown channel should fall back to the default"
    );

    // Other settings in the same config must survive the fallback.
    let cfg: Config = toml::from_str(
        "[features]\nupdate_channel = \"manual\"\nmemory = false\n\n[display]\ncentered = true\n",
    )
    .expect("config with unknown update_channel should parse");
    assert_eq!(cfg.features.update_channel, super::UpdateChannel::Stable);
    assert!(!cfg.features.memory);
    assert!(cfg.display.centered);
}

#[test]
fn known_update_channel_values_still_parse() {
    let cfg: Config = toml::from_str("[features]\nupdate_channel = \"main\"\n")
        .expect("main update_channel should parse");
    assert_eq!(cfg.features.update_channel, super::UpdateChannel::Main);

    let cfg: Config = toml::from_str("[features]\nupdate_channel = \"stable\"\n")
        .expect("stable update_channel should parse");
    assert_eq!(cfg.features.update_channel, super::UpdateChannel::Stable);
}

#[test]
fn update_channel_parse_accepts_known_aliases_and_rejects_unknown() {
    use super::UpdateChannel;
    assert_eq!(UpdateChannel::parse("stable"), Some(UpdateChannel::Stable));
    assert_eq!(UpdateChannel::parse("release"), Some(UpdateChannel::Stable));
    assert_eq!(UpdateChannel::parse("main"), Some(UpdateChannel::Main));
    assert_eq!(UpdateChannel::parse("nightly"), Some(UpdateChannel::Main));
    assert_eq!(UpdateChannel::parse("edge"), Some(UpdateChannel::Main));
    assert_eq!(UpdateChannel::parse(" Main "), Some(UpdateChannel::Main));
    assert_eq!(UpdateChannel::parse("manual"), None);
    assert_eq!(UpdateChannel::parse(""), None);
}

impl Config {
    fn external_auth_source_allowed_for_path_config(&self, source_id: &str, path: &Path) -> bool {
        let Ok(entry) = Self::trusted_external_auth_path_entry(source_id, path) else {
            return false;
        };
        self.auth
            .trusted_external_source_paths
            .iter()
            .any(|value| value.trim().eq_ignore_ascii_case(&entry))
    }
}

#[test]
fn populate_context_limits_from_config_ref_seeds_global_cache() {
    use super::{NamedProviderConfig, NamedProviderModelConfig};

    // Regression test for issue #366: a named OpenAI-compatible provider with a
    // per-model `context_window` must be honored by the global context-limit
    // resolution path, not just the provider instance's own context_window().
    let model_id = "issue366-custom-gateway-model";
    let mut cfg = Config::default();
    cfg.providers.insert(
        "issue366-gateway".to_string(),
        NamedProviderConfig {
            base_url: "https://gateway.example.test/v1".to_string(),
            models: vec![NamedProviderModelConfig {
                id: model_id.to_string(),
                reasoning: None,
                reasoning_effort: None,
                context_window: Some(1_000_000),
                input: Vec::new(),
            }],
            ..Default::default()
        },
    );

    populate_context_limits_from_config_ref(&cfg);

    assert_eq!(
        crate::provider::context_limit_for_model(model_id),
        Some(1_000_000),
        "global context-limit resolution should respect named provider context_window"
    );
}

#[test]
fn populate_context_limits_from_config_seeds_qualified_runtime_model_shapes() {
    use super::{NamedProviderConfig, NamedProviderModelConfig};

    // Regression test for issue #421: the runtime request model can be
    // provider-qualified (`cachyai-a2000:qwen...`) or a slash path served by
    // llama.cpp (`ornith-box-1:/opt/models/ornith-1.0-35b-Q4_K_M.gguf`). The
    // configured context_window must resolve for every shape, not just the
    // bare id, otherwise budgeting falls back to the 200K default and
    // over-sends context.
    let mut cfg = Config::default();
    cfg.providers.insert(
        "issue421-gateway".to_string(),
        NamedProviderConfig {
            base_url: "http://10.15.15.53:8080/v1".to_string(),
            models: vec![
                NamedProviderModelConfig {
                    id: "issue421-qwen-128k".to_string(),
                    reasoning: None,
                    reasoning_effort: None,
                    context_window: Some(131_072),
                    input: Vec::new(),
                },
                NamedProviderModelConfig {
                    id: "/opt/models/issue421-ornith-35b-q4.gguf".to_string(),
                    reasoning: None,
                    reasoning_effort: None,
                    context_window: Some(131_072),
                    input: Vec::new(),
                },
            ],
            ..Default::default()
        },
    );

    populate_context_limits_from_config_ref(&cfg);

    // Bare id.
    assert_eq!(
        crate::provider::context_limit_for_model("issue421-qwen-128k"),
        Some(131_072)
    );
    // Profile-qualified spec, as persisted by session restore.
    assert_eq!(
        crate::provider::context_limit_for_model("issue421-gateway:issue421-qwen-128k"),
        Some(131_072),
        "profile-qualified model spec must resolve the configured context_window"
    );
    // Slash-path model id: the lookup reduces to the slash base.
    assert_eq!(
        crate::provider::context_limit_for_model("/opt/models/issue421-ornith-35b-q4.gguf"),
        Some(131_072),
        "slash-path model id must resolve the configured context_window"
    );
    // Profile-qualified slash-path spec, exactly as reported in issue #421.
    assert_eq!(
        crate::provider::context_limit_for_model(
            "issue421-gateway:/opt/models/issue421-ornith-35b-q4.gguf"
        ),
        Some(131_072),
        "profile-qualified slash-path spec must resolve the configured context_window"
    );
}

#[test]
fn migrate_legacy_swarm_spawn_mode_flips_visible_to_inline_once() {
    let _guard = crate::storage::lock_test_env();
    let prev_home = std::env::var_os("JCODE_HOME");
    let dir = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir.path());

    let config_path = dir.path().join("config.toml");
    let original = "[display]\ncentered = true\n\n[agents]\nswarm_spawn_mode = \"visible\"\nswarm_max_concurrent_agents = 32\n";
    std::fs::write(&config_path, original).expect("write config");

    assert!(
        Config::migrate_legacy_swarm_spawn_mode_once(),
        "migration should rewrite a legacy visible spawn mode"
    );
    let migrated = std::fs::read_to_string(&config_path).expect("read config");
    assert!(
        migrated.contains("swarm_spawn_mode = \"inline\""),
        "spawn mode should be flipped to inline: {migrated}"
    );
    // The rest of the file is untouched.
    assert!(migrated.contains("centered = true"));
    assert!(migrated.contains("swarm_max_concurrent_agents = 32"));
    let parsed: Config = toml::from_str(&migrated).expect("migrated config parses");
    assert_eq!(parsed.agents.swarm_spawn_mode, SwarmSpawnMode::Inline);

    // Marker written: a later explicit "visible" survives future launches.
    std::fs::write(&config_path, "[agents]\nswarm_spawn_mode = \"visible\"\n")
        .expect("write config");
    assert!(
        !Config::migrate_legacy_swarm_spawn_mode_once(),
        "migration must run at most once"
    );
    let content = std::fs::read_to_string(&config_path).expect("read config");
    assert!(content.contains("swarm_spawn_mode = \"visible\""));

    restore_env_var("JCODE_HOME", prev_home);
}

#[test]
fn migrate_legacy_swarm_spawn_mode_noops_without_visible_value() {
    let _guard = crate::storage::lock_test_env();
    let prev_home = std::env::var_os("JCODE_HOME");
    let dir = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir.path());

    // No config file at all: no migration, but the marker is written.
    assert!(!Config::migrate_legacy_swarm_spawn_mode_once());
    assert!(
        dir.path()
            .join("migrations")
            .join("swarm-spawn-mode-inline")
            .exists(),
        "marker should be written even when there is nothing to migrate"
    );

    // Explicit non-visible values are never rewritten (marker already set,
    // but check the matcher too with a fresh home).
    let dir2 = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir2.path());
    let config_path = dir2.path().join("config.toml");
    std::fs::write(&config_path, "[agents]\nswarm_spawn_mode = \"headless\"\n")
        .expect("write config");
    assert!(!Config::migrate_legacy_swarm_spawn_mode_once());
    let content = std::fs::read_to_string(&config_path).expect("read config");
    assert!(content.contains("swarm_spawn_mode = \"headless\""));

    restore_env_var("JCODE_HOME", prev_home);
}

#[test]
fn migrate_idle_animation_off_flips_true_to_false_once() {
    let _guard = crate::storage::lock_test_env();
    let prev_home = std::env::var_os("JCODE_HOME");
    let dir = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir.path());

    let config_path = dir.path().join("config.toml");
    let original = "[display]\ncentered = true\nidle_animation = true\nanimation_fps = 60\n";
    std::fs::write(&config_path, original).expect("write config");

    assert!(
        Config::migrate_idle_animation_off_once(),
        "migration should rewrite an enabled idle animation"
    );
    let migrated = std::fs::read_to_string(&config_path).expect("read config");
    assert!(
        migrated.contains("idle_animation = false"),
        "idle animation should be flipped off: {migrated}"
    );
    // The rest of the file is untouched.
    assert!(migrated.contains("centered = true"));
    assert!(migrated.contains("animation_fps = 60"));
    let parsed: Config = toml::from_str(&migrated).expect("migrated config parses");
    assert!(!parsed.display.idle_animation);

    // Marker written: a later explicit re-enable survives future launches.
    std::fs::write(&config_path, "[display]\nidle_animation = true\n").expect("write config");
    assert!(
        !Config::migrate_idle_animation_off_once(),
        "migration must run at most once"
    );
    let content = std::fs::read_to_string(&config_path).expect("read config");
    assert!(content.contains("idle_animation = true"));

    restore_env_var("JCODE_HOME", prev_home);
}

#[test]
fn migrate_idle_animation_off_noops_without_enabled_value() {
    let _guard = crate::storage::lock_test_env();
    let prev_home = std::env::var_os("JCODE_HOME");
    let dir = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir.path());

    // No config file at all: no migration, but the marker is written.
    assert!(!Config::migrate_idle_animation_off_once());
    assert!(
        dir.path()
            .join("migrations")
            .join("idle-animation-off")
            .exists(),
        "marker should be written even when there is nothing to migrate"
    );

    // Already-false values are never rewritten (fresh home to bypass marker).
    let dir2 = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir2.path());
    let config_path = dir2.path().join("config.toml");
    let original = "[display]\nidle_animation = false\n";
    std::fs::write(&config_path, original).expect("write config");
    assert!(!Config::migrate_idle_animation_off_once());
    let content = std::fs::read_to_string(&config_path).expect("read config");
    assert_eq!(content, original);

    restore_env_var("JCODE_HOME", prev_home);
}

#[test]
fn machine_written_sponsors_optout_is_respected() {
    let raw = "[sponsors]\nenabled = false\nendpoint = \"https://api.jcode.sh/v1/discovery\"\n";
    let config: Config = toml::from_str(raw).expect("parse");
    assert!(!config.sponsors.enabled);
}

fn read_with_legacy_default_on_repair(raw: &str) -> Config {
    let mut config: Config = toml::from_str(raw).expect("parse with legacy reader");
    let doc = raw.parse::<toml::Value>().expect("parse raw config");
    let machine_written = doc
        .get("sponsors")
        .and_then(toml::Value::as_table)
        .is_some_and(|table| {
            table.len() == 2
                && table.get("enabled").and_then(toml::Value::as_bool) == Some(false)
                && table
                    .get("endpoint")
                    .and_then(toml::Value::as_str)
                    .is_some_and(super::is_default_discovery_endpoint)
        });
    if machine_written {
        config.sponsors.enabled = true;
    }
    config
}

/// A persisted opt-out stays disabled through loading and saving.
#[test]
fn sponsors_optout_survives_a_real_config_round_trip() {
    let _guard = crate::storage::lock_test_env();
    let prev_home = std::env::var_os("JCODE_HOME");
    let dir = tempfile::TempDir::new().expect("tempdir");
    crate::env::set_var("JCODE_HOME", dir.path());
    Config::invalidate_cache();

    let path = Config::path().expect("config path");
    std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config parent");
    std::fs::write(
        &path,
        "[display]\ncentered = false\n\n[sponsors]\nenabled = false\nendpoint = \"https://api.jcode.sh/v1/discovery\"\n",
    )
    .expect("write frozen config");

    let loaded = Config::load();
    assert!(
        !loaded.sponsors.enabled,
        "loading must never undo an opt-out"
    );

    loaded.save().expect("save config");
    let rewritten = std::fs::read_to_string(&path).expect("read config");
    assert!(
        rewritten.contains("[sponsors]") && rewritten.contains("enabled = false"),
        "saving must durably preserve the explicit opt-out: {rewritten}"
    );
    assert!(
        !rewritten.contains("https://api.jcode.sh/v1/discovery"),
        "the default endpoint must be omitted so legacy repair cannot mistake the opt-out for a generated default: {rewritten}"
    );
    assert!(
        !read_with_legacy_default_on_repair(&rewritten)
            .sponsors
            .enabled,
        "the saved opt-out must survive the old default-on repair behavior"
    );
    assert!(
        !Config::load().sponsors.enabled,
        "discovery must stay disabled after a save/load round trip"
    );

    if let Some(prev) = prev_home {
        crate::env::set_var("JCODE_HOME", prev);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
    Config::invalidate_cache();
}

#[test]
fn legacy_endpoint_optout_is_respected() {
    let raw =
        "[sponsors]\nenabled = false\nendpoint = \"https://api.solosystems.dev/v1/discovery\"\n";
    let config: Config = toml::from_str(raw).expect("parse");
    assert!(!config.sponsors.enabled);
}

#[test]
fn hand_written_sponsors_optout_is_respected() {
    for raw in [
        "[sponsors]\nenabled = false\n",
        "[sponsors]\nenabled = false\nendpoint = \"https://discovery.internal/v1\"\n",
    ] {
        let config: Config = toml::from_str(raw).expect("parse");
        assert!(
            !config.sponsors.enabled,
            "explicit user opt-out must survive: {raw}"
        );
    }
}

#[test]
fn sponsors_settings_serialize_without_changing_operator_intent() {
    for endpoint in [
        "https://api.jcode.sh/v1/discovery",
        "https://api.jcode.sh/v1/discovery/",
        "https://api.solosystems.dev/v1/discovery",
        "https://api.solosystems.dev/v1/discovery/",
    ] {
        let mut config = Config::default();
        config.sponsors.endpoint = endpoint.to_string();
        let rendered = toml::to_string_pretty(&config).expect("serialize known default opt-out");
        assert!(
            !read_with_legacy_default_on_repair(&rendered)
                .sponsors
                .enabled,
            "known default endpoint opt-out must survive legacy repair: {rendered}"
        );
    }

    for (enabled, endpoint) in [
        (false, "https://discovery.internal/v1"),
        (true, "https://api.jcode.sh/v1/discovery"),
        (true, "https://discovery.internal/v1"),
    ] {
        let mut config = Config::default();
        config.sponsors.enabled = enabled;
        config.sponsors.endpoint = endpoint.to_string();
        let rendered = toml::to_string_pretty(&config).expect("serialize");
        let legacy = read_with_legacy_default_on_repair(&rendered);
        assert_eq!(legacy.sponsors.enabled, enabled, "{rendered}");
        assert_eq!(legacy.sponsors.endpoint, endpoint, "{rendered}");
    }
}

#[test]
fn enabled_legacy_sponsors_endpoint_round_trips_unchanged() {
    for endpoint in [
        "https://api.solosystems.dev/v1/discovery",
        "https://api.solosystems.dev/v1/discovery/",
    ] {
        let mut config = Config::default();
        config.sponsors.enabled = true;
        config.sponsors.endpoint = endpoint.to_string();
        let rendered = toml::to_string_pretty(&config).expect("serialize legacy opt-in");
        let reparsed: Config = toml::from_str(&rendered).expect("reparse legacy opt-in");
        assert!(reparsed.sponsors.enabled, "{rendered}");
        assert_eq!(reparsed.sponsors.endpoint, endpoint, "{rendered}");
    }
}

#[test]
fn config_reload_generation_increments_on_cache_invalidation() {
    let before = crate::config::config_reload_generation();
    crate::config::invalidate_config_cache();
    let after = crate::config::config_reload_generation();
    assert!(
        after > before,
        "invalidate_config_cache must bump the reload generation ({before} -> {after})"
    );
}
