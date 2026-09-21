# Fork default-branch CI was red for configuration reasons, and its branches ran no CI

Date: 2026-09-21. Scope: the fork's own GitHub Actions, not upstream.

## Observed

The fork's default branch (`ianalitis/jcode` `master`) carried upstream's
workflows unchanged, so every scheduled and push run on the default branch failed
before compiling anything:

| Job | Failure |
| --- | --- |
| `Quality Guardrails`, both `Build & Test` legs, `Windows Cross-Target Check` | `The ssh-private-key argument is empty` at "Configure SSH for cargo git dependencies": the fork has no `DEPLOY_KEY` secret |
| `iOS TestFlight` | "Export and upload to App Store Connect" ran with empty App Store secrets, because `build-and-upload` is not gated on the owning repository |
| `Format` | rustfmt drift in four `jcode-tui` files, all upstream code |

`ci.yml` also triggered `push` only on `main`/`master`, so the fork's own
`pr/*` and `jcode/*` branches produced **no** CI runs at all: our published
contribution branches were pushed with zero fork validation.

## Change

Two commits on the fork's default branch, both fast-forwards, no force:

- `3e5c99e49` takes the workflow-only validation work already staged as
  `68b341633` on `jcode/ci-format-baseline`: push CI on every branch,
  `permissions: contents: read`, no `DEPLOY_KEY`/ssh-agent,
  `persist-credentials: false`, telemetry disabled for validation,
  `build-and-upload` gated on `github.repository == '1jehuang/jcode'`,
  diagnostic artifact uploads fork-scoped, plus the standalone
  `workflow-lint.yml` (actionlint and shellcheck) and `docs/FORK_CI.md`.
- `8439023cc` applies `cargo fmt` to the four drifting `jcode-tui` files.

Only those six paths differ from upstream `master`; source stays identical to
upstream. `actionlint -no-color` passes.

## Result

| Job | Before | After |
| --- | --- | --- |
| `Format` | failure | success |
| `iOS TestFlight` | failure | success |
| `Windows Cross-Target Check (Linux)` | failure | success |
| `Workflow Lint` | n/a | success |
| push CI on `pr/*` branches | none scheduled | runs |

## What is still red, and why it is not a fork configuration problem

Reproduced on a pristine upstream `master` copy, so these are upstream defects:

- `Quality Guardrails` fails `cargo check --all-targets --all-features`:
  `src/bin/tui_bench.rs` does not compile (`diff_line_wrap` is not a member of
  `TuiState`; `focus_revision` missing from a `SidePanelSnapshot` initializer).
  That is the "dev-bins bench build" part of the open lint PR
  ([#1354](https://github.com/1jehuang/jcode/pull/1354)).
- `Build & Test (ubuntu/macos-latest)` fail `Run TUI library tests`. Upstream
  `master` reports 19 `jcode-tui --lib` failures single-threaded (2332 passed).
  The order-dependent part is [issue 1365](https://github.com/1jehuang/jcode/issues/1365)
  / [PR 1366](https://github.com/1jehuang/jcode/pull/1366); the remainder is
  tracked by #1340 and #1342.

Fork parity therefore needs those upstream fixes to land, not more fork
configuration.
