# Handoff: the decision contract, the local arm, and the line's current state

**Snapshot:** 2026-09-22, prepared on `jcode/ci-format-baseline` with a clean tree at
`origin/master...HEAD` = 0 behind, 327 ahead (later documentation commits do not change
anything described below),
runtime `v0.86.234-dev (a61ab0927)` (the daemon is unchanged; none of this work needs
a rebuild). This is a handoff, not a policy and not an authorization. Every number was
measured; where something is unverified it says so.

## Paste into the fresh session

> Read `docs/HANDOFF_2026-09-22_DECISION_ARM_AND_LINE_STATE.md`, then
> `docs/plans/2026-09-22-LAYA_LOCAL_DECISION_ARM.md`. The contract (W1/W2/W5) and the
> W3 transport are implemented and tested; the install was approved and performed, and
> its first run is measured in the plan's §9. That run is a **negative result**: the base
> checkpoint scored 4/10 against the baseline's 5/10 and named a forbidden option, so the
> arm reports and must not gate. Do not install anything further, fetch another checkpoint,
> or promote the daemon without an explicit approval naming it. Work in
> `/Users/ianalitis/.jcode/source/jcode`. Fix demonstrated causes rather than adding
> process, keep a short native todo queue, and report at each checkpoint in fewer than
> five lines: what improved, what remains, the next bounded action.

## 1. Authority and effect gates

Standing, unless the operator says otherwise:

- **Approved and exercised:** pushing focused `pr/*` branches to `fork` and opening the
  upstream PR they correspond to; merging upstream into the integration line and
  publishing that to the fork's default branch; comments that answer a review or record
  a scope change; closing our own duplicate PRs and issues with evidence.
- **Not approved:** installing anything (including laya and its Python stack), changing
  providers or auth, promoting the shared daemon, force-pushing any default branch,
  deleting other people's branches, mutating upstream issues beyond comments, and any
  release action.
- **Never ours to do:** merging upstream (`1jehuang/*`) PRs, dismissing upstream
  alerts, or changing upstream repository settings.

## 2. Current state, measured

### Repository and refs

- The tree is clean on `jcode/ci-format-baseline`; the write lease is free.
- `origin/master` is `ef4c2bd69`. `origin/master...HEAD` is **0 behind, 324 ahead**; the
  new decision commits sit on top of the integration line, not on a branch.
