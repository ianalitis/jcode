# Source closeout review and bounded OpenRouter attempt

## Operator correction and renewed execution, 01:15 UTC

The operator reconfirmed that separate OpenAI OAuth credits cover usage after the
included weekly allowance is exhausted. The earlier capacity stop below was too
broad. The existing dotfiles economy plan's acceptance correction already states
this. A 100% weekly meter alone is not a stop for explicitly requested bounded
OAuth work. Actual auth/quota failures still close a frozen treatment. Product
credit debit, OpenRouter invoices and API-equivalent estimates remain distinct.

One `openai-oauth:gpt-5.6-sol/high` writer was assigned the three-file AGENTS repair,
with two implementation iterations maximum. No Claude or paid API substitution
was made. Token/agent economy is now an explicit acceptance objective: compact
packets, deterministic checks first, and cost plus reviewer time per accepted
change, not nominal token price or raw worker throughput.

A separately frozen second DeepSeek free/no-reasoning packet added structured
JSON and receipt-before-grading. Three offline receipt/parse/redirect checks
passed. Task `04433067a3` received the exact requested model from OpenInference,
500 prompt and 436 completion tokens, zero reasoning tokens, in 12.56 seconds.
It produced valid JSON but only **6/12 complete boundary cases were correct**.
The artifact was rejected, with no retry or automatic escalation.

The immediate billing lookup failed. One later read-only reconciliation returned
an explicit **$0 generation invoice** for
`gen-1789867045-ujT6HCBsJIWPKJEye7yI`. The second $0.01 reservation is settled at
zero. The first request's $0.01 ambiguous reservation remains held. Scratch
`free-boundary-packet-v2.py`, its JSONL receipt and
`free-boundary-v2-reconciliation.json` retain the evidence. This is a rejected
screening result, not a successful cheap-worker rollout. Native no-tool dispatch
and packet/context reduction take priority over further toy benchmark retries.

Observed 2026-09-20 00:52–01:01 UTC, canonical branch `jcode/ci-format-baseline`, starting HEAD `749bdc88a`. This is a partial review receipt, not source acceptance or runtime promotion.

## Ownership and treatment

The outgoing writers were absent from live swarm status. The index was empty and 296 dirty/untracked paths remained. No blanket staging, source rewrite, new branch/worktree, installation, daemon restart, publishing, provider-default change, payment, or credential recovery occurred.

Native usage reported OpenAI's weekly window at 100% and Claude's five-hour window at 100% at 00:52 UTC. No premium worker was dispatched, no account failover was attempted, and no paid premium substitute was used. The current captain performed read-only source review and isolated validation. New security-sensitive implementation remains assigned to the Sol/high lane when admitted capacity is available, not silently transferred to a cheap writable worker.

## Ancestor-AGENTS validation: pending tests closed, source not accepted

The three source files exactly matched the prior `ancestor-agents-20260920T0036Z/validation-tree/` overlay, whose `BASE_HEAD` is `f1d193b309930be7555ee9a2a841a285cabd39c7`. The pre-edit ownership files were empty, so these three paths had no earlier unrelated dirty delta at packet start.

A new target and synthetic HOME were used under `~/.jcode/scratch/source-closeout-20260920-review/`. Existing isolated Cargo cache and installed rustup toolchain were used, with `--locked --offline`, four build jobs and wrapper-isolated test state. No dependency installation or trust/configuration mutation was performed.

- `scripts/dev_cargo.sh test --locked --offline -p jcode-base --lib prompt_tests:: -- --test-threads=1`: **42 passed**, zero failed, 1,358 filtered out, task `710881xyu7`.
- Both `captured_agents_md_keeps_split_prompt_stable_after_file_write` and `full_and_split_prompt_builders_select_the_same_ancestor_layers` actually executed and passed.
- Owned `git diff --check` passed.
- Strict `jcode-base --all-targets` Clippy failed first in the committed `jcode-core/src/stdin_detect.rs:333` range-index loop. This was not a prompt lint pass.
- Initial isolated-HOME startup task `642818p91s` failed because mise regarded its host configuration as untrusted. Selecting the already-installed rustup executable for the validation process avoided changing mise trust. This startup failure was not counted as a test failure or pass.
- The isolated target compiled the prior overlay successfully. This does not establish the cause of the earlier canonical `SpawnExecutionEnvelope` import failure. No source definition was removed or invented to work around it.

### Acceptance blockers

