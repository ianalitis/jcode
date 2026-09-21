# Fork posture after upstream v0.86.0

**Date:** 2026-09-21. **Tip:** `ed35ce309`, a merge of upstream `e589cbe5a`
(v0.86.0) into the local line at `11ab2d532`. Merge mechanics, per-file
resolution, and the full verification table are in
[`UPSTREAM_MERGE_V0.86.0.md`](UPSTREAM_MERGE_V0.86.0.md); this document is the
durable operating posture that outlives that merge: what the local line keeps,
what it now takes from upstream, how work is routed and verified, and which
harness invariants a change has to respect.

## 1. What the local line is

A fork of `1jehuang/jcode` (`origin`) published as `ianalitis/jcode` (`fork`). It
carries bespoke behavior upstream does not have, plus fixes intended to go back
upstream. History is a record, not something to tidy, so upstream moves arrive as
**merges**, and the local branch keeps its commits.

### Keep the mirror, integration branch, and PR branches distinct

The GitHub fork badge describes the selected remote branch, not the source or
binary currently running locally. On 2026-09-21, `fork/master` was 1,997 commits
behind upstream while the active integration branch was only one documentation
commit behind. Both fork-only commits were already upstream by patch identity:
`e09acaa7a` matches `1d87eadb6`, and `5cb7b3dad` matches `ba900d276`.

The approved reconciliation later that day published `dc12efa7a` to `fork/master`.
It is 0 behind and 3 ahead by ancestry, with a tree identical to upstream
`2a4edaa02`. The extra ancestry preserves history only. Future mirror checks
should verify both upstream ancestry and `git diff origin/master fork/master`,
not demand zero ahead commits or force-reset the preserved merge. See the
[publication receipt](UPSTREAM_RECONCILIATION_2026-09-21.md).

- `origin/master` is the fetched upstream reference. Local `master` is its
  fast-forward-only mirror, with no local feature commits.
- `jcode/ci-format-baseline` is the existing integration line, not an upstream
  PR branch. Preserve its history and merge reviewed upstream updates here.
- `pr/*` branches contain one issue's contribution on a recent upstream base.
  Port only the relevant fix and tests from the integration line. Never merge
  the integration line into a contribution branch to make a review correction.
- `fork/master` must be checked separately. Fetching or updating local `master`
  does not update GitHub. Publishing requires operator approval. Existing
  divergent history must not be reset or force-pushed as automatic cleanup.

At the start of an integration or contribution task, inspect the lease and all
worktree status, refresh both remotes without pruning, then record these counts:

```sh
scripts/bounded.sh 90 git fetch origin
scripts/bounded.sh 90 git fetch fork
git rev-list --left-right --count origin/master...HEAD
git rev-list --left-right --count origin/master...master
git rev-list --left-right --count origin/master...fork/master
git cherry origin/master fork/master
```

Each count is **behind, ahead**, in that order. Do not substitute the current
branch's configured tracking ref for `origin/master`. When diverged commits are
patch-equivalent, verify their upstream counterpart before proposing a sync.
Recheck these refs before an approved push so a stale plan cannot overwrite
someone else's update. Record the runtime build separately with `selfdev status`:
a source merge alone does not deliver features to the running daemon.

## 2. Upstream behavior adopted because it solved the same problem better

The v0.86.0 merge dropped several local patches in favor of later upstream work.
The rule for the next merge is the same: when upstream solves the same problem
later or better, take upstream and delete our version.

| Adopted | Replaced locally |
| --- | --- |
| Prompt overlay dedupe | our `d834ffc05`, a fix for upstream #1092 that v0.85.0 shipped |
| `dev_cargo.sh` physical-path cwd guard | our equivalent guard |
| `reapply_terminal_modes_to(.., focus_change)` + tmux extended keys | our narrower terminal-mode reapply |
| Auth-route-aware `cache_ttl_is_estimate` | our module-local copy |
| No eager embedding preload at startup | our memory-enabled gate around it |
| Tightened todo ownership and confidence gates | our looser actionability gate |
| Stricter Conifer catalog (undocumented route excluded) | our window fallback |
| `ServerEvent::TextDone`, `Subscribe::supports_pdf_panels` | our ad-hoc completion signalling |
| Alias-aware MCP dispatch | our exact-name dispatch |
| Shared OpenAI usage accounting | our per-call accounting |

