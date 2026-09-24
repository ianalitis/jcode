# Upstream 0.88 reconciliation assessment (2026-09-24)

Status: **merged** on branch `jcode/upstream-088` (operator approved all merges
and syncing, 2026-09-24). Publication to `fork/master` is a separate step.
See the receipt at the end of this file.

## Measured state

| Ref | Commit | Note |
| --- | --- | --- |
| Integration line (`jcode/ci-format-baseline`) | `830856614` | 2 commits added 2026-09-24 (below) |
| `origin/master` (upstream) | `b65931032` | `Cargo.toml` `version = "0.88.0"` |
| Merge base | `ef4c2bd69` | docs: update weekly stars chart |
| `fork/master` | — | 21 commits not in the integration line, 352 not in `fork/master` |

`git rev-list --left-right --count origin/master...HEAD` -> `141  350`:
the integration line is **350 ahead / 141 behind** upstream. `fork/master`
is stale relative to the integration line, so the integration line is the
merge target and `fork/master` is a publication step, not a merge source.

## Size of the merge

- Upstream changed 369 files since the merge base; the integration line
  changed 678; **185 files changed on both sides**.
- `git merge-tree --write-tree origin/master HEAD` (612 s, bounded) reports
  **84 conflicts**: 83 content conflicts plus one modify/delete
  (`crates/jcode-provider-grok-build-runtime/src/bin/fake_acp.rs`, deleted
  upstream by the grok-build HTTPS/ACP refactor, modified here).
- `mergiraf` is the configured merge driver and auto-solved 8 of them, at a
  measured cost of roughly 10 minutes for the whole trial merge. Budget for
  that; a quick `--no-commit` merge is not quick.

## Where the conflicts land (our bespoke areas, not noise)

- Provider and routing: `server/provider_control.rs` (+its tests),
  `cli/provider_init_tests.rs`, `jcode-config-types`, `jcode-provider-core/models`,
  `jcode-provider-metadata/{catalog,lib}`, `jcode-base/{provider_catalog_tests,
  provider/catalog_routes,jev}`, and the anthropic / openai / openrouter runtime
  crates. This is exactly where upstream also moved (Yolo-Auto provider #1432,
  Typesafe auto routing, grok-build over HTTPS, removal of the Claude Code CLI
  subprocess transport).
- Server/session: `server.rs`, `comm_session.rs`, `client_lifecycle.rs`,
  `base/session.rs`, `session/persistence.rs`.
- Tools and TUI: `tool/{bash,bg,browser*,communicate,discover,todo,mod}.rs`,
  ~20 `jcode-tui` test files, `jcode-tui-render/swarm_gallery.rs`.
- Also `Cargo.lock`, `jcode-sdk/{auth,ssh,structured}.rs`,
  `jcode-protocol/wire.rs`, `src/cli/acp.rs`.

## Upstream 0.88 worth having

Voice/Jev routing and hedging, `fix(session): prune stale active-pid markers
left by the exec reload (#1419)` (directly relevant to our reload path),
`feat(tui): support embedded and multiple slash commands (#1278)`, Yolo-Auto
provider, `deps: bump h2 / rustls past RUSTSEC-2026-0258/0285`, and a batch of
TUI/clippy fixes.

## Proposed procedure (needs approval before it runs)

1. Merge `origin/master` (v0.88.0) into the integration line, on the existing
   line or a dedicated `jcode/upstream-088` branch (branch creation needs
   approval). Merge, not rebase: 350 local commits with bespoke routing must
   keep their identity, and `FORK_POSTURE.md` forbids rebasing the divergence.
2. Resolve the 84 conflicts by area, in this order: config-types and
   provider-metadata (types first), then provider-core and the provider
   runtimes, then app-core server/agent, then tools, then TUI tests, then
   `Cargo.lock` (regenerate with cargo, never hand-merge).
3. Re-verify, at minimum: `cargo check --all-targets`, `cargo test` for
   `jcode-provider-core`, `jcode-base`, `jcode-app-core`, `jcode-config-types`,
   plus `cargo fmt --check` and clippy 1.98 to match the fork CI guards.
4. Publish to `fork/master` by merge or fast-forward **only after** step 3
   passes and the operator approves the push.

## Decisions the operator owns

- Merge on the existing integration line, or a fresh `jcode/upstream-088`
  branch?
- Keep or drop the Claude Code CLI subprocess transport and grok-build ACP
  paths that upstream removed, if the fork still has callers.
- Approve the merge at all now, or park it behind the routing work.

## Commits added to the integration line this session

- `0443741ef` cli: stop a stale session provider pin from framing a spawned server
- `830856614` provider: declared quota-fallback ladder for a spent subscription window


## Merge receipt (2026-09-24)

