# Fable 5.1 fresh-session handoff: source closeout and harness economy

Prepared 2026-09-20 around 02:33 UTC. **Read this first.** It supersedes execution-state claims in the earlier source-closeout handoff, not safety/provider policy. The final ownership addendum below is authoritative over intermediate worker reports.

## 1. Operator goal and authority

The operator repeatedly approved continued source implementation, closing inherited unfinished work, and bounded cheaper OpenRouter tasks within the existing budget. Latest direction: prioritize token/agent economy across actual cost, elapsed/review time, accuracy, networking and automatic assessment/improvement. Latest request is this comprehensive handoff for **Fable 5.1 in a fresh session**, with architectural improvement and implementation continued.

Use Fable 5.1 as the architecture lead via the explicitly pinned included route `claude-oauth:claude-fable-5-1`. Current canonical provider policy restricts Fable to concise architecture consultation, not routine implementation/review loops. Organize implementations through bounded Sol/high Jcode source packets; Terra handles suitable routine non-security packets. The user's model request does not authorize silently changing those policies or using a paid API equivalent. If the selected Claude route is unavailable, shared quota unknown/exhausted, or OAuth fails, report the frozen model/effort and stop that treatment. Do not silently substitute another provider/account.

The operator explicitly confirmed a separate OpenAI OAuth credit pool of hundreds of dollars after included weekly exhaustion. **Native weekly 100% is not a hard stop for explicitly requested bounded useful work.** Credits, API-equivalent estimates and OpenRouter invoices are different quantities. No quota-filling work.

Still separately gated: install/binary promotion/shared-daemon restart, push/publish/release/deployment, branch/worktree creation, provider-default/auth changes, destructive cleanup, host credential recovery. None occurred in this continuation. Repeated Continue approval is not approval for those effects.

## 2. Read order and source identity

1. This handoff and its final ownership addendum.
2. `docs/HARNESS_ECONOMY_CYCLE_2026-09-20.md`: accepted metric contract and deadline evidence.
3. `docs/SOURCE_CLOSEOUT_REVIEW_2026-09-20.md`: accepted ancestor instructions, lint prerequisites and cheap-task receipts.
4. For specific older questions only, `docs/HANDOFF_2026-09-20_SOURCE_CLOSEOUT.md` and `docs/SOURCE_CLOSEOUT_2026-09-20.md`. Do not repeat the broad audit.
5. Canonical policy: `~/dotfiles/policy/providers.md`, `policy/upstream.md`, and `~/dotfiles/docs/plans/2026-09-19-agent-token-economy-optimization.md`, especially its acceptance correction. Historical route diagrams and balance claims below that correction are not current authority.

Canonical checkout: `/Users/ianalitis/.jcode/source/jcode`, branch `jcode/ci-format-baseline`.
At 02:32 UTC, source HEAD was `98196b0744af60498d953015835b26136566d1f9`, index empty, **290 dirty/untracked paths**. This document will add a local docs commit. Runtime binary is not source HEAD; no accepted changes below have been promoted by this session. Do not assume a running daemon measures this source.

The inherited tree contains extraction, J5 and other owners' work. Never blanket-stage, reset, stash, switch branch or merge it. Accepted source has been staged in exact slices, sometimes by applying a patch only to the index while preserving a different working-tree extraction. New ledger files are untracked and must not be omitted if eventually accepted.

## 3. Accepted local commits and evidence

