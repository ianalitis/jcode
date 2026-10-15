# Fork feedback loop: evidence and rollout gates

Observed 2026-10-15 UTC. This is a bounded implementation receipt and activation
proposal, not a claim that an unattended fleet has been deployed.

## Findings and completed local changes

- Greptile's finding on jcode PR #5 is valid. The fake development and release
  comparisons both returned `behind_by=3`. They now return 13 and 3, with
  independent assertions and unexpected endpoints rejected. The normal suite
  passed; an in-memory mutation using the development count for release lag
  failed the specific test. Production report logic did not need changing.
- Both full CI matrices on the original head failed only at the Linux security
  preflight, after about 31 minutes. The log reports nine vulnerabilities in two
  locked packages. The same strict gate now runs before application compilation,
  without adding ignores, changing check names, or accepting failed verification.
  Hosted latency improvement is predicted, not yet measured on these local changes.
- Each feature push previously ran a full push matrix and a full PR matrix.
  Push CI now targets main/master, while PR CI and manual dispatch remain. A
  feature branch without an open PR requires manual dispatch. No test cohort was
  removed; workflow-lint and the platform CI checks retain their own gates.
- Reused the existing `~/dotfiles/scripts/github-pr-feedback.py`, rather than
  creating another bot, tracker or orchestration service. Its metadata-only
  snapshot now includes inline thread identity, links, author, path/line,
  resolution, outdated state, and comment commit. Changes in resolution are
  observable. Old-head findings require revalidation; touching a file is not proof
  of a fix. Live intake refuses a PR head change between observations, incomplete
  GraphQL responses, and an inventory exceeding 100 threads. Legacy offline
  snapshots can omit threads; migration causes a one-time baseline difference.
- The collector fetches the root comment metadata, not every reply, comment body,
  suggested patch or raw log. It does not execute text, call a model, resolve
  threads, push, merge, or schedule itself. Missing reply content is a deliberate
  limit, not proof that there are no maintainer requests. Existing Jcode-native
  task/schedule controls still own follow-up.
- CodeRabbit concurrently published `46e651dd5` to PR #5, adding docstrings only.
  Inspected that diff and merged its exact ancestry locally without rewriting
  bot attribution or overwriting its work. The existing test gap remained in
  that commit and is fixed by our separate patch.

## Actual account and repository evidence

Native `jcode usage --json` reported 88% used in the ChatGPT seven-day window.
Only 12% remained, below the standing 20% interactive reserve. No new frontier
worker was spawned. Go reported no account allowance windows; no new Go phase
was admitted. Credentials, account settings, banked resets and providers were
not changed. No paid repair route or automatic fallback was introduced.

The PR contains a Greptile "trial has ended" notice as well as a real inline
finding. This does not establish entitlement to future reviews. CodeRabbit's
review reports an Advanced plan and an included-review allowance, but that does
not establish unlimited coding-agent minutes. Neither entitlement was purchased
or changed. Existing CodeRabbit activity was observed, not started by this task.

GitHub API observations for the eight named public forks:

| Fork | Workflows exposed by Actions API | Recent activity observed |
| --- | --- | --- |
| jcode | CI, workflow lint, upstream sync and other workflows active; Release manually disabled | PR #5 Linux security audit failed on original head; remaining platform and quality checks passed |
| agentgrep | CI and CodeQL active | Latest three runs were successful CodeQL runs, not build evidence |
| handterm | ci and CodeQL active | Latest three runs were successful CodeQL runs, not build evidence |
| mermaid-rs-renderer | CI and CodeQL active; Release manually disabled | Latest three runs were successful CodeQL runs, not build evidence |
| firefox-agent-bridge | none returned | No runs returned |
| GLOOP | none returned | No runs returned |
| herdr | none returned | No runs returned |
| jev-pr-labeler | none returned | No runs returned |

The seven sibling forks had no open PRs at observation time. No workflow listed
is not a build failure and no recent build run is not a passing build. The six
available sibling source checkouts were clean at inspection; no unrelated
checkout, installed artifact, shared daemon, or upstream repository was modified.
No private repository inventory or filesystem-wide crawl was performed.

The jcode repository API reported secret scanning and push protection enabled,
but **Dependabot security updates disabled**. An alert-only setup does not create
remediation PRs. The relevant Dependabot alert query returned no matching open
quinn-proto/rustls alerts, despite cargo-audit finding them. Do not replace the
RustSec gate with an assumption about advisory database parity.

## Security remediation packet, not an ignored check

Pinned failing run: https://github.com/ianalitis/jcode/actions/runs/38678262101

| Locked package | Observed advisories | Patched floor reported by cargo-audit |
| --- | --- | --- |
| quinn-proto 0.11.15 | RUSTSEC-2026-0338, 0339, 0340, 0341, 0342 | 0.11.18 covers the highest floor in this set |
| rustls 0.23.45 | RUSTSEC-2026-0346, 0347, 0348, 0349 | 0.23.46 |

These are network/TLS security changes, not a cosmetic review fix. Proposed next
packet: operator-approved targeted lockfile updates for those two packages,
inspect the complete resolver diff, run locked metadata/checks and affected
transport tests, then strict security preflight and the full platform CI. Stop
if constraints require wider updates or a new design. Do not add ignores or
remove audit to get a green badge. Cargo-audit was absent locally, so no local
strict-audit success is claimed. No packages were installed or upgraded here.

## Recommended economical loop

