# Fresh-session handoff: harness reliability, OpenCode Go and model economy

Prepared 2026-09-20, approximately 19:11 UTC. This is a local handoff, not an upstream publication or authorization to transmit repository/private context to a new provider.

## Start here

1. Read this file, `AGENTS.md`, `CONTRIBUTING.md`, `docs/plans/TOKEN_ECONOMY_PLAN.md`, and the current diff. Preserve all dirty work. Do not reset, clean, checkout another branch, or assume worker tests equal captain acceptance.
2. Confirm old writers are quiescent before editing their files. See ownership below. The running daemon still has the OLD swarm-stop defect: do not use `swarm stop` as proof a worker stopped.
3. Review and integrate the two pending fixes separately, with the listed tests and an independent race review of the final stop implementation. Commit scoped verified work, not the whole dirty tree.
4. Build with coordinated `selfdev build target=tui`, then advise the operator that a runtime reload is needed. Follow selfdev reload guidance and verify actual runtime identity. No reload has occurred in this work.
5. Research and verify the new OpenCode Go subscription route and exact model identifier before changing routing. The user calls it “DeepSeek Flash 4.1”; do not silently equate it with the earlier OpenRouter `deepseek/deepseek-v4-flash-0731` route.

The operator has repeatedly authorized routine implementation, tests, review, and integration without permission checkpoints. Interrupt only for real scope/security/cost decisions, blockers requiring their action, or deployment approval. Do not keep asking to continue.

## New operator direction, verbatim substance

User reports purchasing OpenCode Go and wants heavy iterative work through its included DeepSeek Flash 4.1 usage. The fresh session is intended to use that route. They report cancelling Claude, which ends later this week, and reducing ChatGPT to the $100/month Pro plan. These are user-reported subscription changes, not independently verified product/account details. Do not reuse the earlier quota/plan assumptions as current fact.

Desired outcome: comprehensive model-selection and routing updates across Jcode and `~/dotfiles`, so complementary models are employed automatically, efficiently and reliably. Modular, maintainable, measurement-backed design, with candidate discovery and pricing refresh as models change. Avoid a sprawling second orchestrator or parallel policy/ledger implementation.

Priority distinctions:
- High priority: avoid use of user/project data for model training.
- Prefer ZDR, but reasonable explicit exceptions are possible. Do not call a route ZDR without evidence, or silently treat unspecified retention as zero.
- Preference to avoid foreign-hosted models where possible is lower priority. Hosting jurisdiction, operator/vendor nationality, training, retention and subprocessors are separate questions.
- Maximize included Go value where admitted, use the remaining ChatGPT subscription deliberately, and retain bounded metered OpenRouter as complementary capacity. Included does not mean unlimited, route-compatible, private, or qualified for all tasks.
- Research actual Go policies and compatibility next session. Determine whether its entitlement is supported through Jcode's provider API, only through OpenCode, or under other documented restrictions. Do not scrape/reuse credentials or impersonate a different client to bypass product restrictions.

No auth/provider settings, subscriptions, account limits or installs were changed by this agent. The user changed subscriptions and OpenRouter limits themselves.

## Repository and runtime

- Repository: `/Users/ianalitis/.jcode/source/jcode`
- Branch: `jcode/ci-format-baseline`
- Last source commit at handoff preparation: `e066b376e`.
- Active session originally reported `v0.84.229-dev (d027491f6)`.
- TUI/CLI source work, not desktop.
- Upstream remote is named `origin` (`1jehuang/jcode`), personal remote `fork` (`ianalitis/jcode`), default upstream branch `master`. Do not rename.
- No pushes, installs, deployments, branch/worktree creation, or upstream publication authorized. Contribution preparation is allowed. Current CONTRIBUTING guidance requires a linked issue for PRs; recheck before publishing and obtain effect approval.
- `~/dotfiles/policy/upstream.md` and `policy/providers.md` are canonical harness policies. Generated `~/AGENTS.md` and generated swarm policy must not be edited directly. The new user direction requires a deliberate canonical-policy update, not silent drift or a permanent Sol-only assumption.
- Old runtime/channel identity was inconsistent: current channel, shared-server symlink, version marker and live daemon can differ. `selfdev status` alone reads a marker and is not proof of the running binary. Resolve symlinks and verify the actual daemon before runtime claims.

## Completed commits in this work

