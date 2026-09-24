# Benchmark research: how to measure fork changes and lane choices (2026-09-24)

Purpose: decide what evidence is good enough to change a routing default or
claim a fork change helped. Not a leaderboard chase.

## jcode-bench (primary instrument)

`1jehuang/jcode-bench` (read 2026-09-24). Each task gives a working, tested
primitive, an exhaustive verifier and a deterministic cost model; the task is
"make it faster, stay correct on every input".

- Score = log2(given_cost / your_cost), cost = callgrind instruction count, so
  it is machine-independent and deterministic. Continuous, not pass/fail.
- Live tasks: `json-unescape`, `float-print`, `utf16-transcode`.
- Contamination resistance comes from the task shape: exhaustive verification
  leaves nothing to overfit and the starting implementation is the baseline.
- **Variance is the key fact.** Upstream measured run-to-run spread of ~0.09 SD
  from agent behavior on one task, vs 0.004 SD grader noise. Runs needed at
  ~95%: gap 0.25 → k≥2, 0.08 → k≥11, 0.02 → k≥237.

Consequence for us: a single run never justifies a default change. Only
effects of ≥0.25 are affordable to detect (k=2-3 per cell); smaller claimed
gains are unmeasurable at our budget and must not drive policy.

## External practice (priors, not measurements of this fork)

- **SWE-bench Verified**: human-validated subset of SWE-bench; widely
  saturated and contaminated by now (public repos, public fixes). Useful only as
  a smoke test that a harness change did not break patch application.
- **SWE-bench Live / rolling variants**: fresh issues after model cutoffs to
  reduce contamination; expensive to run, Docker-heavy.
- **Terminal-Bench**: shell-task agentic benchmark; closer to jcode's tool
  surface (bash, files) and cheaper per task.
- Common controls: fixed harness version, fixed model/effort, pass@k with k
  reported, cost and wall-time per task reported alongside score, and
  per-task-class breakdowns rather than one aggregate.

Recheck these descriptions against their sites before citing numbers; this
section records method, not results.

## What we measure, and how

| Question | Instrument | Minimum evidence |
| --- | --- | --- |
| Did a fork harness change regress coding ability? | jcode-bench, 1 task, k=2, before/after on the same model | Drop ≥0.25 on both runs = regression |
| Is lane A better value than lane B for a task class? | jcode-bench k=3 per lane + tokens/cost from `jcode usage` | Score gap ≥0.25 or equal score at ≤50% cost |
| Does auto-delegation help? | Same task, captain-only vs captain+workers, k=3 | Score and wall time, with cost |
| Tool-surface regressions (patch apply, bash) | 5-10 SWE-bench Verified smoke tasks | Pass/fail parity only |

Every run records: jcode commit, model route, effort, bench commit, task, seed,
score curve (`scores.jsonl`), tokens and cost. Receipts go under
`docs/measurements/`.

## Execution

- Docker via colima (`aarch64` VM, Docker 29.5.2). callgrind needs Linux; run
  `--platform linux/amd64` only if the bench image requires x86 (check its
  Dockerfile first; native arm64 is much faster).
- Baseline before any routing change: Opus 5.5 high on `utf16-transcode`, k=2.
- Blocked on quota: ChatGPT resets 2026-09-27, Go weekly limit unknown. Do not
  spend Claude headroom below 20% on bench runs.

## Acceptance for this doc's follow-up (P3)

`docs/measurements/2026-09-2x-jcode-bench-baseline.md` exists with k≥2 runs,
full provenance, and the variance table reproduced for our runs.