| Commit | Scope and verified result |
| --- | --- |
| `dd547d8f4` | Core platform-specific stdin lints. Strict core Clippy passes. Serial original/candidate both 38 pass and the same one stdin-reading test failure. Not full core test green. |
| `da56ca22f` | Terminal-launch macOS needless return. 29 tests and strict all-target Clippy pass. |
| `5d3093341` | Three mechanical base lints: Cursor macOS return, equivalent auth conditional, usage-row alias. Unrelated lifecycle test extraction was not accepted. |
| `812c6de33` | MCP test fixture keeps environment lock outside current-thread runtime and restores cwd/home through guards. Three pool tests pass. |
| `43cbbfd4c` | Three-file bounded ancestor AGENTS implementation. 44 prompt tests and strict all-target base Clippy pass in exact isolated export with prerequisites. |
| `884587e31` | Bounded economy measurement/acceptance contract. Not automatic routing implementation. |
| `952ab8928` | One checked absolute deadline across guarded provider opening and stream consumption. Old code fails regression at captain-measured 1.879s for a one-second deadline; candidate passes the same <1.5s assertion. Worker measured 1.856s -> 1.002s. 13 caller tests, 8 single-send tests and strict library Clippy independently pass. Overflow regression prevents a new panic for u64::MAX. |
| `98196b074` | Durable receipt for deadline acceptance, not ledger persistence. |

Ancestor contract: global first, repo root through cwd, nonrepo global+cwd, no outer workspace inheritance; 32 unique files, 64 KiB/file, 256 KiB total, regular UTF-8 only, canonical dedupe/containment, visible bounded diagnostics, source/layer/hash provenance. Captured tuple reused, recaptured by existing cwd/clear/restore lifecycle. Global user-selected symlinks retained. Snapshot-time checks do not defend hostile concurrent path replacement. A generic silent-error helper was rejected before acceptance.

Deadline fix is scoped to the existing frozen-attempt caller. It is not a full tool-using worker lifetime deadline and does not wire a native proposal CLI. A permanent regression now automatically detects the concrete timing regression. No generalized economic evaluator or automatic promotion was implemented.

## 4. Current unfinished priority: durable LocalLedger, NOT accepted

Owner: Sol/high **parrot**, `session_parrot_1789870291969_8fabcb5e9ec75488`. Root requested safe pause at 02:32 UTC for handoff. Read final addendum before resuming or assigning another writer.

The first writable packet completed two implementation iterations. Candidate adds explicit `LocalLedger::open(path, cap)`, retains `new(cap)` memory mode, uses a stable lock sidecar held for the last clone, persists mutations before returning, and poisons uncertain handles. It moves existing concrete ledger code rather than inventing a parallel ledger.

Allowed paths:
- `crates/jcode-attempt-types/src/lib.rs`
- new `crates/jcode-attempt-types/src/ledger.rs`
- new `crates/jcode-attempt-types/src/ledger_tests.rs`
- `crates/jcode-attempt-types/src/tests.rs` (unchanged in initial packet)
- `crates/jcode-provider-openrouter-runtime/src/attempt_caller_tests.rs`

No dependency/Cargo edits, CLI wiring, provider defaults, live inference, source staging/commit, installation or other source paths were authorized. Existing `lib.rs` also contains inherited J5 changes, so **do not accept its whole HEAD diff**. Use worker pre-edit/incremental artifacts to separate ownership.

Initial, now-rejected candidate artifact:
`~/.jcode/scratch/agents-captain-0127/durable-ledger-artifact/{receipt.md,exact.diff,scoped-tests.log,caller-test.log}`.
Initial exact diff SHA-256: `2c2e592cd09ff74233bb50049590f9fd5b6ea112936fc3f228609e7a904749c5`. It is historical once corrections start, not proof of current bytes.

Worker-reported initial checks: 57 attempt-types tests pass, four qualified exact durable tests pass, one caller reopen/duplicate loopback test passes, strict attempt-types all-target Clippy passes, scoped formatting passes. **Captain did not accept this candidate despite those passes.** Runtime package Clippy was blocked by reported pre-existing warnings at `openrouter_provider_impl.rs:319` and `openrouter_catalog_merge_tests.rs:129`.

### Captain acceptance blockers and corrective contract

A new corrective Sol/high packet was explicitly bounded to **one iteration then checkpoint**, same paths. At handoff request its final implementation/validation was not yet accepted.

