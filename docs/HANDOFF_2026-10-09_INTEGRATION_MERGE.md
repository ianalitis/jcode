# Integration-line handoff, 2026-10-09

Fresh-session handoff for merging the fork default branch into the integration
line `jcode/ci-format-baseline`. Written from the fork-default side by the
session that performed the v0.93.0 sync. Nothing here is authorization: the
merge, its push and any branch or worktree change need their own operator
approval.

## Read these first

| Document | Line | What it owns |
| --- | --- | --- |
| `docs/UPSTREAM_SYNC_2026-10-09.md` | fork default (this branch) | The sync itself: provenance, ratchet repair, attribution of every changed entry, the measured next-merge conflict set, local and hosted validation |
| `docs/FORK_MASTER_SYNC_2026-10-09.md` | integration (`836ed9e61`) | The same sync audited from the integration line, plus the integration line's own upstream-feedback work |
| `docs/FORK_POSTURE.md` | integration only (`08700b0f1`) | Contribution posture and preflight |

Document shas in these receipts are blob ids, not commits. Confirm with
`git cat-file -t <sha>`.

## State when this was written

| Ref | Head | Relation |
| --- | --- | --- |
| `origin/master` (upstream) | `04c7d2b04` | `v0.93.0` plus four commits, `git describe` reads `v0.93.0-4-g04c7d2b04` |
| fork default, before the fast-forward | `3ff648b71` | 26 commits not in the integration line |
| `jcode/fork-master-sync-20261009` (sync tip) | `fbfe8ec5c` | 41 commits on top of the fork default, all of them this sync |
| `jcode/ci-format-baseline` (integration) | `8fe5468f0` | 535 commits not in the sync tip |
| Merge base, integration against sync tip | `a61c38ee9` | 2026-10-06, `auth: predict onboarding default provider and model from logins` |

Re-derive before acting, because these move:

```sh
git merge-base jcode/fork-master-sync-20261009 jcode/ci-format-baseline
git rev-list --left-right --count jcode/ci-format-baseline...jcode/fork-master-sync-20261009
git merge-base --is-ancestor origin/master jcode/ci-format-baseline; echo $?
```

That last check exits non-zero today: **the integration line has not absorbed
upstream `origin/master`**. The fork default has, because the sync commit
`c15afe46d` merges `04c7d2b04` in. Merging the fork default into the integration
line is therefore also how upstream v0.93.0 reaches that line, and the merge
carries 67 commits that are not on it.

## The merge is conflicted, measured

```sh
git -c core.attributesFile=/dev/null merge-tree --write-tree --name-only \
  jcode/fork-master-sync-20261009 jcode/ci-format-baseline
```

Both directions report the same set. The merge base is `a61c38ee9`. No
resolution was applied and none is proposed here, because the conflict count is
a property of the tree, not of the tool.

| Area | Conflicted paths |
| --- | --- |
| Workflows and manifests | `.github/workflows/ci.yml`, `.github/workflows/windows-smoke.yml`, `Cargo.toml` |
| `jcode-app-core` | `server/client_lifecycle.rs`, `tool/computer/mod.rs`, `tool/discover.rs`, `tool/todo.rs` |
| `jcode-base` | `hooks.rs`, `platform_tests.rs`, `provider/tests.rs`, `session_tests/cases.rs`, `side_panel.rs`, `skill.rs` |
| Other crates | `jcode-config-types/src/lib.rs`, `jcode-harness-api-server/src/translate_tests.rs` |
| `jcode-tui` tests | `app/tests/onboarding_flow.rs`, `app/tests/onboarding_sim.rs`, `app/tests/remote_startup_input_02/part_01.rs`, `app/tests/scroll_copy_01/part_01.rs`, `ui_tests/basic/body_cache.rs` |
| Ratchets, scripts, CLI | `scripts/code_size_budget.json`, `scripts/swallowed_error_budget.json`, `scripts/test_size_budget.json`, `scripts/test_check_warning_budget.py`, `src/cli/startup.rs` |
| Fork docs | `docs/FORK_CI.md` |

### The count depends on the merge driver

These measurements ran on a machine whose **global** `~/.config/git/attributes`
routes every path to the structural driver `mergiraf` (`* merge=mergiraf`, driver
defined in `~/.config/git/config`, neither is a repository setting).

