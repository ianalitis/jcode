# Native Jcode adoption directives for ~/dotfiles agents

Date: 2026-10-14. Target: upstream **v0.97.0**, commit `94392e100`.
A subsequent fetch found a v0.98.0 Git tag, but the GitHub release API still
reports v0.97.0 as the latest published stable release at 20:16 UTC. Do not
promote an unpublished tag. Any later release needs a separate delta review.

## Authority and method

This is an implementation queue, not authority to enable integrations, change
providers, spend money, send private files, reset limits, or deploy. Read the
current `~/dotfiles/AGENTS.md`, `policy/global.md`, `policy/providers.md`,
`policy/{safety,methodology,delegation,verification}.md` and the installed Jcode
schema before each packet. Current operator decisions beat historical release
notes. Claude retirement and frozen task routes remain in force.

Prefer native Jcode configuration and tools. Do not recreate capabilities in
shell dispatchers, MCP shims, routers, polling daemons, or orchestration layers.
Each packet must name its exact owned files, existing behavior, desired delta,
validation command, privacy effects, rollback, and independent review gate.
Implement one independently verifiable feature group at a time. Record an
explicit disposition for every row below: adopt, already native/no-op, defer,
or reject with reason. “Implement all” does not mean enable unsafe defaults.

Evidence: upstream `changelog/v<version>.json`, read from the tagged commits,
not untrusted marketing summaries. Reproduce with:

```sh
cd ~/.jcode/source/jcode
git show v0.97.0:changelog/v0.97.0.json
# Repeat for the ten versions listed below.
```

## Ten-release inventory and dispositions