## 3. Bespoke behavior this fork keeps

| Kept | Why | Location |
| --- | --- | --- |
| Ancestor-layered `AGENTS.md` loading | project instructions compose from repository ancestors outward to global | `jcode-base/src/prompt/` |
| Atomic background status writes | a reader must never see a truncated `<task>.status.json`, which made `bg wait` report a running task as missing | `jcode-base/src/background.rs` |
| Truncated tool output spilled to disk | long output stays inspectable instead of being dropped | `jcode-app-core/src/agent/tool_output_spill.rs` |
| Bounded `list_models` | provider catalogs cannot balloon a tool result | `jcode-app-core/src/tool/` |
| Included-subscription route pinning | premium work uses the pinned included subscription route, never a silent metered or account substitute | `jcode-base/src/provider/`, `jcode-app-core/src/server/provider_control*` |
| Fail-closed session tool-policy dispatch | an agent turn without a registered policy is denied rather than dispatched | `jcode-app-core/src/tool/session_policy_dispatch*` |
| Split test layout | the oversized-file and oversized-test ratchets are gates, so new tests land in `*_part_N_tests.rs` files | `crates/*/src/**/*tests*.rs` |
| Privacy removals | no feedback tool, no todo telemetry collection | fork-only deletions, kept dropped through the merge |

`docs/FORK_CI.md` covers the fork-specific CI surface.

## 4. Routing and configuration posture

Configuration is user-owned and deliberately lives outside this repository:

| Surface | Path | Owns |
| --- | --- | --- |
| Runtime config | `~/.jcode/config.toml` | default provider/model, swarm model and effort, provider blocks |
| Provider policy | `~/dotfiles/policy/providers.md` | lane defaults, effort discipline, admission rules |
| Swarm prompt | `~/.jcode/swarm-prompt.md` | the generated prompt workers receive |
| Measurement receipts | `~/dotfiles/docs/measurements/` | per-lane evidence, including `2026-09-20-opencode-go-glm-reroute.md` |

The repository must not carry secrets, and must not ship a default that silently
overrides the user's file.

**Default inner-loop workhorse:** `opencode-go:deepseek-v4.1-flash` as
`[agents] swarm_model` while its special pricing runs (to 2026-09-27), with
`[provider] default_model = "glm-5.3-flash"` as the interactive default. The lane
table is owned by `~/dotfiles/policy/providers.md` and is not duplicated here;
corrected 2026-09-21 after the Fable review found this row stale (rows here had
named `minimax-m2.7`, which returns HTTP 500 through this route).

One frozen treatment per task: model, effort, and tool surface are fixed before
execution, routing is sticky per job rather than per turn, and a quota stop hands
off visibly instead of triggering a hidden provider or account swap.

Before dispatching on the Go lane, check it rather than assuming:

```sh
jcode usage -p opencode-go --json      # key status and local spend
jcode model list -p opencode-go        # live catalog (36 models at writing)
```

Known lane constraint: `opencode-go` is declared as a single OpenAI-compatible
profile, but Go serves Anthropic-shaped models (MiniMax, Qwen) through
`/messages` and answers an opaque 500, while GLM models work. Packet:
`docs/upstream-feedback/2026-09-21-go-anthropic-shaped-models-500.md`. Keep the
session header enabled: Go validates jcode v0.81.6 or newer.

## 5. Verification posture

