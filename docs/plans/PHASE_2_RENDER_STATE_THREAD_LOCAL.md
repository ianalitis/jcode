# Phase 2: flicker frame history becomes thread-local under test

> **Implemented:** `62008c4ee`, now an ancestor of integration. The render-family
> results are recorded in [TUI test flakiness](../TUI_TEST_FLAKINESS.md#fixed-2026-09-21-both-families-closed-phases-1-and-2).
> The original packet below is historical. Its whole-suite green goal is not a current completion claim.

Workstream W2 family (a). Sequencing: after Phase 1 lands. Same writer lease.

## 1. Goal and non-goals

Goal: `tui::ui::tests::basic::test_changelog_overlay_repeated_renders_are_stable`
and `tui::ui::messages::tests::render_system_message_uses_scheduled_task_card`
stop failing under parallel runs, and `jcode-tui --lib` reaches six consecutive
green parallel runs.

Non-goals: no change to production (non-test) render behavior; no change to
`create_test_app`; no removal of `render_state_test_lock`.

## 2. Evidence that narrows this phase (verified 2026-09-21, HEAD `3dac3022f`)

`docs/TUI_TEST_FLAKINESS.md` proposes "make render state thread-local". Most of
it already is: `crates/jcode-tui/src/tui/ui.rs` lines 243 to 251 declare
`TEST_LAST_LAYOUT`, `TEST_LAST_STATUS_AREA`, `TEST_PROMPT_VIEWPORT_STATE`,
`TEST_TAIL_FOLLOW_SNAP_PENDING`, `TEST_VISIBLE_COPY_TARGETS`, `TEST_COPY_VIEWPORT`,
`TEST_LAST_MAX_SCROLL` and siblings as `thread_local!`, with every setter
branching on `#[cfg(test)]`.

What `clear_test_render_state_locked` (`ui_render_lock_tests.rs:116`) still
touches that is process-global:

- `frame_metrics::clear_flicker_frame_history_for_tests()` ->
  `FLICKER_FRAME_HISTORY: OnceLock<Mutex<FlickerFrameHistory>>`
  (`ui_frame_metrics.rs:418`), read at five sites in the same file, and
  `flicker_detection_enabled()` is hard-wired `true` under `cfg(test)`
  (`ui_frame_metrics.rs:455`). This is the "⚠ flicker detected" line that
  shifts the changelog test's rows.
- `set_last_chat_scrollbar_visible(false)` inside that helper, plus
  `slow_frame_history` and `frame_perf_stats` in the sibling
  `clear_slow_frame_history_for_tests`.

So the fix is one static, not a redesign: give `FlickerFrameHistory` the same
`#[cfg(test)] thread_local!` treatment the `ui.rs` statics already have.

## 3. Acceptance test

Same six-run loop as Phase 1 (reuse the command, log prefix `phase2-run-`).
Numbers that must change: `test_changelog_overlay_repeated_renders_are_stable`
from 5 of 6 to 0 of 6; `render_system_message_uses_scheduled_task_card` from 2
of 6 to 0 of 6; total failures per run **0** for six runs. Wall time still
under 60s per run.

Regression selector for production: `cargo test -p jcode-tui --lib frame_metrics`
and `cargo test -p jcode-tui --lib flicker` must pass unchanged, and
`cargo build -p jcode --bin jcode` must produce no new warnings (the
`cfg(not(test))` branch keeps the `OnceLock<Mutex<..>>`).

## 4. Freeze

| Field | Value |
| --- | --- |
| Lane, model | `opencode-go:glm-5.3-flash`; escalate to `openai-oauth:gpt-5.6-terra` medium if the first attempt returns a diff wider than `ui_frame_metrics.rs` |
| Effort | medium |
| Writer | one, lease held |
| Reader (optional, parallel, read-only) | `opencode-go:mimo-v2.5`: list every reader of `flicker_frame_history()`, `slow_frame_history()`, `frame_perf_stats()` and `last_chat_scrollbar_visible` outside `ui_frame_metrics.rs`, and whether any runs on a non-render thread. Return the list, no edits |
| Permitted files | `crates/jcode-tui/src/tui/ui_frame_metrics.rs` only. If the scrollbar/perf statics also need the treatment, same file |
| Iteration cap | 2 |

## 5. Method

1. Add a `#[cfg(test)] thread_local! { static TEST_FLICKER_FRAME_HISTORY: RefCell<FlickerFrameHistory> }`
   and route `flicker_frame_history()`'s five call sites through a small accessor
   that uses the thread-local under `cfg(test)` and the `OnceLock<Mutex>` otherwise.
   Mirror the exact shape used by `set_last_max_scroll` in `ui.rs:370`.
2. Run the acceptance loop. If `render_system_message_uses_scheduled_task_card`
   still fails, apply the same treatment to `slow_frame_history` and
   `last_chat_scrollbar_visible` (iteration 2, cap).

## 6. Gate and rollback

`bash scripts/check_guardrails.sh` (full, since production code changes:
clippy `-D warnings` and the panic ratchet both apply), then the acceptance
loop, then `cargo test -p jcode-tui --lib -- --test-threads=1` once to confirm
the single-thread count is unchanged (77s baseline).

Rollback: `git checkout -- crates/jcode-tui/src/tui/ui_frame_metrics.rs`.

## 7. Budget and stop

One `jcode-tui` test build, ~14 runs, two iterations. Under 40 minutes wall.
Stop on: diff outside one file; any new clippy finding; single-thread count
changes; reader reports a non-render-thread reader of flicker history (then the
thread-local would hide real events in production; hand off to Sol/high).

## 8. Criticism

- Over-engineered? The prior doc's option 1 ("make all render state
  thread-local") would have been, but it is already done. This is the residue.
- Missing evidence: whether a background task reads flicker history in
  production. The reader packet answers it before the writer commits. If yes,
  the `cfg(test)` split still keeps production identical, so the risk is to
  test fidelity, not to users.
- Regression: none in production by construction (`cfg(not(test))` path
  untouched). In tests, a test that *expects* to observe a sibling's flicker
  events would now fail; grep `flicker` in `ui_tests` first (reader task).
- Abandon if: after both iterations the changelog test still fails, meaning the
  row shift comes from something other than the flicker notification. Then
  `docs/TUI_TEST_FLAKINESS.md`'s bisection must be redone, and that is a fresh
  packet.

## 9. Handoff

Commit `test(tui): flicker frame history is thread-local under cfg(test)` with
the six-run table. Captain updates `docs/TUI_TEST_FLAKINESS.md`: replace
"Suggested direction" with the verified state, and record the green six-run
result. Captain then closes the "Known flakiness" bullets in
`docs/FORK_POSTURE.md §6` and §8. After this phase, decision O4 (reload) is
ready for the operator.
