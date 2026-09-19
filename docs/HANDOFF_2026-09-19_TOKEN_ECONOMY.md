# Handoff: token economy packets and source reconciliation

Written 2026-09-19 for a fresh session on `openai-oauth:gpt-6-astra`, effort
`low`. Raise effort per turn only for J5/J7 design decisions. Read this file,
then `docs/plans/TOKEN_ECONOMY_PLAN.md` (sections 8 to 11), then
`docs/SOURCE_CLOSEOUT_2026-09-19.md`. Do not re-derive the plan.

## State at handoff

- Branch `jcode/ci-format-baseline`, HEAD `0a14c404f`. Eight commits since
  the closeout doc `f3d0072ff`, all mine, all offline-tested:
  `a2653e00e` plan, `b02029c6d` smoke + packet split, `addda634e` pareto
  finding, `e64800b0e` J8, `93d65c5d7` J2, `0abdeb5c6` J3, `9003367fd` live
  smoke example, `0a14c404f` J1.
- Working tree: 166 tracked dirty, 115 untracked. **Not mine.** A concurrent
  agent owns a large test-extraction and protocol-axes effort (app-core,
  TUI, provider-core, openrouter-runtime test partitions). Never `git add -A`.
  Stage your own hunks with `git add -p` or a filtered `git apply --cached`
  (see how `0abdeb5c6` staged only `ServedModel` hunks in
  `turn_loops.rs`, `turn_streaming_mpsc.rs`, `tui/app/turn.rs`).
- Dotfiles (`~/dotfiles`, HEAD `3bd17e9`) has the policy twin:
  `docs/plans/2026-09-19-agent-token-economy-optimization.md` and two
  followups in `docs/followups.md`. Its agent had uncommitted edits to
  `policy/providers.md` and the swarm prompt. Boundary: dotfiles owns
  policy, credit-burn program, promotion screen. This repo owns crates.
- Quotas: OpenAI 7-day window 100% until 2026-09-23 18:03 UTC. Claude
  windows low but the operator wants them preserved. OpenRouter key cap is
  $10/day, balance topped up; account exclusions `openai/*`, `anthropic/*`
  set on the Auto Router only.
- Measured and recorded (do not repeat): `openrouter/pareto-code` ignores
  account and request exclusions and serves `openai/gpt-5.6-sol` at API
  rates at every tier. It is refused by J2. `openrouter/auto-beta` honors
  exclusions, sticks per session, returns `task_type`.

## Packets remaining, in order

Each is one bounded commit with offline tests. Run
`cargo test -p <crate> --offline`, `cargo clippy -p <crate> --all-targets
--offline -- -D warnings`, `cargo fmt -p <crate> -- --check` before commit.
`jcode-executor-pi` has two timing-flaky tests (`run_times_out_mid_stream`,
`rejected_prompt_cannot_produce_success`) that pass with `--test-threads=1`.

### J5 spawn envelope (next)
- `CommSpawn` (find via `agentgrep "allowed_tools: Option"` in
  `jcode-protocol`/`jcode-app-core`) gains optional `max_micro_usd`,
  `deadline_secs`, `data_class`, `router: Option<RouterPolicy>`.
- `Agent::build_base` / `new_with_initial_ownership` carry them like
  `allowed_tools` already is.
- Plan-level cap: sum `max_micro_usd` across children of one `run_plan`;
  refuse the next spawn when reserved exceeds the cap. Reuse
  `LocalLedger` in `jcode-provider-openrouter-runtime/src/attempt_caller.rs`
  (reserve / settle / mark_ambiguous already exist).
- Tests: spawn without budget on a metered route is refused; plan halts at
  cap; ambiguous settlement keeps exposure.

### J4 effort to route
- `[agents.effort_routes]` in `jcode-config-types` (`AgentsConfig`, next to
  `swarm_model`). Map: `none|minimal|low -> openrouter:openrouter/auto-beta`
  with `cost_tier: low`; `medium -> auto-beta medium`; `high ->
  openai-oauth:gpt-5.6-terra`; `xhigh|max -> openai-oauth:gpt-5.6-sol`.
- Spawn resolves effort to route only when `model` is absent. Explicit
  model wins. Tests for each effort and the override.
- The auto-beta route needs the J2 `RouterPolicy` attached so freeze admits
  it; wire the `plugins` array (`{"id":"auto-beta-router","cost_tier":..,
  "excluded_models":[..]}`) into the OpenRouter request when
  `record.router` is `Some`. That request-side emission is **not done yet**:
  J3 only parses the response. Add it in `openrouter_provider_impl.rs`
  `complete_inner` next to the `provider` object, with a frozen-request
  test in `router_shaping_tests.rs`.

### J6 attribution
- `jcode usage --json` OpenRouter rows per day and per served model, USD
  from `ServedModel.micro_usd` (billed) and tokens. Mirror
  `~/.jcode/openai_oauth_usage.json`. Surface unpriced count.
- Store: extend `jcode-base/src/model_usage.rs` (sqlite `turns` table) with
  `served_model`, `micro_usd`, `task_type` columns, or a sibling table.

### J7 worker context diet
- Measured baseline: 117k input tokens per request, 542 output. Target under
  40k per worker turn.
- Worker system prompt = packet only; tool schemas limited to
  `allowed_tools`; per-tool output byte cap with continuation; proactive
  compaction default for workers; a test that the request prefix hash is
  byte-identical across two consecutive turns.
- Needs Sol/high review before integration (touches `jcode-app-core`
  prompting and tool dispatch).

### J8 classifier-fed routing
- After J6 has logged `task_type` for a week. Deterministic template match
  first, local Ornith 9B on mlx-serve for private packets, Jev Choice for
  public/synthetic (D3). Label selects among admitted routes only.

## Source reconciliation (after packets, or interleaved when blocked)

Follow `docs/SOURCE_CLOSEOUT_2026-09-19.md` section "Next execution order"
exactly. Summary:

1. Upstream `origin/master` was force-updated (`5cb7b3dad -> eebc4996f`).
   Compare merge bases and `git range-diff` before choosing a base. Never
   force-push or reset to converge.
2. Close dirty ownership one packet at a time: confirm the concurrent agent's
   extraction is finished, compare moved bodies to sources, verify identical
   named-test inventory, then commit per crate. Do not adopt protocol-axes
   changes inside extraction commits.
3. Repository acceptance still fails four ratchets (production size, test
   size in app-core `comm_control_tests/dag_e2e.rs` and
   `swarm_persistence_tests.rs`, panic use, swallowed errors in
   `src/cli/tui_launch.rs` 4 -> 6). Fix, never raise baselines.
4. Integrate topics into the fork one at a time from the tables in the
   closeout doc; non-force refspec only after combined gates pass.
5. Upstream PRs: plan only, none created. Existing #1293 and #1295 continue.

## Ground rules carried over

- One frozen treatment per task; no silent provider fallback.
- Never stage another agent's hunks; never raise a ratchet baseline; never
  weaken an assertion to go green.
- Receipt provenance and artifact freshness are still unproven; do not claim
  them.
- Commit each packet as you go with a message that names the packet and the
  test delta.