- `3982c4774 fix(build): remove stale package profile overrides`: removed 12 obsolete overrides for cosmic-text/swash/unicode-linebreak/yazi across dev/selfdev/test. Full Cargo metadata unchanged. Selfdev build passed. No linker suppression.
- `020e51b51 docs: reconcile harness economy and local inference assessment`: current amendments in TOKEN_ECONOMY_PLAN.
- `7d6ffbca4 docs: record queued worker surviving swarm stop`: sanitized local incident receipt.
- `e066b376e test(swarm): capture worker context and cache invariants`: test-only E1 baseline and honest measurement receipt. No production savings claimed.

Earlier root checks: fingerprint 3/3, base prompt 45/45, E1 5/5, spawn narrowing 4/4. Scoped formatting and size gates passed at that commit. Not full-workspace acceptance.

## Pending dirty track A: worker stop lifecycle

Owner: llama, `session_llama_1789928743592_c1d4b72d252ce55e`, frozen `openai-oauth:gpt-5.6-sol`, high. Writer asked at 19:09 UTC to reach safe checkpoint, stop further edits/tasks and report quiescence. Do not infer quiescence merely from swarm membership removal. Mouse performed independent review, no edits: `session_mouse_1789928768397_cbb5b6a5cadded50`.

Files in latest observed dirty snapshot:
- `crates/jcode-app-core/src/agent.rs`
- `crates/jcode-app-core/src/agent/interrupts.rs`
- `crates/jcode-app-core/src/server.rs`
- `crates/jcode-app-core/src/server/comm_session.rs`
- `crates/jcode-app-core/src/server/comm_session_e1_tests.rs`
- `crates/jcode-app-core/src/server/comm_session_tests.rs`
- new `crates/jcode-app-core/src/server/comm_session_stop_tests.rs`
- `crates/jcode-app-core/src/server/live_turn.rs`
- `crates/jcode-app-core/src/server/state.rs`
- `crates/jcode-app-core/src/server/swarm_stop_ownership.rs`
- `docs/upstream-feedback/2026-09-20-swarm-stop-queued-worker.md`

Read current `git status` rather than assuming this list is immutable. Intermediate changes to other lifecycle files were removed by the writer. Do not revive abandoned intermediate versions.

Original incident: a completed scout was sent a wake message, then `swarm stop` returned success. It continued patching/testing, became unaddressable, and nearly overlapped a replacement writer. Original scout eventually reached terminal response before ownership transfer. Existing handler removed routing/membership and only `try_lock`ed Agent to mark closed, without cancelling a busy turn.

Early candidate passed 42 tests but was rejected by captain/independent review for:
1. A reserved live wake before cancellation registration could miss the one-shot cancellation.
2. A live sender holding a cloned queue or persisted fallback could append after final clear/Done.
3. Timeout removed membership and made retry unknown, or could cache timeout permanently.
4. Busy Agent skipped mark_closed, cleanup/hooks; polling without retaining terminal guard was not atomic quiescence.
5. Production then test-size ratchets failed during intermediate attempts. Never update baselines or suppress failures to accept it.

Latest writer-documented design (NOT yet final captain acceptance):
- Handler moved coherently to existing `swarm_stop_ownership.rs`.
- Mark member `stopping`; gate interrupt producers in existing queue lifecycle, wait for admitted producers, repeatedly cancel late-registered turns.
- Acquire and retain terminal Agent guard before verified live/persisted clear, mark closed, cleanup/removal and Done.
- Timeout/final-clear failure leaves target and delivery gate intact/resolvable for retry. Uses existing no-final-replay mutation path.
- Late reserved wake rejected; startup recovery excludes stopping members; direct closed-Agent queueing rejected.
- Queue registration is explicit resume boundary, must not reopen during in-flight stop. Lifecycle gate follows session rename.
- Guarantee is LOCAL turn/tool-dispatch quiescence, not undoing external effects started before cancellation.

Latest writer-reported tests (verify artifacts and rerun final diff):
- Pre-fix regression failed 0/1: `939779b17o`.
- Stop module 4/4: `13115290zq`.
- Parallel comm-session module 45/45: `29438400rr`.
- Parallel live-turn action suite 12/12: `283398oyhb`.
- Session control/queue lifecycle suite 21/21: `289858xgiw`.
- Seven structural guards passed, no baseline updates. At that snapshot comm_session 1445 vs baseline1620; parent test1057 and stop test560, under1200.

