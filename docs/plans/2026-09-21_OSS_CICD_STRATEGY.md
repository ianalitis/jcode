# OSS CI/CD strategy for the public fork portfolio

**Date:** 2026-09-21. **Status:** items 1-3 applied and verified, with results in the
[rollout receipt](../OSS_CICD_ROLLOUT_2026-09-21.md). The remaining §7 actions are
still not authorized or executed.

**Scope:** how CI/CD is organized around the public open-source repositories this
account maintains as forks, starting with `ianalitis/jcode` and extending to the
same posture for the other public forks. It covers tool selection, where each
change is allowed to live, and the approval each change needs.

**Evidence basis:** GitHub API reads of all five public forks, live Actions run
history for `ianalitis/jcode` and `1jehuang/jcode` (2026-09-21), the workflow
sources in this checkout, and the ingested vendor comparison at
`~/Downloads/best alternatives to this common github plugins.md`. Every claim
below marked *verified* was observed in this session; vendor marketing and
pricing claims are marked *unverified*.

## 1. What the ingested source recommends, and our verdict

The source is a Greptile-alternatives comparison. Its recommended stack for a
public repo is layered by function: AI code review (Greptile), security static
analysis (CodeQL, Semgrep), dependency hygiene (Dependabot or Renovate), and
GitHub Actions compute. Its free-tier reasoning is sound, but it was written for
a conventional single-repo workflow and misses the constraint that actually
governs this account: our repositories are **forks whose default branches mirror
someone else's project**.

| Source claim | Verdict here |
| --- | --- |
| GitHub Actions minutes are unlimited/free for public repos on standard runners | **Accepted.** This is documented GitHub policy, not a vendor claim. Cache and artifact storage have separate allowances (`docs/FORK_CI.md` already records this). |
| GitHub Advanced Security / CodeQL is free on public repos | **Accepted, and now enabled.** It was unexploited (`state: not-configured` on all five forks); it is now configured on all five, and it delivers real Rust coverage. See the finding below. |
| Greptile is free for OSS under their open-source program | **Unverified vendor claim.** The source itself flags that the program is manually reviewed, limited to qualified non-commercial MIT/Apache projects, and that marketing pages disagree about whether "free" means unlimited. Trust it only after written confirmation. |
| CodeRabbit / Qodo / Cursor BugBot / Graphite at `$24-48/user/month` | **Rejected.** Per-seat pricing is the wrong shape for a solo maintainer, and a second AI reviewer on a fork nobody else reviews adds cost without adding a decision. |
| Macroscope usage-based (`$0.05/KB`) | **Rejected as a default.** Usage-priced review on a fork portfolio is an unbounded spend with no enforcement, which is precisely the condition that blocks all metered dispatch in `policy/providers.md`. |
| Semgrep Community / OSS | **Deferred.** Real value, but it overlaps CodeQL on the layers we can already get free, and Semgrep's Rust coverage is weaker than its JS/TS and Python coverage. Revisit only if CodeQL leaves a language uncovered. |
| Dependabot and Renovate as "free and native" | **Split by repository.** Correct upstream, actively harmful on a mirror fork. See §2 and §6. |
| General vendor comparison tables and "Nx more bugs" benchmarks | **Unverified.** Vendor-published head-to-heads. Any AI reviewer we keep should be judged on our own PR history, not on vendor tables. |

The one recommendation worth adopting wholesale is the source's *layering*
principle: each tool should own a distinct job, and a layer that duplicates
another layer's findings is noise, not coverage. The rest of this document is
that principle applied to a fork portfolio rather than a single repository.

### Finding: CodeQL's Rust coverage is real, and it is slow on a large crate

Added after enabling it and reading the results to completion. CodeQL does
analyze Rust here: `/language:rust` analyses were recorded on `jcode`,
`handterm`, `mermaid-rs-renderer`, and `agentgrep`. The `rust` entry in the
advertised language list is not a menu item that quietly does nothing.

