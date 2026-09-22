# W3: a laya-backed local decision arm

**Date:** 2026-09-22. **Status:** not started; the install is gated on operator
approval. The runtime boundary and the footprint ceiling both have measured defaults
now (§4, §8) so that only the install approval still blocks writing the runner.
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

Acceptance 2 is checkable without the model and should be written that way: the child
inherits a scrubbed environment (`env_clear` plus an explicit allowlist), no
`HF_TOKEN`/`HUGGING_FACE_HUB_TOKEN` and no proxy variable, and the test asserts the child
process's environment rather than trusting the model not to call out. Acceptance 1 needs the
weights, so it is the only one that the install approval actually gates.

## 4. The runtime boundary (one shape, one default, still the operator's to redirect)

Two constraints leave one shape. "No second scheduler, router service or daemon" rules
out an always-on sidecar or a resident MPS server; the contract lives in a crate that
is documented as pure (no network, no shell, no model calls), so the arm cannot be an
in-process Python call.

Recommended: **one short-lived child process per batch**, speaking the W1 types as
JSON lines, one request per line on stdin and one result per line on stdout. The Rust
side owns validation, the wall-clock bound and the fail-closed policy; the child owns
only the forward pass and never decides anything. A batch of 10 fixtures is one
process, so the load cost is paid once per batch and not once per question.

The child is thin, and that is now verified rather than assumed: laya 0.3.5's entire
inference surface is `Agent.system_one(state, questions)`, taking `choice`/`score`/`noul`
questions in an internal dict form and returning answers plus probabilities. So the child is
`json.loads` per line, one mapping of the contract's `options` into `criteria`, one
`system_one` call, one mapping back. It holds no policy: it does not threshold, rank against
a baseline, or decide whether to abstain. `max_len` defaults to 512 and `head_max_len` to 192
in the checkpoint config, which is worth recording because an option set whose rendered
markers exceed the head length makes `system_one` raise rather than truncate. That is a
contract-shaped failure (the arm cannot answer the question it was given), and the runner
should surface it as a recorded error, not as an abstention it chose.

Open sub-questions, to settle before writing it:

- **Where the runner lives.** Not in `jcode-s1-eval`, which would break its stated
  purity. Three real options, with the evidence:

  | Option | Cost | Evidence |
  | --- | --- | --- |
  | New crate `jcode-s1-laya-runtime`, depending only on `jcode-s1-eval` and tokio/std | workspace membership plus a line in `CRATE_OWNERSHIP_BOUNDARIES.md` | mirrors the existing `jcode-provider-*-runtime` naming; the ownership doc's primary goal is shrinking the root's recompile surface, and this adds no fan-out into the root |
  | A module in `jcode-base`, next to `jev.rs` | grows the crate that is already the largest | keeps both decision transports in one place and reuses the Jev pattern (purpose, bounded request/response, fail-closed); `crates/jcode-base/src/jev.rs` is the closest precedent for a decision transport |
  | The root crate under `src/` | the root must take a dependency on `jcode-s1-eval` | matches the ownership doc's literal rule that process-spawning behavior stays in the root until a boundary can move cleanly |

  Recommendation: the new crate. The runner is a transport, the contract crate stays
  pure, and neither the root nor `jcode-base` grows. There is no existing local-model
  runner seam to reuse: `crates/jcode-base/src/sidecar.rs` is a cloud client, and the
  subprocess calls elsewhere in `jcode-base` are auth and background helpers, not
  inference transports.
- **The error surface.** A child that crashes, times out or answers unparseably is an
  abstention with a recorded error, never a default-allow. Where the caller's policy
  lives is a caller's decision; the contract only insists that the failure is visible.
- **Batching key.** Whether the runner batches by fixture set (simplest, and what the
  acceptance measures) or by session. Default: batch by fixture set now, because that is
  what the acceptance measures and a session-scoped runner would need a lifetime for the
  child that the "no daemon" constraint makes awkward. Revisit only if a real caller needs
  lower per-question latency than one load per batch gives.

## 5. The install gate (approval still required; feasibility now measured)

Re-measured 2026-09-22 against PyPI metadata and the actual distribution files, because
the packet's own warning stands: check the wheel, not the metadata.

- **The cp314 risk is closed.** `torch` 2.14.0 ships `torch-2.14.0-cp314-cp314-macosx_14_0_arm64.whl`
  (127.3 MB) and a `cp314t` variant (127.7 MB). This host is macOS 26.6.2 on arm64, so the
  platform tag is satisfied. No interpreter downgrade is needed; `python3` 3.14.7 (mise) works.
  A CPU-only `+cpu` wheel does not exist for macOS arm64: there is one wheel family and it
  carries MPS, so the "cut the download, lose MPS" trade in the packet is not a real choice
  here. It is either this wheel or no torch.