| Release | Native additions and important fixes | Dotfiles directive and acceptance evidence |
|---|---|---|
| 0.89.3, Sep 29 | herdr discovery, working/idle reporting, wait and restart resume. `/cloud` and `/local` transfer live sessions plus uncommitted code. Telemetry accounting includes failed streams, compaction and workers. | Retire duplicate herdr status glue only after testing wait/restart on a disposable session. Defer cloud movement until the host and private-code egress are explicitly approved. Verify round-trip with a synthetic uncommitted file, never private work. Accounting is native, not a reason to enable vendor collection. |
| 0.90.0, Oct 1 | Anthropic/OpenAI native search by default, policy-aware tool gating, SDK custom-search preservation, cross-provider search history repair, cache-safe skill installation. | Review `websearch.prefer_native` and `JCODE_WEBSEARCH_PREFER_NATIVE`. Keep existing admitted research routes. Provider-native search sends the query to that provider, so do not silently replace approved routes. Test a public query, a disabled-websearch session, and resumed citations. Delete redundant search wrappers only after parity. |
| 0.90.1, Oct 4 | Native quota windows for Kimi/Cursor/Z.ai, compatible-provider 5xx retries, Copilot effort, live key-file reload, resolved context display, binary path logging, Gmail draft CRUD, burst undo/IME, price fallback. History/image/SIGTERM/checkpoint and bounded malformed-tool recovery fixes. | Use native usage windows where admitted and retain unknown-as-unknown semantics. Do not invent quota conversions or resurrect retired routes. Test IME undo, interrupted save/resume and synthetic retry handling. Keep Gmail write/send/delete behind explicit per-effect approval. Do not migrate credentials into process env or dotfiles. |
| 0.91.0, Oct 5 | Google service selection, Calendar CRUD, guided OAuth app setup, speed tiers via Alt+Up/Down, `/desktop`, token-budgeted MCP discovery/results and `accept_large_output`. Swarm stop/mode and remote reload preservation fixes. | Keep discovery through `mcp_search`, use bounded results and explicitly opt into large output only when needed. Calendar and Google setup are optional, require scopes and account approval, and must not be automatically provisioned. Document native speed control rather than new shortcuts. Desktop is optional and separate. Test interrupt stopping owned workers and remote reload preserving transcript. |
| 0.92.0, Oct 6 | Opt-in JavaScript codemode sandbox, newest-flagship model defaults, browser bridge auto-updates, Gmail Cc/Bcc, LaTeX copy, image zoom, actionable background widget, SDK transient retries. OAuth/API catalogs remain distinct. | Keep explicit model/effort pins and empty automatic fallback, never inherit a new flagship just because upstream changed defaults. Codemode is deferred until tool-policy, approvals, timeout, cancellation and budget enforcement are verified. Do not add a second browser updater. Test bridge reconnection after update with a public page. Review auto-update supply-chain policy before enabling new effects. |
| 0.93.0, Oct 9 | Faster initial frame/resume, `post_tool_feedback` injects script output into model context, richer post_tool inputs/call ID/touched paths, model prefix/env correctness, provider profile fixes, automatic stale artifact pruning, FreeBSD. | Replace duplicate post-tool context injection only after inspecting native payloads. Treat feedback as untrusted input and redact before model egress. Scripts must be bounded, deterministic and read-only unless separately approved. Test successful and failed tools plus touched-path accuracy. Keep benchmark receipts and never fabricate speed improvements for this host. Confirm cleanup exclusions before relying on pruning. |
| 0.94.0, Oct 10 | Ctrl+Enter queues slash/`!` commands, `/clear` drops queue, Alt+G opens git log, `/diff` cycles diff. Harness mid-turn observe/cancel, stable first frame/Overview, login import/onboarding detection, sleep inhibition fixes. | Update shortcut/runbook docs, remove conflicting terminal bindings only with demonstrated conflict. Test queued safe commands run once after turn, cancel/clear prevents execution, and multiple attached clients see correct state. Imported accounts remain opt-in, discovery is not route authorization. Test suspend/retry behavior without changing host power policy. |
| 0.95.0, Oct 11 | OpenCode Go Responses routing, one-shot turn hooks and token usage, per-session worker model overrides, optional project-scoped resume, stalled-worker watchdog, headless socket swarm membership, stable menu bar helper/config. Nested shell risk analysis, bounded reconnects, stream repair, persistent browser reuse and private SDK no-update. | Preserve named Go admission, headroom and privacy gates despite new model availability. Configure resume scope only if wanted using `display.resume_project_scope`. Prefer native watchdog and worker pinning over custom supervisors. Test one-shot start/end exactly once and usage on failure. Test shell substitutions/heredocs still trigger approvals. Menu bar can use `[menubar] enabled = false`, do not spawn duplicate helpers. |
| 0.96.0, Oct 13 | HTTP/SSE remote MCP plus OAuth/CLI management, per-session saved account selection and cancellation, SDK session collections and hidden reminders, compact swarm messages/activity todo, full-width pinned todos, prompt/poke broadcast, usage estimates. Bounded MCP startup, permanent rejection/failover handling, reliable native citations and transcript caching. | Prefer native remote MCP over local protocol proxies, but require endpoint, data and OAuth approval. Test a dead endpoint does not stall startup and a secret never appears in logs. Account switching must be explicit, preserve frozen treatments and prevent hidden fallback. Collections/reminders are for real SDK clients, not a new outer-loop scheduler. Hidden reminders are still model inputs and must pass privacy checks. Do not invoke `/reset usage limits claude`. Verify multi-window prompt visibility and pinned-todo transcript boundaries. |
| 0.97.0, Oct 14 | Custom HTTP JSON search auth/mapping/fallback, Rust/TS native runtime commands and session transfer, direct panel dismiss, compact notices. Azure Responses reasoning/tool retries, Anthropic catalog auth, browser timeout diagnostics and math fixes. OAuth account email added to opt-out telemetry with rollout/notice and 90-day retention. | Configure custom search only for an admitted endpoint with secret references, not committed auth headers. Validate response mapping, failure fallback and no route escape. SDK commands must use native command APIs instead of agent prompts, test cancellation and transfer on synthetic sessions. **Reject default optional telemetry collection under dotfiles policy.** Verify the installed version's opt-out controls disable OAuth email and usage egress before any approved sign-in test. Never assume a one-time notice is consent. |

