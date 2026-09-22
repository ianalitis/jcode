# OSS CI/CD rollout receipt: 2026-09-21

**Date:** 2026-09-21. **Status:** applied and verified, except where marked
unverified. Companion to
[`plans/2026-09-21_OSS_CICD_STRATEGY.md`](plans/2026-09-21_OSS_CICD_STRATEGY.md),
which owns the reasoning; this document owns what actually changed and what was
observed.

Operator approval named: the labeler fix publish, CodeQL default setup on the five
public forks, and Dependabot alerts on the five public forks. Nothing else in the
strategy's §7 was executed.

## 1. Labeler check: fixed, published, verified green

Published to the fork default branch as `43a2e7539`, pushed
`f65d11cf4..43a2e7539` with `--no-force`. The push was a verified fast-forward:
`git merge-base --is-ancestor fork/master HEAD` was checked against a real
`git fetch fork` immediately before the push, and the remote accepted the
non-forced update. Three paths moved: `.github/workflows/label-pr.yml`,
`docs/FORK_CI.md`, `docs/upstream-feedback/2026-09-21-greptile-labeler-missing-key.md`.
`docs/README.md` and the strategy document were deliberately left off the fork's
default branch to keep divergence to the minimum that makes the check green.

**Before:** every Greptile review produced a failing `Semantic PR labels` check.
Observed runs `35666684356`, `35666684019`, `35667296411`, `35667296342`, all
`failure`, all citing `"reason": "OPENROUTER_API_KEY is required."`.

**After**, verified by dispatching the workflow's own manual entry point
(`gh workflow run "Semantic PR labels" --ref master -f pull-request=3`), which
exercises the new code path end to end with a safe outcome because no key exists:

| Evidence | Value |
| --- | --- |
| Run | `35668523374`, conclusion **success** (8s) |
| Job `Check labeling is configured` | `success` |
| Job `label` | `skipped` |
| Annotation | `::notice::OPENROUTER_API_KEY is not configured for this repository; skipping semantic PR labels.` |
| Step output | `Set output 'enabled'` |
| On the push event itself | check run `label: completed/skipped` |

The gate decided; the job did not merely error into a green state. This is
runtime verification on GitHub, not a local simulation.

## 2. CodeQL default setup: enabled on all five

`PATCH /repos/{r}/code-scanning/default-setup` with
`{"state":"configured","query_suite":"default"}`. Four repositories returned an
initial-run id; `GLOOP` returned `run_id: 0`, consistent with its empty detected
language list. `GET` now returns `state=configured` for all five.

| Repo | Initial run id | Analyses recorded |
| --- | --- | --- |
| `ianalitis/jcode` | 35668431059 | `actions`, `javascript-typescript`, `python`, `swift` |
| `ianalitis/handterm` | 35668433959 | none yet (`no analysis found`) |
| `ianalitis/mermaid-rs-renderer` | 35668436559 | `python` |
| `ianalitis/agentgrep` | 35668439121 | `python` |
| `ianalitis/GLOOP` | 0 (no languages detected) | none |

**No new failing check was introduced.** `Analyze (python)`,
`Analyze (actions)`, `Analyze (javascript-typescript)` and `Analyze (swift)` all
completed `success` on `43a2e7539`. This mattered enough to wait on: adding a red
check while removing a red check would have been a straight regression.

**The vendor document's implied coverage is not what this repository gets.**
CodeQL created an `Analyze (rust)` job, so Rust is nominally in scope, and the
`rust` entry in the advertised language list is why it did. But ~15 minutes in,
with `actions`, `javascript-typescript`, `python` and `swift` all `success`,
`Analyze (rust)` was still `in_progress` and zero `/language:rust` analyses had
been recorded. `mermaid-rs-renderer` and `agentgrep` — both Rust projects —
produced **Python-only** analyses. Rust coverage is therefore unproven, and the
repository's usable Rust security signal remains
`scripts/security_preflight.sh --strict` (cargo-audit) plus the clippy gate.

## 3. Dependabot alerts: enabled on all five

`PUT /repos/{r}/vulnerability-alerts` returned `204` on all five, and
`GET` verified `204` (enabled) on all five afterwards.

`PUT /repos/{r}/automated-security-fixes` was **deliberately not** called: it opens
version-bump PRs against the fork's default branch, which the strategy's §6
excludes because that branch is a mirror. Alerts notify without that churn.

## 4. New finding: three forks have no registered workflows at all

The strategy's item 4 assumed the inert forks had CI that had simply never run.
That is wrong, and it changes what item 4 costs.

