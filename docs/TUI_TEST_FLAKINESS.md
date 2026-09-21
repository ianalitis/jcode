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

## The env lock alone can still hang (observed 2026-09-21)

A second hang was observed after the v0.86.0 merge, with the same signature and a
narrower cause: **eleven** test threads (one per core) sat in
`__psynch_mutexwait` inside `jcode_base::storage::lock_test_env`, at near-zero
CPU, for fifty minutes. Unlike the 2026-09-01 ABBA deadlock, nothing was waiting
on the render-state lock, so the env lock itself was the cycle.

`std::sync::Mutex` is not reentrant, so the two shapes that produce this are
(a) a test that calls `lock_test_env()` while its own thread already holds the
lock, and (b) a guard that never drops because the thread went away. Both look
identical from outside: `sample` on the wedged binary shows every resident test
thread blocked in `lock_test_env`, and a self-deadlocked owner is
indistinguishable from a waiter. The static sweep for a nested call site
(`lock_test_env()` reached from a function that already acquires it, directly or
through a helper) found no obvious offender, and the exact site is still open.
Candidate helpers that acquire the lock and are reachable from locked tests
include `with_temp_jcode_home` and `with_reasoning_current_home` in
`tui/app/tests/support_failover/part_01.rs`, called from 186 sites.

### Mitigation in place

The wait is bounded and self-diagnosing instead of infinite:

- `lock_test_env()` takes the lock with `try_lock` first and otherwise polls.
- It warns once after 10s and fails after 300s, naming the test that is stuck and
  the **last successful acquirer**, which is the holder while the lock is wedged
  and therefore the usual culprit.
- `JCODE_TEST_ENV_LOCK_TIMEOUT_SECS` shortens the bound for diagnosis.

So a recurrence is now a named test failure with a culprit instead of a session
that hangs until it is killed. Tests:
`storage::tests::shared_test_env_lock_timeout_defaults_and_parses_override` and
`storage::tests::a_contended_env_lock_is_acquired_once_the_holder_releases`.

### Rules this leaves

- Take the lock once per test.
- A helper that may run under a locked test must `try_lock` and document why, as
  `ensure_test_jcode_home_if_unset` does.
- Never hold the lock while waiting on a thread that needs it.

## Parallel failure set measured 2026-09-21 (after the v0.86.0 merge)

Six consecutive `cargo test -p jcode-tui --lib` runs at the default thread count
on an 11-core host: 2353, 2351, 2353, 2353, 2348 and 2352 passed, with 2, 4, 2,
2, 7 and 3 failures. A `JCODE_TEST_ENV_LOCK_TIMEOUT_SECS=60` bound was set for
all six runs and no env-lock wait ever timed out, so the wedge did not recur.
Every failing name passed when run by itself.

The six-run failure set, by frequency:

| Failures | Test |
| ---: | --- |
| 5 | `tui::ui::tests::basic::test_changelog_overlay_repeated_renders_are_stable` |
| 2 | `tui::ui::messages::tests::render_system_message_uses_scheduled_task_card` |
| 2 | `tui::app::tests::test_model_picker_reuses_cached_entries_until_invalidated` |
| 2 | `tui::app::tests::test_local_model_picker_render_shows_antigravity_models_exactly_as_user_sees_them` |
| 1 each | `test_login_completed_surfaces_new_provider_models_in_local_model_picker`, `test_login_smoke_model_picker_renders_unstacked_provider_rows`, `test_local_model_picker_surfaces_antigravity_models_from_multiprovider`, `test_model_picker_waits_for_async_post_login_catalog_activation`, `test_open_model_picker_without_routes_shows_actionable_guidance`, `test_local_model_picker_openrouter_bare_openai_route_uses_openai_catalog_prefix`, `test_agents_picker_uses_provider_default_when_inherited_model_is_unknown`, `test_agent_model_picker_inherit_row_uses_provider_default_when_inherited_model_is_unknown`, `fast_and_slow_counters_persist_independently` |

Two families, both matching the analysis above:

- **Render state** (`test_changelog_overlay_*`, `render_system_message_*`): the
  `create_test_app` reset racing a concurrently-running render assertion.
- **Provider catalog and model picker** (`state_model_poke_02*`, `state_model_poke_03*`,
  `shortcut_hints`): these tests only call `ensure_test_jcode_home_if_unset()`,
  which `try_lock`s and proceeds without exclusion, then assert on provider
  catalog and picker-cache state that a sibling test is mutating. The
  degradation documented under "The deadlock" is exactly what this costs.

A targeted fix worth measuring next: have that second family take
`lock_test_env()` for the duration of the test (only that family, not the whole
suite, which is what blew the runtime up before) and reset the catalog cache the
way its siblings do. `jcode-app-core --lib` shows the same shape in
`tool::tests::test_context_guard_refusal_names_the_spilled_output`, which reads a
spill file whose name comes from shared state and passes in isolation.

## Fixed 2026-09-21: both families closed (Phases 1 and 2)

- **Picker/catalog family** (`3e410d7d0`): every test in `state_model_poke_03*`
  takes `lock_test_env()` for its duration. 12 parallel runs: 0 failures from
  the family, no lock timeout, wall time unchanged (24 to 27s).
- **Render-state family** (`62008c4ee`): `ui_frame_metrics` flicker history,
  slow-frame history and perf stats are per-thread under `cfg(test)` (a leaked
  `Mutex` per test thread keeps the `&'static Mutex<T>` accessors). The
  changelog test went 5/6 to 0/6; the scheduled-task card test now accepts
  either glyph variant because `TERM`/`TERM_PROGRAM` are set by siblings.
- The env-lock guard (`450702133`) now clears its holder record on drop, so a
  thread that released the lock cannot be reported as re-entrant when it later
  waits behind another holder.

Six-run tail after both fixes: 0 to 2 failures per run, each at most 2/6, all
timing or benchmark assertions
(`benchmark_resume_loading_reports_timings`, `smoothness_benchmark_*`,
`animation_cadence_*`, `test_alt_shift_i_*`, `issue_1206_*drop*`,
`test_restore_session_adds_reload_message`). Single-threaded: 2355/2355. The
next packet, if wanted, is to bound those benchmarks by CPU time rather than
wall time; they are not shared-state races.
