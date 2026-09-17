# Harness security posture and first hardening slice

**Intake:** 2026-09-16. **Scope:** Jcode TUI/CLI execution and a possible bounded Pi worker.
**Status:** Design intake plus a scoped gate-hardening patch, not a claim of sandbox isolation.

## Decision

Treat model output as an untrusted proposal. The trusted runtime owns authorization,
execution, budgets, evidence, and promotion. Reuse Jcode's existing registry and task
lifecycle rather than adding a second orchestrator or a prompt-only policy engine.
Pi remains an optional task-local engine, not an automatic inner-loop replacement.

The first concrete change is fail-closed behavior for **configured** `pre_tool` gates.
No hook still means no hook. Observer failures remain non-blocking. This changes the
previously documented compatibility contract, so operators must review gate scripts
before promoting a build. No host configuration or installed daemon is changed by
this intake.

## What the research establishes, and what it does not

The supplied research, titled *How would you optimally implement this policy into*,
was reviewed against primary sources and the local implementation. Its useful core
is least authority, independent verification, bounded jobs, and runtime enforcement.
Do not adopt its implementation suggestions literally:

| Claim | Disposition |
| --- | --- |
| Models propose and runtimes execute | Adopt. Jcode already projects `Tool` objects into declaration-only `ToolDefinition` values. A model never receives a Rust callable. That alone does not constrain what its shell tool can do. |
| Remove `execute` from Pi's tool objects | Reject as an isolation mechanism. Installed Pi agent-core 0.84.2 validates arguments, invokes `beforeToolCall`, and then calls `prepared.tool.execute`. An adapter may forward authorized intents to an executor, but removing the required implementation is not a process boundary. |
| Never let an agent read the policy/executor source | Separate confidentiality from integrity. Public source must remain safe when known. Keep credentials and sensitive policy data out of worker mounts/context, and prevent workers from modifying or bypassing the running authority. Explicitly authorized harness-development agents may inspect source without gaining deployment authority. |
| Canonicalize paths and allowlist shell commands | Defense in depth only. Check-then-open races, symlinks, hard links, nonexistent write targets, interpreters, subprocesses, inherited descriptors, and alternate tools defeat simplistic checks. Use OS-enforced isolation and descriptor-relative resource access where appropriate, not shell regexes as a jail. |
| Worktrees isolate workers | They isolate code changes, not credentials, processes, filesystem access, sockets, or networks. Do not label a worktree or alternate state directory a sandbox. |
| JSON feature lists and test-first loops enforce correctness | Useful workflow aids, not authorization or a verifier. A worker-editable `passes` flag is only a claim. Preserve independent acceptance evidence and an operator-controlled promotion boundary. |
| Policy checks are microsecond operations and audit can be async | Not established by the research for this runtime. An external hook spawns a process per call. Measure latency and receipt completeness. An async observer alone is neither durable nor tamper-resistant evidence. |

## Observed implementation boundaries

Source baseline inspected: `74e7a4be54ae1db736bbd0e32ae1e6f59b475044`, with substantial
pre-existing uncommitted work. Running harness identified itself as
`v0.84.47-dev (11be474fe)`. Source tests and live-daemon behavior are distinct evidence.

- [`Tool::to_definition`](../crates/jcode-tool-core/src/lib.rs) separates provider
  declarations from host implementations. `ToolContext::resolve_path` joins relative
  paths to the working directory and accepts absolute paths. It is **not** confinement.
- [`Registry::execute`](../crates/jcode-app-core/src/tool/mod.rs) resolves aliases,
  checks a registered session tool policy, runs the configured `pre_tool` gate, then
  invokes the implementation. The inspected baseline skipped absent policies. The
  presence follow-up below denies absent **AgentTurn** policies while preserving
  trusted **Direct** compatibility.
  This is a tool-name boundary, not a filesystem or network capability kernel.
- [`BatchTool::execute`](../crates/jcode-app-core/src/tool/batch.rs) sends each child
  through `Registry::execute` with inherited context. A parent batch approval does
  not replace per-child checks.