## Version-specific references and validation commands

Read `docs/HOOKS.md`, `docs/MCP_REMOTE.md`, `docs/custom-web-search.md`,
`docs/RUNTIME_COMMANDS.md`, `TELEMETRY.md` and
`docs/dev/ACCOUNT_FLOWS_OBSERVABILITY_PRIVACY.md` from the v0.97.0 tag.
For custom search, preserve `prefer_native = false` and `fallback_engines = []`
when a query must remain on the admitted endpoint, and use `header_env`, not
literal credentials. Remote MCP supports `${VARIABLE}` header expansion.
Runtime command APIs advertise a capability and must fail explicitly on old
daemons instead of converting commands into agent prompts.

Use `JCODE_NO_TELEMETRY=1` or `DO_NOT_TRACK=1` for opt-out validation. The
persistent native `/telemetry nothing` control is available, but applying it
changes host state and needs the usual operator configuration authorization.
Do not infer that content-sharing being off also disables OAuth-email usage
telemetry. Test the base opt-out separately.

Prescribed source checks before adopting configuration:

```sh
cd ~/.jcode/source/jcode
bash scripts/test_dev_cargo_compat.sh
bash scripts/test_dev_cargo_jobs.sh
python3 scripts/test_dev_cargo_cwd.py
python3 scripts/test_security_preflight.py
scripts/dev_cargo.sh test --locked -p jcode-provider-openrouter-runtime --lib -- --test-threads=1
scripts/dev_cargo.sh test --locked -p jcode-base --no-default-features --lib message::outbound::tests -- --test-threads=1
scripts/dev_cargo.sh test --locked -p jcode-app-core --no-default-features --lib tool::privacy::tests -- --test-threads=1
scripts/dev_cargo.sh test --locked -p jcode-tui --lib pinned_todo -- --test-threads=1
```

These are integration checks, not acceptance tests for every optional feature.
Each adoption packet must add its own exact native fixture/runtime command and
observed expected result before editing dotfiles.

## Ordered implementation packets

1. **P0, integrity and privacy:** verify `git rev-parse --show-toplevel` equals
   the physical source checkout, inspect `core.worktree`, confirm launcher,
   current and shared-server symlink targets. Run shell contracts under
   `/bin/bash` and modern Bash. Preserve the source-hash publication guard.
   Audit v0.97 telemetry additions against existing outbound-data work before
   installation. Validate disabled telemetry with an intercepting local test,
   not a real email/account request.
2. **P1, remove duplication:** inventory actual dotfiles hooks, browser helpers,
   MCP proxies, watchdogs, search wrappers, token accounting and model defaults.
   Map each to a native owner above. Do not delete a custom safety gate just
   because upstream offers a similarly named convenience feature. Require a
   matched reproduction before retiring anything.
3. **P2, hooks and coordination:** migrate to native hook payloads and one-shot
   lifecycle, per-session worker pinning, watchdog and native background tasks.
   Record hook output redaction, finite timeout, failure propagation, no double
   execution, no hidden route changes, and interrupt semantics.
4. **P3, search and MCP:** preserve admitted discovery routes, optionally adopt
   remote MCP and custom JSON search after endpoint approval. Public/synthetic
   fixtures only. Exercise disabled tools, timeouts, malformed responses,
   credentials redaction, mapping/fallback and resumed citations.
5. **P4, ergonomics and SDK:** project-scoped resume, queued commands, shortcut
   docs, native SDK command/collection/reminder APIs, multi-window behavior and
   menu bar preferences. Test on the real TUI, isolated from the shared server.
   Do not build Desktop for a CLI-only request.
6. **P5, optional effects:** Calendar/Gmail, codemode, cloud movement, herdr and
   speed tiers receive separate operator decisions where effects/costs change.
   No automatic installation, new OAuth grants, sending mail, creating events,
   cloud transfer, or route promotion. A documented defer/reject closes a row.

