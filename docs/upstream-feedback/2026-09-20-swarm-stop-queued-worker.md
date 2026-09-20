# Observed swarm stop does not terminate queued worker work

Date: 2026-09-20. Status: local observation, not yet independently reproduced or patched.
Runtime reported `v0.84.229-dev (d027491f6)`. Canonical source at observation: `020e51b51`.

## Expected and observed

A root-owned scout completed its initial read-only phase. The captain sent a
follow-up implementation message with `delivery=wake`. `await_members` still
reported the previous completed state. `resume` and `wake` actions returned
`No swarm plan exists for this swarm` (those actions require a task graph).
At 17:54:39 UTC, `swarm stop` returned `Stopped agent` for that exact scout ID.
The captain then started one replacement writer.

The original scout nevertheless executed `apply_patch` and selfdev tests after
the stop. Its journal contains those calls, and two test-request records identify
that same original session at 17:58:59 and 17:59:25. This is not merely a guessed
session label: its own journal records the edits and test invocations. The
replacement writer reported no edits and was paused, avoiding actual conflicting
changes. The original worker subsequently attempted a completion report at
18:03:19, which could not reach the removed swarm membership.

Expected: stop cancels active/queued owned work and acknowledges only when the
worker cannot start further effects, or explicitly reports cancellation pending.
Observed: membership became unaddressable while work continued. Subsequent
stop/message attempts returned unknown session/target. Debug `cancel` was also
unavailable because debug control is disabled. No debug settings were changed,
no shared daemon was killed, and no credentials or provider settings were read.

## Preserved local evidence

- Original: `session_lion_1789926341985_ccc22fbec86264cc`.
- Replacement: `session_wolf_1789926902554_308d6599db31a9bf`.
- Original journal: `~/.jcode/sessions/<original>.journal.jsonl`.
- Discovery task: `139523u1nf`, request
  `selfdev-build-b8cc7822032b4d3b80211f889b60a88b`.
- Focused test request: `selfdev-build-40966421d4df4032b40c6fa41ff332ae`.
- Preserved source candidate:
  `~/.jcode/scratch/e1-ownership-20260920/comm_session_e1_tests.rs` and
  `module-declaration.patch`.

These paths are local evidence, not attachments approved for upstream upload.
Do not export session transcripts. Prepare a synthetic regression that races
queued wake, completion and stop before attributing a root cause. Inspect the
existing stop/ownership/session-control tests first. Distinguish stale lifecycle
reporting, task cancellation, registry membership removal and delivery loss.

## Safe continuation

Do not treat a completed/removed membership alone as evidence that a previously
woken worker has quiesced. Pause other writers, preserve their individual receipts,
and verify the original reaches a terminal response before handing its files to
another worker. No reset, deletion or source overwrite is needed. This incident
blocks accepting the current E1 candidate until ownership and review are resolved;
it does not authorize a broad lifecycle redesign or upstream publication.

The original journal reached a terminal assistant response at 18:03:54. The
replacement worker was then explicitly assigned sole ownership of the two E1
fixture files. No concurrent source edit from the replacement was observed.

Read-only source localization: `server/comm_session.rs::handle_comm_stop` removes
the session entry and interrupt queue, then uses `agent_arc.try_lock()` to mark
it closed. The inspected path does not request cancellation before removal. A
busy agent can fail that lock while its running task still owns an `Arc`.
This is a concrete candidate cause, not yet a regression-test result. Reuse
existing `SessionControlHandle`/interrupt primitives rather than inventing a
second cancellation registry. A regression must cover queued wake and active
headless work, not just an idle member disappearing from the list.
