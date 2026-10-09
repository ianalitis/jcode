# Fork master sync audit, 2026-10-09

Follows `docs/PORTFOLIO_RECONCILIATION_2026-10-07.md` §"Fork sync attempted, not
accepted", which left the next sync as an operator decision: adopt
upstream-inherited baselines on the mirror-like fork default, or authorize a
bounded cleanup of the imported debt.

This audit answers that question with numbers, and separates the **fork-master
sync** from the **integration-line merge** that actually failed in October.

## Current state

| Ref | Head | vs `origin/master` (`4a7400819`) |
| --- | --- | --- |
| `origin/master` | `4a7400819` | - |
| `fork/master` | `3ff648b71` | 6 behind, 26 ahead |
| `jcode/ci-format-baseline` (integration) | `df9792506` | 6 behind, 523 ahead |
| local `master` mirror | `21eb960a2` | 2 behind, 0 ahead |

`fork/master` already contains `3f3d837d2 "Merge upstream master and restore
validated fork quality gates"` on top of upstream `a61c38ee9`, plus `3ff648b71`
(Node 24 CI). The six commits it lacks are:

```
4a7400819 codemode: build without QuickJS on FreeBSD
e311e60bb docs: update weekly stars chart
21eb960a2 test(tui): isolate recommendation persistence and cached TeX probes (#1761)
3a213174a docs: update weekly stars chart
ff7eb9ab7 fix(ci): restore rustfmt on master (#1739)
6961272cf fix(clippy): push a single char in mcp clip_description (#1736)
```

None of them touches any budget file, so the sync does not import a competing
baseline.

## The merge is clean

```
$ git merge-tree --write-tree --name-only fork/master origin/master
b305cd168932aaec4e982aadd7287c1a9aa2373f      # exit 0, no conflicts
```

The nine conflicted paths recorded on 2026-10-07 came from merging upstream into
the **integration line**, not into `fork/master`. This sync has no conflicts.

## Gate audit on the merged tree

The merged tree was extracted with `git archive` and all five ratchets were run
against it (nothing written back to the repository, no worktree created).

| Gate | Verdict on the merged tree |
| --- | --- |
| `check_code_size_budget.py` | **PASS** - 122 tracked, 0 regressions, 5 improvements |
| `check_test_size_budget.py` | **PASS** - 3 improvements |
| `check_panic_budget.py` | **PASS** - total 166, files 59 |
| `check_wildcard_reexport_budget.py` | **PASS** - total 17 |
| `check_swallowed_error_budget.py` | **FAIL** - `crates/jcode-codemode/src/engine.rs (3)` |

`fork/master` itself passes the code-size ratchet outright (122 tracked, 0
regressions, 5 improvements), so the fork's refreshed baselines are intact.

## The one failure is a file-move artifact, not imported debt

Upstream `4a7400819` splits `crates/jcode-codemode/src/lib.rs` into `lib.rs` plus
a new `engine.rs` (QuickJS is not built on FreeBSD). The swallowed-error checker
counts `.ok()`, `let _ =` and `.unwrap_or_default()` per file:

| Tree | `lib.rs` | `engine.rs` |
| --- | --- | --- |
| `fork/master` | 3 | absent |
| merged `b305cd168` | 0 | 3 |

The global total is unchanged: the fork baseline already records
`total = 3704`, which is exactly the post-split total. The gate therefore fails
only because three pre-existing usages moved into a new file, which the per-file
rule reports as a new usage.

Upstream is red on this gate for its own, larger reasons - at `origin/master` the
checker reports `total 3667 -> 3704` plus new usage in
`crates/jcode-app-core/src/tool/codemode.rs (7)` and growth in
`tool/mcp.rs` and `auth/google.rs`. That is upstream debt, not fork debt, and it
is not imported by this sync.

## Disposition

The fork-master sync is blocked by exactly one artifact. Two ways through, both
needing operator approval:

1. Refresh only `scripts/swallowed_error_budget.json` on the sync commit, with
   the move recorded in the commit message. Smallest change, keeps the gate
   meaningful for everything else.
2. Teach `check_swallowed_error_budget.py` to attribute moved usages (compare
   against the merge base, or match the global total when a file is new and a
   sibling dropped by the same count). Larger, but stops the false positive for
   every future split.

The mirror-like default should **not** adopt upstream's baseline files: upstream
`scripts/code_size_budget.json` is 32 regressions behind its own tree and
`scripts/swallowed_error_budget.json` is red as well, so adopting them makes the
fork default permanently red. See
[the size-ratchet receipt](upstream-feedback/2026-10-09-master-size-ratchet.md).

## Reproduce

```sh
git merge-tree --write-tree --name-only fork/master origin/master
mkdir -p "$JCODE_SCRATCH_DIR/merged-b305cd168"
git archive b305cd168932aaec4e982aadd7287c1a9aa2373f | tar -x -C "$JCODE_SCRATCH_DIR/merged-b305cd168"
cd "$JCODE_SCRATCH_DIR/merged-b305cd168"
for s in check_code_size_budget check_test_size_budget check_panic_budget \
         check_swallowed_error_budget check_wildcard_reexport_budget; do
  python3 scripts/$s.py; echo "$s exit=$?"
done
```