1. Predictable `.tmp` opened with `create(true)/truncate(true)` can clobber an unrelated pre-existing file or follow a symlink. Require uniquely owned, same-directory `create_new` temporary files; only remove files owned by this operation.
2. `Path::exists()` suppresses metadata errors and dangling-symlink state, permitting incorrect empty initialization. Read/open directly, distinguish true absence, reject ledger/lock symlinks/nonregular paths. No reset on any read/parse/version/cap error.
3. Relative paths remain sensitive to later cwd changes, disconnecting the data path from the held lock. Resolve the parent once to a stable absolute path and handle bare filenames correctly. Document a private trusted-directory assumption; do not claim hostile directory race protection.
4. JSON map duplicate keys normally deserialize last-wins. Reject duplicate reservation keys rather than dropping exposure. Validate persisted invariants. Durable mutations must not admit empty IDs that make their own saved state unreopenable.
5. Poison checks before mutex acquisition can race a failed persistence operation and report stale usable exposure. Check under the relevant mutex as well.
6. Full-map cloning was introduced even for non-durable mutations. Preserve existing in-memory allocation behavior. Correction: reserve already scans exposure O(n); the new cost is extra full-map/string allocation and previously O(log n) settle/mark/reconcile becoming O(n). Do not add a speculative exposure cache or journal framework.

Require baseline-failing regressions for sentinel temp/symlink preservation, dangling ledger rejection, stable cwd-independent paths, duplicate keys, invalid IDs and poison handling. Retain reopen held/ambiguous/settled/reconciled/overage behavior, process lock refusal/release, post-rename failure injection, and zero additional caller sends after admission rejection. Aggregate exposure overflow must remain fail-closed without forgetting recorded actual spend. A local reservation ledger is not a provider-side bill cap. Non-Unix directory-fsync limits must be explicit.

## 5. Why native cheap proposal dispatch is still blocked

`src/cli/dispatch.rs` routes `jcode run` to `run_single_message_command`; `src/cli/commands.rs` constructs a selected provider, Registry/MCP tools, Agent and optional resumed session. This is not a safe isolated no-tool proposal entry point.

Reuse the guarded chain:
`run_frozen_attempt` -> `LocalLedger::reserve` -> `complete_single_send_with_expected_final_request` -> exact final-body/destination guarded single send -> stream -> settle/ambiguous -> harness receipt.

Present: data-class/floating-model/router restrictions at freeze, MeteredRemote/exact model checks, request body/destination binding, one-send/no retry/redirect/proxy path, absolute opening+stream deadline, receipt digests and ambiguity accounting.

Still missing or not demonstrated end to end:
- durable state and receipt persistence across process exits;
- actual enforcement of `max_input_bytes`, `max_output_bytes`, `prompt_hash` against supplied bytes;
- full tool/system/endpoint/provider/router-policy binding;
- explicit outbound eligibility of supplied packet bytes. A public filesystem root or user/model-supplied label is not proof that inherited instructions, symlink targets or tool results may be sent.

Do not wire the CLI until these are resolved in bounded packets. Do not enable unrestricted metered writable swarms. Sequence after ledger acceptance: byte/tool/request binding with zero-send tests, then explicit eligible no-tool entry point, then paired real useful task trials. Reuse existing types/tests, not a new routing platform.

## 6. Actual cheap-route attempts and budget state

The operator explicitly wants useful cheaper OpenRouter work, not endless premium-only planning. However no cheap artifact has yet passed acceptance in this continuation.

Last sanitized dedicated-key metadata: limit **$5.00**, remaining **$4.896949018**, reset null, usage $0.103050982. This is historical; refresh through sanitized metadata before a new dispatch. Account balance/top-up is not a task budget. The old $10/day plan is historical, not fresh proof. Never print/store keys or raw credential/config contents.

