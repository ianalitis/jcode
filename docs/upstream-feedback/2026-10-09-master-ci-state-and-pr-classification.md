# Upstream master CI state and fork-PR classification, 2026-10-09

Upstream master head at capture: `a6ba7844f` (run `37893897817`, started
2026-10-09T06:30Z, still in progress when this was written). Local
`origin/master` ref is stale at `9948f0e8c`; every upstream claim below is read
from the API at an explicit SHA, not from the local ref.

Local fork: HEAD `2fb2858fe` on `jcode/ci-format-baseline`, fork CI run
`37888192151` green.

This receipt continues `2026-10-07-master-format-gates.md` and
`2026-10-09-master-size-ratchet.md`. It corrects one ancestry claim made in the
earlier notes (see §5) and classifies the ten open fork PRs by failure cause.

## 1. Master carries four failure classes; two are already fixed upstream

| # | Class | Evidence | State |
| --- | --- | --- | --- |
| 1 | rustfmt drift, now **3 hunks in 2 files** | Format job `113701122131` (run `37893897817`, head `a6ba7844f`, 06:32Z) | **open** |
| 2 | warning budget, `current=2 baseline=0` | run `37727625832` @ `21eb960a2`: B&T ubuntu (job `113149293057`) **and** macos (job `113149292962`) failed at step 17 `Enforce warning budget (Linux, macOS)` | fixed by `671b65131` (content) + `b4851ae86` (gate robustness), both inside `21eb960a2..a6ba7844f` |
| 3 | TUI library tests, 2 tests | runs `37573819816` @ `a61c38ee9` and `37722861763` @ `ff7eb9ab7` | fixed by #1761 = `21eb960a2` |
| 4 | oversized-file ratchet | #1764's run `37727638919`: guardrails failed only at step 11 `Enforce oversized-file ratchet` | **open**, latent behind class 1 |

### Class 1: the drift grew, and #1764 no longer covers it

Hosted output at head (`a6ba7844f`):

```
Diff in .../crates/jcode-provider-openai-runtime/src/lib.rs:966:
Diff in .../crates/jcode-tui/src/tui/app/onboarding_flow_control.rs:633:
Diff in .../crates/jcode-tui/src/tui/app/onboarding_flow_control.rs:874:
```

- `lib.rs:966` (`reload_cached_reasoning_efforts`) is the hunk reported in
  #1740, introduced `bda5f3d9c` (2026-10-06).
- Both `onboarding_flow_control.rs` hunks are **new**: the file changed between
  `21eb960a2` (1752 lines) and `a6ba7844f` (1785 lines), and the rehearsal block
  rustfmt now wants collapsed is the code added by `72faaf145`
  ("onboarding: Alt+5 rehearses the real first-run flow"), one of the 21 commits
  in `21eb960a2..a6ba7844f`.

Consequence for the maintainer's own fix: **PR #1764 no longer restores master's
Format gate.** Its PR run `37727638919` (base `21eb960a2`, head `13375d186`)
passed `Check formatting` because at that base only the `lib.rs` hunk existed.
Its file list does not include `onboarding_flow_control.rs`:

```
crates/jcode-command-risk/src/{assess_tests,lib,tokenize}.rs
crates/jcode-provider-openai-runtime/src/lib.rs
crates/jcode-tui/src/tui/app/tests/remote_startup_input_02/part_01.rs
crates/jcode-tui/src/tui/ui.rs
crates/jcode-tui/src/tui/ui/copy_selection.rs
src/cli/login/google.rs
```

So a re-run of #1764 against current master fails on the two onboarding hunks.
The fix is two more rustfmt hunks (or a rebase plus them), and only then does
class 4 become the next visible gate, exactly as `2026-10-09-master-size-ratchet.md`
records.

### Class 2: two dead-code warnings in `jcode-tui`, fixed by the copy-selection commit

macos job `113149292962` (run `37727625832` @ `21eb960a2`) ends its step 17 with:

```
Warning budget exceeded: current=2 baseline=0
Run scripts/check_warning_budget.sh --update only after intentional cleanup.
##[error]Process completed with exit code 1.
```

The two warnings, earlier in the same log, are:

```
warning: unused import: `display_col_slice`
warning: function `copy_selection_text_from_raw_lines` is never used
warning: `jcode-tui` (lib) generated 2 warnings
```

`671b65131` "fix: include a wide char when a copy selection ends inside it
(fixes #1766)" changes `crates/jcode-tui/src/tui/ui/copy_selection.rs` (+6/-4)
and `crates/jcode-tui/src/tui/ui/display_width.rs` (+53/-1), which is what makes
the import and the function live again; at head both are present and used
(`copy_selection.rs:4` import, `:128` call site). `b4851ae86` "fix: warning gate
fails closed on cargo check errors and prints diagnostics (fixes #1762)" is a
separate gate-robustness fix: the old script swallowed `cargo check`'s exit
status, so a compiler failure without warnings printed
`Warning budget OK: current=0 baseline=0`.

#1764's run at base `21eb960a2` passes step 17, consistent with its own edits to
`copy_selection.rs`; that is a property of the PR's diff, not of master.

### Class 3: the two TUI failures were fixed upstream, not left behind

Failing tests in ubuntu B&T job `113134656858` (run `37722861763` @ `ff7eb9ab7`),
identical at `a61c38ee9`:

```
test tui::app::tests::test_model_picker_preserves_recommendation_priority_order ... FAILED
  panicked at crates/jcode-tui/src/tui/app/tests/remote_startup_input_01/part_02.rs:567
test tui::ui::tests::basic::test_prepare_body_with_math_never_blocks_on_a_stalled_tex_toolchain ... FAILED
  panicked at crates/jcode-tui/src/tui/ui_tests/basic/body_cache.rs:1163
test result: FAILED. 2480 passed; 2 failed; 19 ignored
```

`21eb960a2` is "test(tui): isolate recommendation persistence and cached TeX
probes (#1761)", by Hercules, and it changes exactly those two files
(`part_02.rs` +4, `body_cache.rs` +15). Runs at or after it pass the step:
#1764's run at that base has `Run TUI library tests (Linux only)` = success.
No flakiness hypothesis is needed and none is claimed.

## 2. Fork PR classification (10 open PRs, author `ianalitis`)

No open fork PR currently fails for a reason inside its own diff.

| PR | head | newest CI run | date | failing steps | class |
| --- | --- | --- | --- | --- | --- |
| 1354 | `cea3daac0` | `36228772577` | 09-26 | SSH step x5 | stale infra; also CONFLICTING |
| 1356 | `2f0a99b50` | `36103997043` | 09-25 | SSH x5, Check formatting | stale infra |
| 1357 | `ee9c3d845` | `37689255021` | 10-07 | Check formatting x2, warning budget, TUI tests | refreshed; inherited master debt only |
| 1362 | `2a8c4e8dc` | `35653824363` | 09-21 | none (runs `action_required`) | never ran; CONFLICTING |
| 1493 | `9dc3e5b0c` | `36228847917` | 09-26 | SSH x5 | stale infra |
| 1494 | `75a6c94ee` | `37689253524` | 10-07 | Check formatting x2, warning budget, TUI tests | refreshed; inherited master debt only |
| 1496 | `3a01e8065` | `36228044589` | 09-26 | SSH x5 | stale infra; CONFLICTING |
| 1511 | `6feb8610e` | `36337711541` | 09-27 | SSH x5, Check formatting | stale infra |
| 1512 | `e2be52fd2` | `36233133797` | 09-26 | SSH x5, Require Linked Issue | stale infra + policy gate |
| 1513 | `d4132078d` | `36234531659` | 09-26 | SSH x5, Require Linked Issue | stale infra + policy gate; CONFLICTING |

Merge state now: CONFLICTING/DIRTY = #1354, #1362, #1496, #1513. The other six
are MERGEABLE/UNSTABLE (green mergeability, red checks).

### The SSH class is a pure stale-run artifact

Every affected job fails the same step with the same message (job
`108367974528`, run `36228847917`):

```
##[group]Run webfactory/ssh-agent@v0.9.0
##[error]The ssh-private-key argument is empty. Maybe the secret has not been
configured, or you are using a wrong secret name in your workflow file.
```

Fork `pull_request` runs never receive the secret, so this step failed for every
fork PR. Upstream removed it in `027434399` (2026-10-03, "ci: allow fork PR
builds without an unused SSH deploy key", `.github/workflows/ci.yml` -24 lines),
and `.github/workflows/ci.yml` at `a6ba7844f` contains no `ssh-agent` step at
all. All seven runs above were created before 10-03. A rebase (or a re-run)
clears the class; nothing in those diffs needs changing for it.

### The refreshed pair fails only on master's debt

#1357 and #1494 were refreshed onto `a61c38ee9` on 10-07 and now fail only
`Check formatting` (class 1), `Enforce warning budget` (class 2, already fixed
upstream), and the two TUI tests (class 3, already fixed upstream by #1761).
Refreshing them again is the whole remaining action.

## 3. Fork side: green, and does not carry the drift

- Fork run `37888192151` @ `df9792506`: all ten jobs success, including `Format`,
  `Quality Guardrails` (which runs the ratchet step), and all three
  `Build & Test` legs.
- Our tree passes both tests locally: `cargo test --profile selfdev -p jcode-tui
  --lib -- test_model_picker_preserves_recommendation_priority_order
  test_prepare_body_with_math_never_blocks_on_a_stalled_tex_toolchain` →
  `2 passed; 0 failed` (`scratch/tui-two-tests.txt`).
- Our tree does not contain the new onboarding rehearsal block, so it cannot
  carry hunks 2 and 3; the `lib.rs` call sites are already in the shape rustfmt
  wants. The fork's own `Check formatting` is green on hosted stable.

## 4. The two ratchet baselines are not comparable, and that is fine

Earlier notes framed the fork-vs-upstream baseline difference as a pending
disposition. Measured directly:

| | fork (`HEAD`) | upstream (`4a7400819`) |
| --- | --- | --- |
| tracked entries | 91 | 119 |
| `crates/jcode-base/src/auth/lifecycle.rs` baseline | 1336 | 2768 |
| that file in-tree | 1336 | 3065 |
| `crates/jcode-app-core/src/tool/discover.rs` in-tree | 1680 | 2982 |
| `crates/jcode-app-core/src/server.rs` in-tree | 2067 | 2431 |

The fork's integration line split and shrank those files (`5bc975ef8`,
`0cf9558ac`, and `67cec079b` "chore(ci): refresh the four stale quality ratchets
against upstream"), so its baseline describes its own tree and its gate passes
on it. Upstream's 32 regressions are measured against upstream's own baseline on
upstream's tree. Neither number is evidence about the other, and the fork has no
ratchet action implied. The upstream remedy remains a maintainer call, as
recorded in `2026-10-09-master-size-ratchet.md`.

## 5. Correction to earlier notes

`git merge-base HEAD origin/master` is `a61c38ee9` (2026-10-06); the fork
carries 527 commits master does not, and master carries 24 the fork does not.
`4a7400819` and `21eb960a2` are **not** ancestors of the fork HEAD; they were
used only as comparison revisions (`git show <rev>:<path>`). Any statement that
the fork already contains the 10-07..10-09 master line is wrong.

## 6. Boundary

No issue, PR, branch or check was mutated while producing this receipt. Still
awaiting operator approval: refreshing #1357/#1494 (a push to a PR branch),
refreshing or closing #1354/#1362/#1496/#1513, commenting the ratchet finding on
#692, any baseline PR upstream, and the fork-master sync disposition.

Reproduce independently:

```sh
gh api repos/1jehuang/jcode/actions/jobs/113701122131/logs --allow-escape-sequences
gh api repos/1jehuang/jcode/actions/jobs/108367974528/logs --allow-escape-sequences
gh api repos/1jehuang/jcode/actions/jobs/113149292962/logs --allow-escape-sequences
gh api repos/1jehuang/jcode/actions/jobs/113134656858/logs --allow-escape-sequences
gh api 'repos/1jehuang/jcode/compare/21eb960a2fb9b9ffada350138f01ec1be5c67349...a6ba7844f81ae64ed44ce7285e9341022b4f7796'
gh run view 37727638919 -R 1jehuang/jcode --json jobs
```

Job ids come from `gh run view <id> --json jobs`; the field is `databaseId`,
not `id`, and a job log for a run that is still in progress is empty, so fetch
logs by job id rather than by run.
