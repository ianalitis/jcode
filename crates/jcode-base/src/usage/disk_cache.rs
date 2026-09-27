//! Cross-process cache and single-flight gate for Anthropic OAuth usage.
//!
//! A short-lived CLI process may not hold a file lock while awaiting HTTP. Instead,
//! it briefly locks this state file to claim one bounded in-flight request. The
//! claim generation makes a pre-reset or timed-out request unable to publish a
//! later success or alter the newer retry gate.

use super::display::usage_reset_passed;
use super::{CACHE_DURATION, ModelScopedUsageWindow, UsageData};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Last-good limits can still describe the account during a bounded backoff.
const LAST_GOOD_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
/// A crashed CLI process cannot suppress another process indefinitely.
const IN_FLIGHT_MAX_AGE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct Snapshot {
    fetched_unix_ms: i64,
    five_hour: f32,
    five_hour_resets_at: Option<String>,
    seven_day: f32,
    seven_day_resets_at: Option<String>,
    seven_day_opus: Option<f32>,
    #[serde(default)]
    model_scoped: Vec<(String, f32, Option<String>)>,
    extra_usage_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct Entry {
    last_good: Snapshot,
    /// Lets reset invalidation find anonymous token entries without persisting
    /// token-derived key material.
    #[serde(default)]
    token_key: bool,
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    failures: u32,
    #[serde(default)]
    until_ms: i64,
    #[serde(default)]
    in_flight_until_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FetchClaim(u64);

pub(super) enum FetchDecision {
    Fresh(UsageData),
    BackingOff {
        left: Duration,
        last_good: Option<UsageData>,
    },
    InFlight {
        wait: Duration,
        last_good: Option<UsageData>,
    },
    Leader(FetchClaim),
}

fn path() -> Result<PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("anthropic_usage_cache.json"))
}

fn lock_path() -> Result<PathBuf> {
    Ok(path()?.with_file_name("anthropic_usage_cache.lock"))
}

fn persisted_key(cache_key: &str) -> String {
    let digest = Sha256::digest(cache_key.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn age(unix_ms: i64) -> Duration {
    Duration::from_millis(now_ms().saturating_sub(unix_ms).max(0) as u64)
}

fn instant_for(unix_ms: i64) -> Instant {
    let now = Instant::now();
    now.checked_sub(age(unix_ms)).unwrap_or(now)
}

fn load_unlocked() -> Result<HashMap<String, Entry>> {
    let path = path()?;
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(error) => Err(error.into()),
    }
}

/// Run a short state transition under an advisory, cross-platform file lock.
/// The file handle drops before this function returns, so it cannot cross an await.
fn with_locked_map<T>(update: impl FnOnce(&mut HashMap<String, Entry>) -> (T, bool)) -> Result<T> {
    let lock_path = lock_path()?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lock_path)?;
    lock.lock()?;

    let mut map = load_unlocked()?;
    let (result, changed) = update(&mut map);
    if changed {
        let path = path()?;
        crate::storage::write_json_secret(&path, &map)?;
    }
    Ok(result)
}

fn to_usage(snapshot: &Snapshot) -> UsageData {
    UsageData {
        five_hour: snapshot.five_hour,
        five_hour_resets_at: snapshot.five_hour_resets_at.clone(),
        seven_day: snapshot.seven_day,
        seven_day_resets_at: snapshot.seven_day_resets_at.clone(),
        seven_day_opus: snapshot.seven_day_opus,
        model_scoped: snapshot
            .model_scoped
            .iter()
            .map(
                |(model_name, utilization, resets_at)| ModelScopedUsageWindow {
                    model_name: model_name.clone(),
                    utilization: *utilization,
                    resets_at: resets_at.clone(),
                },
            )
            .collect(),
        extra_usage_enabled: snapshot.extra_usage_enabled,
        fetched_at: Some(instant_for(snapshot.fetched_unix_ms)),
        last_error: None,
    }
}

fn usable_last_good(entry: &Entry) -> Option<UsageData> {
    (entry.last_good.fetched_unix_ms > 0)
        .then(|| to_usage(&entry.last_good))
        .filter(|usage| {
            age(entry.last_good.fetched_unix_ms) < LAST_GOOD_MAX_AGE
                && !usage_reset_passed([
                    usage.five_hour_resets_at.as_deref(),
                    usage.seven_day_resets_at.as_deref(),
                ])
                && !usage
                    .model_scoped
                    .iter()
                    .any(|window| usage_reset_passed([window.resets_at.as_deref()]))
        })
}

