# Upstream reconciliation receipt: 2026-09-21

This follows [the Astra workforce handoff](HANDOFF_2026-09-21_ASTRA_EPHEMERAL_WORKFORCE.md).
It records verified source changes, not a deployed runtime or an accepted unattended workforce.

## 1. Current posture and approved publication

**Published with operator approval at 16:44 UTC. GitHub fork/master is now 0 behind upstream.**

The non-force atomic push completed at 16:47 UTC. `fork/master` is `dc12efa7a`,
with the exact source tree of upstream `2a4edaa02`. Its three ahead commits are
the two preserved historical fork commits plus the merge, not source differences.
PR #1356 now points to `27231db08`; PR #1357 points to `dc4651417`. GitHub APIs
and `git ls-remote` independently confirmed these heads. Neither PR was merged.

Before publication, with upstream at `2a4edaa02`:

| Ref | Behind upstream | Ahead upstream | Role |
| --- | ---: | ---: | --- |
| `master` | 0 | 0 | Local upstream mirror |
| `jcode/ci-format-baseline` at `178a7d446` | 0 | 257 | Local integration, before this receipt |
| `fork/master` at `e09acaa7a` | 1997 | 2 | Stale GitHub fork default branch |

`origin` is `1jehuang/jcode`. `fork` is `ianalitis/jcode`. The GitHub banner did
not describe the active development checkout. It described the stale remote
mirror. The integration checkout initially lacked only one upstream docs commit,
which was merged as `803b731d2`. Existing local `master` was fast-forwarded to
upstream. No branch was reset or force-pushed.

Both fork-only commits are patch-equivalent to changes already upstream:

| Fork commit | Upstream equivalent | Stable patch ID |
| --- | --- | --- |
| `e09acaa7a` | `1d87eadb6` | `b4300226388c251a7037753fcc7fd38283f7dca8` |
| `5cb7b3dad` | `ba900d276` | `ead0ed449406afe368327a9095891928d76ce73c` |

The captain verified patch identity and upstream ancestry. There is no unique
feature content to rescue from those two commits.

The operator approved the following on 2026-09-21 at 16:44:53 UTC. All three
publication steps are complete:

1. Creating `jcode/fork-mirror-sync` from refreshed `fork/master`, merging refreshed
   upstream into it, and resolving the known account-login/chart conflicts to
   exact upstream content. Verify the resulting tree equals upstream while both
   histories remain ancestors. Do not include the private integration line.
2. Non-force push of that reviewed result to `fork/master`.
3. Non-force pushes of the reviewed local PR branches listed below to their
   matching `fork` branches. This updates PRs, not merges them upstream.

Only the approved sync branch and three non-force remote updates were created.
No new worktree, history rewrite, upstream PR merge, deletion, installation,
auth/provider/config change, or daemon reload occurred.

## 2. PR findings and published follow-ups

All four supplied Greptile findings were valid on the published PR branches.

