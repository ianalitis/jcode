# Architect review: fork posture and phased execution, 2026-09-21

Stage A output per `docs/HANDOFF_2026-09-21_FABLE_ARCHITECT_PHASE_PLANNING.md`.
Read-only review; nothing implemented. State re-verified at 03:02 UTC: HEAD
`3dac3022f`, tree clean, 0 behind `origin/master` (`e589cbe5a`), running binary
`v0.86.211-dev (5e08d2cfc)`, all four PRs show only Greptile (SUCCESS), no
Actions run. Lane windows: Claude 7-day Fable 60%, ChatGPT 7-day 30%, OpenRouter
$92.24 balance, Go key valid.

Companion files: `PHASE_1_PICKER_FAMILY_ISOLATION.md`, `PHASE_2_RENDER_STATE_THREAD_LOCAL.md`,
`PHASE_3_UPSTREAM_BASE_SUITE_PR.md`, `PHASE_4_ROUTING_MEASUREMENT_CONTRACT.md`,
`2026-09-21_PHASE_ROUTING.md`. Execution axes, ladder, and the admission to gate
sequence are as defined in `docs/HARNESS_LOOP_ARCHITECTURE.md`; not restated.

## 1. Posture review

The fork is in the best shape it has been: merge landed, gates green locally,
upstream contributions filed with reproductions. Two things erode that from
underneath, and both are test-harness problems, not product problems:

1. `jcode-tui --lib` is red 6 of 6 parallel runs (2 to 7 failures). Every
   verification claim about the TUI currently carries an asterisk.
2. Process-global test state (`JCODE_HOME`, config cache, render state, provider
   catalog cache) is coordinated by one non-reentrant mutex plus `try_lock`
   degradation. The degradation is documented and deliberate, but it is exactly
   what produces family (b) below.

Everything else in the eight workstreams is either done, waiting on other
people (PR review), or a measurement question that should not consume
implementation budget until 1 and 2 are closed.

## 2. Workstream verdicts

| WS | Verdict | Reason |
| --- | --- | --- |
| W1 upstream follow-through | **wait, then one bounded packet** | Four PRs need reviewer action, not ours. #1352 and #1358 are the only new work (Phase 3). |
| W2 TUI/app-core determinism | **Phase 1 and Phase 2. Highest value.** | Two distinct families with distinct fixes; do not conflate them in one packet. |
| W3 env-lock re-entrancy | **stop hunting** | The wedge did not recur in six bounded runs; the lock now names its holder on recurrence. Hunting 186 call sites for a bug that reports itself is negative return. Reopen only when a bounded run names a culprit. |
| W4 base-suite isolation upstream | **Phase 3, contingent on operator branch approval** | Fix already exists in this fork (1530/1530); the packet is a port, not a design. |
| W5 harness/token economy | **Phase 4, docs only** | One accepted cycle exists. What is missing is a measurement contract, not more machinery. No code. |
| W6 routing re-measurement | **fold into Phase 4** | Two dated triggers (Go pricing 09-27, ZDR 09-30) need a calendar, not a phase. |
| W7 merge cadence | **rule, not phase** | Merge when upstream tags a release or when a PR of ours lands, whichever first. Duplicate-fix rule already in FORK_POSTURE §2: take upstream, delete ours. |
| W8 privacy posture | **rule, not phase** | Add a grep-level check to the next merge receipt template; no CI gate until a second regression occurs (ADR 055 rule). |

## 3. Evidence review of W2 (the phase that matters)

Verified against source this session:

- **Family (a), render state.** `create_test_app` resets process-global render
  state via `clear_test_render_state_for_tests` under a `try_lock` that
  deliberately declines when contended (`ui.rs`, `render_state_test_lock`). The
  changelog test loses 5 of 6 runs. `TUI_TEST_FLAKINESS.md` already names the
  right fix: make render state thread-local. That doc's option 2 (skip the reset)
  needs an 810-site audit and is worse.
- **Family (b), picker/catalog.** Confirmed shape at
  `state_model_poke_03.rs:559`: `ensure_test_jcode_home_if_unset()` then
  `clear_persisted_test_ui_state()` then render clear, then assertions on
  `model_picker_cache` and catalog contents. No env lock for the test's duration.
  Twelve named tests, three files (`state_model_poke_02*`, `state_model_poke_03*`,
  `shortcut_hints`). Taking `lock_test_env()` for only this family adds a
  bounded serialization: 12 tests, each sub-second, so the runtime cost is
  seconds, not the 10 minutes the blanket lock cost.
- **`jcode-app-core` spill test** (`tool/tests.rs:884`) already holds the env
  lock and sets a temp `JCODE_HOME`. It flakes because *siblings* that touch
  `JCODE_HOME` do not lock. Same family (b) pattern, different crate. Include it
  in Phase 1 as a reader question, not a writer task, until a sibling is named.

