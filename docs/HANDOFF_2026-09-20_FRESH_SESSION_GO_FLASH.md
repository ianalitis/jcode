# Fresh-session handoff: OpenCode Go Flash, harness fixes and remaining work

Prepared 2026-09-20 ~21:25 UTC for a fresh Jcode session running
`opencode-go:deepseek-v4.1-flash`. This is a local handoff: nothing was pushed,
published, deployed, or transmitted to a new provider.

## 1. Start here

1. Read this file, `AGENTS.md`, `CONTRIBUTING.md`, `~/dotfiles/policy/providers.md`,
   and the current `git log`. The tree is **clean** at `97a62b6b5`; the previous
   session's dirty patches are all committed, so there is nothing to preserve
   beyond that.
2. The running daemon and the `~/.local/bin/jcode` launcher are both on
   `v0.84.255-dev (97a62b6b5)`. Verify with
   `lsof -p $(pgrep -f "builds/shared-server/jcode serve" | head -1) | awk '/txt.*jcode$/{print $NF; exit}'`.
3. Do not push, publish, or install. Reloading (`selfdev reload`,
   `build-reload`) restarts the operator's live session host: ask first, it is a
   one-line question, and they have approved every reload so far.
4. Another agent works in `~/dotfiles` concurrently. Leave their dirty files
   (`docs/current-state.v1.json`, `docs/current.md`, specs) alone and commit only
   your own paths.

## 2. Frozen route facts (do not re-derive)

Verified 2026-09-20; full receipt with evidence URLs, dates and the privacy
matrix: `~/dotfiles/docs/measurements/2026-09-20-opencode-go-deepseek-v41-flash-route.md`.

| Fact | Value |
| --- | --- |
| Route | `opencode-go:deepseek-v4.1-flash` (bare model `deepseek-v4.1-flash`) |
| Endpoint | `https://opencode.ai/zen/go/v1` (chat completions) |
| Credential | `OPENCODE_GO_API_KEY`, already present in the operator's environment |
| Entitlement | Included $10/month Go usage; monthly $15 for this model now that the limited-time 4x boost ended 2026-09-20 |
| Windows | 5h = 20% of monthly, weekly = 50%, monthly = 100%; **not exposed to Jcode**, console is authoritative |
| Privacy | Training not used, 0-day retention, DeepSeek ZDR agreement valid through **2026-09-30** |
| Client contract | Its own user agent plus a stable per-conversation `x-opencode-session`; Jcode sends both |
| Conifer context data | `GET https://api.conifer.build/v1/catalog`, 2026-09-20, 260 entries, sha256 `ebc246a5b882057afd0e7571a9adbda498c494ece16670a958b1f2155993f621`, raw capture in `~/.jcode/scratch/b10-conifer-catalog-20260920/` |

Re-check dates: the ZDR agreement on 2026-09-30, and the Go window/token table
whenever the documented 4x boost, model list, or pricing changes. Keep "Use
balance" (Zen prepaid overflow) disabled unless the operator opts in. Never
scrape credentials or send a generic SDK user agent.

## 3. Current routing state

- `~/.jcode/config.toml`: `agents.swarm_model = "opencode-go:deepseek-v4.1-flash"`,
  `[provider] default_provider = "opencode-go"`, `default_model = "deepseek-v4.1-flash"`.
  Backup: `~/.jcode/config.toml.pre-go-swarm-20260920`.
- `~/dotfiles/policy/providers.md` is canonical policy: OpenCode Go is the default
  iterative worker family, Terra is the frontier worker, Claude rows are marked
  ending, metered OpenRouter DeepSeek is complementary capacity. Generated
  `home/private_dot_jcode/swarm-prompt.md` is current, and the live
  `~/.jcode/swarm-prompt.md` was rendered from it (backup
  `~/.jcode/swarm-prompt.md.pre-go-routing-20260920`).
- Jcode classifies `opencode-go` as an included-subscription route
  (`openai_compatible_profile_is_included_subscription`), which is what lets
  private context use it. Every unverified OpenAI-compatible profile stays
  fail-closed metered: OpenCode Zen, direct DeepSeek/Z.AI/Kimi keys.

## 4. What this session changed (commits, oldest first)

Harness reliability:

| Commit | Change |
| --- | --- |
| `c146629a0` | Swarm stop is quiescent: interrupt-delivery gate, cancel re-fire while acquiring a retained terminal Agent guard, verified clears, retryable timeout |
| `1d45c4e10` | Portable host-wide Cargo gate via Python `fcntl.flock` when `flock(1)` is absent (macOS) |
| `094b96b5a` | Provider credential fixtures are isolation-independent; the CLI auto-provider fixture clears every catalog key |
| `956ba76f3` | New code extracted into `spawn_envelope.rs`, `communicate_model_list.rs`, `openrouter_single_send.rs` to stay inside the size budgets |

Routing and catalog:

| Commit | Change |
| --- | --- |
| `e627e8eb6` | `opencode-go` admitted as an included-subscription route; unverified profiles stay metered |
| `0b0cac62d` | Test pinning `agents.swarm_model` → selection → route class |
| `b56ba9b9c` | Budgetless spawn envelopes use the shared no-budget spawn contract instead of demanding a metered budget at provider dispatch |
| `4d4fefbc0` | B10 resolved: Conifer's published `context_window` values, 25 aliases |
| `8d25f0836` | `*_usd_per_mtok` pricing parsed and scaled to the per-token contract |

Agent-facing correctness and economy:

| Commit | Change |
| --- | --- |
| `9e5b5305e` | Spawn `working_dir` refuses unexpanded `~`/`$` and missing directories |
| `17b4466ab` | `list_models` capped at 60 detail lines with an omitted count, plus an optional `query` filter |
| `4665e00a3` | `list_models` lists available routes before unavailable ones |
| `2ab5b9e34` | 512 KiB history cap spills the full output and names the path |
| `1c28e5a23` | Context-guard refusals and opt-in truncation spill the full output |
| `75dae5a21` | Bash tool's own 30k-char truncation spills the full command output |
| `49f184a9c` | Webfetch truncation spills the full response |
| `97a62b6b5` | A worker's tool allowlist is persisted and restored instead of silently widening on restart |

Docs: `46d7ab7c2`, `58ff86482`, `7652dd064`, `f7158f347` in this repo; `0ad12af`,
`1976f49`, `cdfbb4b`, `df7bdb0` in `~/dotfiles`.

## 5. Verified state at `97a62b6b5`

All five library suites are green, run through `scripts/dev_cargo.sh` (the
coordinated wrapper with the host-wide build gate) unless noted:

```sh
scripts/dev_cargo.sh test --locked --offline -p jcode-base --lib -- --test-threads=1          # 1417 passed, 0 failed, 2 ignored
scripts/dev_cargo.sh test --locked --offline -p jcode --lib -- --test-threads=1               # 279 passed, 0 failed
scripts/dev_cargo.sh test --locked --offline -p jcode-app-core --lib -- --test-threads=1      # 1381 passed, 0 failed, 25 ignored
scripts/dev_cargo.sh test --locked --offline -p jcode-provider-openrouter-runtime --lib -- --test-threads=1  # 181 passed, 0 failed, 1 ignored
scripts/dev_cargo.sh test --locked --offline -p jcode-provider-core --lib -- --test-threads=1  # 131 passed, 0 failed

python3 scripts/check_module_files.py
python3 scripts/check_code_size_budget.py
python3 scripts/check_test_size_budget.py
python3 scripts/check_panic_budget.py
python3 scripts/check_swallowed_error_budget.py
python3 scripts/check_dependency_boundaries.py
python3 scripts/check_wildcard_reexport_budget.py
bash scripts/check_warning_budget.sh
git diff --check
```

All eight pass. Not run this session: full-workspace tests, full Clippy, release
builds, desktop crate, and any live cross-platform CI. The historical 15-test
inventory in `docs/HARNESS_CONTINUATION_PLAN.md` is stale; its section-4 update
records the current set (empty except for the environment traps below).

Live evidence, not just tests:

- A swarm worker spawned on `opencode-go:deepseek-v4.1-flash` ran a provider turn
  (HTTPS/SSE, 2.0 s, `stop_reason=stop`), reached `ready`, and stopped cleanly;
  its session persisted `provider_key=opencode-go`, `model=deepseek-v4.1-flash`.
- Stopping a mid-turn worker exercised the stop fix: `stopping` → cancel fired →
  membership removed in 0 ms → clean exit.
- A bounded patch admission (one file, one failing test, exact validation
  command) produced a correct two-line fix that the captain re-verified.
- The bash spill named a real file holding the full 40,045-char output.
- A spawned worker's session file contains `"spawn_allowed_tools": ["bash"]`.

## 6. Traps and lessons (each cost real time this session)

1. **`TMPDIR=/tmp` on macOS.** `/tmp` is a symlink to `/private/tmp`, and
   `auth::transfer` walks ancestors with `O_NOFOLLOW`, so every temp dir under
   `/tmp` yields `TransferError::UnsafePath`. Six `auth::transfer` tests "fail"
   this way and all pass with a real path. Do not "fix" the guard; use
   `/private/tmp/...` or the default `/var/folders/...`.
2. **`JCODE_HOME` beats `HOME`/`XDG_CONFIG_HOME`.** `scripts/dev_cargo.sh` sets
   `JCODE_HOME` for test-state isolation, so a fixture that writes config under a
   temp `HOME` writes where nothing reads. Pin `JCODE_HOME` per test and write
   through `jcode_storage::app_config_dir()`.
3. **Changing `HOME` breaks `cargo`.** `mise` owns the `cargo` shim and refuses a
   synthetic `HOME` ("config files are not trusted"). For a HOME-isolated run use
   the toolchain binary directly, e.g.
   `~/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo`, with
   `CARGO_HOME`/`RUSTUP_HOME` pinned.