| PR | Decision and repair | Local follow-up |
| --- | --- | --- |
| [#1356](https://github.com/1jehuang/jcode/pull/1356) | Clear held ownership before the underlying env mutex unlocks. Update explicitly typed callers to `TestEnvGuard`. Signal contention from the actual `WouldBlock` path, not a sleep/readiness guess. | `pr/test-env-lock-bounded-wait` at `27231db08` |
| [#1357](https://github.com/1jehuang/jcode/pull/1357) | Run atomic status publication on the blocking pool. Keep serialization through rename even if the awaiting caller is cancelled. Deterministically pause a staged write while readers and the runtime are exercised. | `pr/background-status-atomic-writes` at `dc4651417` |
| [#1360](https://github.com/1jehuang/jcode/pull/1360) | Integrate the already-published documented-default assertion correction, preserving author attribution. | `9618d3e95`, integrated as `4145bb765` |
| [#1354](https://github.com/1jehuang/jcode/pull/1354), [#1355](https://github.com/1jehuang/jcode/pull/1355) | No additional inline findings at inspection. Their lint/socket repairs are represented in integration. | No invented follow-up changes |

The two new PR follow-ups are committed and **published to the matching fork branches**. They retain
the older contribution layout and do not merge the integration branch or its
refactors. All 13 changed/new atomic methods match accepted integration behavior,
except that an unrelated pre-existing adopted-output change was deliberately not
ported. The six new atomic regression tests match byte-for-byte.

### Atomic publication details

Integration commits: `d763de3ab` and cancellation error handling `178a7d446`.

- Reuse one existing mutex as the status read/modify/publish gate. No new actor,
  per-task registry, or coalescing mechanism.
- Move its owned guard into `spawn_blocking` with the atomic temp-write/rename.
  An aborted async waiter cannot release serialization while rename is pending.
- Cover initial, progress/checkpoint, delivery, watchdog, completion, cancellation,
  reload, and reconciliation status publication. Preserve terminal-before-prune
  and initial-before-start notification ordering.
- Lock order is status gate before live-task map. Map guards drop before awaits.
- Cancellation still stops live work when status JSON is missing or corrupt.
  Cancelling the cancellation caller cannot strand the removed task as Running.
- Log unexpected join/signal failures. Expected aborted joins and Unix ESRCH are
  handled explicitly. No error-budget baseline was raised.

The filesystem work remains best-effort, matching the existing public API.
A slow filesystem serializes status operations globally, but does not occupy a
Tokio worker during publication. This tradeoff is explicit, not a throughput claim.

### Deterministic lock coverage

Integration already contained the guard-lifetime repair from `450702133`.
`0221b6ae2` strengthens its regressions:

- Actual `WouldBlock` observation must arrive before the holder releases.
- A released same-thread record is not reentry, a live same-thread record is,
  and a different thread is not the recorded owner.
- After a real guard drops, acquiring the raw mutex proves the last record is
  released without depending on which other test thread ran in between.

## 3. Test-harness repairs and acceptance evidence

Six reactive compaction assertions failed both parallel and serial runs because
`CompactionManager::new()` loaded the operator's proactive mode. This was not a
thread race. `c622b79ba` pins eight reactive fixtures, including two otherwise
vacuous checks, to the strategy each test intends to exercise. No production
compaction behavior or operator config changed.

A later full run exposed `effort_scaling_never_shrinks_the_base_budget` reading
process-global configuration repeatedly while other tests switched `JCODE_HOME`.
`5d101edcd` adds the shared env guard to that test. No provider timeout changes.

| Gate | Result |
| --- | --- |
| Compaction family after fixture repair | 37 passed |
| Full integration `jcode-base --lib`, parallel | 1549 passed, 0 failed, 5 ignored |
| Full integration `jcode-base --lib -- --test-threads=1` | 1549 passed, 0 failed, 5 ignored |
| Integration background selector after final cancellation logging | 44 passed |
| Full integration `jcode-app-core --lib` after final repair | 1529 passed, 0 failed, 31 ignored |
| Integration `scripts/check_guardrails.sh` | All configured gates pass |
| PR #1356 `jcode-base --lib storage::tests` | 11 passed |
| PR #1356 `cargo check --workspace --all-targets` | Passed, existing warnings remain |
| PR #1357 `jcode-base --lib background::` | 27 passed |
| PR #1357 `cargo check --workspace --all-targets` | Passed, existing warnings remain |

The full parallel base suite was also rerun on the restored final integration
source and passed again. The serial run precedes the final logging-only follow-up.
Background tests, app-core, all-target/all-feature checking, clippy, formatting
and ratchets were rerun after that follow-up. `cargo machete` is optional and was
not installed, so the existing guardrail script reported it skipped. Nothing
was installed to alter that state.

Negative controls were run and restored:

- Truncating the destination JSON while a staged replacement was held caused the
  staged-old/new regression to fail. Restored code passed all six new regressions.
- Leaving `held = true` on real guard drop caused the released-owner test to fail.
  Restored code passed all 11 storage tests.
- The worker's pre-fix single-worker heartbeat proof failed under synchronous
  terminal publication and passed with the off-worker publisher.

The broad `background` substring on the older #1357 baseline also selected three
reactive compaction tests and failed those known fixture assertions. The actual
`background::` module passed. The unrelated fixture fixes were not folded into
that PR, and its entire library suite is not claimed green.

Logs under `~/.jcode/scratch/`:

- `reconcile-base-full-before.log`, `reconcile-compaction-isolated.log`
- `reconcile-base-final.log`, `reconcile-base-final-serial.log`, `reconcile-base-final-head.log`
- `reconcile-background-final.log`, `reconcile-app-final.log`
- `reconcile-atomic-negative-control.log`, `reconcile-atomic-restored.log`
- `reconcile-storage-negative-control.log`, `reconcile-storage-restored.log`
- `reconcile-guardrails-final.log`
- `reconcile-pr1356-storage.log`, `reconcile-pr1356-workspace.log`
- `reconcile-pr1357-background.log`, `reconcile-pr1357-module.log`,
  `reconcile-pr1357-workspace.log`

Older PR branches lack `scripts/bounded.sh`. Their commands used an unchanged
copy from integration in scratch, not an unbounded command or a PR scope expansion.

## 4. Preserved work and remaining limits

Fresh script-generated inventory: `~/.jcode/scratch/upstream-reconcile-ledger.md`.
Its timeout entries are unresolved checks, not demonstrated conflicts. All
existing branches and worktrees were preserved. No stashes existed at inventory.
Older ledger stash entries are historical, not deletion instructions.

One dirty worktree is intentionally untouched:
`../worktrees/data-class-admission/crates/jcode-harness-api-server/src/translate_regression_tests.rs`.
Its untracked 3298 bytes are a byte-identical prefix of the current 18600-byte
canonical file. It was not deleted, even though no unique content was found.
The active integration checkout and the two prepared PR branches are clean after
committing. This is not a claim that every old branch has been integrated.

The prior app-core refused-socket failure did not reproduce in two full runs.
That does not prove its intermittent cause is fixed. Existing auth-route,
privacy/data-admission branch remainders and TUI timing tests still need bounded,
separate review. Do not wholesale merge old branches based on ancestry counts.

Read-only Sol/high lifecycle review did **not** accept unattended self-compaction
or tool-enabled Pi. Native notes remain volatile, request/event correlation and
failure/retry are incomplete, duplicate notes replace one another, byte/character
limits disagree, and automatic post-compaction continuation is unproven. Bridge
attempt authentication, ACK correlation and durable restart semantics need work.
Review artifact: `~/.jcode/scratch/reconcile-sol-review.md`.

The shared/running binary was `a61ab0927` at inventory. No reload was done. New
source tests are not evidence that the daemon now serves these changes.

Tool-interface observation on `v0.86.234-dev (a61ab0927)`: schema-admitted null
optional booleans were rejected as `invalid type: null, expected a boolean`.
Explicit false was the safe workaround. No unrelated harness-schema edit was made.

## 5. Keeping contribution friction low

[The fork posture](FORK_POSTURE.md) now distinguishes mirror, integration and
single-purpose PR branches and gives explicit behind/ahead checks. Use those
checks at session start and before preparing a contribution. They are documented
workflow checks, not an installed automatic synchronization service.

Keep `master` a mirror, local policy/customizations on the integration line, and
PR branches based on current upstream with only their own reviewed change.
Classify old work using the existing branch-ledger script and patch equivalence,
not ancestry alone. Refresh refs before an approved push, preserve unrelated
staging, run the scoped tests, and never publish the integration line as a mirror.

## 6. Publication verification

- Mirror merge: `dc12efa7a687fe97e2a319917938f24aaf3121a1`.
- Parents: old fork `e09acaa7a8828bcd46d9a5ab7c3434112e04e713` and upstream
  `2a4edaa02057ac994a601311c4f03ed450e1b3c9`. Both remain ancestors.
- Entire tree equals upstream: `3f3ca1fdf954104fc8b9eb5f58270c80ba846e55`.
- Resolved only the reviewed account-login implementation, its tests, and
  `docs/images/star-history.svg` conflicts to exact upstream content.
- Account-login smoke tests: 6 passed, 0 failed, 1 live test intentionally ignored.
- `git push --atomic` updated exactly `fork/master`,
  `fork/pr/test-env-lock-bounded-wait`, and `fork/pr/background-status-atomic-writes`.
  No force flag or integration-branch push was used.
- Fetched remote refs, verified all three full SHAs with `git ls-remote`, and
  verified both open PR head SHAs through GitHub.
- Returned to `jcode/ci-format-baseline`. Local `master` still exactly mirrors
  upstream. The sync branch is retained, not deleted.
- At the immediate post-push check, both Greptile reviews were in progress.
  Local acceptance above does not claim hosted review completion.

Publication artifacts in `~/.jcode/scratch/`: `reconcile-mirror-merge.log`,
`reconcile-mirror-account-login.log`, `reconcile-approved-push.log`, and
`reconcile-published-refs.json`.

## 7. Detached cancellation grace review follow-up

A subsequent #1357 review correctly found that detached Unix cancellation held
`status_updates` while awaiting the caller's grace timeout. Off-worker filesystem
publication did not prevent that manager-wide lock from delaying other tasks.

- Integration repair: `f1233129b`.
- Focused PR-branch repair: `245c44b4d`, one local commit after published `dc4651417`.
  The operator separately approved this follow-up at 17:35 UTC. It was pushed
  non-force at 17:36 UTC and verified with `git ls-remote` and GitHub PR metadata.
  The first immediate PR read lagged the successful push, so verification was
  repeated without pushing again. The expected head is `245c44b4dcd40e564f9fe5970bda3925a5b23f63`.
- Release the owned guard after TERM and before the grace wait. Reacquire and
  reread status afterward. Proceed only if still Running, detached, and using the
  original PID. This preserves newer terminal states and avoids stale delayed
  KILL/publication against a replacement record.
- Use refreshed progress/delivery metadata when cancellation does proceed.
  Existing atomic publication and cancelled-caller guard ownership are unchanged.
- Three deterministic Unix regressions use isolated process groups, explicit
  grace-boundary handshakes, paused time, bounded waits, and fixture cleanup.
  They verify unrelated progress can publish, a newer terminal status wins, and
  real cancellation retains updates made while its target remained Running.
- The unrelated-progress test failed before the fix. Captain's negative control
  removing the refreshed-status assignment made the metadata regression fail.
  Restored code passes 30 background tests and the full base suite:
  **1552 passed, 0 failed, 5 ignored**.
- Full configured integration guardrails pass. Optional absent `cargo machete`
  remains skipped by the existing script. On the older PR branch, all 30
  background tests and `cargo check --workspace --all-targets` pass, with its
  existing warnings. The ported method and tests match integration exactly.

Evidence under `~/.jcode/scratch/`: `cancel-grace-builder.md`,
`cancel-grace-metadata-negative.log`, `cancel-grace-background-captain.log`,
`cancel-grace-base-full.log`, `cancel-grace-guardrails.log`,
`cancel-grace-pr1357-tests.log`, and `cancel-grace-pr1357-workspace.log`.
The approved follow-up was published. No PR merge, configuration change, or daemon reload was made.

## 8. Approved local and public cleanup

The operator approved both batches in the
[existing branch ledger](BRANCH_LEDGER_2026-09-21.md) at 17:54 UTC. Execution on
2026-09-21 was limited to those named targets:

- Rechecked all six approved branch tips, then removed their local branch names.
  Removed the clean integrated `attempt-billing-route` worktree without force
  before deleting its branch. Removed only the verified 3298-byte duplicate
  untracked test file in `data-class-admission`, keeping that worktree and branch.
  Inventory is now 27 local branches, 6 clean worktrees, and 0 stashes.
- Closed PR #1293 as already fixed upstream by `e7a22f695`, and PR #1295 as
  superseded by focused PR #1354. Both exact approved heads were rechecked before
  closure. Neither was merged and neither remote branch was deleted.
- Closed issue #1292 as completed. The closure preserves the verification caveat:
  the unchanged upstream fingerprint test still fails only for the independent
  `JCODE_MEMORY_JEV_PROVIDER` omission tracked by #1358 / PR #1360.
- PR #1360 already starts with `Fixes #1358.` at approved head `9618d3e95`.
  That equivalent closing linkage makes an additional `Closes #1358` unnecessary,
  so no description edit was made. #1294 and #1358 remain open.
- Independent GitHub reads confirmed 5 open / 2 closed authored PRs and 25 open /
  13 closed authored issues. All five active contributions remain open.
- Regenerated the canonical ledger against `851ff2c8c`. Its timeout rows remain
  unknown merge outcomes, not demonstrated conflicts. Unreviewed branch remainders
  and historical plans were preserved, not declared integrated or deleted.

Evidence: `approved-cleanup-a.json`, `approved-cleanup-b.json`,
`approved-cleanup-inventory.json`, and `approved-cleanup-ledger.md` under
`~/.jcode/scratch/`. This follow-up changes no production code, source-test result,
provider/config setting, or runtime deployment. No new push or PR merge occurred.