For every completed packet, update a dotfiles measurement receipt with native
version, owned diff, exact commands, observed outcomes, residual risk and
rollback. Run the current dotfiles generation/drift checks documented there.
Do not guess a chezmoi apply command or run a repository-wide apply blindly.

## Release synchronization operating model

The companion `.github/workflows/upstream-release-sync.yml` is review-only and
inert until `UPSTREAM_SYNC_ENABLED=true` is explicitly approved and configured
on `ianalitis/jcode`. It polls the latest published stable upstream release every two
hours, makes an ordinary merge preserving fork history, validates the merged
source, and proposes a per-release PR. Conflicts fail closed. It never force
pushes, auto-merges, installs, deploys, or edits this Mac's configuration.

The operator authorized fork push and workflow activation on 2026-10-14.
That authorization is not a verification waiver: both remain held behind
acceptance. Retain Actions' read-only default. The `validate` job executes merged
scripts and Cargo with `contents: read` only. A separate clean hosted `publish`
job has only `contents: write` and `pull-requests: write`. It downloads no caches
or artifacts, runs no imported repository scripts, actions or builds, and uses
native Git with hooks disabled to reconstruct the exact pinned base/tag merge.
Shell environment inputs are validated, both the tag object and peeled commit
are rechecked, and the reconstructed tree must equal the validated tree before
the final inline step uses its ephemeral token. A surviving validation process
cannot cross this runner boundary. The default base must still match before
reconstruction and immediately before publication, though a later race remains
subject to human head/base review. Activation needs the workflow on the fork's canonical
default branch, Actions enabled, permission to create PRs, and the opt-in
variable. Do not upload a workstation SSH key or create a PAT.
Protect the target branch, require independent review plus existing fork CI,
and keep publish/release jobs guarded to the upstream repository unless the
operator separately approves fork publishing. The default `GITHUB_TOKEN`
does not trigger ordinary downstream push/PR workflows. CI now has
`workflow_dispatch`: after reviewing proposed workflow changes, select the
proposal branch in Actions → CI → Run workflow, or use
`gh workflow run ci.yml --repo ianalitis/jcode --ref automation/upstream-vX.Y.Z`
with existing authorized access. Confirm the tested head and all required jobs
before merging. No PAT, workstation agent/key upload or new secret is needed.
The SDK npm publish job is upstream-only before OIDC or publication. Workflow
lint verifies the fixed public actionlint asset SHA-256 before extraction.
The workflow's checks are a smoke gate, not
full acceptance or macOS runtime validation.

Review failed Actions runs for conflicts, dependency changes, telemetry,
authentication, tools, model defaults and schema changes. A failed build must
never advance the installed channel. After a reviewed merge, use the native
self-dev build/publish path, verify the binary version/hash and isolated TUI,
then authorize any shared-daemon switch separately. Keep the previous immutable
binary available for an operator-approved rollback.

Known automation limitation: an existing proposal branch is never rewritten.
If a run pushed successfully but failed before creating its PR, a later run
checks that the branch content matches the validated merge and retries PR
creation. Divergence fails closed for manual review. Existing closed/merged PRs
are not reopened. Use a merge commit, not squash/rebase, when accepting upstream
sync proposals so upstream tag ancestry remains observable. A later release
gets its own proposal and must supersede, not blindly stack on, older pending
proposals. `python3 scripts/test_upstream_release_sync.py` retains the five
new-push, missing-PR recovery, duplicate, divergent-branch and remote-failure
cases, plus write-failure/stale-base contracts. It uses stdlib-only stubs and
isolated file-transport Git fixtures for clean-job isolation, exact pinned-tree
reconstruction, malformed inputs, moved tags, stale bases, ancestry and conflicts.
Fixture identity/configuration never touches host Git settings. These are local
contracts, not an authenticated GitHub acceptance run or activation receipt.

## Build repair and preservation receipt