An earlier draft of this section said the opposite, and the way it went wrong is
worth keeping. `jcode`'s Rust analysis took roughly twenty minutes; every other
language on the same commit finished in under ten. A snapshot taken at the
fifteen-minute mark therefore showed `actions`, `javascript-typescript`, `python`
and `swift` complete with zero `/language:rust` analyses recorded, which reads as
"Rust is not supported" if you stop there. It is not. **Do not read a mid-run
snapshot as a capability limit**; either wait for the job to settle or, faster,
check a smaller repository with the same language, where `handterm` had already
recorded `/language:rust` while `jcode` was still running.

CodeQL also produces signal immediately rather than only infrastructure. On
`jcode` it opened 100+ Rust alerts, 6 `critical`
(`rust/hard-coded-cryptographic-value`) and 94 `high` (90
`rust/cleartext-logging`, 4 `rust/cleartext-transmission`). A first pass suggests
the six criticals are heuristic false positives — two are in
`jcode-tui/src/tui/ui_animations.rs`, a TUI animation module, and four are in
`jcode-tui-mermaid/tests/layout_cache_resize_probe.rs` — but that is a
first-pass impression, not a triage. The `rust/cleartext-logging` cluster is not
uniformly test code: `src/cli/login.rs` carries 24 of them and
`src/cli/commands.rs` 13, in an authentication-heavy CLI, which is exactly where
that rule is worth reading rather than bulk-dismissing.

None of this fails a check: CodeQL default setup reports findings without
failing the workflow, because `security_baseline` is not the
`security-and-quality` suite. That is the advisory-first mode the source document
recommends, and it means the alerts are a backlog to triage, not a blocked merge.

So CodeQL is worth having for exactly what it delivered — Rust, the workflow
files, the TypeScript SDK, and the Python tooling — and the repository's other
Rust signal (`scripts/security_preflight.sh --strict`'s cargo-audit, and the
clippy gate) is now complementary rather than the only layer.

## 2. The placement rule

This is the decision everything else follows from, and it is not in the source
document.

Work added to a fork's default branch is **divergence debt**: it must be
re-applied across every upstream merge, and `docs/FORK_POSTURE.md` treats the
fork's default branch as a mirror of upstream that should stay as close as
possible. So:

1. **A change that upstream would sensibly accept goes upstream first** (issue,
   then a `pr/*` branch on a current upstream base). Fork-local copies of
   general-purpose CI are only justified when the fork must be green *before*
   upstream acts, and then they should be identical to the proposed upstream
   version so the fork copy can be deleted once the upstream PR lands.
2. **A change that only makes sense for this account's fork stays fork-local.**
   Examples: secretless validation, fork-scoped artifact uploads, skipping jobs
   that need secrets the fork does not have.
3. **Fork-local CI prefers new files over edits to upstream files.** A new path
   never conflicts on merge; every edit to an upstream workflow is a permanent
   merge conflict. This is why `workflow-lint.yml` and `docs/FORK_CI.md` are
   cheap to carry and edits to `ci.yml` are not.
4. **Configuration beats code.** Settings that GitHub exposes as repository
   configuration (code scanning, dependency alerts, rulesets) cost no divergence
   at all. Prefer them over workflow files wherever they exist.
5. **Lowest divergence wins ties.** When two options give the same capability and
   one is a repository setting, the setting wins.

## 3. Verified current state

`ianalitis/jcode` is in better shape than the source document assumes. Recorded
here so the gap register is not inflated:

