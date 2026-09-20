# Fresh-session handoff: source closeout, native instructions, and budgeted workers

Prepared 2026-09-20 UTC. This supersedes the execution-state portions of the September 19 handoff, not repository/global policy. Read this file first, then `docs/SOURCE_CLOSEOUT_2026-09-20.md`. Do not restart the broad investigation.

## 1. Operator goal and latest authority

The operator repeatedly authorized continuing source implementation, resolving inherited dirty work, and reaching a good closeout with the highest-priority tasks completed. The latest request is to prepare a comprehensive handoff for a fresh session and begin making substantial use of cheaper OpenRouter models. The operator says a spending budget is already in place and the account was substantially topped up.

Treat that as explicit intent to use budgeted paid OpenRouter work, not a reason to remain exclusively on premium subscription workers. It does not erase ZDR, pinned treatment, data-class, tool-authority, or spend-enforcement requirements. Do not infer an unlimited numeric budget or invent current balance/key limits from historical notes. No payment/recharge operation is requested.

Still separately gated: installation/binary promotion/shared-daemon restart; push/release/deployment; branch/worktree creation; destructive host/data cleanup; credentials/auth/provider-default changes. The host Bedrock incident below still needs named recovery approval. No such effects were performed in this closeout.

## 2. Exact source and ownership state

- Canonical checkout: `/Users/ianalitis/.jcode/source/jcode`.
- Branch: `jcode/ci-format-baseline`.
- Latest source checkpoint before this handoff document: `f1d193b30`.
- Parent implementation commit: `312f9b37a` privacy removal.
- At 00:46 UTC: 181 tracked dirty paths and 116 untracked files. Counts change as the handoff is written; remeasure. Do not blanket-stage them.
- Index was empty after the privacy/receipt commits. Check it before staging anything.
- Existing approved privacy worktree: `/Users/ianalitis/.jcode/source/worktrees/privacy-no-collection`, branch `jcode/privacy-no-collection`, commit `c81735fdf`, clean when checked at 00:46 UTC. Its privacy behavior is now integrated into canonical source. Do not merge it again or delete the worktree.
- Runtime identity is not source HEAD. No binary was installed or daemon restarted by this closeout. Earlier session context reported `11be474fe`; a sibling packet reports another runtime. Neither proves the identity of every running process. Verify only if needed, without secret-bearing environment reads.

There was one active writer: Sol/high **snake**, session `session_snake_1789863115287_241364091af33914`. It completed privacy integration and then began the separate ancestor-AGENTS packet. At handoff request the captain instructed it to pause safely and return a current-generation checkpoint. See the final ownership addendum below before starting another writer.

Reader **swan**, session `session_swan_1789864359821_afd2f79c78674f05`, reviewed privacy and fixture correction. It owns no source edits. All earlier implementation workers were retired. Never assume an old completion event belongs to the current task generation.

## 3. Accepted commits and what they establish

| Commit | Accepted scope |
| --- | --- |
| `db358b3f6` | Provider classifier/free-model planning, not activation |
| `a86dcfe46` | Cargo wrapper respects foreign checkout cwd; synthetic regression |
| `459784734` | Bedrock test extraction plus per-test state/credential-hint isolation |
| `3ba9f8a76` | Fresh local JCODE_HOME/runtime for wrapper-owned Cargo tests by default |
| `a1a5d213c` | Shared ledger relocation, full actual-cost accounting, explicit reconciliation and overflow rejection |
| `96411b62d` | One-file rustfmt correction to ledger re-export |
| `312f9b37a` | Remove optional collection from fork source, independently validated exact staged candidate |
| `f1d193b30` | Durable privacy/budget checkpoint and unresolved-gate receipt |

Ledger checks: 48 attempt-types tests, 11 loopback caller tests, strict types Clippy, package formatting and module checks passed. Over-reservation settlement retains actual cost as ambiguous; explicit reconciliation records authoritative actual spend; aggregate and admission overflow fail closed. This is accounting, NOT a proof of maximum provider bill.