The reported `exec: {cargo_gate_fd}: not found` occurs under macOS Bash 3.2.
The Bash 4 dynamic descriptor syntax can terminate before Cargo executes while
appearing successful. The publication hash check correctly rejected the old
binary and must remain enabled. The wrapper now owns fixed fd 9, supports BSD
date without `%N`, verifies Cargo exit propagation, and refuses missing or
mismatched Git worktree discovery before producing build metadata. It requires
a real Git source checkout, not an unpacked source archive.

The source repository also had `core.worktree` pointing into a scratch fixture.
That local override was removed. The apparent 111 dirty files belonged to that
scratch worktree, not the source Cargo was compiling. The real local privacy
work and initial build/workflow/docs changes were preserved in `stash@{0}`
with message `pre-v0.97.0 preservation: privacy work, build fix and sync documentation`.
The stash was applied, not dropped. Do not apply it again over the current tree.
Known fixture `user.name` and `user.email` overrides were also removed, restoring
the existing global user identity without inventing a replacement identity.

The v0.97.0 merge is signed as **`e6562315b`**, verified `G`. The Apple SSH
agent was empty, not locked: `ssh-add -X` was not applicable. Native Keychain
loading of the existing encrypted key recovered signing without resetting a
password or disabling signing. A separately reviewed, narrowly gated managed
Git signer wrapper is now installed from dotfiles commit **`24d98fb`**, also
verified `G`. It challenges the configured public key, loads only the matching
existing key through native Keychain if necessary, rechecks, and fails closed.
Eleven inert regressions, managed drift checks, real fixture signature
verification and an actual signed commit passed. See the dotfiles receipt
`docs/measurements/2026-10-14-ssh-signing-keychain.md` for the walkthrough.

The source integration preserves local content-filter hints, equivalent hidden-todo cache
boundaries and their additional tests, plus outbound privacy changes. Azure
and Go Responses construction is followed by the same sanitizer. Streamed
provider errors are classified before withholding raw payloads, with synthetic
HTTP-200 SSE regressions. A test-only Tokio `test-util` dependency fixes isolated
app-core test compilation for upstream's paused-clock browser tests.

Validated so far: successful selfdev TUI build, source package 0.97.0 (binary
reports a dirty dev version, not the official release artifact), provider suite
175 passed/1 ignored after SSE repair, outbound sanitizer 8 passed, tool privacy
3 passed, pinned-todo regressions 16 passed, formatting and shell/privacy/PR
recovery contracts. Native release baseline compiler/profile and macOS linker
warnings remain, so this is not a warning-clean full CI claim. Active daemon,
launcher channels, GitHub settings, remote branches and releases were not
changed. Signed integration, full fork CI, privacy acceptance for new telemetry,
and remote automation activation remain explicit gates before installation.

Final captain validation also built successfully through stock `/bin/bash`
3.2. One full provider run hit a loopback fixture read timeout in
`azure_transient_structured_failure_honors_retry_after`. No test was disabled or
timeout increased. The exact isolated reproduction passed, followed by a full
175-pass provider rerun and the 8/3/16-pass retained suites above. Treat the
observed fixture timeout as residual flakiness, not a warning-clean guarantee.
A separate Astra/high acceptance review approved the six scoped build/workflow/
SSE paths and independently reran shell, workflow lint and five mocked recovery
scenarios. That review did not substitute for the captain-owned Rust tests.

## Fork-preservation acceptance hold, 20:18 UTC

A second, conflict-resolved merge preserves the fork's default `master` history
at `7655eca5e`; it is deliberately uncommitted. Do not abort the merge, discard
its index or reapply the preservation stash. No remote push, workflow activation,
Jcode channel update or shared-daemon restart has occurred. Fork publication
jobs remain restricted to the upstream repository. Imported CI action-version
upgrades still require compatibility and supply-chain review before publishing.

Captain's combined acceptance command passed offline binary `cargo check`,
workspace formatting, Bash compatibility and job contracts, then found a real
macOS test-fixture alias: `/var/folders` and `/private/var/folders` denote the
same location. Canonicalizing only the fixture's project path fixed all five
CWD tests without weakening equality assertions. PR-recovery scenarios, eight
warning-budget unit tests, three security-preflight tests and workflow actionlint
also passed. These checks do not imply full fork CI passed.

