# Fork master sync audit, 2026-10-09

Follows `docs/PORTFOLIO_RECONCILIATION_2026-10-07.md` §"Fork sync attempted, not
accepted", which left the next sync as an operator decision: adopt
upstream-inherited baselines on the mirror-like fork default, or authorize a
bounded cleanup of the imported debt.

This audit answers that question with numbers, and separates the **fork-master
sync** from the **integration-line merge** that actually failed in October.

> **Read the addendum at the end of this file first.** `origin/master` advanced
to `9948f0e8c` (v0.93.0) while this audit was in flight. The merge is no longer
conflict-free, and the merged tree now fails three of the five ratchets. The
sections below are the record at `4a7400819` and are kept as written.

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

---

## Addendum: the head advanced, the merge re-became conflicted

`origin/master` moved from `4a7400819` to `9948f0e8c` (`release: v0.93.0`, 12
further commits, merge base `a61c38ee9`) before this audit was acted on.
Re-running the sync against the new head changes two of its conclusions.

### The merge now conflicts on exactly one file

```
$ git merge-tree --write-tree --name-only fork/master origin/master
c50bbd482bdf23fab0dba5aacdfb2b8efd55906a      # exit 1
scripts/check_warning_budget.sh               # 3 stages
```

Both sides fixed issue #1762 independently, so this is a same-problem
collision, not a mechanical one:

| Side | Commit | Fix |
| --- | --- | --- |
| fork | `e68b94560` | `grep` instead of the absent `rg`, baseline validated before compiling, bounded diagnostics, `scripts/test_check_warning_budget.py` |
| upstream | `b4851ae86` | `CARGO` override, cargo status captured separately, diagnostics printed on failure, `scripts/test_check_warning_budget.sh` |

The merged tree carries **both** self-tests, so merged CI would run both.

### The budget files still do not conflict

For all three budget JSONs the merge-base blob equals upstream's
(`code_size` `296c9fbab`, `test_size` `85adb9c1f`, `swallowed_error`
`d395773af`): only the fork has refreshed them, so the merge keeps the fork's
versions (`0d8a02e55`, `f37c35b3f`, `d6d6db015`) with no conflict.
`warning_budget.txt` is `0` on all three refs.

### Resolution: neither side's script satisfies both contract suites

Measured, not assumed:

| Content | `test_check_warning_budget.py` | `test_check_warning_budget.sh` |
| --- | --- | --- |
| upstream's script alone | **FAIL 7/8** (`test_over_budget_output_is_bounded_and_counted`: upstream prints every warning line) | 6/6 |
| the fork's script alone | 14/14 | **FAIL 6/6** (no `CARGO` override, no failed-check diagnostic) |
| union (landed) | 14/14 | 6/6 |

Landed ahead of the merge as `2aed425e2` on the integration line, so the merge
resolves this path by keeping ours. It is the fork's bounded, baseline-first
counting plus upstream's two requirements (`cargo_bin=${CARGO:-cargo}` and
`error: cargo check failed (exit N); warning budget not evaluated`), with the
same wording upstream uses so a later merge stays quiet. `shellcheck` and
`bash -n` are clean; the fork's older 8-case suite passes against the same
content in the merged tree.

### Gate audit at the new head: three gates fail, all of it upstream growth

| Gate | `fork/master` `3ff648b71` | `origin/master` `9948f0e8c` | merged `c50bbd48` |
| --- | --- | --- | --- |
| `check_code_size_budget.py` | **PASS** (5 improvements) | **FAIL** 46 entries | **FAIL** 22 entries |
| `check_test_size_budget.py` | **PASS** (3 improvements) | **FAIL** 10 | **FAIL** 3 |
| `check_panic_budget.py` | PASS | PASS | PASS |
| `check_swallowed_error_budget.py` | **PASS** | **FAIL** 30 (total 3667 -> 3728) | **FAIL** 15 (total 3704 -> 3728) |
| `check_wildcard_reexport_budget.py` | PASS | PASS | PASS |

Every path in the merged tree's failure lists also appears in upstream's own
list. The set difference is empty for both size ratchets, and the three
swallowed-error entries unique to the merged tree are aggregate counters
(`total`, `dot_ok`, `let_underscore`), which differ only because the fork's
baseline starts 37 higher. **The fork contributes zero ratchet regressions.**

Why the verdict flipped from the earlier "blocked by exactly one artifact": at
`4a7400819` the merged tree passed both size ratchets, and the only
swallowed-error entry was the `engine.rs` move artifact. The 12 upstream commits
since then are 75 files, +3169/-762, and they grew exactly the oversized files
the fork's baseline tracks (`crates/jcode-app-core/src/tool/mod.rs` 1731 ->
1855 with the hooks commit, `src/cli/provider_init.rs` 1902 -> 1979 with the
model-prefix commit). The swallowed-error total is now real growth, not a move.

### Upstream's ratchets are not currently enforcing

Local measurement is equivalent to CI here: all three gates enumerate files with
`rglob("*.rs")` and have no git dependency, so upstream's own tree failing them
should make upstream's CI red. It does not, because the ratchets never run:

```
$ gh run view 37727625832 -R 1jehuang/jcode   # master @ 21eb960a2, failure
JOB Quality Guardrails -> failure
   5 success   Check module declarations resolve
   6 failure   Check formatting
   7 skipped   Check all targets and all features
  11 skipped   Enforce oversized-file ratchet
  12 skipped   Enforce oversized-test ratchet
  13 skipped   Enforce panic-prone usage ratchet
  14 skipped   Enforce swallowed-error usage ratchet
  17 skipped   Enforce wildcard re-export ratchet
```

Five consecutive master runs (`21eb960a2`, `ff7eb9ab7`, `a61c38ee9`,
`711f4001d`, `152365582`) all stop at `Check formatting` with every ratchet step
skipped. That is how upstream's baseline drifted 37 regressions behind its own
tree without anyone noticing, and it means the fork's ratchets are the only
place these gates actually execute. PR-side enforcement was not checked.

### Updated disposition

The sync is no longer blocked by one artifact, and option 1 above (refresh only
`swallowed_error_budget.json`) is no longer sound: the +24 is upstream growth,
not the `engine.rs` move. The remaining ways through all need operator approval:

1. **Defer the sync.** `fork/master` is green on all five gates at `3ff648b71`;
syncing to `9948f0e8c` imports 22 + 3 + 24 regressions of upstream growth that
the fork's own rule ("oversized files must shrink or stay flat") rejects.
2. **Sync with all three baselines refreshed** on the sync commit, recording in
the message that upstream's growth is adopted as the new floor. Explicit
acceptance of upstream debt, and the opposite of the ratchet discipline the
fork just spent effort restoring.
3. **Shrink first.** Reduce the 22 oversized files, 3 oversized test files and
24 swallowed-error usages upstream, then sync. Correct but large, and it is
upstream's work.

The warning-gate conflict itself is resolved either way by `2aed425e2`.

### Reproduce this addendum

```sh
git merge-tree --write-tree --name-only fork/master origin/master
for ref in fork/master origin/master; do
  d="$JCODE_SCRATCH_DIR/attr-$(echo "$ref" | tr / -)"; mkdir -p "$d"
  git archive "$ref" | tar -x -C "$d"
  (cd "$d" && for s in check_code_size_budget check_test_size_budget \
       check_swallowed_error_budget; do python3 scripts/$s.py; echo "$s exit=$?"; done)
done
gh run view 37727625832 -R 1jehuang/jcode --json jobs   # ratchet steps skipped
```
