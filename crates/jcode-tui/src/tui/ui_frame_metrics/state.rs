//! Frame-metrics storage. Production keeps one process-global instance per
//! metric; under `cfg(test)` every test thread gets its own so parallel render
//! tests cannot observe each other's flicker or slow-frame history.

use super::{FlickerFrameHistory, FramePerfStats, SlowFrameHistory};
use std::sync::Mutex;
#[cfg(not(test))]
use std::sync::OnceLock;

#[cfg(not(test))]
static FRAME_PERF_STATS: OnceLock<Mutex<FramePerfStats>> = OnceLock::new();
#[cfg(not(test))]
static SLOW_FRAME_HISTORY: OnceLock<Mutex<SlowFrameHistory>> = OnceLock::new();
#[cfg(not(test))]
static FLICKER_FRAME_HISTORY: OnceLock<Mutex<FlickerFrameHistory>> = OnceLock::new();

// Under test every test thread gets its own frame-metrics state. The suite
// runs tests in parallel against one process, and `create_test_app` resets
// this state from ~800 sites: a flicker event recorded by one test added a
// "flicker detected" notice to a concurrently running render test and shifted
// its rows (`test_changelog_overlay_repeated_renders_are_stable`). Production
// has one render thread, so the process-global statics below are unchanged
// there; the per-thread mutex is leaked once per test thread to keep the
// `&'static Mutex<T>` signature every caller already uses.
#[cfg(test)]
thread_local! {
    static TEST_FRAME_PERF_STATS: &'static Mutex<FramePerfStats> =
        Box::leak(Box::new(Mutex::new(FramePerfStats::default())));
    static TEST_SLOW_FRAME_HISTORY: &'static Mutex<SlowFrameHistory> =
        Box::leak(Box::new(Mutex::new(SlowFrameHistory::default())));
    static TEST_FLICKER_FRAME_HISTORY: &'static Mutex<FlickerFrameHistory> =
        Box::leak(Box::new(Mutex::new(FlickerFrameHistory::default())));
}

pub(super) fn frame_perf_stats() -> &'static Mutex<FramePerfStats> {
    #[cfg(test)]
    {
        TEST_FRAME_PERF_STATS.with(|m| *m)
    }
    #[cfg(not(test))]
    {
        FRAME_PERF_STATS.get_or_init(|| Mutex::new(FramePerfStats::default()))
    }
}

pub(super) fn slow_frame_history() -> &'static Mutex<SlowFrameHistory> {
    #[cfg(test)]
    {
        TEST_SLOW_FRAME_HISTORY.with(|m| *m)
    }
    #[cfg(not(test))]
    {
        SLOW_FRAME_HISTORY.get_or_init(|| Mutex::new(SlowFrameHistory::default()))
    }
}

pub(super) fn flicker_frame_history() -> &'static Mutex<FlickerFrameHistory> {
    #[cfg(test)]
    {
        TEST_FLICKER_FRAME_HISTORY.with(|m| *m)
    }
    #[cfg(not(test))]
    {
        FLICKER_FRAME_HISTORY.get_or_init(|| Mutex::new(FlickerFrameHistory::default()))
    }
}
