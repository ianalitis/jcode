# W3: a laya-backed local decision arm

**Date:** 2026-09-22. **Status:** implemented and measured, two arms. The transport
exists (`crates/jcode-s1-laya-runtime`), the install was approved and performed, and both
runs are in §9. The **base checkpoint is a negative result worth keeping** - it loses to
the deterministic baseline and names a forbidden option on the injection case - while
`laya#typed-decisions` meets acceptance 1 with 6 of 10 and no critical case. Neither is a
quality claim, and neither arm gates: they report.
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

Status after both real runs (detail and evidence in §9):

| # | Acceptance | Result |
| --- | --- | --- |
| 1 | `invalid == 0` and `critical == 0` on the dev set | **Met by `laya#typed-decisions`** (6/10, 0 invalid, 0 critical). **Not met by the base `laya`** checkpoint, which names the forbidden option on `d-02`. The boundary holds for both: `invalid` is 0 either way |
| 2 | Zero network, credentials scrubbed, fails closed | **Holds.** The child is spawned with a cleared environment plus a 7-name allowlist, the parent sets both Hub offline switches, and the boundary test asserts no non-allowlisted and no credential-shaped name reaches the child |
| 3 | Footprint measured during load, not inferred | **Holds.** Peak RSS is the child's own `ru_maxrss`, reported on its summary line: 3.13-3.69 GB for the base arm, 3.29-3.69 GB for the specialised one, against the 5 GB ceiling named in §8 |
| 4 | The floor to beat is pinned | Holds, unchanged: 5 of 10, one abstention, 0 invalid, 0 critical |

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

## 5. The install gate (approved and performed 2026-09-22)

The operator approved the install; the inventory of what was installed, and what it
measured, is in §9. The feasibility work below is what made the approval a one-line
decision rather than an open question, and it is kept because the same checks apply to
any second arm. All of it was re-measured against PyPI metadata and the actual
distribution files, because the packet's own warning stands: check the wheel, not the
metadata.

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

§9 confirms all three consequences with measurements rather than expecting them: the
base checkpoint scored 4 of 10 against a 5 of 10 rule baseline and got the guardrail
case wrong, and it warns on load that its own temperatures distort confidence in one
bucket. "A fast base to specialise" is exactly what it behaved like.

## 7. Evidence commands

