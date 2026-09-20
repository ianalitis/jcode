//! Durable continuation references for tool output that cannot stay in history.
//!
//! Session history caps one tool result at 512 KiB and the context guard can
//! refuse an oversized result outright, so the discarded bytes used to be
//! unrecoverable. That pushed callers into re-running a guessed-narrower
//! command and hoping the answer was in the surviving prefix. Instead, keep the
//! full text under `<JCODE_HOME>/tool-output/` and name the path in the notice,
//! so the caller can read exactly the section it needs.
//!
//! Retention is bounded: entries older than [`SPILL_RETENTION_DAYS`] are pruned
//! on each write, and the directory only exists once something was truncated.
//! These files are the same content class as session history, which already
//! persists tool results, and they are created owner-only.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::logging;
use crate::storage;

/// How long a spilled tool output may live before the next spill prunes it.
pub(super) const SPILL_RETENTION_DAYS: u64 = 7;

/// Newest spills kept regardless of age, so a busy session cannot accumulate
/// unbounded copies of large command output.
const SPILL_MAX_FILES: usize = 200;

const SPILL_PREFIX: &str = "tool-output-";
const MAX_NAME_TOKEN_CHARS: usize = 48;

/// Directory holding spilled outputs, created on demand.
fn spill_dir() -> Option<PathBuf> {
    let home = match storage::jcode_dir() {
        Ok(home) => home,
        Err(error) => {
            logging::warn(&format!(
                "Could not resolve the Jcode home directory for tool output: {error}"
            ));
            return None;
        }
    };
    let dir = home.join("tool-output");
    match storage::ensure_dir(&dir) {
        Ok(()) => Some(dir),
        Err(error) => {
            logging::warn(&format!(
                "Could not create the tool-output spill directory {}: {error}",
                dir.display()
            ));
            None
        }
    }
}

fn sanitize_token(raw: &str) -> String {
    let mut token: String = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .take(MAX_NAME_TOKEN_CHARS)
        .collect();
    if token.is_empty() {
        token.push_str("unknown");
    }
    token
}

fn prune_spills(dir: &Path) {
    let Some(cutoff) = SystemTime::now().checked_sub(Duration::from_secs(
        SPILL_RETENTION_DAYS.saturating_mul(24 * 60 * 60),
    )) else {
        return;
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            logging::warn(&format!(
                "Could not list {} to prune spilled tool output: {error}",
                dir.display()
            ));
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(SPILL_PREFIX) {
            continue;
        }
        let expired = match entry.metadata().and_then(|meta| meta.modified()) {
            Ok(modified) => modified < cutoff,
            Err(_) => false,
        };
        if !expired {
            continue;
        }
        if let Err(error) = std::fs::remove_file(entry.path()) {
            logging::warn(&format!(
                "Could not prune spilled tool output {}: {error}",
                entry.path().display()
            ));
        }
    }
    prune_spills_beyond_cap(dir);
}

/// Keep only the newest [`SPILL_MAX_FILES`] spills. Names start with the write
/// timestamp, so lexicographic order is chronological order.
fn prune_spills_beyond_cap(dir: &Path) {
    let mut spills: Vec<PathBuf> = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_spill = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(SPILL_PREFIX));
        if is_spill {
            spills.push(path);
        }
    }
    if spills.len() <= SPILL_MAX_FILES {
        return;
    }
    spills.sort();
    let excess = spills.len() - SPILL_MAX_FILES;
    for path in spills.into_iter().take(excess) {
        if let Err(error) = std::fs::remove_file(&path) {
            logging::warn(&format!(
                "Could not prune spilled tool output {}: {error}",
                path.display()
            ));
        }
    }
}

/// Write `full_text` for `tool_name` and return the path the caller can read.
///
/// Returns `None` when the file cannot be written, in which case the caller
/// keeps its existing advice instead of pointing at a path that does not exist.
pub(crate) fn spill_truncated_output(
    session_id: &str,
    tool_name: &str,
    full_text: &str,
) -> Option<PathBuf> {
    let dir = spill_dir()?;
    let stamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_millis(),
        Err(_) => 0,
    };
    let file_name = format!(
        "{SPILL_PREFIX}{stamp}-{}-{}.txt",
        sanitize_token(session_id),
        sanitize_token(tool_name)
    );
    let path = dir.join(file_name);
    match std::fs::write(&path, full_text) {
        Ok(()) => {}
        Err(error) => {
            logging::warn(&format!(
                "Could not spill truncated tool output to {}: {error}",
                path.display()
            ));
            return None;
        }
    }
    storage::harden_secret_file_permissions(&path);
    prune_spills(&dir);
    logging::info(&format!(
        "Spilled {} chars of `{}` output to {}",
        full_text.chars().count(),
        tool_name,
        path.display()
    ));
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HomeGuard {
        previous: Option<std::ffi::OsString>,
        dir: tempfile::TempDir,
    }

    impl HomeGuard {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().expect("temp dir");
            let previous = std::env::var_os("JCODE_HOME");
            crate::env::set_var("JCODE_HOME", dir.path());
            Self { previous, dir }
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => crate::env::set_var("JCODE_HOME", value),
                None => crate::env::remove_var("JCODE_HOME"),
            }
        }
    }

    #[test]
    fn spills_the_full_text_and_returns_a_readable_path() {
        let _lock = crate::storage::lock_test_env();
        let home = HomeGuard::new();
        let full = "z".repeat(600);

        let path = spill_truncated_output("session_abc", "bash", &full).expect("spill path");

        assert!(path.starts_with(home.dir.path()));
        assert_eq!(std::fs::read_to_string(&path).expect("read spill"), full);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        assert!(name.starts_with(SPILL_PREFIX), "{name}");
        assert!(name.contains("session_abc"), "{name}");
        assert!(name.contains("bash"), "{name}");
    }

    #[test]
    fn pruning_removes_only_expired_spills() {
        let _lock = crate::storage::lock_test_env();
        let _home = HomeGuard::new();
        let dir = spill_dir().expect("spill dir");
        let fresh = dir.join(format!("{SPILL_PREFIX}fresh.txt"));
        let unrelated = dir.join("keep-me.txt");
        std::fs::write(&fresh, "fresh").expect("write fresh");
        std::fs::write(&unrelated, "unrelated").expect("write unrelated");

        prune_spills(&dir);

        assert!(fresh.exists(), "a fresh spill must survive pruning");
        assert!(unrelated.exists(), "pruning must ignore unrelated files");
    }
}