| Already present | Evidence |
| --- | --- |
| Fork CI is secretless and green where upstream is green | `docs/upstream-feedback/2026-09-21-fork-ci-default-branch.md`; `Format`, `iOS TestFlight`, `Windows Cross-Target Check`, `Workflow Lint` all pass on `master` |
| Push CI runs on branches that **contain** the fork's `ci.yml` | `3e5c99e49`; confirmed by runs on `jcode/ci-format-baseline` and `pr/tui-lib-test-failures`. **Corrected 2026-09-22**: upstream's trigger is `push: branches: [main, master]` and a workflow only runs if it is present in the pushed commit, so a `pr/*` branch based on `origin/master` gets no fork push CI. Opening a fork PR against `fork/master` is what validates it. Rollout receipt §6b |
| Rust advisory scanning exists in CI | `scripts/security_preflight.sh --strict` (secret-pattern scan, world-writable check, `cargo-audit`) invoked from the `Build & Test` Linux leg |
| Workflow lint with pinned tooling | `workflow-lint.yml`, actionlint 1.7.12 + shellcheck |
| Release checksums are produced and verified by installers | `release.yml` "Generate checksums" writes `SHA256SUMS`; `scripts/install.sh` and `scripts/install.ps1` both verify |
| npm provenance for the TypeScript SDK | `publish-typescript-sdk.yml` uses `npm publish --provenance` |
| Secret scanning and push protection enabled on all five public forks | `security_and_analysis.secret_scanning = enabled`, `secret_scanning_push_protection = enabled` on every repo checked |
| Repositories are MIT-licensed public projects | `license.spdx_id = MIT` on `jcode`, `GLOOP`, `handterm`, `mermaid-rs-renderer`; `agentgrep` reports `null` and needs checking |

What is genuinely missing:

| Gap | Evidence |
| --- | --- |
| Code scanning is unconfigured on all five public forks | `GET /repos/{r}/code-scanning/default-setup` → `{"state":"not-configured"}` for every repo. **Resolved** for all five; see the rollout receipt. |
| Dependabot alerts and security updates are off on all five | `security_and_analysis.dependabot_security_updates = disabled`; `GET /vulnerability-alerts` → 404. **Alerts resolved** for all five; security updates deliberately left off. See the rollout receipt. |
| Actions are not required to be SHA-pinned | `actions/permissions` → `allowed_actions: all`, `sha_pinning_required: false` |
| No branch protection or ruleset on the fork default branch | `GET /branches/master/protection` → 404; `rulesets` → empty |
| Release binaries carry no build provenance | no `attest-build-provenance` or SBOM step anywhere in `release.yml`; `SHA256SUMS` is unsigned and hosted on the same origin as the assets it describes |
| Four of five public forks have never executed a single workflow run | `gh run list` returns empty for `agentgrep`, `handterm`, `mermaid-rs-renderer`, `GLOOP`; `agentgrep` and `GLOOP` have no `.github/workflows` on their default branch at all. **Cause corrected:** in `handterm` and `mermaid-rs-renderer` the workflow files exist in the tree but are not registered, so there was nothing to run. CodeQL now runs on four of them. See the rollout receipt. |
| One permanently red check on `ianalitis/jcode` | `Semantic PR labels` fails on every Greptile review with `"reason": "OPENROUTER_API_KEY is required."` — see §8 item 1, fixed on the integration line in this session |

Upstream carries the same `Semantic PR labels` defect for a different reason:
`1jehuang/jcode` fails with `"reason": "API request failed: HTTP 402. No labels
should be assumed updated."` That is an upstream OpenRouter credit/configuration
problem only its owner can fix, and it should be reported rather than worked
around.

## 4. Gap register

Ranked by (value to us) / (cost + divergence + risk). "Approval" is the
authorization the change still needs before it can be executed.