Privacy removes collector/identity/queue/upload machinery, transcript-upload lifecycle, maintainer-feedback tool, sponsor usage metering, installer reports and optional OpenRouter attribution. Discovery defaults off and saves a durable opt-out. CLI/TUI accurately refuse collection enablement. Inert compatibility functions preserve local callers. Functional inference/MCP/auth/update traffic, local logs, explicit exports/support and swarm completion reports remain. No historical local/remote purge occurred.

### Privacy exact-candidate verification

Original `c81735fdf` delta was staged separately from unrelated dirty work. A reviewer required validation of the index, not just the combined working tree. The index was exported via `git archive`, without branch/worktree operations, to:

`~/.jcode/scratch/privacy-index-validation-20260920/`

Final candidate tree: `0691c1993822fc6acfc412c7795a178b985fef74`. Changed file bytes matched the export before commit.

Passed with `--locked --offline` and isolated test state:
- telemetry core all-target tests, including no-collection subprocess cases; strict all-target Clippy;
- base config 79, app discovery 33, TUI privacy 7;
- OpenRouter shaping 10 and caller 11;
- fake-curl installer fixtures, module resolution, staged whitespace check.

One test initially failed legitimately: banned-family fixture billed 2,070 micro-USD against reserve 500/cap 1,000. The corrected ledger rejected the overage before the banned-family assertion. Both synthetic limits are now exactly 2,070. Billed amount, served model and rejection assertion are unchanged. This is not a raised production budget or ratchet waiver. The combined dirty fixture still has its earlier J5 5,000 values unstaged; do not silently bundle that change.

Full combined-tree captain checks additionally passed support 10 and discovery 34. Broad app-core concurrency had 21 passes and one J5 fixture failure, described below. No full-workspace green claim.

Privacy artifacts:
- `~/.jcode/scratch/privacy-integration-20260920T0012Z/RECEIPT.md`
- same directory `privacy-incremental.diff`, `pre-edit/`, `post-edit/`, `owned-paths.txt`
- initial exact-index task `692730fs57` failed only at the underfunded router fixture after preceding tests passed;
- corrected router/caller task `9731386b40` passed;
- strict telemetry Clippy/installer passed in task `911795nadl`, whose aggregate exit failed because Git metadata was initially missing for the archive module check. That check was then rerun successfully with read-only GIT_DIR/GIT_WORK_TREE context. Do not count the failed initial invocation as a pass.

## 4. Native ancestor-AGENTS packet: current priority

Operator supplied packet:
`~/dotfiles/docs/plans/2026-09-20-native-context-discovery.md`.
It is not at canonical `docs/plans/` yet. It was handed to the existing source owner, not a competing writer.

Approved writable scope:
1. `crates/jcode-base/src/prompt.rs`
2. `crates/jcode-base/src/prompt_tests.rs`
3. `docs/SYSTEM_PROMPT_CONFIG.md`

Approved contract after read-only assessment:
- repository-root through cwd ancestors only, plus global once;
- global first, then outer-to-inner project layers;
- non-repository launch retains global plus cwd;
- no outer-workspace inheritance until an explicit admitted root mechanism exists;
- 32 unique instruction files, 64 KiB/file, 256 KiB total UTF-8 bytes;
- canonical identity dedupe, regular-file checks, bounded visible rejection diagnostics, no safety-text truncation;
- project symlink targets remain inside canonical repo root (nonrepo cwd); preserve/document existing user-selected global symlink behavior;
- source path/layer/hash provenance using existing sha2/hex;
- reuse captured tuple snapshot. Existing lifecycle recaptures on cwd change, clear and restore/resume. Do not promise immutable inputs across those events;
- no watchers, credentials/config/hooks/MCP/skills/provider auto-merge or lifecycle edits;
- two bounded implementation iterations, then checkpoint rather than scope expansion.

At 00:46 UTC these three files had a combined working diff of 648 additions/60 deletions relative to HEAD. That is NOT a reviewed/accepted implementation claim and may include prior dirty content. Pre-edit snapshot is essential.

Current artifact directory:
`~/.jcode/scratch/ancestor-agents-20260920T0036Z/`.

