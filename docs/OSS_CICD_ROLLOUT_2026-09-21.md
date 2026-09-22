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

**The vendor document's coverage claim holds, and Rust is included.** CodeQL does
analyze Rust here: `/language:rust` analyses were recorded on `jcode`,
`handterm`, `mermaid-rs-renderer`, and `agentgrep`. Final `jcode` state on
`43a2e7539` — every one of these is `success`:

| Language | `Analyze` job | Drawn from |
| --- | --- | --- |
| actions | success | workflow files |
| javascript-typescript | success | `sdk/` |
| python | success | `scripts/`, `telemetry-worker/` |
| rust | success | the crates |
| swift | success | `ios/` |

**How this was nearly reported wrong, which is the useful part.** An earlier draft
of this receipt claimed CodeQL's Rust coverage was "unproven" and that the
repository's Rust security signal therefore remained cargo-audit. That was a
mid-run snapshot read as a capability limit: at the fifteen-minute mark every
language except Rust had finished, and the Rust job was still `in_progress` with
zero recorded analyses. It completed at roughly twenty minutes. Meanwhile
`handterm`, a much smaller Rust repo, had already recorded `/language:rust` while
`jcode` was still running — a faster way to settle the same question than waiting
on a large crate.

**What CodeQL found on first run, which is the actual payload.** 100+ open Rust
alerts on `jcode`: 6 `critical` (`rust/hard-coded-cryptographic-value`) and 94
`high` (90 `rust/cleartext-logging`, 4 `rust/cleartext-transmission`). Honest
triage status: **not triaged**, and the raw counts should not be repeated as
vulnerability counts. A first pass suggests the six criticals are heuristic false
positives on non-cryptographic seeding — two in
`crates/jcode-tui/src/tui/ui_animations.rs`, a TUI animation module, four in
`crates/jcode-tui-mermaid/tests/layout_cache_resize_probe.rs` — but the
`rust/cleartext-logging` cluster is not uniformly test code: `src/cli/login.rs`
carries 24 and `src/cli/commands.rs` 13, in an authentication-heavy CLI. That is
a backlog to read, not a set of confirmed vulnerabilities.

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

The fork carries **192** `v*` tags (an earlier revision of this section said 30,
which was wrong: `git ls-remote --tags fork | grep -v '\^{}' | grep -c
'refs/tags/v'` returns 192, of 195 tags total). `Release` had **0 runs ever** and
the fork has **0 releases**, so the trigger was latent, not exercised. But pushing
a new `v*` tag to the fork today would start a release workflow that creates a
fork release with fork-built assets. The iOS `build-and-upload` job was already
gated by the earlier fork CI repair; the release workflow as a whole was not.

This is not covered by any item in the strategy's gap register and should become
one. The two candidate remedies are gating the workflow on
`github.repository == '1jehuang/jcode'`, or disabling the `Release` workflow on the
fork. The second is smaller and does not diverge from upstream.

### Applied 2026-09-22: item 10, the fork `Release` workflow is now disabled

```sh
gh api repos/ianalitis/jcode/actions/workflows/350405739 --jq '{name,path,state}'
# before: {"name":"Release","path":".github/workflows/release.yml","state":"active"}
gh workflow disable Release --repo ianalitis/jcode
gh api repos/ianalitis/jcode/actions/workflows/350405739 --jq '{name,path,state}'
# after:  {"name":"Release","path":".github/workflows/release.yml","state":"disabled_manually"}
```

Disabling rather than editing means zero divergence from upstream: the file on the
fork default branch is still upstream's. Rollback is `gh workflow enable Release
--repo ianalitis/jcode`. The better long-term remedy, gating the workflow on
`github.repository`, remains a valid upstream contribution and is now the only
fork-release exposure left to close; see §7.

**Residual after item 10, recorded rather than assumed.** `discord-release.yml` is
also unguarded (trigger `release: types: [published]`, `contents: write`), so it is
the second half of a `tag → release → announce` chain. It was left active because
the fork has **zero Actions secrets** (`gh api
repos/ianalitis/jcode/actions/secrets --jq '.secrets[].name'` returns nothing), so
`DISCORD_RELEASE_WEBHOOK` resolves empty and the job fails rather than announcing
to upstream's Discord; and it is not push-triggered, so the only way to reach it
now that `Release` is disabled is to publish a fork release by hand. An upstream
`github.repository` guard would close both halves at once.

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

**A ref-relative count was misread, and the conclusion drawn from it was wrong.**
The original text here claimed `origin/master...HEAD` read `0 281` before a fetch
and `0 7` after, concluding that the integration line was "in sync with upstream
rather than far ahead of it". The `0 7` belonged to `origin/master...fork/master`,
and the pre-fetch `0 281` was the correct figure. Measured at `c53f2bedc` after a
confirmed `git fetch --no-tags origin`:

| Ref pair | behind | ahead |
| --- | --- | --- |
| `origin/master...HEAD` | 0 | **285** |
| `origin/master...fork/master` | 0 | 7 |
| `origin/master...master` | 0 | 0 |

`origin/master` is `2a4edaa02057ac994a601311c4f03ed450e1b3c9` and is an ancestor
of `HEAD`. The 285 commits are 3 merges and 282 non-merge commits. **The
integration line is 285 commits ahead of upstream, not in sync with it**, and the
`0 281` that was originally dismissed as a stale-ref artifact was the right number.
Two independent sessions then measured 283 at `b3689db26` and 285 at `c53f2bedc`,
which agree.

The durable lesson is narrower than "refs go stale": a plausible-looking number
that contradicted the one taken moments earlier was accepted without being
re-measured, and it inverted the conclusion. Report ref-relative counts only after
a fetch whose success was confirmed, and treat a count that contradicts an earlier
one as suspect rather than as an improvement.

## 7. Verified state, and what is still open

Applied: strategy items 1, 2, 3, and **item 10** (fork `Release` disabled
2026-09-22, §5). Item 12 (the CodeQL alert triage) is **read and concluded**:
166 open alerts, zero confirmed leaks, two actionable items, in
[`docs/upstream-feedback/2026-09-22-codeql-rust-alert-triage.md`](upstream-feedback/2026-09-22-codeql-rust-alert-triage.md).
Nothing else has been applied.

Still requiring approval, unchanged from the strategy's §7:

- Item 4, now known to mean *enable* workflows rather than *dispatch* them, with
  `mermaid-rs-renderer`'s `release.yml` resolved first.
- Item 5, upstream `dependabot.yml`.
- Item 6, SHA-pin actions then tighten `allowed_actions`.
- Item 7, build provenance for release binaries.
- Item 8, default-branch ruleset; with item 10 applied, that general ruleset is now
  the only remaining protection for the mirror line, and it needs a design that
  permits the documented sync.
- Item 11, enabling workflows on `handterm` and `mermaid-rs-renderer`.
- The Greptile open-source application, which stays operator-only.

Not verified, and stated as such: whether `handterm` ever records a second analysis
(it recorded only `/language:rust`, unlike the others), whether the labeler gate
behaves correctly when a key *is* present (no fork has one), and why the alert
counts recorded on 2026-09-21 were lower than the 166 measured on 2026-09-22.
Dismissing the fork's triaged alerts is a repository mutation and was deliberately
not done.
