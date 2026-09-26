//! Cross-process backoff for the Anthropic usage endpoint.
//!
//! The in-memory cache only protects one process. Short-lived `jcode` runs each
//! started with an empty cache and fetched immediately, so a 429 was retried
//! every few seconds. Failures are therefore persisted per account under
//! `~/.jcode/usage-backoff.json` with capped exponential backoff and jitter.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const BASE_DELAY: Duration = Duration::from_secs(60);
const MAX_DELAY: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Entry {
    failures: u32,
    until_ms: i64,
}

fn state_path() -> Option<PathBuf> {
    crate::storage::jcode_dir()
        .ok()
        .map(|dir| dir.join("usage-backoff.json"))
}

/// Hash the cache key so no token prefix is written to disk.
fn entry_key(cache_key: &str) -> String {
    let digest = Sha256::digest(cache_key.as_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn load() -> HashMap<String, Entry> {
    state_path()
        .and_then(|path| crate::storage::read_json(&path).ok())
        .unwrap_or_default()
}

fn save(state: &HashMap<String, Entry>) {
    if let Some(path) = state_path() {
        let _ = crate::storage::write_json_fast(&path, state);
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Delay after the `failures`-th consecutive failure. `jitter` in [0, 1)
/// spreads retries by -20%..+20%. The server's Retry-After wins when longer,
/// and the result never exceeds the 15-minute cap.
pub(super) fn backoff_delay(failures: u32, jitter: f64, retry_after: Option<Duration>) -> Duration {
    let exp = failures.saturating_sub(1).min(16);
    let base = BASE_DELAY.saturating_mul(1u32 << exp).min(MAX_DELAY);
    let jittered = base.mul_f64(0.8 + 0.4 * jitter.clamp(0.0, 1.0));
    jittered.max(retry_after.unwrap_or_default()).min(MAX_DELAY)
}

/// Remaining backoff for this account, if a recent failure is still cooling down.
pub(super) fn remaining(cache_key: &str) -> Option<Duration> {
    let entry = load().remove(&entry_key(cache_key))?;
    let left = entry.until_ms - now_ms();
    (left > 0).then(|| Duration::from_millis(left as u64))
}

pub(super) fn record_failure(cache_key: &str, retry_after: Option<Duration>) -> Duration {
    let mut state = load();
    let entry = state.entry(entry_key(cache_key)).or_default();
    entry.failures = entry.failures.saturating_add(1);
    let delay = backoff_delay(entry.failures, rand::random::<f64>(), retry_after);
    entry.until_ms = now_ms() + delay.as_millis() as i64;
    save(&state);
    delay
}

pub(super) fn record_success(cache_key: &str) {
    let mut state = load();
    if state.remove(&entry_key(cache_key)).is_some() {
        save(&state);
    }
}

pub(super) fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Points `JCODE_HOME` at a temp dir; the `TempDir` is held only to keep it alive.
    struct HomeGuard(
        Option<std::ffi::OsString>,
        #[allow(dead_code)] tempfile::TempDir,
    );

    impl HomeGuard {
        fn new() -> Self {
            let prev = std::env::var_os("JCODE_HOME");
            let home = tempfile::TempDir::new().expect("temp home");
            crate::env::set_var("JCODE_HOME", home.path());
            Self(prev, home)
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(prev) => crate::env::set_var("JCODE_HOME", prev),
                None => crate::env::remove_var("JCODE_HOME"),
            }
        }
    }

    #[test]
    fn delay_grows_exponentially_with_jitter_and_caps_at_fifteen_minutes() {
        assert_eq!(backoff_delay(1, 0.5, None), Duration::from_secs(60));
        assert_eq!(backoff_delay(2, 0.5, None), Duration::from_secs(120));
        assert_eq!(backoff_delay(1, 0.0, None), Duration::from_secs(48));
        assert_eq!(backoff_delay(1, 0.999_999, None).as_secs(), 71);
        assert_eq!(backoff_delay(50, 1.0, None), MAX_DELAY);
        assert_eq!(
            backoff_delay(1, 0.5, Some(Duration::from_secs(300))),
            Duration::from_secs(300)
        );
        assert_eq!(
            backoff_delay(1, 0.5, Some(Duration::from_secs(86_400))),
            MAX_DELAY
        );
    }

    #[test]
    fn failure_is_visible_to_a_fresh_process_and_cleared_by_success() {
        let _lock = crate::storage::lock_test_env();
        let _home = HomeGuard::new();
        assert!(remaining("acct").is_none());

        record_failure("acct", None);
        // A new process only has the file, so reading it back must see the backoff.
        let left = remaining("acct").expect("backoff persisted");
        assert!(
            left > Duration::from_secs(40) && left <= MAX_DELAY,
            "{left:?}"
        );
        assert!(remaining("other").is_none());

        let raw = std::fs::read_to_string(state_path().unwrap()).unwrap();
        assert!(!raw.contains("acct"), "account key must be hashed: {raw}");

        record_success("acct");
        assert!(remaining("acct").is_none());
    }

    #[tokio::test]
    async fn fetch_skips_the_network_while_another_process_is_backing_off() {
        let _lock = crate::storage::lock_test_env();
        let _home = HomeGuard::new();
        let key = "backoff-fetch-test".to_string();
        record_failure(&key, Some(Duration::from_secs(600)));

        let err = super::super::fetch_anthropic_usage_data("invalid-token".into(), key)
            .await
            .expect_err("backoff must short-circuit the fetch");
        assert!(err.to_string().contains("backing off"), "{err}");
    }
}
