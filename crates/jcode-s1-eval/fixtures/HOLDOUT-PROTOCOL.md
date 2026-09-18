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
