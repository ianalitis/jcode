# Handoff: Fable 5.1 architecture review, then phased execution on the workhorse lanes

Prepared 2026-09-21 ~03:00 UTC for a fresh session. **Read this first.** It
supersedes execution-state claims in `HANDOFF_2026-09-21_V0.86.0_MERGE_COMPLETE.md`
and `HANDOFF_2026-09-20_FABLE_HARNESS_ECONOMY.md`; provider policy and safety rules
in `~/dotfiles/policy/providers.md` still govern, and this document does not widen
them.

## 1. Operator goal for this session

Two stages, in this order:

**Stage A, now:** run the session as an **architect reviewer on Fable 5.1** to lay
the groundwork for several phases of iteration. Deliverable is *plans and
criticism*, not implementation: phase plans with acceptance tests, a routing plan
that assigns each phase to a workhorse lane, an explicit criticism of each plan
(over-engineering, missing evidence, regression risk), and a stop rule for
anything that needs new authority.

**Stage B, after Stage A is reviewed:** execute those phases with the workhorse
lanes: OpenCode Go (`glm-5.3-flash` primary, `mimo-v2.5` cheap, `minimax-m2.7`
fallback, `kimi-k2.7-code` A/B, `qwen3.7-plus` escalation), OpenRouter (metered,
admission-gated), and the ChatGPT subscription for frontier packets, review, and
integration judgement.

Do not start Stage B work in this session beyond reading. Do not implement during
Stage A except for trivial evidence-gathering commands.

## 2. Authority and boundaries

Approved in the previous session, already done, do not redo:

- The v0.86.0 merge (`ed35ce309`), the bounded test-env lock (`ba5b79f45`), and the
  docs commits through `cc3ea6ead`.
- Reload onto the merged binary, verified: launcher and shared server both resolve
  to `~/.jcode/builds/versions/5e08d2cfc/jcode`, reporting
  `jcode v0.86.211-dev (5e08d2cfc)`.
- Four upstream pull requests and six issues were filed (section 3).
- Branch and worktree creation for that PR work; the worktrees are gone now.

Still gated, ask first every time:

- Push, publish, deploy, release, install, or provider/auth/default changes.
- Reloading the running daemon (`selfdev build-reload`), because it restarts the
  operator's live session.
- New branches or worktrees beyond reading.
- Metered spend that has not passed admission, and heavy ChatGPT use.
- Destructive cleanup of anything outside your own scratch paths.

Standing rules: never bypass a gate, never weaken a test or a budget to get green,
never print or store secrets, no hidden provider or account fallback, and leave
`~/dotfiles` alone because another agent works there concurrently.

## 3. Frozen state, verified 2026-09-21T02:57Z