- [`hooks.rs`](../crates/jcode-base/src/hooks.rs) previously failed open on invalid
  commands, spawn/wait failures, timeouts, and unexpected exit codes. Its timeout
  covered waiting for output but not writing the input pipe. These are the first
  patch's concrete targets, not hypothetical reasons to rewrite the executor.
- The current hook configuration is reloadable, and `JCODE_HOOKS_DISABLED` suppresses
  hooks for recursion prevention. A same-user shell and mutable configuration must
  not be mistaken for an immutable policy boundary. This patch does not remove those
  escape surfaces or establish protected receipts.

## Target boundary, not yet implemented

```text
Operator-approved task and grants
        |
Trusted Jcode runtime: identity, authorization, budgets, verifier, promotion
        |
Validated intent dispatch, per call and per resource
        |
Restricted execution environment: workspace projection + explicit I/O grants
        |
Jcode or optional Pi worker: task, allowed declarations, bounded tool results
```

The diagram expresses authority, not a requirement for a new daemon. Separate the
restricted execution environment from the control plane only where an actual OS
boundary exists. A socket is not authority by itself: authenticate its peer and bind
requests to server-owned run identity and grants. Worker-supplied session IDs, tool
names, paths, or approval booleans cannot mint authority.

Children must receive the intersection of parent grants, task grants, and platform
limits, including filesystem, network, credentials, time, and spawn budget. Policy
version and immutable request identity must bind an approval to the exact action.
Revocation, resume, retry, batching, deferred MCP dispatch, and background work must
not widen those grants. This is a future acceptance contract, not a verified property
of current swarm construction.

A worker's view contains its task, project code, visible tests, and bounded results.
The control plane owns credentials, sensitive policy data, held-out acceptance tests,
and deployment access. Read, grep, shell, MCP, browser uploads, skills, memory,
compaction, and historical-session retrieval all need coverage. Restricting only
`read` is insufficient. Operator diagnostics must not be retrievable through a
supposedly restricted worker's log or history tools.

## Ordered implementation and acceptance gates

1. **Configured policy failures block execution.** Add red/green regressions for
   missing/invalid hook commands, nonzero and signal exits, deadline expiry, and a
   blocked input pipe. Keep ordinary allow, explicit deny, no-hook, observer, and
   multi-hook behavior covered. Test through the real registry with harmless marker
   effects, including a batch whose parent is allowed but child gate fails.
2. **Bind authorization to immutable run grants.** Audit direct, deferred MCP,
   child-spawn, resume, and background routes before changing policy registration.
   Missing/stale grants must deny in an explicitly restricted execution mode. Prove
   that aliasing, fabricated run identity, and wider child requests cannot escalate.
   Do not silently break trusted direct-call consumers by changing a global default.
3. **Admit one real isolated execution workload.** Use an already-approved native
   isolation option where available. Prove workspace writes and real build/test work
   succeed while host sentinels, executor/control sockets, credentials, and ungranted
   egress remain inaccessible. Include symlink/race, subprocess, inherited-FD, and
   cancellation checks. A blocked network probe alone is not a passing workload.
4. **Protect evidence and promotion.** Verify the candidate revision outside the
   worker-writable surface. Bind local receipts to run/call IDs, policy version,
   decision, artifact revision, and observed checks, without logging secrets or full
   arguments by default. Define receipt-loss behavior explicitly for consequential
   actions. Agent completion claims and writable progress files never authorize ship.

For each slice, freeze the task, route, writable files, predicted behavior, test
command, and stop conditions. Record failing-first evidence where practical, inspect
the diff independently, run the acceptance workflow, and stop at the bounded outcome.
Reuse native todo/session state rather than adding another feature tracker or runner.
Measure accepted outcomes, prevented unauthorized effects, verifier coverage, gate
latency, and recovery cost, not token throughput or self-reported completion counts.

## Evidence and rollout

Validation on macOS arm64 used the real workspace test targets through coordinated
`selfdev test`, without promoting a build:

```bash
scripts/dev_cargo.sh test --offline -p jcode-base --lib hooks::tests:: -- --test-threads=1
scripts/dev_cargo.sh test --offline -p jcode-app-core --lib tool::pre_tool_gate_registry_tests:: -- --test-threads=1
scripts/dev_cargo.sh test --offline -p jcode-app-core --lib tool::tests::registry_execute_ -- --test-threads=1
```

