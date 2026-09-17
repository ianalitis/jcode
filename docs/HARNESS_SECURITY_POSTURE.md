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

Remaining limits include trusted hook processes, unbounded stderr capture before
truncation, and no process-tree containment guarantee from `kill_on_drop`. Reloadable
configuration and the recursion guard are still not an immutable security boundary.
These require bounded follow-up work, not claims that the first patch solves isolation.

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