- `fork/master` is `e7fb1bf88`, **0 behind, 11 ahead** of upstream, 15 files: the fork's
  CI guards, its quarantine list and the two deltas that mirror open upstream PRs
  (#1354, #1373). The `protect default branch` ruleset (id 23795856) blocks deletion and
  non-fast-forward, nothing else.
- Sibling forks are all 0 behind their upstreams and ahead only by their recorded CI
  repair: `handterm` 4, `mermaid-rs-renderer` 3, `agentgrep` 2, and `ianalitis/GLOOP` is
  identical to `redacktion/GLOOP` (its GitHub parent is gone, so there is nothing to
  sync and no upstream to contribute to).
- Our open upstream PRs: **13**, all `MERGEABLE`, each linked to its own issue. #1379
  was withdrawn as a duplicate of `costajohnt`'s #1378 and its branch deleted; the tip
  is archived locally at `refs/archive/local-2026-09-22/pr-ctrl-up-preserve-draft`.
- Local inventory: 20 branches, 14 worktrees, 18 local archive refs (116 across all
  remotes). Two worktrees (`pr-swarm-stop-quiesce`, `fork-master-ci`) carry history, not
  live work.

### Commits this session, oldest first

| Commit | What |
| --- | --- |
| `c44fca71f` | `FORK_POSTURE.md`: the default branch diverges deliberately, on all four forks |
| `d2bed6bda` | Phase 4's acceptance closed; the `mlx-serve` row named a command that does not exist |
| `4e9bc1b04` | upstream `ef4c2bd69` merged into the integration line |
| `6d713007e` | resync receipts, the withdrawn duplicate, the sibling-fork posture |
| `e1dd4c631` | **W1/W2**: the typed decision contract, and the routing Jev consumer key |
| `47e584dc2` | the contract's fixture harness and the measured baseline floor |
| `2760d104a` | this handoff, and the laya W3 plan |
| `371173c5c` | the runner's home narrowed with evidence; the holdout protocol extended to decisions |
| `16a362309` | two of the three gates resolved by measurement (cp314 wheels exist; the ceiling's real source) |
| `63917d89d` | **W3**: the arm's process boundary, its boundary tests, and the first measured run |

### The decision arm, as implemented

- `crates/jcode-s1-laya-runtime`: the process boundary. One short-lived child per batch,
  JSON lines, std-only, depends only on `jcode-s1-eval`. The child
  (`python/laya_arm_child.py`) owns the forward pass and no policy; the parent owns
  validation, the wall-clock bound, the environment and the fail-closed decision.
- `python/stub_arm_child.py` lets the 11 boundary tests run with no torch: the allowlist,
  one-child-per-batch, timeout, crash, garbage output, an out-of-set option, a child that
  summarises early, and ceiling enforcement are all asserted against it.
- Installed with approval: venv at `~/.jcode/local-arms/laya/venv` (922 MB), pinned
  `torch` 2.14.0 / `transformers` 5.17.0 / `laya` 0.3.5, weights 842,609,210 bytes in the
  Hub cache. Measured: mps, 25 s load, 41-97 ms per decision, peak RSS 3.13-3.69 GB,
  **4/10 correct, 0 invalid, 1 critical**. The arm does not beat the rule baseline.

### The decision contract, as implemented

- `crates/jcode-s1-eval/src/decision.rs` (+ `decision_tests.rs`, + 431 lines total with
  the fixture set). Re-exported from the crate root as `validate_decision`,
  `validate_decision_request`, `score_decisions`, `bundled_decision_fixtures`.
- `DecisionRequest` / `DecisionResult` with `Noul` / `Choice` / `Score`; a closed 2..=16
  option set of described ids; `authority` admitted only as `advisory`; a `prompt_sha256`
  that binds kind, criterion, option ids, descriptions and their order; a `threshold`
  refused unless `calibration_ref` is something other than `"uncalibrated"` (W5 at the
  schema); abstention first-class and never an error; literal grounding shared with the
  classifier as `is_grounded`.
- `DecisionArm`, `DeterministicDecisionBaseline`, and `score_decisions` over
  `fixtures/decisions.dev.json` (10 synthetic cases: 5 choice, 3 noul, 3 score, an
  injection case with a forbidden option, and two abstention cases).
- **Measured baseline fit: 5 of 10 correct, 1 abstention (the correct answer), 0
  invalid, 0 critical**, pinned in the test so a real arm's improvement is visible.
- `crates/jcode-base/src/jev.rs`: `JevPurpose::Routing` with its own
  `JCODE_ROUTING_JEV_PROVIDER`, capability `routing_jev`, `JevClient::for_routing()`, and
  a 16-option cap on routing choice/score questions. Memory and browser are unchanged and
  pinned by tests. **No caller exists yet** until a cloud arm is admitted (D3).
- Evidence: 23 tests in `jcode-s1-eval`; 42 in the `jev` filter plus an end-to-end
  routing-resolution test; clippy clean on `jcode-s1-eval --all-targets` and
  `jcode-base --lib --tests`.

### Gate state

- `cargo test -p jcode-base --lib -- --test-threads=1`: **1558 passed, 0 failed, 5
  ignored**. This is the corrected result; see §5 for what looked like three real
  failures and was not.
- `cargo test -p jcode-tui --lib -- --test-threads=1` still has the failures owned by
  upstream issues (#1340/#1344, #1367/#1368, #1339, #1342) plus four macOS-only
  `Alt`-label tests the fork's CI gates on Linux. That is documented, not new.
- `scripts/check_guardrails.sh --skip-slow` on the integration line fails two ratchets
  (code size: `jev.rs` new oversized at 1407 LOC, `navigation.rs` 2003 -> 2066,
  `ui.rs` 3626 -> 3627; swallowed errors: `dot_ok` 1206 -> 1208, `ui/url.rs` 2 -> 4).
  **These belong to the concurrent session's live files** (`navigation.rs`, `ui/url.rs`,
  and its new `jev.rs`), were recorded rather than repaired, and are still open. The
  same person's uncommitted work was in those files during the day, so coordinate before
  touching them. The decision work adds **zero** growth to either ratchet, and the
  panic-prone ratchet now **passes** where it previously failed at 88 -> 89: the
  transport's own `let _ =`, `.ok()` and `unwrap_or_default()` sites and the contract
  crate's two were removed rather than baselined.
- `cargo clippy -p jcode-base --all-targets -- -D warnings` fails on
  `examples/nari_pcm.rs` (`chunks_exact`), a pre-existing finding our upstream PR #1354
  covers. Use `--lib --tests` to check that crate's code paths.

## 3. What the next session should do, in order

1. **Read the plan's §9 before touching the arm.** W3 is implemented and measured, and the
   measurement is negative: the base checkpoint scores 4/10 against the baseline's 5/10 and
   names a forbidden option. Any claim that the arm helps is unsupported today.
2. **The three bounded next steps are in the plan's §9**, in order: a dev-only calibration
   pass, the `typed-decisions` subfolder as a comparison arm (one more 843 MB fetch, needs
   approval), and D2 before any quality claim.
3. **Name the footprint ceiling.** Measured peak RSS is 3.13-3.69 GB; the shipped default is
   4 GB and the plan proposes 5 GB. One number from the operator closes it.
4. **W4** (the eligibility input, `effective_class = max_restrictive(...)` feeding
   `DataClass::is_remote_eligible`) and **H3** (typed pre-execution risk gating in
   `pre_tool`) both sit on the same contract and are the next two uses of it after W3.
5. **The PR queue.** 13 PRs await upstream review; nothing is pending on our side.
   Re-check states before assuming nothing moved, and re-check for a competing PR on the
   same issue *immediately before* pushing, because #1378 was opened one minute after the
   last check and a duplicate was avoided only by closing ours.
6. **The two failing ratchets** in §2 are the concurrent session's; agree ownership
   before touching them.

## 4. Traps that cost time this session

- **A stale *dependency* artifact, not a stale test binary.** Three `jcode-base` tests
  (`gateway_defaults_to_loopback`, `discovery_is_disabled_by_default`,
  `sponsors_settings_serialize_without_changing_operator_intent`) failed because the
  compiled `jcode-config-types` rlib predated the `security(gateway): default remote
  access to loopback` change. Touching the *dependent* crate's sources did not fix it;
  `touch crates/jcode-config-types/src/lib.rs` did, and the suite went green
  immediately. When a default looks wrong in a test but right in the source, suspect the
  dependency's artifact before writing a bug report.
- **`touch` creates files.** `touch crates/jcode-base/src/config_file.rs` made a 0-byte
  stray at that path (the real file is `src/config/config_file.rs`). It was removed; do
  not touch paths you have not listed.
- **`scripts/bounded.sh` only exists on the integration line.** From a worktree based on
  `origin/master`, call it by absolute path or it silently does not run.
- **A wedged `cargo test` from 2026-09-21 15:40 is still alive** in
  `worktrees/pr-swarm-stop-quiesce` (plus its two parent shells, ~17 hours old, under a
  second of CPU each). Do not use that worktree's `target/` as a shared
  `CARGO_TARGET_DIR`.
- **`grep -h '^test .* FAILED'` also matches `test result: FAILED`.** Filter
  `^test result` out before counting failures, or the totals are off by one per run.
- **The evidence store for `webfetch` is full** (32 snapshots). Use `curl` for public
  reads until the operator clears it.

## 5. Corrections to earlier statements

- The three `jcode-base` failures I first reported as genuine regressions on our line
  were **stale-artifact phantoms**, resolved by rebuilding `jcode-config-types`. The
  control that established it is in §4. No code change was needed and none was made.
- `FORK_POSTURE.md` and `FORK_BRANCH_CLEANUP_2026-09-22.md` described `fork/master` as a
  pristine mirror whose `git diff origin/master fork/master` should be empty. Both are
  corrected: the divergence is deliberate, 10 commits and 15 files, and resetting it
  would delete the fork's CI work.
- The `2026-09-22` dependency receipt's acceptance of seven `windows-sys` edges moving
  backwards applies to the local commit only; those edges are deliberately absent from
  PR #1375, which also now carries the SDK lockfile's `fast-uri` rise.

## 6. Open decisions, all the operator's

| Decision | Why it blocks work | Where it is recorded |
| --- | --- | --- |
| Where the local arm's process runner lives | **Closed**: built as the new leaf crate `jcode-s1-laya-runtime`, with the ownership-doc exception recorded | `docs/plans/2026-09-22-LAYA_LOCAL_DECISION_ARM.md` §4 |
| The footprint ceiling number | **Proposed as 5 GB** after measuring 3.13-3.69 GB; 4 GB is the shipped default. Still needs one number from the operator | same, §8 |
| Approve the laya install (venv, torch, transformers, weights) | **Approved and done 2026-09-22.** Inventory and measurements in the plan's §9 | same, §5 and §9 |
| D1 / D2 / D3 (contract admission, a fresh adjudicated holdout, cloud-arm admission) | Any quality claim depends on D2 | research note §9 |
| Dismiss the fork's 166 triaged CodeQL alerts | They hide real findings, but dismissal is a security judgement | `docs/upstream-feedback/2026-09-22-codeql-rust-alert-triage.md` |
| One Actions-tab click on `handterm` and `mermaid-rs-renderer` | Their push and PR triggers stay inert until then | `FORK_POSTURE.md` §1 |
| Q1 / Q2 (routing thresholds, the `wall_ms` boundary) | The routing contract's numbers cannot be written | `docs/plans/PHASE_4_ROUTING_MEASUREMENT_CONTRACT.md` |

## 7. Method notes worth reusing

- **Measure the claim the packet hands you.** laya's facts were checked against its own
  README, which also yielded the finding the packet missed: its documented weakness
  begins above ~20 options, so our 2..=16 cap sits inside its design point.
- **A control run beats an argument.** Stashing the change and re-running the same three
  tests is what separated "my regression" from "stale artifact".
- **Pin the floor you measured.** The baseline's 5/10 on the dev set is asserted, so the
  next arm's improvement is visible in one number rather than in prose.
- **Yield to the earlier contributor.** #1378 appeared one minute after the duplicate
  check; the right move was to close ours, delete the branch, and move the review work to
  theirs, with the credit and the record intact.