| Requirement | Observed evidence |
| --- | --- |
| Failure must not authorize execution | Failing-first task `158558t173`: 10 passed, 2 expected failures. Old code returned `Allow` on timeout and exceeded the outer deadline while writing stdin. |
| Configured hook failures deny and remain bounded | Independent task `485840raim`: **13 hook tests passed**, covering invalid/missing commands, abnormal exits/signals, timeout, blocked stdin, failed delivery with exit 0, and exit-2 compatibility. The wait-error branch was inspected, not fault-injected. |
| Batch cannot bypass child authorization | Same task: **1 registry test passed**, using real `Registry::execute` and `BatchTool` with a synthetic marker tool. Direct and child effects remained absent, while the parent batch was explicitly allowed. Infrastructure diagnostics did not appear in returned errors. |
| Existing behavior remains usable | Independent task `5408843atv`: **2 existing registry tests passed** for alias-based session policy and ordinary hook allow/deny. The hook suite also covered no-hook behavior, observers, multi-hook denial, and input delivery. |
| Scoped change and reviewed design | Captain inspected the implementation, Sol/high reviewed this boundary document, local document links resolved, and scoped whitespace checks passed. No new dependency, runner, service, or configuration knob was added. |

**16 focused tests passed.** This is component/integration evidence in a dirty
workspace, not a full-suite, Windows/Linux, or installed-daemon acceptance claim.
A new Unix-only test module is wired separately to preserve unrelated test extraction.
The unrelated config-type extraction still carries old fail-open Rustdoc and must be
reconciled when that extraction is integrated; it is not included in this patch.

Runtime promotion, daemon reload, host sandbox setup, and Pi integration remain
separate steps. Before rollout, review the [hook compatibility contract](HOOKS.md):
exit 0 now requires successful stdin delivery, and broken gates no longer silently
permit tools. Source tests do not establish that the live daemon is hardened.

Remaining limits include trusted hook processes and no process-tree containment
guarantee from `kill_on_drop`. The first slice also retained unbounded stderr before
truncation; the bounded-capture follow-up below addresses that memory gap. Reloadable
configuration and the recursion guard are still not an immutable security boundary.
These require bounded follow-up work, not claims that these patches solve isolation.

## Session-policy presence follow-up

The next bounded contract distinguishes three states that must not collapse into
one another: a registered unrestricted policy permits calls, a registered empty
allowlist denies them, and an absent policy denies **AgentTurn** calls. Existing
**Direct** contexts retain their caller-trusted compatibility behavior when no policy
is registered. Direct is not suitable as an untrusted RPC admission mode.

The registry is the first checkpoint, including each batch child. An awaited
`pre_tool` gate creates another checkpoint before implementation dispatch. Deferred
MCP search/call must carry execution context into their policy checks, rather than
interpreting an absent session ID entry as unrestricted access. No missing-policy
error should enumerate the tool catalog, expose policy paths, or log full arguments.

This is presence-check hardening, not immutable grants or atomic revocation. The
existing execution mode is server-supplied context, not a new sandbox or authentication
primitive. A policy can still be replaced under the same session ID. Calls already
inside tool implementations or `McpManager::call_tool` can await further locks,
connection startup, and remote I/O after the dispatch checkpoint. Generation-bound
leases and cancellation/revocation semantics remain later work.

Failing-first task `460403g00w` reproduced four gaps: missing and removed policies
allowed marker effects, the batch child bypassed removal, and fixed MCP dispatch
reached the unconfigured backend instead of denying admission. Two compatibility
cases already passed.

Independent captain task `2728310yiu` passed the seven new regressions plus existing
registry alias/hook, deferred MCP filter, Agent drop/successor, streaming/blocking
native tool-turn, and failed-hook batch tests. Task `3250025g2c` also passed the Agent
clear/new-session lifecycle check. Together these cover **16 distinct focused tests**
(one alias test ran twice). The new manager-lock regression holds the actual lock,
polls dispatch to Pending, removes the registration, and verifies generic denial
when the lock is released. No external MCP server or provider was started by these
focused checks.