4. **Size and swallowed-error ratchets reject growth.** Already-oversized files
   must shrink or stay flat; new files must have zero `let _ =`, `.ok()` and
   `.unwrap_or_default()` hits. Extract into a sibling module (`include!` keeps
   the parent's imports and paths) instead of touching a baseline; `--update`
   needs operator approval.
5. **`swarm stop` proof.** The old daemon had the queued-worker defect; the fix is
   live now, but still verify a stopped worker's terminal state rather than
   trusting the acknowledgement alone.
6. **Restart policy.** `recover_member_status` rewrites a persisted `running`
   member to `crashed` and a persisted `ready` member to `stopped` ("idle worker
   not restored after server restart"); only the first is restored. A worker
   blocked in a long tool call is persisted as `ready`, so it is deliberately
   *not* restored. Verify restore behaviour with unit tests, not live reloads.
7. **Spawn `working_dir` is literal.** Shell syntax is not expanded and the
   directory must exist; pass an absolute path.
8. **`list_models` is large.** Pass `query` (case-insensitive over model,
   provider, auth method) instead of dumping the catalog.
9. **Truncated output is on disk.** Notices from the guard, bash and webfetch
   name a file under `<JCODE_HOME>/tool-output/`; read it with `offset`/`limit`
   instead of re-running a guessed-narrower command. Retention: 7 days, newest
   200 files.

## 7. Remaining work, in priority order

### 7.1 Metered dispatch admission (blocked, largest)

Metered repository dispatch stays on hold. `run_frozen_attempt` still has no
approved production caller, and whole-task spend enforcement does not exist. The
smallest proposed packet is preserved at `~/.jcode/scratch/j5-f1-remainder.md`:
synthetic no-tool one-shot, `OutboundPacket` required, frozen model/backend/
endpoint/effort with no fallback, integer price ceilings and a proven token bound,
worst-case cost ≤ reservation before send, one constrained send, durable ledger,
ambiguous on mismatch. Reuse the existing `FrozenAttempt`/`OutboundPacket`/ledger
primitives; do not build a second dispatcher.

### 7.2 Economy measurements and durable receipts (E1 remainder)

- Durable compact accepted-task/cache receipt (schema and retention are separate
  decisions, do not silently drop data).
- A real workload quality/review-cost/cache evaluation for the Go lane: paired
  fixed packets, deterministic acceptance, measuring correctness, abstention,
  strict-schema behaviour, tool reliability, latency, cache hit/miss and accepted
  task cost. The bounded patch check in the receipt is the starting point, not a
  benchmark.
- OpenCode Go window visibility: Jcode cannot see Go's 5h/weekly/monthly windows,
  so the first real signal of an approaching limit is the console.

### 7.3 Catalog and pricing refresh

Candidate discovery can be automatic; promotion must not be. Conifer pricing is
now parsed from the live shape, but nothing refreshes the static fallback lists
or re-verifies per-model windows when a provider's catalog changes. Prefer a
documented, dated evidence step over a new subsystem.

### 7.4 Known flake to diagnose

`agents_md_resolves_linked_git_worktree_root` passes alone and fails among
parallel prompt tests (suspected process PATH/env interference; root cause not
established). Treat as a bounded reproduction task, not a rewrite.

### 7.5 Cross-project

`~/dotfiles/docs/plans/2026-09-20-lightweight-local-inference-and-agent-economy.md`
owns local inference benchmarking and service lifecycle. Do not duplicate it in
the source repo.

## 8. Ownership and approval boundaries

- Approved so far: local builds/tests, scoped commits on
  `jcode/ci-format-baseline`, `selfdev` builds and reloads (each approved when
  asked), config edits with backups, policy/receipt commits in `~/dotfiles`.
- Not authorized: push, PR, publish, install, deployment, branch or worktree
  creation, provider/auth changes, spending beyond included usage, and starting
  metered dispatch.
- Keep one writer per file, commit only owned paths, and never commit another
  agent's dirty work.
- The operator has repeatedly authorized routine implementation, tests and
  integration without checkpoints. Interrupt only for real scope, security, cost,
  or deployment decisions, or when blocked on their action.

## 9. Evidence index

- Go route receipt: `~/dotfiles/docs/measurements/2026-09-20-opencode-go-deepseek-v41-flash-route.md`
- B10 resolution data: `~/.jcode/scratch/b10-conifer-catalog-20260920/catalog.json`
- Prior B10 investigation: `~/.jcode/scratch/b10clarify-20260917T155008Z/REPORT.md`
- Swarm-stop worker evidence: `~/.jcode/scratch/swarm-stop-queued-worker/`
- Metered packet draft: `~/.jcode/scratch/j5-f1-remainder.md`
- Continuation plan (stale inventory + current update):
  `docs/HARNESS_CONTINUATION_PLAN.md`
- Harness loop architecture standard: `docs/HARNESS_LOOP_ARCHITECTURE.md`
