# Agent and token economy plan

Status: plan, 2026-09-19. Extends `docs/HARNESS_LOOP_ARCHITECTURE.md` (the
five axes, route table, receipts) and revises its section 8 rejection of
dynamic routers. Numbers marked **[measured]** come from local ledgers on this
machine on 2026-09-19; everything else is a proposal.

## 1. Objective

Minimize accepted-work cost, not token price. The objective per task class is
still

$$C_{acc} = \frac{c_{attempt} + c_{verify} + c_{review}}{p_{accept}} + (1-p_{accept})\,c_{escalate}$$

with two changes in emphasis:

1. Frontier subscriptions are now a throttled, sunk resource. Their marginal
   price is zero until a window hits 100%, then infinite. They are scheduled,
   not optimized.
2. The metered lane (OpenRouter open-weight models) is where price matters,
   and where the harness must earn its keep by keeping context small, cached
   and verifiable.

## 2. Where the money goes today [measured]

Source: `~/.jcode/openai_oauth_usage.json`, `~/.jcode/model-usage-v1.sqlite3`,
`jcode usage --json`.

| Metric | Value |
|---|---|
| OpenAI OAuth requests, 2026-09-13 to 09-19 | 17,718 |
| Input tokens | 2.08 B (95.5% cache reads, 94 M uncached) |
| Output tokens | 9.6 M |
| Average per request | 117,580 input / 542 output |
| API-equivalent estimate (not a bill) | $2,539 known, 5,606 unpriced requests |
| Per request | $0.143 API-equivalent |
| OpenAI 7-day window | 100% used, resets in 3d 21h |
| Claude 5h / 7d / Fable windows | 4% / 9% / 16% |
| OpenRouter | $6.48 of $15 balance, key limit reported $5.00 / $5.00 |
| Turn ledger by route | 663 gpt-6-astra, 30 fable-5-1, 10 mlx-serve, 1 opus-5 |

Three conclusions follow directly:

- **The unit of waste is context replay, not the model.** 117 k input per
  request with 542 output means every turn re-sends the whole conversation.
  Frontier providers hide this behind a 95% cache hit. A cheap model without
  prompt caching would receive the same 117 k tokens at full price:
  DeepSeek-class input at $0.28/M is $0.033 per request, $580 for the same
  week, which is not a bargain relative to an already-paid subscription. Cheap
  models only win after context per request drops or caching is confirmed on
  the chosen endpoint.
- **The frontier subscription is quota-bound, not credit-bound.** The window
  is exhausted while credits sit unused. Optimization there means scheduling
  bounded batches against window resets and dropping effort to the floor, not
  "use it more".
- **The metered lane is effectively unfunded.** A $5 key limit at $5 spent is
  a hard stop before any workhorse experiment. D2 (account-side cap) must be
  re-set deliberately, not discovered mid-run.

## 3. Lanes after this plan

| Lane | Role | Selection rule |
|---|---|---|
| Included frontier (OpenAI OAuth Astra/Terra/Sol, Claude Fable/Opus) | Captain, architecture, security review, batch burn of expiring credit | Window headroom above 20% for interactive; scheduled batches when a window resets; effort floor by default |
| Metered open-weight pinned (DeepSeek V4 Flash, GLM, Kimi, Qwen via OpenRouter) | Default worker for implementation, tests, fixes, extraction, review drafts | Cheapest route on the task-class frontier with measured $p_{accept}$; ZDR, `data_collection: deny`, `allow_fallbacks: false`, `require_parameters: true`, `max_price` |
| Metered dynamic (`openrouter/auto`, `openrouter/auto-beta`, `openrouter/pareto-code`) | Discovery lane: finds which cheap model the market and benchmarks currently favor per task kind | Public/synthetic data only until ZDR-constrained routing is verified; `cost_tier: low`, `min_coding_score` per task class, model allowlist/ignore; the routed `model` from the response is recorded and becomes a promotion candidate for the pinned lane |
| Local (mlx-serve Ornith 9B, gemma-4-e2b) | System 1 classification, reranking, summarization, private-data pre-processing | Always for private data class when the task is closed-set; never route owner |
| Remote System 1 (Jev) | Closed-set judgments on public/synthetic state, task-class labeling of sanitized packets | $0.04/M; D3 holds: no private working-tree content until ZDR |