1. **Native size gate:** the candidate makes `prompt.rs` a new oversized production file at **1,306 LOC**, exceeding the existing 1,200 limit. No baseline or rule was relaxed. Reduce within the approved slice or obtain explicit scope for a small existing-convention module extraction. Do not add a general trust framework.
2. **Repository-root provenance:** `git_repository_root` inherits `GIT_DIR` and `GIT_WORK_TREE`. In a synthetic temporary repository, `git -C repo/child rev-parse --show-toplevel` with `GIT_DIR=repo/.git` and `GIT_WORK_TREE=parent` returned the parent outside the ordinary repository root, and the current `starts_with` guard accepted it. This is a deterministic Git-call reproduction, not an executed resolver unit test. Add the actual resolver regression before changing the boundary. The approved packet explicitly deferred outer-root inheritance.
3. **Bounded read review:** `prompt.rs:1052` uses `std::fs::read` after path metadata validation. Its second length check rejects oversized content only after allocation/read. A concurrently growing or replaced file is not bounded by that read, and canonicalization/metadata/read are separate path lookups. Static finding only: a race fixture was not executed. At minimum use a bounded read of limit+1 without safety-text truncation, and explicitly settle the regular-file/containment race contract before asserting hostile concurrent replacement protection.

Complexity review: existing tuple snapshot, sha2/hex and direct ancestor traversal are reused. No lifecycle watcher or configuration framework is needed. The 1,306-line size failure prevents accepting the three-file implementation merely because the 42 existing tests pass.

## Separate inherited core lint candidate: not accepted

The dirty tree already contains a two-line iterator correction in `crates/jcode-core/src/stdin_detect.rs`. Only that source file was copied into the disposable overlay to examine the existing lint fix. Canonical source and the separate dirty `stdin_detect_tests.rs` were untouched.

- Candidate formatting and whitespace passed.
- Full core library test task `956271c95e`: **38 passed, one failed**, `id::tests::avoiding_allocator_uses_every_available_identity_before_reuse` (`allocator reused an available identity`). The following Clippy command did not run because the test command failed.
- Narrow `stdin_detect` task `0348427nec`: **one passed, one failed**, `test_own_process_not_reading_stdin` reported `Reading`. Following Clippy again did not run.
- These failures have not been attributed by a baseline-versus-candidate run. Do not call them proven unrelated, suppress their assertions, or commit the candidate as validated. No further core implementation iteration was made.

The prior validation overlay now contains this extra core source file. Do not reuse its original `OWNED_SHA256SUMS` as a claim about the entire overlay. The initial 42-test receipt preceded this extra overlay.

## OpenRouter: budget verified, one free task attempted, none accepted

Sanitized live `/api/v1/key` metadata:

- limit: **$5.00**, reset: null;
- remaining: **$4.896949018**;
- usage: **$0.103050982**, daily usage: **$0**.

Native usage also reported about $106.38 account balance. The key limit, not the account top-up, was used as the numeric guardrail. No credentials or complete account configuration were printed or persisted.

**Paid treatment: `deepseek/deepseek-v4-flash-0731`, OpenInference FP8, reasoning disabled. STOP before inference:** current exact paid-model endpoint metadata contained no `open-inference/fp8` endpoint. No fallback request was sent.

**Separately admitted treatment: `deepseek/deepseek-v4-flash-0731:free`, `open-inference/fp8`, reasoning disabled.** The exact free endpoint was separately found in the model endpoint list and `/api/v1/endpoints/zdr`, with zero prompt/completion prices, FP8 and required parameter support. OpenInference's current public privacy policy promises transient-only content processing, no retained inputs/outputs and no training. This is published provider evidence, not independent verification of internal handling or a blanket claim about aggregator metadata.

One synthetic boundary-vector task was dispatched. It contained no repository files, inherited instructions, user transcript, tools, plugins, personal data or secrets. It requested expected counters for 12 abstract byte/count/UTF-8/duplicate boundary cases, checked by a deterministic captain oracle.

Frozen controls:

- one request, concurrency one, no retries, fallback, redirects or proxy;
- exact model/provider and FP8, `zdr=true`, `data_collection=deny`, required parameters;
- prompt and completion unit-price ceilings zero;
- 2,035 request bytes (8 KiB bound), 2,048 maximum output tokens;
- 90-second whole-packet alarm, 45-second inference socket timeout;
- **$0.01 durable reservation before dispatch**, retained on ambiguous failure.

Task `78764557ga` stopped after 9.769 seconds with `JSONDecodeError`. **No generated artifact was accepted and no repeat was sent.** The probe did not persist the raw response/generation ID before JSON parsing, so the exact parse layer and model output are unavailable. Do not describe this as a demonstrated model-answer formatting defect. This is also a probe-observability defect: future separately admitted packets must save sanitized response identity/status before grading, while excluding secrets and arbitrary error bodies.

A read-only key query after the stop returned exactly unchanged usage and remaining limit. That is not a per-generation invoice. Keep the $0.01 reservation ambiguous, leaving **$4.886949018** after that local reservation, absent other callers. No claim of accepted-task throughput, successful rollout, or paid writable-worker containment is supported.

Scratch artifacts:

- `~/.jcode/scratch/source-closeout-20260920-review/free-boundary-packet.py`
- sibling `free-boundary-packet.jsonl` (reservation and sanitized stop)
- isolated Cargo target/home/state in the same review directory.

Public admission sources checked:

- `https://openrouter.ai/api/v1/models/deepseek/deepseek-v4-flash-0731/endpoints`
- `https://openrouter.ai/api/v1/models/deepseek/deepseek-v4-flash-0731:free/endpoints`
- `https://openrouter.ai/api/v1/endpoints/zdr`
- `https://www.openinference.ai/privacy`

## Refreshed native gates and J5

Task `885618g7ky` ran existing check scripts without updates:

| Gate | Observed result |
| --- | --- |
| Module declarations | Pass |
| Dependency boundaries | Pass |
| Wildcard reexports | Pass, total 17 |
| Production size | Fail, 38 path regressions |
| Test size | Fail, two path regressions |
| Panic-prone usage | Fail, 95 versus baseline 77, 12 path regressions |
| Swallowed errors | Fail, 3,276 versus baseline 3,248, 72 path regressions |

Formatting was checked only for the reviewed prompt paths and core candidate, not the entire workspace. No full-workspace green claim.

Read-only J5 localization confirmed `DelayedTestProvider` supplies name `test` and no explicit route metadata, while `spawn_route_policy` correctly refuses ambiguous `openai` billing. The handoff's failure remains unclosed. No production admission weakening, mock workaround, new bill-bound design or J5 implementation was performed.

## Next bounded source packet

Resume the Sol/high source lane only with current admitted capacity. Priorities remain:

1. Add the resolver root-environment regression, settle bounded-read/containment behavior, and get the AGENTS slice under the unchanged size gate. Preserve the passed captured-snapshot contract.
2. Attribute core test failures against the original source before accepting its inherited lint/test delta.
3. Reproduce the concrete J5 fixture failure under isolated state and correct the fixture's explicit route contract without weakening production admission.
4. Close verified extraction packets individually. No unrestricted metered writable swarm until the missing native request/tool/data/spend boundaries pass.

Host Bedrock credential-file recovery remains outside this request. No active compiler or writer is needed to preserve this checkpoint.

## 01:45 UTC continuation: exact-candidate evidence

- Accepted `dd547d8f4`: two-file core platform lint correction. Serial baseline and candidate each passed 38 tests and failed the same `test_own_process_not_reading_stdin`. Candidate strict core Clippy passed. Evidence: `~/.jcode/scratch/core-lint-ab-0132/`. This is not full core test acceptance.
- Accepted `da56ca22f`: inherited terminal-launch macOS needless-return correction. All 29 terminal-launch tests, strict all-target Clippy and changed-file formatting passed in the isolated captain export.
- Corrected ancestor AGENTS snapshot passed all **44** prompt tests. All five candidate/core hashes matched the tested snapshot. The generic silent error helper was rejected and replaced with bounded visible diagnostics or logging. Source writer is frozen on the three AGENTS paths.
- Base strict Clippy remains **failed**, identically for committed baseline and candidate: `auth/cursor.rs:338` needless return, `auth/lifecycle.rs:231` collapsible if, and `model_usage.rs:133` type complexity. No lint allowances or budget changes. AGENTS remains uncommitted pending closeout of this gate, not claimed as a fully green package. Evidence: `~/.jcode/scratch/agents-captain-0127/{correction-hashes.json,correction-tests.log,terminal-tests.log,terminal-clippy.log,terminal-base-clippy.log,baseline-base-clippy.log}`.
- The separately frozen structured boundary packet produced only 6/12 completely correct cases and was rejected. Later authoritative generation reconciliation reported actual cost zero. The original parse-failure reservation remains ambiguous.
- A distinct public allocator proposal used the admitted free DeepSeek/OpenInference FP8 endpoint, medium reasoning, no tools, at most 8 KiB input and 2,048 output tokens. Its single request timed out at 90 seconds without a response identity. No retry, hidden substitution, or source integration. A second $0.01 reservation remains ambiguous, **$0.02 total unresolved local reservations**. This is not an invoice or a refreshed account balance. Artifacts: `~/.jcode/scratch/source-closeout-20260920-review/public-allocator-proposal.{py,jsonl}` and `free-boundary-v2-reconciliation.json`.
- Zero cheap-model artifacts accepted so far. Do not promote the free route based on price alone. Cost per accepted change, including captain review and timeout cost, remains the goal. Native writable metered swarms are still not admitted.
- Sole Sol/high writer receives the next bounded J5 fixture packet, two test paths only and at most two iterations. It must reproduce the named fill-slots failure and supply truthful fixture route identity, never weaken production billing admission. No runtime promotion, provider-default changes, push, or host credential recovery.