The paid `deepseek/deepseek-v4-flash-0731` lacked the prescribed OpenInference FP8 endpoint. That treatment closed before inference. Separately admitted free slug `deepseek/deepseek-v4-flash-0731:free`, exact `open-inference/fp8`, had current endpoint/ZDR/privacy checks for those attempts. Constraints: no fallback, ZDR true, data_collection deny, required parameters, FP8, zero unit-price ceilings, <=8 KiB input, 2,048 output tokens, one request/concurrency one, 90-second cap, no tools/proxy/redirects/retry. This scratch probe does not establish native writable routing enforcement.

Three outcomes under `~/.jcode/scratch/source-closeout-20260920-review/`:
1. `free-boundary-packet.py/.jsonl`: stopped with JSONDecodeError after 9.769s before response identity was persisted. Exact failure layer unknown. $0.01 reservation remains ambiguous.
2. `free-boundary-packet-v2.py/.jsonl`: observability fixed and offline checked; valid response, 500 input/436 output tokens, 12.56s; only **6/12 complete cases correct**, rejected. Later authoritative generation reconciliation reported actual cost zero in `free-boundary-v2-reconciliation.json`. Generation `gen-1789867045-ujT6HCBsJIWPKJEye7yI`.
3. `public-allocator-proposal.py/.jsonl`: distinct public-function patch proposal, medium reasoning, 2,010 request bytes, timed out at 90 seconds without response identity. No retry. Additional $0.01 reservation ambiguous.

**$0.02 total unresolved local reservations.** Do not release based on unchanged aggregate key usage. Zero accepted results means cost per accepted result is undefined, not zero. Do not rank models from these heterogeneous three outcomes. No model/provider defaults or account limits changed. Pareto/auto routers remain held; availability in `list_models` is not authorization.

## 7. Other unfinished source and host issue

J5 named reproduction: `tool::communicate::tests::communicate_fill_slots_tops_up_to_concurrency_limit`, isolated HOME/JCODE_HOME, failed with `cannot establish spawn route billing for openai before launch`; build 4m32s, test 0.27s. Worker turkey proposed setting runtime provider to `ollama` as a local billing proxy. Captain rejected it, removed only worker changes, and verified byte equality with its pre-edit snapshot. Production admission was not weakened. Literal fixture route identity remains unresolved. Artifact: `~/.jcode/scratch/j5-fill-slots-sol-20260920T0145Z/pre-edit/`. Turkey is stopped.

Earlier base/core/full-tree debt remains. Native whole-tree size/test-size/panic/swallowed-error budgets and broad formatting are not green. No gate/baseline was relaxed. Core serial stdin failure and a separate parallel allocator test failure are unresolved. Do not turn each prerequisite into an open-ended repair sweep.

**Host Bedrock incident remains outside authority:** earlier nonisolated tests touched `~/Library/Application Support/jcode/bedrock.env`. No contents/backups were read or restored. Do not inspect/recover/re-authenticate without named approval for that file and effect. Source isolation corrections prevent recurrence but do not recover credentials.

## 8. Reusable validation environment and evidence discipline

Warm export: `~/.jcode/scratch/agents-captain-0127/tree`; target: sibling `target`.
This is a **mutable overlay**, not an immutable current HEAD export. Different baseline/candidate runs replaced files. Verify hashes and exact contents before claiming tests measure a candidate. Worker/captain before files and hashes take priority over an old generic manifest.

Use existing toolchains, no installation/trust changes:

```bash
export PATH=/Users/ianalitis/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin
export CARGO_HOME=/Users/ianalitis/.jcode/scratch/privacy-integration-20260920T0012Z/cargo-home
export RUSTUP_HOME=/Users/ianalitis/.rustup
export HOME=/Users/ianalitis/.jcode/scratch/agents-captain-0127/home
export JCODE_HOME=/Users/ianalitis/.jcode/scratch/agents-captain-0127/state
export XDG_RUNTIME_DIR=/Users/ianalitis/.jcode/scratch/agents-captain-0127/runtime
export CARGO_TARGET_DIR=/Users/ianalitis/.jcode/scratch/agents-captain-0127/target
export CARGO_BUILD_JOBS=4 GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
cd /Users/ianalitis/.jcode/scratch/agents-captain-0127/tree
scripts/dev_cargo.sh test --locked --offline -p jcode-attempt-types
scripts/dev_cargo.sh clippy --locked --offline -p jcode-attempt-types --all-targets -- -D warnings
scripts/dev_cargo.sh test --locked --offline -p jcode-provider-openrouter-runtime --lib attempt_caller::tests -- --test-threads=1
```