Branch `jcode/upstream-088`, created from the integration line at `e5597e261`,
merges `origin/master` `b65931032` (`v0.88.0-54`). All 84 conflicts were
resolved by the captain: the five Go workers spawned for it all failed at their
first request with `429 GoUsageLimitError` (weekly limit), and the ChatGPT 7-day
window was at 100%, so there was no admitted worker lane.

Resolution policy (same classes as the 0.86 merge, `ed35ce309`):

- **Take upstream where it replaced the same thing:** keyed OpenAI tool
  streaming (`46af73120`, `53098c4d8`), Claude Opus 5.5 as default and its
  catalog order, removal of the Claude Code CLI subprocess transport (dropped
  our `claude_provider()` fallback in `accessors.rs`), grok-build over HTTPS
  (accepted the `fake_acp.rs` deletion), upstream's `done` framing in the
  harness bridge (five of our test copies asserted `[0]`, synced to `.last()`).
- **Keep ours where the fork's behaviour is the point:** dedicated headless
  Firefox agent profile (now the Gecko branch of upstream's multi-browser
  `launch_browser_detached`; non-Gecko browsers keep upstream's
  already-running guard), inert telemetry (whole file, no collection), fail-closed
  socket permissions, fail-closed session MCP policy (`&ToolContext` signature;
  upstream's new SDK test adapted), boxed `Outbound::Reply`, token-capped tool
  parameter descriptions, `save_for_resume`, the split test layout.
- **Combine:** browser tool metadata (upstream `browser`/`detected_via`/
  `connected_browser` plus our `agent_profile`/`agent_headless`), jev purposes
  (our Routing plus upstream Voice), session persistence (our `improve_mode`
  plus upstream system-prompt/canary/debug), tool dispatch (our permission check
  plus upstream SDK custom-tool disable), SDK login pipes (our non-panicking
  pipe handling plus upstream's bounded stderr drain), exempt lists.
- **Port into extracted files instead of restoring inline code:** upstream edits
  to `session_startup_converters.rs`, `session_startup_stub.rs`,
  `acp/mapping.rs`, `acp/tests.rs`, `ui_render_lock_tests.rs`,
  `config_hooks.rs`, `wire_events.rs`, and about 40 test partitions.

Defects found and fixed while merging (not upstream's, not a weakened test):

- `provider/mod.rs`: our `b0c874a49` records "not configured" as a failover
  reason, which made a credential-free daemon offer a switch to another
  unconfigured provider instead of the `/login` error. The prompt is now only
  offered to an alternate that is configured.
- `config.rs`: `JCODE_QUOTA_FALLBACK` (added by `830856614`) was missing from
  `CONFIG_ENV_KEYS`.
- TUI `with_temp_jcode_home` now isolates `JCODE_ACTIVE_PROVIDER` and
  `JCODE_INITIAL_PROVIDER_EXPLICIT`; running the suite from inside a jcode
  session made onboarding look like an explicit `--provider` choice.
- Upstream items that fail our gates on macOS: `allocator.rs` Linux-only
  constant/`return` (restructured, no allow), two clippy lints in upstream tests.
- Both sides added `Orcarouter`; git merged them cleanly into duplicate match
  arms and a duplicate login entry, and a duplicate `claude-opus-5-5` price row.

Verification (macOS, this checkout):

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `scripts/check_guardrails.sh --skip-slow` | all gates pass |
| `jcode-base --lib` | 1661 passed, 0 failed, 6 ignored |
| `jcode-app-core --lib` | 1574 passed, 0 failed, 31 ignored |
| `jcode-tui --lib` | 2414 passed, 3 failed: 1 fixed (env leak, above), 2 pass in isolation (known parallel render/JCODE_HOME races) |
| `jcode --lib` / `--bin` | 289 / 8 passed |
| provider-core, openai, openai-runtime, openrouter-runtime, anthropic-runtime, metadata | 145, 35, 141, 188, 70, 18 passed |
| harness-api-server, harness-api, protocol, config-types, setup-hints, tui-mermaid, tui-render | 156, 41, 89, 19, 150, 64, 69 passed |
| `jcode-sdk` | 58 passed; `auth::tests::processes` 20/20 serially, 1-2 timing flakes under parallel load (300 ms fixture timeouts) |

Ratchet baselines were refreshed with each script's `--update`, the same as
the 0.86 merge. Every size entry was attributed against both parents: all
growth is the sum of the two sides except `tool/browser.rs` (+18) and
`browser.rs` (+64), which are the Gecko agent-profile launcher kept inside
upstream's multi-browser launcher, and `jcode-setup-hints/src/lib.rs` (+19),
which is our clippy-drift fixes on top of upstream's file.

Open follow-ups: split `browser.rs`/`jev.rs` below the 1200-line budget; the SDK
auth fixture timeouts; an upstream note that `b772d6f8f` ("perf: skip full
session scan...") also deletes the Claude CLI runtime crate.
