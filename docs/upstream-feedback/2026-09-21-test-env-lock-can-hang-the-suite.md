# The shared test-env lock can wedge a test binary forever

Date: 2026-09-21. Upstream code: `crates/jcode-base/src/storage.rs` (`test_env_lock`,
`lock_test_env`) is identical at `e589cbe5a` (v0.86.0) and on the local merge tip.
Not yet published as an issue or PR.

## Expected and observed

`lock_test_env()` returns `test_env_lock().lock()`:

```rust
pub fn lock_test_env() -> MutexGuard<'static, ()> {
    test_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
```

`std::sync::Mutex` is not reentrant, and the lock guards process-global state
(`JCODE_HOME`, config, auth overrides), so it is acquired from many call sites,
including helpers that a test may reach while it already holds the lock. A
re-entrant acquisition on the same thread is not a slow test, it is a permanent
hang, and it takes the whole binary with it.

Observed on this machine (`aarch64-apple-darwin`, 11 cores, `v0.86.0` merge base
plus local commits) after `cargo test -p jcode-tui --lib`:

```text
$ sample <pid of the wedged test binary> 3
    2637 Thread_44670464: tui::app::tests::answering_no_on_continue_prompt_shows_...
    2637 Thread_44670511: tui::app::tests::auto_poke_does_not_repeat_until_incomplete_...
    ...  eleven test threads, one per core, each:
      11  jcode_base::storage::lock_test_env
          std::sync::poison::mutex::Mutex::lock
          __psynch_mutexwait
    main thread: run_tests_console -> recv CompletedTest -> park
```

Eleven test threads sat in `__psynch_mutexwait` with near-zero CPU; the run had
not terminated after fifty minutes and was killed. Nothing was waiting on the
render-state lock, so the env lock itself was the cycle. Two shapes fit the
sample, and they are indistinguishable from outside because a self-deadlocked
owner and an ordinary waiter have the same stack:

1. a thread that called `lock_test_env()` while it already held the lock, or
2. a guard whose thread went away before releasing it.

An earlier wedge in the same suite was a genuine ABBA cycle between this lock and
the render-state lock (`tui::ui`), fixed downstream by fixing the lock order and
by letting `with_render_state_lock` degrade to `try_lock`.

## Minimal reproduction of the hang

A nested acquisition never returns:

```rust
#[test]
fn nested_env_lock_hangs_forever() {
    let _outer = jcode_base::storage::lock_test_env();
    let _inner = jcode_base::storage::lock_test_env(); // never returns
}
```

## Proposed fix

Bound the wait and make the failure name its cause. This keeps the uncontended
path a single `try_lock` and leaves the poisoned-lock recovery as it is:

```rust
pub fn lock_test_env() -> MutexGuard<'static, ()> {
    let mutex = test_env_lock();
    match mutex.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => wait_for_test_env_lock(mutex),
    }
}
```

`wait_for_test_env_lock` polls, warns once after 10s, and after a bound (300s by
default, `JCODE_TEST_ENV_LOCK_TIMEOUT_SECS` to shorten it) panics with the waiting
test's name and the last successful acquirer. Recording the acquirer on the
acquire path is enough to name the culprit: while a holder is wedged no other
thread can acquire, so the record still points at it.

The same record also detects the re-entrant case directly, in about 200ms instead
of the full bound: if the recorded holder is this thread and the lock still cannot
be taken, this thread already holds it and the wait cannot end. The grace period
covers the few instructions between taking the lock and storing the record.

The alternative, letting nesting succeed, requires replacing the guard type
(`MutexGuard<'static, ()>` becomes a wrapper or a reentrant-mutex guard) at around
forty annotated sites in the tree. It is the better end state if nesting is
expected; the bounded wait is the smaller change that removes the unbounded hang.

Local tests covering the change:
`storage::tests::shared_test_env_lock_timeout_defaults_and_parses_override`,
`storage::tests::a_contended_env_lock_is_acquired_once_the_holder_releases`, and
`storage::tests::re_entering_the_env_lock_is_reported_instead_of_hanging` (the
last fails in 0.21s instead of hanging).

## Evidence

- `git show e589cbe5a:crates/jcode-base/src/storage.rs` lines 28-38.
- Stack sample and the fifty-minute wedge: recorded in this fork's
  `docs/TUI_TEST_FLAKINESS.md`, section "The env lock alone can still hang".
- The 2026-09-01 ABBA predecessor and its fix: same document, "The deadlock".
