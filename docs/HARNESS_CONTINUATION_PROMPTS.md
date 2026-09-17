# Fresh-session launch prompts: harness continuation

**Companion plan:** [HARNESS_CONTINUATION_PLAN.md](HARNESS_CONTINUATION_PLAN.md).
**Current evidence receipt:** `7da67dbe8`, not a runtime-promotion receipt.

These are prompts for the operator to select and hand to **fresh sessions**. Preparing,
reading or forwarding this file does not execute them, select a provider, authorize
source writes, or grant cross-project messaging. Do not launch every role at once.
The other agent prepares the dotfiles-owned handoff separately.

## Launch order

1. Start **A**, the source captain, in the existing Jcode checkout. Select an admitted
   `openai-oauth:gpt-5.6-sol` / `high` treatment for new self-dev work. If unavailable,
   stop rather than falling back. Do not change an already-running treatment.
2. A starts with **B**, the narrow config diagnosis, itself or via one authorized
   reader. **C** can be the second reader if independent budget triage is useful.
   Admission, fresh usage checks and concurrency limits still apply.
3. Use **D** only after the operator has approved a concrete repair and filled all
   required fields. Keep exactly one source writer.
4. Use **E** for independent review. A integrates and owns the result. Use **F** only
   as a separately selected read-only design task after immediate blocker work.
5. Relay **G** to the dotfiles owner through the operator. No socket or SDK workaround.

Each STOP report starts with the actual frozen model/effort. A model name in a prompt
is not proof that the session runs that route. Check session metadata and current policy.

## A. Fresh source captain

Paste this into a fresh session in `/Users/ianalitis/.jcode/source/jcode`:

```text
You own continuation of the Jcode TUI/CLI harness source work. First read current
AGENTS.md, CONTRIBUTING.md, ~/AGENTS.md, and the applicable dotfiles upstream/provider
policy. Then read docs/HARNESS_CONTINUATION_PLAN.md and the latest-source/environment
sections of docs/HARNESS_SECURITY_POSTURE.md. Use native todo state, not another runner.

My immediate task is bounded diagnosis and planning of the remaining blockers. Start
with D1/B07, the config environment-fingerprint guard, unless I explicitly select a
different ticket. Do not start broad repairs or run the entire verification catalog.
A source repair requires current operator scope covering the named writable files.
If that scope is absent, return one consolidated minimal repair packet after diagnosis.

Verify current HEAD/index/dirty-input identity first. The last evidence receipt was
7da67dbe8; product code last changed at 9ceffe4a2. At handoff the branch was
jcode/ci-format-baseline, with 157 unrelated tracked dirty files (+2247/-53277) and 113
nonignored untracked files. These are not disposable or yours to stage. In particular,
tool/mod.rs and tool/tests.rs contain unrelated dirty hunks. Do not reset, stash,
clean, switch branches, create worktrees or sync as startup work.

Latest native results: app-core 1300 passed / 0 failed / 24 ignored; base 1378 passed / 10 failed / 2 ignored;
root library 262 passed / 5 failed / 0 ignored. Four budget gates remain failed. Prior check/Clippy/format passed;
none of this is pristine-revision, full-CI or installed-daemon acceptance. Promotion
stays blocked. Read the exact receipt before claiming any independent verification.

Do not repeat the completed e7f8405-vs74e7a4b comparison, the already-proven socket
listener experiment, or the superseded three-test packet. Only one listener fixture
has individual paired causal confirmation. Do not infer the other two were paired.

Use existing coordinated selfdev test only for a selected, inspected fixture in an
explicit credential-free synthetic environment. Validate an absolute short scratch
TMPDIR and JCODE_BUILD_TMPDIR, including final socket suffix length under 104 bytes on
this macOS API. Use the actual checkout for Git-dependent tests. Fresh HOME/JCODE_HOME
and offline Cargo are not a security sandbox. No real auth stores, keychain, provider
calls, live daemon sockets or GUI operations. Preserve exit codes and safe failure
categories, never credential-bearing assertion operands or inherited env dumps.

Prefer one source writer and at most two independently useful readers. Check native
model availability and fresh quota evidence before optional delegation. Workers do not
choose successors or widen scope. Do not switch model/provider/account after a stop.
Dotfiles ownership and cooperation stay manual; do not contact cross-swarm peers by a
socket/debug/SDK workaround or edit their repo.

Deliver: current input/ownership checkpoint, the D1 diagnosis with exact source and
observed evidence, a minimal named-file repair proposal if warranted, and a concise
operator-relay receipt. Preserve unrelated work. No install, auth/provider change,
budget reset, test suppression, push, publication, shared-daemon reload or promotion.
```