For `--exact`, use the full module-qualified test name and verify nonzero count. The first read-only assessment suggested unqualified exact filters, which were corrected before implementation. Wrapper rejects zero tests with exit97. Fake HOME/JCODE_HOME are necessary but not a filesystem/network/credential sandbox. Inspect fixture effects. No actual model calls in loopback tests.

Captain deadline evidence: `deadline-captain-{hashes.json,baseline.log,caller.log,single.log,clippy.log}` under the warm root. Baseline failure must be the intended assertion, not compile or environment failure. Changed-file formatting, staged hash equality and `git diff --cached --check` are required. Full build success alone is not runtime behavior proof. Runtime activation requires separate approval.

Efficiency: one writer, at most useful independent readers; fresh compact packets, bounded iterations, existing warm caches, async waits rather than polling. Do not dump `swarm read_context` or entire model catalogs: those tools produced huge outputs. `swarm await_members` can return stale prior-generation reports; compare live status/task/artifacts. `list_models` returned an enormous route/provider inventory when unbounded; avoid repeating it just for curiosity. Models and effort are fixed per task; a failed treatment closes, not silently switches.

## 9. First actions for the fresh captain

1. Confirm branch/HEAD/index and read final ownership addendum. Do not resume from the initial ledger completion report alone.
2. Review corrected ledger diff against pre-edit bytes. Prefer a small rejected checkpoint over committing unsafe durable accounting. If the one corrective iteration is exhausted, define a fresh explicit packet rather than an unbounded recovery loop.
3. Independently run exact-candidate filesystem, subprocess, admission/zero-send and existing accounting tests. Review cross-process lock/path assumptions and all failure paths, not only happy-path persistence.
4. Commit only accepted incremental ownership. Preserve inherited `lib.rs` J5 changes and all unrelated dirty files. Record actual outcomes/limitations.
5. Continue one minimal byte/tool-binding prerequisite, then native no-tool proposal dispatch and same-task cheap-vs-baseline measurements. No routing platform, broad model shopping, automatic spending-limit increase or silent host activation.

## Final ownership addendum

Sealed at 02:36 UTC. Parrot returned a safe checkpoint and was explicitly **stopped** by the captain. Turkey and rabbit were also stopped after their packets. No ledger validation was running at pause, no new validation was launched for handoff, and no further source writer is authorized by this session. Other projects have background tasks; do not cancel or adopt them. All session-owned swarm waits resolved. Fresh captain must explicitly take ownership before edits or validation.

**The corrective source has been written but has NOT been formatted, compiled, tested or linted.** None of the previous 57-pass results apply to corrected bytes. The worker reports all requested corrective areas implemented, but that report is unverified. Start with compiler-guided validation, not commit or CLI integration.

Frozen scratch checkpoint: `~/.jcode/scratch/handoff-fable-20260920T0235Z/` contains `CHECKPOINT.json`, `git-status.txt`, copies of the four current source files under `source/`, and both original/corrective worker artifacts. At capture: HEAD `98196b074`, empty index, 291 status entries including this new handoff. After its docs-only commit, the pre-existing 290 dirty entries should remain.

| Current path | SHA-256 at seal |
| --- | --- |
| `crates/jcode-attempt-types/src/lib.rs` | `dc6b8f2e9df81ca596c665686a69965987ddd6a3e03a1c15c80c597c41522fe5` |
| `crates/jcode-attempt-types/src/ledger.rs` | `f5c75c2887027215b06c505099f27359b4f54a00e10ee49a9437e04d3ddf63c1` |
| `crates/jcode-attempt-types/src/ledger_tests.rs` | `6fae12735c7701f0b04480c9b267001f8833f71eedd37959fa04c45eefa34aad` |
| `crates/jcode-provider-openrouter-runtime/src/attempt_caller_tests.rs` | `145b5dcfbf9b27b66261d9b696ab596ecd51186f5fc837501f07aaae9dee741d` |

