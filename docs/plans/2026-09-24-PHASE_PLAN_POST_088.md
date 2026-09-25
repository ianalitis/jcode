# Phase plan after the 0.88 merge (2026-09-24)

Admission, freeze and receipt follow `docs/HARNESS_LOOP_ARCHITECTURE.md`; this
file only lists nodes. Lanes are from `policy/providers.md`; with ChatGPT at
100% until 2026-09-27 and Go at its weekly limit, every node below is executed
by the Opus captain unless a lane has reset.

Evaluation that fed this plan: [handoff §Evaluation](../HANDOFF_2026-09-24_OPUS55_SESSION_RESULTS.md#evaluation-findings).

## Done in this session

| Node | Commit / ref |
| --- | --- |
| P0 daemon on merged build | `f668b9db9` build live |
| P0 upstream `8870f1993` into integration line | `f668b9db9` |
| P0 `fork/master` sync | `5fb914af8` (0 behind, 22 ahead) |
| P0 copilot HOME leak PR | upstream #1487 |
| P0 Firefox agent artifacts retired | moved to `~/.Trash/jcode-firefox-retire-20260924/` |
| Hermetic communicate tests | `ffe3ad277` |
| macOS clippy for bridge_reload | `2fca0d192` |
| P1 agent browser guard | `08d491015` |

## Remaining nodes

| ID | Node | Lane / effort | Writable paths | Gate | Validation | Depends |
| --- | --- | --- | --- | --- | --- | --- |
| U1 | Upstream PR: communicate tests JCODE_HOME isolation | captain / medium | `pr/communicate-test-home` | **done: #1489** | before 11/19, after 19/19 | none |
| U2 | Extend existing PR #1354 ("clear clippy 1.98 and rustfmt drift", CONFLICTING) instead of a new PR: rebase on `origin/master`, add the 0.88 macOS clippy fixes (`parse_cmdline`, setup-hints dead code, stdin_detect import, `await_holding_lock` test allows, fmt of 4 files). fork/master `5fb914af8` has the full set | captain / medium | #1354 head branch | upstream clippy + fmt clean on macOS | `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings` | none |
| U3 | Rebase PRs 1354, 1356 (CONFLICTING) | Terra / medium after 09-27 | their `pr/*` heads | mergeable, green | PR checks | none |
| U4 | Recheck our open issues against 0.88 (1113-1122, 1146-1154, 1176-1177, 1274, 1294, 1348-1352) | Go / none (read-only) | none | table of fixed/open | `gh issue view` | none |
| U5 | Upstream issue: agent `browser` auto-detect falls through to personal Safari | captain / low | `docs/upstream-feedback/` | issue filed | n/a | guard soak (1 week) |
| U6 | jev-pr-labeler PR: skip cleanly without key | Go / low | fork `ianalitis/jev-pr-labeler` | PR open | its CI | none |
| H1 | Split `jcode-base/src/browser.rs` (1789) and `jev.rs` (2152) under 1200, ratchet down | Go / medium | those files + new modules, `scripts/code_size_budget.json` | budgets lowered, tests green | `cargo test -p jcode-base --lib`; `python3 scripts/check_code_size_budget.py` | none |
| H2 | Fix TUI parallel JCODE_HOME races and SDK auth fixture timeouts at the root (shared test-env lock, see PR 1356) | Terra / high | test helpers only | 3 consecutive full-suite runs green | `cargo test -p jcode-tui --lib` ×3; `cargo test -p jcode-sdk` ×3 | U3 |
| H3 | Sweep for other tests that read the real `~/.jcode` (pattern of 928837a2b, ffe3ad277) | Go / low | tests only | config sha unchanged after full workspace test run | `shasum ~/.jcode/config.toml` before/after `cargo test --workspace --lib` | none |
| H4 | Upstream the integration line's fixes for 3 lib tests that fail on `origin/master` and `fork/master` on macOS: `rejects_non_utf8_git_paths` (needs `#[cfg(target_os = "linux")]`, APFS rejects the name) and the two tool-description token caps. Integration line passes all three | captain / low | fold into U2's PR | upstream lib test green on macOS | `cargo test -p jcode-app-core --lib -- rejects_non_utf8_git_paths tool_descriptions_stay_under_token_cap tool_parameter_descriptions_stay_under_token_cap` | U2 |
| M1 | jcode-bench baseline, Docker | captain / high | `docs/measurements/` | receipt with k≥2 | see BENCHMARK_RESEARCH | quota ≥40% |
| M2 | Routing arms A/B/C | captain + workers | `docs/measurements/` | decision recorded | see ROUTING_AND_DELEGATION | M1, capped key |

Order: U1, U2, H3, H4 (cheap, captain) → U4, U6, H1 (Go when reset) → U3, H2
(Terra after 09-27) → M1 → M2. U5 after a week of soak.

## Known flakes (not regressions)

- `server::tests::background_task_wake_runs_live_session_immediately_when_idle`:
  failed once under full-suite load on fork/master, passed 3/3 alone.
- TUI parallel render / JCODE_HOME races, SDK `auth::tests::processes` 300 ms
  fixtures (H2).
- Workspace-wide `cargo test --workspace --lib` (2026-09-25, 3 runs): one or two
  tests fail per run under load and a different set each time, all pass 3/3
  alone: `provider::tests::test_resolve_model_capabilities_uses_provider_hint`,
  `executor-pi agent_end_terminal_assistant_error_cannot_produce_success`,
  TUI `test_restore_session_with_selfdev_reload_tool_result_queues_continuation`,
  `test_real_draw_click_on_body_anchored_image_label_cycles_level`,
  `issue_1206_key_stream_preserves_escaped_prefix_until_submission`,
  `skill_invocation_with_prompt_attaches_pending_image_to_user_message`,
  `jcode-core avoiding_allocator_uses_every_available_identity_before_reuse`.
- Git-fixture tests (`jcode-sdk worktrees::`, `build-support dirty_source_state`,
  `prompt agents_md_resolves_linked_git_worktree_root`) fail when the developer's
  global git config signs commits and the key is locked. Run the suite with
  `GIT_CONFIG_GLOBAL=/dev/null`.

## Status update 2026-09-25 (06:20Z)

| Node | State |
| --- | --- |
| U2, H4 | Done: #1354 head `07db0864d` carries both. Its red checks are the upstream "Configure SSH for cargo git dependencies" step, which has no secret on fork PRs, not code |
| U4 | No upstream commit or PR fixes any of the 22 open issues (1110-1352). Go worker failed at first request (Go lane still unavailable) |
| U6 | Branch `skip-without-openrouter-key` in `~/.jcode/scratch/jpl` (fork clone): action skips with a notice when the key is empty; `tests/test_action.py` fails 1/2 before, 116/116 after. Not pushed: commit signing blocked |
| H3 | Done at the root: `jcode_storage::jcode_dir()` resolves to a per-process temp home inside `target/**/deps` test binaries when `JCODE_HOME` is unset. First sweep leaked 159 active-pid markers and 103 session files into the real home. After: no new markers or test sessions, config hashes unchanged |
| Prune | 9 worktrees of merged/closed PRs removed (branches kept); 7 stale scratch copies (~60G) and the 103 leaked sessions moved to `~/.Trash/jcode-prune-20260925/` |
| Blocker | `commit.gpgsign=true` with an SSH key that is not in the agent: every `git commit` waits on a passphrase. Operator: `ssh-add --apple-use-keychain ~/.ssh/id_ed25519` |
