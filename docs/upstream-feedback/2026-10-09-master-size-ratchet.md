# Upstream master size-ratchet reproduction, 2026-10-09

Upstream head: `4a7400819`. Local fork HEAD: `df9792506` on
`jcode/ci-format-baseline`.

This receipt continues `docs/upstream-feedback/2026-10-07-master-format-gates.md`
and the standing upstream issues for master gate failures. It records what sits
**behind** the formatting failure we already reported, so the next gate is known
before it is hit.

## 1. What the hosted jobs actually failed on

The Quality Guardrails job runs the gates in sequence, so a formatting failure
hides everything after it.

| Run | Head | Job | Result |
| --- | --- | --- | --- |
| master `37727625832` | `4a7400819` | QG `113149293022` | failed at **step 6 "Check formatting"**; steps 7-18 skipped |
| PR #1764 `37727638919` | `13375d186` | QG `113149334110` | step 6 passed; failed at **step 11 "Enforce oversized-file ratchet"** |

So on master the only visible guardrail failure is still the rustfmt drift
reported in #1740, and #1764 (the maintainer's fix for it) clears that gate and
immediately exposes the next one.

Step names on master's failed job:

```
6   Check formatting                    failure
7   Check all targets and all features  skipped
8   Run clippy with warnings denied     skipped
...
11  Enforce oversized-file ratchet      skipped
```

## 2. The ratchet failure is inherited from master, not caused by #1764

`scripts/check_code_size_budget.py` compares the checked-out tree against the
committed baseline `scripts/code_size_budget.json` (119 tracked files, threshold
1200 LOC) and fails when a tracked file grew or a new file crosses the
threshold. #1764 does not touch the baseline, so its tree carries master's
baseline unchanged.

Hosted output on #1764's head (job `113149334110`, log lines 2193-2236) is
**32 regressions**: 29 tracked files grew and 3 files newly crossed 1200 LOC.

Independent recomputation at `origin/master` (`4a7400819`), reading the tree and
the baseline straight out of git objects with the same production-file filter
and the same comparison rules, reproduces the hosted list entry for entry:

```
treeish           origin/master (4a7400819)
baseline entries  119  threshold 1200
current oversized 122
regressions       32  (grew 29, new 3)
improvements      10  (shrank 10, gone 0)
VERDICT           FAIL
```

The single difference is `crates/jcode-provider-openai-runtime/src/lib.rs`:
hosted `1494 -> 1525`, master `1494 -> 1526`. #1764's own rustfmt fix collapses
one line in that file, so its tree is one line shorter than master's. Everything
else matches exactly, including the three new oversized files
(`crates/jcode-app-core/src/tool/mcp.rs` 1338 LOC,
`crates/jcode-provider-copilot-runtime/src/lib.rs` 1201 LOC,
`crates/jcode-tui/src/tui/app/hotkey_feedback.rs` 1216 LOC).

Largest single growth at master: `crates/jcode-base/src/auth/lifecycle.rs`
2768 -> 3065 (+297).

## 3. Consequence

Master's Quality Guardrails will fail at step 11 as soon as the formatting step
passes, so **no PR can turn the guardrails green while the committed baseline
lags the tree**. #1764 fixes the reported formatting drift correctly and still
cannot go green on its own. This is the same class of inherited debt recorded
for #547 (resolved by `--update` in #550, and again in #587) and reported for
current master in #692 by another contributor on 2026-10-06, whose count of 17
regressions has since grown to 32.

Two remedies, both maintainer calls, neither taken here:

1. Refresh the baseline (`scripts/check_code_size_budget.py --update`), the
   remedy upstream has used before (#550, #587). Cheapest, keeps the gate
   meaningful only from the refresh point forward.
2. Structural: compare against the merge base instead of a snapshot baseline, or
   re-baseline in the same change that grows a file. This stops an unrebased PR
   from failing on debt it did not create.

## 4. Boundary

No issue or PR was mutated while producing this receipt. Posting this as a
comment on #692 (the standing code-size ratchet issue) is a mutation beyond
answering a review and needs operator approval, as does any baseline PR.

Read the jobs independently:

```sh
gh api repos/1jehuang/jcode/actions/jobs/113149293022 --jq '.steps[]|"\(.number)\t\(.name)\t\(.conclusion)"'
gh api repos/1jehuang/jcode/actions/jobs/113149334110/logs --allow-escape-sequences
```

Local recomputation helper used above:
`/Users/ianalitis/.jcode/scratch/ratchet_check.py` (reads any treeish; no
checkout, no writes).
