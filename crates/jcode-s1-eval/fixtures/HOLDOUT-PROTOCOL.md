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
