# Permanent test-profile validation, 2026-10-17

## Scope and authority

Resumed handoff `264c9340d940af0c4e5645d8e068c1c5639e4490` under the operator's instruction to continue overnight with dotfiles as primary dispatcher. Both Jcode checkouts were clean at their recorded HEADs. Existing common Git lease `release-sync-bat-20261017` was acquired without takeover for the isolated release-sync checkout, limited to `Cargo.toml` and this dated receipt. Dotfiles captain dog confirmed no overlapping source writes or Cargo builds.

No push, publication, release, installation, host/auth/provider configuration write, banked reset, shared-daemon change or runtime activation occurred. The primary checkout remains at `921807b98d74f2fcd102570327547189e70dd8f3`. Local source integration and activation remain separate gates.

## Minimal change

Set `[profile.test] opt-level = 1` and remove the now-unnecessary app-core package-specific runtime override. Preserve debug information level zero, incremental compilation, 256 codegen units, debug assertions, overflow checks, unwind behavior, explicit package overrides and the inherited non-workspace dependency wildcard zero. This is a workspace test default, not optimization of every dependency and not a release/selfdev profile change.

Independent Astra/high reviewer bug admitted this exact minimal change after inspecting the original completed experiment. The reviewer found and required a scratch-harness correction before native validation: build stderr must not be overwritten by app-core runtime stderr, and warning-suppression pattern checks must be retained. Both were corrected before execution. No linker warning suppression, test weakening or additional tuning was added.

Complexity challenge: the single profile default replaces narrower tuning. No linker flags, source restructuring, new dependency, extra abstraction or further package override is needed.

## Native no-override validation

- Runner: `/Users/ianalitis/.jcode/scratch/permanent-test-opt1.sh`.
- Evidence: `/Users/ianalitis/.jcode/scratch/permanent-test-opt1.G88ucu`.
- Tested tree: handoff HEAD plus the exact `Cargo.toml` diff saved in evidence. Index was empty before and after validation.
- Toolchain: rustc 1.98.1 (`48a229cea`, LLVM 22.1.8), aarch64-apple-darwin.
- Cargo commands use the permanent manifest without `--config` profile overrides. Existing Cargo wrapper coordinates the shared target, telemetry is off, hooks are enabled for the complete native suites, and fixture homes/runtime directories are isolated.

| Artifact | `__eh_frame` | Profile and role |
| --- | --- | --- |
| Supporting CLI | `0xd32f24` | opt1, debug0, assertions/overflow true, test=false |
| Provider integration harness | `0x796718` | opt1, debug0, assertions/overflow true, test=true |
| E2E harness | `0x7514bc` | opt1, debug0, assertions/overflow true, test=true |
| App-core libtest | `0x7db438` | opt1, debug0, assertions/overflow true, test=true |

All four were rebuilt (`fresh=false`), retain `__unwind_info`, and have unwind frames below 16 MiB. Their feature sets remain exactly `bedrock/default/pdf`. Raw compiler JSON, non-JSON build text and separate build stderr passed warning/suppression checks. Full and ignored test inventories match the admitted experiment exactly.

Complete native suites passed with zero failures and zero filtered tests: app-core **1,814 passed / 33 ignored**, provider **9 / 0**, E2E **60 / 7**. The deliberate panic/catch regression passed. This is artifact and execution evidence, not a claim based only on successful linking.

This is a warm-target run, not a cold build. CLI-stream artifacts: opt0 cached595, opt1 rebuilt24/cached66, opt3 cached25. App-core stream: opt0 cached480, opt1 rebuilt4/cached52, opt3 cached14.

### App-core build-script nuance

Removing its package override changes the app-core custom-build unit from opt1 to opt0, while library/libtest remain opt1. Native records show the opt0 custom-build artifact was cached. Its generated `jcode_docs.rs` SHA256 is `b7e37b112fa2141435836e8d30826bbe088cc6c28c5f64a28be57b6ca4bce2ce`, identical to both original experiment outputs. The build script enumerates documentation and emits rerun directives, not linker/cfg/environment changes. Do not claim byte-identical rebuild scope to the experiment.

## Full local acceptance

The existing `/Users/ianalitis/.jcode/scratch/release-sync-acceptance.sh` ran sequentially after the native profile checks. Evidence `/Users/ianalitis/.jcode/scratch/release-sync-acceptance.U6SmTy` records **36/36 gates passed**. Combined background task `363289tlpw` completed in 437.53 seconds with exit0.

Coverage remains the handoff's local ratchets/boundaries/fmt/actionlint, workflow/release/recovery contracts, warning contracts, locked metadata, all-target/all-feature compiler and clippy, SDK parity, retention, actual secret PTY tests, browser/stdin/provider/E2E and whitespace gates. SDK parity still visibly reports its explicitly triaged TypeScript-only `close` diagnostic. That diagnostic is not the resolved Apple linker warning.

Final independent Astra/high evidence/diff review by bug: **ACCEPT for scoped local native promotion, no blocking findings**. The reviewer independently reproduced JSON/text splits, verified current executable hashes, artifact/feature and external-override equivalence, complete/ignored inventories, raw warning checks, generated documentation bytes, and 36 unique successful gates. Read-only review performed no builds, suite execution or writes. This verdict authorizes no external effect or integration into another checkout.

## Remaining boundaries

Linux hotkey behavior, Windows ARM64, FreeBSD and hosted release validation remain unexecuted on this Mac. No platform installation or Docker startup was attempted. Rust SDK parity is not npm packaging acceptance. The existing proposal-only synchronization workflow remains unchanged, and remote synchronization is not fixed merely by local acceptance.

Dotfiles owns a disjoint two-file usage-proposal evidence correction under its own lease. Retrieval follow-up remains separate, with no Jcode dependency/config/runtime change. Deferred native routing packets must be separately bounded and reviewed, not inferred authority for cross-project product changes.
