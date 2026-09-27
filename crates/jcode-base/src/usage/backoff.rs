//! Retry schedule helpers for the Anthropic OAuth usage gate.
//!
//! `disk_cache` owns the persisted state, file lock, and generation checks. Keeping
//! the schedule here makes the exponential and Retry-After policy explicit while
//! ensuring there is exactly one authoritative cross-process gate.

use std::time::Duration;

const BASE_DELAY: Duration = Duration::from_secs(60);
const MAX_DELAY: Duration = Duration::from_secs(15 * 60);

/// Delay after the `failures`-th consecutive failure. `jitter` in [0, 1)
/// spreads retries by -20%..+20%. The server's Retry-After wins when longer,
/// and the result never exceeds the 15-minute cap.
pub(super) fn backoff_delay(failures: u32, jitter: f64, retry_after: Option<Duration>) -> Duration {
    let exp = failures.saturating_sub(1).min(16);
    let base = BASE_DELAY.saturating_mul(1u32 << exp).min(MAX_DELAY);
    let jittered = base.mul_f64(0.8 + 0.4 * jitter.clamp(0.0, 1.0));
    jittered.max(retry_after.unwrap_or_default()).min(MAX_DELAY)
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
}
