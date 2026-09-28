use std::time::{Duration, Instant};

/// Backdate `Instant::now()` by up to `amount`, saturating instead of
/// panicking when the clock's epoch is too recent.
///
/// `Instant` counts from boot on Windows (QPC) and Linux (CLOCK_MONOTONIC), so
/// `Instant::now() - one_hour` panics with "overflow when subtracting duration
/// from instant" when the machine booted more recently than that. This hit
/// real users right after a reboot: git cache invalidation runs
/// after every bash/edit tool, crashing the whole TUI (issue #424).
pub(crate) fn backdated_now(amount: Duration) -> Instant {
    let now = Instant::now();
    let mut backdate = amount;
    loop {
        if let Some(instant) = now.checked_sub(backdate) {
            return instant;
        }
        if backdate < Duration::from_millis(1) {
            return now;
        }
        backdate /= 2;
    }
}
