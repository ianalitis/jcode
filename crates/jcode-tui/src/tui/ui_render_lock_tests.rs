//! Process-global render-state lock and reset used only by tests.

use super::*;

/// The one lock guarding process-global render state in tests.
///
/// Render snapshots, scroll metrics, flicker history, and prompt positions all
/// live in process globals, so *every* test that renders must serialize on the
/// same mutex. Two separate helpers previously each defined their own private
/// lock, which serialized nothing between them and produced failures that
/// appeared only under parallelism (same root cause as issue #593). Both now
/// delegate here.
pub(crate) fn render_state_test_lock() -> RenderStateTestGuard {
    // Reentrant: a thread that already holds the lock gets a depth-only guard
    // instead of blocking on a mutex it owns. Tests nest these through helper
    // wrappers (`scroll_render_test_lock`, `viewport_snapshot_test_lock`), and
    // a plain `Mutex` would self-deadlock on the inner take.
    if render_state_lock_depth() > 0 {
        RENDER_STATE_LOCK_DEPTH.with(|depth| depth.set(depth.get() + 1));
        return RenderStateTestGuard { _guard: None };
    }

    let guard = render_state_lock_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    RENDER_STATE_LOCK_DEPTH.with(|depth| depth.set(1));
    RenderStateTestGuard {
        _guard: Some(guard),
    }
}

fn render_state_lock_mutex() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

/// Guard for [`render_state_test_lock`] that also records ownership on this
/// thread, so a nested `clear_test_render_state_for_tests` can tell it is
/// already inside the lock instead of deadlocking on it.
///
/// `_guard` is `None` for a reentrant (nested) acquisition: the outermost
/// guard owns the mutex, and inner ones only maintain the depth count. A bool
/// would be wrong here, because dropping an inner guard would clear it while
/// the outer guard still held the mutex, and the next
/// `clear_test_render_state_for_tests` on this thread would then block on a
/// lock it already owns.
pub(crate) struct RenderStateTestGuard {
    _guard: Option<std::sync::MutexGuard<'static, ()>>,
}

impl Drop for RenderStateTestGuard {
    fn drop(&mut self) {
        RENDER_STATE_LOCK_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Take the render-state lock unless this thread already holds it.
///
/// `clear_test_render_state_for_tests` mutates the same globals the lock
/// protects, but it is called from both locked contexts (rendering tests) and
/// unlocked ones (`create_test_app`, used by ~570 tests). Acquiring
/// unconditionally would deadlock the former; not acquiring at all lets the
/// latter wipe state from under the former, which is the race behind
/// jcode-tui's intermittent layout failures.
///
/// Tracking ownership per thread lets one function serve both: the outermost
/// holder owns the guard, and nested calls become no-ops.
fn with_render_state_lock<T>(body: impl FnOnce() -> T) -> T {
    if render_state_lock_held() {
        return body();
    }

    // `try_lock`, not `lock`. This is the one acquisition that must never
    // wait, because it is reached from `create_test_app` (~570 tests), which
    // is routinely called by tests already holding the *env* lock via
    // `with_temp_jcode_home`. A render test scoping `JCODE_HOME` legitimately
    // waits for env while holding render; if this path also waited for render
    // while holding env, the two close an ABBA cycle. That deadlocked the
    // whole suite at default parallelism.
    //
    // Declining is safe: this is an incidental reset of render globals before
    // a test builds its app, not a render assertion. Failing to serialize it
    // is the behaviour that existed before the lock was introduced, and it
    // costs at most the flakiness the lock was added to reduce. A hung suite
    // costs everything.
    let Ok(guard) = render_state_lock_mutex().try_lock() else {
        return body();
    };
    RENDER_STATE_LOCK_DEPTH.with(|depth| depth.set(1));
    let _guard = RenderStateTestGuard {
        _guard: Some(guard),
    };
    body()
}

thread_local! {
    /// How many render-state guards this thread currently holds. A count
    /// rather than a flag so nested acquisitions unwind correctly: only the
    /// outermost drop releases the mutex's logical ownership.
    static RENDER_STATE_LOCK_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn render_state_lock_depth() -> usize {
    RENDER_STATE_LOCK_DEPTH.with(|depth| depth.get())
}

fn render_state_lock_held() -> bool {
    render_state_lock_depth() > 0
}

pub(crate) fn clear_test_render_state_for_tests() {
    with_render_state_lock(clear_test_render_state_locked)
}

/// The actual reset, run with the render-state lock held.
fn clear_test_render_state_locked() {
    set_last_max_scroll(0);
    set_pinned_pane_total_lines(0);
    set_last_diff_pane_effective_scroll(0);
    set_last_diff_pane_max_scroll(0);
    set_last_total_wrapped_lines(0);
    set_last_resolved_chat_scroll(0);
    TEST_TAIL_FOLLOW_SNAP_PENDING.with(|cell| cell.set(false));
    update_user_prompt_positions(&[]);
    // Flicker events recorded by sibling tests add a "⚠ flicker detected"
    // notification line to subsequent renders, shifting every layout-sensitive
    // assertion (click mapping, snapshot rows).
    frame_metrics::clear_flicker_frame_history_for_tests();
    TEST_LAST_LAYOUT.with(|snapshot| {
        *snapshot.borrow_mut() = None;
    });
    TEST_LAST_STATUS_AREA.with(|snapshot| {
        *snapshot.borrow_mut() = None;
    });
    set_visible_copy_targets(Vec::new());
    clear_copy_viewport_snapshot();

    TEST_PROMPT_VIEWPORT_STATE.with(|state| {
        *state.borrow_mut() = PromptViewportState::default();
    });
}

mod lock_order_tests {
    use super::{clear_test_render_state_for_tests, render_state_test_lock};

    /// Resetting render state never waits for the render lock.
    ///
    /// This is the property that broke the deadlock. `create_test_app` calls
    /// `clear_test_render_state_for_tests` from ~570 tests, many of which
    /// already hold the env lock; a render test scoping `JCODE_HOME` waits for
    /// env while holding render. If this path also waited, the two would close
    /// an ABBA cycle and hang the suite.
    #[test]
    fn clearing_render_state_never_waits_for_the_lock() {
        let holder_started = std::sync::Arc::new(std::sync::Barrier::new(2));
        let holder_may_exit = std::sync::Arc::new(std::sync::Barrier::new(2));
        let started = holder_started.clone();
        let may_exit = holder_may_exit.clone();

        let holder = std::thread::spawn(move || {
            let _render = render_state_test_lock();
            started.wait();
            may_exit.wait();
        });

        holder_started.wait();
        let start = std::time::Instant::now();
        clear_test_render_state_for_tests();
        let elapsed = start.elapsed();
        holder_may_exit.wait();
        holder.join().expect("render lock holder thread");

        assert!(
            elapsed < std::time::Duration::from_millis(200),
            "clearing render state blocked for {elapsed:?} while another \
             thread held the lock; it must decline instead of waiting"
        );
    }

    /// Nesting the render lock is a no-op rather than a self-deadlock, which
    /// is what lets `clear_test_render_state_for_tests` be called from both
    /// locked and unlocked callers.
    #[test]
    fn render_lock_nests_without_deadlocking() {
        let _outer = render_state_test_lock();
        let _inner = render_state_test_lock();
        clear_test_render_state_for_tests();
    }
}
