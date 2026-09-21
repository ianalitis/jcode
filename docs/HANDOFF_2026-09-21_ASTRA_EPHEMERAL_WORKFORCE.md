# Fresh Astra handoff: ephemeral Jcode + Pi workforce

Prepared 2026-09-21, closeout around 15:28 UTC. Read this before executing the
workforce plan. **This is a checkpoint, not a claim that the workforce is ready.**

## 1. Goal and operator direction

The operator wants Astra primarily for planning, thinking and delegation from
here, with bounded cheaper-model workers doing implementation. They asked to
continue, stabilize this session and prepare a comprehensive fresh-agent handoff.
Do not resume unlimited fanout. Start by auditing the partially implemented
self-compaction and Pi bridge, then produce one bounded next packet.

Desired outcome: low-friction natural-language kickoff, closed-set Jev/local
classification, deterministic route admission, short-lived Jcode/Pi workers,
automatic checkpoint/compaction/refresh, durable verified progress across stops,
strong containment and explicit integration gates. Optimize accepted work per
cost and operator attention, not worker count, context consumption or quota burn.

Requested references:
- https://www.youtube.com/watch?v=3b0U4_02bAE
- https://github.com/disler/self-compact-pi-agent
- Downloaded captions: `~/.jcode/scratch/selfcompact/transcript.en.vtt`.
- Cleaned auto-caption text: `~/.jcode/scratch/selfcompact/transcript.txt`.
- Reference clone: `~/.jcode/scratch/selfcompact/repo`, commit
  `576fe4abda021849f5cde5b6f5796467ffa4bcbd` (MIT).
- Digest: `docs/references/2026-09-21-self-compact-pi-agent.md`.
Auto-captions and the author's performance anecdotes are not validated benchmarks.
The reference's 10/20/30% defaults and our installed 40/60/70% are hypotheses,
not demonstrated optimal thresholds. Compaction and a fresh worker are different.

## 2. Freeze: checkout, runtime, authority and routes

- Repository: `/Users/ianalitis/.jcode/source/jcode`.
- Branch: `jcode/ci-format-baseline`.
- Stabilization commit: `44c33ac59`, parent `4cb2923e2`.
- Main checkout clean after stabilization; this handoff adds a docs commit.
- No build/reload/push in this closeout. Last verified running binary from the
  prior session is `v0.86.234-dev (a61ab0927)`. Do not assume it serves new tools.
  Recheck the actual shared daemon, launcher and selected binary before runtime tests.
- `origin/master` was `2a4edaa02` (v0.86.0 plus one docs commit). v0.86 was
  already merged via `ed35ce309`. Recheck current upstream instead of merging
  the entire history again. Local changes since then are not automatically upstream.
- Pi reports 0.86.1. Its install/config files are outside this Git repository.
- Repo lease was free on closeout entry. Acquired as `astra-handoff-closeout`
  for stabilization and documentation, to be released after the handoff commit.
  **Lease uses git-common-dir and covers ALL linked worktrees.** Earlier claims
  that it applied only to the main checkout were wrong. Lack of a hook does not
  authorize ignoring it. Never impersonate another holder or force-release it.
- Global/project policy still controls installs, provider/auth changes, pushes,
  destruction and containment. Old task-specific approvals are not blanket
  approval to delete future worktrees or enable paid traffic. Preserve all
  existing branches/worktrees/stashes. No cleanup is necessary for this handoff.
- Worker tool restrictions remain real even if bash could reach an omitted
  tool via raw sockets/CLI. Plain-text worker reports are the supported return.

Routing for the fresh session:
1. Pin captain/planning to `openai-oauth:gpt-6-astra` as requested. Do not silently
   change an existing treatment. Sol/high remains appropriate for security and
   core runtime review. Cheap worker lanes remain provider-qualified Go routes.
2. Read fresh `swarm list_models` and `jcode usage --json` before dispatch. Pin
   model, effort, tools, scope, deadline and stop conditions once per packet.