Tests must specifically cover real blocked provider cancellation; reserved wake before turn registration; producer barriers across clear for live/persisted/direct queues; uncooperative lock timeout followed by successful retry; unrelated sessions and ownership. Independently review new gate lifetime, producer completion/drop, lock ordering, restart/resume, direct Agent paths and failure cleanup. This is a concurrency/security-sensitive diff, not a cheap mechanical task.

E1 fixture isolation correction included: its environment guard now invalidates the reloadable config cache after changing/restoring environment under the existing test lock. These failures were OUR recently added fixture defect, not legitimately dismissed as unrelated pre-existing flakiness.

## Pending dirty track B: portable host-wide build gate

Owner: wolf, `session_wolf_1789926902554_308d6599db31a9bf`, frozen Sol/high. Terminal final response observed 18:55:14 UTC; no new implementation packet after that. Asked 19:09 to confirm safe handoff, no new work.

Exclusive files:
- `scripts/dev_cargo.sh`
- new `scripts/test_dev_cargo_gate.py`

Implementation uses Python stdlib `fcntl.flock` when native `flock` is absent, on a shell-owned inherited FD (currently fixed FD9). No new installed dependency, no stale directory-lock scheme. Existing native path retained. Review FD inheritance/collision, nested guard, child surviving parent signal, errors, shell portability, timeout notices and wait metrics. Missing both flock and python currently logs unavailable and continues as before; assess against intended build policy, don't falsely call it fail-closed.

Worker report:
- Gate suite 5 passed, 1 native-flock interoperability case skipped because Mac has no `flock(1)`.
- Isolation suite 11/11 passed.
- CWD routing, syntax, ShellCheck, Ruff, AST parse, diff checks passed.
- No real builds, no commits, no Rust edits or linker-warning changes.

Captain has inspected diff but has NOT independently rerun this track's tests or accepted it. Native interoperability must run on an available existing CI/host, not by installing flock without approval.

Suggested commands:
```sh
python3 scripts/test_dev_cargo_gate.py
python3 scripts/test_dev_cargo_isolation.py
bash scripts/test_dev_cargo_cwd.sh
bash -n scripts/dev_cargo.sh
# Only if already installed:
shellcheck scripts/dev_cargo.sh
ruff check scripts/test_dev_cargo_gate.py
git diff --check
```

Apple `__eh_frame` >16MiB compact-unwind warning remains. Treat as measured unwind-size/performance debt, not proven correctness failure. Keep visible. Investigate matched binary section sizes/unwind behavior before changing profile/linker options. Incremental sccache skip, selected nightly frontend threads and memory-aware job cap are intentional status messages, not build failures.

## E1 economy findings and unfinished work

- Workers already receive independent sessions and packet plus report boilerplate, not captain transcript.
- Ancestor AGENTS resolution already exists: global then repo-root-to-cwd, captured policy, provenance, bounds. Older dotfiles prose saying it doesn't is stale. Do not rewrite traversal.
- Tools sorted and locked after first request; spawn allowlist intersects config. Stable/dynamic prompt split exists.
- Provider payload fingerprints already log canonical hashes/byte counts/prefix comparisons without raw text. Avoid a new unused receipt abstraction.
- Unknown cache metrics remain Option/None. Do not convert unsupported cache metrics to zero.
- E1 synthetic 3 packets x2 repeats: inherited median47,955 vs relevant-tool median6,095 serialized bytes at HIGH-LEVEL PROVIDER boundary. Existing capability measurement, not adapter wire bytes/cache-hit evidence or new production gain.
- Workers still load full base prompt/skill catalog/captured policy, memory generally enabled. Don't disable memory blindly; prior regression #729 exists.
- Remaining: durable output continuation references (generic caps currently truncate/refuse); persisted spawn allowlist/schema/policy/envelope on disk resume; durable compact accepted-task/cache receipt; real workload quality/review-cost/cache evaluation. Separate schema/retention decisions, no silent data loss.

## OpenRouter status and admission review

User raised workspace daily cap to $20 and removed key daily cap. Fresh native usage at18:29 showed key-limit entry absent, $8.72 today, $92.28 balance. Workspace cap is user-reported, not exposed by that usage result. Refresh before any spend, preserve required reserve, account for concurrent dotfiles activity. This is NOT authorization to consume entire balance. Prior '$1.28 remaining' report is explicitly stale and wrong.