Missing evidence, stated plainly: nobody has measured whether family (b) is
purely `JCODE_HOME` (fixable by lock) or also the in-process provider catalog
cache (fixable only by a per-test reset). Phase 1's first step is the six-run
measurement with the lock added; if failures persist, the cache reset is the
second iteration, and that is the iteration cap.

## 4. Sequencing argument

```
Phase 1 (picker lock, small)  ──┐
                                ├─► green parallel TUI suite ─► every later "verified" claim is honest
Phase 2 (render thread-local) ──┘
Phase 3 (upstream port of env guard) ── independent; needs branch approval; benefits from Phase 1 helper shape
Phase 4 (measurement contract, docs) ── independent; cheapest lane; no gate dependency
```

Phase 1 unlocks the most per token: three files, a known helper, a measurable
number (failures per six runs). Phase 2 is the structural fix and carries
regression risk in production render code, so it goes second, after Phase 1
proves the measurement loop works. Phases 3 and 4 can run in parallel with 1
and 2 on different lanes because they touch disjoint paths (a worktree, and
docs respectively).

Do not run Phase 1 and Phase 2 concurrently in the same tree: both touch
`support_failover/part_01.rs`, and the failure counts would be unattributable.

## 5. Criticism (what is over-engineered, what could regress)

- **Over-engineered:** any proposal to make `lock_test_env` reentrant. Forty
  annotated sites and a new guard type to fix a bug that has not recurred and
  now self-reports. Rejected.
- **Over-engineered:** re-measuring every lane on a schedule. Measure when a
  decision depends on it (Phase 4 lists exactly three triggers).
- **Regression risk, Phase 2:** production has one render thread, but any
  helper thread that reads render state (flicker detection, layout snapshots
  consumed from a background task) would silently see an empty thread-local.
  The packet must grep for every reader of those statics before switching.
  Abandon Phase 2 if a non-render-thread reader exists and cannot be routed
  through a message; fall back to a narrower fix (reset only the flicker
  history in `create_test_app`).
- **Regression risk, Phase 1:** a test in the family that transitively calls a
  helper which also takes the env lock would now self-deadlock, and the
  bounded wait turns that into a 300s failure per test. Run with
  `JCODE_TEST_ENV_LOCK_TIMEOUT_SECS=20` during the packet.
- **Missing evidence, Phase 3:** we have not confirmed upstream's eleven
  failures are the same eleven with our guard applied on a pristine checkout;
  the PR body must include that A/B or it will not survive review.
- **Abandon conditions** are in each phase file. Common one: if the phase
  needs a second iteration beyond its cap, stop and return the diff and the
  numbers; do not widen.

## 6. Delete or stop doing

1. Stop the env-lock re-entrancy hunt (W3). Keep the diagnostic.
2. Stop blanket-lock experiments; two are on record, both reverted.
3. Stop re-verifying lane windows in every handoff unless a dispatch decision
   follows. Cite the last measurement path instead.
4. Retire the historical lane proposals in `TOKEN_ECONOMY_PLAN.md` below the
   2026-09-20 amendment into an appendix or delete them at the next docs pass.
   The amendment is the contract; the rest is noise for a fresh reader.
5. Do not open more upstream issues until at least one of #1354 to #1357 gets a
   maintainer response; the signal-to-noise on the upstream side matters.
6. No new router, ledger, dispatcher, or scheduler in any phase. Already
   rejected in `HARNESS_LOOP_ARCHITECTURE.md §8`; restated so a worker does not
   reinvent one under a different name.

## 7. Decisions that belong to the operator

| # | Decision | Needed by |
| --- | --- | --- |
| O1 | Approve committing Stage A docs on `jcode/ci-format-baseline` | now |
| O2 | Approve a worktree and branch off `origin/master` for the #1358 port (Phase 3) | before Phase 3 |
| O3 | Approve pushing that branch to `fork` and opening the PR | Phase 3 end |
| O4 | Whether to reload the daemon after Phase 1/2 land, or leave it on `5e08d2cfc` until the next merge | after Phase 2 |
| O5 | D2 from the harness architecture: create a capped OpenRouter inference key (smallest real spend enforcement). Without it, metered dispatch stays read-only packets | before any metered Stage B worker |
| O6 | Go special pricing ends 2026-09-27 and ZDR expires 2026-09-30: re-verify or re-route. Not a fork decision | calendar |
| O7 | Whether a red upstream CI (currently red at `e589cbe5a`) blocks our merge cadence rule, or we merge tags regardless and fix forward as in v0.86.0 | before next merge |
| O8 | Whether #1352 (swarm stop) deserves a PR now or waits for a maintainer reply on the existing four | Phase 3 scope |