Independent Astra/high review caught 13 stale imported CI test exclusions and
new tool-description cap exemptions. Both were removed, preserving only the
two source-HEAD TUI exclusions. The tool-test budget increase was undone.
Two new runtime-command tests were adapted to the fork's boxed `Outbound::Reply`
via the existing helper, not relaxed. The repair worker passed both runtime
regressions, parameter-description caps, relevant library checks and formatting.
The main description-cap test still fails for `bash`, approximately 53 tokens
against its 20-token cap. Do not exempt it or remove safety semantics to hide
this failure. A separately admitted, bounded correction is still required.

A separate read-only Astra/high reviewer evaluated revision blobs in memory.
Pristine published `v0.97.0`, source HEAD, staged merge and working tree share
the same size/error-budget failing-path sets:

- 57 production-size files and 17 test-size files exceed their recorded budgets.
- 48 swallowed-error checks fail: 44 file checks and four aggregate checks.
  The total is 3,884 against 3,785, with `.ok()` 1,499/1,458,
  `let _` 1,418/1,388 and `unwrap_or_default` 967/939.
- Retained local privacy work adds 2 LOC in `agent/turn_execution.rs`, 20 in
  `tool/mod.rs`, 3 in `tui/ui_prepare.rs` and 3 in `openrouter_tests.rs` versus
  pristine v0.97.0. Those incremental additions are not upstream baseline debt.
- The pending fork merge adds no additional growth to those failing paths.
  Its swallowed-error budget changes relocate three existing `.ok()` counts,
  not a total increase. A four-line onboarding assertion budget increase still
  needs a no-relaxation fix or explicit approval.

The three checker scripts are unchanged across the compared revisions. Their
checks were executed and failed, not skipped. Even a baseline adjustment limited
to measured pristine-release debt relaxes verification and needs an explicit
operator exception. Do not run any checker's `--update`, suppress failures or
start an unrelated dozens-file refactor under this task's existing authority.
The single pending decision is whether to accept only the measured pristine
v0.97.0 ratchet debt while keeping incremental local changes under the existing
gates. That exception would not approve token-cap bypasses, stale test skips,
unreviewed CI upgrades or any secret/credential migration.

Final captain rerun completed at 20:19 UTC: runtime commands 2 passed, provider
175 passed/1 existing ignored, outbound sanitizer 8 passed, tool privacy 3 passed
and pinned todo 16 passed: **204 passing regressions**. This validates the scoped
repairs and retained behavior, not the failing ratchets or description cap.
The conflict-free merge and owned corrections are staged for safe resumption;
no source commit or remote activation was made while acceptance is blocked.

## Pristine-only ratchet exception receipt, 20:39 UTC

The operator's **20:21 UTC exception** admits only measured debt in published
v0.97.0, immutable commit **`94392e100dd05868172c9884944d97bd1eee5d8e`**.
The 20:29 scope extension includes `panic_budget.json` and the local outbound
sanitizer's panic removal. It does not admit incremental local growth, checker
changes, larger thresholds, new exemptions or test exclusions. No checker's
`--update` was run. Two bounded source-edit iterations completed this correction.

Immutable Git blobs were read in memory through the **unchanged checker
functions**, with only filesystem reads redirected to those blobs. The proposal
starts from the admitted staged baselines, restores onboarding's 1,717 ceiling,
and adds only pristine debt. Every file/pattern allowance is independently
asserted `<= max(old budget, pristine count)`, except the already documented
three `.ok()` relocation from `helpers.rs` to `helpers_clock.rs`. All six
production and three test extraction decreases remain subtracted, including
where pristine growth overlaps an extraction. The aggregate bounds are:

| Ratchet | Pristine failures | Admitted aggregate |
|---|---:|---|
| Production size | 57 files | Threshold unchanged at 1,200 LOC |
| Test size | 17 files | Threshold unchanged at 1,200 LOC |
| Swallowed errors | 44 files + 4 aggregates | 3,785 → 3,884; `.ok()` 1,499, `let _` 1,418, defaults 967 |
| Panic-prone usage | 12 files + total | 169 → **179**, independently measured, not the estimated 178 |

