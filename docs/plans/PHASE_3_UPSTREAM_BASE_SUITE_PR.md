# Phase 3: port the base-suite env guard upstream (#1358)

Workstream W4 and W1. Independent of Phases 1 and 2. **Blocked on operator
decisions O2 (branch and worktree) and O3 (push and PR).** Do not start without
O2.

## 1. Goal and non-goals

Goal: a PR against `origin/master` that makes the eleven tests in #1358 pass at
the default thread count on a pristine checkout, with the A/B evidence in the PR
body.

Non-goals: porting our split test layout, our storage/status changes, or any
fork-only behavior. Not #1352 (swarm stop): that is a separate packet pending O8.
Not the `background::tests` race, which is #1350 / PR #1357.

## 2. Evidence that shapes the packet (verified 2026-09-21)

This is **not a cherry-pick**. `git diff --stat origin/master..HEAD -- crates/jcode-base/src`
shows the fork's `jcode-base` tests are split into `*_partition_NN_tests.rs` and
`*_body_tests.rs` files (3155 insertions / 2826 deletions across 25 files), so
the guard sites do not map 1:1 onto upstream's inline modules. The fork's
`auth/codex_tests.rs` has 7 `lock_test_env` sites and `auth/tests.rs` 15;
upstream's copies have fewer. The port is per test: find the eleven upstream
test bodies, add the guard the fork's copy uses, and nothing else.

The #1358 body already says the fork "gives each suite one shared guard, takes
the env lock once per test, routes status-file writes through temp-plus-rename".
Only the first two are in scope here; the third is PR #1357.

## 3. Acceptance test

In a worktree on a branch off `origin/master` (`e589cbe5a`):

```sh
cargo test -p jcode-base --lib 2>&1 | tail -3          # before: 1472 passed / 11 failed
# apply the port
touch $(git diff --name-only)                           # trap from the handoff: stale test binaries
cargo test -p jcode-base --lib -- --list | wc -l        # confirm the binary rebuilt
for i in 1 2 3; do cargo test -p jcode-base --lib 2>&1 | grep '^test result'; done
cargo test -p jcode-base --lib -- --test-threads=1 2>&1 | grep '^test result'
```

Numbers: 11 failed to **0 failed** in three consecutive parallel runs; the
single-thread count unchanged. Also `cargo fmt --all --check` and
`cargo clippy -p jcode-base --all-targets -- -D warnings` on the branch, noting
that upstream's own clippy is red at `e589cbe5a` for reasons covered by PR
#1354; the PR body must state that the branch introduces no *new* clippy
findings, measured by diffing the clippy output before and after.

## 4. Freeze

| Field | Value |
| --- | --- |
| Lane, model | writer `openai-oauth:gpt-5.6-terra`, effort medium. Reason: the port needs judgement about which of two test layouts each guard belongs in, and the PR body is reviewer-facing prose |
| Reader (optional) | `opencode-go:mimo-v2.5`, none effort: for each of the eleven upstream tests, return file, line range, and whether the fork's same-named test holds `lock_test_env`. Read-only, both trees |
| Writer scope | the worktree only. Permitted files: the eleven upstream test files that contain the named tests, plus `crates/jcode-base/src/storage.rs` only if the guard helper itself must be ported (check first: upstream already has `lock_test_env`) |
| Iteration cap | 2, then hand the diff and numbers back |

## 5. Gate and rollback

Gate: the acceptance block above, then `bash scripts/check_guardrails.sh --skip-slow`
inside the worktree. Rollback: delete the worktree and branch (both need O2
approval to create, and their removal is the writer's own scratch).

## 6. Budget and stop

Cold `jcode-base` test build in a fresh worktree: ~4 to 5 minutes. Three
parallel runs, one serial run. Under 45 minutes wall. Terra medium: budget
150K tokens; stop at that or at the iteration cap.

Stop on: any failing test outside the eleven; any change to production code;
a guard that needs a helper upstream lacks (then the PR grows and needs O3
re-approval with the wider scope).

## 7. Criticism

- Over-engineered? The risk is the opposite: a reviewer may ask why not fix the
  root (tests reading the developer's real `~/.jcode`). The honest answer is in
  #1358 already: a clean CI runner may not reproduce, and the guard is the
  minimal change that makes the developer-machine case deterministic. Put that
  sentence in the PR body.
- Missing evidence: whether all eleven reproduce on a pristine `e589cbe5a`
  *today* on this machine. The "before" run in section 3 is that evidence;
  if fewer than eleven fail, port only the ones that do and say so.
- Regression: adding `lock_test_env` to a test that transitively calls a locked
  helper self-deadlocks; upstream's lock is unbounded (PR #1356 bounds it), so
  run with `--test-threads=1` once as a sanity check and use
  `timeout 600` around every parallel run in the worktree.
- Abandon if: the port needs more than the eleven test bodies plus the guard
  helper. That means the fork's isolation depends on its split layout, and the
  upstream fix is a different change that should be discussed on the issue
  first.

## 8. Handoff

Needs O2 before spawn, O3 before push. Output: branch name, worktree path,
three-run table, clippy before/after diff, PR number once opened. Captain
updates `docs/FORK_POSTURE.md §7` table row for #1358.