Dynamic routers are admitted as a **discovery** lane, not as a route owner.
The frozen-treatment rule still applies: the attempt records the router slug
requested and the model actually served (from the response `model` field);
the receipt binds both. A router is never a hidden fallback for a pinned route.

## 4. Work packets, in order

Each packet has a measurement it must move. No packet ships on a vibe.

### P0. Instrument before optimizing (1 to 2 days)

- `jcode usage` already reports API-equivalent USD for OpenAI OAuth. Extend the
  same accounting to every route: OpenRouter (actual billed from the
  `generation` endpoint), Claude OAuth (API-equivalent), mlx-serve (wall time
  and RSS). Store per-turn: route, input, cached, output, USD, task class,
  outcome (accepted / rejected / escalated).
  Owner: `jcode-base/src/model_usage.rs`, `provider_activity.rs`.
- Add `context_bytes_by_source` per request: system prompt, tool schemas,
  conversation, tool outputs, memory injections. This is the breakdown that
  decides P1.
- Fix the unpriced gap: 5,606 of 17,718 requests have no price. Unknown cost
  is not zero cost.
- Metric: every request in the ledger has route, USD (or explicit
  `unpriced`), and context breakdown.

### P1. Cut context per request (largest lever, 1 week)

Target: median input per request under 40 k on worker sessions, under 70 k
on captain sessions, without losing acceptance rate.

- Tool schema and system prompt budget: measure bytes; drop or lazy-load
  tool definitions not used by the session's allowed tool set (spawn already
  narrows `allowed_tools`; the schemas sent should match).
- Tool output caps by tool: bounded default with explicit "read more"
  continuation, so a stray `cat` or test log does not pin 30 k tokens into
  every later turn.
- Compaction: switch default from `reactive` to `proactive` with a floor at
  0.4 fill, and add a **cost-aware trigger**: compact when projected replay
  cost for the remaining turns exceeds the one-time summarization cost.
  Track compaction accuracy with the existing `retention_readiness` tests.
- Cache-aware ordering: keep the stable prefix (system, tools, early context)
  byte-identical across turns so OpenRouter, Anthropic and OpenAI caching all
  hit. Any per-turn mutation of the prefix (timestamps, usage banners) breaks
  the cache on every provider at once. Add a test that the prefix hash is
  stable across two consecutive turns.
- Metric: input tokens per request and cached fraction, per route, before and
  after, on the same replayed sessions.

### P2. Fund and constrain the metered lane (half a day, needs operator)

- D2: set the OpenRouter key limit to a real weekly budget with an
  account-side hard cap. Add balance and key limit to the 20% headroom rule
  the swarm prompt already applies to subscriptions.
- Pin the request envelope for every metered route: `provider.zdr = true`,
  `data_collection = "deny"`, `allow_fallbacks = false`,
  `require_parameters = true`, `max_price` from the route table, and
  `models` allowlist. `effective_routing` and the frozen-request validator
  already exist; extend them to carry `zdr`, `max_price`, `models`.
- Confirm prompt caching per candidate endpoint (DeepSeek and GLM report
  cache hits in usage); refuse to promote an endpoint whose cache field is
  absent for a worker task class.

### P3. Dynamic-router discovery lane (2 to 3 days)

- Lift the blanket ban in `is_banned_router_family` into a typed
  `RouteClass::DynamicRouter` that is admitted only for `Public | Synthetic`
  data, with a per-request `models`/`ignore` list that excludes the frontier
  families we already pay for through subscription.
