# Observed swarm stop does not terminate queued worker work

Date: 2026-09-20. Status: filed upstream as
[#1352](https://github.com/1jehuang/jcode/issues/1352), and locally reproduced,
patched, and integrated on `jcode/ci-format-baseline`.
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

## Deterministic reproduction and local patch

The initial deterministic regression failed on the unpatched implementation
because stop did not fire the registered active-turn signal. The final scoped
regression module covers four concrete races:

1. A real worker turn emits one text delta and blocks while live and persisted
   follow-up work is queued. Stop must end that turn, empty both queues, leave no
   active-turn signal, avoid a second provider call, and preserve an unrelated
   session/member.
2. `idle_live_agent` reserves an Agent before a turn registers its cancellation
   signal. Stop begins, then the reserved wake is handed to
   `spawn_tracked_live_turn`. The late wake is discarded without calling the
   provider, the Agent is closed, and `Done` arrives only after the guard is
   released and quiesced.
3. Live and persisted queue producers enter before stop, then append after the
   stop gate is set. Stop waits for those producers, performs a final verified
   clear, and rejects all post-stop live, persisted and direct-Agent enqueues.
   Queue registration is the explicit resume boundary; it cannot reopen delivery
   while stop is in flight, but can reopen a terminal stopped lifecycle.
4. An uncooperative Agent lock exceeds the five-second quiescence timeout. The
   target remains registered with status `stopping`; releasing the lock and
   retrying the same stop succeeds instead of returning `Unknown session` or
   replaying the cached timeout.

The stop handler now lives in the existing `swarm_stop_ownership.rs` stop module.
After the existing ownership checks, it marks the member `stopping`, atomically
closes interrupt delivery and waits for already-admitted producers. It repeatedly
fires `SessionControlHandle::request_cancel()` while waiting up to five seconds
to acquire and retain the terminal Agent guard. Only while holding that guard does
it perform final verified live/persisted clears, mark the Agent closed, remove
routing and membership, complete the interrupt lifecycle, and emit `Done`.

The interrupt gate is co-located with the existing per-session queue lifecycle,
moves with session renames, and is restored by queue registration for a new or
resumed lifecycle. A timeout or final-clear failure leaves membership, Agent and
the closed delivery gate intact so a later stop can retry safely. Stop mutations
use the existing no-final-replay path, so a persisted timeout is not replayed on
retry. Headless startup recovery excludes `stopping` members. Direct queueing on
a closed Agent is also refused. The guarantee is local turn/tool-dispatch
quiescence; it does not retroactively undo an external effect that began before
cancellation reached a safe point.

The first parallel module run also exposed a real fixture-isolation defect in the
recently added E1 receipt tests: their environment guard changed config inputs but
did not invalidate the reloadable config cache, so another test could leave the
cache inside its throttle window with `features.memory` still enabled. The E1
guard now invalidates the cache after applying and restoring its environment,
under its existing global test lock.

Validation used `scripts/dev_cargo.sh` through the coordinated self-dev runner.
The pre-fix regression failed 0 passed / 1 failed (`939779b17o`). The final stop
module passed 4 / 4 (`13115290zq`). The complete parallel comm-session module,
including five E1 tests plus adjacent ownership and spawn coverage, passed 45 / 45
(`29438400rr`). The parallel live-turn action suite passed 12 / 12
(`283398oyhb`), and the session-control/queue lifecycle suite passed 21 / 21
(`289858xgiw`). All seven structural ratchets passed: module files, production
size, test size, panic usage, swallowed errors, dependency boundaries and wildcard
re-exports. `comm_session.rs` is 1445 lines versus its 1620-line baseline;
`comm_session_tests.rs` is 1057 lines and the scoped stop test module is 560 lines,
both below the 1200-line test limit. No baseline was updated.
