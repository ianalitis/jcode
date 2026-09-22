# Handoff: fork CI/CD repair, upstream contributions, and the remaining red CI

Snapshot: 2026-09-21, prepared at source
`b3689db26d2854464d587c44caa42e9ca92ff760` on `jcode/ci-format-baseline`,
runtime `v0.86.234-dev (a61ab0927)`. **Extended 2026-09-22 at source
`c53f2bedc`** by the second source session, which added §3.5 and §4.0 after the
CodeQL and Dependabot rollout landed and corrected one measurement error in §2
and §6. **Extended again 2026-09-22 at source `ed55c31ee`** by the third source
session, which applied item 10, completed the item-12 triage, corrected items 11
and 12 and the `bounded.sh` provenance, and opened three upstream contributions:
#1373 (from #1339), #1371 (from new #1369) and #1372 (from new #1370). New
material is in §3.6, §4.0, §4.1, §4.2, §4.4 and §9. This is a handoff, not a policy or an authorization. Every number below
was measured; where something is unverified it is labelled as such.

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

- Source `c53f2bedc` on `jcode/ci-format-baseline`, working tree clean, write
  lease free, no stashes, 11 worktrees.
- After a **confirmed** `git fetch --no-tags origin`: `origin/master...HEAD` is
  `0 287` (behind, ahead), `origin/master...fork/master` is `0 7`, and
  `origin/master` is `2a4edaa02057ac994a601311c4f03ed450e1b3c9`. Upstream has not
  moved since this workstream started, so no rebase is outstanding. The ahead
  count grew 285 -> 287 while this handoff was being written, from the concurrent
  session's own commits, which is why it is stated with the command rather than
  as a standing fact.
- **The integration line is 285 commits ahead of upstream, not in sync with it.**
  Composition: 3 merges and 282 non-merge commits. An earlier revision of this
  handoff and of `docs/OSS_CICD_ROLLOUT_2026-09-21.md` §6 recorded `0 7` for this
  count and concluded the line was "in sync with upstream". That reading was
  wrong: `0 7` belongs to `origin/master...fork/master`, and the `0 281` figure
  taken before the fetch was the correct one all along (281 plus four later
  commits = 285). Two independent sessions then measured 283 at `b3689db26` and
  285 at `c53f2bedc`, which agree. Do not plan as though a rebase or sync is
  unnecessary; confirm with the command above before relying on any count here.
- Runtime, re-measured 2026-09-22. **The symlinks no longer describe the running
  binary**, so resolve the process, not the link:
  - The daemon serving sessions is `a61ab0927`. Verified with
    `lsof -p 84667 | awk '$4=="txt"'`, which shows
    `versions/a61ab0927/jcode`; the process started Sun Sep 20 01:25 and is
    executing the binary it was started with.
  - `current`, `shared-server` and `stable` **all** now point at
    `versions/56f5d8238/jcode`, and a freshly launched CLI reports
    `jcode v0.86.277-dev (56f5d8238)`. It was started from the `shared-server`
    symlink, which has since been repointed — the clearest possible illustration
    of why resolving that symlink tells you where a *new* daemon would come from,
    not what is running.
  - `~/.jcode/builds/canary/` does not exist. The earlier claim that `stable` was
    `8ffa8c333` is wrong; `versions/8ffa8c333` is installed but unreferenced.
- **`scripts/bounded.sh` does not exist in a worktree based at `origin/master`**,
  and that is not a race: it was introduced on the integration line only
  (`872f7d148`, contained by `jcode/ci-format-baseline` alone) and is absent from
  upstream. From any worktree, use the absolute path
  `/Users/ianalitis/.jcode/source/jcode/scripts/bounded.sh`; otherwise the
  wrapper never runs and its own missing-file error is what surfaces. This
  corrects the second session's §6 correction, which was right about `HEAD` but
  left the absence looking transient. See the rollout receipt §6.
