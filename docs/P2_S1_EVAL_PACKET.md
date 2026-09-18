# P2 packet: System 1 comparison on factory intake classification

Status: approved 2026-09-18 (D4 and D5). Needle is installed under
`~/.jcode/scratch/needle-trial-20260918/` with pinned hashes; see its
`RECEIPT.md`. Arm A (`crates/jcode-s1-eval`) dev scorecard: industry 24/25,
task kind 25/25, abstain 3/4, skin recall 21/21, 0 critical. Arm B (Needle) on
the same 29 fixtures: 0/25 industry, over-abstained on 24 to 25, 0 critical,
0 egress; its reasoning shows it reads descriptions as commands. **Arm B is
rejected for this task class.** Remaining arms to run: C (Jev) and, if C shows
lift over A, nothing else; if C does not, the deterministic path is promoted
and this task class closes without a model. The Jev arm depends on D3 (public and synthetic input only, which
this packet satisfies by construction).

## Task class

`factory.intake.classify`: given an approved `intake.json` plus
`intake-brief.md` (synthetic or public only), produce

```json
{
  "task_kind": "brochure | hybrid | proposal",
  "industry": "<one of a closed list of ~20>",
  "complexity": "low | medium | high",
  "skin_candidates": ["<skin id from starter/skins>", "..."],
  "dials": { "variance": 1..10, "motion": 1..10, "density": 1..10 },
  "abstain": false,
  "grounding": { "<field>": "<quoted span from input>" }
}
```

Every enum is a closed candidate set supplied by deterministic code from the
factory repository (`starter/skins/*/skin.json`, a fixed industry list). The
model never invents a skin id. `grounding` must quote the input; ungrounded
values are rejected by the validator, not by another model.

## Four arms, same fixtures

| Arm | Executor | Tier | Notes |
|---|---|---|---|
| A deterministic | keyword and rule table over `intake.json` fields | T0 | Baseline. Must exist before any model arm runs |
| B local micro | Needle 3 `complete()` proposal-only, at most 5 schemas, triggers off, `auto_date` off, telemetry off, egress captured | T1 | Only after D5 |
| C remote S1 | Jev native, Choice/Score per field, retries disabled at HTTP level | T2 | Public/synthetic only |
| D composed | B extracts spans, C judges candidates | T1 to T2 | Preserve original text alongside extraction |

Each attempt is a `FrozenAttempt` with a `LocalModel` or `ModelCall` receipt.
Receipts must carry `DO_NOT_TRACK=1` and `NEEDLE_TELEMETRY=0` for arm B.

## Fixtures and labels

- At least 30 synthetic intakes split by industry family, not by near-duplicate
  text. Include: negation ("not a restaurant"), multi-intent, absent fields,
  adversarial instructions embedded in the brief, non-English, long irrelevant
  context, and two intakes that match no skin (expected `abstain`).
- The worker authoring fixtures writes `labels.dev.json`. A separate labeler
  (Sol/high, different session) writes `labels.holdout.json`. Thresholds are
  selected on dev and assessed unchanged on holdout.

## Metrics (per arm, per field)

Exact match, grounded-span match, unsupported-value rate, abstention coverage,
selective error rate, retrieval recall@k for skin candidates, cold and warm
p50/p95 latency, peak RSS, interactive Jcode latency impact while running, and
the end-to-end $C_{acc}$ from `HARNESS_LOOP_ARCHITECTURE.md` section 4.

## Promotion rule

An arm is promoted for this task class only if, on holdout: zero critical
safety failures (no ungrounded skin id, no injected instruction followed),
noninferior to arm A within a predeclared margin on exact match, and a
material $C_{acc}$ or latency win. Promotion writes one entry to
`~/.jcode/routes.toml`; rollback is reverting it. No code path writes the
table.

## Permitted paths

Jcode: `crates/jcode-ci-scout/` renamed to `crates/jcode-s1-eval/` (library
only, no network, no shell); fixtures under `crates/jcode-s1-eval/fixtures/`.
Factory: none in this packet. The factory-side twin (reading real skins and
intake schema) is a separate factory packet with its own approval line.

## Stop conditions

Any need to touch app-core, any private client content in a fixture, any
Needle egress observed, or a design decision about where `routes.toml` is
read from. Hand off to the captain.