- **`laya` on PyPI is the right project, and it is tiny.** `laya` 0.3.5, Apache-2.0, author
  "Convai Innovations", homepage `huggingface.co/convaiinnovations/laya`, summary "Fast,
  non-autoregressive System 1 decision engine with calibrated probabilities". Its wheel is
  **40.7 KB of Python** (8 modules: `agent`, `common`, `email`, `lang`, `presets`, `router`,
  `shortlist`). Nothing is vendored; the checkpoints come from the Hub.
- **Declared dependencies are broad, so pin them rather than letting pip resolve.**
  `laya` 0.3.5 declares `torch>=2.0.0`, `transformers>=4.48.0`, `safetensors>=0.4.0`,
  `huggingface_hub>=0.20.0`, `numpy>=1.20.0`. An unpinned install pulls `transformers` 5.17.0
  and `huggingface_hub` 1.32.0. PyPI classifiers stop at Python 3.13 for both and the wheel
  is `py3-none-any`, so 3.14 is untested by the publisher: pin the versions, do not float.
- **The arm's real API is one call.** `laya.load(model_id, subfolder=...)` returns an `Agent`
  whose whole inference surface is
  `system_one(state, questions) -> {answers, probabilities, calibrated confidence, token usage}`,
  taking `choice` / `score` / `noul` questions in the contract's own vocabulary. The runner
  therefore needs no adapter layer beyond mapping `DecisionRequest` to that dict and back.
- **Download cost, measured from the Hub's own file listing:**
  `convaiinnovations/laya` 2.37 GB (38 files), `-multilingual` 0.68 GB, `-typed-decisions`
  0.85 GB. All three are 3.9 GB; the base checkpoint alone is 2.37 GB, and that is the one
  W3's acceptance needs. Nothing in this repository downloads weights, so the fetch stays an
  explicit, attributable, operator-watched step. For scale, the resident MLX lane's own
  models are 0.35-5.5 GB, so a 2.37 GB checkpoint is not an outlier on this disk (554 GB free).
- **One correction to §1/§9's framing:** `laya-multilingual` and `-typed-decisions` are
  *subfolders of the same HF repo* (`laya.load("convaiinnovations/laya", subfolder="multilingual")`),
  not three separate repositories. The download budget is one repo, not three.

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

- **Where the runner lives.** Not a policy question any more, only a placement preference:
  §4 recommends a new `jcode-s1-laya-runtime` crate, and the one-line diff to change that is
  known. A default is proposed so this does not block writing code.
- **The footprint ceiling: found, and it is not in this repository.** The constraint's source
  is the packet `~/dotfiles/docs/packets/jcode-decision-contract-and-local-arm.md`, and the
  number it defers to lives in the operator's local-lane wrapper,
  `~/dotfiles/home/dot_local/bin/executable_mlx-local`. Measured today:

  | Fact | Value |
  | --- | --- |
  | `mlx-serve` resident budget | `--max-resident-mem 20GB`, `--max-resident-models 2` |
  | Physical RAM | 36.0 GB |
  | History that set those flags | 2 co-resident servers budgeted 40 GB against 36 GB; a 27B load once failed 503 while the box held 2.7 GB swap |
  | Live now | `mlx-serve` up on 127.0.0.1:11234 holding **0.02 GB** (zero-weight process) |
  | Live free memory | 90% free; swap 880 MB of 2048 MB used |

  So the ceiling is real but it is a *lane* budget, not a number the arm can be checked
  against directly: `mlx-serve` may legitimately hold up to 20 GB, and the arm must not be
  the reason the box crosses into swap. The honest form of the constraint is therefore a
  ceiling **on the arm**, and it needs one number from the operator. Proposed, on the
  evidence above: **peak RSS <= 4 GB for the arm's child, and never a second concurrent
  child** — laya is a 421M-parameter encoder, so 4 GB is roughly fp32 weights plus a 512-token
  batch's activations, with room to spare, and it keeps the arm plus a full 20 GB MLX lane at
  24 GB against 36 GB physical. An operator who wants a different number should name it;
  otherwise the receipt will report measured peak RSS against 4 GB and say so.
- D1 (admit the contract at all), D2 (a fresh adjudicated holdout) and D3 (whether any
  cloud Jev arm is admitted before spend enforcement exists) are unchanged and are the
  operator's. Q1/Q2 belong to the routing measurement contract and do not block this.