## B. Bounded config-fingerprint investigator

Use within A's admitted scope, or as an operator-selected fresh diagnostic session.
**Suggested new treatment:** Sol/high. **Product writes:** none.

```text
Task D1/B07 only: diagnose
config::tests::config_env_fingerprint_tracks_every_apply_env_override_var.
Read docs/HARNESS_CONTINUATION_PLAN.md, current repo policy and the packet from the
source captain. Do not assume an inherited transcript or mutation authority.

Readable source scope: crates/jcode-base/src/config.rs, config_tests.rs, config/**,
crates/jcode-config-types/**, and their direct configuration definition references.
Read Cargo.toml/Cargo.lock, scripts/dev_cargo.sh, .github/scripts/run_with_timeout.py,
and the handoff's evidence/metadata as needed to identify the exact runtime context.
If the actual implementation requires a wider source area or a product decision,
name it and stop instead of expanding the packet. Do not inspect host config/auth.

Writable scope: only a new, non-overwriting scratch receipt and scratch build/test
artifacts inside the operator-approved synthetic environment. No repository edits,
commits, branches, worktrees, dependency changes, live daemon work or provider traffic.
Use the existing toolchain. Record revision and relevant dirty-file hashes.

Read the guard and environment override implementation first. Name a falsifiable
hypothesis about key coverage, not values. If the fixture's effects fit the approved
synthetic scope, run this exact selector through coordinated selfdev test:

scripts/dev_cargo.sh test --offline -p jcode-base --lib config::tests::config_env_fingerprint_tracks_every_apply_env_override_var -- --exact --test-threads=1

This command is an inner command, not permission to inherit live credentials. Apply
the plan's validated real-checkout cwd, fresh HOME/JCODE_HOME/XDG, short scratch
TMPDIR/JCODE_BUILD_TMPDIR, offline toolchain and exit-preserving receipt rules.

Maximum two exact test invocations. No full-suite rerun, comparison to the old patch
baseline, source fix, skipped assertion or new config knob. Expected reproduction
failure is evidence; an unexpected effect or unavailable safe fixture stops the task.

Return: actual frozen route/effort, input identity, exact contract, source locations,
command/task/exit/count, safe failing phase, evidence paths/hashes, and whether the
smallest correction belongs in production coverage or a stale fixture. Name proposed
writable files and predicted red-to-green effect, but do not implement. Never print
operator environment values, tokens or unsanitized assertion operands.
```

## C. Independent budget and ownership reviewer

**Suggested new treatment:** Sol/high. **Default execution:** read-only source/log review.

```text
Task Q1: classify the four failed project budget gates without fixing or resetting them.
Read current policies and docs/HARNESS_CONTINUATION_PLAN.md. Work in the existing
Jcode checkout. Do not become a second writer or try to make the repository green.

Read the saved warning-budget, code-size, panic-budget and swallowed-errors logs under
/Users/ianalitis/.jcode/scratch/jcode-latest-9ceffe4a2-20260917T0341Z/receipts/.
Check their result JSON/log hashes. Inspect the corresponding scripts/baselines and
only the source diffs necessary to classify reported paths. Scope is the four named
gates and ownership, not a general repository audit. Do not read auth stores or env.

Separate (a) the hardening commits 2fe37f78b/e7f8405e2/9ceffe4a2, (b) unrelated tracked
or untracked dirty work, and (c) uncertainty. A dirty path is not proof of authorship.
Do not infer a contributor's authority from a session label. Conflicting ownership
is a stop/coordination item. Preserve current source exactly.

No --update flags, warning suppression, lint allows, scanner exclusions, dependency
changes, test rewrites, commits, installations, full-suite reruns or live effects.
Existing evidence should normally suffice. If a stale result requires execution,
return the exact proposed gate and input mismatch to the captain rather than running
an unbounded build. Scratch-only receipt writes are allowed within the packet scope.

Deliver one matrix: gate, exact failed limit/path, current diff provenance, confidence,
owner coordination needed, smallest separately scoped repair candidate and acceptance
command. Distinguish warning-budget default-feature behavior from all-feature Clippy.
Do not claim the four failures were caused by this hardening or are pre-existing
unless actual evidence establishes it. Promotion remains blocked.
```