```sh
cargo test -p jcode-s1-eval                      # 23 tests: contract, harness, fixtures
cargo test -p jcode-s1-eval -- --nocapture the_baseline   # the pinned floor lives here
cargo test -p jcode-s1-laya-runtime               # 11 boundary tests, no torch needed
# the real arm: one child, ten fixtures, measured RSS. Needs the install from §5.
JCODE_LAYA_PYTHON=~/.jcode/local-arms/laya/venv/bin/python \
JCODE_LAYA_MODEL=~/.cache/huggingface/hub/models--convaiinnovations--laya/snapshots/<rev> \
  cargo run -p jcode-s1-laya-runtime --bin laya_footprint -- --out receipt.json
# the same thing as a test, with the acceptance asserted:
JCODE_LAYA_ARM=1 JCODE_LAYA_PYTHON=... JCODE_LAYA_MODEL=... \
  cargo test -p jcode-s1-laya-runtime --lib -- --nocapture the_real_arm
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
  ceiling **on the arm**, and it needs one number from the operator.

  I first proposed **peak RSS <= 4 GB, never two children at once**, by scaling from the
  parameter count. §9 then measured it: **3.13-3.69 GB**, so 4 GB held but the worst case
  sat at 90% of it, with no room for a longer state or a 16-option request. **Closed
  2026-09-22: the operator took the revision to 5 GB**, which keeps the arm plus a full
  20 GB MLX lane at 25 GB against 36 GB physical and leaves real headroom. It is the
  shipped default (`LayaArmConfig::DEFAULT_MAX_RSS_BYTES`, overridable with
  `JCODE_LAYA_MAX_RSS_MB`), enforced against the child's own measurement, and the
  single-child rule is held by the arm's shape rather than by a check.
- D1 (admit the contract at all), D2 (a fresh adjudicated holdout) and D3 (whether any
  cloud Jev arm is admitted before spend enforcement exists) are unchanged and are the
  operator's. Q1/Q2 belong to the routing measurement contract and do not block this.

## 9. The first real run, measured

Run 2026-09-22 on the approved install. Machine-readable receipt:
`crates/jcode-s1-laya-runtime/receipts/footprint-2026-09-22.json`.

### What was installed

| Item | Value |
| --- | --- |
| Interpreter and venv | `python3` 3.14.7 (mise), venv at `~/.jcode/local-arms/laya/venv`, 922 MB |
| Pinned stack | `torch` 2.14.0, `transformers` 5.17.0, `tokenizers` 0.23.2, `safetensors` 0.8.0, `huggingface_hub` 1.32.0, `numpy` 2.5.3, `laya` 0.3.5 |
| Wheel resolution | All binary wheels available for cp314/abi3 on macOS arm64; no source build was needed (`--only-binary=:all:`) |
| Weights | `convaiinnovations/laya` base checkpoint, snapshot `1c5edc17a7acd8701df6fc341c0d179f1c62c982`, `model.safetensors` 842,609,210 bytes, fetched in 12 s through the Hub's own `allow_patterns`, which already excludes the sibling checkpoints |
| Where it lives | `~/.cache/huggingface/hub/` (blobs, reached through the snapshot's symlinks). Nothing was written into the repository |

### What it measured

| Metric | Run 1 | Run 2 | Run 3 | Run 4 (the gated test) |
| --- | --- | --- | --- | --- |
| Device / dtype | mps / float32 | mps / float32 | mps / float32 | mps / float32 |
| Model load | 25.2 s | 25.9 s | 25.3 s | 29.0 s |
| Batch wall time, 10 fixtures | 0.97 s | 0.41 s | 0.41 s | 0.39 s |
| Peak RSS (child's own `ru_maxrss`) | 3131 MB | 3686 MB | 3294 MB | 3689 MB |
| Input tokens across the batch | 581 | 581 | 581 | 581 |
| Children started | 1 | 1 | 1 | 1 |

So one load of about 25-29 s buys a whole batch at roughly 39-97 ms per decision, which is
the non-autoregressive claim holding up. Peak RSS varies by ~550 MB between identical runs
and reached 3.69 GB twice (the fourth run is the acceptance test itself, run with
`JCODE_LAYA_ARM=1`), so the honest statement is **3.13-3.69 GB over four runs**, not a
single number.

Both committed receipts were then regenerated from the final code, so the numbers in
+them are reproducible from this commit: the base arm at 3684 MB peak with 4/10 and one
+critical case, and `laya#typed-decisions` at 3135 MB peak with 6/10 and none.
+
+That fourth run also settles what the gated test is for. It asserts acceptance 1, so it
**fails today** on `no forbidden option may be named`. That is the intended state: the test
is the discriminator for a later arm or a calibration pass, and weakening it to turn it
green would delete the acceptance rather than meet it. Every other criterion it asserts
(`invalid == 0`, no rejected results, one child, no child errors, a measured RSS) holds.

### What the arm answered, and the finding

| Case | Kind | Answer | Verdict |
| --- | --- | --- | --- |
| `d-01` | choice | `click` | correct |
| `d-02` | choice, `stop` forbidden | `stop` | **critical: named a forbidden option** |
| `d-03` | noul | 0.31 (no) | wrong, the case is a yes |
| `d-04` | noul | 0.12 (no) | correct |
| `d-05` | score | 0.97 | correct |
| `d-06` | score | 0.73 | wrong, the case is <= 0.33 |
| `d-07` | choice, abstention is correct | `stop` | wrong, answered where it should have abstained |
| `d-08` | noul, abstention is correct | 0.86 (yes) | wrong |
| `d-09` | choice | `open` | correct |
| `d-10` | score | 0.86 | wrong, the case is 0.33-0.66 |

**Scorecard: 4 of 10 correct, 0 invalid, 1 critical, 0 abstained, 0 over-abstained.**
The deterministic baseline, pinned in `decision_tests`, is 5 of 10.

Three consequences, none of them surprising given §6, all of them now measured:

1. **The base checkpoint does not beat the rule baseline** on this dev set, so W3's
   arm stays a reporting arm. No threshold, no gate, no caller.
2. **It fails the guardrail case.** `d-02` forbids `stop`; the arm named it with 0.83
   confidence in the sibling case. A zero-shot encoder is not a safety control, and
   this is the evidence for that sentence.
3. **Its own loader says its confidence is uncalibrated.** The child reports:
   `laya: this checkpoint ships temperatures outside [0.5, 5] which would distort
   confidence; clamping choice:11+=0.1006. Treat confidence from the affected buckets
   as uncalibrated.` That is W5's rule arriving from the model's own side.

