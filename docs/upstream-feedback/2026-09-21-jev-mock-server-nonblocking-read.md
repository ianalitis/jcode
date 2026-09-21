# Jev mock-server test reads a non-blocking socket and fails on macOS

Date: 2026-09-21. Status: locally reproduced, fixed on
`jcode/ci-format-baseline`. Not yet published upstream. Defect observed at
upstream `e589cbe5a` (v0.86.0).

## Expected and observed

`crates/jcode-base/src/jev.rs` test helper `mock_server` binds a listener,
calls `listener.set_nonblocking(true)` so it can poll `accept()` with a deadline,
then reads the accepted stream with a 5-second read timeout:

```rust
let mut stream = loop { match listener.accept() { Ok((stream, _)) => break stream, ... } };
stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
let n = stream.read(&mut buffer).unwrap();
```

On BSD/macOS an accepted socket inherits the listener's `O_NONBLOCK` flag, so
`read` returns `WouldBlock` immediately instead of waiting for the request:

```text
thread '<unnamed>' panicked at crates/jcode-base/src/jev.rs:827:54:
called `Result::unwrap()` on an `Err` value: Os { code: 35, kind: WouldBlock,
message: "Resource temporarily unavailable" }
```

The mock thread dies, and the test that drives it then fails on a downstream
assertion, which hides the real cause.

Observed: `jev::tests::auth_billing_and_redirect_errors_are_redacted_and_never_retried`
failed 2 of 3 runs at default parallelism. Linux does not propagate the flag from
listener to accepted socket, so a Linux-only CI would stay green.

## Proposed fix

Restore blocking mode on the accepted socket before setting the read timeout:

```rust
// On BSD/macOS an accepted socket inherits the listener's non-blocking flag,
// so reading with a timeout returns WouldBlock instead of waiting.
stream.set_nonblocking(false).unwrap();
stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
```

After the change the test passed 5 of 5 consecutive runs.

## Evidence

- Source: `crates/jcode-base/src/jev.rs` (`mock_server`).
- Local run logs: `~/.jcode/scratch/guardrails-after-merge.log` and the
  `jcode-base --lib` runs recorded in
  `~/dotfiles/docs/measurements/2026-09-20-upstream-v0.86.0-orientation.md`
  context; the flake reproduces with
  `jcode-base --lib jev::tests::auth_billing_and_redirect_errors_are_redacted_and_never_retried`.
