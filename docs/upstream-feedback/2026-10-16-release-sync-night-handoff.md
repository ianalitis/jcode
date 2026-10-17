# Release synchronization and CI night handoff, 2026-10-16

## Stop state and acceptance boundary

Source baseline: `64b560188390b4ac592c96ba77a3d4778b275686`, branch
`pr/fu149-content-filter-hint`. The working tree was clean before this two-document
handoff. No merge was started in the working tree or index. No conflict has been
claimed resolved, no upstream integration accepted, and no remote refs published.
This handoff is a continuation packet, not another controller or approval grant.

The operator requested genuine conflict resolution and night closeout. Explicit
isolation approval requested earlier has not been supplied. Global policy still
requires approval before creating a branch/worktree. Do not mutate an existing
worktree owned by another task to evade that gate. Fresh native usage at 05:56 UTC
reported 97% of the seven-day window used. Preserve the 2% interactive reserve.
No reset, provider switch, paid fallback or optional worker dispatch was admitted.
Do not start a large unreviewable merge just to leave an unfinished index overnight.

Source captain owns source integration. Peacock owns only dotfiles receipts and
metadata research, preserving two unrelated October 13 files. Source closeout
uses the documentation-only `panda-night-close` lease, released after commit.
No background implementation or runtime activation is requested by this packet.

## Verified remote and local distinction

Read-only GitHub observations at approximately 04:50 UTC:

- Fork `ianalitis/jcode` master CI `38708675955` succeeded at `d2a673788`;
  Fork Health and Workflow Lint also succeeded. No open fork PR existed.
- Recurring release-sync run `38851516522` failed on genuine merge conflicts.
  Keep its failure visible. Do not suppress the job or use wholesale ours/theirs.
- Latest stable upstream release was `v0.98.1`, published October 15.
  Tag object: `61ed7d9802058e8d93894e5931f620732f7b349a`.
  Peeled commit: `1c7426eb1925a1167eb56011fa479ad707cc28b6`.
  Local pinned ref: `refs/upstream/release-20261016`.
- Upstream release retry `38653442354` succeeded; earlier tag runs failed.
  An old red run alone is not evidence that the current release remains broken.
- Fetched upstream master was `0c5968d138b1d94e7182aebe08d0860ba4ae2638`.
  Master is not the stable-release integration target.
- Local source versus the pinned stable release has 128 local-only and 72
  release-only commits. Local source already contains the observed fork master.
  Never replace the local source with fork master or reset the fork to upstream.

Earlier accepted source cohort: 277 unique tests (218 task-control, 29 persistence,
30 swarm), production library check and owned formatting, with independent actual
patch review. This is NOT full workspace CI and NOT a test of an integrated release.

## Newly observed local CI failures, 05:58 UTC

These checks were run on the source baseline, not a merged candidate. They were
not bypassed, baseline-updated or described as green:

| Gate | Observed failure |
| --- | --- |
| `check_code_size_budget.py` | agent.rs 1294 -> 1311; agent/turn_execution.rs 1624 -> 1629; server/comm_control.rs 2625 -> 2787; server/swarm.rs 3440 -> 3596; tool/mod.rs 1883 -> 1888 |
| `check_test_size_budget.py` | agent_tests.rs 3233 -> 3326; server/swarm_persistence_tests.rs 1300 -> 1466 |
| `check_panic_budget.py` | total 179 -> 181; new per-file production panic-prone usage in server/comm_control.rs: 3 |
| `check_swallowed_error_budget.py` | let_underscore 1418 -> 1420; server/comm_control.rs 41 -> 43 |

Paths without a crate prefix in the table are under
`crates/jcode-app-core/src/`. These are detector results, not proof that every
matched expression is a runtime panic or incorrectly discarded error. Inspect the
actual matched lines and contracts before editing. Do not replace error handling
with another unchecked spelling merely to evade a detector.

Passing current-baseline checks:

- `python3 scripts/check_module_files.py`
- `python3 scripts/check_dependency_boundaries.py`
- `python3 scripts/check_wildcard_reexport_budget.py` (total 18)
- Earlier 04:50 baseline: `python3 scripts/test_fork_health.py` (5 tests) and
  `python3 scripts/test_upstream_release_sync.py` (26 tests).