## D. Operator-approved implementation packet template

**Do not execute with placeholders.** The operator or authorized captain must provide
an actual authorization reference and named scope. The template itself grants nothing.
For authentication, concurrency, security or self-dev work, use an admitted Sol/high
writer. A reviewer does not expand the writer's scope.

```text
Task ID: <one concrete D1/D2/D3/D4/Q1-derived ticket>
Frozen actual route and effort: <provider-qualified route / effort>
Operator authorization reference: <actual instruction covering checkout and files>
Checkout: /Users/ianalitis/.jcode/source/jcode
Base HEAD + relevant dirty-input hashes: <verified current values>
Permitted readable paths: <finite source/fixture/evidence set>
Permitted writable files/hunks: <exact list, including ownership constraints>
Predicted observable effect: <one behavior and expected test result>
Reproduction evidence: <command/task/exit, safe failure phase, artifact>
Exact focused validation: <complete command and selected tests>
Compatibility acceptance: <specific preserved behaviors and commands>
Integration gates: <bounded relevant full suite/quality checks>
Environment: <validated synthetic resources, actual cwd/toolchain, short socket paths>
Reviewer and receipt destination: <source captain or independent reader>

Do not begin if any field is missing or operator authority does not cover the work.
Read current repository/global policy. Preserve unrelated changes and any red gates
outside this task. Do not manufacture a baseline by changing live source revisions.

Make one minimal patch for the stated contract. Run the prescribed validation and
compare predicted versus actual outcome. If a new design decision, wider writable
scope, unexpected failure or adverse effect is required, freeze the diff and return
a STOP receipt. Do not start a hidden fallback/repair loop or change route/effort.
An expected pre-edit reproduction failure is not an unexpected execution failure.

No weakened test, added ignore, broader budget, host config/auth/provider mutation,
new dependency, branch/worktree, credential access, push, publication, shared-daemon
reload or promotion. Only separately approved runtime effects may run. Scratch HOME
and worktrees are not isolation. Keep receipts free of secrets and full raw arguments.

Return inspectable diff, exact tests and exits/counts, remaining failures, owned file
hashes, predicted/observed comparison and one complexity challenge: can the same
contract be met with less code without losing verified behavior? Do not refactor for
that question alone. Leave integration/commit ownership as specified by the captain;
no two writers stage the same files. Source completion is not promotion approval.
```

## E. Independent integration reviewer

```text
Review one frozen packet from the source captain, not the whole repository. Use the
actual admitted route/effort and current policy. Read docs/HARNESS_CONTINUATION_PLAN.md,
the operator's named scope, candidate diff, base/input hashes and test receipts.
No product edits or competing integration. Write only your bounded scratch report.

Verify that the diff is within scope, preserves unrelated dirty hunks and matches its
predicted behavior. Inspect both production paths and real fixture integration.
For hook/policy work, protect no-hook/observer behavior, successful stdin gating,
exit 2 denial compatibility, 16 KiB stderr retention, timeout behavior, AgentTurn missing
policy denial, Direct compatibility, per-child Batch and deferred MCP checks.
Do not claim immutable grants, atomic revocation or OS isolation from these changes.

Check test identity, actual nonzero test counts, pass/fail/ignored values, preserved
exit codes, source/binary identity, environment differences and sanitization limits.
Use existing verified artifacts first. Re-execute only the exact acceptance commands
admitted by the packet, using synthetic resources and coordinated native tools.
No old comparison/superseded socket packet rerun and no new live provider/daemon work.

Return findings by severity with file/line evidence, scope deviations, verification
performed versus reported, unresolved risk and a source-integration recommendation.
A green focused test does not waive failed full gates. A review recommendation does
not approve publishing, promotion or cross-project authority. Stop if missing inputs
prevent a defensible recommendation rather than assuming the worker's report is true.
```