3. Preserve >=20% headroom in every applicable exposed subscription window.
   Missing/stale windows are unknown. Go windows are not exposed to Jcode.
   Wall-clock peak hours alone do not establish price, capacity or quality.
4. The operator reports an OpenAI $100/month plan. Entitlements and current
   availability must be verified, not inferred from the price/name.
5. Metered OpenRouter dispatch remains held behind real admission/spend controls.
   Required OpenAI/Anthropic router exclusions stay in place. Free models are
   not automatically ZDR. Verify endpoint, retention, collection, routing and
   account controls for each candidate. Jev remains public/synthetic unless
   separate policy allows otherwise. Never send private packets for classification
   because their paths were merely labelled public by a model.

## 3. Delivered work and exact state by phase

### Earlier base-suite and render work

- Stage A review `3f622fb9b`; diagnostic TestEnvGuard/background-write fixes
  `450702133`, `c4fb31fe4`; picker-family isolation `3e410d7d0`; per-thread test
  frame metrics `62008c4ee`; prior daemon reload at `a61ab0927`.
- PR https://github.com/1jehuang/jcode/pull/1360 from
  `fork/pr/base-suite-env-guard` addresses the original 11 #1358 failures.
  Reproduction disproved the original race diagnosis: all 11 also failed serially.
  Causes included obsolete assertions, missing config fingerprint key, macOS
  SID query portability and cross-process pmset assertion matching.
  Original 11 absent in three parallel/one serial post-fix runs, but other
  failures remained. Do NOT describe the entire suite as green.
- PR and Phase 3 documentation have overprecise counts/causal language in places.
  Re-audit evidence before claiming all remaining failures reproduced on base.
  Do not publish corrections without checking the existing approval scope.

### Workforce plan and reference

`94a331dd8`: `docs/plans/2026-09-21_EPHEMERAL_WORKFORCE_PLAN.md` plus digest.
It is a draft roadmap, NOT an execution-authorizing specification. See section 5
below for corrections that supersede unsafe or premature recommendations.

### 5a/5b/10b: Pi self-compaction and prompts

Installed under `~/.pi/agent/extensions/self-compact/` with MIT license,
40% notice, 60% warning, 10% buffer. Registered explicitly in settings.json.
Prompts resolve from `~/.pi/agent/.pi/self-compact/`:
`USER_PROMPT_SOFT_SELF_COMPACT.md`, `USER_PROMPT_WARNING_SELF_COMPACT.md`,
`USER_PROMPT_COMPACTION_MESSAGE.md`.
The original copied prompt files nested inside the extension directory are not
runtime-discovered. Avoid assuming those are authoritative.

Jcode summary twin: `~/.jcode/prompts/compaction/SELF_COMPACT_SUMMARY.md`.
**No code currently loads this file into Jcode's summary path.**

`~/.pi/agent/skills/jcode-worker/SKILL.md` added; Pi default thinking changed
low -> medium. Existing extensions/skills preserved. Per-packet authority and
iteration caps override this skill's generic wording. Skill discovery did not
show i-have-adhd despite the configured path; this remains a configuration audit
item, not proof all shared skills load correctly.

Worker unit report: reference clone 30 unit tests pass. Prompt paths verified
via Pi info/RPC. Requested fake `ping` demo exited 0 but produced a hardcoded
summary and NO self_compact call or compaction entry. Thus neither Pi note
round-trip nor real post-compaction continuation has been demonstrated.
Evidence: `~/.jcode/scratch/packet5b-validation.doYJ3k/receipt.txt`.

### 5c: Jcode self_compact / view_context, PARTIAL implementation

Commits `1f4a2e847`, `33716b1c1`, `4cb2923e2`; stabilization `44c33ac59`.
- `tool/self_compact.rs`: validates note <=24000 BYTES, process-global
  session-keyed pending-note map, tool output marker; usage from manager plus
  persisted session messages. This is not guaranteed current in-memory usage.
- `agent/compaction.rs`: next completion poll drains map, calls manual compaction,
  keeps pending note on Agent, delivers prefixed user message on a compaction event.
