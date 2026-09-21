# P3 spill-isolation patch parked, plus the tool_concurrency baseline finding

Written 2026-09-21 04:01Z by the dotfiles captain session (Fable 5.1, `session_turtle`). The
source tree had a second live writer (the TestEnvGuard refactor across `jcode-base` and callers),
so this session yielded the tree and released its lease. Nothing of the other writer's was
modified, committed, or discarded.

## Parked work

`git stash@{0}` on `jcode/ci-format-baseline`, also `~/.jcode/scratch/p3-spill-isolation.patch`:
4 files in `crates/jcode-app-core` (`agent/tool_output_spill.rs`, `tool/mod.rs`, `tool/tests.rs`,
`tool/tests_partition_01_tests.rs`). Adds `spill_truncated_output_in(dir, ...)`, a `#[cfg(test)]`
thread-local spill-dir override, injects a tempdir in
`test_context_guard_refusal_names_the_spilled_output`, and adds `lock_test_env()` to
`test_batch_guards_both_its_subcalls_and_its_own_aggregate` (root cause: hook tests set
`JCODE_HOOK_PRE_TOOL` under the lock; the batch test did not take it). Production path is
byte-identical; clippy clean on touched files. Apply after the lock refactor lands.

## Baseline finding: tool_concurrency fails 7-9/14 on every run

`agent/tool_concurrency.rs:168` treats any configured `pre_tool` or `post_tool` hook as a serial
barrier. `~/.jcode/config.toml` has `[hooks] post_tool` set on this host, and the tests read the
live home, so only call `"one"` ever starts and `wait_for_starts` times out at
`tool_concurrency_tests.rs:146`. Reproduce: `cargo test -p jcode-app-core --lib tool_concurrency`
fails; with `JCODE_HOME` pointed at an empty directory it should pass. Fix belongs in the tests
(isolate `JCODE_HOME` or clear hooks under the lock), not in the runtime.

Dotfiles side of this plan: `~/dotfiles/docs/research/2026-09-21-fable-architecture-review.md`.