The production diff reuses the existing policy map and execution mode. One shared
presence helper covers registry and deferred MCP entry points, without a new grant
object or a redundant second lookup on the no-hook registry path. Touched Rust files
passed scoped rustfmt and whitespace checks. This is source/component evidence,
not installed-daemon or OS isolation acceptance.

Broader library validation is not green: task `3250025g2c` ran all `jcode-base` library
tests with **1366 passed, 17 failed, 2 ignored**. Failures were in auth, browser,
config, platform, provider and provider-catalog tests, outside this patch's touched
implementations. No before-patch full-suite run establishes their cause. The chained
app-core suite did not run after that failure. Independent task `397211i7ni` then ran
`scripts/dev_cargo.sh test --offline -p jcode-app-core --lib -- --test-threads=1`
and passed **1300 tests, 0 failed, 24 ignored**.
No failures were suppressed, unrelated files repaired, or runtime build promoted.
These are affected-library suites, not full workspace or installed-runtime gates.

## Bounded hook stderr follow-up

`wait_with_output` retained the full stderr stream before formatting a short reason.
The follow-up replaces that capture with a fixed-size read loop that retains only
the first **16 KiB of raw stderr** and drains/discards the remainder. Stdin delivery,
stderr draining and child wait run concurrently under the existing deadline. Finite
noisy exit-0 hooks can still allow after full stdin delivery; overflow alone does
not deny a call. Exit 2 uses the retained prefix, trimmed and capped at 2000 UTF-8-safe
bytes, or the existing fallback when the prefix is empty/whitespace. A read error,
wait error or timeout fails closed without returning infrastructure diagnostics.

This bounds retained diagnostics, not total emitted bytes, CPU work, tool-input
size or descendant processes. Lossy UTF-8 decoding and returned reasons also have
bounded allocations derived from the prefix. No detached reader, new dependency,
configuration knob or observer behavior change is introduced.

Failing-first task `066914ujs6` passed 13 existing tests and failed the new prefix
regression: the old implementation returned a sentinel emitted after 16 KiB of
whitespace instead of the fallback. That proves the capture-contract change, not a
measured peak-RSS limit. Independent captain task `436305hsjs` passed **18 hook
tests** and the full app-core library suite (**1300 passed, 24 ignored**). This includes
exact prefix retention, invalid UTF-8/multibyte reason formatting, finite noisy stderr
with large stdin, a noisy deadline stall, and the existing real registry/batch tests.
The stderr-read and wait-error mappings were inspected, not fault-injected. Scoped
formatting and whitespace checks passed. The matched comparison below narrows the
earlier full-base failure attribution but does not produce a green gate. No runtime
promotion is implied.

## Matched base-library comparison

At the operator's request, the original 17 failures were compared using one gitless
scratch source directory. This is a **component-only comparison with unrelated dirty
inputs held constant**, not validation of pristine full-repository revisions. Only
`crates/jcode-base/src/hooks.rs` and `config/default_file.rs` varied: candidate bytes
came from `e7f8405e2a371e810dfad784106ed6f6ecb9c445`, before-patch bytes from
`74e7a4be54ae1db736bbd0e32ae1e6f59b475044` (parent of the first hook patch).
App-core is not a dependency of the tested base-library target. The later
stderr-bound follow-up was excluded from both arms.

Both arms used the same source cwd, Cargo.lock, Rust/Cargo 1.98.0, default features,
fixed build metadata, nine build jobs, non-incremental scratch target, and exact gate:

```bash
scripts/dev_cargo.sh test --offline -p jcode-base --lib -- --test-threads=1
```

Fresh synthetic HOME, JCODE_HOME, XDG directories and TMPDIR were reset between arms.
This deliberately differs from the original ambient-environment run. No credential
stores were copied. Two ignored live tests stayed ignored. Remote cargo and sccache
were off in both arms. These controls do not establish OS sandbox containment.