- Worker originally omitted runtime drain entirely. Captain added it and one
  regression test. Existing six self_compact tests now pass.
- Note includes `[self_compact note_to_self]\n` prefix; content after prefix is
  preserved. Schema maxLength counts characters while implementation counts bytes:
  document/fix the mismatch and add Unicode tests before acceptance.
- No durable pending-note persistence/restart recovery, forced tool lock,
  transient notice/warning, automatic resumed model turn, or exactly-once delivery
  across crashes. No proven all-executor/headless integration. In-memory state
  is not a durable checkpoint and an arbitrary compaction event may be insufficient
  correlation with the requested handoff. Failure/retry and multiple requests
  require review. Do not enable this as unattended production workflow yet.

### 6a: classifier prototype

`0902fdab7`, `crates/jcode-plan/src/intake.rs`, intake tests and 25 synthetic
fixtures. Reported agreement: task 24/25, judgment 20/25, containment 25/25.
This is only one small self-authored fixture test, NOT a safety/admission gate or
held-out evidence. No Jev call or spawn wiring implemented. Secret-path detection
must never authorize sending secrets to a model, even if it recommends sandbox.

### 7a/7c/7d: routing documents and snapshot

- `46e3c6f09`: `docs/ROUTING_MEASUREMENT_CONTRACT.md`, 6 MISSING fields.
  Phase 4 remains a DRAFT pending quantitative thresholds, monotonic wall-time
  boundary and independent review. Earlier message calling it closed was wrong.
- Dotfiles `a9b1c14ad85b6f5ca73350c7381b418a29e8c07e`: adds lane doctrine to
  `~/dotfiles/policy/providers.md`. No generated-surface regeneration performed.
  Read canonical policy before editing; additive prose is not routing enforcement.
- `7dad1d775`: `scripts/lane-snapshot.sh` and fixture test. Advisory read-only
  display, not automated routing. Review stale/missing windows and OpenRouter
  credit-unit handling. Its unconditional Go recommendation is NOT admission.
  Never convert a usage percentage into dollars or tokens without schema evidence.

### 8a: attempt worktree helper

`f92c14169`: `scripts/attempt-worktree.sh`, `scripts/attempt_worktree_test.sh`.
Tests passed previously. Worktrees isolate Git state, NOT host reads, credentials,
network or effects. Discard uses force removal/branch deletion behind --yes:
never use without fresh target-specific operator approval. It does not replace
repo-wide lease coordination. No VM/container/sandbox profile has been approved
or proven. Tool-enabled Pi executor remains blocked by containment requirements.

### 9a/9b: interop contract and local bridge prototype

`1b3c764f8`: `docs/JCODE_PI_INTEROP.md`, 5 MISSING bindings and open ordering/
identity/steering questions. Generic Pi tool_result can expose the saved note.

Bridge installed and registered by frog (not source-controlled in this repo):
`~/.pi/agent/extensions/jcode-bridge/{jcode-bridge.ts,bridge.ts,client.ts,wire.ts,README.md}`
and `test/bridge.test.mjs`. Requires JCODE_SOCKET, JCODE_SESSION_ID and
JCODE_ATTEMPT_ID, otherwise inert. Emits comm_report envelopes containing note
JSON, parses attempt-targeted steer frames, guards pending/failed compaction.

Captain reran `node --test ~/.pi/agent/extensions/jcode-bridge/test/bridge.test.mjs`:
8/8 pass. Directory-form command from worker report failed locally with
MODULE_NOT_FOUND. These are fake event/socket tests only. No actual daemon
registration, authenticated identity binding, inbound routing or completed Pi
worker lifecycle proven. Worker-generated terminal prose is NOT the authoritative
harness receipt. Keep `jcode-executor-pi` no-tools until containment and complete
wire validation. Do not invent SDK events from stale bundled docs.

### 10a: local config changed earlier, not acceptance-tested

