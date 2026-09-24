# Handoff: Opus 5.5 evaluation, planning, and phased execution (2026-09-24)

A dated snapshot, not standing authority. Recheck every fact marked *measured*
before acting on it. Supersedes `HANDOFF_2026-09-24.md` for line state.

## 0. Who you are and what this session is for

Treatment: `claude-oauth:claude-opus-5-5`, effort high (xhigh only for the
evaluation pass in §4.1). You are the captain for three stages, in order:

1. **Evaluate** the 2026-09-24 work (§4.1) with fresh eyes. The previous captain
   resolved an 84-file upstream merge alone; independent review is the point.
2. **Plan**: write the phase plan and research docs (§4.2), each with
   acceptance gates and exact validation commands.
3. **Execute** a few iterations of the top phases (§5), then write the next
   handoff (§7) for a follow-on session that runs the remaining phases.

Operator preferences that bind every stage: reports under 5 lines, next action
first, no em dashes. Clean, tidy, minimal installs. Keep contributing upstream.

## 1. Authority (operator, 2026-09-24)

Approved: all merges and syncing on the fork (integration line and
`fork/master`), the two forks below, headless CDP browser backends, Docker for
jcode-bench, shared-daemon reload, filing the bridge XPI issue, and follow-up
upstream issues/PRs from our fork work. Still separately gated: force-push,
deleting branches/worktrees/stashes, installs beyond those named, auth or
provider changes, paid/metered overflow, releases.

## 2. Line state (*measured* 22:45Z)

