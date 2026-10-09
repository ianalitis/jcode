# Fork master sync to upstream origin/master, 2026-10-09

## Scope and authorization

Operator approved finishing the upstream synchronization, the ratchet repair it
forces, this receipt and publishing the sync branch to the fork. No force push,
branch deletion, toolchain installation or provider change.

Upstream source is `origin/master` = `04c7d2b04`, which is four commits past the
`v0.93.0` tag (`9948f0e8c`); `git describe` reads `v0.93.0-4-g04c7d2b04`. The tag
is not the merge target. Base fork default is `3ff648b711d9`. The merge is on
`jcode/fork-master-sync-20261009` in the retained worktree
`~/.jcode/scratch/fork-master-sync-20260928`. The previous receipt is
`UPSTREAM_SYNC_2026-10-07.md` (upstream `a61c38ee9`).

This file is named `UPSTREAM_SYNC_*` to match that predecessor, and because
`docs/FORK_MASTER_SYNC_2026-10-09.md` already exists on the integration line
`jcode/ci-format-baseline` (blob `836ed9e61`) as an audit written from that
line's point of view. Two different documents at one path would be an add/add
conflict at the next integration merge, so this receipt takes the other name.
Wherever this receipt cites a document sha it is a blob id, not a commit: verify
with `git cat-file -t <sha>`, which reports `blob`.

## Merge provenance

Every conflict list below is reproduced with
`git merge-tree --write-tree --name-only <parent1> <parent2>`, not from memory.

| Commit | Parents | Content |
| --- | --- | --- |
| `c15afe46d` | `3ff648b71` + `04c7d2b04` | upstream `origin/master` merge (`v0.93.0` plus four commits), 89 files, +3908/-1650. Three conflicts: `.github/workflows/ci.yml`, `.github/workflows/windows-smoke.yml`, `scripts/check_warning_budget.sh`. All resolved to union. 11 files carry both sides. |
| `dd151ccc3` | `c15afe46d` | refresh the three stale quality ratchets, 3 files, +78/-58. |
| `befccf4b2` | `dd151ccc3` + `4ceceaa8b` | upstream PR #1354 head (`pr/clippy-1.98-lint-drift`). One conflict: `crates/jcode-base/src/side_panel.rs`. 4 files carry both sides. |
| `2bb566b5e` | `befccf4b2` | absorb #1354's line growth into the ratchets, 2 files, +7/-7. |

Whole sync against `fork/master`: 99 files, +4039/-1728. No file is deleted and
no file is renamed (`--find-renames` reports none), so fork content is preserved
by construction; the two quarantine deltas are verified individually below.

## Why the ratchets had to be refreshed

`fork/master` does not carry upstream's baseline files. Its three baseline blobs
were refreshed against the fork default on 2026-10-07 (swallowed `d6d6db015`,
code size `0d8a02e55`, test size `f37c35b3f`), while the merge base `a61c38ee9`
and upstream `origin/master` both still carry upstream's own blobs (`d395773af`,
`296c9fbab`, `85adb9c1f`). Because the merge base matches upstream, this merge
brings no competing baseline and cannot conflict on these three paths. It also
means the merged candidate is measured against a baseline that predates
v0.93.0's growth.

