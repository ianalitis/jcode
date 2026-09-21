# Branch, worktree and contribution cleanup ledger

Updated 2026-09-21 after the operator approved both cleanup batches at 17:54 UTC.
Batch A executed at 17:58 UTC and batch B at 17:59 UTC. The PR #1357 follow-up
remains published as `245c44b4d`. This is the existing canonical ledger refreshed,
not a new tracker. The exact completed actions and retained work are below.

## Current inventory

- 27 local branches, down from 33, including the active integration branch omitted
  by the script.
- 6 worktrees, down from 7, with 0 stashes and no detached worktrees.
- All six worktrees verified clean, including untracked files, before this
  documentation update. The duplicate file in batch A is gone.
- 5 open upstream PRs and 25 open upstream issues authored by `ianalitis`.
  13 authored issues and 2 authored PRs are closed. No authored PR was merged.
- The personal fork has no open PRs and its issues feature is disabled.
- `fork/master` is 0 behind / 3 history-only commits ahead, with an exact upstream
  source tree. Keep the mirror and active contribution branches distinct.

The table below is verbatim output of
`scripts/bounded.sh 240 bash scripts/branch_ledger.sh --base 851ff2c8c --timeout 8`.
Its `TIMEOUT` rows are **unresolved dry runs, not demonstrated conflicts**, despite
the script's `conflicts` disposition. A large `git cherry` count is not proof of
missing behavior after our test splits and manual ports. Do not merge old grab-bag
branches or retire them from subject matching alone.

## Generated snapshot

### Branch ledger: base 851ff2c8c (851ff2c8c), 2026-09-21T17:59:18Z

| disposition | branch | unique/total | relanded | behind | merge | files | age | worktree |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| cherry | `backup/kv-cache-pre-rebase-v0.81.7` | 53/59 | 5 | 2431 | large |  | 15d |  |
| cherry | `backup/kv-cache-pre-rebase-v0.83.0` | 60/66 | 5 | 2421 | large |  | 14d |  |
| cherry | `backup/kv-cache-telemetry-single-pass-20260904` | 30/34 | 4 | 2455 | large |  | 17d |  |
| cherry | `chore/macos-warning-clean` | 3/3 | 0 | 2431 | large |  | 17d |  |
| cherry | `deps/resvg-usvg-align` | 2/3 | 0 | 2307 | large |  | 11d |  |
| cherry | `fix/anthropic-fable-history` | 4/5 | 0 | 2431 | large |  | 17d |  |
| cherry | `fix/ci-gate-integrity` | 30/36 | 5 | 2431 | large |  | 16d |  |
| cherry | `fix/compaction-token-accounting` | 1/1 | 0 | 2431 | large |  | 17d |  |
| cherry | `fix/provider-cli-routing` | 5/6 | 0 | 2431 | large |  | 17d |  |
| cherry | `jcode/auth-preserve-billing-route` | 6/6 | 0 | 2274 | large |  | 6d |  |
| cherry | `jcode/data-class-admission` | 9/9 | 0 | 2209 | large |  | 1d | /Users/ianalitis/.jcode/source/worktrees/data-class-admission |
| cherry | `jcode/dev-cargo-cwd-guard` | 29/30 | 1 | 2307 | large |  | 11d |  |
| cherry | `jcode/fix-fresh-improve-bootstrap` | 132/136 | 3 | 2456 | large |  | 19d |  |
| cherry | `jcode/fix-gemini-individual-oauth-status` | 4/4 | 0 | 2457 | large |  | 21d |  |
| cherry | `jcode/focus-runtime-safe` | 67/73 | 5 | 2329 | large |  | 11d | /Users/ianalitis/.jcode/source/worktrees/focus-runtime-safe |
| relanded | `jcode/fork-mirror-sync` | 2/3 | 2 | 262 | n/a |  | 0d |  |
| cherry | `jcode/privacy-no-collection` | 1/1 | 0 | 2200 | large |  | 1d | /Users/ianalitis/.jcode/source/worktrees/privacy-no-collection |
| cherry | `jcode/route-receipts` | 19/21 | 0 | 2308 | large |  | 13d | /Users/ianalitis/.jcode/source/worktrees/route-receipts |
| integrated | `master` | 0/0 | 0 | 262 | n/a |  | 0d |  |
| cherry | `perf/kv-cache-telemetry-single-pass` | 84/91 | 7 | 2329 | large |  | 13d |  |
| conflicts | `pr/background-status-atomic-writes` | 3/3 | 2 | 263 | CONFLICT(1) |  | 0d |  |
| conflicts | `pr/base-suite-env-guard` | 3/10 | 0 | 262 | TIMEOUT |  | 0d | /Users/ianalitis/.jcode/source/worktrees/base-suite-env-guard |
| conflicts | `pr/clippy-1.98-lint-drift` | 1/1 | 0 | 263 | TIMEOUT |  | 0d |  |
| conflicts | `pr/jev-mock-server-nonblocking-read` | 1/1 | 0 | 263 | TIMEOUT |  | 0d |  |
| conflicts | `pr/test-env-lock-bounded-wait` | 2/2 | 0 | 263 | TIMEOUT |  | 0d |  |
| cherry | `security/harden-network-and-approval-boundaries` | 4/6 | 1 | 2431 | large |  | 17d |  |