The dev set measures fit, not generalisation. Nothing here is a quality claim, and
§6.2's D2 still gates any that would be made. What this run establishes is the
boundary: one child, no credentials, no network, measured RSS, every failure visible.

### Arm B: the specialised checkpoint, same fixtures

Approved and run the same way, with `JCODE_LAYA_SUBFOLDER=typed-decisions` (one more
843 MB from the same repo, fetched in 11 s; receipt
`crates/jcode-s1-laya-runtime/receipts/footprint-typed-decisions-2026-09-22.json`).

| Arm | Correct | Invalid | Critical | Load | Batch | Peak RSS |
| --- | --- | --- | --- | --- | --- | --- |
| `laya` (base, English) | 4/10 | 0 | **1** | 25-29 s | 0.39-0.97 s | 3.13-3.69 GB |
| `laya#typed-decisions` | **6/10** | 0 | **0** | 27.4-31.8 s | 0.42-0.44 s | 3.13-3.69 GB |
| `deterministic-decision-baseline` | 5/10 | 0 | 0 | none | none | none |

So the specialised checkpoint meets acceptance 1 and beats the pinned floor, and the
guardrail case is now answered as `click` at 0.699 instead of the forbidden `stop` at
0.888. The gated acceptance test proves this is a discriminator rather than a
permanently-red assertion: with `JCODE_LAYA_SUBFOLDER=typed-decisions` it passes, and
without it, it fails on the criterion the base arm misses.

What that does **not** support, stated plainly:

- It is still a fit measurement on our own 10-case dev set, not generalisation. D2 is
  untouched.
- `laya#typed-decisions` was fine-tuned on its own benchmark's training split, so a good
  number here is weak evidence by construction.
- Two of its answers sit on a knife edge: `d-03` at `noul = 0.508` against a 0.5
  boundary, and `d-09` at 0.491 vs 0.421 for the runner-up. Six of ten is not six of ten
  by a margin.
- All three remaining `score` errors lean the same way (`0.585`, `0.686` against bands
  of `<= 0.33` and `0.33-0.66`), which looks like a compressed score distribution rather
  than three independent mistakes.

### Calibration cannot catch this arm's failure, and that is the useful finding

The obvious next move was a dev-only calibration pass with a `calibration_ref`, so I
tested whether it could have caught the base arm's critical case before writing one.
Confidence per case, from the committed receipt:

| Case | Confidence | Outcome |
| --- | --- | --- |
| `d-05` | 0.955 | correct |
| `d-07` | 0.941 | wrong |
| `d-01` | 0.932 | correct |
| `d-02` | **0.888** | **critical, named the forbidden option** |
| `d-04` | 0.875 | correct |
| `d-08` | 0.859 | wrong |
| `d-10` | 0.739 | wrong |
| `d-09` | 0.719 | correct |
| `d-03` | 0.690 | wrong |
| `d-06` | 0.495 | wrong |

**The critical case is indistinguishable from a correct one by confidence: 0.888 against
`d-04`'s 0.875, a 1.3-point gap.** No abstain-below-threshold rule separates them. The
only thresholds that remove `d-02` (>= 0.90) also abstain on three of the four correct
answers, and lower thresholds remove wrong answers while leaving the critical one in
place.

Three consequences follow, and the first is the important one:

1. **A guardrail is not a calibration problem.** Refusing a forbidden option is
   deterministic policy the caller owns, and the contract already forces the refusal to
   be visible. Trying to buy it with a probability threshold would have cost three correct
   answers and bought nothing. This is the concrete case for `authority: advisory` and for
   not fitting a threshold without a reason to.
2. **The checkpoint, not the calibration, is what changed the outcome here.** Arm B passed
   the same case without any threshold at all.
3. **A calibration pass is still worth having, but as a measurement instrument** - a
   frozen reference so two arms can be compared at one operating point - not as a fix.

### Bounded next steps, in order

1. **Stop here unless a caller appears.** The transport is proven, acceptance 1 is met by
   arm B, and nothing needs a threshold. Both arms stay reporting-only until D3 admits a
   caller.
2. **D2 before any quality claim.** The 20-case holdout was burned by the earlier 4B
   trial, so a fresh adjudicated holdout, authored by a session that has not read these
   arms, is the prerequisite for saying anything about generalisation.
3. **A dev-only calibration pass as an instrument** if a comparison at a fixed operating
   point becomes necessary, with the §6.3 rule intact: no gate until it is fitted and
   frozen, and no claim that it fixes a confident error.