| # | Change | Effect | Cost / divergence | Approval needed |
| --- | --- | --- | --- | --- |
| 1 | Gate `label-pr.yml` on labeler configuration | Removes a permanently red check that masks real failures; already committed on the integration line | Small edit to an upstream file; also a clean upstream contribution | Publish to fork default branch requires operator approval |
| 2 | Enable CodeQL default setup on the five public forks | Free security static analysis across `actions`, `javascript-typescript`, `python`, and whatever else each repo's language detection covers | Zero code, zero divergence | Repository settings mutation, external effect |
| 3 | Enable Dependabot alerts on the five public forks | Vulnerability visibility without opening PRs against a mirror | Zero code, zero divergence | Repository settings mutation, external effect |
| 4 | Confirm fork CI actually runs on the four inert forks | Turns "unknown" into a fact; likely surfaces the same missing-secret class of failure that `jcode` had | Read-only plus a manual dispatch | Push or `workflow_dispatch` on those repos |
| 5 | Add `dependabot.yml` upstream only | Dependency hygiene where PRs are actually merged | Zero fork divergence if upstream-only | Upstream contribution |
| 6 | Require SHA-pinned actions, then tighten `allowed_actions` | Closes the tag-mutation supply-chain hole on a repo that publishes binaries | Requires pinning every `uses:` first, or CI breaks immediately | Repository settings plus an upstream contribution |
| 7 | Add `actions/attest-build-provenance` to the release path | Users running an installer can verify which workflow built the binary; makes `SHA256SUMS` tamper-evident rather than merely convenient | Edits a 36 KB upstream workflow and adds `id-token: write` / `attestations: write` | Upstream contribution, with its own verification |
| 8 | Add a ruleset protecting the fork's default branch | Prevents accidental force-pushes and deletions of the mirror line | Must not block the operator's own approved mirror pushes | Repository settings mutation; needs a ruleset design that permits the documented sync |
| 9 | Fork-portfolio CI template | Gives the four inert forks the same posture `jcode` has | One small workflow file per repo, additive | Per-repo operator approval |
| 10 | Resolve the unguarded `Release` workflow on the fork | `Release` is active on the fork, fires on `push: tags: ['v*']`, holds `contents: write`, and has no `github.repository` guard, so a `v*` tag pushed to the fork today starts a fork release | Zero divergence if the workflow is disabled on the fork rather than edited | Repository setting (disable), or an upstream gated-workflow change. **Applied 2026-09-22**: disabled on the fork (`state: disabled_manually`), recorded in the rollout receipt §5. The upstream `github.repository` guard is still open |
| 11 | Register workflows on the forks that have them | `handterm` and `mermaid-rs-renderer` carry `ci.yml` (mermaid also `release.yml`) but have no registered workflow, so they have never run and cannot be dispatched | **Not configuration only**: `gh workflow enable ci.yml` returns `HTTP 404: workflow ci.yml not found on the default branch`, because an unregistered workflow has no Actions-registry entry to enable. Registration appears to follow a *processed push event*, and the same push that registers `ci.yml` also registers `mermaid-rs-renderer`'s publishing `release.yml` | Needs an operator decision, and on `mermaid-rs-renderer` a guard on `release.yml` **first**. See the rollout receipt §4 for the evidence |
| 12 | Triage the CodeQL alert backlog | 166 open alerts on `jcode` (6 `critical`, 158 `high`), dominated by `rust/cleartext-logging`; `src/cli/login.rs` alone carries 24 | Reading, then either dismissal or a focused fix; no CI change | **Done 2026-09-22**: read and concluded in [`docs/upstream-feedback/2026-09-22-codeql-rust-alert-triage.md`](../upstream-feedback/2026-09-22-codeql-rust-alert-triage.md). Zero confirmed leaks; two actionable items (`freebsd-smoke.yml` job permissions, `sanitize_secret_value` naming). No alert dismissed |

**Applied since this table was written:** items 1, 2, 3 and 10. Item 12 is read and
concluded. See the [rollout receipt](../OSS_CICD_ROLLOUT_2026-09-21.md) and the
[triage receipt](../upstream-feedback/2026-09-22-codeql-rust-alert-triage.md).

Items 1 through 4 are the high-value, low-cost set. Items 6 through 8 are real
hardening but each can break something if applied before its precondition
(item 6 breaks CI on contact if actions are still tag-referenced). Item 10 is
cheap and should go first if the fork's 30 existing `v*` tags are ever joined by a
new one.

## 5. Public fork portfolio

The five public forks, with what each one needs. This is an inventory for a
later deep dive, not a work plan; `jcode` is the only one audited in depth.

| Repo | Parent | Default | Workflow files in tree | Registered workflows | First action |
| --- | --- | --- | --- | --- | --- |
| `ianalitis/jcode` | `1jehuang/jcode` | `master` | 11 | 12 (incl. `CI`, `Release`, `CodeQL`) | item 10 (unguarded `Release`), then items 6-8 |
| `ianalitis/handterm` | `1jehuang/handterm` | `master` | `ci.yml` | **`CodeQL` only** | item 11. `ci.yml` is secretless, so it is safe on its own terms; **the CLI cannot register it** (rollout receipt §4) |
| `ianalitis/mermaid-rs-renderer` | `1jehuang/mermaid-rs-renderer` | `master` | `ci.yml`, `release.yml` | **`CodeQL` only** | **Guard `release.yml` before anything registers workflows here.** It is armed by the same push that would register `ci.yml`, and it creates a fork release (`softprops/action-gh-release@v3`) and runs `cargo publish` |
| `ianalitis/agentgrep` | `1jehuang/agentgrep` | `master` | none | `CodeQL` only | decide whether this fork needs CI at all before upstream has any |
| `ianalitis/GLOOP` | `redacktion/GLOOP` | `main` | none | none | confirm whether this fork is meant to be maintained; if not, leave it alone |

