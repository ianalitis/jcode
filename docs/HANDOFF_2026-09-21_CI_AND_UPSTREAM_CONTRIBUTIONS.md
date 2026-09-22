# Handoff: fork CI/CD repair, upstream contributions, and the remaining red CI

Snapshot: 2026-09-21, prepared at source
`b3689db26d2854464d587c44caa42e9ca92ff760` on `jcode/ci-format-baseline`,
runtime `v0.86.234-dev (a61ab0927)`. This is a handoff, not a policy or an
authorization. Every number below was measured; where something is unverified it
is labelled as such.

## Paste into the fresh source session

> Read `docs/HANDOFF_2026-09-21_CI_AND_UPSTREAM_CONTRIBUTIONS.md`. Continue the
> fork CI/CD and upstream-contribution work it describes, testing and reviewing
> each result. Fix demonstrated causes rather than adding process, and reuse the
> existing receipts, branch ledger, plans and contribution preflight. Prepare and
> verify focused upstream changes, but stop at unapproved external effects. Keep a
> short native todo queue and evidence receipts. Do not silently switch provider
> routes, weaken gates, discard retained work, or claim that a source test proves
> runtime deployment. At each checkpoint report what improved, what remains, and
> the next bounded action in fewer than five lines.

Work in `/Users/ianalitis/.jcode/source/jcode`.

## 1. Authority and effect gates

Read `AGENTS.md`, `CONTRIBUTING.md`, `docs/FORK_POSTURE.md`,
`docs/UPSTREAM_RECONCILIATION_2026-09-21.md`,
`docs/BRANCH_LEDGER_2026-09-21.md`, `docs/FORK_CI.md`,
`docs/OSS_CICD_ROLLOUT_2026-09-21.md` and
`docs/plans/2026-09-21_OSS_CICD_STRATEGY.md`. The contribution preflight is
already installed in the root `AGENTS.md`.

Standing gates, in force unless the operator says otherwise:

- Pushing a focused `pr/*` branch to `fork` and opening the upstream PR it
  corresponds to has been approved for this workstream and has been exercised.
  Keep doing that for new focused fixes.
- Merging anything, pushing to `origin`, mutating issues/PRs beyond comments that
  answer a review, deleting branches/worktrees/stashes, installing packages,
  changing providers or auth, promoting the shared daemon, and any release
  action all still need explicit operator approval.
- Do not reopen a resolved upstream issue or duplicate an existing contribution
  merely because a local branch looks unmerged. Several of the TUI failures this
  workstream fixed were already claimed by other contributors' open PRs; check
  first (see §4).

## 2. Verified starting state

### Repository

- Source `b3689db26` on `jcode/ci-format-baseline`, working tree clean, write
  lease free, no stashes, 11 worktrees.
- After a **confirmed** fetch: `origin/master...HEAD` is `0 283` (behind, ahead),
  `origin/master...fork/master` is `0 7`, and `origin/master` is
  `2a4edaa02057ac994a601311c4f03ed450e1b3c9`. Upstream has not moved since this
  workstream started, so no rebase is outstanding.
- Runtime: running `a61ab0927`; current and shared-server channels `56f5d8238`;
  stable `8ffa8c333`. **Source contains work that the running binary does not.**
  A build/reload is not needed for any of the work below: it is all docs, CI
  plumbing, and test fixes.

### Concurrency warning, read this before touching the main checkout

A second session has been committing to this same checkout throughout the day.
It added `a09441d85`, `89946934a`, `b3689db26` and others, and at one point had
uncommitted URL-opening work in `crates/jcode-tui/src/tui/{ui.rs,ui/url.rs,app/navigation.rs}`.
Consequences to plan around:

- Use a **dedicated worktree** for each contribution branch. Do not switch the
  main checkout's branch; twice during this workstream it was mid-change and a
  branch switch would have destroyed another session's uncommitted work.
- `git commit --only -- <paths>` every time, and check `git status` before
  assuming the tree is yours.
- Two transient failures were caused by this, and both are recorded incorrectly
  elsewhere (see §6).

## 3. What was completed

### 3.1 Fork default-branch CI repaired (published)

The fork's default branch carried upstream's workflows, so every push and
scheduled run failed before compiling, and the fork's own `pr/*` branches ran
**no CI at all**. Two fast-forward commits on `fork/master`:

| Commit | Change |
| --- | --- |
| `3e5c99e49` | Secretless fork validation: no `DEPLOY_KEY`/ssh-agent, `permissions: contents: read`, `persist-credentials: false`, telemetry off for validation, `build-and-upload` gated on `github.repository == '1jehuang/jcode'`, artifact uploads fork-scoped, push CI on every branch, plus `workflow-lint.yml` and `docs/FORK_CI.md` |
| `8439023cc` | `cargo fmt` on the four `jcode-tui` files drifting on `master` |
| `f65d11cf4` | `FORK_PARENT` opt-in so a fork PR can satisfy `Require Linked Issue` with an upstream issue |
| `43a2e7539` | Published by the other session: labeler gate skips when `OPENROUTER_API_KEY` is absent |