fn fresh_last_good(entry: &Entry) -> Option<UsageData> {
    usable_last_good(entry)
        .filter(|usage| age(entry.last_good.fetched_unix_ms) < CACHE_DURATION && !usage.is_stale())
}

/// Atomically select the one process allowed to fetch, or return the state it
/// must respect. A lock failure is surfaced to callers rather than fetching
/// without the cross-process gate.
pub(super) fn begin_fetch(cache_key: &str) -> Result<FetchDecision> {
    with_locked_map(|map| {
        let entry = map
            .entry(persisted_key(cache_key))
            .or_insert_with(|| Entry {
                token_key: cache_key.starts_with("token:"),
                ..Default::default()
            });
        let now = now_ms();

        if let Some(fresh) = fresh_last_good(entry) {
            return (FetchDecision::Fresh(fresh), false);
        }
        let last_good = usable_last_good(entry);
        if entry.until_ms > now {
            return (
                FetchDecision::BackingOff {
                    left: Duration::from_millis((entry.until_ms - now) as u64),
                    last_good,
                },
                false,
            );
        }
        if entry.in_flight_until_ms > now {
            return (
                FetchDecision::InFlight {
                    wait: Duration::from_millis((entry.in_flight_until_ms - now) as u64),
                    last_good,
                },
                false,
            );
        }

        entry.generation = entry.generation.wrapping_add(1);
        entry.in_flight_until_ms = now + IN_FLIGHT_MAX_AGE.as_millis() as i64;
        (FetchDecision::Leader(FetchClaim(entry.generation)), true)
    })
}

/// Record a failure only if it belongs to the current fetch claim. This is the
/// sole persisted Retry-After/exponential gate.
pub(super) fn finish_failure(
    cache_key: &str,
    claim: FetchClaim,
    retry_after: Option<Duration>,
) -> Result<Option<(Duration, Option<UsageData>)>> {
    with_locked_map(|map| {
        let Some(entry) = map.get_mut(&persisted_key(cache_key)) else {
            return (None, false);
        };
        if entry.generation != claim.0 {
            return (None, false);
        }
        entry.in_flight_until_ms = 0;
        entry.failures = entry.failures.saturating_add(1);
        let delay =
            super::backoff::backoff_delay(entry.failures, rand::random::<f64>(), retry_after);
        entry.until_ms = now_ms() + delay.as_millis() as i64;
        (Some((delay, usable_last_good(entry))), true)
    })
}

/// Publish a successful response only for the current claim. A reset or newer
/// leader changes the generation, making an older response a no-op.
pub(super) fn finish_success(cache_key: &str, claim: FetchClaim, data: &UsageData) -> Result<bool> {
    let snapshot = Snapshot {
        fetched_unix_ms: now_ms(),
        five_hour: data.five_hour,
        five_hour_resets_at: data.five_hour_resets_at.clone(),
        seven_day: data.seven_day,
        seven_day_resets_at: data.seven_day_resets_at.clone(),
        seven_day_opus: data.seven_day_opus,
        model_scoped: data
            .model_scoped
            .iter()
            .map(|window| {
                (
                    window.model_name.clone(),
                    window.utilization,
                    window.resets_at.clone(),
                )
            })
            .collect(),
        extra_usage_enabled: data.extra_usage_enabled,
    };
    with_locked_map(|map| {
        let Some(entry) = map.get_mut(&persisted_key(cache_key)) else {
            return (false, false);
        };
        if entry.generation != claim.0 {
            return (false, false);
        }
        entry.last_good = snapshot;
        entry.failures = 0;
        entry.until_ms = 0;
        entry.in_flight_until_ms = 0;
        (true, true)
    })
}