No live metered workers or inference were launched. Current metered repository dispatch remains fail-closed. New Go subscription is a DIFFERENT route: assess its own terms/privacy/bounds, do not blindly reuse either OpenRouter's restrictions or assumptions.

Existing OpenRouter pieces: common positive-budget/default-private/public-or-synthetic admission, reservations/deadlines, durable LocalLedger, FrozenAttempt single-send transport (HTTPS/no proxy/no redirects/no retries), hash/equality guard, input/output byte limits, conservative ambiguous settlement. `run_frozen_attempt` currently has only offline/test callers, not approved production integration.

Remaining blockers include full request provenance, caller-supplied expected JSON not semantic security proof, backend/fallback/privacy fields not frozen, token/unit-price worst-case proof absent, post-spend settlement not preventive, no backend receipt validation, extra_body override hazards, retrying generic swarm send, plan ledger persistence/settlement, tools absent from assign-next envelope, dedup omitting security envelope, effort failure only warns. Avoid asserting a data_class label authorizes inherited AGENTS/memory/tool results.

Smallest proposed J5-F1 packet, preserved in `~/.jcode/scratch/j5-f1-remainder.md`:
- Synthetic no-tool one-shot only; keep metered repository CommSpawn refused.
- Require OutboundPacket; fixed code-owned minimal system; one synthetic message; no AGENTS/memory/transcript/images/reminders/tool results or working-directory context.
- Freeze exact model/backend/endpoint/effort, no fallback, ZDR/data_collection deny/required params where contract supports them.
- Integer input/output price ceilings, conservative proven input token bound, output token/byte cap, worst-case cost <= reservation before send.
- Construct and validate final JSON internally after all merges, one constrained send, durable ledger, served identity/privacy evidence, mismatch/unknown cost ambiguous.
- Offline tests: privacy/provenance refusal before reserve, body override attempts, caps/overflow, single send on retryable response, backend mismatch, durable duplicate refusal and repository-spawn refusal preserved.

Reassess this packet against the newly requested Go architecture before building duplicate paths. Reuse proven FrozenAttempt/OutboundPacket/ledger primitives where appropriate; no parallel dispatcher merely for a new subscription.

## Next-session routing research and implementation plan

### R0. Verify facts, route availability and privacy

Use official current sources with dated citations. Verify exact model IDs and versions, OpenCode Go entitlement/limits/reset windows, supported API/auth surfaces, Jcode adapter compatibility, caching semantics and request caps. Confirm reported ChatGPT and Claude changes without altering accounts. Inspect native route discovery narrowly; `swarm list_models` returned ~69k tokens and was withheld. Do not dump huge model catalogs.

Build a small route matrix: route/account/product, exact model/backend, included vs metered, capabilities/tool/schema support, context/output limits, cache policy and cache pricing, retention duration, training use and opt-out scope, ZDR availability/eligibility, logging/abuse exceptions, subprocessors and hosting jurisdiction, evidence URL/date and unknowns. Distinguish provider vs inference backend vs intermediary obligations. Claims by model weights authors do not establish hosting/privacy policy.

Privacy routing should default conservatively when evidence is missing. User permits reasonable exceptions, not unrestricted private-data transmission. Define explicit task/data-class scoped exceptions with rationale and expiry. Do not weaken no-training requirements to force a cheap route. Keep credentials, secrets and protected application reads out of model packets.

### R1. Update canonical policy, not generated copies

Coordinate `~/dotfiles` agent ownership. Update canonical policy/provider definitions and generate existing surfaces through repository commands. Current generated defaults pin Terra/Sol and disable some lanes; record new explicit policy decisions rather than silently overriding them. Route changes are task-local, frozen per attempt. No silent model/account/provider fallback on error, quota or refusal. Do not force the next session onto an unverified guessed alias. If selected Go route unavailable, clearly report it rather than substitute paid OpenRouter.

### R2. Objective cheap-model admission

Start with small checkable extraction/classification, fixture proposals and low-risk patches. No authority to choose successors, route risk or widen effects. Measure correctness, abstention/refusal, strict-schema behavior, tool execution reliability, timeouts/retries, prompt injection resistance, latency, peak context, cache hit/miss and accepted-task cost including captain review/rework. Use paired fixed packets and deterministic acceptance. Include production-representative tasks, not only toy benchmarks. Preserve context/tool prefix stability within frozen sessions and avoid gratuitous compaction/provider switching.