`~/.jcode/config.toml`: compaction reactive -> proactive,
memory_sidecar_enabled false -> true, concurrency 3 -> 4.
Backup: `~/.jcode/config.toml.bak-2026-09-21`.
These changes were made before establishing all route/privacy/runtime implications.
Review sidecar egress/model/entitlement and source-of-truth configuration before
relying on it. Neither loading the new values nor automatic private memory
processing was verified. Do not blindly restore the whole backup over later edits.

### 10d: ledger, billing-route review and failed implementation packet

`bb252f2b8` records Sol review of `jcode/auth-preserve-billing-route`:
- `387828e79`: port route-preservation behavior by hand, not merged.
- `6490e5263`, `5d6acb298`, `7002c8444`, `11be474fe`: reported superseded.
- `ce207d443` paid-request CLI: blocked, caller-created fresh admissions permit
  uncapped aggregate spend. No wholesale branch merge or recommended cherry-picks.

Crab's implementation packet produced NO edits/commit in
`~/.jcode/source/worktrees/attempt-billing-route` (clean at `bb252f2b8`). Its final
report drifted into Cerebras descriptors/login behavior instead of preserving
provider/model/billing route on refresh. Reject that report as implementation
and do not resume it unchanged. Fresh Sol/high packet should write precise red
regressions before a cheaper builder changes code. Security work needs adequate
judgment, not automatic medium-effort delegation.

Privacy branch `c81735fdf` remains unintegrated. Prior claims of a straight
cherry-pick were unsubstantiated; ledger says mostly relanded, six-file conflict.
Inspect only the unique remainder. Other divergent branches remain in the ledger.

## 4. Verification receipts and current failures

All logs below are under `/Users/ianalitis/.jcode/scratch/`:

| Check during closeout | Result | Artifact |
|---|---|---|
| `scripts/bounded.sh 600 bash scripts/check_guardrails.sh` | exit 0, all gates pass; cargo-machete skipped because absent | handoff-guardrails.log |
| `cargo test -p jcode-app-core --lib -- self_compact tool_descriptions tool_parameter_descriptions` | 8 pass | handoff-app-tests.log |
| `cargo test -p jcode-plan --lib` | 100 pass | handoff-plan-tests.log |
| `cargo check -p jcode-tui --all-targets` | pass | handoff-tui-check.log |
| `cargo test -p jcode-tui --lib test_changelog_overlay_repeated_renders_are_stable` | 1 pass | handoff-tui-test.log |
| `cargo test -p jcode-app-core --lib` | **FAIL: 1528 pass, 1 fail, 31 ignored** | handoff-app-full.log |
| isolated `connect_socket_preserves_refused_socket_path` | 1 pass | handoff-socket-recheck.log |
| explicit Pi bridge test file | 8 pass | handoff-pi-bridge-file-tests.log |
| Pi bridge test directory argument | FAIL, MODULE_NOT_FOUND | handoff-pi-bridge-tests.log |

App-core failure: `server::socket_tests::connect_socket_preserves_refused_socket_path`
(socket_tests.rs:124) expected connection failure after dropping listener, but
UnixStream connected to its temporary socket. Isolated pass is not proof the
parallel failure is fixed or pre-existing. Preserve evidence; do not suppress it.
Earlier full run failed only tool-description limits, since repaired.

Stabilization `44c33ac59` fixed collapsible-if Clippy, placed frame-metrics child
module at explicit `#[path = "ui_frame_metrics/state.rs"]`, moved the new test to
partition 02, grouped context-tool registration, moved usage accessors into
agent/provider.rs, and moved jcode-plan's existing inline tests into src/tests.rs.
No ratchet baselines raised. No full TUI suite or new-binary runtime acceptance.

## 5. Criticism and guardrails for the fresh plan

The big roadmap overpromises in several places. Apply these corrections first:
- Refresh at measured task/checkpoint/context boundaries, not arbitrary hourly
  disposal. Include cache rewarming, summary cost and lost-work rate in economics.
- A summary or worker note cannot grant authority, mark untested work proven,
  replace admission or become a trusted user instruction. Preserve provenance.