Measured effect on fork `master`: `Format`, `iOS TestFlight`,
`Windows Cross-Target Check (Linux)` and `Workflow Lint` went failure →
success, and push CI now runs on every branch. Full receipt:
`docs/upstream-feedback/2026-09-21-fork-ci-default-branch.md`.

Remaining red on fork `master`, all reproduced on a pristine upstream copy and
therefore **upstream defects, not fork configuration**: `Quality Guardrails`
fails `cargo check --all-targets --all-features` (`src/bin/tui_bench.rs` does
not compile), and `Build & Test (ubuntu/macos-latest)` fail the `jcode-tui`
library tests.

### 3.2 Greptile is live on the fork and has been used

Operator installed the Greptile GitHub App on `ianalitis/jcode`. Verified: fork
PR #3 ran `Greptile Review: pass`, and fresh reviews run on push. It found a
real defect in our own fix, which was then fixed (§3.3). Use it: open a fork PR
when a change needs independent review before it goes upstream. Note that a fork
author cannot `RequestReviewsByLogin`; Greptile re-runs on push, and a
force-push counted as a push.

### 3.3 Four upstream contributions opened, all reviewed

| PR | Issue | Subject | Review state |
| --- | --- | --- | --- |
| [#1362](https://github.com/1jehuang/jcode/pull/1362) | #1352 | swarm stop quiesces queued and active turns | 5/5 after a review follow-up |
| [#1364](https://github.com/1jehuang/jcode/pull/1364) | #1363 | two unused test-module imports | 5/5 |
| [#1366](https://github.com/1jehuang/jcode/pull/1366) | #1365 | TUI git-probe test isolation (a regression of closed #1133) | 5/5; one P1 pointed at files now owned by #1368 and was answered |
| [#1368](https://github.com/1jehuang/jcode/pull/1368) | #1367 | two account tests assert labels `upsert_account` never assigns | 0 findings on the fixed head |

Details that matter for a fresh session:

- **#1362 carries one upstream-cleanup line** that is not part of the defect: it
  moves a misplaced `#[expect(clippy::too_many_arguments)]` from
  `resolve_swarm_spawn_effort` (2 args) onto `spawn_swarm_agent` (21 args),
  because the expectation is unfulfilled on `master` and the patch cannot compile
  under `-D warnings` without it. It is called out in the PR body.
- **#1368 nearly shipped a vacuous test.** Greptile's P2 was correct: a single
  inserted account is auto-activated, so switching to it passed even with
  `submit_input` removed. The test now inserts two accounts and has a failing
  negative control. Lesson worth keeping: for review-bot findings, reproduce the
  finding before accepting or rejecting it. This one was reproduced by deleting
  the switch and observing the test still pass.
- Combined effect of #1366 + #1368 on upstream `master`:
  `cargo test -p jcode-tui --lib -- --test-threads=1` goes **19 failed → 15**.

### 3.4 Receipts written

- `docs/FORK_POSTURE.md` — packet table now lists #1362/#1364/#1366/#1368 with
  their review verdicts.
- `docs/upstream-feedback/2026-09-21-fork-ci-default-branch.md` — the fork CI
  repair and why the rest is upstream.
- `docs/upstream-feedback/2026-09-21-app-core-unused-test-imports.md` — #1363.
- `docs/upstream-feedback/2026-09-20-swarm-stop-queued-worker.md` — extended with
  the upstream applicability proof, port evidence, the review follow-up, and the
  related app-core lint findings.
- `docs/OSS_CICD_ROLLOUT_2026-09-21.md` (other session) — the portfolio rollout.

## 4. Immediate next work, in priority order

### 4.1 Confirm the remaining TUI failures are not already claimed

`master` still has **15** `jcode-tui --lib` failures single-threaded after
#1366 + #1368. Before writing code, map each to an existing issue/PR. Known
ownership as of this handoff:

- #1340 (copy-selection notice suffix, 3 tests) and #1341 (OrcaRouter) are claimed
  by **PR #1344** (author `MatrixMagician`).
- #1358 / the 11 `jcode-base` lib failures are claimed by **our PR #1360**.
- #1339 (`Session::save()` skipping `is_debug`/`canary`, 4 tests) — **open and
  unclaimed**. Our line fixes it in `113c0cd06`
  (`fix(session): persist headless and cleared sessions for resume`, 4 files:
  `agent.rs`, `server/client_session.rs`, `server/client_session_clear.rs`,
  `server/headless.rs`). This is the best next candidate: well diagnosed by the
  issue author, no PR claims it, and our fix is small and targeted.
- #1342 tracks the remainder. Verify each against that body before implementing.

Capture the authoritative list from a pristine upstream tree rather than from a
stale note:

```sh
scripts/bounded.sh 900 cargo test -p jcode-tui --lib -- --test-threads=1 2>&1 \
  | grep -E "^test .*FAILED" | sed 's/^test //; s/ \.\.\. FAILED//' | sort -u
```

### 4.2 The `tui_bench` all-targets break

`cargo check --all-targets --all-features` fails for `src/bin/tui_bench.rs`
(`diff_line_wrap` is not a member of `TuiState`; `focus_revision` missing from a
`SidePanelSnapshot` initializer). It is already fixed inside **PR #1354**
(same author, still OPEN). Do not duplicate it; either wait for #1354 to land or,
if the fork needs green sooner, gate the fork's own expectation on it.

### 4.3 Remaining local-only fixes that are real and unshipped

These commits on our line reduce TUI failures but have no upstream PR. Evaluate
each against §4.1 before porting:

| Commit | Subject | Evidence in its message |
| --- | --- | --- |
| `113c0cd06` | persist headless and cleared sessions for resume | fixes the #1339 ENOENT |
| `34f2fbd2d` | stop the test suite deadlocking at default parallelism | hung >10 min → 45 s |
| `9b3f8b1f2` | stop the live git probe leaking real repo state | shipped as #1366 |
| `36e4106a6` | avoid rearming focus reports on focus events | — |
| `c22c4…`, `5dfb9a674`, `f5604cb98`, `43b625219` | cohort/expectation and predicate fixes | check each |

Winner `34f2fbd2d` is structurally important (it unblocks running the suite at
default parallelism) but touches render-state locking; treat it as a
concurrency change needing its own review, not a drive-by.

### 4.4 Fork CI/CD items still open (need approval)

From `docs/OSS_CICD_ROLLOUT_2026-09-21.md` §7, unchanged:

- Enable workflows on the four other public forks. Note the new finding: those
  forks have **no registered workflows** despite `ci.yml` being present in the
  tree, so this is *enable*, not *dispatch*. Resolve
  `mermaid-rs-renderer`'s `release.yml` first.
- `dependabot.yml` upstream, SHA-pin actions, build provenance, and a
  default-branch ruleset.
- **New and worth doing early:** the fork's `Release` workflow is *active* on
  `push: tags v*` with `contents: write` and no `github.repository` guard, and the
  fork already carries 30 `v*` tags. It has 0 runs ever, so pushing a tag today
  would create a fork release with fork-built assets. The two remedies are a
  repository guard or disabling the workflow on the fork; the second is smaller
  and does not diverge from upstream. This is an unguarded external-effect path
  and should be closed before any tag work.

## 5. Method that worked here

- **Reproduce on a pristine upstream copy before claiming anything is upstream's
  defect.** `git archive origin/master | tar -x -C <scratch>` plus `cargo
  --locked --offline` is enough, and it is what separated "our fork is
  misconfigured" from "upstream master is red".
- **Negative-control every fix.** Delete the mechanism and confirm the test fails
  for the intended reason. Two of this workstream's fixes were only proven by
  that step, and one (#1368) was only caught as vacuous because a review bot
  prompted it.
- Worktree per branch, `--only` commits, `scripts/bounded.sh` for anything that
  can wedge.
- Upstream's CI on our PRs needs maintainer approval (`action_required`) because
  we are an external fork. That is upstream policy, not a defect; Greptile is the
  only check that runs automatically there.

## 6. Corrections to existing records

Two claims in `docs/OSS_CICD_ROLLOUT_2026-09-21.md` §6 are wrong and should be
corrected by whoever next edits that file:

- **"`scripts/bounded.sh` does not exist."** It does. It is tracked, present at
  `HEAD` (`git cat-file -e HEAD:scripts/bounded.sh` succeeds), introduced by
  `872f7d148`, and 1471 bytes. It was *transiently* absent from the working
  directory while the concurrent session's checkout moved, which is also observed
  twice by this session. The real defect is narrower and worth recording: during a
  branch switch the file can vanish mid-command, and a failure hidden behind a
  pipe makes a skipped fetch look successful. Prefer re-running and verifying the
  effect over treating `bounded.sh` as unusable.
- **"`origin/master` was stale by 274 commits."** After a confirmed fetch the
  count is `0 283` (behind, ahead) and `origin/master` is `2a4edaa02`, which has
  not moved. The stale-ref observation was probably real at the time but the
  conclusion is not reproducible now; state ref counts only after a fetch whose
  success was confirmed.

## 7. Definition of done for the next session

For each packet: owned diff inspected, discriminating check passed (with a
negative control where a defect is being fixed), required review completed,
scoped commit recorded in an existing receipt, and the branch pushed only to
`fork`. State source-only verification separately from runtime and from upstream
publication/merge. If blocked, leave a reproducible receipt rather than a hidden
worker or a recursive scheduled task.
