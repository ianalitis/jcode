# Ctrl+Up discards an unsubmitted draft

Date: 2026-09-22. Observed on upstream `ef4c2bd694d07eb10d75603f20e9f450be3d373a`
(macOS). Reported as
[#1361](https://github.com/1jehuang/jcode/issues/1361) by `theammir`, who
diagnosed it and proposed the one-line shape of the fix. Fix prepared as
[#1379](https://github.com/1jehuang/jcode/pull/1379) from
`pr/ctrl-up-preserve-draft`.

## Expected and observed

Pressing Ctrl+Up (or Alt+Up, or Cmd+Up, which share the binding) with a
non-empty composer jumps straight into prompt history even mid-draft. When the
composer holds text that is not in history, the draft is overwritten with no
undo snapshot and no history slot, so it is unrecoverable: Ctrl+Z has nothing to
restore, and Ctrl+Down at the newest entry clears to empty rather than walking
back to it.

Browsing history should not be destructive. The plain-Up path already refuses to
replace an unmatched draft, and the Ctrl+R search overlay in the same module
already captures and restores the draft it displaces.

## Where the draft is dropped

`crates/jcode-tui/src/tui/app/input.rs`, `handle_prompt_history_navigation`, the
`explicit_history` branch:

```rust
let Some(current_index) = history.iter().rposition(|prompt| prompt == &app.input) else {
    if explicit_history && matches!(code, KeyCode::Up) {
        return history
            .last()
            .map(|prompt| {
                app.input = prompt.clone();          // draft overwritten here
                app.cursor_pos = app.input.len();
                app.reset_tab_completion();
                app.sync_model_picker_preview_from_input();
            })
            .is_some();
    }
    return false;
};
```

The dispatcher in `handle_key_core` normalizes the modifier to `CONTROL`
precisely so this path is taken mid-draft, and that intent is preserved by the
fix. Both the local and the remote/SSH key paths reach this one function
(`remote/key_handling.rs` calls `input::handle_prompt_history_navigation`).

## Reproduction

Two tests were added in
`crates/jcode-tui/src/tui/app/tests/scroll_copy_01/part_02.rs`:

```console
$ cd <worktree at ef4c2bd69>
$ cargo test -p jcode-tui --lib -- --test-threads=1 ctrl_up
test tui::app::tests::test_ctrl_up_mid_draft_keeps_the_draft_recoverable ... ok
test tui::app::tests::test_ctrl_up_repeated_recall_still_restores_the_draft ... ok
test result: ok. 5 passed; 0 failed
```

Against the same tree with only the production hunk reverted, both fail:

```console
test tui::app::tests::test_ctrl_up_mid_draft_keeps_the_draft_recoverable ... FAILED
test tui::app::tests::test_ctrl_up_repeated_recall_still_restores_the_draft ... FAILED
```

Full-suite control, `cargo test -p jcode-tui --lib -- --test-threads=1`, new
tests present in both runs:

| Run | Result |
| --- | --- |
| pristine (production hunk reverted) | 2334 passed, **19 failed**, 18 ignored, 61.22s |
| patched | 2336 passed, **17 failed**, 18 ignored, 64.71s |

The two failure sets differ only by these two tests, so no pre-existing failure
changes state. The remaining 17 are the failures already owned elsewhere: 13 from
the Linux CI list (copy-selection trio #1340/#1344, account label #1367/#1368,
`test_improve_mode_persists_in_session_file` #1339, and the eight tracked by
#1342) plus 4 macOS-only `Alt`/`⌥`-label tests that the fork's CI already gates
on `runner.os == 'Linux'`.

## Fix

```rust
app.remember_input_undo_state();
app.input = prompt.clone();
```

`remember_input_undo_state()` is the mechanism roughly twenty other input
mutations already use (word deletion, line cut, paste, autocomplete,
drag-and-drop insertion), backing Ctrl+Z through `undo_input_change()`. History
recall was the conspicuous omission. The snapshot is taken only on the step that
replaces user-authored text, so walking within history does not bury the draft
under recalled prompts.

## Deliberately unchanged

The forward direction still clears to empty at the newest entry. Making the draft
the slot past the newest entry, the way shell and readline do and the way the
Ctrl+R overlay already behaves with `original_input`, needs a new `App` field and
touches every construction site; the report asks for recoverable "and/or"
non-destructive, and the undo snapshot is the smaller justified change. If
upstream prefers the fuller behavior, it is a follow-up on this PR rather than a
second one.

## Red checks on this branch

`Quality Guardrails` and `Format` fail on any branch based on current `master`
for reasons unrelated to this change: `src/bin/tui_bench.rs` does not compile
(`diff_line_wrap` is no longer a member of `TuiState`, `focus_revision` missing
from a `SidePanelSnapshot` initializer) and `cargo fmt --all --check` reports
drift in files this branch does not touch. Both are covered by
[#1354](https://github.com/1jehuang/jcode/pull/1354). `rustfmt --edition 2024
--check` on the two touched files reports no diff inside the changed hunks; the
two diffs it does report are at `part_02.rs:450` and `:484`, both pre-existing.

## Evidence commands

```sh
cargo test -p jcode-tui --lib -- --test-threads=1 ctrl_up
cargo test -p jcode-tui --lib -- --test-threads=1 prompt_history
cargo test -p jcode-tui --lib -- --test-threads=1     # A/B, logs in $JCODE_SCRATCH_DIR
rustfmt --edition 2024 --check crates/jcode-tui/src/tui/app/input.rs \
  crates/jcode-tui/src/tui/app/tests/scroll_copy_01/part_02.rs
```
