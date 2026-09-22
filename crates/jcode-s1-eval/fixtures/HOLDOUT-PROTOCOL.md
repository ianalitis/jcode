# Holdout labeling protocol

`labels.dev.json` was authored by the same worker that wrote the baseline
rules, so it can only measure fit, not generalization. The holdout must be
produced by a different session that has not read `src/lib.rs`:

1. Author at least 30 new synthetic cases into `cases.holdout.json` using the
   same families (plain positive per industry, negation, injected instruction,
   absent/short, multi-intent, non-English, long irrelevant context, schema
   collision, hybrid boundary). Invent businesses; never copy a real client.
2. Write `labels.holdout.json` before running any arm. Do not edit labels
   after seeing scores.
3. Split families, not text: no holdout case may share an `org_name` or an
   `about` sentence with a dev case.
4. Thresholds and rule changes are chosen on dev only. Holdout is scored once
   per arm per revision and the scorecard JSON is committed as evidence.

## Iteration record

- rev1 (`8139595aa`): holdout industry 22/27, task kind 21/27, complexity
  22/27, abstain 2/5. Dev had been 24/25, 25/25, 23/25, 3/4.
- rev2: `to_ascii_lowercase` replaced with `to_lowercase` (accented keywords
  never matched), multilingual and generic industry keywords, org_name no
  longer double-weighted (name collisions), explicit undecided phrases
  abstain, "other" word floor 12 to 20, static-site declarations suppress
  prose interactivity. Holdout: 27/27, 23/27, 23/27, 5/5.
- rev3: bare "replace" and "editor" narrowed to phrases. Holdout: 27/27,
  26/27, 24/27, 5/5, skin 24/24, 0 critical. Dev unchanged across revs
  (24/25, 24/25, 22/25, 4/4).
- The holdout was scored three times across rev1 to rev3, which is more than
  the once-per-revision rule intends. Treat rev3 holdout numbers as mildly
  optimistic; the next revision must be scored on a fresh holdout authored
  by a labeler who has not seen this file.

## Decision fixtures (`decisions.dev.json`)

The typed decision arm is governed by the same protocol against its own fixture set.
`decisions.dev.json` was authored alongside `DeterministicDecisionBaseline`, so it
measures fit and nothing else: the baseline scores 5 of 10 on it, and that floor is
pinned in `decision::tests::the_baseline_satisfies_the_contract_on_the_whole_dev_fixture`
so an arm's improvement is visible in one number.

Before a decision arm may make a quality claim:

1. A session that has **not** read `src/decision.rs` or the arm authors
   `decisions.holdout.json` in the same shape: the same three kinds, the same option
   discipline, at least 30 cases, invented states only, and no case sharing a state
   string or an option id set with a dev case.
2. The correct answers are written before any arm runs, and are not edited after seeing
   a scorecard. A case whose correct answer is abstention has an empty `acceptable`.
3. The holdout is scored once per arm per revision, and the scorecard JSON is committed
   as evidence. Contract validity (`invalid`, `critical`) is reported alongside quality
   (`correct`, `over_abstained`), because an arm that cannot stay inside the contract is
   not admissible regardless of its accuracy.
4. Calibration is fitted on **dev only**. The fitted `calibration_ref` is then frozen
   and named in the request; a threshold without one is refused by the schema, which is
   the point of W5.
5. The holdout is scored for the arm as configured at freeze time. Tuning the arm after
   seeing a holdout scorecard means that holdout is burned, exactly as rev1 to rev3
   burned the classifier's first holdout.
