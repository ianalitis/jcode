# `Session::save()` drops debug, canary and improve-mode state on a blank session

Date: 2026-09-22. Observed on upstream `2a4edaa02057ac994a601311c4f03ed450e1b3c9`
(macOS). Status: reported upstream as
[#1339](https://github.com/1jehuang/jcode/issues/1339) by `MatrixMagician`, fix
prepared as [#1373](https://github.com/1jehuang/jcode/pull/1373) from
`pr/session-persist-explicit-state`.

## Expected and observed

`Session::save()` skips the write when a session has no visible message, no title
and no parent. That guard exempts caller-set explicit state — `custom_title`,
`title`, `parent_id` were added by #1144 — but not `is_debug`, `is_canary` or
`improve_mode`, which are explicit state by the same argument.

Setting one of those and saving therefore drops it silently. `save()` returns
`Ok(())`, the error branch in the caller never fires, and the load-by-id that
follows fails:

```rust
// crates/jcode-base/src/session/persistence.rs, before the fix
if !self.persist_state.snapshot_exists
    && !self.messages.iter().any(super::is_visible_conversation_message)
    && !self.saved
    && self.custom_title.is_none()
    && self.title.is_none()
    && self.parent_id.is_none()
{
    return Ok(());
}
```

Four tests fail as a result: three in `tests/e2e/session_flow.rs`
(`test_debug_create_session_marks_debug`,
`test_debug_create_selfdev_session_marks_canary`,
`test_clear_preserves_debug_for_resumed_debug_session`) and one in
`crates/jcode-tui/src/tui/app/tests/commands_accounts_02/part_02.rs`
(`test_improve_mode_persists_in_session_file`). All four fail with
`No such file or directory (os error 2)`.

## Reproduction

```console
$ cd <worktree at 2a4edaa02>
$ cargo test --test e2e session_flow:: -- --test-threads=1
failures:
    session_flow::test_clear_preserves_debug_for_resumed_debug_session
    session_flow::test_debug_create_selfdev_session_marks_canary
    session_flow::test_debug_create_session_marks_debug
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 60 filtered out

$ cargo test -p jcode-tui --lib test_improve_mode_persists_in_session_file -- --test-threads=1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2368 filtered out
```

The issue reported this on Linux; reproducing it on macOS shows it is not
platform-specific.

## Fix

Add the three missing clauses to the exemption list, with comments in the style
the surrounding ones already use:

```rust
    && self.parent_id.is_none()
    && self.improve_mode.is_none()
    && !self.is_debug
    && !self.is_canary
```

`improve_mode` is not in the issue's suggested fix but is required for the fourth
test. This was measured, not assumed: with only the `is_debug`/`is_canary`
clauses applied, the three e2e tests pass and
`test_improve_mode_persists_in_session_file` still fails with the identical
ENOENT.

## Evidence

Negative control is the same tree with the one hunk reverted, so the guard is the
only variable:

| State | `--test e2e session_flow::` | `-p jcode-tui --lib` (improve_mode) |
| --- | --- | --- |
| guard reverted | 3 failed / 6 | 1 failed |
| `is_debug` + `is_canary` only | 0 failed / 6 | **1 failed** (proves the third clause is needed) |
| all three clauses | 0 failed / 6 | 0 failed |

Wider suite, `cargo test -p jcode-base --lib session -- --test-threads=1`:
`126 passed; 3 failed`, and all three fail identically with the fix reverted, so
they are pre-existing and environmental, not regressions:

| Test | Cause |
| --- | --- |
| `auth::tests::cursor_status_is_available_for_authenticated_cli_session` | `left: NotConfigured, right: Available`; needs the `cursor` CLI configured |
| `platform::platform_tests::spawn_detached_creates_new_session` | fails at `platform_tests.rs:28`, before any session logic |
| `session::tests::cases::streaming_guard_creates_visible_macos_sleep_assertion` | asserts a macOS power assertion is released; reports 0 `BackgroundTask` assertions |

## Open question left for the maintainer

Other fields are explicit state in the same sense and remain unexempted:
`provider_key`, `reasoning_effort`, `subagent_model`, `autoreview_enabled`,
`autojudge_enabled`, `working_dir`, `short_name`. None is named by a failing test,
so the patch does not touch them, but the class recurs every time a field is
added. Expressing the guard as "persist once the metadata differs from a fresh
session's defaults" would close it permanently at the cost of a larger change.

## Relevant history

`783c979a0` introduced the early return; `2ca90e209` is a pure test split that
moved these tests and predates the regression. Related: #1119, #1249, #1144,
#1342.

## The integration line independently converged on the same clause

`crates/jcode-base/src/session/persistence.rs` on `jcode/ci-format-baseline`
carries `&& self.improve_mode.is_none()` in this same guard, reached by a
different route, which is corroboration that the third clause is the right answer
rather than a local workaround. That line also refactored the guard behind
`save_inner(resume_required)` with a `save_for_resume` entry point; that shape is
deliberately **not** proposed upstream here, because it is a wider API change
than this defect needs.
