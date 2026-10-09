# Fork master sync to upstream v0.93.0, 2026-10-09

## Scope and authorization

Operator approved finishing the upstream synchronization, the ratchet repair it
forces, this receipt and publishing the sync branch to the fork. No force push,
branch deletion, toolchain installation or provider change.

Upstream source is tag `v0.93.0` = `04c7d2b04`. Base fork default is
`3ff648b711d9`. The merge is on `jcode/fork-master-sync-20261009` in the
retained worktree `~/.jcode/scratch/fork-master-sync-20260928`. The previous
receipt is `UPSTREAM_SYNC_2026-10-07.md` (upstream `a61c38ee9`).

This file is named `UPSTREAM_SYNC_*` to match that predecessor, and because
`docs/FORK_MASTER_SYNC_2026-10-09.md` already exists on the integration line
`jcode/ci-format-baseline` (`836ed9e61`) as an audit written from that line's
point of view. Two different documents at one path would be an add/add conflict
at the next integration merge, so this receipt takes the other name.

## Merge provenance

Every conflict list below is reproduced with
`git merge-tree --write-tree --name-only <parent1> <parent2>`, not from memory.

| Commit | Parents | Content |
| --- | --- | --- |
| `c15afe46d` | `3ff648b71` + `04c7d2b04` | upstream v0.93.0 merge, 89 files, +3908/-1650. Three conflicts: `.github/workflows/ci.yml`, `.github/workflows/windows-smoke.yml`, `scripts/check_warning_budget.sh`. All resolved to union. 11 files carry both sides. |
| `dd151ccc3` | `c15afe46d` | refresh the three stale quality ratchets, 3 files, +78/-58. |
| `befccf4b2` | `dd151ccc3` + `4ceceaa8b` | upstream PR #1354 head (`pr/clippy-1.98-lint-drift`). One conflict: `crates/jcode-base/src/side_panel.rs`. 4 files carry both sides. |
| `2bb566b5e` | `befccf4b2` | absorb #1354's line growth into the ratchets, 2 files, +7/-7. |

Whole sync against `fork/master`: 99 files, +4039/-1728. No file is deleted and
no file is renamed (`--find-renames` reports none), so fork content is preserved
by construction; the two quarantine deltas are verified individually below.

## Why the ratchets had to be refreshed

The three baseline JSON files in `fork/master` are upstream's own files, imported
verbatim on 2026-10-07. Upstream's baseline is stale against upstream's own
source, so a merged tree can never match it and the imported numbers must be
re-measured against the merged candidate. Refreshes used the scripts' own
`--update` path on the merged tree. No tracked entry was added or removed:
`code_size_budget.json` still tracks 122 production files,
`test_size_budget.json` 49 test files, `swallowed_error_budget.json` 536 paths.
Panic-prone total is unchanged at 166 across 59 files; the wildcard re-export
budget is unchanged at 17.

Measured totals after the refresh: swallowed-error 3704 -> 3730 (`dot_ok`
1420 -> 1438, `let_underscore` 1364 -> 1373, `unwrap_or_default` 920 -> 919).

### Attribution of every changed entry

45 tracked files changed value (32 up, 13 down). Each was classified by blob
identity against upstream v0.93.0, the #1354 head `4ceceaa8b` and `fork/master`:

- **31 are byte-identical to upstream v0.93.0.** They changed only because
  upstream's imported baseline was stale. Includes `tool/mod.rs` 1731 -> 1855,
  `src/cli/provider_init.rs` 1902 -> 1979 and `src/cli/dispatch.rs`
  1518 -> 1554.
- **5 are #1354's own content**: `tool/tests.rs` 1877 -> 1941,
  `auth/lifecycle.rs` 3065 -> 3093, `onboarding_flow_control.rs` 1752 -> 1767,
  `state_ui_input_helpers.rs` 2236 -> 1819, `tests/onboarding_flow.rs`
  1662 -> 1661.
- **4 are pre-existing fork content that this sync did not touch**, so only the
  stale upstream baseline explains the change: `agent_tests.rs` 3021 -> 2930,
  `session_tests/cases.rs` 2959 -> 2928, `translate_tests.rs` 4028 -> 4009,
  `tui/app/helpers.rs` 1716 -> 1670.
- **5 are unions produced by the merges**: `server/client_lifecycle.rs`
  3906 -> 3875, `tool/discover.rs` 2982 -> 2927, `tool/todo.rs` 2532 -> 2494,
  `translate.rs` 3500 -> 3485, `tests/scroll_copy_01/part_01.rs`
  1557 -> 1583.

An earlier draft of this receipt named only `scroll_copy_01/part_01.rs` as a
union. That was too narrow: the table above is the verified attribution, and the
five union files are exactly the paths where the merged tree matches neither
parent on both sides.

