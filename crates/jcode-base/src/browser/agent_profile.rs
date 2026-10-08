//! The dedicated agent Firefox profile: never the user's personal profile.

#[cfg(not(unix))]
use anyhow::Context;
use anyhow::Result;
use std::path::PathBuf;

#[cfg(not(unix))]
use super::xpi_path;
use super::{EXTENSION_ID_LISTED, browser_dir};

#[cfg(unix)]
#[path = "profile_io.rs"]
mod profile_io;

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
/// Always rooted under Jcode's browser state. `JCODE_BROWSER_PROFILE` is not
/// supported: accepting an arbitrary profile risks acting on personal tabs,
/// cookies and logins. Jcode never falls back to Firefox's default profile.
pub fn agent_profile_dir() -> PathBuf {
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
    #[cfg(unix)]
    {
        profile_io::prepare(&super::jcode_dir())?;
        Ok(agent_profile_dir())
    }
    #[cfg(not(unix))]
    {
        ensure_agent_profile_portable()
    }
}

#[cfg(not(unix))]
fn ensure_agent_profile_portable() -> Result<PathBuf> {
    let profile = agent_profile_dir();
    let extensions_dir = profile.join("extensions");
    // Reject existing links and Windows junctions before mutating any profile
    // payload. This portable check is not a directory-race guarantee. Windows
    // auto-launch remains disabled until an isolated direct launch is verified.
    for (path, directory) in [
        (browser_dir(), true),
        (profile.clone(), true),
        (extensions_dir.clone(), true),
        (profile.join("user.js"), false),
        (
            extensions_dir.join(format!("{}.xpi", EXTENSION_ID_LISTED)),
            false,
        ),
        (xpi_path(), false),
    ] {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                #[cfg(windows)]
                let reparse =
                    std::os::windows::fs::MetadataExt::file_attributes(&metadata) & 0x400 != 0;
                #[cfg(not(windows))]
                let reparse = false;
                if metadata.file_type().is_symlink()
                    || reparse
                    || (directory && !metadata.is_dir())
                    || (!directory && !metadata.is_file())
                {
                    anyhow::bail!("Unsafe path in dedicated agent profile");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot inspect dedicated agent profile"),
        }
    }
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

/// Construct commands without spawning a browser, so isolation is testable on
/// every host. The caller tries these candidates in order.
pub(super) fn firefox_launch_commands(
    os: &str,
    args: &[std::ffi::OsString],
) -> Vec<std::process::Command> {
    use std::process::Command;
    match os {
        "linux" => {
            let mut commands = Vec::new();
            for binary in ["firefox", "firefox-esr"] {
                let mut command = Command::new(binary);
                command.args(args);
                commands.push(command);
            }
            let mut flatpak = Command::new("flatpak");
            flatpak.args(["run", "org.mozilla.firefox"]).args(args);
            commands.push(flatpak);
            commands
        }
        "macos" => {
            // Invoke the app binary directly. `open -a` could hand the request
            // to a running personal-profile instance instead.
            let mut candidates = vec![
                PathBuf::from("/Applications/Firefox.app/Contents/MacOS/firefox"),
                PathBuf::from("/Applications/Firefox Developer Edition.app/Contents/MacOS/firefox"),
            ];
            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join("Applications/Firefox.app/Contents/MacOS/firefox"));
            }
            candidates
                .into_iter()
                .filter(|binary| binary.exists())
                .map(|binary| {
                    let mut command = Command::new(binary);
                    command.args(args);
                    command
                })
                .collect()
        }
        // Windows auto-launch stays disabled until a direct Firefox command
        // preserving --no-remote and the owned profile is verified. `cmd start`
        // without those arguments can attach to a personal Firefox instance.
        _ => Vec::new(),
    }
}

/// Launch the dedicated agent Firefox instance, detached, without any install
/// prompt. Returns whether a launch was started (not whether Firefox finished
/// starting).
///
/// The instance always uses the dedicated agent profile, so a personal Firefox
/// profile is never the automation target and a Firefox already running with
/// the user's own profile is left alone.
pub(super) fn launch_firefox_agent_profile_detached(url: Option<&str>) -> bool {
    let profile = match ensure_agent_profile() {
        Ok(profile) => profile,
        Err(e) => {
            crate::logging::warn(&format!(
                "Could not prepare the dedicated agent browser profile: {e}"
            ));
            return false;
        }
    };
    let mut args = firefox_launch_args(&profile, agent_profile_headless());
    if let Some(url) = url {
        // Replace the trailing about:blank start page with the requested URL.
        args.pop();
        args.push(url.into());
    }

    for mut cmd in firefox_launch_commands(std::env::consts::OS, &args) {
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Ok(child) = crate::platform::spawn_detached(&mut cmd) {
            crate::platform::reap_detached(child);
            return true;
        }
    }
    false
}