`scripts/check_guardrails.sh` is the board, and it mirrors the CI quality job:
module declarations resolve, `cargo fmt --all --check`,
`cargo check --all-targets --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, `Cargo.lock` is up to
date, the warning budget, the code/test size ratchets, the panic, swallowed-error
and wildcard-reexport ratchets, crate dependency boundaries, and the onboarding
state-space invariants. `cargo machete` is optional and only runs when installed.
`--skip-slow` exists for a fast pass, not full acceptance. `--fix` regenerates
budget files as well as formatting; never run it automatically to make a failed
gate green. Baseline changes require separate approval and evidence of an
intentional cleanup, not suppression of a newly observed failure.

Evidence standard: reproducible commands, before/after numbers for a fix, and the
fully qualified test name as the validation selector. Line numbers are hints, not
identifiers.

Board after the v0.86.0 merge: guardrails all green; `jcode-base --lib` 1527
passed, `jcode-app-core --lib` 1517, `jcode --lib` 285,
`jcode-provider-core` 134, `jcode-provider-openrouter-runtime` 184, and
`jcode-tui --lib` 2349 with six failures that are the pre-existing parallel
races, all six green in isolation.

## 6. Test-harness invariants

The test suites run in parallel against process-global state: `JCODE_HOME`, the
config cache, render state, reload markers, and provider auth overrides.
`jcode_base::storage::lock_test_env` is the coordination point.

- The lock is **not reentrant**. A thread that calls `lock_test_env()` while it
  already holds it deadlocks the whole suite, and the stuck thread looks exactly
  like an ordinary waiter (`__psynch_mutexwait`, no CPU). Take the lock once per
  test. A helper that may be reached from an already-locked test must degrade to
  `try_lock` and say why, as `ensure_test_jcode_home_if_unset` does.
- Never hold the lock while waiting on a thread that itself needs it.
- Two incidents are on record: the 2026-09-01 ABBA deadlock between the env lock
  and the render-state lock (fixed by lock ordering, a `try_lock` degradation,
  and a reentrant render guard; `docs/TUI_TEST_FLAKINESS.md`), and an
  2026-09-21 env-lock-only hang where eleven test threads sat in
  `__psynch_mutexwait` with no visible holder, consistent with either a
  re-entrant acquisition or a leaked guard.
- The wait is now bounded: `lock_test_env` warns after 10s and fails after 300s
  (`JCODE_TEST_ENV_LOCK_TIMEOUT_SECS` overrides), naming the last successful
  acquirer. A repeated deadlock therefore fails loudly with a culprit instead of
  hanging a session.
- Known flakiness (closed 2026-09-21, see `TUI_TEST_FLAKINESS.md`): the picker
  family holds the env lock and frame metrics are per-thread under test; what
  remains is a small timing-assertion tail. `tests/remote_events_reload_05.rs` and
  `tests_input_scroll.rs` take no env guard and write reload state under the
  process-wide `JCODE_HOME`, so parallel tests redirect each other. The intended
  fix is a per-test temporary `JCODE_HOME` for those files; a blanket lock
  serialized the suite from about 27s to over 10 minutes and was reverted.

## 7. Contributing back

### Automatic source-agent preparation

The repository's [`AGENTS.md`](../AGENTS.md#contribution-preflight-discover-before-implementing)
owns the contribution preflight. Jcode loads it into fresh source sessions,
including self-dev workers. `CLAUDE.md` imports the same file instead of owning a
second workflow. The preflight discovers relevant retained work and review
feedback before proposing code; it does not run a periodic audit or grant public
effect authority. Keep current receipt/ledger links in `docs/README.md` so the
instructions do not need a new hard-coded handoff for every iteration.
This is persistent agent guidance, not a deterministic gate that blocks a
skipped preflight.

AGENTS content is captured per session for a stable prompt prefix. Start a fresh
source session to pick up an instruction edit; no binary rebuild or daemon reload
is needed. Existing sessions do not silently adopt edits mid-turn. A linked
worktree loads its own branch's `AGENTS.md`, not the integration checkout's copy.
When preparing work there, carry this preflight in its explicit task packet if
that branch predates the instruction change. Missing fork-only records can be
read from the integration checkout located with `git worktree list`, without
switching branches or assuming its dated claims are current.

Verified 2026-09-21 without rebuilding: a fresh source worker with no tools
received and quoted the preflight from its injected project instructions.
Existing regressions `full_and_split_prompt_builders_select_the_same_ancestor_layers`
and `worker_context_receipt_policy_snapshot_stays_stable_then_fresh_spawn_invalidates`
both passed. These prove instruction loading and snapshot behavior, not that
every future model will follow every preparation step.

`CONTRIBUTING.md` requires an issue first and a PR that links it. Reproducible
packets for upstream are staged under `docs/upstream-feedback/`, each with
expected versus observed behavior, a minimal reproduction, a proposed fix, and
the evidence commands:

| Packet | Issue | Pull request | Subject |
| --- | --- | --- | --- |
| `2026-09-21-clippy-1.98-lint-drift.md` | [#1348](https://github.com/1jehuang/jcode/issues/1348), [#1294](https://github.com/1jehuang/jcode/issues/1294) | [#1354](https://github.com/1jehuang/jcode/pull/1354) | lint and rustfmt drift under clippy 1.98, plus the `dev-bins` bench build |
| `2026-09-21-test-env-lock-can-hang-the-suite.md` | [#1349](https://github.com/1jehuang/jcode/issues/1349) | [#1356](https://github.com/1jehuang/jcode/pull/1356) | the shared test-env lock can wedge a test binary forever |
| `2026-09-21-background-wait-torn-status-read.md` | [#1350](https://github.com/1jehuang/jcode/issues/1350) | [#1357](https://github.com/1jehuang/jcode/pull/1357) | `bg wait` reports a running task as missing during a status write |
| `2026-09-21-jev-mock-server-nonblocking-read.md` | [#1351](https://github.com/1jehuang/jcode/issues/1351) | [#1355](https://github.com/1jehuang/jcode/pull/1355) | the jev mock server reads a non-blocking socket on macOS |
| `2026-09-20-swarm-stop-queued-worker.md` | [#1352](https://github.com/1jehuang/jcode/issues/1352) | [#1362](https://github.com/1jehuang/jcode/pull/1362), from `pr/swarm-stop-quiesce` | stop acknowledges before owned work is cancelled; ported to upstream and opened 2026-09-21 (5/5 regressions, 0/4 pre-fix control), then extended to route debug-queued interrupts through the closed delivery gate after review; re-review 5/5 with no findings |
| `2026-09-21-go-anthropic-shaped-models-500.md` | comment on [#1224](https://github.com/1jehuang/jcode/issues/1224) | not needed | Go serves Anthropic-shaped models through an OpenAI-compatible profile |
| `jcode-base --lib` parallel failures | [#1358](https://github.com/1jehuang/jcode/issues/1358) | [#1360](https://github.com/1jehuang/jcode/pull/1360) | eleven failures; re-check showed code-vs-test drift plus two environment leaks, not thread races |
| two unused test-module imports blocking `-D warnings` | [#1363](https://github.com/1jehuang/jcode/issues/1363) | [#1364](https://github.com/1jehuang/jcode/pull/1364) | `Mutex`/`OnceLock` unused in two `jcode-app-core` test modules, outside #1348's list |

Each packet carries the reproduction, the proposed fix, and the evidence. The four
pull requests are pushed from `fork` and based on `origin/master` with only their
own change. Open further upstream work only with operator approval.

Hard stop: nothing is pushed, published, deployed, or installed without explicit
operator approval, and the running daemon is only reloaded when asked.

## 8. Open items

- The env-lock holder-lifetime and contention regressions are fixed locally and
  published in #1356. Do not restart an unbounded re-entry hunt from the old
  incident alone. A recurrence needs a fresh bounded reproduction and named holder.
- Give `tests/remote_events_reload_05.rs` and `tests_input_scroll.rs` their own
  temporary `JCODE_HOME` so the parallel flakiness ends.
- Send the staged packets upstream once the operator approves opening issues.
- Keep the token-economy follow-ups (`docs/plans/TOKEN_ECONOMY_PLAN.md`,
  `docs/HARNESS_LOOP_ARCHITECTURE.md`) in step with the merged tree.