The resulting local panic count is **178**: `caps.get(0)` now withholds the
match as `[REDACTED_CARD]` if absent instead of panicking. No allowance was added
for `message/outbound.rs`. Other local growth fits through equivalent changes:
manual-result saves return their existing `Result` directly; the observer hook
moves verbatim to the existing privacy module; provider privacy tests are
registered as a sibling of the legacy tests; duplicate cache-boundary prose is
condensed without changing TUI code. Onboarding initializes its guaranteed
fixture directly and reads suggestions once, retaining the four-line stronger
assertion. Final LOC: turn execution **1,624**, tool registry **1,881/1,883**,
UI preparation **2,740**, legacy provider tests **3,728**, onboarding **1,717**.

Bash guidance now says `Bash. Search: agentgrep/rg. rg skips hidden/ignored
unless --hidden --no-ignore.` Its claim-based regression retains routing,
hidden/ignored semantics and both flags, and checks Windows guidance too.
Descriptions measure **20 tokens for Bash, 13 for Windows**. The shared cap test,
caps/exemptions, Bash execution implementation and destructive gate are unchanged.

Reproduction and evidence, all local and offline:

```sh
python3 ~/.jcode/scratch/astra-v97-gates-immutable.py --verify
python3 ~/.jcode/scratch/astra-v97-gates-preservation.py
/bin/bash ~/.jcode/scratch/astra-v97-gates-validate.sh
```

The immutable proof script SHA-256 is
`c69ba501e7fdf38f923a33fa31919ed1ecd1b16dc6c06f47932dbf740da7bf3d`.
It proves the bounds and retained extraction deltas against the tag and signed
source HEAD `e6562315b0f66ec799aa051e0cf9c268e5c6c7a3`. The preservation check
also verifies exact observer relocation, unchanged protected files, the stronger
onboarding assertion, allowed paths only and an unchanged staged merge/index.
The `git ls-files --stage` digest remains
`6f3a40ec2ba290b4fefc5911d21d35698492e87b4878e55ccd9be7ebbf5afcea`.

`~/.jcode/scratch/astra-v97-gates-validation-final.log` records all four ratchets,
workspace rustfmt, offline locked `cargo check -p jcode --bin jcode`, and **209
passing tests**: caps 2, Bash guidance 1, tool privacy 3, runtime commands 3,
outbound sanitizer 8, provider 175, pinned todo 16, onboarding 1. This covers
the retained 204-regression set; the provider suite retains its one existing
ignored test. No new skip or exemption was introduced.

An initial parallel pinned-todo invocation stalled and its owned test process
was stopped. The existing prescribed `--test-threads=1` invocation passed all
16 tests in 0.49s. The original stall remains recorded in
`astra-v97-gates-validation.log` and `astra-v97-gates-pinned-todo.log`; parallel
fixture safety is not claimed. Existing compiler/linker warnings remain visible.
This is not full CI or activation signoff. No staging, commit, workflow edit,
network install, configuration/auth change, publication or daemon reload occurred.
Independent review, captain rerun and separately owned CI security work remain.

## CI isolation correction receipt, 21:00 UTC

The scoped correction separates read-only release validation from clean hosted
publication, adds manual full CI dispatch, restricts SDK npm publication to
upstream, and checks the static actionlint asset digest before extraction. The
existing approved action versions are unchanged. Current fork guidance removes
the 13 stale exclusions from policy while retaining historical skip receipts.
No remote workflow, repository variable or permission was activated here.

Only the unused direct `rustls` line in `crates/jcode-base/Cargo.toml` and its
entry in that package's lockfile dependency list were removed. The global
rustls package/version, root AWS-LC feature and root provider installation are
untouched. An offline feature-tree check confirms standalone base still enables
AWS-LC through Azure auth/core, TypeSpec and reqwest 0.13.4. The separately
authorized `jcode-desktop-open` fix applies the Niri structs' exact cfg to their
`Deserialize` import, with no lint suppression or dependency change.