The second refresh (`2bb566b5e`) is entirely #1354 content
(`auth/lifecycle.rs` +5, `tool/tests.rs` +15, `tests/onboarding_flow.rs` +6,
`onboarding_flow_control.rs` -3, `state_ui_input_helpers.rs` -1) plus the two
union residuals `tool/discover.rs` and `tool/todo.rs` at -1 each. It exists
because #1354 was merged after the first refresh, which moved the counts again.

## Quarantine deltas

`FORK_CI.md` lists two fix deltas that existed only so the fork could be green
before upstream acted. Their status after this sync:

- `src/bin/tui_bench.rs`: `HEAD` = `fork/master` = `4ceceaa8b` = upstream
  `04c7d2b04` (`b1079e16`). Upstream now ships the same content, so the fork
  delta is gone and the `FORK_CI.md` entry is deleted in this commit.
- `crates/jcode-base/src/session/persistence.rs`: `HEAD` (`4d1e94ca`) still
  differs from upstream (`1e8489fc`), and upstream did change the file in this
  release (`perf(session): parse startup snapshots with from_slice`). The #1373
  delta is retained and is still required.

The single conflict in `befccf4b2` was resolved to the #1354 wording of one
comment, so `HEAD`'s `side_panel.rs` is byte-identical to `4ceceaa8b`. The fork's
previous wording of that comment ("Mirrors upstream's helper shape so downstream
merges stay cheap") was replaced by the PR's equivalent ("the helper shape is
load-bearing for downstream merges"). The `#[allow(clippy::too_many_arguments)]`
attribute and all code are unchanged: the fork lost a comment phrasing, not
behavior.

## Local acceptance

All 15 `Quality Guardrails` steps pass in 98s on the merged tree:

1. module declarations, 2. formatting, 3. `check --all-targets --all-features`,
4. clippy `-D warnings`, 5. `Cargo.lock` currency, 6. warning-budget gate test,
7. warning-budget contract tests, 8. warning budget (0 against baseline 0),
9. oversized-file ratchet, 10. oversized-test ratchet, 11. panic-prone ratchet
(166/59), 12. swallowed-error ratchet (3730/536), 13. crate dependency
boundaries, 14. SDK surface parity, 15. wildcard re-export ratchet (17).

`cargo test -p jcode-sdk parity -- --nocapture` run directly reports 3 passed
(`the_typescript_sdk_implements_every_shared_capability`,
`the_rust_sdk_implements_every_shared_capability`,
`neither_sdk_has_an_untriaged_public_capability`). The gate harness keeps only
the last lines of a step, so its captured tail shows the final target's
`0 passed; 5 filtered out` summary rather than the parity target's own result;
the step is non-vacuous and was verified separately.

The 15 steps are the quality job only. They are not the TUI test suite, the
integration cohort or the platform builds; those are covered by hosted CI below.

## Hosted validation

CI run `37951757293` on `2bb566b5e`:

| Job | Conclusion |
| --- | --- |
| Quality Guardrails | success |
| Format | success |
| PowerShell Syntax | success |
| Release Automation | success |
| TypeScript SDK | success |
| Setup Friction Eval (Linux installer) | success |
| Windows Cross-Target Check (Linux) | success |
| Build & Test (windows-latest, macos-latest, ubuntu-latest) | see the run |

The final three job conclusions and the fast-forward of `fork/master` are
recorded in the session receipt for this sync; a green local gate set is not a
claim that hosted CI or runtime behavior is green.

## Paths that will conflict at the next integration merge

`jcode/ci-format-baseline` carries its own copies of two fork docs, and one
fork-referenced doc is missing from the default branch entirely:

- `docs/FORK_CI.md` differs: fork default `3dddfa8ea`, integration line
  `476d707cf`. The next merge conflicts on content and needs a union resolution,
  keeping this sync's landed-delta note.
- `docs/FORK_POSTURE.md` exists only on the integration line (`08700b0f1`).
  `AGENTS.md` on every branch tells sessions to read it, so on `fork/master`
  that instruction points at a file that is not there. Either the posture doc
  belongs on the default branch or the instruction needs a branch qualifier;
  deciding which is an operator call, not a merge detail.
- This receipt avoided a third collision by taking the `UPSTREAM_SYNC_*` name
  (see the note at the top).

## Open follow-up

- **Skip-list pruning.** All 13 tests in the `ci.yml` quarantine skip list now
  live in upstream-authored files, and upstream v0.93.0 includes
  `test(tui): isolate recommendation persistence and cached TeX probes` (#1761),
  which targets the same isolation class. Pruning an entry requires a Linux leg
  run without that filter, so no entry is removed here.
- **`persistence.rs` #1373 delta** stays until that PR lands upstream.
- Carried from the previous receipt and unchanged: the macOS `__eh_frame`
  link warning, Node 20 deprecation warnings from `actions/checkout@v4` and
  Windows msvc setup, the advisory Windows ARM64 Linux cross-compilation, and
  the absence of local `cargo-audit`/`cargo-machete`.
- The integration line handoff is a separate document; this receipt covers the
  fork default branch only.