| Fact | Value |
| --- | --- |
| Repository | `/Users/ianalitis/.jcode/source/jcode`, branch `jcode/ci-format-baseline`, tree clean |
| HEAD | `cc3ea6ead` (docs). Merge is `ed35ce309`; lock bound is `ba5b79f45` |
| Running binary | `jcode v0.86.211-dev (5e08d2cfc)`; only docs commits since that build (verified: no code drift) |
| Upstream | `origin/master` = `e589cbe5a` (`v0.86.0`), 0 commits behind, confirmed by fetch |
| Fork remote | `fork` = `github.com/ianalitis/jcode`; PR branches are pushed there |
| Open PRs | [#1354](https://github.com/1jehuang/jcode/pull/1354) lint and rustfmt drift plus the `dev-bins` bench build (closes #1348, #1294); [#1355](https://github.com/1jehuang/jcode/pull/1355) jev socket (closes #1351); [#1356](https://github.com/1jehuang/jcode/pull/1356) bounded test-env lock (closes #1349); [#1357](https://github.com/1jehuang/jcode/pull/1357) atomic status writes (closes #1350) |
| Open issues | #1348, #1349, #1350, #1351, #1352 (no PR yet), [#1358](https://github.com/1jehuang/jcode/issues/1358) |

PR state as of preparation: only the Greptile review check is pending on all four;
no GitHub Actions run has started. Recheck before assuming anything about CI.

## 4. Read order

1. This document.
2. `docs/FORK_POSTURE.md`: what this fork keeps, what it adopted from upstream, the
   config and routing surfaces, the gates, and the harness invariants.
3. `docs/UPSTREAM_MERGE_V0.86.0.md`: the merge receipt (scope, per-class resolution,
   verification table, carried-forward issues).
4. `docs/HARNESS_LOOP_ARCHITECTURE.md`: the execution axes, the T0-T6 ladder, the
   admission to gate sequence, and the acceptance-cost function that every phase
   plan must name rather than restate.
5. `docs/TUI_TEST_FLAKINESS.md`: the parallel-state analysis, two recorded wedges,
   the bounded lock, and the measured failure sets.
6. `docs/plans/TOKEN_ECONOMY_PLAN.md` and `docs/HARNESS_ECONOMY_CYCLE_2026-09-20.md`
   for the metric contract already agreed.
7. `docs/upstream-feedback/*`: the reproducible packets behind the filed issues.
8. Canonical policy, not for restating but for compliance:
   `~/dotfiles/policy/providers.md`, `~/dotfiles/docs/plans/2026-09-19-agent-token-economy-optimization.md`,
   `~/.jcode/config.toml`, `~/.jcode/swarm-prompt.md`.

## 5. Lanes, live budget, and what each is for

Measured with `jcode usage --json` at 2026-09-21T02:57Z. Re-measure before
dispatching; these windows move.

| Lane | Model ids | State now | Admitted use |
| --- | --- | --- | --- |
| OpenCode Go (included) | `opencode-go:glm-5.3-flash` primary; `mimo-v2.5` cheap; `minimax-m2.7` fallback; `kimi-k2.7-code` A/B; `qwen3.7-plus` escalation | key valid; local spend $1.91 today, $3.78 this month; provider windows not exposed to Jcode, console is authoritative | the default iterative workhorse for every phase |
| OpenRouter (metered) | `deepseek/deepseek-v4-flash-0731` pilot | balance $92.26 of $115.00 (19.8% credits used), $0.03 today, $8.75 this month | bounded, checkable packets only, after admission: pinned model, no fallback, ZDR, data collection denied, unit-price ceiling, output cap |
| ChatGPT subscription | `openai-oauth` frontier lanes | plan prolite; 7-day window 30.0% used, resets 2026-09-27T17:08Z; today 115.6M input / 484.8K output tokens, $89.63 API-equivalent estimate (not a bill) | packet writing, review, integration judgement. Scarce: keep at least 20% headroom and do not spend it on loops |
| Claude subscription | `claude-oauth:claude-fable-5-1` for this review | 5-hour window 0%, 7-day 31.0%, **7-day Fable window 59.0% used, about 41% left**, resets 2026-09-25T06:59:59Z; long-context extra usage disabled | concise architecture consultation only, which is exactly Stage A. Not a routine implementation or review loop |
| Local `mlx-serve` | loopback only | last used 2 days ago | private classification, extraction, reranking, abstention. Never a route owner |

Rules that apply to every phase plan you write: one frozen treatment per task
(model, effort, tool surface fixed before execution), routing sticky per job rather
than per turn, a quota stop or OAuth failure ends the treatment and hands off
visibly, no silent substitution, no quota-filling work, and no model may widen its
own authority or pick its own successor. Keep the Go session header behavior as it
is: Go validates jcode v0.81.6 or newer and jcode sends `x-opencode-session` for
OpenCode hosts only.

## 6. Workstream inventory: what is solved, what is open

Use this as the raw material for phases. Each item names the evidence and the
selector, so a phase plan can reuse them instead of re-deriving.

**W1. Upstream contribution follow-through.**
Open: four PRs awaiting review; #1352 (swarm stop) has no PR yet; #1358 (base-suite
isolation) has no fix proposed. Evidence: the packets in `docs/upstream-feedback/`
and the PR bodies (each has a verification section). Questions for the architect:
what cadence, what to answer when reviewers push back, and which of our fork
patches deserve to go upstream next.

**W2. Test determinism in `jcode-tui` and `jcode-app-core`.**
Open: a varying 2-7 `jcode-tui --lib` failures per parallel run, all passing alone.
Two families, measured over six runs: render state
(`tui::ui::tests::basic::test_changelog_overlay_repeated_renders_are_stable` in 5 of
6 runs, `tui::ui::messages::tests::render_system_message_uses_scheduled_task_card`),
and provider catalog or picker state
(`tui::app::tests::test_model_picker_reuses_cached_entries_until_invalidated`,
`tui::app::tests::test_local_model_picker_render_shows_antigravity_models_exactly_as_user_sees_them`,
`tui::app::tests::test_model_picker_waits_for_async_post_login_catalog_activation`,
`test_login_completed_surfaces_new_provider_models_in_local_model_picker`,
`test_login_smoke_model_picker_renders_unstacked_provider_rows`,
`test_open_model_picker_without_routes_shows_actionable_guidance`,
`test_local_model_picker_surfaces_antigravity_models_from_multiprovider`,
`test_local_model_picker_openrouter_bare_openai_route_uses_openai_catalog_prefix`,
`test_agents_picker_uses_provider_default_when_inherited_model_is_unknown`,
`test_agent_model_picker_inherit_row_uses_provider_default_when_inherited_model_is_unknown`,
`tui::app::shortcut_hints::tests::fast_and_slow_counters_persist_independently`).
The picker family only calls `ensure_test_jcode_home_if_unset()`, which `try_lock`s
and proceeds without exclusion. `jcode-app-core`'s
`tool::tests::test_context_guard_refusal_names_the_spilled_output` has the same
shape via a spill-file name. A blanket env lock was tried and reverted because it
serialized the suite from ~27s to over 10 minutes; the intended direction is per-test
temporary `JCODE_HOME`, or scoping the lock to the affected family, with the runtime
cost measured. This is the highest-value phase: red suites erode every other
verification claim.

