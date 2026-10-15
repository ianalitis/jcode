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

## Approved overnight checkpoint, 05:12 UTC

The operator subsequently approved the named updates, publication and jcode
Dependabot activation, and authorized continued overnight included inference.
This supersedes the approval-pending statements above, not the verification
requirements or restrictions on destructive actions, secrets and paid services.

- Published head `2cdba92c2916f59a0780f4e640538c077cd64c77` to the existing
  `automation/fork-health` PR #5, preserving the independently delivered bot
  commit. No force push, merge, release or upstream push was performed.
- Precisely updated `quinn-proto` to 0.11.18 and `rustls` to 0.23.46. Only the
  required `rand_pcg` package was added. No manifests or audit ignores changed.
- Enabled Dependabot security updates for jcode and verified the API readback.
- Locked TUI compilation and all 20 `jcode-config-types` tests passed. All 19
  script contracts and actionlint passed. Local preflight passed secret and
  permission checks, but skipped dependency auditing because cargo-audit is
  absent. This is not a strict-audit success or runtime acceptance claim.
- Verified the new head creates one full PR CI run, `38685360793`, rather than
  duplicate push/PR matrices. Cancelled only superseded CI runs `38684064016`
  and `38684056821`; cancellation is not recorded as a passing check.
- Admitted a separate Astra/high read-only review of the scoped changes and
  upstream integration risks. Review outcome and hosted CI remain pending.
- Fetched the released v0.98.1 commit
  `1c7426eb1925a1167eb56011fa479ad707cc28b6` without merging it. The ten known
  integration conflicts still require deliberate resolution and verification.
- Found a policy-reference gap: `docs/HARNESS_LOOP_ARCHITECTURE.md` is absent in
  this checkout. Do not fabricate an authoritative replacement or silently
  claim compliance with its unavailable contents.

## Standards mapped to evidence, not badges

The following primary sources were read on 2026-10-15 with Jcode webfetch.
These are risk-based improvement targets, not certification or attestation.
OMG XTCE is a telemetry exchange standard, not an SDLC compliance framework.
For this work, use established software supply-chain and development standards
instead of inventing a universal proprietary score or new control plane.

| Reference | Concrete control and current evidence | Remaining gate |
| --- | --- | --- |
| [NIST SSDF](https://csrc.nist.gov/Projects/ssdf), PO | Versioned policy, scoped task packets, single-writer leases, separate review | Repair stale canonical references and verify each fleet repository's own acceptance commands |
| NIST SSDF, PS | Read-only validation, separate clean publication runner, no retained checkout credentials in affected workflows | Audit release artifact provenance and action pinning before stronger supply-chain claims |
| NIST SSDF, PW | Reproduction-guided fixes, regression tests, quality/lint/platform checks | Green checks on the exact candidate plus representative runtime validation for source changes |
| NIST SSDF, RV | Strict RustSec gate, triage record, Dependabot security PRs, metadata-only review intake | Confirm patched graph passes hosted strict audit and revalidate findings after head movement |
| [SLSA 1.2](https://slsa.dev/spec/v1.2/) | Use source/build tracks and provenance requirements as separate verification targets | No SLSA level claimed. Hosted CI alone does not establish provenance generation, distribution or consumer verification |
| [OpenSSF Scorecard](https://securityscorecards.dev/) | Prioritize dangerous-workflow, token-permission, dependency-update and review controls | Scorecard has not been run here. A composite score is advisory, not correctness or permission to weaken controls |

NIST explicitly recommends adapting practices to risk, cost and feasibility,
not treating SSDF as a universal checklist. Existing SDK publication already
uses npm provenance, so inspect and verify that path rather than adding a second
publisher. A provenance flag alone does not prove that every fork binary has
verified provenance. Keep release workflows disabled until separately validated.

Measure accepted outcomes with denominators: accepted fixes / attempted fixes,
reopened regressions / accepted fixes, detection-to-validated-fix time, duplicate
CI runs / candidate heads, operator interventions / accepted task, actual model
usage and actual runner time per accepted task. Unknown cost stays unknown.
Track unresolved critical findings separately: a better average must never hide
one unresolved security blocker. No numeric performance improvements or enterprise
compliance score are claimed from this single incident.

The overnight loop uses native schedules, existing feedback intake, scoped
workers and this receipt. It does not install a new daemon, give PR text command
authority, enable paid bot autopilot or auto-merge unchecked changes. Each repair
packet needs an exact head, allowed files, finite attempts, acceptance commands
and an independent gate. Failure closes that packet before any escalation.

## 05:25 UTC checkpoint

- PR #5 still points to `2cdba92c2916f59a0780f4e640538c077cd64c77`.
  Its hosted `Security preflight (Linux)` passed in run `38685360793`.
  Quality, format, SDK, cross-target and release-automation checks passed;
  Linux/macOS/Windows build-and-test and Rust analysis remain in progress.
- Independent Astra/high review found no blocking introduced defect in the four
  fix commits. It corroborated the patched dependency graph against upstream's
  released lockfile. This does not replace full candidate CI acceptance.
- The review identified a separate baseline gap: `jcode/privacy-no-collection`
  remains unintegrated. Current telemetry defaults to enabled unless opted out.
  Preserve existing environment/marker opt-outs and outbound sanitization, but
  do not claim unconditional no-collection or import that branch's unrelated
  ancestry as part of stable synchronization.
- The upstream integration packet also requires semantic CI reconciliation:
  retained fork-only quality steps need upstream's `!cancelled()` condition to
  satisfy its new workflow contracts. This is a static prediction from review,
  not a reported execution failure on a merged tree. A conflicted integration
  needs an isolated writer and subsequent independent review, not partial merge
  state in a checkout used by active harness sessions.
- Read herdr's own project/contribution rules, task recipes and pinned tooling.
  Its full acceptance command is `just check`, not jcode's cargo commands.
  Installed Zig is 0.16.0 while its workflows require 0.15.2; no installation,
  downgrade, release, publication or source change was attempted.
- Herdr's existing `just maintenance-test` ran 101 tests and failed once:
  `PreviewNotesTests.test_post_stable_history_selects_release_and_bases_range_on_stable_tag`
  could not create its temporary `v0.7.0` tag. Both user Git signing settings are
  enabled. A process-local rerun of that single test with commit/tag signing
  disabled passed. This isolates a non-hermetic fixture, not a passing full
  fleet gate. Global Git configuration and real commit-signing policy were not
  changed. Subsequent chained recipes did not run after the failure.
- Next herdr repair packet should isolate signing only in its temporary test
  repository and prove behavior with hostile inherited signing configuration.
  Respect its commit-alignment/contribution policy before publication; do not
  silently disable signing for actual repository commits or upstream releases.