First fresh-session task: read the final worker checkpoint, compare its incremental diff to pre-edit state, inspect implementation complexity/security, run actual nonzero prompt test filters with synthetic home/repo fixtures. Verify containment, missing/unreadable/invalid-UTF8/oversized/nonregular files, aliases/escape, ordering and captured builders. Confirm no host files can be read through fixtures. If code-size or API requirements exceed the approved three-file slice, report exact delta rather than silently adding a trust framework. No runtime/shared-daemon promotion is authorized by this packet.

## 5. Unaccepted J5 and why cheaper writable swarms are not ready yet

Inherited workers **hippo** then **peacock** left unfinished J5 across app-core/protocol/provider-core/OpenRouter. Peacock was stopped after timeout. Do not restart its open-ended task or adopt it wholesale.

Known findings:
- current fail-closed guard rejects every concrete OpenRouter worker as lacking enforceable per-request cost bounds;
- the new absolute deadline only surrounds provider opens/streams, not complete child tool execution/idle lifetime;
- ambiguous transport exposure must not be released to zero without evidence;
- default provider implementations and visible/headless admission paths require explicit envelope handling;
- hard spend bounds need trusted billable-input/output limits, rate guards, separately billable dimension exclusions, and no retries/fallback/redirects;
- reserving remaining child capacity is better accounting but does not itself bound the provider bill.

The reviewer identified existing seams to reuse: `AttemptRecord`, `LocalBudget`, `FrozenAttempt`, `run_frozen_attempt`, `validate_expected_final_request`, `complete_single_send_with_expected_final_request`, `prepare_single_send_transport`, `run_stream_once`. Avoid a new routing platform.

Artifacts:
- `~/.jcode/scratch/j5-inherited-20260919T2315/inherited-j5.patch`
- `~/.jcode/scratch/j5-owned-checkpoint-20260919T2346/owned-incremental.patch`
- same directory `owned-paths.txt`, `inherited-baseline/`
- ledger repair incremental: `~/.jcode/scratch/jcode-attempt-types-ledger-repair.patch` (now adopted with relocation in `a1a5d213c`).

Concrete current failure: `tool::communicate::tests::communicate_fill_slots_tops_up_to_concurrency_limit`, in `communicate_tests/assignment.rs`, errors `cannot establish spawn route billing for openai before launch` under fresh isolated JCODE_HOME. The test provider lacks unambiguous route metadata under unfinished J5. Do not weaken production admission just to pass a mock. Earlier host-config leakage produced a different Terra-switch error; wrapper isolation fixed that source of leakage, not the new J5 route-contract mismatch.

J4 effort-routing/plugin emission, J6 billed attribution, J7 worker context reduction, J8 classifier-fed routing remain separate planned packets. Do not silently activate auto routing or count response parsing as routing enforcement.

## 6. Begin using cheaper OpenRouter models usefully

The operator now explicitly wants heavier use within the existing spending budget. Next session should turn that into bounded real work, not another broad model-shopping exercise.

1. Read canonical `~/dotfiles/policy/providers.md` and `docs/measurements/2026-09-13-openrouter-deepseek-0731-pilot.md` there. The latter measured useful synthetic extraction/code proposals below one cent but explicitly did not prove unrestricted native writable-worker containment.
2. Confirm current numeric account/key limit and remaining allocation through a sanitized native/account metadata path. Never print key values or whole credentials/config. Earlier `$10/day` and older unlimited-key observations are historical, not current proof. If current effective cap is unknown, ask one consolidated missing-budget question rather than assume the top-up is a per-task cap.
3. Revalidate exact endpoint ZDR/retention/pricing and supported request controls. Preferred already-studied candidate: `deepseek/deepseek-v4-flash-0731` pinned to `open-inference/fp8`. Exact model/provider, no fallback, ZDR true, data collection denied, required parameters, FP8 and unit-price/output caps. A free variant is a separate treatment, not interchangeable with the paid slug.
4. Start with short, public/synthetic or explicitly admitted non-sensitive packets: extraction inventory, test-case generation, narrowly bounded patch proposals, log classification after sanitization, document consistency. Keep tools absent or read-only to a closed file set; reviewed patch proposals are not automatic filesystem authority.
5. Use finite request/concurrency/elapsed/spend limits, reserve before dispatch and retain ambiguous spend. One or two cheap workers is enough initially. Pin effort before dispatch; prior high-reasoning tests consumed the entire output cap without useful content, so don't default every cheap task to high reasoning.
6. Captain/trusted subscription lane reviews every artifact and runs project acceptance. Measure accepted tasks, reviewer time, correction cost, latency and actual billed costs. Promote task categories only from evidence. No quota-filling busywork.
7. Expand writable workers only after the missing native spend/tool/data boundaries are actually enforced and tested. An account cap limits account damage, not task permissions, confidentiality or per-task overspend. Avoid hidden fallback and don't switch an active worker treatment after failure.