| Ref | Value |
| --- | --- |
| Integration line `jcode/ci-format-baseline` | `928837a2b`, **pushed** to `fork/jcode/ci-format-baseline` (fast-forward, 0/0) |
| Merge commit | `18797818d` (parents `e5597e261`, upstream `b65931032` = v0.88.0-54) |
| `origin/master` | `b60d6663b`: 2 commits past the merge base (harness-api working_dir, selfdev reload-bridge). Small, take next sync |
| `fork/master` | `b2d35a63b`, 21 ahead / 141 behind upstream. Deliberate divergence (see `FORK_POSTURE.md`). **Not yet synced.** Test merge `fork/master` + `origin/master` conflicts in 5 files: `browser_fast.rs`, `message/tests.rs`, `model_usage.rs`, `session/persistence.rs`, `harness-api-server/translate.rs` |
| Working branch | `jcode/upstream-088` == integration line (can be deleted after approval, it is redundant now) |
| Untracked | `scripts/local_model_probe.py` (someone else's local-model probe; leave it) |
| Shared daemon | runs `d7fdf2961` (pre-merge). Selfdev channel has `18797818d`. See §5 P0 |

## 3. What landed on 2026-09-24

| Commit | What |
| --- | --- |
| `0443741ef` | Stale session provider pin no longer leaks into a spawned server's env |
| `830856614` | Declared quota-fallback ladder (`JCODE_QUOTA_FALLBACK`) for a spent subscription window |
| `a7fa819e4`, `18797818d` | 0.88 assessment, then the merge. Receipt with resolution policy, defects fixed, and full test table: `plans/2026-09-24-UPSTREAM_088_RECONCILIATION.md` §Merge receipt |
| `e5597e261`, `d729f9002` | Browser policy: personal (Comet daily, Safari privacy) vs agent (headless only). Measured agent ladder. jehuang dependency map. `plans/2026-09-24-BROWSER_HEADLESS_POLICY_AND_JEHUANG_DEPS.md` |
| `8213eef2a` | Upstream issue (filed as bridge #11): firefox-agent-bridge v0.10.0 ships the 0.9.7 XPI. `upstream-feedback/2026-09-24-firefox-agent-bridge-stale-xpi.md` |
| `928837a2b` | Test fix: `save_github_token_creates_config_dir` wrote a trust entry into the developer's real `~/.jcode/config.toml` on every run (126 dead tempdir entries had accumulated; pruned, backup in `~/.jcode/scratch/config.toml.pre-leaktest-20260924T2243`). **Defect is also on `origin/master`: upstream PR candidate** |

Outside the repo:

- Forks created: `ianalitis/firefox-agent-bridge`, `ianalitis/jev-pr-labeler`.
- Lightpanda 0.4.1 pinned at `~/.jcode/browser-tools/lightpanda/0.4.1/` with
  `SHA256SUMS`; launcher `~/dotfiles/scripts/lightpanda-mcp.sh` (dotfiles
  `a50ff79`, checksum-verified, minimal env, `LIGHTPANDA_DISABLE_TELEMETRY=true`);
  registered as MCP server `lightpanda` in `~/.jcode/mcp.json` (backup
  `mcp.json.pre-lightpanda-20260924.bak`). Live-tested through jcode MCP.
- Headless Chromium: Playwright's cached `chrome-headless-shell` 149 validated
  over CDP (`~/.jcode/scratch/cdp-probe/probe.js`). No GUI Chrome installed.
- `~/.jcode/config.toml` `[tools].disabled` now includes `browser` (the
  extension-bridge tool). Reason: the operator is **uninstalling Firefox**, and
  with Firefox gone and Comet unmapped, upstream auto-detect
  (`browser_detect::resolve_detection`) falls through to Safari, which
  `is_installed()` always reports on macOS. Agents must never drive Safari.

## 4. Stage 1 and 2 work

### 4.1 Evaluation targets (read the diffs, do not trust the receipt)

Rank findings by severity and fix or file each. Highest risk first:

1. **OpenAI streaming take-upstream.** We took upstream's keyed tool-event
   rewrite of `crates/jcode-provider-openai/src/stream.rs` wholesale. Confirm our
   earlier fix `521474224` (and its tests in `stream_tool_tests.rs`) is still
   covered by upstream's `ToolUseEndFor` keyed API, not silently dropped.
2. **Fail-closed policies kept against upstream:** socket permissions
   (`server.rs restrict_socket_pair_permissions`), session MCP policy
   (`&ToolContext` signature), `provider/mod.rs` failover now offered only to a
   *configured* alternate. Look for any path where upstream's new code bypasses
   them (SDK custom tools, harness bridge, `prune_active_pids_owned_by`).
3. **Ported-into-extracted-modules correctness:** ~40 test partitions and
   `session_startup_converters.rs`, `acp/mapping.rs`, `fallback_model.rs`,
   `ui_render_lock_tests.rs`. Method: for each upstream in-place edit in
   `git diff e5597e261...b65931032`, confirm the equivalent exists at HEAD.
   A test that exists upstream but not here is a finding.
4. **Browser merge:** Gecko branch inside upstream `launch_browser_detached`.
   Now largely moot for this machine (tool disabled), but it ships in the fork.
5. **Ratchet refresh.** Budgets were raised with `--update`. The receipt claims
   all growth is the sum of both sides except three named files. Spot-check it.
6. **Known flakes** (not fixed): 2 TUI parallel render/JCODE_HOME races; SDK
   `auth::tests::processes` 300 ms fixture timeouts under load.

Commands (always via `scripts/bounded.sh`): `cargo check --workspace
--all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
`scripts/check_guardrails.sh --skip-slow`, then per-crate `cargo test -p <crate>
--lib`. `touch` touched sources after any `git apply/merge`.

### 4.2 Planning deliverables (write under `docs/plans/`, commit each)

1. `2026-09-2x-PHASE_PLAN_POST_088.md`: the phases in §5 turned into nodes with
   owner lane, effort, writable paths, acceptance gate, validation command, and
   dependency order. Follow `HARNESS_LOOP_ARCHITECTURE.md` for the admission
   and freeze sequence rather than restating it.
2. `2026-09-2x-AGENT_BROWSER_PROVIDER.md`: decide whether the built-in
   `browser` tool gets a CDP backend (Lightpanda first, headless Chromium
   fallback) or is retired in favour of the MCP ladder. Must include a hard
   guard: agent sessions never target Comet, Safari, or any personal profile.
   The code change is small (`resolve_provider` in
   `crates/jcode-app-core/src/tool/browser.rs`, `resolve_detection` fallback).
3. `2026-09-2x-BENCHMARK_RESEARCH.md`: jcode-bench (`1jehuang/jcode-bench`,
   "uncontaminatable, deterministically scored") plus current agentic
   benchmark practice (SWE-bench Verified/Live, Terminal-Bench, contamination
   controls, pass@k vs cost). Output: how we measure the fork's changes and
   lane choices, not a leaderboard chase.
4. `2026-09-2x-ROUTING_AND_DELEGATION.md`: openrouter/auto as default router
   with auto-delegation, reconciled against `policy/providers.md` lanes and the
   quota reality in §6. Measure before changing defaults.

## 5. Phase backlog (ordered)

**P0 now, small, unblocks runtime truth**

- Reload the shared daemon onto the merged build (`selfdev build-reload`, or
  `jcode self-dev --build`). Until then every `jcode run` check measures
  `d7fdf2961`. Warn: interrupts live sessions.
- Sync `fork/master`: merge `origin/master` (5 conflicts listed in §2), keeping
  the fork-CI deltas. Then merge the 2 new upstream commits into the
  integration line. Fast-forward pushes only.
- ~~File the firefox-agent-bridge XPI issue~~ done: `1jehuang/firefox-agent-bridge#11`. Watch for a reply.
- Upstream PR: the copilot test HOME leak (`928837a2b`), on a fresh `pr/*`
  branch from `origin/master` (branch creation is approved for PR heads).

**P1 correctness and hygiene**

- Agent browser guard + provider decision (§4.2 item 2).
- After Firefox is uninstalled: retire `~/.jcode/browser/agent-profile`, the
  staged XPI and host binaries (deletion needs approval naming the paths), and
  remove the native-messaging manifest if present.
- Split `crates/jcode-base/src/browser.rs` (1789 lines) and `jev.rs` (2152)
  under the 1200-line budget, then ratchet the budget back down.
- Fix the two TUI parallel races and the SDK auth fixture timeouts at the root
  (shared test-env lock, see upstream PR 1356).

**P2 upstream contribution**

- Open PRs: 1354 and 1356 are **CONFLICTING** after 0.88, rebase on
  `origin/master`; 1357 and 1362 are mergeable and green. None reviewed yet.
- Open issues we filed: 1113-1122, 1146-1154, 1176-1177, 1274, 1294,
  1348-1352. Recheck which 0.88 fixed before new work.
- jev-pr-labeler PR: skip cleanly when its key is absent
  (`upstream-feedback/2026-09-21-greptile-labeler-missing-key.md`).
- Upstream PRs from CI-only commits in our agentgrep, mermaid-rs-renderer, and
  handterm forks.

**P3 measurement**

- jcode-bench in Docker: `docker run --platform linux/amd64` (host is an
  aarch64 Ubuntu 24.04 VM, Docker 29.5.2, colima/lima installed). Nothing run
  yet. Establish a baseline before changing routing defaults.
- Routing/delegation doc (§4.2 item 4) then implementation.

Parked: voice work.

## 6. Lanes and quota (*measured* 22:45Z, recheck with `jcode usage --json`)

| Lane | State |
| --- | --- |
| Claude OAuth | 5h 27%, 7d 59% (resets 2026-09-25 07:00Z), Fable 7d **100%**. Opus 5.5 is your lane; keep 20% headroom |
| ChatGPT (prolite) | 7d **100%**, resets 2026-09-27 17:08Z. No Astra/Terra workers until then |
| OpenCode Go | Weekly limit hit (429 `GoUsageLimitError` on all 5 workers). Do not spawn Go workers until the console shows reset. Probes need `x-opencode-session` |
| OpenRouter | $73.10 of $115 left, $18.74 spent today. Metered dispatch still lacks a per-key cap; stay off it for writable work |
| Local mlx-serve | Advisory only |

Consequence: this session is effectively single-captain. Plan swarm nodes, but
expect to execute them yourself until ChatGPT or Go windows reset.

## 7. Next handoff contract

Before ending, write `docs/HANDOFF_<date>_<topic>.md` with: line state table
(measured), commits landed, evaluation findings and dispositions, phases done
and remaining with gates, lane/quota state, and an explicit continuation
prompt. Update `docs/README.md` "Current continuation prompt".

## 8. Operating notes that cost time before

- The bash guard rejects writes to computed `$VAR` paths and plain `cp` over a
  destination; write via Python with literal paths.
- Repo lease: `~/dotfiles/scripts/repo-lease.sh acquire --holder <h> --purpose <p>`
  (45 min). Dotfiles commits need `JCODE_LEASE_HOLDER=<holder>`.
- Commit with `git commit --only -- <paths>`; never add `scripts/local_model_probe.py`.
- `git merge-tree` and `cargo test` can wedge: wrap in `scripts/bounded.sh`.
- Mergiraf is the merge driver and times out on large files, falling back to git.
- Merge helpers from 0.88 are in `~/.jcode/scratch/merge088/` (`rz.py`,
  `show.py`) for the `fork/master` sync.

## Continuation prompt

> You are the Opus 5.5 captain. Read
> `docs/HANDOFF_2026-09-24_OPUS55_EVALUATION_AND_PLANNING.md`, recheck §2 and
> §6, do P0 in §5, then the §4.1 evaluation at xhigh, then write the §4.2
> plans, then execute P1 iterations. End with the §7 handoff.