A first scratch-log redirection command was refused by the tool gate and did not
run. Checks were rerun directly without that redirection, not by bypassing approval.

## Exact release conflict inventory and resolution intent

A non-worktree `git merge-tree --write-tree --name-only` simulation produced tree
`0bd666087505352c5d7d571a2d783a0086a0f0d6` with unresolved marker blobs. This is
NOT a valid release tree. Existing configured Mergiraf ran during the simulation,
automatically resolving portions of CI YAML and agent tests. Those automatic
resolutions need independent inspection too. Marker absence alone is not correctness.
The hosted fork-master merge reported eleven conflicted paths, including CI YAML;
the local-source simulation reported the following ten. The baselines differ.

| Conflict | Genuine resolution to implement and prove after admission |
| --- | --- |
| `crates/jcode-app-core/Cargo.toml` | Both sides add the same Tokio test-util dependency. Keep it once, with the isolated-package paused-time rationale. Verify app-core tests compile standalone. |
| `crates/jcode-app-core/src/server/client_lifecycle.rs` | Reconcile competing helper extraction: local `client_lifecycle_working_dir.rs` include plus retained attachment/prompt/env helpers versus upstream `client_subscribe_context.rs` module. Use one implementation of each helper, preserve target attachment/no-daemon-cwd semantics and upstream explicit load-error handling. Check visibility, all callers and both existing regression sets. Do not retain duplicate glob-imported definitions. |
| `crates/jcode-companion/src/server.rs` | Both sides make the Windows listener mutable. Keep one cfg-gated binding and upstream named-pipe rationale, then compile the real Windows target on an admitted runner. |
| `crates/jcode-tui/src/tui/app/tests/commands_accounts_02/part_01.rs` | Reconcile route-capture fixtures. Preserve the account-switch contract that the runtime actually accepts an account-qualified route and active account changes. Local exact-spec capture and upstream dual-method capture are not interchangeable without inspecting their provider semantics. Retain both relevant assertions, not whichever textual side compiles first. |
| `crates/jcode-tui/src/tui/ui.rs` | Incorporate the upstream copy-pane-drag extraction once; check existing local imports for duplication and caller visibility. Exercise copy-pane drag behavior and TUI compile/format tests. |
| `crates/jcode-voice/src/global/hotkey.rs` | Preserve strict all-fields parsing with Result and malformed-registration rejection; do not keep filter_map, which drops malformed fields. Keep the equivalent flattened focus/heartbeat/TTL predicate and prove malformed fields cannot shift a record into a valid shape. Run real hotkey regressions. |
| `scripts/code_size_budget.json` | Preserve intentional upstream reductions and local safeguards. Resolve against actual merged production sources after scoped cleanup, not max(local, upstream) or blanket regeneration. A source-length increase still needs cleanup or explicit budget-change approval. |
| `scripts/test_size_budget.json` | Reconcile extracted/moved test ownership and keep existing assertions. Use cohesive existing test modules where justified, no dummy splits to hide growth. Run the actual ratchet and both affected regression sets. |
| `scripts/panic_budget.json` | Recompute observations only after correcting newly introduced production sites and reviewing merged upstream removals. Do not raise total or add allowances without explicit approval. |
| `scripts/swallowed_error_budget.json` | Preserve intentional reductions; review new ignored results and disposition contracts. No detector-evasion rewrites or budget inflation. |

The six production/test conflicts were inspected directly from their simulated
marker blobs. Budget hunks were inventoried, but the combined output was truncated
near the end of the test-size baseline. Reread that final file in bounded excerpts
before making any resolution claim. No merged source was compiled or reviewed.

## Ordered next-session execution

1. **Re-admit and freeze.** Read this packet and the current canonical source
   receipt. Check HEAD/index/worktrees/leases and fresh native quota. Recheck release
   and fork refs, since remote observations above are timestamped. Pin a specific
   release/tag/commit. Obtain explicit isolated integration branch/worktree approval
   before creating one. Preserve the 2% reserve; no automatic reset or fallback.
2. **Restore the current source CI baseline.** Reproduce all four failing ratchets.
   Inventory matched production expressions and oversized modules. Use a scoped
   implementation packet, one writer and independent substantive review. Preserve
   accepted startup/save/terminal/ack tests. Existing modules are preferred to a
   generic new transaction/controller abstraction. Do not broaden this cleanup
   into unresolved authority or scheduler design. Do not raise budgets silently.