**W3. Env-lock re-entrancy.**
The lock is bounded now and reports its holder, but the site that re-entered it is
still unknown (`with_temp_jcode_home` and `with_reasoning_current_home` in
`tui/app/tests/support_failover/part_01.rs` are candidates, called from 186 sites).
Phase question: keep the diagnostic and eliminate the call site, or change the guard
type to a reentrant one at roughly forty annotated sites. Evidence: the wedge sample
and the three tests in `storage::tests`.

**W4. Base-suite isolation on upstream and here.**
Issue #1358 records eleven `jcode-base --lib` failures at the default thread count
with an A/B control (1472 passed / 11 failed on pristine `e589cbe5a`, same set here).
Our fork's copy passes 1530 of 1530. Phase question: port our env-guard pattern
upstream as a PR, and confirm our copy stays green as upstream moves.

**W5. Harness and token economy.**
Open: `docs/plans/TOKEN_ECONOMY_PLAN.md` items, the acceptance-cost function, and the
question of what to measure next. Evidence already exists for one cycle in
`HARNESS_ECONOMY_CYCLE_2026-09-20.md`. This is where the Fable review can add the
most architectural value, provided it produces measurement contracts rather than
opinions.

**W6. Routing codification and re-measurement.**
Open: Go's 5-hour, weekly and monthly windows are not exposed to Jcode, so they are
unknown rather than free; the ZDR agreement expires 2026-09-30; the limited-time 4x
boost ended 2026-09-20; OpenRouter admission needs end-to-end spend enforcement
before unattended use; ChatGPT is the scarce resource. Phase question: which
measurement, refreshed on what schedule, should change a routing default, and what
evidence would justify promoting or retiring a lane.

**W7. Merge and CI cadence.**
Upstream is at `e589cbe5a` and will move; this fork merges rather than rebases and
its receipt is the template. Note that upstream's own gates are red at `e589cbe5a`
(`cargo check --all-targets --all-features` fails on the `dev-bins` bench,
`cargo clippy -D warnings` and `cargo fmt --all --check` fail on drift, and its base
suite fails eleven tests), so "upstream is green" is not a safe assumption. Phase
question: cadence, and what to do when upstream lands a fix that duplicates ours.

**W8. Privacy and telemetry posture.**
This fork removes the feedback tool and todo telemetry collection. Phase question:
how to keep that property as upstream adds features, without carrying a large
divergence.

## 7. The planning brief: what the architect must produce

For each phase you select, answer these, briefly and concretely:

1. Goal and non-goals, with the operator-visible outcome stated as a check.
2. The **acceptance test**: the fully qualified test name or the exact command, and
   the number that must change. Evidence over assertion.
3. The **freeze**: lane, model id, effort, tool surface, and the single writer.
4. The **gate**: which `scripts/check_guardrails.sh` gates and which suites prove it,
   plus the rollback if it fails.
5. The **budget**: expected tokens or wall time, and the stop condition.
6. The **criticism**: where the plan is over-engineered, what evidence is missing,
   what could regress, and what would make you abandon it.
7. The **handoff**: what Stage B needs to start, in what order, with what
   verification before the next phase begins.

Also produce:

- A single routing table: phase to lane to model to effort to verification bar.
- A sequencing argument: which phase unlocks the most, and which are blocked on it.
- A short list of things this fork should **delete or stop doing** to stay close to
  upstream.
- An explicit list of decisions that are the operator's, not yours.

## 8. Phase framework to apply

Name `docs/HARNESS_LOOP_ARCHITECTURE.md` rather than restating it: the execution
axes, the T0-T6 ladder, admission then freeze then receipt then gate, and the
acceptance-cost function. Then apply these local constraints:

- One writer and at most two independent readers per phase; workers stay inside an
  assigned node and return inspectable evidence.