Corrective worker checkpoint: `~/.jcode/scratch/agents-captain-0127/durable-ledger-correction/CHECKPOINT.md`, SHA-256 `df31295d898a25a4652d06f3a8a8d8b603b008568f06657e957af33f19b322e8`; sibling `checkpoint.diff` SHA-256 `c2f9eadfab3f71843855557cfa113c78417b79b6bef18d949273b95bda25ec23`. Those are the correction relative to the rejected candidate, not the total delta relative to source HEAD. Preserve the initial `exact.diff` and pre-existing changes separately.

Worker validation environment differed from the captain export: fake HOME/JCODE_HOME under `agents-captain-0127/durable-ledger-validation/`, `JCODE_SCRATCH_DIR` at its `fs/`, shared warmed target, existing `/Users/ianalitis/.cargo` cache. Read its checkpoint before reusing. Do not confuse its combined-tree results with an independently exported staged candidate.

### Fresh-session opening prompt

> Read `docs/HANDOFF_2026-09-20_FABLE_HARNESS_ECONOMY.md`, especially the final ownership addendum, then `docs/HARNESS_ECONOMY_CYCLE_2026-09-20.md`. Continue as the explicitly selected Fable 5.1 architecture lead within current provider policy, delegating bounded Jcode implementation to Sol/high. Prioritize safe useful cheap-model work and cost/time per accepted result. First verify sealed hashes and take ownership of the unformatted, uncompiled durable-ledger correction. Review and validate it before any commit or native CLI wiring. Preserve all unrelated dirty work. No install, push, routing-default/auth change or host credential recovery is approved. Avoid repeating broad audits or model shopping. Use compact packets, permanent regression tests, exact-candidate captain verification, and honest billing/latency evidence.

## Continuation receipt (02:38 to 03:00 UTC, fresh captain)

Ownership of the durable-ledger correction was taken after verifying the four sealed hashes. Two commits were accepted, each validated on an exact `git archive` of the index in the isolated HOME/JCODE_HOME environment:

| Commit | Scope | Evidence |
| --- | --- | --- |
| `89a2ae8b8` | Durable `LocalLedger` (ledger.rs, ledger_tests.rs, lib.rs module extraction only, caller reopen test). Captain fixes over the worker checkpoint: `next_entry::<String, Reservation>()` type annotation, and a bounded `try_lock` retry (50 x 5 ms) because a concurrent `fork` in the test process briefly duplicates the advisory lock (flaky `LockUnavailable`, reproduced standalone; 0/150 after). | 66 attempt-types tests, strict all-target Clippy (1.98.1 and 1.94.1), 14 caller tests. All 8 new regressions fail on the rejected candidate with inert API shims. |
| `c97302202` | `run_frozen_attempt` binds `tool_allowlist` (empty admits none), `max_input_bytes` against the serialized expected body, nonzero `max_output_bytes`, and stops consumption past `max_output_bytes` (`OutputLimitExceeded`, exit 125, ambiguous exposure). All refusals happen before reservation and before any send. | 17 caller tests plus 10 router_shaping tests, strict runtime lib Clippy. 3 new regressions fail on HEAD's caller. |

J5's `SpawnExecutionEnvelope` block in `lib.rs` remains unstaged and untouched (40 lines). 287 dirty entries remain. No install, push, routing/auth change, host credential recovery or inference occurred. The $0.02 ambiguous OpenRouter reservations are unchanged.

Still missing before native no-tool proposal dispatch: `prompt_hash` binding to the supplied bytes (deferred because the fixture producers in `router_shaping_tests.rs` sit in a dirty inherited file and would need a hash-from-body helper), system/endpoint/router-policy binding, explicit outbound eligibility of packet bytes, and receipt persistence. Next packet: bind `prompt_hash = sha256(expected body)` with a `frozen_for(body)` test helper, then the eligible no-tool entry point.

