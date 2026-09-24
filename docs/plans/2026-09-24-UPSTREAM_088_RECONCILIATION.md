# Upstream 0.88 reconciliation assessment (2026-09-24)

Status: assessment only. No merge performed. Publish still needs approval.

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
