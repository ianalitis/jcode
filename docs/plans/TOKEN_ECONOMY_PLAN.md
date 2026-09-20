# Agent and token economy plan

## Current execution amendment: 2026-09-20

This amendment takes precedence over the historical lane proposals below. It
does not change provider policy or authorize new spend. The operator's goal is
cheap, reliable accepted work, with OpenAI supplemental and no dependency on
renewing Anthropic. Expiring-credit burn is not the optimization objective.
Keep active attempts pinned. Reconcile policy in `~/dotfiles` separately rather
than changing generated instructions or host provider settings here.

### Immediate sequence and ownership

1. **Jcode: stabilize build evidence.** Canonical source was clean at
   `2836cf95c`. Coordinated TUI build `883219jfir` passed in 9.1 seconds.
   Inventory and classify every diagnostic before changing build flags. Remove
   only proven no-op profile overrides first. Preserve unwind support and
   existing active dependency optimization. No blanket warning suppression.
2. **Jcode: reconcile runtime and outstanding work.** Distinguish running binary,
   launcher target, channel marker and channel symlink. Finish the branch ledger
   from `../ASTRA_HANDOFF_2026-09-20.md`, verify narrow regressions, and resolve CI
   before claims of full integration. No branch/worktree deletion or upstream
   publication. The untracked data-class `translate_regression_tests.rs` is
   byte-identical to canonical and canonical includes it from
   `translate_tests.rs`; preserve the untracked copy pending cleanup approval.
3. **Jcode: reduce context and prove admission.** Inspect existing budget,
   packet, receipt and provider work before implementing another classifier or
   router. Measure stable-prefix bytes, uncached input, schema/output bytes,
   cache usage, latency, retries and review burden per accepted task. Preserve
   unknown cost as unknown. Reuse existing tests and eval crates.
4. **Dotfiles agent: host inference comparison and canonical policy.** Own
   service lifecycle, installed engines/models, benchmark runner and generated
   policy surfaces. Jcode owns request contracts, deterministic enforcement,
   bounded packets and usage receipts. This is a proposed ownership boundary,
   not a claim that the other agent has received or accepted the packet.
5. **Joint acceptance: promote one small improvement at a time.** Synthetic
   fixtures first, paired baseline/candidate, exact model/engine revisions,
   negative cases and a rollback path. Installation, service replacement,
   account classifiers and metered experiments retain separate approval gates.

### Build diagnostic baseline

Full retained log: `~/.jcode/scratch/harness-assessment-20260920/build-before.log`.
This is a fresh canonical reproduction, not a recovered original terminal log.

Follow-up build `023960qpr2` exited zero (Cargo elapsed 17.38 seconds): all twelve
unmatched-profile warnings disappeared. The linker warning and wrapper notices
remain. This validates only the scoped manifest cleanup, not full runtime or
cross-platform correctness. The before/after builds do not measure speedup:
the latter recompiled build metadata and dependent crates.

| Diagnostic | Classification and next check |
|---|---|
| 12 unmatched package profile warnings | `cosmic-text`, `swash`, `unicode-linebreak`, `yazi`, each in dev/selfdev/test. Absent from lockfile. Confirm resolved metadata, then remove inert overrides only. |
| Apple linker `__eh_frame` exceeds 16 MiB compact-unwind encoding | Build succeeds, but potential exception/unwind performance impact is real. Separate investigation with binary section sizes and relevant unwinding tests. Do not disable unwinding or switch global profiles just to hide it. |
| `flock` unavailable, no host-wide Cargo gate | Operational coordination gap on this Mac, not a compiler warning. Inventory existing macOS locking helpers before choosing a fallback. Coordinated selfdev is not proof of a host-wide gate across other repositories. |
| sccache skipped for incremental units | Expected cache incompatibility, not evidence of a cache miss bug. Compare matched incremental/non-incremental edits before changing defaults. |
| parallel nightly front end, four threads | Informational. Record toolchain and resource use in comparisons. |
| ten Cargo jobs under memory pressure | Resource-control decision, not a failure. Compare wall time and peak memory, including local inference contention. |