| Arm | Native task | Passed | Failed | Ignored | Exit |
| --- | --- | ---: | ---: | ---: | ---: |
| Original candidate, ambient environment | `3250025g2c` | 1366 | 17 | 2 | 101 |
| Candidate base files, matched scratch environment | `024130pc9k` | 1372 | 11 | 2 | 101 |
| Before-patch base files, same scratch environment | `258291jwiq` | 1369 | 11 | 2 | 101 |

The matched arms failed the **same 11 test identities with the same normalized
failure classes**, with no candidate-only failure. Ten of the original 17 failures
reproduced in both arms:

```text
auth::codex::tests::multi_account_active_switch_works
auth::cursor::tests::vscdb_missing_key_returns_error
auth::lifecycle::tests::every_model_login_provider_has_explicit_lifecycle_normalization
auth::oauth::tests::basic::save_claude_tokens_preserves_existing_account_metadata
auth::tests::cursor_status_is_available_for_authenticated_cli_session
auth::tests::full_and_fast_auth_status_document_cursor_cli_exception
config::tests::config_env_fingerprint_tracks_every_apply_env_override_var
platform::platform_tests::spawn_detached_creates_new_session
provider::tests::test_same_provider_account_candidates_include_other_openai_accounts
provider_catalog::provider_catalog_tests::every_static_profile_model_has_a_known_context_limit
```

Seven original failures did not reproduce in either matched arm. Their cause remains
unresolved, not proven pre-existing or fixed:

```text
auth::transfer::tests::every_existing_destination_is_refused_without_reading_or_modifying_it
auth::transfer::tests::openai_jwt_expiry_fallback_is_checked_and_persisted_without_source_mutation
auth::transfer::tests::racing_imports_have_exactly_one_winner_and_no_partial_or_leftover_files
auth::transfer::tests::round_trip_only_active_account_leaves_source_and_other_stores_unchanged
auth::transfer::tests::supported_provider_set_and_expiry_rules_are_explicit
auth::transfer::tests::symlink_and_nonregular_destinations_cannot_be_followed
browser::browser_tests::test_paths
```

`registry::registry_tests::cleanup_stale_preserves_live_socket_paths` additionally
failed in both scratch arms but not in the original run. It is not candidate-only;
no root-cause claim follows from its location or the environment difference alone.

The captain independently compared both 2016-entry input manifests, checked the
four varied file hashes against immutable Git blobs, and verified both post-run
source manifests equal their pre-run manifests. Exactly the two named files differ.
The common-input SHA-256 is
`a7c9f208d9385715e2690a87bca25b18eae2f4be729b78f1396e0a87db08c6d9`.
Both test pipelines recorded `test=101 sanitizer=0 tee=0`. Sanitized failure identities
and classes, commands, manifests and runtime metadata remain under
`~/.jcode/scratch/jcode-base-matched-e7-vs-74e-20260917T0307Z/receipts/`.
Setup attempts `796158c6f1`, `828726lbm2` and `893306nrfz` provided no suite result;
absolute scratch paths and a pinned existing toolchain resolved the runner setup.
Compiler/profile warnings, including a nonfatal rust-objcopy stripping failure,
were not suppressed. One preparatory `cargo --version` probe ran before wrapper
action logging was disabled and may have appended an operational rust-actions log
record outside scratch. No live source, configuration or authentication content was
changed by the comparison. This was controlled test input, not OS-level isolation.

**Disposition:** the patch is not required to reproduce ten original failures under
these controlled conditions. Do not generalize that to all 17, claim pristine-revision
acceptance, or mark the full gate green. No unrelated source repair, installation,
publication or daemon promotion occurred. Rollout remains blocked.

## Latest-source project gates

The operator relayed dotfiles corroboration commit `a560926`. The completed matched
comparison was not repeated. Instead, native selfdev task `5707308wrx` tested the
latest working source at `9ceffe4a2f720cc9ebd2516bed0e30b7515188d7`, including the
stderr-bound follow-up. This is **not pristine-revision validation**: the 2016-file
snapshot preserves the existing 157-file dirty diff and nonignored untracked inputs.
In particular, `tool/mod.rs` and `tool/tests.rs` contain preserved unrelated dirty
changes and do not byte-match their committed versions. The other seven files in
this hardening series, including `hooks.rs`, match their `9ceffe4a2` Git blobs.

