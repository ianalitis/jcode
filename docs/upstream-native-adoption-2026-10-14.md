# Native Jcode adoption directives for ~/dotfiles agents

Date: 2026-10-14. Target: upstream **v0.97.0**, commit `94392e100`.
A subsequent fetch also found v0.98.0. It is outside this ten-release audit and
must receive a separate delta review, not an unreviewed latest-version bump.

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
on `ianalitis/jcode`. It polls the latest stable upstream release every two
hours, makes an ordinary merge preserving fork history, validates the merged
source, and proposes a per-release PR. Conflicts fail closed. It never force
pushes, auto-merges, installs, deploys, or edits this Mac's configuration.

Activation requires separate approval to push the workflow to the fork's
canonical default branch, enable Actions and allow Actions to create PRs.
Protect the target branch, require independent review plus existing fork CI,
and keep publish/release jobs guarded to the upstream repository unless the
operator separately approves fork publishing. The default `GITHUB_TOKEN`
does not trigger ordinary downstream PR workflows, so validation runs inside
the sync job. Full CI must be manually dispatched or separately enabled on
proposal branches before merging. Do not introduce a PAT to work around this
without a credentials decision. The workflow's checks are a smoke gate, not
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
proposals. `python3 scripts/test_upstream_release_sync.py` exercises new push,
missing-PR recovery, duplicate prevention, divergent branch and network failure
with local command stubs only, not an authenticated GitHub acceptance run.

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

The v0.97.0 merge is intentionally **uncommitted** because the configured SSH
signer waited for interactive unlock. Signing was not disabled. The source
integration preserves local content-filter hints, equivalent hidden-todo cache
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
