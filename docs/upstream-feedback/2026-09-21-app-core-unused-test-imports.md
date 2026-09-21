# Two unused test-module imports fail `-D warnings` in `jcode-app-core`

Date: 2026-09-21. Status: filed upstream as
[#1363](https://github.com/1jehuang/jcode/issues/1363) and published as
[#1364](https://github.com/1jehuang/jcode/pull/1364) from
`pr/app-core-unused-imports` (branch base `origin/master` `2a4edaa02`, commit
`efc95edca`).

## Expected and observed

`scripts/warning_budget.txt` is `0`, and CI runs
`cargo check --all-targets --all-features` followed by
`cargo clippy --all-targets --all-features -- -D warnings`. On upstream `master`
`cargo check` reports two unused-import warnings in `jcode-app-core` test
modules:

```
warning: unused imports: `Mutex` and `OnceLock`
  --> crates/jcode-app-core/src/server/debug_command_exec.rs:656:22
warning: unused import: `OnceLock`
  --> crates/jcode-app-core/src/server/provider_control_tests.rs:10:65
```

`debug_command_exec.rs`'s `mod tests` imports `{Arc, Mutex, OnceLock}` while
every `Mutex` use is spelled `std::sync::Mutex` and `OnceLock` is unused, so only
`Arc` is needed. `provider_control_tests.rs` imports
`{Mutex as StdMutex, MutexGuard as StdMutexGuard, OnceLock}` where `StdMutex` and
`StdMutexGuard` are used and `OnceLock` is not.

## Reproduction

```bash
git checkout master
cargo check -p jcode-app-core --all-targets --all-features 2>&1 | grep '^warning'
```

Expected: no warnings. Observed: the two above, on `rustc 1.98.1` and `1.94.1`
alike (the warnings are toolchain-independent).

## Fix and evidence

Two import lines, no behavior change. After the change the two `unused import`
warnings are gone and no new warning appears.

The nine `never constructed` / `never used` warnings that `cargo check` also
reports for `tool/goal.rs` are a separate cause: the initiative tool was removed
from the production registry while its module stayed compiled. They are handled
by the `#[cfg(test)] mod goal;` gating in the open lint PR for
[#1348](https://github.com/1jehuang/jcode/issues/1348).

## Why this is separate from the open work

- The sites are not in #1348's list of twelve platform-independent clippy sites
  plus the macOS `stdin_detect` one.
- The open test-env-lock PR touches both files but only changes `lock_env`'s
  return type to `jcode_base::storage::TestEnvGuard`; `Mutex`, `MutexGuard` and
  `OnceLock` remain unused after it.

## Also observed while measuring (not in this patch)

`crates/jcode-app-core/src/server/comm_session.rs` carries
`#[expect(clippy::too_many_arguments, ...)]` above `resolve_swarm_spawn_effort`,
which takes two arguments, instead of above `spawn_swarm_agent`, which takes
twenty-one. The expectation is unfulfilled, so `-D warnings` fails there too.
That attribute is moved by the swarm-stop PR
([#1362](https://github.com/1jehuang/jcode/pull/1362)), which needs the fix to
compile under CI's `-D warnings`; it is not duplicated here.