The gitless snapshot used the existing Rust/Cargo 1.98 toolchain, offline dependencies,
default features for test commands, six build jobs, one test thread, a fresh synthetic
HOME/XDG/TMPDIR per gate, and an explicit environment without inherited credentials.
Remote Cargo, sccache and wrapper action logging were disabled. Each command had a
600-second timeout, recorded its own exit status, and ran independently of prior
failures. This is a local macOS arm64 check, not an OS sandbox or the full CI matrix.

| Gate | Observed result | Exit |
| --- | --- | --- |
| Full `jcode-base --lib` | 1377 passed, **11 failed**, 2 ignored | 101 |
| Full `jcode-app-core --lib` | 1277 passed, **23 failed**, 24 ignored | 101 |
| Root `jcode --lib --bins` | Library: 252 passed, **15 failed**; binaries compiled but execution stopped after library failure | 101 |
| Root `jcode --bins`, separate task `196440ydwg` | 8 passed; two other binary harnesses contained zero tests | 0 |
| `check --all-targets --all-features` | Passed for the default root package and its dependencies | 0 |
| `clippy --all-targets --all-features -- -D warnings` | Passed for the default root package and its dependencies | 0 |
| `cargo fmt --all -- --check` | Passed | 0 |
| `cargo metadata --offline --locked --format-version 1` | Passed | 0 |
| Warning-budget gate tests | 7 passed | 0 |
| Warning budget | **Failed**, 3 warnings against baseline 0 | 1 |
| Code-size ratchet | **Failed**, oversized production files grew | 1 |
| Test-size ratchet | Passed | 0 |
| Panic-prone usage ratchet | **Failed**, total 77 to 84 | 1 |
| Swallowed-error ratchet | **Failed**, total 3248 to 3338 | 1 |
| Crate dependency boundaries | Passed | 0 |
| Wildcard re-export ratchet | Passed | 0 |
| SDK parity filter | 3 passed; other selected harnesses had no matching tests | 0 |
| Native module-file check | Passed, read-only in the live Git repository | 0 |

Cargo test/check/clippy commands used `scripts/dev_cargo.sh` with `--offline`;
tests used `-- --test-threads=1`. The module check requires Git and therefore ran
outside the gitless snapshot. Task `5707308wrx` exited 1 as the aggregate failure
signal; the table preserves each underlying gate status. The binary-only follow-up
executed previously skipped targets, not a rerun of the failed library. Nonfatal
rust-objcopy stripping warnings were retained, and no budget baseline was updated.

The latest base failure identities exactly equal the eleven in both earlier matched
arms. The five additional passing base tests are the stderr-bound regressions. This
is only an identity comparison with saved results, not another matched experiment.
At this checkpoint, app-core's 23 failures and the root library's 15 failures were
**unattributed** in this changed test environment. They include socket/startup/
communication and CLI fixtures. The subsequent environment diagnosis below narrows
that uncertainty without rewriting these original failed receipts. The earlier
ambient app-core 1300-pass receipt alone did not establish that the latest failures
were regressions or that environment changes caused them. Test failure names and
counts were retained while panic operands were omitted; those receipts alone did
not establish root causes.

The captain checked every recorded input against both the snapshot and live source
before writing this receipt. All 2016 inputs were unchanged, and the final snapshot
manifest equals its pre-run manifest, SHA-256
`c9a28763693138cf150dff876ebfe7a87713c2731b129e172cf3810db6835bf0`.
Commands, explicit environments, individual results, sanitized logs, input manifests
and per-file Git comparisons are under
`~/.jcode/scratch/jcode-latest-9ceffe4a2-20260917T0341Z/receipts/`.

**Disposition:** latest full-base and local project evidence is now recorded, but
acceptance remains failed and promotion blocked. No failure repair, dependency
installation, budget suppression, reload or promotion was performed. Release builds,
other platforms, full-workspace tests, live provider/daemon acceptance and the CI
unused-dependency installation step were not run. Cross-project cooperation remains
operator-relayed. Do not repeat the completed before/after comparison to close these
remaining failures; any further diagnosis needs its own bounded scope.