Advance Go worker scope only after evidence. Security/concurrency lifecycle review needs appropriate review quality irrespective of subscription price. Do not interpret included tokens as a reason for unlimited iterations or giant packets. Existing long-lived workers accumulated large prompt histories; prefer concise fresh packets once ownership safely transfers.

### R3. Modular automation without unchecked self-modification

Reuse existing route/admission/attempt/receipt/ledger types. Deterministic policy selects from a closed eligible candidate set; classifiers advise only. Separate model catalog discovery, evidence validation, operator-owned eligibility, cost/quality ranking, frozen execution, and outcome feedback. Pricing refresh may invalidate stale admissions; it must not autonomously authorize a new backend, different privacy terms, spending or writable authority. Candidate discovery can be automatic; promotion requires the existing evidence and approval boundary. Guard against taxonomy/model alias drift and unknown pricing, preserve one treatment per task and transparent handoff at phase gates.

Reduce worker context and tool surface before adding classifiers. Unknown cache metrics remain unknown. OpenRouter sampled classifiers are telemetry, not dispatch routing or success evidence. Jev cloud classification is a paid outbound request with its own privacy/cost policy. Compare deterministic features/local inference/Jev only where measured avoided work exceeds classification/review overhead.

## Local inference and dotfiles coordination

Hardware M3 Pro36GB, desired lightweight resident inference under10GB INCLUDING load peaks/caches. Dotfiles agent owns runtime benchmarks and service lifecycle. Plan: `~/dotfiles/docs/plans/2026-09-20-lightweight-local-inference-and-agent-economy.md`, commit `a824a3b`.

Benchmark installed llama-server first, official MLX-LM challenger. MLX acceleration doesn't require mlx-serve; no replacement proven fastest/stablest yet. One quantized2B–4B model, one slot, bounded context. Existing launcher allows20GB across2models, not desired target. SemIf documented MLX path loads about9GB source weights before more memory; do not promote it to resident daemon without measurement. Older RSS/schema/throughput receipts have errata.

Research/transcript already ingested, do not redownload by default:
- `~/Downloads/53wDOI_7x8I-transcript.en-US.txt` and timestamped VTT from dotfiles agent.
- Source-session copy `~/.jcode/scratch/semif-research/53wDOI_7x8I.en-US.vtt` and `.txt`.
- VTT SHA256 `b6739924af66e5bcede2435aaba750e0d65968f6a26001556235f2d76931f1ee`.
- Original supplied research is a Markdown file in Downloads with unusual URL-shaped name beginning `[https___github.com_TheoLeeCJ_SemIf]`.
- Dotfiles verification previously24 evaluator tests/policy gate passed; full repo blocked by2 Pi fixtures involving pre-existing Exa-extension vs empty-extension contract. Don't overwrite other agent's work to fix it.

## Acceptance commands and limits

Use coordinated `selfdev test` for Rust commands, preferably `scripts/dev_cargo.sh test --locked --offline ...`; targeted tests before broader expensive gates. Discover exact test names with `-- --list` and ensure filters run nonzero tests.

```sh
scripts/dev_cargo.sh test --locked --offline -p jcode-app-core --lib comm_session_tests
scripts/dev_cargo.sh test --locked --offline -p jcode-app-core --lib worker_context_receipt_ -- --test-threads=1
scripts/dev_cargo.sh test --locked --offline -p jcode-app-core --lib spawn_tool_narrowing_tests -- --test-threads=1
python3 scripts/check_module_files.py
python3 scripts/check_code_size_budget.py
python3 scripts/check_test_size_budget.py
python3 scripts/check_panic_budget.py
python3 scripts/check_swallowed_error_budget.py
python3 scripts/check_dependency_boundaries.py
python3 scripts/check_wildcard_reexport_budget.py
git diff --check
```

Live-turn and queue test filter names/commands should be recovered from the referenced worker test tasks or discovered before execution. No full workspace test, full Clippy, full formatting or final runtime pass is claimed. Historical repository guardrail/formatting debt and known flaky changelog/concurrency tests exist; do not suppress failures or mislabel new regressions as historical.

## Ownership and safe transition