## F. Optional cross-project consent/sender design reviewer

This is not required to unblock D1 and must not delay it. No implementation is authorized.

```text
Read-only C1 design task. Read the cooperation disposition in
/Users/ianalitis/.jcode/source/jcode/docs/HARNESS_SECURITY_POSTURE.md and the
operator-forwarded dotfiles cooperation packet. Do not contact the peer or send any
message through debug, SDK user-message injection, sockets or a changed swarm topology.
Do not modify either repository, config or daemon. Scratch report only.

Inspect the existing CommMessage transport/handler, server-side session identity and
plan participation/dispatch boundaries. Validate current source before relying on old
claims. Produce one concise threat/acceptance brief for native pair-scoped opt-in
notifications, not a new implementation or general agent messaging framework.

The first question is whether operator consent is distinguishable from model requests
and whether sender identity is bound to a server-owned live principal. Worker-supplied
IDs/approved booleans and plan participants are not authority. A TUI button is not a
protected channel against unrestricted same-user shell/UI automation. State the trust
assumption or isolation prerequisite explicitly instead of promising impossible consent.

Only conditionally design bounded notify-only delivery: two consenting sessions,
third-peer denial, no inference wake or inherited tools/credentials/spawn authority,
byte/rate/queue/dedupe limits, expiry/revocation, generation/restart invalidation,
spoof/replay rejection and distinguishable queue/delivery/processing receipts.
No full transcripts or automatic result-body sharing. Pi is deferred until Jcode proof.

Return verified source facts, unresolved principal/approval decisions, the smallest
acceptance fixture and a minimal next proposed packet. No runtime probes, permission
relaxation, product edits, new service, implicit approvals or promotion.
```

## G. Operator relay to the dotfiles owner

Replace the handoff revision with the actual commit containing these documents before
sending. This is a coordination brief, not a substitute for the dotfiles agent's plan.

```text
Manual relay from Jcode source owner. No native peer delivery or transferred approvals.

Source handoff commit: <commit containing HARNESS_CONTINUATION_PLAN.md and PROMPTS.md>.
Read those files in /Users/ianalitis/.jcode/source/jcode/docs/ and receipt 7da67dbe8.
Your dotfiles handoff remains separately owned. Please do not create a source writer.

Current native source-owner reports: app-core 1300 passed / 0 failed / 24 ignored, base 1378 passed / 10 failed / 2 ignored,
root library 262 passed / 5 failed / 0 ignored. Four budget gates remain failed. Source inputs were unchanged during
validation, but the checkout includes unrelated dirty work; this is not pristine-revision
or installed-runtime acceptance. Promotion stays blocked.

Only one listener fixture received long/short/long causal confirmation. Your exact
three-test pinned-binary packet was not executed and is explicitly superseded; no repeat
is needed. Full app-core subsequently passed in the actual Git checkout with short
scratch temp paths. Do not generalize that into per-fixture proof for the other two.

The next proposed source task is D1, bounded config-fingerprint diagnosis, with Q1 budget
ownership review optional and read-only. Repairs require their own named-file scope.
Do not conflate your independent network-denying sandbox results with these native runs.

Please reconcile inspected code/artifacts separately from reported tests, add your own
fresh-session plan/prompt locations, and return one bounded receipt with any conflicts
or missing evidence. Do not edit this source tree, broaden permissions, change auth,
contact cross-swarm peers indirectly, or promote the build.
```

## Return format for every fresh-session packet

Keep the operator-facing answer short and put detail in a scoped receipt:

```text
Treatment and task:
Input revision / dirty hashes / ownership:
Changed files or explicit no-change statement:
Evidence: inspected vs executed vs reported, exact command/task/count/exit:
Predicted vs observed effect:
Remaining blockers and attribution limits:
Next single bounded action:
Promotion and external effects: not performed unless specifically authorized.
```

Do not turn a handoff into permission for recurring autonomous work. Complete the selected
bounded outcome, integrate only what the captain verifies, and hand back control with an
honest receipt when the scope ends or a hard stop is reached.
