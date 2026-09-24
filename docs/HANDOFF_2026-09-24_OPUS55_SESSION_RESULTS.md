# Handoff: Opus 5.5 session results, post-0.88 (2026-09-24, 23:40Z)

A dated snapshot, not standing authority. Supersedes
`HANDOFF_2026-09-24_OPUS55_EVALUATION_AND_PLANNING.md` for line state. Plan of
record: [`plans/2026-09-24-PHASE_PLAN_POST_088.md`](plans/2026-09-24-PHASE_PLAN_POST_088.md).

## Line state (measured 23:40Z)

| Ref | Value |
| --- | --- |
| Integration line `jcode/ci-format-baseline` | this commit's parent chain from `08d491015`, pushed to fork (0/0) |
| `origin/master` | `8870f1993`, fully merged into the integration line (`f668b9db9`) |
| `fork/master` | `5fb914af8`: 0 behind, 22 ahead of upstream. Fast-forward push |
| Shared daemon | `f668b9db9-dirty` (merged build), reloaded 23:02Z. Rebuild after this commit to pick up the browser guard and `2fca0d192` |
| Worktrees added | `worktrees/pr-copilot-test-home-leak`, `worktrees/pr-communicate-test-home` (PR heads, approved) |
| Untracked | `scripts/local_model_probe.py` (not ours, leave it) |

## Commits and external effects

| Ref | What |
| --- | --- |
| `f668b9db9` | Merge upstream `8870f1993` (bridge `--help`, reload-bridge, harness working_dir) |
| `2fca0d192` | `bridge_reload::parse_cmdline` dead on macOS: clippy clean on the integration line |
| `ffe3ad277` | Communicate server tests set `JCODE_HOME`; they had read the real `agents.swarm_model` and failed 8-9 spawns |
| `08d491015` | Agent browser guard: every action but `status` refuses non-Gecko targets (Safari, Chrome, Edge, Brave, Chromium) |
| `5fb914af8` on `fork/master` | Upstream sync, 7 conflicts resolved, the fork's #1373 guard kept, upstream's fmt/clippy drift fixed, budgets refreshed |
| upstream #1487 | copilot test HOME leak |
| upstream #1489 | communicate test JCODE_HOME leak (before 11/19, after 19/19) |
| Firefox retirement | `~/.jcode/browser` and `~/Library/Application Support/Mozilla` moved to `~/.Trash/jcode-firefox-retire-20260924/` (operator approved; empty Trash to finalize) |

## Evaluation findings

Ranked by severity. None blocks the line.

1. **Fixed: test leak into the real config (2 cases).** `copilot_auth_tests`
   (earlier) and 13 communicate test setups. Both exist upstream; PRs filed.
   Full `jcode-app-core --lib` now passes with the real HOME and leaves
   `config.toml`'s hash unchanged. H3 sweeps the rest of the workspace.
2. **Fixed: agent browser could target personal browsers.** Details and
   decision: [`plans/2026-09-24-AGENT_BROWSER_PROVIDER.md`](plans/2026-09-24-AGENT_BROWSER_PROVIDER.md).
3. **OK: OpenAI streaming take-upstream.** Both `521474224` regressions
   (`late_named_earlier_call_…`, `missing_or_empty_call_id_…`) exist at HEAD on
   the keyed `ToolUseEndFor` API; empty call-ids fall back via `.trim().is_empty()`.
   `jcode-provider-openai`: 35 passed. The remaining `expect()` calls are on keys
   just collected from the same map, so they cannot panic.
4. **OK: fail-closed policies.** Socket perms: the daemon (`restrict_socket_pair_permissions`),
   the bridge (0600 chmod after bind, regression test) and ssh (0700 private dir) are all covered.
   Session tool policy: `Registry::execute` calls `check_session_tool_permission`
   before the SDK custom-tool branch, and `AgentTurn` without a policy bails.
   Failover prompts only for a configured alternate and never switches silently.
   `prune_active_pids_owned_by` only removes markers whose PID equals ours.
5. **OK: port completeness.** All 307 tests upstream added in the 0.88 range exist
   at HEAD. Of 36 fork-side test names that no longer exist, every sampled one
   was removed by an upstream commit that replaced its feature (grok ACP → HTTPS,
   palette role identity #1399, voice intent rework, multiedit merged into edit,
   keyed OpenAI streaming, browser detection). One file-level loss,
   `grok-build-runtime/tests/fake_acp.rs`, is the accepted ACP deletion.
6. **OK: ratchet claim.** 49 grown code-size entries; exactly 3 exceed the
   sum of both sides (`browser.rs` +64, `setup-hints/lib.rs` +19,
   `tool/browser.rs` +18), matching the receipt.
7. **Upstream CI finding (for U2).** Upstream CI fails at `cargo fmt --check`,
   so its clippy step never runs; about 30 clippy errors had accumulated
   unseen. fork/master now fixes them. The integration line was already clean.
8. **Tooling hazard.** Sharing one `CARGO_TARGET_DIR` across two worktrees on
   different trees produced phantom E0599/E0609 errors. Use a separate target
   dir per worktree (`~/.jcode/scratch/fm-target` was used for fork/master).

## Gates run (integration line, macOS)

- `cargo clippy --workspace --all-targets --all-features -D warnings`: clean.
- `cargo fmt --check`: clean. The size, test-size, panic and swallowed-error
  budgets are OK after the refresh in `08d491015`. The attribution is in its message.
- `cargo test -p jcode-app-core --lib`: 1577 passed (real HOME, config unchanged).
- `jcode-harness-api-server` 157, `jcode-provider-openai` 35, `jcode-base` copilot 52,
  `jcode-base` prompt 73, `tool::browser` 89: all pass.

## Lanes and quota (measured 23:35Z)

| Lane | State |
| --- | --- |
| Claude OAuth | 5h 61% (resets 02:30Z), 7d 61% (resets 09-25 07:00Z), Fable 7d 100% |
| ChatGPT prolite | 7d 100% until 2026-09-27 17:08Z |
| OpenCode Go | Key valid; weekly window not visible to jcode. Probe before spawning |
| OpenRouter | $72.36 / $115 left, $19.48 today, still no per-key cap |

## Continuation prompt

> You are the captain. Read `docs/HANDOFF_2026-09-24_OPUS55_SESSION_RESULTS.md`
> and `docs/plans/2026-09-24-PHASE_PLAN_POST_088.md`. Recheck line state and
> `jcode usage --json`. Rebuild and reload the daemon onto the integration line
> head (`selfdev build-reload`). Then run the plan in its stated order: U2+H4
> (extend PR #1354), H3, then U4/U6/H1 on Go if its window has reset, then U3/H2
> on Terra after 2026-09-27, then M1. Keep 20% quota headroom and report in
> under 5 lines.
