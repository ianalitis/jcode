# Upstream sync candidate, 2026-10-07

## Scope and authorization

Operator approved upstream synchronization, measured fork-only baseline adoption, CI repair, focused upstream submissions and harness update. Integration's stricter budgets are not changed by this candidate. Existing fork history and CI guards are preserved. No force push, deletion, toolchain installation or provider change.

Upstream source is `a61c38ee9dd945fe9f930fa09c7f3454a2ae4110`. Previous fork default is `08873a2a53bc9725aaa47b6582fd5ad5afa89064`. The merge is on the retained `jcode/fork-master-sync-20260928` branch.

## Repairs

- Normalize the two inherited rustfmt sites. Existing community PR #1739 covers the CLI site.
- Clear inherited Clippy failures in MCP and Google login. Existing community PR #1736 covers MCP and issue #1740 covers both.
- Mark two genuinely test-only TUI helpers/imports test-only, removing the macOS compiler warning-budget failure without raising its zero baseline.
- Remove 12 no-op Cargo profile overrides for four packages absent from the locked all-features dependency graph.
- Construct calendar midnight infallibly, preserving the panic budget of 166 instead of accepting the upstream expect increment.
- Remove an unnecessary mutable test binding and retain existing lock-rationale comments inline. No additional lint suppression.
- Isolate model recommendation ordering from previously persisted selections using a temporary test home.
- Run the stalled-TeX regression in a fresh test subprocess, preserving both responsiveness and pending-cache assertions while avoiding a process-global missing-toolchain probe cached by earlier tests.

## Measured inherited debt

The exact upstream baseline files are stale against their own source. Code/test/error-pattern baselines were generated from an exported frozen upstream tree, not from the merged candidate. Only the two proved rustfmt normalizations preceded measurement. This imports inherited production/test size debt and error-pattern count 3704 (1420 `.ok()`, 1364 `let _`, 920 `unwrap_or_default`). Three existing parse patterns are transferred from helpers.rs to the fork's retained helpers_clock.rs without changing global totals. No integration baseline is modified, and warning/panic thresholds stay unchanged.

## Local acceptance

- `cargo fmt --all --check`: passed.
- `cargo check --all-targets --all-features -j2`: passed.
- `cargo clippy --all-targets --all-features -j2 -- -D warnings`: passed.
- `scripts/check_warning_budget.sh`: passed, current 0, baseline 0.
- Locked all-features Cargo metadata: passed, no no-op profile warnings.
- Module declarations, code/test/panic/error-pattern ratchets, dependency boundaries and wildcard reexports: passed.
- `cargo test -p jcode-sdk parity -- --nocapture`: 3 passed. One explicitly triaged TypeScript-only `close` method is a diagnostic, not an untriaged parity failure.
- Full TUI library suite in a disposable home with inherited provider environment excluded and telemetry opt-out explicit: 2477 passed, 0 failed, 19 ignored. Terminal capability was preserved. Before the two isolation fixes the clean-home suite reproduced the same two hosted failures.
- Focused MCP: 15 passed. Calendar bounds: 1 passed. Copy selection: 23 passed. Wide-character slice: 1 passed. Speed-cycle binding: 1 passed.

## Open build/environment follow-up

- macOS test links still warn that `__eh_frame` exceeds 16MB, in both default and selfdev profiles. Measure codegen-unit/unwind layout changes separately, do not suppress the warning or strip exception metadata.
- Hosted actions/checkout@v4, Windows msvc setup and sccache emit Node 20 deprecation warnings. Review documented supported action revisions and validate upgrades independently.
- Windows ARM64 Linux cross-compilation is advisory upstream because cargo-xwin/ring flags are incompatible. Native Windows validation remains required.
- Cargo audit/machete are not installed locally. Hosted checks must supply their existing runner tooling, no local install claimed.
- Running tests inside the harness inherits a named provider profile and can cause five additional fixture failures. Existing PR #1513 addresses related isolation. Clean-environment validation avoids using operator credentials, not hiding actual CI failures.
- Integration merge-tree preview hit the 90-second bound in structured merging. No integration merge or runtime activation is claimed here.

Hosted conclusions and publication/activation identities must be recorded after exact-head verification. Green local checks are not a claim that hosted CI or runtime behavior is green.
