# Fork maintenance

For review-feedback intake, CI cost reductions, agent billing limits, and the
current security remediation packet, see [the feedback-loop receipt](upstream-feedback/2026-10-15-fork-feedback-loop.md).

## What the ahead/behind badge means

A customized fork is expected to be ahead of its parent. It becomes behind as
upstream lands new commits. Both at once is normal Git divergence, not evidence
that commits were lost. The operational target is **zero missing commits from the
chosen stable upstream release**, with reviewed fork safeguards retained. It is
not necessarily zero missing commits from upstream's moving development branch.
Never reset the fork to upstream or force-push to make the badge disappear.
Use ancestry-preserving merges for upstream sync PRs, not squash or rebase merges:
those discard upstream ancestry and make already-imported commits appear missing.

## Observed incident, 2026-10-15 UTC

The GitHub comparison reported `ianalitis/jcode:master` 56 ahead and 77 behind
`1jehuang/jcode:master`. The latest published stable release was v0.98.1, with
72 release commits missing. Scheduled sync run 38660726886 failed at the merge,
before build validation or proposal publication. It reported conflicts in:

- app-core Cargo.toml and server/client_lifecycle.rs
- companion server.rs and voice global/hotkey.rs
- TUI ui.rs and commands_accounts_02/part_01.rs
- code_size_budget.json, panic_budget.json, swallowed_error_budget.json and test_size_budget.json

The workflow is enabled (`UPSTREAM_SYNC_ENABLED=true`). It is not merely absent
or disabled. The failed conflict gate is protecting custom changes as designed.
The local checkout was named `pr/fu149-content-filter-hint`, tracking that old PR
branch, while HEAD exactly matched the cached fork/master ref. Its reported 164
commits ahead described the old tracking branch, not distance from upstream.
Local master also tracked upstream rather than fork master. These are different
comparisons. Always name both refs when reporting distances, and fetch before
using cached remote-tracking counts. This change does not rename branches or
alter local Git configuration.

## Automated loop

1. `upstream-release-sync.yml` checks every two hours for a published stable
   release. No new release ancestry means no build. A conflict fails closed and
   now puts pinned SHAs, release and conflicted paths in the Actions run summary.
2. A conflict-free candidate is checked in a read-only job. Publication runs on a
   separate clean hosted runner, reconstructs the exact tested tree, and creates
   an idempotent review-only proposal. It never merges or overwrites proposals.
3. `fork-health.yml` reports both development-branch and stable-release distance
   every six hours, with read-only permissions and a five-minute limit. It is a
   dashboard, not an automatic conflict resolver or a release deployment.
4. Review the proposal, dispatch normal CI on its exact branch, and merge only
   after checks and safeguard review. GitHub suppresses normal push/PR workflow
   triggers for GITHUB_TOKEN-created proposals. Do not assume green proposal
   validation is the full CI suite. No PAT or broad write credential is added to
   work around this boundary.

Schedules start only after workflows reach the default branch and Actions is
enabled. GitHub may delay schedules or disable them on inactive public repos.
Hosted standard runners are free for public repositories, not all runners or
private repositories. Bound runtimes even when minutes are free.

## Other forks

The same read-only command accepts an explicit repository allowlist, discovers
parents and default branches, and uses only GET requests through `gh api`:

```sh
python3 scripts/fork_health.py ianalitis/jcode
# Add other approved owner/repository arguments to audit multiple forks.
python3 scripts/test_fork_health.py
```

Each JSON line distinguishes default-branch lag from stable-release lag. No
published release is `null`, never a claim that the fork is current. API failures
produce an error record and nonzero exit status, not a false healthy result.
Reports are observations across API calls, not an atomic snapshot of moving refs.
A scheduled workflow's repository token cannot inspect arbitrary private forks.

For rollout to a second real fork, first audit its release/build system and
safeguards. Port the read-only monitor with an explicit repository guard, then
adapt the isolated proposal workflow and project-specific acceptance commands.
Do not blindly reuse Rust validation or upstream identity. Publishing workflows,
enabling repository settings, new credentials, tags and releases each need
operator approval. No other repositories have been modified by this change.

## Public fork audit, 2026-10-15 UTC

The monitor now uses this explicit public-only allowlist. No other repository's
workflows or settings were modified. All eight API reports completed successfully.

| Fork | Ahead / behind default | Latest parent release | Missing release commits |
| --- | --- | --- | --- |
| agentgrep | 2 / 0 | v0.1.7 | 0 |
| firefox-agent-bridge | 0 / 5 | v0.10.1 | 4 |
| GLOOP | 0 / 0 | none | unknown |
| handterm | 4 / 15 | v0.1.0 | 0 |
| herdr | 0 / 30 | v0.9.5 | 20 |
| jcode | 56 / 77 | v0.98.1 | 72 |
| jev-pr-labeler | 0 / 0 | none | unknown |
| mermaid-rs-renderer | 3 / 0 | v0.3.1 | 0 |

Prioritize jcode conflict resolution, then project-specific validation of herdr
and firefox-agent-bridge updates. Those last two had no fork-only commits at the
observed default refs, making fast-forward proposals candidates after validation,
not authorization to push. handterm demonstrates why a diverged badge alone is
not a release incident: its latest stable parent release is already contained.

## Next integration gate

Resolve the ten v0.98.1 conflicts deliberately, including behavior and privacy
regressions, not by wholesale `ours` or `theirs`. Run the normal CI matrix and
review the exact tested merge before publishing it. Then the existing scheduled
proposal loop can resume. Fully automatic conflict resolution or release
promotion is intentionally not authorized by this report-only improvement.
