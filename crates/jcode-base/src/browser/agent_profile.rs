//! The dedicated agent Firefox profile: never the user's personal profile.

use anyhow::{Context, Result};
use std::path::PathBuf;

use super::{EXTENSION_ID_LISTED, browser_dir, xpi_path};

/// Prefs written into the dedicated agent profile.
///
/// The agent profile exists so browser automation never attaches to a personal
/// Firefox profile. Everything here suppresses first-run prompts and optional
/// reporting so a headless launch is quiet and self-contained.
const AGENT_PROFILE_USER_JS: &str = "\
user_pref(\"browser.shell.checkDefaultBrowser\", false);
user_pref(\"toolkit.telemetry.enabled\", false);
user_pref(\"datareporting.policy.dataSubmissionEnabled\", false);
user_pref(\"datareporting.healthreport.uploadEnabled\", false);
user_pref(\"browser.discovery.enabled\", false);
user_pref(\"extensions.autoDisableScopes\", 0);
user_pref(\"extensions.enabledScopes\", 15);
user_pref(\"browser.startup.homepage_override.mstone\", \"ignore\");
";

/// Directory of the dedicated agent Firefox profile.
///
/// Override with `JCODE_BROWSER_PROFILE`. Jcode never falls back to the user's
/// default Firefox profile: attaching automation to a personal profile risks
/// acting on personal tabs, cookies and logins.
pub fn agent_profile_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("JCODE_BROWSER_PROFILE") {
        return PathBuf::from(dir);
    }
    browser_dir().join("agent-profile")
}

/// Whether the agent Firefox instance should run headless.
///
/// Headless is the default so automation does not steal focus or open a visible
/// window. Set `JCODE_BROWSER_HEADLESS=0` to watch the agent profile.
pub fn agent_profile_headless() -> bool {
    !matches!(
        std::env::var("JCODE_BROWSER_HEADLESS").as_deref(),
        Ok("0") | Ok("false") | Ok("off") | Ok("no")
    )
}

/// Create (or refresh) the dedicated agent profile and make it self-contained.
///
/// The signed bridge XPI is copied into `extensions/` so the profile works
/// without any manual install step, and `user.js` carries the quiet-launch
/// prefs. Only this profile's own files are created or overwritten.
pub fn ensure_agent_profile() -> Result<PathBuf> {
    let profile = agent_profile_dir();
    let extensions_dir = profile.join("extensions");
    std::fs::create_dir_all(&extensions_dir)
        .context("Failed to create the dedicated agent browser profile")?;

    let source_xpi = xpi_path();
    if source_xpi.exists() {
        let target = extensions_dir.join(format!("{}.xpi", EXTENSION_ID_LISTED));
        let needs_copy = match (std::fs::read(&target), std::fs::read(&source_xpi)) {
            (Ok(existing), Ok(source)) => existing != source,
            _ => true,
        };
        if needs_copy {
            std::fs::copy(&source_xpi, &target).context("Failed to install the bridge XPI")?;
        }
    }

    let prefs_path = profile.join("user.js");
    let prefs_current = match std::fs::read_to_string(&prefs_path) {
        Ok(existing) => existing == AGENT_PROFILE_USER_JS,
        Err(_) => false,
    };
    if !prefs_current {
        std::fs::write(&prefs_path, AGENT_PROFILE_USER_JS)
            .context("Failed to write the agent profile prefs")?;
    }

    Ok(profile)
}

/// Arguments for launching the dedicated agent Firefox instance.
///
/// `--no-remote` keeps this instance from handing the request to a Firefox that
/// is already running with the user's own profile, and `-profile` pins the
/// dedicated profile explicitly so no default-profile fallback is possible.
pub(super) fn firefox_launch_args(
    profile: &std::path::Path,
    headless: bool,
) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = vec![
        "--no-remote".into(),
        "-profile".into(),
        profile.as_os_str().to_os_string(),
    ];
    if headless {
        args.push("-headless".into());
    }
    args.push("about:blank".into());
    args
}