- Carry `cost_tier` (auto) and `min_coding_score` (pareto-code) as route
  table fields, chosen per task class.
- Record the served `model` from the response on the receipt. Aggregate
  served models per task class weekly. Any model that wins a task class on
  accepted-work cost for two weeks becomes a pinned candidate.
- Metric: served-model distribution per task class and $C_{acc}$ versus the
  pinned baseline on the same fixtures.

### P4. Task-class classification feeding admission (3 to 4 days)

The route table already keys on `task_class`; the wire does not yet carry it.
Fill it in three tiers, in order:

1. Deterministic: explicit `task_class` on the node, or derived from the
   packet template (test-fix, extraction, review, docs, implementation).
2. Local S1: Ornith 9B on mlx-serve, closed candidate set of task classes,
   used for private packets. Abstain routes to the captain, never to a
   bigger model automatically.
3. Jev Choice: public/synthetic packets only (D3), $0.001 per 29 calls in the
   P2 eval. Use it to label the holdout that calibrates $p_{accept}$ per
   route and class.

Classification never sets authority, tools or data class; it only selects
among routes the admission table already permits.

### P5. Frontier burn schedule (ongoing, zero engineering)

- Captain stays on Astra at effort `low` (already configured). Raise effort
  per turn, never per session.
- Maintain a queue of bounded, receipt-gated packets suitable for Terra/Sol
  (architecture review, security audit, oversized-file extraction review).
  When `jcode usage --json` shows a window reset, `fill_slots` from that
  queue up to the 80% headroom line, then stop.
- Verify whether purchased ChatGPT credits extend past the 7-day cap or only
  fund another product surface. If they do not extend the cap, the credit is
  burned only by time; the schedule above is the whole plan for it.

### P6. Executor economics (1 week, after P1 to P3)

- Metered workers run as Pi no-tools or headless Jcode with a narrowed tool
  set, a deadline and a budget on the spawn envelope (still open per
  `HARNESS_LOOP_ARCHITECTURE.md` section 6). Budget is enforced by the local
  ledger reservation, not by the model.
- Three cheap failures stop the attempt and hand off visibly. No ladder
  climbing inside a worker.
- Verify gates are the cost floor of every route: deterministic checks first,
  S1 judge second, frontier review only for security-sensitive or ambiguous
  outcomes.

## 5. Weekly review loop

Run every Monday from the ledger, no model needed:

1. Cost per accepted node by task class and route.
2. Input tokens per request and cache fraction by route.
3. Served-model distribution from the discovery lane.
4. Promotions and demotions in the route table, each with an evidence path.
5. Subscription window utilization: wasted quota (window reset with headroom
   unused) versus blocked interactive work (window at 100% during captain
   hours).

A route is promoted when it wins on $C_{acc}$ for two consecutive weeks on
the same fixtures, and demoted the first week it loses by more than 20%.

## 6. What this plan does not do

- It does not make a router the route owner. Admission, the frozen record and
  the receipt validator stay deterministic and local.
- It does not send private working-tree content to a dynamic router until
  ZDR-only routing is verified end to end (request flag, response provider,
  activity log).
- It does not substitute a metered model for an exhausted subscription
  silently. A quota stop is a visible handoff.
- It does not add a daemon, a router service or a new UI. Fields on receipts,
  the route table and `jcode usage` are the whole surface.

## 7. Open decisions for the operator

- D2 amount: weekly OpenRouter cap.
- D3: keep Jev public-only, or purchase ZDR to allow private packets.
- D8 (new): admit `openrouter/auto` with `cost_tier: low` for public
  discovery now, or wait for P1 context reduction so its measurements reflect
  the future request shape.
- D9 (new): which task classes may run on `pareto-code` at
  `min_coding_score` 0.5 versus 0.8. Suggested: tests and extraction at 0.5,
  implementation at 0.8, review never (review stays pinned).