- **Source contains work that the running daemon does not.** A build/reload is not
  needed for any of the work below: it is all docs, CI plumbing, and test fixes.
  Anything that does need the new binary needs `selfdev build-reload` plus its own
  verification of the promoted binary.

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
- `docs/upstream-feedback/2026-09-21-greptile-labeler-missing-key.md` (other
  session) — the `Semantic PR labels` gate, its upstream contribution path, and
  the note that upstream's own labeler fails for an unrelated reason (HTTP 402).
- `docs/plans/2026-09-21_OSS_CICD_STRATEGY.md` (other session) — the fork
  portfolio strategy, the placement rule, and the gap register items 10-12 that a
  fresh session should work from.

### 3.5 Portfolio security configuration applied (by the second session)

Three of the strategy's gap-register items are now **applied and verified**, not
proposed. All five public forks (`jcode`, `handterm`, `mermaid-rs-renderer`,
`agentgrep`, `GLOOP`):

| Item | State | Verification |
| --- | --- | --- |
| Code scanning (CodeQL default setup) | configured on all five | `code-scanning/default-setup` returns `state=configured`; on `jcode`, `Analyze (actions/javascript-typescript/python/rust/swift)` all `success` |
| Dependabot alerts | enabled on all five | `PUT` and `GET /vulnerability-alerts` both `204` |
| `Semantic PR labels` gate | published as `43a2e7539`, green | dispatch run `35668523374` = success, `Check labeling is configured` = success, `label` = skipped |

Deliberately **not** done: Dependabot *security updates*
(`PUT /repos/{r}/automated-security-fixes`), because they open version-bump PRs
against the fork's default branch, which is a mirror. Do not turn them on without
deciding that first.

Two traps this rollout exposed, both already recorded in the rollout receipt:

- **Do not read a mid-run snapshot as a capability limit.** CodeQL's
  `Analyze (rust)` job was still `in_progress` roughly fifteen minutes in with
  zero `/language:rust` analyses while every other language had finished, and an
  earlier draft of the receipt concluded from that snapshot that Rust coverage was
  "unproven". That was wrong: `/language:rust` analyses were recorded on `jcode`,
  `handterm`, `mermaid-rs-renderer` and `agentgrep`, and `jcode`'s simply took
  ~20 minutes. A smaller repository with the same language settles the question
  faster than waiting on a large one.
- **`actions/permissions.enabled = true` does not mean a fork's workflows are
  registered.** See §4.4.

### 3.6 Third session: three more upstream contributions, and two items closed

Opened 2026-09-22, each from its own worktree at `origin/master`, each pushed to
`fork`, each linked to a real upstream issue.