Tooling note: `cargo` on PATH is a broken mise shim (untrusted config). Invoke `/Users/ianalitis/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo` with that toolchain's `bin` first on PATH so `cargo clippy` also resolves.

### Continuation 2 (03:02 to 03:12 UTC)

| Commit | Scope | Evidence |
| --- | --- | --- |
| `ffc493a2a` | `prompt_hash_for(body)` public helper; caller refuses when frozen `prompt_hash` differs from the supplied body digest. Fixtures freeze against the body they supply. | 28 tests (18 caller + 10 router_shaping), strict lib Clippy, exact staged export. Regression fails with the check disabled. |
| `b5cb120aa` | Caller refuses when frozen `endpoint` differs from `expected_destination`. System prompt is bound via the body digest, so no separate check. | 29 tests, strict lib Clippy, exact staged export. Regression fails with the check disabled. |

Request binding is now: model, route class, endpoint, body digest (which covers messages, system, model, stream options), tool allowlist, input/output byte bounds, deadline, ledger reservation. Router policy is still enforced only at freeze (`check_router_admission`) and at the receipt gate (served model), not re-checked against the body's `provider.order`/exclusions; that is a small follow-up if the shaped body ever diverges from the frozen policy.

Remaining before native no-tool dispatch: (1) explicit outbound eligibility of packet bytes (design decision: the entry point should accept only a `DataClassPolicy`-classified `input_paths` set plus literal text, refusing anything derived from unclassified paths, reusing `admit_node`'s classification rather than a new mechanism); (2) receipt persistence beside the ledger (append-only JSONL under the same trusted directory, same create_new/rename discipline). Both are bounded packets; neither has been started.

### Continuation 3 (03:14 to 03:20 UTC, operator approved)

| Commit | Scope | Evidence |
| --- | --- | --- |
| `52d712240` | `OutboundPacket::assemble(policy, parts, max_bytes)` and `check_frozen(attempt)`: literals carry a declared class, files are classified through `DataClassPolicy` (undeclared is Private), symlinks/dirs/non-UTF-8/oversize refused, secret roots and secret shapes refused, packet class must not exceed the frozen `data_class`. | 71 attempt-types tests, strict Clippy, exact staged export. |
| `8a26d3400` | `ReceiptLog`: append-only JSONL beside the ledger; canonical path, `create_new`, symlink refusal on open and append, `O_APPEND` + fsync, `read_all` reports torn lines/duplicates/missing newline as `Corrupt`, append refuses duplicates and never rewrites. No lock of its own; the ledger lock is the serialization point. | 75 attempt-types tests, strict Clippy, exact staged export. |

All prerequisites named in section 5 now exist as types with tests. Nothing is wired into `jcode run` or any default.

**Blocking design decision for the no-tool entry point.** The single-send seam refuses unless the provider's built body equals `expected_final_request`, but that body is produced inside `complete_inner` (~800 lines, dirty inherited file) from provider state: reasoning fields, cache breakpoints, `session_id`, `max_tokens`, plugin shaping. A captain cannot compute `prompt_hash` for a real provider without either (a) a pure `build_final_request(messages, tools, system) -> Value` extracted from `complete_inner` and reused by it, or (b) a two-phase call where the provider returns the body it would send and the captain freezes against that before authorizing the send. (a) is the honest binding; (b) risks freezing whatever the provider proposes. Recommend (a) as one Sol/high safe-refactor packet with a byte-equality test between the extracted builder and the body observed by the loopback server, before any CLI wiring. Not started.

Economy note for this session: zero metered spend, zero worker spawns, five source commits accepted in about 40 minutes of captain time. Cost per accepted result is therefore captain time only; the cheap-lane figure remains undefined until a paired trial runs through the bound entry point.
