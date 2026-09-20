# jcode-tui test flakiness: root cause

`cargo test -p jcode-tui --lib` fails a small, varying set of tests per run.
This is a parallelism race on process-global state, not a logic bug.

> **Update (2026-09-01):** the suite also *deadlocked* at default parallelism,
> which is a separate and much more serious defect than the flakiness below.
> That is fixed; see "The deadlock" at the end. The remaining flakiness is
> unchanged and the analysis below still applies to it.

## Evidence

- `cargo test -p jcode-tui --lib -- --test-threads=1` passes **2006/2006** (16 ignored).
- The failing set changes between runs at the default thread count.
- Individually, each failing test passes when run alone.

Counts were taken on 2026-07-27 and will drift as tests are added. Reproduce
on an otherwise idle machine: under memory pressure (this host has 15 GiB and
was running concurrent workspace builds) `cargo` gets SIGTERMed mid-compile,
which is a different failure from the race described here.

## Root cause

`create_test_app()` (and its `create_named_provider_test_app` sibling) in
`crates/jcode-tui/src/tui/app/tests/support_failover/part_01.rs` calls:

```rust
crate::tui::ui::clear_test_render_state_for_tests();
```

That wipes **process-global** render state: the flicker frame history, layout
snapshots, status-area snapshots, copy targets, and scroll positions.

Rendering tests guard exactly that state with `render_state_test_lock()`. But
`create_test_app` clears it *without* taking the lock, so any of its ~810 call
sites can reset a concurrently-running render test's state mid-assertion.

The mechanism for the most frequent victim
(`test_changelog_overlay_repeated_renders_are_stable`) is documented in
`clear_test_render_state_for_tests` itself: a recorded flicker event adds a
"⚠ flicker detected" notification line to later renders, shifting every
layout-sensitive assertion by a row.

### Bisected proof

Bisecting the 959 `tui::app::tests::` tests against the changelog test
identifies `test_tui_login_providers_have_real_tui_handlers`, which calls
`create_test_app()` in a loop (once per login provider). Running just those two
does not reproduce; the race needs enough concurrent load to interleave, which
is why it presents as order-dependent flakiness.

## What does not work

**Taking `render_state_test_lock` inside `create_test_app`.** This is correct
but serializes all ~810 call sites: suite runtime goes from ~12s to over 10
minutes. Measured, then reverted.

**Asserting a floor instead of an exact count** in the changelog test's
`buffered_samples` check, and **calling `clear_test_render_state_for_tests`**
at the top of that test. Both measured over 5 runs: the test still failed 5/5
with *and* without the change. Reverted rather than committed as churn.

## Suggested direction

The real fix is to stop sharing this state across tests rather than to
serialize access to it:

1. Make the render state thread-local rather than process-global, so parallel
   tests cannot observe each other's resets. Production has one render thread,
   so this should not change runtime behavior.
2. Failing that, have `create_test_app` skip the render-state clear entirely.
   Only rendering tests depend on it, and they already clear it under the lock.
   This needs an audit of which app tests implicitly rely on the current clear.

Option 1 is preferred: it removes the shared mutable state instead of adding
coordination around it.

## Scope note

This is pre-existing and independent of the render-path performance work in
commits `0ba0154c6`, `2b8e78e34`, `8b44fc83b`, `8142f1a0b`. Verified by
stashing those changes and reproducing the same failure rate.

## The deadlock (fixed 2026-09-01)

Separately from the flakiness above, the suite **hung indefinitely** at the
default thread count. It was not slow: 11 test threads sat in
`__psynch_mutexwait` with near-zero CPU, and the run never terminated. The
documented `--test-threads=1` workaround hid it, because a single thread
cannot form a cycle.

### Cause

Two process-global mutexes, acquired in both orders:

- `jcode_base::storage::test_env_lock` (env lock), guarding `JCODE_HOME`,
  config, and auth overrides.
- the render-state lock in `tui::ui`, guarding render globals.

Both orders existed in the suite:

```
Thread A: with_temp_jcode_home()          Thread B: a render test
  holds ENV                                 holds RENDER
  -> create_test_app()                      -> with_reasoning_current_home()
     -> clear_test_render_state_for_tests()    -> lock_test_env()
        -> waits for RENDER                       -> waits for ENV
```

A textbook ABBA deadlock. It needed enough concurrent load to interleave,
which is why it presented as "the suite is slow sometimes" rather than as an
obvious hang.

### Fix

Two parts, both small:

1. **Two tests took the locks in the wrong order.** Swapped to env-first,
   matching every other test that needs both
   (`smoothness_benchmark_simulated_streaming_turn_stays_within_budget` and
   `test_alt_shift_i_toggles_inline_images_and_persists`).
2. **`with_render_state_lock` no longer waits.** It is reached from
   `create_test_app`, used by ~570 tests, many already holding the env lock.
   It now `try_lock`s and proceeds without the lock if another thread holds
   it. Declining is safe there: it is an incidental reset before a test builds
   its app, not a render assertion, and not serializing it is exactly the
   behaviour that existed before the lock was added. A missed serialization
   costs some of the flakiness above. A hung suite costs everything.

The render-state guard is also now reentrant (a depth count, not a bool), so
nesting it is a no-op instead of a self-deadlock.

### Measured

| | before | after |
|---|---|---|
| default thread count | hung (>10 min, killed) | **21-30s** |
| `--test-threads=1` | 77s | 77s |
| failures at default | n/a (never finished) | 28-30 |
| failures single-threaded | 26 | 26 |

The 2-4 extra failures under parallelism are the pre-existing races described
above; each passes in isolation, and they include
`test_changelog_overlay_repeated_renders_are_stable`, the same test named as
the most frequent victim earlier in this document.

Regression coverage lives in `tui::ui::lock_order_tests`.