At intake, `selfdev status` reported running `d027491f6`, current `2836cf95c`,
and shared server `68a6a4093`. The shared-server symlink instead targeted
`77cdac8ff-dirty-f4f975f58de9`. Source inspection shows the status field reads
`builds/shared-server-version`, not the symlink or running process. Therefore
that field is not runtime identity evidence. Do not manually repoint it or
restart the shared daemon during diagnosis. Known-binary private-socket tests
remain the safe behavior check.

CI run `35524397694` at 17:39 UTC had nine completed successful jobs and Windows
Build & Test still running. Overall success remains unproven.

### Research intake and corrections

Read the supplied Perplexity export and downloaded the English subtitles for
[Sam Witteveen's video](https://www.youtube.com/watch?v=53wDOI_7x8I).
Local originals: `~/.jcode/scratch/semif-research/53wDOI_7x8I.en-US.vtt` and
derived `.txt`. VTT SHA-256:
`b6739924af66e5bcede2435aaba750e0d65968f6a26001556235f2d76931f1ee`.
Additional language variants returned HTTP 429; the English download completed.
Do not commit the full third-party transcript. Its recommendations and numerical
claims are research leads, not hardware acceptance evidence.

Primary sources checked on September 20:
- [SemIf README](https://github.com/TheoLeeCJ/SemIf/blob/master/README.md)
- [SemIf MLX implementation notes](https://github.com/TheoLeeCJ/SemIf/blob/master/docs/MLX.md)
- [OpenRouter classifiers](https://openrouter.ai/docs/guides/features/classifiers)

SemIf is independent of Jev. It directly scores option logits, including native
MLX direct/serial/shared modes. Its headline 5.21x comparison is on an RTX 3090,
not this M3 Pro. Direct and generated answers agreed on only 18/21 criteria in
that systems comparison. Option softmax values are not calibrated confidence.
The MLX default checkpoint is about 9 GB **on disk**, with additional runtime
memory. In-memory Q4/Q8 quantization changes probabilities and does not establish
a sub-10-GB peak loading footprint. Shared suffix batching also increases memory.
Do not infer that a small quantized artifact or allocator counter proves total
Mac resident memory fits the target.

The video explicitly demonstrates hard-task and wording failures. Its suggestion
to escalate uncertain outputs is useful only inside deterministic policy, with
visible attempt boundaries. A confident classifier may still be wrong. Its
claim about Jev benchmarking terms needs current terms review before any live
comparative hosted evaluation.

OpenRouter workspace classifiers are asynchronous, billed secondary generations
after the original request. They cannot choose that request's route. They receive
serialized prompts, tool names rather than full schemas, and turns truncated at
5,000 characters. Oversized classification can fail without failing the original
request. Therefore absence of a tag is missing evidence, not a low-risk label.
Do not claim an actual test outcome or model overprovisioning from those tags
alone. Join tags to verified task receipts and observed cost/retries instead.

Keep the current two 10% classifiers unchanged during intake. Measure taxonomy
overlap, tag coverage, billed classifier cost and actionable disagreement before
recommending removal of Task type. Prefer one useful sampled taxonomy over two
duplicates, but do not substitute a guessed `outcome=pass` tag for test results.
No private prompt may be newly sent to hosted Jev merely because an account
classifier is already enabled. Hosting eligibility and terms remain explicit.

### M3 Pro 36-GB inference comparison contract

The working target is **under 10 GB total local-inference working set**, including
weights, active KV/recurrent state, runtime, allocator cache and peak loading,
not just model download size. Report pressure and swap deltas alongside process
and MLX allocation figures, without adding overlapping counters together.

Compare only these initial paths, one active model/process at a time:
- Existing mlx-serve as the control, using an installed small checkpoint.
- Existing `llama-server` with Metal and a matched supported GGUF checkpoint,
  prioritizing standard harness API compatibility. Installed binary presence
  does not prove a suitable model or direct-logit API is available.
- MLX-LM/native SemIf scoring as a candidate for high-frequency closed-set
  questions, not an automatic replacement for the general inference server.
  Installing its pinned environment or downloading weights needs approval.

MLX is a compute framework, mlx-serve a particular serving implementation, and
SemIf a scoring approach. A defect in one does not reject the others. OpenAI API
compatibility likewise does not guarantee tokenizer, logit, batching, prefix
cache or cancellation equivalence. Do not add an adapter until a real consumer
needs a missing operation. Start on demand, not with another always-on daemon.

Reuse dotfiles' local-model evaluator and its September 8 measurement errata.
Historical `bytesResident` was provider metadata, not measured RSS; legacy schema
checks were insufficient; throughput trials had unequal output lengths. These
receipts do not establish the current fleet's reliability or memory budget.

Jcode already has `crates/jcode-s1-eval` and `../P2_S1_EVAL_PACKET.md`: a
deterministic factory-intake baseline and prior model-arm receipts. That packet
records a rejected Needle arm and a Jev development-set result, with independent
holdout still required. Reuse the evaluation discipline, but do not confuse
factory intake accuracy with harness-route classification or inherit its old
spend approval for a new experiment.

Acceptance workload: small sanitized task-family/skill ranking and abstention
fixtures, with explicit allowed labels and deterministic expected boundaries.
Measure cold load, cold/warm prefill, p50/p95 end-to-end latency, schema/semantic
accuracy, missing evidence, shuffled labels, prefix changes, concurrency 1/2,
cancellation, restart and idle residency. For logits, compare fresh and cached
choices. For generation, use matched prompts/output lengths. Report errors and
timeouts in the denominator. Judge recurring saved cloud work against RAM,
energy, startup and maintenance costs. Do not extrapolate decode tokens/sec to
classification latency or MTP gains to a no-generation scorer.

Deterministic policy owns credentials, tools, files, effects, spend and route
eligibility. Classifiers may label, rank or abstain, never grant permission.
Cache keys must cover model/tokenizer revision, schema/options, prompt template
and exact relevant state. Stable prefixes help only when this preserves meaning.
An unavailable classifier returns abstention, not silent remote fallback.

### First implementation and stop criteria

First bounded source slice: eliminate the twelve inert profile warnings after
metadata proof and repeat the same coordinated build. No runtime behavior change
or reload should be required for that manifest-only cleanup. Then select one
reproduced runtime-identity or host-gate defect, or the smallest existing
packet/admission slice, using expected effect and exact validation before editing.
Do not build a new routing framework or automatic classifier service from this
research packet. Cheap-model promotion waits for enforced eligibility/spend and
accepted-task evidence, not attractive advertised token prices.

Status: plan, 2026-09-19, revised same day after operator approval and live router smoke. Extends `docs/HARNESS_LOOP_ARCHITECTURE.md` (the
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

## 8. Live router smoke, 2026-09-19 [measured]

Operator raised the OpenRouter key cap to $10/day and approved trying the
routers. Four requests, identical coding prompt, `provider.zdr = true`,
`data_collection = "deny"`, `require_parameters = true`, `session_id` set,
`X-OpenRouter-Metadata: enabled`. Raw responses in
`~/.jcode/scratch/router-smoke-20260919/`.

| Request | Served model / provider | Cost | Notes |
|---|---|---|---|
| `openrouter/auto-beta`, `cost_tier: low`, `excluded_models: [openai/*, anthropic/*, google/*, x-ai/*]` | `xiaomi/mimo-v2.5` / Novita | $0.000053 | `task_type = code:general_impl` returned in metadata; 192/266 prompt tokens cached on first call, 256 on the second (session stickiness works) |
| same, second session | `xiaomi/mimo-v2.5` / Novita | $0.000043 | exclusions honored |
| `openrouter/pareto-code`, `min_coding_score: 0.4` (medium tier) | **`openai/gpt-5.6-sol` / Azure** | $0.00207 | Premium family at API rates. Pareto has no exclusion field. This is the case `is_banned_router_family` exists to prevent |
| `openrouter/pareto-code`, `min_coding_score: 0.2` (low tier) | `google/gemini-3.8-flash` / Google | $0.00045 | 40x the auto-beta cost on the same prompt, mostly reasoning tokens |

Naming, to avoid the confusion the operator flagged: **Pareto Router** here is
OpenRouter's own `openrouter/pareto-code` slug (coding shortlist by
Artificial Analysis percentile, cheapest in tier). It is unrelated to the
third-party "Pareto" blended-model product. Both routers are OpenRouter
software; neither is a model.

Decisions the smoke forces:

- **Auto-beta with `excluded_models` is the admitted discovery lane.** It
  honors exclusions, returns the classifier's `task_type` for free, sticks to
  a model per session, and lands on open-weight models at low tier.
- **Pareto is admitted only with a post-hoc receipt check.** The medium tier
  resolved to a subscription family we already pay for. Until Pareto gains
  exclusions, a served `model` matching the banned families closes the
  attempt with `ReceiptError` and the cost is logged as a routing violation.
  Pareto low tier is acceptable for `tests`/`extraction` classes; medium is
  not admitted for anything until the shortlist is re-checked weekly.
- **Account defaults belong in the dashboard too** (Routing page: excluded
  models `openai/*, anthropic/*`, "prevent overrides" on) so a misconfigured
  request cannot bypass the exclusion. The request-level exclusion stays as
  defense in depth.

## 9. Division of work with the dotfiles plan

The dotfiles agent committed
`docs/plans/2026-09-19-agent-token-economy-optimization.md` (dotfiles
`4b717f5`). Its sections 3, 5, 6, 7 own policy, lane tables, the credit-burn
program and the promotion screen. This document owns the harness layer.
Later agents implement one packet each; packets do not cross the boundary.

| Dotfiles slice | Jcode packet here | Crate(s) | Acceptance |
|---|---|---|---|
| R1 per-repo data class | **J1** `[[data_class]]` config plus `DataClass::for_path` | `jcode-config-types`, `jcode-attempt-types` | Unlisted path Private; listed root Public; Secret never eligible |
| R2 router admission | **J2** `RouteClass::DynamicRouter { exclusions, tier }`; `is_banned_router_family` accepts a router only when exclusions cover the banned families or the tier is low; `validate_receipt_for_gate` rejects a served model in a banned family | `jcode-attempt-types` | Tests: auto-beta with exclusions admitted; pareto medium refused at admission; served `openai/*` on any router rejected post hoc |
| R3 request shaping | **J3** `ProviderRouting` gains `zdr`, `data_collection`; request gains `plugins`, `session_id` = Jcode session id, metadata header; stream parses response `model`, `provider`, `usage.cost`, `task_type` into the receipt (`binary_id` = served `provider:model`) | `jcode-provider-openrouter`, `-runtime` | Frozen-request test asserts body; stream test asserts receipt fields from a fixture SSE |
| R4 effort to route | **J4** `[agents.effort_routes]`; spawn resolves effort to a route unless `model` is explicit | `jcode-app-core` swarm spawn, `jcode-config-types` | Each effort resolves to the mapped route; explicit model wins |
| R5 spend reservation | **J5** `CommSpawn` carries `max_micro_usd`, `deadline_secs`, `data_class`; `LocalLedger` reservation summed per plan; plan halts at cap | `jcode-protocol`, `jcode-app-core`, `-openrouter-runtime` | Plan halts at cap; ambiguous settlement stays reserved |
| R6 attribution | **J6** OpenRouter per-day, per-served-model USD from `usage.cost` and the generation endpoint; `jcode usage --json` rows; unpriced count surfaced | `jcode-base/usage`, `model_usage.rs` | Every OpenRouter request has USD or explicit `unpriced` |
| R7 context diet | **J7** worker system prompt = packet only; tool schemas limited to `allowed_tools`; per-tool output byte cap with continuation; stable prefix hash test; proactive compaction default for workers | `jcode-app-core` | Mean input per worker turn under 40k on the three screening tasks; prefix hash stable across two turns |

Order: J2, J3 first (they gate any router traffic and are pure library work,
testable offline). J1 and J5 next (they gate repository content and spend).
J4, J7 when the window resets and Sol can review. J6 runs alongside.

Coordination rule: this repository does not edit `policy/*.md` or the swarm
prompt; the dotfiles agent does not edit crates. Findings cross over as
measurements (`docs/measurements/` in dotfiles, `~/.jcode/scratch/` receipts
here) and as the served-model distribution from J3/J6.

## 10. Post-exclusion check and lane policy, 2026-09-19 [measured]

Operator set account-level Auto Router exclusions for `openai/*` and
`anthropic/*`. Three more requests after that change:

| Request | Served | Cost |
|---|---|---|
| `pareto-code`, `min_coding_score: 0.5` | `openai/gpt-5.6-sol` / Azure | $0.00246 |
| `pareto-code`, `min_coding_score: 0.9` | `openai/gpt-5.6-sol` / Azure | $0.00147 |
| `auto-beta`, `cost_tier: medium`, no request-level exclusions | `xiaomi/mimo-v2.5` / Novita | $0.000023 |

**The Auto Router exclusion list does not bind `pareto-code`.** Pareto has
its own shortlist and no exclusion field. Consequences for the route table:

- `openrouter/pareto-code` is **not admitted** at any tier for now. J2's
  post-hoc rejection would catch it, but it would still pay for the leaked
  request. Re-evaluate only if OpenRouter adds exclusions to the plugin.
- `openrouter/auto-beta` (and `auto`) is the sole dynamic lane. Account
  exclusions bind it; request-level `excluded_models` stays as defense in
  depth. `cost_tier: low` for extraction, summary, triage, docs;
  `cost_tier: medium` for implementation and tests until the served-model
  ledger (J3/J6) shows which pinned open-weight model wins each class.
- Frontier families (OpenAI, Anthropic) leave the metered lane entirely. Via
  OAuth they remain the captain (Astra), the credit-burn lane (Terra, Sol,
  image generation where useful) and, later, a selective escalation for
  complex work. J4's `effort_routes` default therefore becomes:
  `none/minimal/low -> auto-beta low`, `medium -> auto-beta medium`,
  `high -> openai-oauth:gpt-5.6-terra` while credit remains, then
  `auto-beta medium`, `xhigh/max -> openai-oauth:gpt-5.6-sol`.
- Pinned workhorse candidates for promotion once J3 records served models:
  DeepSeek V4 Flash 0731 (already pilot-admitted), GLM, Kimi, and whatever
  auto-beta actually serves for `code:*` classes (today: `xiaomi/mimo-v2.5`).

## 11. Classifier-fed routing (operator direction, 2026-09-19)

The lane split in section 10 is itself a classification problem, so it
should be decided by a classifier, not by the caller guessing an effort.
Two signals already exist, one is free:

| Signal | When | Cost | Use |
|---|---|---|---|
| `auto-beta` `task_type` (~30 labels, e.g. `code:debugging`, `agent:multi_step_planning`) | After dispatch, in the response metadata | free | Ground-truth log for calibrating our own pre-dispatch classifier; per-class served-model and acceptance ledger (J3/J6) |
| Jev Choice over a closed label set | Before dispatch, on the packet | ~$0.04/M input | Pre-dispatch routing on **public/synthetic** packets (D3); labels map to `cost_tier`, tool envelope and verify gate |
| Local S1 (Ornith 9B on mlx-serve) | Before dispatch | wall time only | Same label set for **private** packets; abstain hands off to the captain |

Packet **J8** (after J3): `task_class` becomes a first-class field on
`CommSpawn` and DAG nodes; when absent, admission calls a classifier chosen by
data class (Jev for public, local for private, deterministic template match
first for both). The classifier's label selects among routes the route table
already permits and nothing else: no authority, tools, data class or budget
come from the label. Calibration: compare pre-dispatch labels against
`auto-beta` `task_type` and against acceptance outcomes weekly; a label with
under 80% agreement on a class is demoted to advisory for that class.

## 12. Provider classifiers and replaceable model choices (2026-09-19)

Operator direction: combine provider-layer classification and auto routing
with harness System 1 classification. Optimize accepted work, including
review cost and friction, rather than tokens or credit consumption alone.
New model choices should be replaceable per task category without changing
the authority or acceptance contract. This extends J6/J8 planning, not J5.

Verified against OpenRouter's public documentation on 2026-09-19:

- [Custom classifiers](https://openrouter.ai/docs/guides/features/classifiers)
  run asynchronously **after** a generation, using a serialized prompt.
  They do not route that same generation. Use their tags as retrospective
  observations for evaluation and future routing, not ground-truth labels.
- The operator reports six available presets. The public page currently
  enumerates five: Department, Audience, Engineering work, Agent complexity,
  and Capitalizable software expense. Confirm workspace availability before
  selecting one. Custom taxonomies support up to eight dimensions.
- Classification is separately billed to the configuring administrative
  user, not a particular API key. Do not assume a workload key's spend cap
  bounds classifier charges. Sampling, cost accounting and a verified
  applicable budget control are prerequisites to activation.
- [ZDR controls](https://openrouter.ai/docs/guides/features/zdr) apply to
  inference provider routing, not automatically to enabled plugins/tools.
  The classifier documentation does not establish inheritance of the
  originating request's ZDR or exclusion controls. Verify classification
  processing, routing and retention separately before activating it.

### Bounded routing feedback loop

1. Harness classification remains pre-dispatch: deterministic templates
   first, then an admitted local classifier for private packets or Jev for
   public/synthetic packets. Labels are constrained, advisory and may abstain.
2. Admission selects only from an explicitly approved candidate set using
   task class, data policy, tool envelope, budget and required review. The
   provider's auto router supplements this decision inside those constraints.
3. J6 joins served-model, cost, classifier version/labels, acceptance and
   review effort where available. Missing tags or unpriced calls stay explicit.
   Confirm supported tag retrieval before assuming API integration exists.
4. Evaluate candidate replacements offline or in separately authorized bounded
   trials. Promote per category only after acceptance, privacy and total-cost
   gates pass. Pin the chosen treatment for each task; no mid-task model
   substitution or automatic fallback when availability changes.

### Free-model candidate lane

Use genuinely free endpoints when their current endpoint policy satisfies
ZDR and data-collection denial, supports required parameters, and is explicitly
admitted. A `:free` suffix or catalog listing is not privacy evidence. Check
current endpoint eligibility, enforce request controls and fail closed when
none qualify. Never relax privacy or silently switch to a paid endpoint.

Start with public/synthetic, low-stress proposals: fixtures, test cases,
documentation drafts, extraction and bounded generation. No credentials,
repository writes, arbitrary shell, routing authority or external effects.
Trusted review plus deterministic checks owns acceptance and integration.
Count reviewer effort, retries, latency and rate limits in the comparison.
Free inference is useful only when accepted output costs less overall.

No classifier, new route, free-model trial or account setting is activated by
this plan. Implementation follows existing admission and approval gates.

### E1 regression baseline integrated (2026-09-20)

Five offline tests now exercise real `spawn_swarm_agent` startup with a shared
capture provider and isolated synthetic policy. They cover packet and required
ancestor policy inclusion, captain-canary exclusion, repeatable static/tool
prefixes, actual tool-description revision invalidation, captured policy stability
and fresh-spawn invalidation, and unsupported cache metrics remaining `None`.
Dynamic prompt data is hashed separately from the static/tool prefix.

The three synthetic task packets, each repeated twice, measured median captured
serialized request sizes of 47,955 bytes with inherited tools versus 6,095 bytes
with explicit relevant tools. This measures an **existing allowlist capability**
at the high-level Provider boundary, not adapter wire bytes, cache hits, real-task
quality, or new production savings. This change is tests only. Durable output
continuation and persisted resume policy/schema envelopes remain separate gaps.
No new production receipt API or prompt traversal changes were necessary.

Captain acceptance after moving the fixture declaration under the existing test
module: five E1 tests and four adjacent spawn narrowing tests passed. Scoped
rustfmt, diff whitespace, production code-size and test-size gates passed.
Earlier focused checks passed three fingerprint and 45 prompt tests. This is not
a full-workspace verification claim. The existing Apple linker `__eh_frame`
warning and missing host-wide `flock` gate remain unresolved. No reload is needed
for this test-only slice.