Two facts to establish before the deep dive, because both change the answer:

- **Upstream CI coverage varies.** `1jehuang/handterm` and
  `1jehuang/mermaid-rs-renderer` have `ci.yml`; `1jehuang/agentgrep` has no
  `.github/workflows` at all; the `GLOOP` upstream that `ianalitis/GLOOP` names
  as its parent returned 404, so the parent relationship may be stale. A fork
  cannot inherit CI that its parent never had.
- **A fork with no runs is not the same as a fork with passing CI.** All four
  inert forks report `actions/permissions.enabled = true`, so the most likely
  explanation is that Actions were enabled after the mirror push and no push has
  happened since. **Dispatch is not available either**: item 11's `gh workflow enable` returns 404 because an unregistered workflow has no registry entry, so the confirmation path is a push or the Actions-tab enable flow. See the rollout receipt §4.

The same operating posture from `docs/FORK_POSTURE.md` is intended to apply to
each of these: local `master` mirrors upstream fast-forward-only, an integration
line carries our commits, and `pr/*` branches carry one issue's contribution on
a current upstream base. None of the four have that posture documented yet.

Two repositories present in the local source tree are **not** in this portfolio,
and should not be folded into it: `~/.jcode/source/herdr` and
`~/.jcode/source/isohypse` are checkouts of `herdrdev/herdr` and
`redacktion/isohypse` with no fork of either on this account, so there is no
fork default branch to configure, nothing to contribute back through, and no
fork-relationship review context to keep consistent. The only other public
repository on the account, `UTAustinAIMLProjects`, is not a fork and is out of
scope.

## 6. Deliberate exclusions

Recording these because each is a plausible-looking addition that we are
choosing not to make.

- **No Dependabot or Renovate config on fork default branches.** The default
  branch is a mirror. Version-bump PRs against it are churn we would have to
  close or sync, and they widen the fork diff for no benefit. Dependency updates
  belong upstream, where they are actually merged. Security *alerts* (item 3) are
  the exception: they notify without opening PRs.
- **No second AI reviewer.** Greptile is already installed on both upstream and
  the fork (`app.id 867647`, used by `label-pr.yml`). Adding CodeRabbit or
  Macroscope means paying twice for overlapping findings and showing two
  disagreeing bots on a fork that has one human reviewer.
- **No Semgrep until CodeQL proves insufficient.** Two static-analysis engines
  producing overlapping findings is the noise problem the source document warns
  about, and we are not yet consuming the free engine we already have.
- **No seat-priced anything.** `$24-48/user/month` is not a free-tier decision.
- **No paid review actions in fork CI.** `docs/FORK_CI.md` already records that
  fork validation does not enable "paid review actions"; keep that boundary, and
  note that the labeler's OpenRouter call is currently the one paid dependency in
  a workflow that also runs on the fork.
- **No self-hosted or larger runners.** The workload is already 30-40 minutes per
  push on standard runners; the fix for that is scoping, not buying hardware.

## 7. Operator action queue

None of these are executed. Each is an external effect on a public repository
and needs explicit approval naming the target. Preconditions are listed so they
can be approved in the right order.

**A. Enable code scanning (item 2). Applied** to all five forks on 2026-09-21.
Rust coverage is real but slower than the other languages on a large crate, and
the first analysis opened a triage backlog: see §1's finding. Repeat per
repository for a new fork:

```sh
gh api -X PATCH repos/ianalitis/jcode/code-scanning/default-setup \
  --input - <<'JSON'
{"state":"configured","query_suite":"default"}
JSON
```