Possible first useful cheap packet: compare normalized named-test inventories for one already-identified small extraction using supplied public source snippets, returning a structured discrepancy list with deterministic captain verification. This helps close dirty work without entrusting edits or credentials. Another is synthetic boundary-case generation for the ledger or AGENTS resolver.

Important routing facts retained from prior measured handoff:
- `openrouter/pareto-code` was observed serving premium OpenAI despite exclusions; refused by J2. Do not repeat or adopt it as cheap routing.
- `auto-beta` response task_type parsing exists, but safe request/plugin policy and whole-task accounting are incomplete. Auto routing can supplement harness admission, never replace it.
- OpenRouter provider-layer classifiers are a distinct, potentially billed/admin/data-handling surface. Public docs previously listed five presets; operator saw six. Recheck rather than declaring one count universal. ZDR of inference does not automatically cover plugins/classifiers.
- Free labels do not establish ZDR. Eligible free endpoints can do low-stress, high-oversight work after separate admission. Do not weaken privacy to use free credits.

No OpenRouter live inference, payment, account-policy modification or new provider-default change was performed by this session for the latest request. The next session should begin the admitted bounded usage, not mistake this plan for completed rollout.

## 7. Dirty source reconciliation and native gates

Follow `docs/SOURCE_CLOSEOUT_2026-09-19.md` for ancestry/topic ownership, but its hashes/counts are historical. Upstream history was force-updated earlier; do not bulk-merge or force convergence. No remote ref refresh was performed in this phase.

At the last full inventory before privacy integration:
- module/dependency/wildcard reexports passed;
- after our one-file formatter fix, eight other dirty format files remained;
- production-size: 38 regressions; test-size: two;
- production panic: 97 vs baseline77 across13files;
- swallowed errors:3364 vs3248 across72file regressions.

These are historical pre-privacy counts, NOT current green/red receipts. Logs:
`~/.jcode/scratch/source-closeout-gates-a1a5d213/` and `post-repair/`.
No baseline/gate rules were relaxed. HEAD vs dirty differ materially because extraction shrinks files without yet being accepted.

Close extraction one crate at a time: capture ownership, compare normalized bodies and exact named-test inventory, inspect every staged path, then scoped tests/Clippy/fmt and native gates. Existing Anthropic extraction preserved56 named tests in a representative read-only review; this is not blanket verification of all96+ extraction files. Protocol-axis functional additions are not mechanical movement. Preserve untracked modules rather than accidentally omitting them from a commit.

## 8. Critical host incident, still unresolved

An earlier captain Bedrock test run without explicit JCODE_HOME let a fixture touch:
`/Users/ianalitis/Library/Application Support/jcode/bedrock.env`.
Metadata timestamp matched22:49UTC on September19. Contents, backups and original secrets were NOT read. The user was told and asked for targeted recovery approval. Generic Continue/source-closeout approval has not been treated as approval to inspect/restore credentials.

Source per-test isolation and wrapper defaults are now committed. They prevent recurrence; they do not recover the file. Do not guess original contents, delete the file, read backups, reset passwords or reauthenticate. Ask explicitly for the named file/associated backup recovery scope if the operator wants that handled.

## 9. Tool/runtime gotchas worth retaining