## Verification environment diagnosis

The next bounded investigation found two limitations introduced by the verification
setup, without changing product code or repeating the before-patch comparison:

- **Socket path length:** task `257147cgvk` ran the exact
  `server::socket_tests::connect_socket_preserves_refused_socket_path` fixture in
  long/short/long temp roots. The original 82-byte temp root yielded a 104-byte
  example socket path; the short root yielded 64 bytes. Holding source, command and
  other environment values fixed, only `TMPDIR` and its `JCODE_BUILD_TMPDIR` override
  changed. Outcomes were **fail/pass/fail**, with exit codes **101/0/101**. Both failures
  explicitly reported `InvalidInput: path must be shorter than SUN_LEN` at listener
  binding. The diagnostic task's exit 0 means the expected reproduction completed,
  not that its deliberately failing phases passed.
- **Git-dependent fixture:** after short-temp validation, the sole app-core failure
  was `tool::bash::tests::repository_commands_export_a_logged_cargo_function`.
  Task `520328gtss` confirmed its exact `test runs inside the jcode repository`
  expectation failed in the gitless snapshot. Source tracing shows
  `find_repo_in_ancestors` calls `is_jcode_repo`, which requires `.git` to exist.
  No fake Git marker was added and no assertion was removed. The full suite then
  passed in the actual checkout, with a fresh synthetic HOME and short scratch TMPDIR.

Task `3429777oiv` first revalidated the unchanged snapshot using short temp paths
and fresh synthetic homes: base **1378 passed / 10 failed / 2 ignored**, app-core
**1299 / 1 / 24**, root library **262 / 5 / 0**. Every command still exited 101.
This suite-level run also changed HOME/XDG paths, so the single-fixture controlled
proof must not be generalized into individual causal proof for every disappeared
failure. The original receipts remain intact.

Final native real-checkout checks, with source code unchanged from `9ceffe4a2` and
HEAD `f6eec70da` adding only the prior receipt, were:

| Suite | Task | Result | Exit |
| --- | --- | --- | --- |
| Full app-core library | `520328gtss` | **1300 passed, 0 failed, 24 ignored** | 0 |
| Full base library | `640353jz71` | 1378 passed, **10 failed**, 2 ignored | 101 |
| Root library (`--lib --bins`) | `640353jz71` | 262 passed, **5 failed**; binary execution stops at library failure | 101 |

The base and root residual failure identity sets match their short-temp snapshot
runs exactly. The base ten are the same identities reproduced in both earlier
matched revisions. The five root failures concern CLI auth/lifecycle, provider
wiring and auto-poke fixtures; no new root-cause or regression attribution is made.
The previous independent 8-pass binary receipt remains separate, not newly rerun.
The four failed budget gates also remain open.

All 2016 recorded live inputs had identical pre/post hashes for these native checks,
and all snapshot inputs remained unchanged. Test logs and result hashes were
independently checked. Under the same scratch artifact root used above, see
`socket-diagnosis/`, `short-temp-verification/`, `git-fixture-verification/`,
`live-remaining-verification/`, and `environment-diagnosis-summary.json`.

**Operational lesson:** native socket tests need a short actual temp path, including
room for fixture-generated directory and socket names. Git-dependent repository
fixtures need the real checkout, not an invented Git marker in an export. Retain
fresh synthetic auth/home state and scratch build outputs without mistaking them
for OS isolation. Do not promote these one-off diagnostic scripts into a new test
framework. No product repair, baseline update, source rollback, shared-daemon reload
or promotion occurred. Overall acceptance and promotion remain blocked by the
remaining failures; the corrected setup is not a pristine-revision or full-CI claim.

## Cross-project cooperation disposition

The operator-forwarded dotfiles cooperation packet (`284ae35`, 2026-09-17) is a
proposal and handoff, not an approval channel or an installed feature. Its ordered
approach is sound: explicit native Jcode peer admission, then bounded automatic
notifications, then optional Pi task/result exchange. Keep the present cross-swarm
restriction until the prerequisite below is proven. The current concrete workflow
uses an operator-relayed brief and immutable revision/test receipt, with one source
writer and no dotfiles writes from this session.