All pending worker outputs remain untrusted until captain inspects them. Keep scripted build edits and lifecycle edits separate in commits. Do not stop/wake/resume old workers merely to clear chips: `resume`/`wake` swarm actions are DAG controls, `message delivery=wake` actually resumes completed workers, and stale membership status has caused ownership confusion. Old daemon's stop bug remains until accepted code is built and running. Verify terminal response and background tasks before file transfer.

No new session needs this entire prior chat. Use this handoff plus exact local diffs and bounded task packets. Avoid dumping credentials, encrypted reasoning/session payloads, user transcripts or raw prompts into artifacts/provider research. This document contains local paths and operational preferences and should itself be treated as private unless explicitly approved otherwise.

## Final checkpoint supplement (19:14 UTC)

Both writers have terminal responses: llama 19:11:33 UTC, wolf 18:55:14 UTC. The swarm await delivered both as ready at19:13:36. No further implementation packets were sent after the safe-checkpoint request. Mouse is idle. Do not wake them just to begin the next session.

Llama's exact owned patch and test logs are preserved in
`~/.jcode/scratch/swarm-stop-queued-worker/`, including `precise-owned.diff`,
`diff-summary.txt`, `structural-gates.log` and the five task output files named
above. This is evidence, not a patch to blindly reapply over the already-dirty tree.
Final lifecycle diff is11 owned files, +1112/-189 INCLUDING the untracked560-line
test module, which ordinary `git diff --stat` omits.

Additional limitation requiring review: stopped-session gate tombstones are
process-local until explicit resume or daemon restart. Do not claim durable
post-restart stop enforcement without testing and defining that contract.

Exact worker test commands recovered from preserved outputs:
```sh
scripts/dev_cargo.sh test -p jcode-app-core comm_session_stop_tests -- --test-threads=1 --nocapture
scripts/dev_cargo.sh test -p jcode-app-core server::comm_session::comm_session_tests
scripts/dev_cargo.sh test -p jcode-app-core server::client_actions::tests
scripts/dev_cargo.sh test -p jcode-app-core server::client_lifecycle::tests
```
The labels “live-turn” and “queue lifecycle” above describe coverage in these
modules, not literal test-filter names. These worker commands did not include
`--locked --offline`; captain should use those flags where supported and confirm
nonzero discovery. Final `git diff --check` passed at handoff. No pending source
patch was committed or accepted by the captain, and no new build/reload ran.
Only this handoff document is being committed for the transition.

## Session closeout (2026-09-20, 19:57 UTC)

Both pending patches were reviewed, validated, and integrated separately, then
the routing work continued. Seven commits on `jcode/ci-format-baseline`, all
verified, tree clean at `9e5b5305e`:

| Commit | Change |
| --- | --- |
| `c146629a0` | Swarm stop is now quiescent: interrupt-delivery gate, cancel re-fire while acquiring a retained terminal Agent guard, verified clears, retryable timeout |
| `1d45c4e10` | Portable host-wide Cargo gate via Python `fcntl.flock` when `flock(1)` is absent |
| `e627e8eb6` | `opencode-go` classified as an included-subscription route, so Go spawns pass the launch check; unverified profiles stay metered |
| `0b0cac62d` | Test pinning `agents.swarm_model` → selection → route class |
| `b56ba9b9c` | Budgetless spawn envelopes use the shared no-budget spawn contract instead of demanding a metered reservation at provider dispatch |
| `9e5b5305e` | Spawn `working_dir` refuses unexpanded `~`/`$` and missing directories |

Dotfiles: `0ad12af`, `1976f49`, `cdfbb4b`, `df7bdb0` (canonical policy, regenerated
surfaces, receipts).

Verified facts and remaining work are in
`~/dotfiles/docs/measurements/2026-09-20-opencode-go-deepseek-v41-flash-route.md`:
exact model id, entitlement windows, privacy matrix, both production negative
controls, end-to-end spawn acceptance, bounded-patch admission, and the re-check
list (4x-boost end 2026-09-20, DeepSeek ZDR expiry 2026-09-30).

Still open, not started deliberately: OpenCode-Go window visibility (console-only
today), catalog/pricing-refresh automation, and a real quality/review-cost
benchmark. Known pre-existing defect: four `autodetects_*` tests in the
OpenAI-compatible runtime fail under `scripts/dev_cargo.sh` because its test-state
isolation sets `JCODE_HOME`, which overrides the `HOME`/`XDG_CONFIG_HOME` those
tests use; they pass with `JCODE_HOME` unset.