| Repo | `ci.yml` in default-branch tree | Registered workflows | Runs ever |
| --- | --- | --- | --- |
| `ianalitis/jcode` | yes | 12 (incl. `CI`, `Release`, `CodeQL`) | yes |
| `ianalitis/handterm` | yes | **`CodeQL` only** | CodeQL only |
| `ianalitis/mermaid-rs-renderer` | yes (plus `release.yml`) | **`CodeQL` only** | CodeQL only |
| `ianalitis/agentgrep` | no | `CodeQL` only | CodeQL only |
| `ianalitis/GLOOP` | no | none | none |

`gh api repos/{r}/actions/workflows` lists only CodeQL for the first four. The
workflow files are present in the trees but were never registered, so there is
nothing to dispatch and no run to inspect: `gh workflow list --repo
ianalitis/handterm` shows one entry. `actions/permissions.enabled = true` does not
mean fork workflows are registered, and I had used that flag as evidence for the
wrong conclusion in the strategy document.

Consequence: item 4 cannot be completed by `workflow_dispatch`. It requires
enabling workflows on each fork, and `mermaid-rs-renderer` needs its `release.yml`
considered first, because enabling that fork's workflows would make a
fork-scoped release workflow live.

## 5. New finding: an active, unguarded release workflow on the fork

`Release` is an **active** workflow on `ianalitis/jcode`:

- trigger: `push` on tags matching `v*`
- permissions: `contents: write`
- no `github.repository` guard; the only use of that variable is `GH_REPO` at line
  436, which is not a condition

The fork carries 30 `v*` tags. `Release` has **0 runs ever** and the fork has **0
releases**, so the trigger is latent, not exercised. But pushing a new `v*` tag to
the fork today would start a release workflow that creates a fork release with
fork-built assets. The iOS `build-and-upload` job was already gated by the earlier
fork CI repair; the release workflow as a whole was not.

This is not covered by any item in the strategy's gap register and should become
one. The two candidate remedies are gating the workflow on
`github.repository == '1jehuang/jcode'`, or disabling the `Release` workflow on the
fork. The second is smaller and does not diverge from upstream.

## 6. Tooling and ref-state findings

Two process defects were found while doing this work, both worth recording because
each one silently produced a wrong intermediate conclusion.

**`scripts/bounded.sh` is present but can vanish mid-command during a
concurrent branch switch.** Corrected 2026-09-21 by a second source session: the
file is tracked, present at `HEAD` (`git cat-file -e HEAD:scripts/bounded.sh`
succeeds), introduced by `872f7d148`, and 1471 bytes. The original claim that it
does not exist is wrong. What is real, and still worth recording, is narrower: a
second session was committing to this same checkout, and the file was
transiently absent from the working directory while that checkout moved. A `git
fetch` wrapped in it was therefore never executed, and because the shell error
went through a pipe the skipped fetch looked successful. That observation was
reproduced twice more by the second session. Treat this as a signal to re-run and
verify a command's effect rather than as a reason to stop using `bounded.sh`,
which remains the correct bound for anything that can wedge on this host.

**The local `origin/master` ref was stale at the time.** Measured before a real
fetch, `origin/master...HEAD` read `0 281`; after fetching it read `0 7`, so the
integration line is in sync with upstream rather than far ahead of it. Re-measured
later in the day by a second session after a confirmed `git fetch origin`, the
count is `0 283` with `origin/master` at `2a4edaa02057ac994a601311c4f03ed450e1b3c9`,
and upstream has not moved since. The 274-commit figure is therefore a
snapshot of one stale ref, not a standing fact; the durable lesson is the last
sentence. Report ref-relative counts only after a fetch that has been confirmed to
succeed.

## 7. Verified state, and what is still open

Applied: strategy items 1, 2, 3. Nothing else.

Still requiring approval, unchanged from the strategy's §7:

- Item 4, now known to mean *enable* workflows rather than *dispatch* them, with
  `mermaid-rs-renderer`'s `release.yml` resolved first.
- Item 5, upstream `dependabot.yml`.
- Item 6, SHA-pin actions then tighten `allowed_actions`.
- Item 7, build provenance for release binaries.
- Item 8, default-branch ruleset; the unguarded fork `Release` (§5) is a better
  first target than a general ruleset.
- The Greptile open-source application, which stays operator-only.

Not verified, and stated as such: whether CodeQL ever completes a Rust analysis on
`jcode`; whether `handterm` produces any analysis at all; and whether the gate
behaves correctly when a key *is* present, since no fork has one.