Local acceptance commands, with no downloads or installs:

```sh
python3 scripts/test_upstream_release_sync.py
actionlint -no-color
git diff --check
CARGO_NET_OFFLINE=true CARGO_BUILD_RUSTC_WRAPPER='' "$(rustup which cargo)" metadata --locked --offline --format-version 1 > /dev/null
CARGO_NET_OFFLINE=true cargo-machete
CARGO_NET_OFFLINE=true scripts/dev_cargo.sh check --locked --offline -p jcode-desktop-open --all-targets
CARGO_NET_OFFLINE=true "$(rustup which cargo)" tree --locked --offline -p jcode-base --no-default-features -e features -i rustls
/bin/bash scripts/test_dev_cargo_compat.sh
/bin/bash scripts/test_dev_cargo_jobs.sh
python3 scripts/test_dev_cargo_cwd.py
```

All passed in the second iteration: 10 proposal test methods (including the
five original recovery scenarios), all-workflow actionlint 1.7.12, locked offline
metadata, workspace machete, companion all-target check, both shell contracts
and all five CWD cases. The final iteration adds a real file-transport proposal
push proving a malicious validation pre-push hook cannot cross into publication.
The first iteration caught macOS Bash 3.2 ignoring `set -e` on bare `[[ ... ]]`
guards and an off-by-one fixture ancestor. Explicit fail-closed guards and the
correct fixture parent fixed those failures, without removing cases.

The staged merge/index remains unchanged at SHA-256
`6f3a40ec2ba290b4fefc5911d21d35698492e87b4878e55ccd9be7ebbf5afcea`.
The unrelated unstaged diff remains unchanged at SHA-256
`8b790be7d3823d4dca1d41e77d3c7bf60677c614d3b3f2d086523df7d44b57f6`.
This is not full CI, strict workspace Clippy, an authenticated GitHub smoke test,
independent final review, or captain acceptance. A base-ref race after the last
check still needs head/base review. No source staging, commit, push, install,
host configuration, credential change, build-channel update or reload occurred.

## Captain local acceptance, 21:19 UTC

Independent high-effort review approved the seven-file strict-Clippy corrections
and the isolated CI publication boundary. Captain strict workspace Clippy with
`--locked --all-targets --all-features -- -D warnings` passes, as do formatting,
the four immutable-release ratchets, ten proposal contract methods, actionlint,
eight warning-gate tests, both Bash contracts and five canonical-CWD tests.
No additional test skips, token exemptions or local metric allowance were added.

Captain retained-behavior checks pass: provider runtime 175 (one preexisting
ignore), outbound privacy 8, tool privacy 3, description caps 2, pinned todos 16,
Bash search guidance 1, app-core runtime 3, harness runtime 2, onboarding 1,
mock telemetry opt-out 2, SDK parity 3 and provider matrix 9. The Azure loopback
retry fixture previously failed with a server read WouldBlock and timeout. Its
exact isolated rerun and subsequent full serial provider suite both passed.
This is retained as a fixture-flake observation, not a timeout increase or skip.

The standalone mock telemetry build exposed production `tokio::select!` relying
on unrelated feature unification. An independent high-effort review approved
adding only `macros` to provider-core's existing regular Tokio feature list.
The locked standalone tests and subsequent strict workspace Clippy pass, with
no package version or lockfile update. Synthetic opt-out tests send no traffic.

The local secret-pattern and script-permission scans pass. `cargo-audit` is not
installed locally, so advisory verification and all-platform hosted CI are still
separate gates, not claimed green. Existing macOS linker/toolchain warnings are
retained. Actual signed commit, remote workflow activation/run IDs and rebuilt
runtime provenance belong in the dotfiles adoption handoff after those effects
are verified. Future published releases remain review-only proposals, never
automatic local installation, deployment or permission to merge.