3. **Integrate the pinned stable release in approved isolation.** Start from the
   preserved local source, not an obsolete published branch. Review the actual
   two-parent diff, all ten current conflicts, and automatic Mergiraf resolutions.
   If moving the baseline changes conflicts, update the inventory before edits.
   Commit only reviewed resolutions, preserving both ancestry and fork safeguards.
4. **Run narrow checks after each correction, then full gates.** Use coordinated
   `selfdev test` for Cargo. At minimum preserve the existing 277-test cohort and
   add relevant lifecycle, account-routing, companion and voice checks. Require
   nonzero matching tests and real positive controls. Run isolated HOME/runtime
   fixtures with hooks and optional telemetry disabled; preserve cleanup rules.
5. **Outer acceptance on the exact integrated head.** Run current CI workflow's
   wrapper contracts, module resolution, workspace formatting, all-target/all-feature
   check and clippy with warnings denied, Cargo.lock consistency, every ratchet,
   dependency boundaries, SDK parity and applicable full test suites. The Linux,
   Windows, FreeBSD and release matrix cannot be replaced by a macOS library check.
   Do not install toolchains/dependencies without approval. Preserve failed
   invocations and distinguish environmental limits from code failures.
6. **Independent review and sync delivery.** Review the final integrated diff and
   evidence at its exact digest/head. Only after explicit remote-publication
   approval, propose a normal reviewed fork update. No reset, force-push, automatic
   default-branch merge, release or deployment. Existing sync automation pins the
   release and base, validates read-only, reconstructs the same tree on a separate
   write-capable runner, refuses changed refs/trees, and publishes a review-only
   proposal. Preserve that boundary. GITHUB_TOKEN proposals do not automatically
   trigger normal CI: require an explicitly approved check dispatch and verify its
   tested head rather than accepting inherited green checks.
7. **Close the loop.** Rerun fork health and verify the stable release is an ancestor
   of the accepted fork head while retained fork patches remain present. Distinguish
   upstream-master drift from stable-release drift. Report exact accepted commits,
   final check outcomes and remaining gates. Release all leases and leave no
   unfinished merge or running writer before another night closeout.

Exact current ratchet commands:

```bash
python3 scripts/check_code_size_budget.py
python3 scripts/check_test_size_budget.py
python3 scripts/check_panic_budget.py
python3 scripts/check_swallowed_error_budget.py
python3 scripts/check_wildcard_reexport_budget.py
python3 scripts/check_module_files.py
python3 scripts/check_dependency_boundaries.py
python3 scripts/test_fork_health.py
python3 scripts/test_upstream_release_sync.py
```

Use `.github/workflows/ci.yml` from the reviewed candidate as the full acceptance
contract, not an abbreviated handoff command list. The four budget failures above
are first-class CI work, not an invitation to regenerate budgets.

## Other forks and deferred source work

Peacock's public metadata-only inventory, not rerun by the source captain:
`firefox-agent-bridge` main was 0 ahead/5 behind upstream master and missing four
stable-v0.10.1 commits; `herdr` master was 0 ahead/34 behind and missing twenty
stable-v0.9.5 commits. Both had no open PRs and no recent Actions runs, so this is
release drift, not observed hosted-CI failure. GLOOP and jev-pr-labeler were identical
0/0 with no stable release; unknown release status is not a failure. Agentgrep
v0.1.7 and Mermaid v0.3.1 were contained. Handterm master diverged 4 ahead/15 behind
but contained stable v0.1.0. Do not blanket-sync all master branches or infer remote
write approval from this inventory. Pin/check/test/review each applicable fork in
its own repository and preserve local changes.

Existing source queue remains separate: busy-Wake and attached-enqueue result
handling, reused-actor member publication atomicity, version-pinned broadcasts,
expanded-gate positive coverage, durable authenticated-origin/attempt issuance,
same-session ABA and post-check fencing, checked recovery and fsync/power-loss
semantics. None is fixed by a textual release merge. The canonical source receipt
retains the accepted scoped corrections and their limitations. No second authority
issuer or dotfiles wire schema should be invented to bypass those prerequisites.