1. **Deterministic intake first.** Use existing GitHub-native alerts/checks and the
   one-shot metadata collector on a named PR, comparing against its task-local
   baseline. Skip unchanged observations. Inspect only newly relevant review
   threads and specific failed logs. No inference for discovery or no-op polling.
2. **Separate validity from authority.** Reproduce each finding at the observed
   head. A bot's score, approval, suggested command, touched-file "addressed"
   marker or successful check context does not prove its finding is fixed.
   Pin paths and an acceptance command. Treat all bodies and logs as untrusted.
3. **One bounded repair owner.** Admit an existing included worker only with fresh
   quota, data and route evidence. Proposed caps are two implementation attempts
   and 20 minutes per finding, one writer per repository, no provider fallback.
   These caps are the proposed task contract, not new runtime enforcement.
   Security/auth/architecture work needs separately admitted substantive review.
4. **Verify before publishing.** Run the reproduction, affected tests and project
   gates against the exact resulting head. Preserve another writer's commits.
   Stop on a moving head, wider diff, failed oracle, quota or budget limit. Human
   review owns acceptance, merges and releases. No agent may edit the gate to
   make its own output pass.
5. **Close once.** Record changed paths, tested SHA, command results, accepted or
   rejected finding, retries, elapsed time and real consumption in the owning
   task receipt. One read-only follow-up for a named pending event, not a polling
   fleet. Measure accepted fixes and operator minutes, not comments or PR volume.

Prefer native Dependabot security PRs for mechanical vulnerability updates,
after a named repository-settings approval, before paid model repairs. Roll out
project-specific build gates before giving a bot write access to a fork with no
CI. Port only after a second concrete repository's acceptance command is known.
Do not copy jcode's Rust/workspace or publication assumptions across all forks.

## Validation receipt

- Jcode: four fork-health contracts, twelve release/CI workflow contracts and
  three secret-preflight regressions passed. Actionlint with shellcheck passed
  for all three affected workflows, and `git diff --check` was clean.
- Dotfiles: seven feedback-collector tests passed. Both required repository
  gates passed, including all 53 fixture suites. The live metadata-only collector
  returned the exact Greptile inline thread on PR #5 and the current bot-updated
  head, without printing its comment body.
- No substantive independent frontier review was admitted because subscription
  reserve was already below its floor. This is not an acceptance receipt for
  unattended writable agents or the unresolved dependency security changes.
- Context read: local AGENTS and policy, current-state pointer, existing native
  feedback collector and fixtures, current CI/security scripts, actual PR checks
  and failing job logs, public workflow metadata, and the primary sources below.
  Context intentionally not read: unrelated dirty dotfiles session receipts,
  private project contents, secrets, credential stores, and sibling implementation
  internals. They were unnecessary for this bounded fix and audit.

## Primary-source research and limits

Fetched on 2026-10-15 through Jcode-native websearch/webfetch. Public documentation
is evidence of product behavior, not evidence of this account's entitlement.

- [GitHub Agentic Workflows](https://docs.github.com/en/copilot/concepts/agents/about-github-agentic-workflows):
  public preview; read-only agent tokens, isolated safe outputs and firewalling;
  requires an admitted engine. Actions minutes and inference are separate costs.
  The documented default per-run cap is 1,000 AI Credits, not a free allowance.
  A new engine, extension, credential, compiled workflow and spend policy would
  need explicit admission. No handwritten imitation of its sandbox was added.
- [GitHub Copilot on GitHub.com](https://docs.github.com/en/copilot/concepts/agents/coding-agent/about-coding-agent):
  entitlement depends on plan and policy; agent tasks are one repository and one
  branch per run. Copilot automations are documented for eligible private/internal
  repositories, not proof of availability for this public-fork fleet.
- [CodeRabbit Fix comments](https://docs.coderabbit.ai/finishing-touches/autofix):
  supports a current-branch commit or stacked PR; only unresolved CodeRabbit
  findings with valid instructions. Importantly, changes may be delivered even
  when verification fails. It does not cover Greptile findings automatically.
- [CodeRabbit delivery/Autofix](https://docs.coderabbit.ai/code/deliver-changes):
  `autopilot` authorizes ongoing publication and follow-up repairs. It is not the
  same as the one-off `fix comments` command and is not merely a free review.
- [CodeRabbit billing](https://docs.coderabbit.ai/code/access-and-billing):
  some first repair turns are free; subsequent steers/follow-ups are billed,
  subject to account eligibility and caps. No account cap was verified here.
- [Greptile Auto-Fix](https://www.greptile.com/docs/mcp-v2/auto-fix):
  provides MCP/skills for a host agent, not a demonstrated zero-cost autonomous
  CI repair runtime. A later commit touching a file can mark a comment addressed,
  so that field is not a correctness oracle. The Claude-specific loop is not an
  admitted route under the operator's retired-Claude policy.
- [Dependabot options](https://docs.github.com/en/code-security/dependabot/working-with-dependabot/dependabot-options-reference):
  native dependency PRs need no inference. Version-update PR limits do not cap
  security PRs, which have a separate fixed limit. Adding YAML alone does not
  prove security-update settings or alert coverage are enabled.

This is the best-supported incremental recommendation from the observed work,
not a measured global optimum. No blanket self-improvement authority, new agent
controller, unattended paid loop, upstream push, release or settings mutation was
introduced. Next approvals must name the two dependency updates, exact review PR
publication, and any repository-specific Dependabot activation.