The source review found no existing state that safely represents peer consent:

- [`VersionedPlan.participants`](../crates/jcode-plan/src/lib.rs) records plan-update
  recipients. [`require_plan_driver_swarm`](../crates/jcode-app-core/src/server/comm_control.rs)
  also permits deep-plan participants to drive dispatch. Reusing participation for
  peer admission would mix notification consent with task authority.
- [`CommMessage`](../crates/jcode-app-core/src/server/client_comm_message.rs) enforces
  same-swarm targets, but accepts a supplied sender session ID. The
  [tool transport](../crates/jcode-app-core/src/tool/communicate/transport.rs) opens
  a one-shot socket. These paths do not establish a separate operator principal or
  a server-bound peer sender. This is static source evidence, not a live spoofing test.
- Ordinary message delivery can wake a turn or queue an interrupt. The notify-only
  branch avoids those effects, but the response is only `Done`; it does not prove
  recipient processing. A peer brief must not inherit the default DM wake behavior.

**Prerequisite:** distinguish operator confirmation from model-authored requests,
and bind each peer sender to server-owned live identity. Session IDs, project paths,
swarm roles, plan participation, client labels and `approved: true` are not proof.
A candidate local-TUI confirmation must be one-use, expiring and bound to the exact
connection, runtime generation, pair, declared scope and lifetime. It must not be
available through lightweight tool sockets, model context, logs or history. Even
that is insufficient against an unrestricted same-user shell or UI-automation tool
that can impersonate the operator. That threat requires a protected approval
channel or execution isolation, not a stronger prompt. Remote/gateway approval
needs its own principal definition rather than inheriting local assumptions.

Only after that prerequisite, assess an ephemeral two-session grant on the existing
daemon. Reuse native session metadata and event delivery, not a merged swarm or a
second coordinator. Return pair-only metadata, attribute peer text as untrusted
input, and grant no task ownership, tool permissions, credentials, spawning or
promotion. Expiry, revocation, endpoint replacement and daemon restart must revoke
access. Bound message bytes, rates, queues and deduplication state. Distinguish
accepted, queued, attachment-delivered and completed receipts. Do not claim exactly
once processing from a socket write or queue acceptance.

The first acceptance fixture is two synthetic opted-in sessions in disposable
project roots plus a third ungranted session. Prove consent provenance, sender
binding, third-peer denial, stale/replayed/oversized rejection, bounded backpressure,
revocation during concurrent delivery, and no model turn or authority transfer.
Preserve ordinary same-swarm behavior. Only then add deduplicated status, explicit
scope-conflict and completion metadata notifications; no automatic transcript or
completion-body sharing. Pi stays a later bounded artifact exchange, never SDK
user-message injection or unrestricted daemon access.

This disposition is reviewed design, not implemented peer admission. No debug gate
was enabled, no socket workaround used, and no cross-swarm peer was contacted. Full
app-core tests at `397211i7ni` cover existing behavior, not the proposed feature.

## Sources checked

- [Anthropic: Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
  (2025-11-26): incremental features, progress artifacts, and end-to-end verification.
  Its JSON preference is an observed prompting heuristic, not tamper protection.
- [OpenAI: Harness engineering](https://openai.com/index/harness-engineering/)
  (2026-02-11): repository knowledge, enforceable constraints, observable applications,
  and feedback loops. Its throughput anecdotes are not a local performance result.
- [Lilian Weng: Harness Engineering for Self-Improvement](https://lilianweng.github.io/posts/2026-07-04-harness/)
  (2026-07-04): workflow, durable artifacts, inspectable jobs, and evaluation research.
- [Pi agent loop, upstream source](https://github.com/badlogic/pi-mono/blob/main/packages/agent/src/agent-loop.ts):
  moving reference only. Execution behavior was also checked against installed
  `@earendil-works/pi-agent-core` **0.84.2**, rather than assuming `main` matched it.

Secondary aggregations, social posts, and the unrelated healthcare-policy citation
in the supplied document were not used as implementation evidence. No downloaded
instructions were treated as authority over local repository policy.