- Every worker packet names its permitted files, the desired behavior, the exact
  validation command, and a hard no-scope-expansion rule.
- Parallel readers only where they genuinely cover different ground.
- A phase is done when its acceptance test passes with evidence at a stated path,
  not when the diff exists.
- Any unexpected failure, wider-than-authorized diff, or new design decision stops
  the bounded treatment and hands off visibly.

## 9. Output contract for Stage A

Write, and commit only if the operator approves committing in that session:

- `docs/plans/2026-09-21_ARCHITECT_REVIEW.md`: the review itself, the sequencing
  argument, and the criticism.
- `docs/plans/PHASE_<n>_<name>.md` per phase, each following section 7.
- If it helps Stage B, a `docs/plans/2026-09-21_PHASE_ROUTING.md` table.

Stop and ask the operator for anything in section 2, for any decision that changes
routing defaults, and before any phase whose only evidence would be a metered spend.

## 10. Handoff from Stage A to Stage B

Stage B sessions spawn bounded workers with a pinned route, for example
`opencode-go:glm-5.3-flash` for implementation and test iteration,
`opencode-go:mimo-v2.5` for cheap scoped edits and triage,
`openai-oauth` frontier lanes for packet writing and review while the 7-day window
allows, and `claude-oauth:claude-fable-5-1` only for another architecture question.
Every worker receipt opens with its frozen model and effort so results stay
attributable. Plans that need a lane the budget cannot sustain should say so and
propose the cheaper fallback explicitly rather than assuming it.

## 11. Ready-to-paste brief for the Fable 5.1 session

```text
You are the architecture reviewer for a fork of jcode at
/Users/ianalitis/.jcode/source/jcode on branch jcode/ci-format-baseline, HEAD
cc3ea6ead. Frozen treatment: claude-oauth:claude-fable-5-1, effort high, read-only
tools. Do not implement, do not push, do not reload the daemon, do not create
branches or worktrees, and do not spend metered budget.

Read, in order: docs/HANDOFF_2026-09-21_FABLE_ARCHITECT_PHASE_PLANNING.md (this
brief's source of truth), docs/FORK_POSTURE.md, docs/UPSTREAM_MERGE_V0.86.0.md,
docs/HARNESS_LOOP_ARCHITECTURE.md, docs/TUI_TEST_FLAKINESS.md, and the eight
workstreams in section 6 of the handoff.

Produce: (1) a review of the current fork posture and the eight workstreams,
including what is over-engineered and what is missing evidence; (2) a sequencing
argument for three to five phases; (3) one plan document per phase following
section 7 of the handoff, each with a fully qualified acceptance test, a frozen
lane and model, a gate list, a rollback, a budget, and a criticism section; (4) a
phase-to-lane routing table; (5) an explicit list of decisions that belong to the
operator.

Constraints: one writer per phase, evidence over assertion, no plan whose only
proof would be metered spend, and no plan that weakens a gate, test, or budget to
reach green. If a fact you need is not in the repository, say so instead of
assuming it.
```

## 12. Commands to re-verify state before you plan

```sh
cd ~/.jcode/source/jcode
git log --oneline -6 && git status --porcelain | wc -l
git fetch origin master && git rev-list --count HEAD..origin/master
~/.local/bin/jcode --version
readlink -f ~/.jcode/builds/shared-server/jcode
bash scripts/check_guardrails.sh --skip-slow        # fast pass
jcode usage --json                                   # lanes and windows
jcode model list -p opencode-go | head               # Go catalog
gh pr list --repo 1jehuang/jcode --author ianalitis --state open --json number,title
gh pr checks 1354 --repo 1jehuang/jcode
```

## 13. Traps this session hit, so the next one does not

- `cargo clippy --fix` is unsafe for a lint sweep: rustfix also applies
  machine-applicable `dead_code` suggestions, which silently deleted a whole
  `#[cfg(test)] mod tests` block plus test helpers. Apply per lint and review the
  diff; the rebuilt PR is #1354.
- After `git apply` in a fresh worktree, `touch` the touched files or check
  `cargo test -- --list` first: cargo considered the build up to date and ran a
  stale test binary that reported zero matching tests for tests already on disk.
- Two harness-level process-wide mutexes exist in tests: the env lock and the
  render-state lock. Both have produced wedges. Take one lock per test, never hold
  one while waiting on a thread that needs it, and prefer `try_lock` degradation in
  helpers reachable from a locked test.
- Upstream's gates are red at `e589cbe5a`, so a fork-side "all green" claim needs
  the gate command and its output, not an appeal to upstream.
