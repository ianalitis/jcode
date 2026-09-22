# W3: a laya-backed local decision arm

**Date:** 2026-09-22. **Status:** not started; the install is gated on operator
approval, and the runtime boundary in §4 is still a decision.
**Contract and harness:** implemented. `crates/jcode-s1-eval/src/decision.rs` carries
W1 and W5; `crates/jcode-s1-eval/fixtures/decisions.dev.json` is the fixture set the
acceptance names.

## 1. What is decided, and by whom

| Decision | State |
| --- | --- |
| The contract (`DecisionRequest` / `DecisionResult`, `Noul`/`Choice`/`Score`, `authority: advisory`) | W1 implemented and tested, commit `e1dd4c631` |
| A third Jev consumer key for routing decisions | W2 implemented and tested, same commit |
| Calibration is required before a threshold gates | W5 enforced by the contract's schema |
| The local arm's base model | **laya**, operator decision 2026-09-22, over SemIf's 4B |
| The runtime boundary, the footprint ceiling, D1/D2/D3 | open, see §4 and §8 |

laya (`github.com/NandhaKishorM/laya`, Apache 2.0) is a non-autoregressive encoder
family: `laya` (ModernBERT-large, 421M, English), `laya-multilingual` (mmBERT-base,
322M), `laya-typed-decisions` (421M). It answers `choice` / `score` / `noul` in one
forward pass, which is exactly the contract's vocabulary, and our option cap of 16
sits inside its design point: its own README documents that high-cardinality label
spaces degrade above roughly 20 options, so a 2..=16 closed set is what it does well.

## 2. Goal and non-goals

Goal: a local-only arm behind the W1 contract that returns typed option scores for a
fixture, with zero network, and whose footprint is measured during load.

Non-goals, from the packet's unchanged constraints: no second scheduler, router
service or daemon; no always-on tool schema; no gating authority (W5 refuses a
threshold without a calibration reference); no private input on a metered route and no
cloud arm as its fallback; `mlx-serve` stays single-slot under its existing ceiling.

## 3. Acceptance

1. `score_decisions(&LayaArm, &bundled_decision_fixtures())` returns a scorecard with
   `invalid == 0` and `critical == 0`: every result satisfies the contract, no arm
   output names an option it was not given.
2. Zero network. The child runs with no credential in its environment and no endpoint
   configured; a network attempt must fail closed rather than quietly succeed.
3. Footprint measured during load, not inferred: peak RSS and wall time for one batch
   over the fixture set, written to a receipt.
4. The floor to beat is measured: the deterministic baseline scores **5 of 10** on the
   dev set (1 abstention, 0 invalid, 0 critical), pinned in
   `decision::tests::the_baseline_satisfies_the_contract_on_the_whole_dev_fixture`.

## 4. The runtime boundary (recommended, not yet decided)

Two constraints leave one shape. "No second scheduler, router service or daemon" rules
out an always-on sidecar or a resident MPS server; the contract lives in a crate that
is documented as pure (no network, no shell, no model calls), so the arm cannot be an
in-process Python call.

Recommended: **one short-lived child process per batch**, speaking the W1 types as
JSON lines, one request per line on stdin and one result per line on stdout. The Rust
side owns validation, the wall-clock bound and the fail-closed policy; the child owns
only the forward pass and never decides anything. A batch of 10 fixtures is one
process, so the load cost is paid once per batch and not once per question.

Open sub-questions, to settle before writing it:

- **Where the runner lives.** Not in `jcode-s1-eval`, which would break its stated
  purity. `crates/jcode-base/src/sidecar.rs` is a *cloud* client, so there is no
  existing local-process seam to reuse; the options are a small new crate or a module
  in `jcode-base`. This needs a decision, not a default.
- **The error surface.** A child that crashes, times out or answers unparseably is an
  abstention with a recorded error, never a default-allow. Where the caller's policy
  lives is a caller's decision; the contract only insists that the failure is visible.
- **Batching key.** Whether the runner batches by fixture set (simplest, and what the
  acceptance measures) or by session.

## 5. The install gate (operator approval required, not given)

- A virtualenv outside the repo, `pip install laya`, which brings `transformers` 5.x
  and `torch` 2.14. A CPU-only torch wheel would cut the download but lose MPS.
- Measured today: `python3` here is **3.14.7** with no `torch`, no `transformers` and
  no `mlx`. `torch` 2.14.0 declares `requires_python >=3.10`, but whether cp314 macOS
  arm64 wheels exist is **unverified**; if they do not, the arm needs a 3.12 or 3.13
  interpreter instead. Check the wheel, not the metadata, before promising a timeline.
- Nothing in this repository downloads weights. The checkpoints come from Hugging Face
  (`convaiinnovations/laya`, `-multilingual`, `-typed-decisions`) and must be fetched
  explicitly, with the operator watching, so the download is attributable.

## 6. Quality and calibration: the first arm reports, it does not gate

laya's own README is the reason to be careful. Its base checkpoints are **near chance
zero-shot** on typed decisions (0.362 and 0.352 against a 0.318 random and a 0.461
majority-class baseline), and its headline 0.766 comes from a checkpoint fine-tuned on
that benchmark's own training split. It is, in its authors' words, a fast base to
specialise, not a zero-shot decision engine. It also documents a confident-and-wrong
mode (Khmer: 0.000 accuracy at 0.952 confidence), which is the failure W5 exists for.

Three consequences, all already enforced or recorded:

1. The dev set measures fit only. `fixtures/HOLDOUT-PROTOCOL.md` governs the rest: a
   quality claim needs a holdout authored by a session that has not read the arm,
   scored once per revision.
2. Our 20-case holdout is burned by the earlier 4B trial, so **D2 must be answered
   before any quality claim**, not after.
3. No threshold gates until a `calibration_ref` is fitted on dev and frozen. Until
   then the arm runs with `threshold: null` and `calibration_ref: "uncalibrated"`,
   which the schema accepts for reporting and refuses for gating.

## 7. Evidence commands

```sh
cargo test -p jcode-s1-eval                      # 23 tests: contract, harness, fixtures
cargo test -p jcode-s1-eval -- --nocapture the_baseline   # the pinned floor lives here
# once the arm exists, the footprint receipt is measured, not inferred:
#   peak RSS of the child during load, and wall time for one batch over the fixture set
```

## 8. Open questions for the operator

- The runtime boundary in §4, specifically where the runner lives.
- **The footprint ceiling is not in this repository.** The constraint says "keep
  `mlx-serve` single-slot under the existing footprint ceiling"; no file defines that
  number, so it cannot be checked against. Name the number, or name the measurement
  that sets it.
- D1 (admit the contract at all), D2 (a fresh adjudicated holdout) and D3 (whether any
  cloud Jev arm is admitted before spend enforcement exists) are unchanged and are the
  operator's. Q1/Q2 belong to the routing measurement contract and do not block this.