| Driver | Conflicts |
| --- | --- |
| `mergiraf` active | 26 |
| `-c core.attributesFile=/dev/null` (git's own merge) | 33 |

Mergiraf resolves seven of the 33 structurally:
`crates/jcode-app-core/src/tool/goal.rs`,
`crates/jcode-app-core/src/update_tests_body_tests.rs`,
`crates/jcode-base/src/provider/mod.rs`,
`crates/jcode-base/src/session/persistence.rs`,
`crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs`,
`crates/jcode-tui/src/tui/app/tests/remote_startup_input_01/part_02.rs`,
`crates/jcode-tui/src/tui/ui_tests/palette_topology.rs`.

Neither number is a resolution. Both merged trees still carry unresolved
markers, and every one of the 26 or 33 paths needs a union resolution by hand.
Name the driver in whatever receipt you write, because a bare "26 conflicts" is
not reproducible on a machine without that global attribute.

### Two of the 26 are add/add

- `docs/FORK_CI.md`: absent at the merge base. Fork default `3dddfa8ea`, this
  sync `76c95b9fc`, integration `476d707cf`. Keep the integration line's copy
  and fold this sync's landed-delta note into it.
- `scripts/test_check_warning_budget.py`: absent at the merge base, present on
  both lines. Relative to the integration line the sync tip is +6/-75, so the
  integration line's version is the larger one. Compare both before choosing.

### Four paths have a recorded history on the integration line

- `scripts/check_warning_budget.sh` does **not** conflict: `2aed425e2` on the
  integration line already carries the union of the fork's bounded, baseline-first
  counting and upstream's `CARGO` override plus failed-check diagnostic, and it
  is byte-identical to the sync tip's copy. Keep it.
- `scripts/test_check_warning_budget.py` and
  `scripts/test_check_warning_budget.sh` are two suites for one contract.
  `2aed425e2` records the measurement: upstream's script alone fails 7 of the 8
  cases in the Python suite, the fork's script alone fails 6 of 6 in the shell
  suite, and the landed union passes both. Re-run both suites after resolving.
- The three ratchet JSONs conflict because the two lines carry different
  baselines: the fork's were refreshed against the fork default on 2026-10-07
  (`code_size` `0d8a02e55`, `test_size` `f37c35b3f`, `swallowed_error`
  `d6d6db015`), while the integration line still carries upstream's own blobs.
  Do not pick a side. Resolve to one file, then re-measure the merged tree with
  the scripts' own `--update` path, because neither baseline describes the tree
  the merge produces.
- `crates/jcode-codemode/src/lib.rs` -> `engine.rs`: upstream's split moved three
  pre-existing swallowed-error usages into a new file. The fork's refreshed
  baseline records the split, so it is not a regression in either line. If the
  merged tree reports it, the baseline needs the move recorded, not an
  allowance.

## Resolution policy

- Union. Never drop either side's work and never delete or rename a file to make
  a conflict go away.
- Do not weaken a gate, delete an assertion or add an allow-list entry to reach
  green. A ratchet that fails on the merged tree is re-measured, and the
  measurement is stated in the commit message.
- Every resolved conflict is verified by the narrow test for the file it is in,
  not by the fact that the merge completed.

## Verification before the merge is called done

```sh
for s in check_code_size_budget check_test_size_budget check_panic_budget \
         check_swallowed_error_budget check_wildcard_reexport_budget; do
  python3 scripts/$s.py; echo "$s exit=$?"
done
python3 scripts/test_check_warning_budget.py
bash scripts/test_check_warning_budget.sh
cargo fmt --all --check
```

Then the crate tests for the conflicted files, and the TUI test targets under
`crates/jcode-tui/src/tui/app/tests/` and `ui_tests/`. Extract the merged tree
with `git archive` into a scratch directory and run the gates there first. A
green local gate set is not a claim that hosted CI is green.

## Operator decisions this handoff does not make

- **Posture and preflight placement.** `docs/FORK_POSTURE.md` and the
  contribution-preflight section of `AGENTS.md` (82 lines longer on the
  integration line than on the fork default) exist only on the integration line.
  Nothing on the fork default references them, so there is no dangling
  instruction, but a session started on the default branch gets none of that
  guidance. Either the posture doc and its preflight belong on the default
  branch, or the guidance is deliberately integration-line only and should say
  so where a default-branch session can see it.
- **Upstream's own red gates.** At `9948f0e8c` (`v0.93.0`) upstream's tree fails
  `check_code_size_budget.py` on 46 entries and
  `check_swallowed_error_budget.py` on 30 (`total 3667` against a measured
  3728), and the integration line's audit separates that upstream debt from the
  fork's, which contributes zero ratchet regressions. Whether to spend effort on
  upstream's debt is a separate decision from this merge. Re-measure at
  `04c7d2b04` before quoting those numbers, because the four commits after the
  tag are not covered by them.
- **The TUI quarantine skip list.** The default branch's `ci.yml` carries 14
  `--skip` entries on the `jcode-tui` library step and one on the
  `provider_matrix` step, and this sync changes none of them. Upstream
  `origin/master` and the integration line carry only 2, so the merge keeps the
  default branch's list. Fourteen of the fifteen name tests that upstream's own
  tree already contains, and the fifteenth,
  `right_fact_stack_uses_neutral_gray_except_for_context_usage`, names no test in
  either tree and was added by upstream itself. The comment block names an owning
  issue per group (#1340 with #1344, #1367 with #1368, #1342, and #1341 with
  #1344), and `docs/FORK_CI.md` records the same list. v0.93.0 includes
  `test(tui): isolate recommendation persistence and cached TeX probes` (#1761),
  which is the same isolation class as #1344 but not the same fix, so no entry is
  proven obsolete. Pruning one needs a Linux leg run without that filter, and no
  entry was removed by this sync.

## Open follow-ups carried from the sync receipt

- `crates/jcode-base/src/session/persistence.rs` #1373 delta stays until that PR
  lands upstream.
- The macOS `__eh_frame` link warning, Node 20 deprecation warnings from
  `actions/checkout@v4`, the advisory Windows ARM64 Linux cross-compilation, and
  the absence of local `cargo-audit` and `cargo-machete` are all unchanged from
  the 2026-10-07 receipt.