| PR | Issue | Branch | Subject |
| --- | --- | --- | --- |
| [#1373](https://github.com/1jehuang/jcode/pull/1373) | #1339 | `pr/session-persist-explicit-state` | `Session::save()` dropped `is_debug`/`is_canary`/`improve_mode` on a blank session; all four named tests now pass, with the guard-reverted control failing them |
| [#1371](https://github.com/1jehuang/jcode/pull/1371) | [#1369](https://github.com/1jehuang/jcode/issues/1369) | `pr/freebsd-smoke-permissions` | `freebsd-smoke.yml` was the only one of eleven workflows with no `permissions` block; found by the CodeQL triage |
| [#1372](https://github.com/1jehuang/jcode/pull/1372) | [#1370](https://github.com/1jehuang/jcode/issues/1370) | `pr/release-repository-guard` | `Release` and the Discord announcement were not guarded to the canonical repository; the durable version of item 10 |

All three are `OPEN` and `MERGEABLE` as of this handoff. #1373 is the substantive
one: the issue's own suggested fix (is_debug and canary) leaves the fourth test
failing, which was measured by applying two clauses, observing the TUI test still
fail with the identical ENOENT, then adding `improve_mode`. The integration line
had independently added that same clause.

**A `pr/*` branch push gets no fork CI; a fork PR does.** Upstream's `ci.yml`
triggers on `push: branches: [main, master]`, and a workflow only runs if it is
present in the pushed commit, so a branch based on `origin/master` — which the
posture requires — carries upstream's trigger and never runs the fork's push CI.
This corrects the strategy's "push CI runs on every branch". Measured: three
`pr/*` pushes produced one run, `FreeBSD Smoke`, whose trigger is a path filter
and does not depend on `ci.yml`. Validated remedy: open a PR against
`fork/master`, which matches `pull_request` and takes the workflow from the base.
Fork PR #4 for `pr/session-persist-explicit-state` produced `CI` (in progress),
`Require Linked Issue` **success** via the committed
`FORK_PARENT: "1jehuang/jcode"` linking upstream #1339, and a `Greptile Review`.
**Do this for every `pr/*` branch, and never merge the fork PR**, because a merge
commit there would land inside the upstream PR's diff. Detail: rollout receipt §6b.

**A trap that cost real time, worth carrying forward: `gh pr view --json files`
reports a stale, over-inclusive file list for large PRs.** Thirteen open PRs
appeared to modify `freebsd-smoke.yml` and `release.yml`; all thirteen are
byte-identical to `master` in both files, and GitHub cannot even render their
diffs (`HTTP 406: the diff exceeded the maximum number of files (300)`). The
reliable collision test is comparing blob SHAs:

```sh
m=$(gh api "repos/1jehuang/jcode/contents/<path>?ref=master" --jq .sha)
s=$(gh api "repos/1jehuang/jcode/contents/<path>?ref=$head_sha" --jq .sha)
[ "$s" = "$m" ] || echo "this PR changes <path>"
```

Used that way for `crates/jcode-base/src/session/persistence.rs`, all **43** open
PRs are byte-identical to `master`, which is what makes #1339 unclaimed.

## 4. Immediate next work, in priority order

### 4.0 Triage the CodeQL alert backlog — **DONE 2026-09-22**

Read and concluded in
[`docs/upstream-feedback/2026-09-22-codeql-rust-alert-triage.md`](upstream-feedback/2026-09-22-codeql-rust-alert-triage.md):
**166** open alerts, **zero confirmed leaks**, two actionable items (a missing
`permissions` block on `freebsd-smoke.yml`, now PR #1371, and the misleadingly
named `sanitize_secret_value`). No alert was dismissed; dismissal is a repository
mutation and was left as an operator decision. The text below is retained as the
description of what was found, with the count corrected.

CodeQL's first run opened **100+ open Rust alerts on `jcode`** (measured: 166): 6 `critical`
(`rust/hard-coded-cryptographic-value`) and 94 `high` (90
`rust/cleartext-logging`, 4 `rust/cleartext-transmission`). None has been
triaged, so these are **not** vulnerability counts and should not be reported as
such.

A first pass, explicitly not a triage, suggests the six criticals are heuristic
false positives on non-cryptographic seeding: two in
`crates/jcode-tui/src/tui/ui_animations.rs` (a TUI animation module) and four in
`crates/jcode-tui-mermaid/tests/layout_cache_resize_probe.rs`. The
`rust/cleartext-logging` cluster is **not** uniformly test code —
`src/cli/login.rs` carries 24 and `src/cli/commands.rs` 13, in an
authentication-heavy CLI — so start there rather than dismissing the rule.

```sh
gh api "repos/ianalitis/jcode/code-scanning/alerts?state=open&per_page=100" \
  --jq '.[] | "\(.rule.security_severity_level)\t\(.rule.id)\t\(.most_recent_instance.location.path):\(.most_recent_instance.location.start_line)"' \
  | sort
```

CodeQL default setup does not fail the workflow on findings, so this backlog does
not block merges. Dismissing an alert is a repository mutation; triage first, and
only dismiss with evidence.

### 4.1 Confirm the remaining TUI failures are not already claimed

`master` still has **15** `jcode-tui --lib` failures single-threaded after
#1366 + #1368. Before writing code, map each to an existing issue/PR. Known
ownership as of this handoff:

- #1340 (copy-selection notice suffix, 3 tests) and #1341 (OrcaRouter) are claimed
  by **PR #1344** (author `MatrixMagician`).
- #1358 / the 11 `jcode-base` lib failures are claimed by **our PR #1360**.
- #1339 (`Session::save()` skipping `is_debug`/`canary`/`improve_mode`, 4 tests) —
  **taken 2026-09-22**: prepared as **PR #1373** from
  `pr/session-persist-explicit-state`, after confirming the issue was unclaimed by
  blob-SHA across all 43 open PRs rather than by file lists (§3.6). The fix is
  three clauses in the `persistence.rs` exemption list, not the larger
  fork-line `113c0cd06`, which also refactors the guard behind
  `save_inner(resume_required)` and moves `handle_clear_session` to satisfy a
  fork-local file-size ratchet. Packet:
  `docs/upstream-feedback/2026-09-22-session-persist-explicit-state.md`.
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
(same author). **Re-checked 2026-09-22: still `OPEN` and `MERGEABLE`, still carries
`src/bin/tui_bench.rs`, and `origin/master` still contains the break.** Do not
duplicate it; either wait for #1354 to land or, if the fork needs green sooner,
gate the fork's own expectation on it.

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

CodeQL and Dependabot alerts are now **applied** (§3.5). What remains needs
approval, and the cheapest item is also the highest-severity one.

**Item 10, the unguarded fork release path — APPLIED 2026-09-22.** The fork's
`Release` workflow was *active* on `push: tags v*` with `contents: write` and no
`github.repository` guard. It had 0 runs ever, so a `v*` tag pushed today would
have created a fork release with fork-built assets. It is now
`state: disabled_manually` (`gh workflow disable Release --repo ianalitis/jcode`),
rollback is `gh workflow enable Release --repo ianalitis/jcode`, and the disable
costs no divergence because the file on the fork is still upstream's. The
correction to the tag count is in the rollout receipt §5: the fork carries **192**
`v*` tags, not 30.

`discord-release.yml` is the other half of the same chain (`release: [published]`,
`contents: write`, unguarded) and was left active deliberately: the fork has
**zero Actions secrets**, so `DISCORD_RELEASE_WEBHOOK` resolves empty and the job
fails rather than announcing, and with `Release` disabled the only way to reach it
is to publish a fork release by hand. Both halves are closed by the upstream guard
now opened as **PR #1372** / issue #1370, which is the better long-term fix.

**Item 11, register the workflows the forks already carry — the documented
command does not work. CORRECTED 2026-09-22.** `handterm` and
`mermaid-rs-renderer` have `ci.yml` in the tree (mermaid also `release.yml`) but
**no registered workflow**, and the strategy's remedy is not available: both

```sh
gh workflow enable ci.yml --repo ianalitis/handterm
gh workflow enable ci.yml --repo ianalitis/mermaid-rs-renderer
```

return `HTTP 404: workflow ci.yml not found on the default branch`. `ci.yml` *is*
on both default branches and `actions/permissions.enabled` is `true` on both, but
an unregistered workflow has no Actions-registry entry to enable, so there is
nothing to target. Registration appears to follow a **processed push event**, and
neither fork has been pushed since Actions was enabled; §3.6's blob-SHA check and
the rollout receipt §4 carry the evidence, including the same-account comparison
with `ianalitis/jcode`, which has been pushed and has all eleven workflows
registered and running.

**The consequence is worse than the 404.** Whatever push registers
`mermaid-rs-renderer`'s `ci.yml` also registers its `release.yml`, which creates a
fork release (`softprops/action-gh-release@v3` under a `contents: write` job) and
runs `cargo publish` on `push: tags v*.*.*`. So a routine mirror push arms the
same class of path item 10 closed, in a second repository, before anyone decided
to run that fork's CI. Any push there must carry a guard on `release.yml` first.
`handterm`'s `ci.yml` is secretless (`grep -c 'secrets\.'` is 0) and safe on its
own terms; `agentgrep` and `GLOOP` carry no workflow files at all, so item 11 does
not apply to them.

**Still open, lower priority:** `dependabot.yml` as an upstream contribution
(not fork-local, per the strategy's placement rule), SHA-pinning actions before
tightening `allowed_actions`, build provenance for release binaries, and a
default-branch ruleset that does not block the documented mirror publish.

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
  can wedge — referenced by absolute path, since it is a fork-line file and absent
  from a worktree at `origin/master`.
- **Validate a `pr/*` branch by opening a fork PR against `fork/master`**, not by
  waiting for a push run. It is the only automatic validation available: upstream
  CI on the upstream PR reports `action_required` for an external fork, and the
  branch push produces no run at all (§3.6, rollout receipt §6b).
- Upstream's CI on our PRs needs maintainer approval (`action_required`) because
  we are an external fork. That is upstream policy, not a defect; Greptile is the
  only check that runs automatically there.

## 6. Corrections to existing records

**Status: both corrections below are already applied** to
`docs/OSS_CICD_ROLLOUT_2026-09-21.md` §6 by the second session, and a third was
added. Do not re-apply them; read that section instead.

- **"`scripts/bounded.sh` does not exist."** It does. It is tracked, present at
  `HEAD` (`git cat-file -e HEAD:scripts/bounded.sh` succeeds, and `ls -la
  scripts/bounded.sh` shows 1471 bytes), introduced by `872f7d148`. It was
  *transiently* absent from the working directory while the concurrent session's
  checkout moved, which was observed more than twice. The real defect is narrower
  and worth keeping: during a branch switch the file can vanish mid-command, and a
  failure hidden behind a pipe makes a skipped fetch look successful. Re-run and
  verify the effect rather than treating `bounded.sh` as unusable; it remains the
  correct bound for anything that can wedge on this host.
- **"`origin/master` was stale by 274 commits."** The ref-relative count was
  wrong, and so was the conclusion drawn from it. After a confirmed
  `git fetch --no-tags origin`, `origin/master...HEAD` is **`0 285`** with
  `origin/master` at `2a4edaa02`; the pre-fetch `0 281` was the correct figure and
  the post-fetch `0 7` belonged to `origin/master...fork/master`. The integration
  line is therefore **285 commits ahead of upstream, not in sync with it** (3
  merges, 282 non-merge). Verify before planning any work that depends on how far
  the line has diverged.

## 7. Definition of done for the next session

For each packet: owned diff inspected, discriminating check passed (with a
negative control where a defect is being fixed), required review completed,
scoped commit recorded in an existing receipt, and the branch pushed only to
`fork`. State source-only verification separately from runtime and from upstream
publication/merge. If blocked, leave a reproducible receipt rather than a hidden
worker or a recursive scheduled task.

## 8. Addendum: what the second session changed after `b3689db26`

Provenance for the extension, so a fresh session knows which claims were measured
when. Commits on `jcode/ci-format-baseline`, oldest first:

| Commit | Change |
| --- | --- |
| `a09441d85` | Labeler gate; strategy document; upstream receipt for the labeler defect |
| `89946934a` | Scope the fork portfolio to forks that exist on the account |
| `b3689db26` | Rollout receipt for the CodeQL and Dependabot work |
| `c53f2bedc` | Correct the CodeQL Rust coverage claim against the final analyses |

Then, in this handoff: §2's ref counts, §3.4's receipt list, §3.5, §4.0, §4.4, and
§6. The rollout receipt's §6 was corrected in the same pass.

**Applied and verified externally, all five public forks:** CodeQL default setup
(`state=configured`), Dependabot alerts (`204`), and the labeler gate published to
the fork default branch as `43a2e7539` and confirmed green
(run `35668523374`: `Check labeling is configured`=success, `label`=skipped). The
push to `fork/master` was a verified fast-forward: `git merge-base --is-ancestor
fork/master HEAD` was checked against a real fetch immediately before the
non-forced push.

**Nothing else in the strategy's §7 has been executed.** The three items a fresh
session should reach for first are §4.0 (CodeQL triage, no approval needed), §4.4
item 10 (disable the fork's unguarded `Release`, one command, highest severity of
the remaining set), and §4.4 item 11 (enable the workflows the forks already
carry, after resolving `mermaid-rs-renderer`'s `release.yml`).

**Concurrency is still live.** During this extension another session committed
`d914b34ed` and `49e8f99b8` to the same checkout. `git commit --only -- <paths>`
held, and every edit here was preceded by a `git status` check. Assume the main
checkout is shared and never switch its branch to do work.

Also live during the third session below, which committed to the same checkout
while this handoff was being written.

## 9. What the third session did, and what it changed

Commits on `jcode/ci-format-baseline`, oldest first. Every one used
`git commit --only -- <paths>` after a `git status` check; the concurrent session
owned other paths throughout.

| Commit | Change |
| --- | --- |
| `38fb68605` | The CodeQL triage receipt (166 alerts, zero confirmed leaks), plus item 10 recorded as applied in the rollout receipt and items 10 and 12 in the strategy gap register |
| `b4b38718e` | Item 11 corrected: `gh workflow enable` returns 404 and the push that would register `ci.yml` also arms `mermaid-rs-renderer`'s publishing `release.yml`; `bounded.sh` absent at `origin/master`; the portfolio table and the "confirm by dispatching" claim |
| `ed55c31ee` | The `pr/session-persist-explicit-state` packet and the three new contributions in `FORK_POSTURE.md`'s packet table |
| `1837eb9e5` | This handoff: §3.6, §4.0 done, §4.1, §4.2, §4.4, §9, the measured ref count, and the `bounded.sh` correction. Also the same-account corroboration in the rollout receipt §4 |
| `00a898620` | The fourth commit in this table |

The last change of the session is the §6b correction in the rollout receipt and
the matching row in the strategy's §3: a `pr/*` branch push gets no fork CI
because upstream's trigger is `push: branches: [main, master]`, and a fork PR
against `fork/master` is the validated remedy (§3.6).

Applied externally: the fork `Release` disable (§4.4), and three upstream issues
and PRs (§3.6). Nothing was merged, no `origin` push happened, no branch or
worktree was deleted, the shared daemon was not promoted, and no release action
was taken.

### Method notes that paid off, worth reusing

- **A blob-SHA comparison, not a file list, decides whether a fix is claimed.**
  `gh pr view --json files` gave thirteen false collisions (§3.6). The 43-PR
  blob check is cheap and decisive.
- **Measure the fix surface, not just the outcome.** Applying two of the three
  clauses and observing that the fourth test still failed with the identical
  error is what established that `improve_mode` was required rather than
  optional. Without that step, the natural move would have been to ship the
  issue's suggested two-clause fix and leave one of its four tests failing.
- **The negative control was the same binary with one hunk reverted**, so the
  guard was the only variable: 4 failures pristine, 0 with the fix. Where a
  change touches a shared guard, also re-run the *wider* suite pristine and
  patched, because that is what separates an unrelated environmental failure from
  a regression (three `jcode-base` session-suite failures fail identically either
  way).
- **A worktree at `origin/master` is not the integration line.** `bounded.sh`,
  and any other fork-line tooling, is absent there; reference it by absolute
  path.
- **Check that a validation step actually ran before trusting it.** "Fork CI runs
  on pr/* branches" was recorded as an established fact from two branches that
  happened to contain the fork's `ci.yml`; for the branches the posture mandates,
  it is false. Three pushes produced one run, and only reading the run list showed
  it (§3.6). The same instinct is why `Require Linked Issue` passing on fork PR #4
  is recorded as a fact rather than assumed.

### What is genuinely left

- **Operator decisions:** whether to dismiss the fork's 166 triaged CodeQL alerts
  (a repository mutation, deliberately not done); the item 8 default-branch
  ruleset design; and whether to register `handterm` and
  `mermaid-rs-renderer`'s workflows at all, given that the registering push would
  also arm mermaid's publishing workflow.
- **Blocked on other people:** #1339's PR #1373, #1354, #1360, #1344, and the
  remaining TUI failures tracked by #1342.
- **Unattempted, and the largest remaining item:** §4.3's `34f2fbd2d`, the
  render-state locking change that unblocks running the suite at default
  parallelism. It is structurally important and needs its own review, not a
  drive-by.
- **Not verified:** whether a push is genuinely what registers a fork's
  workflows (corroborated by a same-account comparison, not proven), and whether
  the upstream guards in #1371 and #1372 are accepted, which is now upstream's
  call.