That candidate fails three of the five ratchets. Measured on the merged tree
before either refresh (`c15afe46d`, extracted with `git archive` and run through
the scripts' own entry points):

| Gate | Verdict at `c15afe46d` | Entries |
| --- | --- | --- |
| `check_code_size_budget.py` | FAIL | 21 |
| `check_test_size_budget.py` | FAIL | 3 |
| `check_swallowed_error_budget.py` | FAIL | 14 files plus 3 aggregate counters |
| `check_panic_budget.py` | PASS | total 166, files 59 |
| `check_wildcard_reexport_budget.py` | PASS | total 17 |

So both refreshes re-measured the baselines against the merged candidate with the
scripts' own `--update` path. This is the deliberate trade in this sync: **the
fork default stays green because the baseline moved, not because the tree
shrank.** The alternative is to shrink upstream's oversized files inside the
fork, which makes every later merge conflict. Adopting upstream's own baseline
file is not an option either, because it is red against upstream's own tree: the
file says `total = 3667` while upstream's tree measures 3730, which is the same
number the merged tree measures. The merge adds no swallowed-error growth of its
own; all 26 of the increase over the fork baseline is upstream's.

Measured totals after both refreshes: swallowed-error 3704 -> 3730 (`dot_ok`
1420 -> 1438, `let_underscore` 1364 -> 1373, `unwrap_or_default` 920 -> 919).
`swallowed_error_budget.json` tracks 536 paths (532 before: five added, one
removed); `code_size_budget.json` still tracks 122 production files and
`test_size_budget.json` 49 test files, with no path added or removed in either.

### Attribution of every changed entry

Comparing the baselines at `fork/master` with `2bb566b5e` (after both refreshes),
52 tracked entries differ: 46 changed value (34 up, 12 down), five paths were
added and one removed. Each entry was classified by the blob identity of its
source file against upstream `04c7d2b04`, the #1354 head `4ceceaa8b` and
`fork/master`. Each class is consistent with exactly one cause:

- **37 are byte-identical to upstream `origin/master`**, where the fork side was
  unchanged from the merge base and upstream's file simply arrived: includes
  `tool/mod.rs` 1731 -> 1855, `src/cli/provider_init.rs` 1902 -> 1979,
  `src/cli/dispatch.rs` 1518 -> 1554. Five of the 37 are paths upstream added and
  the swallowed-error ratchet newly tracks (`detected_emails.rs` 5,
  `scratch_maintenance.rs` 4, `version_gc.rs` 6, `engine.rs` 3,
  `remote_header_hint.rs` 1), and one is the path upstream's `lib.rs` ->
  `engine.rs` split removed (`crates/jcode-codemode/src/lib.rs`, 3 -> absent).
  That split is the artifact the integration line's audit flagged as a false
  positive; here it is recorded in the baseline instead of explained away.
- **6 are #1354's content**, merged in `befccf4b2`: `auth/lifecycle.rs`
  3065 -> 3098, `onboarding_flow_control.rs` 1752 -> 1764,
  `state_ui_input_helpers.rs` 2236 -> 1818 (code size) and 6 -> 2 (swallowed
  errors), `tool/tests.rs` 1877 -> 1956, `tests/onboarding_flow.rs`
  1662 -> 1667.
- **4 are pre-existing fork content that this sync did not touch**, where the
  fork side changed and upstream did not: `agent_tests.rs` 3021 -> 2930,
  `session_tests/cases.rs` 2959 -> 2928, `translate_tests.rs` 4028 -> 4009,
  `tui/app/helpers.rs` 1716 -> 1670.
- **5 are unions that match no single parent**: `server/client_lifecycle.rs`
  3906 -> 3875 and `tests/scroll_copy_01/part_01.rs` 1557 -> 1583, where fork and
  upstream both changed the file and the merge combined them; and
  `tool/discover.rs` 2982 -> 2926, `tool/todo.rs` 2532 -> 2493 and
  `translate.rs` 3500 -> 3485, where the fork's own content is combined with
  #1354's one-line changes.

37 + 6 + 4 + 5 = 52. In all 47 one-sided classes the losing side's blob is
unchanged from the merge base, so nothing was dropped: those entries are one
side's content arriving whole. The other five are two-sided merges. An earlier
draft of this receipt reported 45 entries (32 up, 13 down) and claimed no path
was added or removed. Both were wrong: it counted only the first refresh's value
changes, missing the second refresh's seven entries and the six added or removed
paths. The numbers above are re-derived from the commits with
`git show <rev>:scripts/*_budget.json`.

The second refresh (`2bb566b5e`) exists because #1354 was merged after the first
one, which moved the counts again. It changed exactly seven entries, all of them
#1354's: `auth/lifecycle.rs` 3093 -> 3098, `onboarding_flow_control.rs`
1767 -> 1764, `state_ui_input_helpers.rs` 1819 -> 1818, `tool/discover.rs`
2927 -> 2926, `tool/todo.rs` 2494 -> 2493, `tool/tests.rs` 1941 -> 1956,
`tests/onboarding_flow.rs` 1661 -> 1667. Two of those paths, `tool/discover.rs`
and `tool/todo.rs`, are counted as unions above because the fork's own content is
still in them; what moved in the second refresh is #1354's one-line change.

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

## The next integration merge, measured

`jcode/ci-format-baseline` and this sync tip were compared with
`git merge-tree --write-tree --name-only` in both directions. The merge base is
`a61c38ee9` (2026-10-06, `auth: predict onboarding default provider and model
from logins`), and both directions report the same conflict set.

| Area | Conflicted paths |
| --- | --- |
| Workflows and manifests | `.github/workflows/ci.yml`, `.github/workflows/windows-smoke.yml`, `Cargo.toml` |
| `jcode-app-core` | `server/client_lifecycle.rs`, `tool/computer/mod.rs`, `tool/discover.rs`, `tool/todo.rs` |
| `jcode-base` | `hooks.rs`, `platform_tests.rs`, `provider/tests.rs`, `session_tests/cases.rs`, `side_panel.rs`, `skill.rs` |
| Other crates | `jcode-config-types/src/lib.rs`, `jcode-harness-api-server/src/translate_tests.rs` |
| `jcode-tui` tests | `app/tests/onboarding_flow.rs`, `app/tests/onboarding_sim.rs`, `app/tests/remote_startup_input_02/part_01.rs`, `app/tests/scroll_copy_01/part_01.rs`, `ui_tests/basic/body_cache.rs` |
| Ratchets, scripts, CLI | `scripts/code_size_budget.json`, `scripts/swallowed_error_budget.json`, `scripts/test_size_budget.json`, `scripts/test_check_warning_budget.py`, `src/cli/startup.rs` |
| Fork docs | `docs/FORK_CI.md` |

Two of those are add/add, the rest are content conflicts:

- `docs/FORK_CI.md` is absent at the merge base and both lines carry their own
  copy (`3dddfa8ea` on the fork default, `76c95b9fc` on this sync, `476d707cf` on
  the integration line). The union resolution keeps this sync's landed-delta
  note.
- `scripts/test_check_warning_budget.py` is also absent at the merge base and
  exists on both lines.

Two fork docs on the integration line do not conflict at all:

- `docs/FORK_POSTURE.md` exists only there (blob `08700b0f1`), so the merge keeps
  it unchanged.
- `AGENTS.md` (`266cacec5` at the merge base and on the fork default, `a1730fcea`
  on the integration line, 82 lines longer) and `docs/README.md` (`7f5d21fca`
  against `1d6ae4b5a`) changed on the integration line only, so both are taken
  without a conflict.

The instruction that points at the posture doc lives in that longer `AGENTS.md`:
contribution-preflight step 1 reads `docs/FORK_POSTURE.md`, and `docs/README.md`
links it. On `fork/master` nothing references the posture doc, so there is no
dangling instruction; the gap runs the other way. A session started on the
default branch gets none of that posture or preflight guidance, and neither doc
is present there. Either the posture doc and its preflight belong on the default
branch, or the guidance is deliberately integration-line only and should say so
where a default-branch session can see it. That is an operator call, not a merge
detail.

The count depends on the merge driver, so the handoff names it. These
measurements ran on a machine whose `~/.config/git/attributes` routes every path
to the structural driver `mergiraf` (`* merge=mergiraf`, driver in
`~/.config/git/config`; neither is a repository setting). With it active the
conflict set is 26 paths. With `-c core.attributesFile=/dev/null`, that is git's
own merge, the same command reports 33 conflicts, because mergiraf resolves seven
of them structurally: `crates/jcode-app-core/src/tool/goal.rs`,
`crates/jcode-app-core/src/update_tests_body_tests.rs`,
`crates/jcode-base/src/provider/mod.rs`,
`crates/jcode-base/src/session/persistence.rs`,
`crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs`,
`crates/jcode-tui/src/tui/app/tests/remote_startup_input_01/part_02.rs` and
`crates/jcode-tui/src/tui/ui_tests/palette_topology.rs`. Neither number is a
resolution: all of them still need a union resolution by hand.

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