- `scripts/dev_cargo.sh` now isolates local test state; it does not sandbox HOME, Keychain, filesystem or network. Inspect fixture effects. Direct cargo bypasses wrapper isolation.
- Before the cwd fix, an inherited cargo function silently redirected foreign-worktree validation to canonical source. Confirm `cargo locate-project`, invocation and target when testing an exported candidate.
- Test filters can match zero; wrapper exits97 deliberately. Use actual module names: `attempt_caller::tests`, not filename `attempt_caller_tests`.
- Tool timeouts are not passes.600-second background cap may require incremental continuation. Do not cancel active compilation just because an await timed out.
- Do not use optional `maintainer_feedback` to report the disabled feedback tool or send private state upstream.
- Swarm await returned a stale previous-generation ready report while `swarm list` showed snake actively implementing its new task. Compare task content, artifacts and timestamps. Never mark the new task complete from that old receipt.
- Scratch logs are evidence aids, not durable acceptance. Important conclusions are committed in the closeout docs.

## 10. Recommended next sequence

1. Verify final writer-pause addendum, current branch/HEAD/index and dirty ownership. Read native checkpoint; no competing writer.
2. Finish/review/test/commit the bounded AGENTS slice if acceptable. Otherwise checkpoint exact blocker without adding a universal trust/config system.
3. Start one safely admitted cheap OpenRouter extraction/proposal packet under current numeric budget and ZDR controls. Continue trusted integration concurrently with at most one source writer.
4. Fix the concrete J5 mock/admission mismatch only with a reproduction and correct route contract; separately design the missing enforceable pinned-model request bound. Do not claim usable J5 merely because it fails closed.
5. Adopt verified extraction packets and fix actual gate debt without baseline changes. Refresh full acceptance and closeout docs.
6. Report source closeout truthfully. Installation/publishing/host recovery remain separate effects.

### Fresh-session starter

> Read docs/HANDOFF_2026-09-20_SOURCE_CLOSEOUT.md and docs/SOURCE_CLOSEOUT_2026-09-20.md. Continue the bounded native-AGENTS checkpoint with one writer, preserve dirty work, and close concrete acceptance failures. The operator approves beginning cheaper OpenRouter work within the existing spending budget: verify current sanitized budget and exact ZDR endpoint controls, then use a small public/synthetic read-only/proposal packet with deterministic captain validation. Do not silently activate unrestricted paid writable swarms, change defaults, install/restart, publish, or touch the unresolved host Bedrock credential file. Verify current task-generation ownership before adopting any worker receipt.

## Final ownership addendum

At 00:50 UTC the native writer **snake** and reader **swan** were explicitly stopped after checkpoint capture. No outgoing source writer remains. The native checkpoint reports no running background task. The fresh session may assign one new owner after verifying live status.

Read first:
`~/.jcode/scratch/ancestor-agents-20260920T0036Z/HANDOFF.md`.

Native changes are **uncommitted and not accepted**. The artifact includes `pre-edit/`, `incremental.patch` (803 patch lines), and `validation-tree/` with `BASE_HEAD`/`OWNED_SHA256SUMS`. The owner reports 648 additions/60 deletions across the approved three files. Review complexity before adoption.

Validation state:
- rustfmt was applied to the two owned Rust files; owned whitespace check passed;
- isolated committed-tree overlay ran `prompt_tests::agents_md_`: **12 passed**, zero failed (task `1581335s3r`, `test-agents-md-isolated.log`);
- `full_and_split_prompt_builders_select_the_same_ancestor_layers`, `captured_agents_md_keeps_split_prompt_stable_after_file_write`, broader prompt tests and strict Clippy remain **unexecuted/pending**;
- first red attempt's Cargo failure was masked by a `tee` pipeline (task `838276gexe`), so do not claim behavioral red/green proof;
- subsequent canonical attempts exited101 with unresolved import `jcode_attempt_types::SpawnExecutionEnvelope` before reaching prompt tests (tasks `0403420ikn`, `122428cn6h`).

**Important captain correction:** at 00:49 UTC the current canonical `jcode-attempt-types/src/lib.rs` still contained `SpawnExecutionEnvelope` at line166, and provider-core reexported it at line67. Therefore the worker's description of a missing dirty-source definition is not established. Exact manifest/dependency paths, Cargo environment and reused target artifacts must be checked before changing source. Export and canonical builds shared the target cache. That is a hypothesis to investigate, not a proven root cause. Use a correctly isolated validation target if needed, without cleaning unrelated user caches or reinstalling dependencies.

No second native implementation iteration was started after the handoff request. Captured-session stability is still a required test, not a completed acceptance claim.