- A forced compaction phase must preserve cancellation, evidence capture and
  safe recovery. Avoid an unrecoverable loop that only permits a failing tool.
- Model/route/effort changes need a newly admitted attempt, never silent mid-task
  quota fallback. A refreshed worker needs identity and ledger reconciliation.
- Cheap workers must not choose or implement security policy independently.
- Pi bridge must speak one verified protocol on the right socket with bound
  session/attempt identities. Self-compaction must not settle the entire job.
- Worktrees do not enforce containment. Test denied reads, writes, subprocesses,
  inherited secrets, outbound traffic and socket access, not just one denied write.
- Do not expose a generic paid CLI, enable sidecars or increase parallelism merely
  because they are installed/native features. Policy and real admission come first.
- The current classifier and lane snapshot do not prove automatic tool/harness/
  skill/plugin selection. Reuse existing types and routes before new abstractions.
- A Rust compile, fake summary, schema test or successful file edit is not an
  end-to-end result. Keep claims scoped to the exercised boundary.

## 6. Next packets, ordered and bounded

Astra captain should plan these; do not start them all at once.

1. **Read-only safety/integration review (Sol/high)**: self_compact request/poll
   lifecycle and Pi bridge. Verify actual Pi 0.86.1 API, idle/resume timing,
   persistence, identity, egress and failure semantics. Return concrete red tests,
   no code changes. Read Sections 3 and 5 above before reading the older plan.
2. **Close one native self_compact vertical slice**: scripted real agent execution
   invokes tool, persists checkpoint, compacts, resumes without operator input,
   writes a required file exactly once. Restart, failed compact, Unicode, duplicate
   requests, cancellation, concurrent sessions and no-tools policy get regressions.
   Then bound a cheap implementation packet around the reviewed seam.
3. **Pi deterministic lifecycle test**: fake provider must actually invoke
   self_compact, not classify every ping as a summary. Run installed extension and
   custom prompts, verify exact note and post-compaction action. No live spend needed.
4. **Billing-route preservation red tests** from Sol review, explicitly excluding
   Cerebras feature work and the paid-request bridge. Keep one writer per repo.
5. **Controlled config audit**: sidecar/recall privacy, feature eligibility,
   generated policy source sync, skills actually loaded, containment decision and
   quota freshness. Operator-approved effects remain separate from code review.
6. **Routing/interop integration** only after receipts and authority binding are
   verified. Choose numeric promotion criteria from measurements, not invented
   percentages. Compare supported free-ZDR routes by verified endpoint evidence.
7. Continue branch-ledger unique remainders iteratively with independent reviews.

Use seven-section packets: goal/non-goals, acceptance, freeze, gate/rollback,
budget/stop, criticism, handoff. Exact permitted files and commands, finite retry
budget, explicit tool allowlist, one recorded writer lease, captain diff review.
No unbounded cleanup, migrations, new dispatcher/ledger or opportunistic paid calls.

## 7. Handoff inventory and first action

All session-owned workers were quiesced using swarm cleanup at closeout. No
running implementation should remain from owl/crab/frog. Preserve their artifacts.
Known remaining worktrees at closeout: main, attempt-billing-route,
base-suite-env-guard, data-class-admission, focus-runtime-safe,
privacy-no-collection, route-receipts. Never delete them as automatic housekeeping.

Start the fresh session on `openai-oauth:gpt-6-astra` with this instruction:

> Read docs/HANDOFF_2026-09-21_ASTRA_EPHEMERAL_WORKFORCE.md first. You are the
> Astra captain for planning and bounded delegation to cheaper workers. Inspect
> HEAD/index/diff, repo lease and native usage windows, then review the current
> self-compaction/Pi bridge gaps before any write or new fanout. Preserve all
> worktrees and unrelated work. Do not treat the prior plan as runtime acceptance.

**FIRST ACTION: inspect `git status --short`, lease status and this handoff,
then define the read-only lifecycle review packet.**
