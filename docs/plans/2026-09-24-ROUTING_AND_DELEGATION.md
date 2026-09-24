# Routing and delegation: openrouter/auto as default router? (2026-09-24)

Status: **not adopted.** Measure first (P3). This doc reconciles the proposal
with `~/dotfiles/policy/providers.md` and the current quota state.

## Proposal under review

Make `openrouter/auto` the default model route and let the captain
auto-delegate subtasks to whatever the router picks.

## Why not as a default today

1. **Policy conflict.** Provider policy makes OpenCode Go
   (`deepseek-v4.1-flash`) the default worker and the captain an OAuth
   subscription lane. OpenRouter is "bounded metered complementary capacity":
   checkable extraction, fixtures and patch proposals, not unrestricted
   writable work.
2. **No spend enforcement.** The OpenRouter account has no per-key cap
   (policy, measured 2026-09-21). `jcode usage` 2026-09-24 23:35Z: $72.36 of
   $115 left, $19.48 spent today. A default metered router with no cap turns every
   session into open-ended spend. A capped inference key is a prerequisite and
   needs operator approval.
3. **Data class.** Router admission requires `openai/*` and `anthropic/*`
   exclusions (`REQUIRED_ROUTER_EXCLUSIONS`) and ZDR. `auto` picks the model per
   request, so repository content goes to a provider chosen at runtime. Private
   repository packets stay off metered routes until data-class admission ships
   (`jcode/data-class-admission` worktree, not merged).
4. **Unmeasured.** `openrouter/auto` worked in a trivial probe (2026-09-20,
   ~17k input tokens for a trivial prompt). There is no task-level evidence that it
   beats the pinned lanes on quality or cost.

## What is adoptable now

- **Declared quota fallback** (`JCODE_QUOTA_FALLBACK`, `830856614`) already
  covers the real need, "keep working when a subscription window is spent",
  without a default router. It is explicit and visible, with no silent
  substitution.
- **`openrouter/auto` as a discovery lane**, per `TOKEN_ECONOMY_PLAN.md`:
  public or synthetic packets only, record the served model (`ServedModel` event)
  and promote a recurring winner to a pinned lane after measurement.

## Measurement gate before any default change

Uses the protocol in [`2026-09-24-BENCHMARK_RESEARCH.md`](2026-09-24-BENCHMARK_RESEARCH.md).

| Arm | Route | k |
| --- | --- | --- |
| A | captain Opus 5.5 high alone | 3 |
| B | captain + Go DeepSeek workers | 3 |
| C | captain + `openrouter/auto` workers (capped key, public task) | 3 |

Adopt C as a default only if it beats B by ≥0.25 score at ≤ equal cost, or
matches B at ≤50% cost, with the capped key in place. Otherwise keep it as a
discovery lane.

## Prerequisites (each separately approved)

1. Capped OpenRouter inference key (`scripts/openrouter-admin.sh`, operator).
2. Data-class admission merged (review `jcode/data-class-admission`).
3. jcode-bench baseline (P3).
4. ChatGPT window reset (2026-09-27) or Go weekly reset, so arm B is runnable.