/// Forget quota and retry state after a limit reset. The retained generation
/// tombstone makes a pre-reset in-flight request unable to reinstall either.
pub(super) fn invalidate_after_reset(account_label: Option<&str>) {
    if let Err(error) = with_locked_map(|map| {
        let mut invalidate = |key: String| {
            let entry = map.entry(key).or_default();
            entry.last_good = Snapshot::default();
            entry.failures = 0;
            entry.until_ms = 0;
            entry.in_flight_until_ms = 0;
            entry.generation = entry.generation.wrapping_add(1);
        };
        match account_label
            .map(str::trim)
            .filter(|label| !label.is_empty())
        {
            Some(label) => invalidate(persisted_key(&format!("label:{label}"))),
            None => {
                invalidate(persisted_key("label:default"));
                for entry in map.values_mut().filter(|entry| entry.token_key) {
                    entry.last_good = Snapshot::default();
                    entry.failures = 0;
                    entry.until_ms = 0;
                    entry.in_flight_until_ms = 0;
                    entry.generation = entry.generation.wrapping_add(1);
                }
            }
        }
        ((), true)
    }) {
        crate::logging::error(&format!(
            "Failed to invalidate Anthropic usage state: {error:#}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_home<T>(test: impl FnOnce() -> T) -> T {
        let _guard = crate::storage::lock_test_env();
        let dir = tempfile::tempdir().expect("temp home");
        let previous = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_HOME", dir.path());
        let result = test();
        match previous {
            Some(value) => crate::env::set_var("JCODE_HOME", value),
            None => crate::env::remove_var("JCODE_HOME"),
        }
        result
    }

    fn sample() -> UsageData {
        UsageData {
            five_hour: 0.4,
            seven_day: 0.2,
            five_hour_resets_at: Some("2999-01-01T00:00:00Z".into()),
            ..Default::default()
        }
    }

    #[test]
    fn corrupt_state_never_restarts_the_fetch_gate() {
        with_home(|| {
            let state = path().expect("state path");
            std::fs::write(&state, b"not json").expect("corrupt fixture");
            assert!(begin_fetch("label:corrupt").is_err());
            assert_eq!(
                std::fs::read(&state).expect("retained fixture"),
                b"not json"
            );
        });
    }

    #[test]
    fn only_one_process_claims_a_cold_usage_fetch() {
        with_home(|| {
            let key = "token:very-secret-prefix";
            let first = begin_fetch(key).expect("first claim");
            assert!(matches!(first, FetchDecision::Leader(_)));
            assert!(matches!(
                begin_fetch(key).expect("second claim"),
                FetchDecision::InFlight { .. }
            ));

            let raw = std::fs::read_to_string(path().unwrap()).expect("state written");
            assert!(
                !raw.contains("very-secret-prefix"),
                "raw token key leaked: {raw}"
            );
        });
    }

    #[test]
    fn reset_rejects_a_pre_reset_success_and_failure() {
        with_home(|| {
            let key = "label:reset";
            let FetchDecision::Leader(claim) = begin_fetch(key).expect("claim") else {
                panic!("expected leader");
            };
            invalidate_after_reset(Some("reset"));

            assert!(!finish_success(key, claim, &sample()).expect("finish success"));
            assert!(
                finish_failure(key, claim, None)
                    .expect("finish failure")
                    .is_none()
            );
            assert!(matches!(
                begin_fetch(key).expect("new claim"),
                FetchDecision::Leader(_)
            ));
        });
    }

    #[test]
    fn fresh_shared_quota_preserves_confirmed_zero() {
        with_home(|| {
            let key = "label:zero";
            let FetchDecision::Leader(claim) = begin_fetch(key).expect("claim") else {
                panic!("expected leader");
            };
            let zero = UsageData {
                five_hour: 0.0,
                seven_day: 0.0,
                five_hour_resets_at: Some("2999-01-01T00:00:00Z".into()),
                ..Default::default()
            };
            assert!(finish_success(key, claim, &zero).expect("store"));

            let FetchDecision::Fresh(shared) = begin_fetch(key).expect("shared") else {
                panic!("expected fresh quota");
            };
            assert_eq!(shared.five_hour, 0.0);
            assert_eq!(shared.seven_day, 0.0);
        });
    }

    #[test]
    fn last_good_for_a_failure_keeps_its_original_fetch_age() {
        with_home(|| {
            let key = "label:quota-test";
            let mut map = HashMap::new();
            map.insert(
                persisted_key(key),
                Entry {
                    last_good: Snapshot {
                        fetched_unix_ms: now_ms() - Duration::from_secs(10 * 60).as_millis() as i64,
                        five_hour: 0.4,
                        five_hour_resets_at: Some("2999-01-01T00:00:00Z".into()),
                        seven_day: 0.2,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            let path = path().unwrap();
            crate::storage::write_json_secret(&path, &map).expect("write fixture");

            let FetchDecision::Leader(claim) = begin_fetch(key).expect("claim") else {
                panic!("expected leader");
            };
            finish_failure(key, claim, Some(Duration::from_secs(60))).expect("failure");
            let FetchDecision::BackingOff {
                last_good: Some(usage),
                ..
            } = begin_fetch(key).expect("backoff")
            else {
                panic!("expected last-good quota during backoff");
            };
            assert_eq!(usage.five_hour, 0.4);
            assert!(usage.fetched_at.expect("fetch age").elapsed() >= Duration::from_secs(9 * 60));
        });
    }
}