Omitting `languages` lets detection choose. The available-language list returned
by the API for `jcode` is `actions, javascript, javascript-typescript, python,
rust, swift, typescript`; confirm after enabling which languages actually produce
analyses rather than assuming Rust is covered.

**B. Enable Dependabot alerts (item 3).** Alerts only; no PR churn.

```sh
gh api -X PUT repos/ianalitis/jcode/vulnerability-alerts
```

Optionally add security updates, accepting that they open PRs against the mirror:

```sh
gh api -X PUT repos/ianalitis/jcode/automated-security-fixes
```

**C. Prove the four inert forks (item 4).** Read-only inspection first, then a
manual dispatch on a branch that exists:

```sh
gh workflow list --repo ianalitis/handterm
gh run list --repo ianalitis/handterm --limit 5
```

**D. SHA pinning and action allowlisting (item 6).** Strictly ordered: pin first,
tighten second. Setting `sha_pinning_required: true` while workflows still use
`actions/checkout@v4` and `dtolnay/rust-toolchain@stable` fails every job
immediately.

**E. Greptile open-source program.** Operator-only, and the source document
explicitly recommends getting the answer in writing: confirm whether approval
means unlimited credits or a larger capped allowance, and whether a *fork* of an
already-enrolled project qualifies or only the upstream project does. Do not
assume the fork inherits upstream's enrollment just because the Greptile app is
installed on it.

## 8. Upstream contribution queue

Both entries follow the contribution path in `AGENTS.md`: issue first, then one
`pr/*` branch on a current upstream base, with the fork copy deletable once
merged.

1. **`Semantic PR labels` fails on a repository without the labeler's provider
   key.** Fork evidence: `"reason": "OPENROUTER_API_KEY is required."` with exit
   code 1. The workflow's own precondition is that the labeler can call its
   provider, and nothing checks that before the run, so a configuration gap
   becomes a permanently red check that hides real results. The fix on our
   integration line adds a gate job (secrets are unavailable in a job-level
   `if:`, so the check needs a job and an output) and skips the work instead of
   failing it. Recorded in
   `docs/upstream-feedback/2026-09-21-greptile-labeler-missing-key.md`.
2. **Upstream's own `HTTP 402` failure.** Distinct from item 1 and not ours to
   patch: the labeler's provider account needs attention. Worth one issue so the
   red check upstream is understood, with no code change proposed.

## 9. Verification and rollback

Each item was chosen so its effect is observable and its failure is reversible:

- Item 1: `actionlint -no-color -shellcheck=shellcheck` passes on the whole
  workflow set (verified in this session). Behaviorally, the check should
  transition from `failure` to `skipped` on the next Greptile review, with a
  `::notice::` annotation naming the missing secret. Rollback is reverting one
  commit.
- Item 2: after enabling, `GET /code-scanning/default-setup` returns
  `state: configured` and analyses appear under the Security tab. Rollback is
  `PATCH` with `{"state":"not-configured"}`.
- Item 3: `GET /vulnerability-alerts` returns 204 instead of 404. Rollback is
  `DELETE` on the same path.
- Item 4: a run appears in the repo's run list. No side effects beyond compute.
- Item 7, when proposed: verification is that a release's attestation verifies
  with `gh attestation verify <artifact> --repo <repo>`.

Setting changes are not visible in the repository's file history, so each applied
change should be recorded in a dated receipt the same way the fork CI repair was,
otherwise the configuration drifts silently.

## 10. Open questions

- Does the Greptile OSS program's free tier cover a fork, and is it unlimited or
  capped? Requires a written answer from the vendor before any workflow depends
  on it (§7 E).
- Is `ianalitis/GLOOP` still a maintained fork? Its named parent returns 404.
- Should the fork's default branch be protected by a ruleset at all, given the
  posture requires the operator to publish approved mirror updates to it? A rule
  that is safe for normal contributors can block the documented sync.
- Is `agentgrep`'s missing license metadata upstream's problem or ours? It is the
  only one of the five public forks reporting a null license, and the Greptile
  OSS program requires an OSI-approved license.