### Stashes


### Detached worktrees

## Approval batch A: local retirement only

**Executed with approval.** Deleted exactly six local branch names, one clean
integrated worktree, and one duplicate untracked file. Every branch tip, worktree
status (including ignored files), and the duplicate's hash/prefix identity were
rechecked immediately before removal. No remote refs, canonical source files,
other worktrees, or plans were deleted. Five branch behaviors were checked in
current source and their named regressions each passed on macOS during the audit.

| Deleted local branch | Recorded tip | Evidence retained in integration |
| --- | --- | --- |
| `attempt/billing-route` | `bb252f2b8d343c0fa8919bf90ba098d48043b136` | Exact ancestor of `3f5296d78`, no unique commits |
| `fix/macos-ctrl5-prompt-rank` | `7ad6bab90df7ebc49920f81460dbb13eb0d5bae4` | `test_ctrl_5_is_a_prompt_rank_jump_on_macos`: 1 passed |
| `fix/persist-session-with-title` | `89c13f6172bb2db7b68da800f51fc5b65f325bb4` | `session_created_with_title_is_persisted_before_first_visible_message`: 1 passed |
| `fix/prompt-overlay-home-dedupe` | `c7e5d46d916aa5a810b59c5ca66127f0777fdde7` | `overlay_and_preferred_tools_are_not_doubled_when_cwd_is_the_home_dir`: 1 passed |
| `fix/sandboxed-home-keychain` | `91b492629a4d8f8e9b4cd8f2bf06159decad0b75` | Native credential discovery checks sandboxed home; `sandboxed_jcode_home_is_detected_without_hiding_explicit_env_credentials`: 1 passed |
| `fix/test-ambient-queue-isolation` | `cfb8a7504f60f42c26d259df2d9aabbfd4b1513a` | Cleanup is constrained to the shared test home; `gather_ambient_info_filters_to_session_reminders_when_ambient_disabled`: 1 passed |

Exact completed filesystem actions:

1. Removed worktree `/Users/ianalitis/.jcode/source/worktrees/attempt-billing-route`
   without force before deleting its local branch. It had no dirty, untracked or
   ignored files immediately before removal and used about 207 MiB at audit.
   Its tip remains an ancestor of integration.
2. Removed **only** the untracked file
   `/Users/ianalitis/.jcode/source/worktrees/data-class-admission/crates/jcode-harness-api-server/src/translate_regression_tests.rs`.
   It was 3298 bytes, SHA-256
   `f6cdc6765d39395d64ed8d8a532129e2e1308edb981888f59a1913f5f9179d5b`,
   and a byte-identical prefix of the tracked canonical file in integration.
   That worktree and its branch remain: they contain other unreviewed work.

Do not force-remove a worktree if its contents have changed. Local deletion does
not authorize deleting matching remote contribution refs. Historical tips above
remain useful provenance even where the accepted implementation was rewritten.

## Approval batch B: public contribution cleanup

**Executed with approval.** Closed these two superseded PRs without merging them,
after rechecking their exact approved heads. Their remote branches were retained:

- [PR #1293](https://github.com/1jehuang/jcode/pull/1293), head `510f7c3ff`:
  the `JCODE_WAKE_MODE` fingerprint entry is already on upstream `2a4edaa02`,
  introduced by ancestor `e7a22f695`. Its currently advertised one-line fix has a
  1242-file GitHub diff (+170560 / -91275). Do not reland the integration history.
- [PR #1295](https://github.com/1jehuang/jcode/pull/1295), head `5d501a7cb`:
  focused open [PR #1354](https://github.com/1jehuang/jcode/pull/1354) already
  includes the same terminal `needless_return` fix and links #1294. The old PR's
  GitHub diff is also 1242 files (+170559 / -91275). Keep #1294 open until its
  replacement PR is merged.

Closed [issue #1292](https://github.com/1jehuang/jcode/issues/1292) as fixed upstream
by `e7a22f695`, with a precise verification caveat: the unchanged upstream
`config_env_fingerprint_tracks_every_apply_env_override_var` test still fails,
but its **only** missing key is `JCODE_MEMORY_JEV_PROVIDER`, not `JCODE_WAKE_MODE`.
That independent remaining defect belongs to #1358 / PR #1360. No test was
weakened or amended to obtain a green claim.

The approved linkage check found that
[PR #1360](https://github.com/1jehuang/jcode/pull/1360)'s existing description
already starts with `Fixes #1358.`. This is equivalent closing linkage, so its
body was preserved unchanged rather than appending redundant `Closes #1358` text.
Its approved head remains `9618d3e9545f1e8f173023ba560f4cd96c44b67d`.
Independent post-action reads confirmed both PRs closed without merge, #1292
closed as completed, and #1294/#1358 still open. All five current PRs remain open.

## Retain and investigate, not clutter to discard

The reader inventories are not security acceptance reviews. Function-name overlap
is a search heuristic, not a proof of behavior. These remain unmerged and retained:

| Branch or work | Evidence-backed reason to keep it | Next bounded question |
| --- | --- | --- |
| `jcode/auth-preserve-billing-route` | Prior Sol review narrowed the interesting remainder to `387828e79`; the paid-request CLI proposal was rejected for caller-minted admissions and uncapped aggregate spend | Reproduce whether credential refresh can change the active billing route, then review only that path |
| `security/harden-network-and-approval-boundaries` | Webfetch protections overlap current code, but single-use approval and discovery-egress hunks need semantic review | One security packet per behavior, not a wholesale merge |
| `jcode/privacy-no-collection` | Core removals and `no_collection.rs` are represented; installer/discovery opt-out remainders remain unverified | Compare and test only residual installer/egress behavior |
| `jcode/data-class-admission` | Competing per-root and current per-path designs; proposed spend/routing pieces are not admission authority | Decide one design before any port |
| `jcode/route-receipts` | Old test splits mostly overlap current structure; receipt feature commit `3e0ec3dfe` needs separate review | Determine whether current attempt receipts already satisfy its concrete need |
| `jcode/focus-runtime-safe`, `perf/kv-cache-telemetry-single-pass`, `fix/ci-gate-integrity`, `jcode/fix-fresh-improve-bootstrap`, `backup/*` | Large grab-bag histories with overlapping patches | De-duplicate concrete missing behaviors before admitting a small packet |
| `fix/compaction-token-accounting` | Token/drop forwarding is relanded; this iteration restores the missing unknown-token regression. Only the old raw-trigger forwarding proposal remains unaccepted | Preserve the existing wire vocabulary; branch retirement would explicitly discard that proposal and still needs approval |
| `deps/resvg-usvg-align` | Renderer alignment and Cargo cwd guard are verified present. The unique 565-line efficiency/Pi proposal was reviewed as historical, not adopted | Keep the branch until the operator decides whether to archive or discard the proposal |
| `chore/macos-warning-clean` | All 24 paths reviewed. macOS cleanup is present or superseded; three Windows tail-return hunks remain unverified on their target | Retain pending a Windows lint check, not a blind port of the old branch |
| `fix/anthropic-fable-history`, `fix/provider-cli-routing`, `jcode/fix-gemini-individual-oauth-status` | Remaining commit-level behavior not fully reviewed in this pass | Preserve, do not delete based on age or provider availability |
| Active `pr/*` branches and the base-suite worktree | Live upstream contributions, with intentional old-layout ports | Keep until upstream disposition and fresh patch-equivalence checks |
| `jcode/fork-mirror-sync`, `master` | Mirror workflow roles, not abandoned feature work | Preserve and use explicit upstream tree/ancestry checks |

Five current PRs (#1354, #1355, #1356, #1357, #1360) remain open. Pending
maintainer review is not local dirt. Do not close their issues merely because
our fork carries the fix. #1352's queued-worker cancellation packet still lacks
a contribution, and older design/reproduction issues remain open unless there
is positive evidence that their exact reported defect is resolved.

## Plans and prior records

The audit counted 17 top-level plan files, 101 top-level documentation files,
15 handoff-named documents, and 6 upstream-feedback packets. Categories overlap.
Completed Phase 1/2 packets now point to their implementation receipts. Phase 3
is clearly 'published, merge pending', not a new env-guard assignment. Phase 4
and unattended Pi/self-compaction remain unfinished. Historical handoffs are
retained with forward pointers rather than deleted. The current docs index and
fork posture are the navigation points, not another top-level plan.

This refresh supersedes the earlier `07eaf3320` ledger snapshot. Its six listed
stashes and detached worktree were historical: none exists now. Earlier merge,
port and cleanup records remain in Git history, not as commands to replay.
No new claims of runtime deployment, unattended-worker safety, or full old-branch
integration follow from this cleanup audit.

Scratch evidence: `lean-branch-ledger.md`, `lean-worktree-inventory.json`,
`lean-plans-audit.md`, `lean-github-audit.md`, `lean-branch-remainders.md`,
`lean-upstream-wake-fingerprint.log`, and the five `lean-<test-selector>.log`
files under `~/.jcode/scratch/`. Captain verified the retirement tests, dirty-file
identity, upstream source/ancestry, and the two public PR diff sizes independently.

Execution evidence in the same scratch directory: `approved-cleanup-a.json`,
`approved-cleanup-b.json`, `approved-cleanup-inventory.json`, and the regenerated
`approved-cleanup-ledger.md`. The bounded ledger run completed in 94 seconds.
Its temporary `.merge_file_*` files appeared during dry runs and were removed by
the script on completion. Final cleanliness was checked after completion, not
during those transient writes. No daemon reload or additional push occurred.

## Follow-up iteration: small branches and test-harness correctness

The operator requested continued iteration after cleanup commit `c290654c3`.
No further refs, worktrees, files, issues or PRs were deleted or closed.
The generated snapshot above remains pinned to its named base, not relabeled as
a fresh ancestry calculation after these source-test changes.

### Compaction branch: production fix present, missing coverage restored

Reviewed `fix/compaction-token-accounting` at
`e79659889eb852715bf14c2c1774d200c6c42512`. Production token-count and drop-count
repairs are already represented by ancestor `ffe90c42b`; the hard-drop regression
was previously relanded as `043ee50f3`. Upstream issue #1178 is closed.

The real OpenAI producer sends `pre_tokens: None`, but the existing native fixture
only tested `Some(80_000)`. Parameterizing that fixture and restoring
`native_compaction_does_not_relabel_response_usage_as_pre_compaction_usage`
closes that gap in `c4749c964`. All seven compaction tests pass. A temporary fallback to response
usage made the new test fail with `Some(24000)` versus `None`; the production file
was restored byte-for-byte and the suite passed again. No production repair was
needed and no test was disabled.

The old branch also forwards `openai_native_auto` rather than the existing remote
`openai_native` label. Both map to automatic wording and identical cache reset in
current in-repo consumers. There is no demonstrated behavior bug requiring a
client-visible vocabulary change. That old proposal is not silently accepted.

### Dependency branch: code represented, historical proposal retained

Reviewed `deps/resvg-usvg-align` at
`25526f21b8673cc7398129fecaba598680637c59`. All eight formerly duplicated renderer
packages have one lockfile version. Productivity tests pass 5/5 and Mermaid tests
64/64. Existing `scripts/test_dev_cargo_cwd.py` passes 5/5, including foreign
projects, inherited shell shims, symlink paths and error propagation. The wrapper
fix is already represented by `a86dcfe46` and subsequent upstream work.

The branch-only `docs/proposals/EFFICIENCY_AND_PI_LOADOUTS.md` is a historical
2026-09-09 assessment, not a current implementation order. Its old embedding
default claim is stale, and its automatic fallback / trusted Pi loadout proposals
do not override current admission and lifecycle policy. No benchmark numbers in
it were remeasured in this iteration. Keep the original on the retained branch:
blob `2d25af4b7e82e047a1ef60296f8de3b9715daa6d`, SHA-256
`70a51b210b013e7de8e08f50d4e9d2887de77e06162c411ce6c9d160f56b6ce0`.
Do not copy 565 lines of superseded recommendations into the active plan.

### Warning branch: retain cross-platform remainder, avoid regressions

Reviewed all hunks of `chore/macos-warning-clean` at
`2e09d209762640e0a395132736b3fad270d7a9f2`. The macOS warning cleanup is already
present or superseded. In particular, current stdin test imports cover both Linux
and macOS because both now use them; the socket fixture uses a short portable
temporary path, not the old hard-coded `/tmp`; and `ImageExpandLevel::next` is
test-only with its cycle test retained, not deleted.

Remaining tail-return style differences are in Windows Cursor auth-path handling,
Windows startup hints, and Windows launcher setup. The Linux hotkey tail has an
existing, documented `clippy::needless_return` allowance; this iteration did not
add or widen it. Only `aarch64-apple-darwin` is installed, so this review does not
claim Windows/Linux cross-target lint acceptance. No targets were installed and
the branch remains retained rather than declared completely integrated.

The associated acceptance run also exposed a separate test-only source-snapshot
fixture defect. Its deterministic reproduction and repair are recorded in
[the reconciliation receipt](UPSTREAM_RECONCILIATION_2026-09-21.md#9-follow-up-test-harness-iteration).
Final macOS acceptance passes: 1530 app-core tests (31 ignored), five stdin tests,
149 setup-hints tests, and all configured guardrails. This is not a cross-target
Windows result or evidence that the previously intermittent socket failure is fixed.
