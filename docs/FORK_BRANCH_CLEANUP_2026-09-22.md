# Fork branch cleanup: 112 branches on `ianalitis/jcode`

**Date:** 2026-09-22. **Status:** executed the same day on operator approval.
§1-§8 are the inventory and the reasoning as first written; **§10 is what
actually happened**, including the two bugs found in the new tooling before it
was ever allowed to delete anything. No repository setting was changed and no
other ref was pushed.

**Why this exists:** the fork carries 112 branches and only 14 of them are
live work. This document records what each one is, which are provably safe to
delete, which hold unique unmerged content, and the posture that stops this
from recurring.


## 1. Why the fork has 112 branches

A fork is upstream's tree plus whatever we push. Non-default branches are not
copied by GitHub, so every one of the 112 was pushed by us, and the mechanism
is mechanical rather than accidental:

- Every agent session pushes its work branch to `fork` so fork CI can run it
  (a `pr/*` branch pushed to `fork` still gets no upstream CI, which is why
  fork-internal PRs exist as well: #3 and #4).
- Nothing ever deletes them. `delete_branch_on_merge` is `false` on the fork,
  and upstream's merge cannot delete a branch in our fork even when it is on.
- The oldest branch is from 2026-02-12 and the newest from 2026-09-21, so this
  has been accumulating for seven months across many sessions.
- One sibling fork adds its own: `mermaid-rs-renderer` has 13 `dependabot/*`
  branches because upstream carries a `.github/dependabot.yml` that the fork
  inherits, and Dependabot opens version-update PRs against the fork that
  cannot be merged without diverging the mirror (§7).


## 2. Method, and the one tool change it needed

`scripts/branch_ledger.sh` already classifies local branches against a base
with the dispositions `integrated`, `relanded`, `merge-ready`, `conflicts`,
`cherry` and `stale`. It only iterated `refs/heads/`, so this task added a
`--refs` mode (and fixed its `--exclude` of the per-remote `HEAD` symref, whose
short name is a bare remote name rather than `<remote>/HEAD`):

```sh
# against upstream: is this content already upstream?
scripts/branch_ledger.sh --refs refs/remotes/fork/ --base origin/master
# against our integration line: has the fork line already absorbed it?
scripts/branch_ledger.sh --refs refs/remotes/fork/ --base HEAD
```

A branch is treated as **provably safe** only when at least one of those two
runs calls it `integrated` (zero unique commits by patch content) or
`relanded` (every unique commit's subject already exists in that base).
Everything else is called unique here, which is the conservative direction.


## 3. Census

| | count |
| --- | --- |
| branches on `fork` | 112 |
| keep (§4) | 14 |
| provably safe to delete (§5) | 65 |
| unique unmerged content, needs a decision (§6) | 33 |


## 4. Keep

| branch | why |
| --- | --- |
| `master` | the mirror; `git diff origin/master fork/master` is the sync check |
| `jcode/ci-format-baseline` | the integration line the fork publishes |

| `pr/app-core-unused-imports` | head of open upstream PR #1364 |

| `pr/background-status-atomic-writes` | head of open upstream PR #1357 |

| `pr/base-suite-env-guard` | head of open upstream PR #1360 |

| `pr/clippy-1.98-lint-drift` | head of open upstream PR #1354 |

| `pr/freebsd-smoke-permissions` | head of open upstream PR #1371 |

| `pr/jev-mock-server-nonblocking-read` | head of open upstream PR #1355 |

| `pr/release-repository-guard` | head of open upstream PR #1372 |

| `pr/session-persist-explicit-state` | head of open upstream PR #1373 |

| `pr/swarm-stop-quiesce` | head of open upstream PR #1362 |

| `pr/test-env-lock-bounded-wait` | head of open upstream PR #1356 |

| `pr/tui-account-labels` | head of open upstream PR #1368 |

| `pr/tui-lib-test-failures` | head of open upstream PR #1366 |


## 5. Provably safe to delete (65)

Each row was `integrated` or `relanded` against `origin/master`, against the
integration line, or both. Unique/total is *unique-by-patch / commits since the
merge base*.

| branch | unique/total vs upstream | vs the line | age | tip |
| --- | --- | --- | --- | --- |
| `add-fpt-ai-marketplace-provider` | 0/1 | 0/1 | 137d | `271546419` |
| `agent/fix-739-742` | 0/0 | 0/0 | 49d | `f044816bf` |
| `agent/issue-699-ctrl-d` | 0/0 | 0/0 | 51d | `02f4afc00` |
| `agent/jcode-triage-20260814-2203` | 0/0 | 0/0 | 38d | `3309ad37b` |
| `agent/sdk-release-followup` | 2/2 | 2/2 | 45d | `2f534cf14` |
| `agent/triage-2026-08-02-clean` | 0/0 | 0/0 | 50d | `30de2a2a1` |
| `agent/triage-2026-08-06` | 0/0 | 0/0 | 46d | `09d328b1c` |
| `agent/triage-2026-08-07` | 12/12 | 12/12 | 45d | `9445e7211` |
| `agent/triage-2026-08-14` | 0/0 | 0/0 | 38d | `5ffba9482` |
| `agent/triage-2026-08-24` | 0/0 | 0/0 | 27d | `3f199fa0f` |
| `agent/triage-20260816` | 0/0 | 0/0 | 36d | `d790cf279` |
| `agent/triage-20260818` | 0/0 | 0/0 | 31d | `3d993cc74` |
| `agent/triage-fixes-20260728` | 0/0 | 0/0 | 55d | `111ee842d` |
| `agent/triage-open-issues-20260809` | 0/0 | 0/0 | 43d | `44ffa5528` |
| `agent/triage-safe-fixes-20260809` | 0/0 | 0/0 | 43d | `7522663b1` |
| `agent/triage-safe-fixes-20260827` | 0/0 | 0/0 | 24d | `e24dd976c` |
| `agent/triage-verification-tests` | 0/0 | 0/0 | 50d | `35e692bc8` |
| `arch/app-decomp` | 3/5 | 3/5 | 109d | `7634a9ed8` |
| `arch/integration` | 6/11 | 6/11 | 109d | `46929ad08` |
| `arch/server-svc` | 2/2 | 2/2 | 109d | `8bb66ae13` |
| `chick/compacted-history-visible-window` | 0/1 | 0/1 | 121d | `c37dccf34` |
| `deps/agentgrep-v0.1.7` | 1/1 | 0/1 | 17d | `35a3ad6e3` |
| `feat/storage-status` | 2/2 | 2/2 | 17d | `5ca058ebf` |
| `fix-set-route-model-alias` | 2/11 | 2/11 | 102d | `1252aaf2b` |
| `fix/acp-mcpservers-tolerated` | 0/0 | 0/0 | 41d | `0517f01f2` |
| `fix/herdr-client-hooks` | 0/0 | 0/0 | 49d | `3c5751474` |
| `fix/installer-path-idempotency` | 0/1 | 0/1 | 56d | `c111c10e3` |
| `fix/issue-543-mcp-format` | 0/3 | 0/3 | 61d | `a52ead259` |
| `fix/issue-657-tract-023` | 0/0 | 0/0 | 54d | `49d9dfd62` |
| `fix/issue-662-ci-red` | 0/0 | 0/0 | 54d | `c3455d951` |
| `fix/issue-754-gemini-mcp-notification` | 0/0 | 0/0 | 48d | `3625a0230` |
| `fix/issue-759-client-hooks` | 0/0 | 0/0 | 48d | `c943b04e8` |
| `fix/issue-762-celeris-config` | 0/0 | 0/0 | 48d | `d7feb6b54` |
| `fix/issue-763-power-inhibitor` | 0/0 | 0/0 | 48d | `dc59f6521` |
| `fix/issue-767-favorite-cycle` | 0/0 | 0/0 | 48d | `0251e0803` |
| `fix/issue-768-menubar-ci` | 0/0 | 0/0 | 48d | `1e80a4033` |
| `fix/issue-779-acp-resume-subscribe` | 0/0 | 0/0 | 48d | `8d46f643c` |
| `fix/latest-remote-session-bugs` | 0/0 | 0/0 | 42d | `fefae76c2` |
| `fix/macos-stdin-false-positive` | 1/1 | 1/1 | 20d | `04d8bb30d` |
| `fix/menubar-dark-streaming-icon` | 1/1 | 0/1 | 20d | `6b9a34000` |
| `fix/menubar-root-sessions` | 3/5 | 3/5 | 65d | `aac9555e8` |
| `fix/openai-quota-window-dedup` | 0/0 | 0/0 | 41d | `30ec7b337` |
| `fix/pinned-todos-config-cache-isolation` | 0/0 | 0/0 | 42d | `446601bc3` |
| `fix/report-alternate-keyboard-keys` | 0/0 | 0/0 | 42d | `a1d7db454` |
| `fix/skill-invocation-multi-word-619` | 0/1 | 0/1 | 56d | `477cb86c6` |
| `fix/soft-interrupt-images` | 0/1 | 0/1 | 56d | `9ea612f9d` |
| `fix/stream-first-byte-timeout` | 0/1 | 0/1 | 56d | `2b0e28b35` |
| `fix/stream-read-error-retry` | 0/0 | 0/0 | 40d | `437c6610a` |
| `fix/test-git-probe-isolation` | 1/1 | 0/1 | 20d | `01d60e5aa` |
| `fix/tui-suite-deadlock` | 1/1 | 1/1 | 20d | `bff4f2d94` |
| `fix/unique-fallback-tool-call-ids` | 0/0 | 0/0 | 41d | `e0aea418e` |
| `ios/ux-production` | 8/9 | 8/9 | 60d | `887194bdf` |
| `jcode/adopted-output-artifact` | 1/1 | 0/1 | 13d | `cba16813a` |
| `jcode/configurable-colors` | 4/24 | 4/24 | 54d | `53c5c0a2f` |
| `jcode/fix-macos-terminal-tail-match` | 1/2 | 0/2 | 4d | `5d501a7cb` |
| `jcode/fix-wake-mode-fingerprint` | 1/2 | 0/2 | 4d | `510f7c3ff` |
| `jcode/focus-report-loop` | 1/1 | 0/1 | 14d | `108caabd2` |
| `jcode/notification-child-reaping` | 1/1 | 0/1 | 14d | `535a479d1` |
| `logical-commits-20260322` | 3/3 | 3/3 | 183d | `2e89a6b45` |
| `perf/cache-and-journal` | 2/2 | 1/2 | 17d | `12dc573b9` |
| `perf/memory-off-agentgrep-v017` | 2/2 | 0/2 | 19d | `2215f0aa4` |
| `security/project-mcp-trust` | 1/1 | 1/1 | 17d | `07fc5a0ab` |
| `test/preserve-iteration-maturity-fixtures` | 0/0 | 0/0 | 49d | `9fc15d499` |
| `triage/issues-2026-07-29` | 0/0 | 0/0 | 54d | `fe77ce9c7` |
| `triage/open-issues-20260812` | 0/0 | 0/0 | 40d | `6fa94a882` |

## 6. Unique unmerged content (33)

Not upstream, not in the integration line, no PR. Age is the tip commit's age;
**nothing here is proposed for deletion without a decision**. The tips are
recorded so any deletion is recoverable locally.

| branch | unique/total vs upstream | vs the line | age | tip |
| --- | --- | --- | --- | --- |
| `feat/computer-observability-348` | 1/1 | 1/1 | 104d | `941b44b28` |
| `fix/transport-retry-classification` | 2/2 | 2/2 | 105d | `664c04142` |
| `jcode/dev-cargo-cwd-guard` | 1/1 | 1/1 | 12d | `d3f3c6c45` |
| `fix-browser-setup-201` | 1/3 | 1/3 | 130d | `f5d363095` |
| `jcode/ci-env-dedup` | 2/2 | 2/2 | 13d | `091618754` |
| `windows-lifecycle-e2e` | 2/3 | 2/3 | 147d | `63c77688f` |
| `jcode/workflow-shellcheck` | 1/1 | 1/1 | 14d | `98cee29c7` |
| `docs/release-base-clarity` | 1/1 | 1/1 | 160d | `f1e9af276` |
| `fix/compaction-token-accounting` | 1/1 | 1/1 | 17d | `e79659889` |
| `fix/anthropic-fable-history` | 5/5 | 4/5 | 17d | `0d16b4aa2` |
| `security/harden-network-and-approval-boundaries` | 6/6 | 4/6 | 17d | `64e764f67` |
| `fix/provider-cli-routing` | 6/6 | 5/6 | 17d | `3481e810a` |
| `perf/kv-cache-telemetry-single-pass` | 33/33 | 27/33 | 17d | `a0f77ccd4` |
| `chore/macos-warning-clean` | 3/3 | 3/3 | 17d | `2e09d2097` |
| `backup/kv-cache-telemetry-single-pass-20260904` | 34/34 | 30/34 | 17d | `4560d117b` |
| `release-workflow-fixes` | 1/2 | 1/2 | 182d | `6dd45bf5d` |
| `fix/prompt-overlay-home-dedupe` | 1/1 | 1/1 | 20d | `c7e5d46d9` |
| `fix/sandboxed-home-keychain` | 1/1 | 1/1 | 20d | `91b492629` |
| `fix/test-ambient-queue-isolation` | 1/1 | 1/1 | 20d | `cfb8a7504` |
| `fix/macos-ctrl5-prompt-rank` | 1/1 | 1/1 | 20d | `7ad6bab90` |
| `fix/persist-session-with-title` | 1/1 | 1/1 | 20d | `89c13f617` |
| `dioxus-gui` | 18/364 | 18/364 | 221d | `7abad06cf` |
| `dioxus-gui-local` | 20/382 | 20/382 | 221d | `e914f97ab` |
| `jcode/fix-gemini-individual-oauth-status` | 3/3 | 3/3 | 22d | `c8b2ec774` |
| `agent/triage-safe-fixes-20260813` | 1/7 | 1/7 | 36d | `5c8b207cb` |
| `jcode/fork-ci-secretless` | 6/6 | 6/6 | 3d | `32961d965` |
| `agent/release-v0.71.1` | 20/21 | 20/21 | 44d | `431ee1219` |
| `ios/mobile-real-nav` | 21/22 | 21/22 | 44d | `673ff4f3e` |
| `agent/triage-2026-08-07-pr` | 9/11 | 9/11 | 45d | `47e8819ce` |
| `feat/issue-664-auto-poke-config` | 4/9 | 4/9 | 54d | `8acc3081b` |
| `fix/windows-global-jcode-path` | 4/4 | 4/4 | 67d | `0bb4d1084` |
| `feat/windows-setup-copilot-key` | 27/32 | 27/32 | 67d | `526af67e0` |
| `fix/computer-tool-schema-and-element-at` | 8/8 | 8/8 | 88d | `1677d567a` |

The four with real feature mass, if any of them should be contributed rather
than archived: `dioxus-gui` and `dioxus-gui-local` (18 and 20 commits, 221 days
old, a desktop GUI that upstream has since rebuilt), `ios/mobile-real-nav` (21
commits), `feat/windows-setup-copilot-key` (27 commits) and the paired
`perf/kv-cache-telemetry-single-pass` / `backup/kv-cache-telemetry-single-pass-20260904`
(33 and 34 commits, one of them explicitly a backup copy).


## 7. The other forks

| fork | branches | what they are |
| --- | --- | --- |
| `mermaid-rs-renderer` | 25 | 13 `dependabot/*` version-update branches plus 12 agent branches (`cursor/*`, `claude/*`, `layout-overhaul`, `parity`, `functional-parity`, ...) |
| `agentgrep` | 4 | `main`, `master`, `fix/macos-nonutf8-test-skip`, `perf/max-count-pushdown` |
| `GLOOP` | 2 | `main`, `gloop/contract-only` |
| `handterm` | 1 | `master` only, already the posture this document wants |


The `dependabot/*` branches are the concrete case the CI/CD strategy predicted:
a fork inherits upstream's `.github/dependabot.yml`, Dependabot opens version
updates against the fork's default branch, and none of them can be merged
without diverging the mirror. Security *updates* are already off on all four
(`disabled`), so only version updates are producing these.


## 8. Posture, so this does not recur

1. **A branch on the fork is either live work or it should not be there.** The
   fork's ref list should be `master`, the integration line, and the heads of
   open PRs. That is 14 today.
2. **Delete the head branch when a PR closes**, whichever way it closes: merged,
   superseded or abandoned. Upstream's merge cannot do it for us.
3. **The audit is one command**, and it belongs in a closeout or the start of a
   contribution task: `scripts/branch_ledger.sh --refs refs/remotes/fork/ --base origin/master`.
   It is read-only.
4. **History lives locally, not on the fork.** Work that is not live goes to the
   integration line if it belongs to the product, to an upstream PR if it belongs
   to upstream, and to a local archive ref otherwise. The fork is the shop window,
   not the attic.
5. **Dependabot version updates have no place on a mirror fork.** The fork's own
   `.github/dependabot.yml` cannot be deleted without diverging the mirror, so the
   remedy is the repository setting, and failing that, pruning the `dependabot/*`
   branches on the same schedule as everything else. Alerts stay on: they are the
   visibility that made the 2026-09-22 dependency bumps possible.
6. **Protect only what must not move.** The fork already has an active ruleset on
   the default branch (`protect default branch`, id 23795856); the documented
   mirror publish has to stay a fast-forward, and that ruleset has not blocked it.
7. **Local hygiene is the same problem.** This checkout has 36 local branches and
   15 worktrees; `scripts/branch_ledger.sh` with no arguments dispositions those.
   Pruning them is a separate approved pass.


## 9. The actions, as proposed and then approved

Approved and run on 2026-09-22; results are in §10.

1. **Archive, then delete the 65 safe branches.** The archive step keeps every
   tip reachable in this clone (`refs/archive/fork-2026-09-22/<name>`), and the
   SHA table in §5 is the durable record if the clone is ever lost:

   ```sh
   scripts/fork_branch_prune.sh --tier safe --archive --dry-run   # review
   scripts/fork_branch_prune.sh --tier safe --archive             # apply
   ```

2. **Decide §6.** Either archive-and-delete all 33 with the same script
   (`--tier unique --archive`), or name the ones to keep and delete the rest.

3. **Sibling forks:** delete `mermaid-rs-renderer`'s 13 `dependabot/*` branches
   and its finished agent branches, `agentgrep`'s two work branches, and
   `GLOOP`'s `gloop/contract-only` if it is finished. `handterm` needs nothing.

4. **If the 13 Dependabot branches should stop appearing**, turn version updates
   off for `mermaid-rs-renderer` in the repository settings (there is no
   documented REST endpoint for that toggle, so it is a UI action).

5. **Local pass:** disposition the 36 local branches and 15 worktrees, then prune
   with the same evidence standard.


## 10. Executed, with results

Every deletion was run by `scripts/fork_branch_prune.sh` with `--archive
--apply`, so each tip is still reachable in this clone as
`refs/archive/<remote>-2026-09-22/<branch>`. Nothing was force-pushed, and no
branch that any open PR points at was touched: the tool refuses those by reading
the API at run time.

| repository | before | after | archived | kept | command |
| --- | --- | --- | --- | --- | --- |
| `ianalitis/jcode` | 112 | **14** | 98 | `master`, the integration line, 12 open PR heads | `--tier safe --archive --apply`, then `--tier unique --archive --apply` |
| `ianalitis/mermaid-rs-renderer` | 25 | **2** | 23 | `master`, the head of upstream PR #147 | `--upstream 1jehuang/mermaid-rs-renderer --line '' --tier safe`, then `--tier unique`, both `--archive --apply` |
| `ianalitis/agentgrep` | 4 | **3** | 1 | `master`, the heads of upstream PRs #6 and #7 | `--upstream 1jehuang/agentgrep --line '' --tier safe --archive --apply` |
| `ianalitis/GLOOP` | 2 | 2 | 0 | unchanged | none |
| `ianalitis/handterm` | 1 | 1 | 0 | already the target posture | none |

Total: 144 branches to 22, 122 archived, 0 failed deletions.

Verified after the jcode run: all twelve open PR heads still resolve on the fork,
the fork's branch list is exactly the keep set of §4, and every deleted tip
resolves through its archive ref. Any of them restores with:

```sh
git push fork refs/archive/fork-2026-09-22/<branch>:refs/heads/<branch>
```

`GLOOP` was left alone deliberately. Its `main` is byte-identical to
`redacktion/GLOOP`'s `main`, and its one other branch, `gloop/contract-only`,
holds a single 33-line documentation commit from 2026-09-15 (`core/GLOOP.md`
plus one line of `adapters/generic/AGENTS.md`) that is aimed at the parent, not
at the fork. Whether that fork is maintained at all is still the open decision
recorded in the CI/CD strategy, and deleting the only work branch before it is
answered would buy nothing.

### Two bugs the tooling grew, and caught, before it deleted anything

1. `git log --grep` exits 0 when it matches nothing, so an exit-status test for
   "this subject is already in the base" called every branch relanded. The first
   dry run reported 98 safe and 0 unique; testing the captured output instead
   gave the real 65 and 33. Under `--apply` that bug would have deleted 33
   branches holding unique work in one pass.
2. The integration line is optional, but the classification tested
   `u_line == 0 || relanded == u_line` unconditionally. On a repo with no such
   line both values are 0, so the clause was trivially true and every branch
   looked integrated. Found while preparing the sibling forks, which have no
   integration line, and fixed before the first sibling run.

Both would have been invisible without a dry-run default and an explicit
`--apply`. The guard that refuses open PR heads is what kept agentgrep's two
live branches and mermaid's one; they were never candidates.
