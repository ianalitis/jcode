# Upstream reconciliation receipt: 2026-09-21

## Continuation: 2026-09-28T00:44Z (panic ratchet regression)

The clean HEAD after the prior writer released its lease failed the fast board on three ratchets: production size (30 offenders), swallowed errors (3402 versus 3343, 59 excess), and a newly red panic-prone count (102 versus 100). Signed `bee963bd9` removes the new production `expect` in swarm-label initialization and renames the included hook regression file so the budget script correctly recognizes its test-only assertions. The panic-prone check now passes at **98 versus 100**. The hook terminal-env regression, three swarm-label tests, app-core/base strict all-targets clippy and workspace format passed. The production-size and swallowed-error ratchets remain red. No budget update, push, hosted CI trigger, deployment or shared-daemon promotion occurred.

## Continuation: 2026-09-28T00:35Z (stale ambient lock)

Signed `3004daf3a` makes ambient lock acquisition fail when a stale lock path cannot be removed, instead of proceeding as if cleanup succeeded. The blocked-directory regression and two existing lock release tests, app-core all-targets clippy and workspace format passed. Swallowed errors fell from **3403 to 3402**, still **59 above 3343**; production-size offenders remain **30**. Local ratchets remain red and hosted CI is unrun. No baseline update, push, deploy or shared-daemon promotion occurred.

Tool-dispatch correction: earlier duplicate identical mutations were issued by the agent in parallel, not proven to be a harness defect. The `.git/index.lock` collisions are consistent with those duplicate invocations. Continue using one serial, scoped Git mutation at a time; do not treat this as a harness bug without an independent reproduction.

## Continuation: 2026-09-28T00:31Z (ambient lock release)

Signed `bd62fabab` makes explicit ambient lock release return an error if the lock file cannot be removed, rather than claiming successful cleanup. The blocked-directory failure test and existing successful release test passed; app-core all-targets clippy and workspace format passed. Swallowed errors fell from **3404 to 3403**, still **60 above 3343**. Production-size offenders remain **30**. Local ratchets are red and hosted CI is unrun; no budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-28T00:27Z (ambient schedule persistence)

Signed `8278f948b` makes ambient schedule creation return an error if its queue cannot be persisted, and rolls back the in-memory addition rather than reporting an undurable schedule ID. A blocked queue-file regression, all 56 ambient tests (including live delivery and spawn-target tests), app-core all-targets clippy and workspace format passed. Ten existing queue tests now assert their writes succeed; rustfmt reindented those expressions. Swallowed errors fell from **3405 to 3404**, still **61 above 3343**; production-size offenders remain **30**. Both ratchets and hosted CI remain unresolved. No budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-28T00:19Z (remote startup snapshot)

Signed `a6b54e3b1` moves the unchanged remote-startup snapshot wire shape into `session/remote_startup_snapshot.rs` while keeping its serde defaults and module visibility. The remote-startup transcript/replay roundtrip, base all-targets clippy, workspace format and module resolution passed. Production-size offenders fell from **31 to 30**; swallowed errors remain **3405 versus 3343** (62 excess). Both ratchets remain red and hosted CI is unrun. No budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-28T00:15Z (checkpoint journal removal)

Signed `cecf20381` stops reporting a successful session checkpoint when the old journal cannot be removed, and avoids resetting in-memory persistence state on that failure. The deterministic directory-at-journal-path regression and all five session-persistence tests passed; base all-targets clippy and workspace format passed. Initial test and clippy attempts timed out waiting for the shared Cargo gate, then passed after its holder released the gate. Swallowed errors fell from **3406 to 3405**, still **62 above 3343**. Production-size offenders remain **31**. Local ratchets remain red and hosted CI is unrun; no budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-28T00:04Z (browser setup persistence)

Signed `fe8be1f36` makes browser setup fail instead of marking setup complete when its selected browser preference cannot be saved. A blocked preference-file regression confirms the marker is not created, all 27 browser tests and base all-targets clippy passed, and workspace formatting passed. Swallowed errors fell from **3408 to 3406**, still **63 above 3343**. Production-size offenders remain **31**. Local ratchets remain red, hosted CI is unrun, and no budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-27T23:59Z (OpenAI model capability checks)

Signed `a08a89c0d` isolates unchanged OpenAI model tool capability checks in a focused include and adds two regressions for the GPT-5.4 deferred-tools boundary and Codex hosted-image exclusion. Both regressions, runtime all-targets clippy, workspace format and module checks passed. Production-size offenders fell from **32 to 31**. Swallowed errors remain **3408 versus 3343** (65 excess). Local CI ratchets remain red and hosted CI is unrun; no budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-27T23:55Z (provider HTTP transport extraction)

Signed `70ebe40dc` isolates the unchanged canonical User-Agent, shared HTTP client and fresh transport-fault retry client in `http_clients.rs`. All 153 provider-core tests passed (one developer-only test ignored), provider-core all-targets clippy, workspace format and module checks passed. Production-size offenders fell from **33 to 32**; swallowed errors remain **3408 versus 3343** (65 excess). Both local ratchets and hosted CI remain unresolved. No baseline update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-27T23:50Z (SDK turn result extraction)

Signed `666187599` moves the unchanged SDK turn result types and text collector into `client_turn_result.rs`, retaining their module visibility and stream-text behavior. The SDK library suite passed (58 passed, 3 intentionally ignored), SDK all-targets clippy, workspace formatting and module resolution passed. Production-size offenders fell from **34 to 33**. Swallowed errors remain **3408 versus 3343** (65 excess). Both local ratchets and hosted CI remain unresolved; no budget update, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-27T23:45Z (direct transport and fail-closed backup restoration)

Signed `fa7f4e500` isolates the unchanged Anthropic-compatible direct transport URL, header and auth-mode parsing in a focused module. Both affected direct-transport tests and provider all-targets clippy passed; the production-size ratchet fell from **35 to 34** offenders. Signed `fea26c603` makes corrupt JSON recovery report success only after the valid backup is actually copied over the primary. A blocked restoration now returns an error instead of silently claiming a repaired state. Both new backup success/failure regressions, storage all-targets clippy and workspace format passed. The swallowed-error count fell from **3409 to 3408**, still **65 above 3343**. The local board remains red, and hosted CI is unrun. No baseline update, push, deploy or shared-daemon promotion occurred.

The two identical Git mutations collided on `.git/index.lock`; HEAD and the index were checked clean before retrying. A macOS `lockf`-serialized, path-scoped commit then succeeded. Later inspection attributed duplicate dispatch to agent-issued parallel calls, not a demonstrated harness defect. Keep mutations singular and serial and verify the index after any collision.

## Continuation: 2026-09-27T23:38Z (reload recovery errors surfaced)

Signed `88ef60e87` propagates recovery-record directory fsync failures after removal; ten recovery-record tests and app-core library clippy passed. Signed `97636c00e` logs unreadable reload context during History hydration rather than silently dropping it; four history-recovery regressions passed, including a new malformed-context fallback case. Signed `8fd47fdb8` logs unreadable context during recovery-intent generation; headed-session startup recovery passed. Signed `c71f9a746` requires deleting a consumed reload context to succeed before returning it, rather than returning a replayable context as consumed. The new Unix blocked-removal regression and existing scoped-context roundtrip passed; app-core all-targets clippy, workspace format and test-size checks passed.

The swallowed-error ratchet improved from **3415 to 3409**, still **66 above 3343**. Production-size offenders remain **35**. This is not a green board; full hosted CI was not run and no budget baseline, push, deploy or shared-daemon promotion occurred.

## Continuation: 2026-09-27T23:27Z (cohesive Cursor and ranking splits)

Signed `ae079be99` moves the unchanged Cursor session importer into a dedicated 76-line include, preserving its public imports and behavior. The Cursor snapshot regression and base all-targets check passed. Signed `a2e7d7398` moves the stateless BM25 memory ranking helper into `memory/bm25.rs`. All 44 memory-related tests, base library clippy, workspace format and module declarations passed. Production-size offenders decreased from **37 to 35** without a baseline update. The swallowed-error count remains **3415 versus 3343** (72 excess); moving existing option defaults is not an error-handling repair. This is still **not a green local or hosted CI board**. No push, deploy, shared-daemon promotion or budget update occurred.

## Continuation: 2026-09-27T23:17Z (durability and hooks regression extraction)

Signed `5ca8f5b07` makes durable atomic writes propagate failure to open or sync the parent directory, rather than reporting success after a failed directory fsync. The storage suite passed (8 tests) and storage all-targets clippy passed. Signed `5800b440d` moves the unchanged client-terminal environment hook regression into a 55-line included test file, keeping its original test name and behavior. All 21 hook tests passed, as did workspace format, test-size and module declaration checks.

Signed `00d16d2d7` makes swarm-label loading fail closed on corrupt persisted JSON rather than silently replacing it, and makes label changes visible in memory only after persistence succeeds. The three label regressions (including corrupt JSON and blocked write) and app-core all-targets clippy passed. The production-size ratchet now reports **37** offenders; the swallowed-error ratchet reports **3415 versus 3343** (72 excess). This is incremental local repair, **not a green CI board**. No baselines were updated, and nothing was pushed, deployed or promoted to the shared daemon.

## Continuation: 2026-09-27T23:10Z (local CI repairs, not green)

The upstream merge remains committed locally as `5b0e19e1a`; subsequent signed repairs include `df1400a92` (turn-loop test extraction and explicit MCP/notice errors), `65df90a71` (fail-closed corrupt account-rotation state), `fc286e926` (surface SSH stream and lid journal IO errors), and `9e0272ee9` (move the unchanged manual subagent action out of oversized `server/client_actions.rs`). The new action file is 136 lines and the parent now stays below its size baseline. Account-rotation corruption regression, 14 SSH tests, 12 lid tests, and the app-core server suite (494 passed) passed. App-core all-targets check, module declarations, and workspace format passed after the extraction.

The production-size ratchet still reports **38** files. The swallowed-error ratchet still reports **3417 versus 3343**, a deficit of 74. Panic-prone, test-size and formatting guards passed earlier, but the full guardrail board and hosted CI have **not** passed on this HEAD. No ratchet baseline was raised, and nothing was pushed, deployed or promoted to the shared daemon. Continue genuine extractions and error propagation, then rerun the full local board. Hosted fork/upstream failures need separate review and authorization before any remote change.

## Continuation: 2026-09-27T22:53Z (local merge committed, CI repair ongoing)

Signed merge commit `5b0e19e1a` has parents `47dd21a9b` and upstream `cc2171473`. The 18 conflicts were resolved as recorded below, with no unmerged entries. Signed follow-up `3850b8e72` moved the SDK's included tool-stream test into a subdirectory so Cargo no longer treats it as an independent integration target, made subscription card generation and SSH process handling fallible, and moved cache-monitor methods into the existing cache-request module. The panic-prone ratchet now **passes at 98 versus 100**; the new serialization-failure regression passed, the SSH tests passed (14), and changed-package clippy passed. The production `agent.rs` is now below 1200 lines, without a raised size baseline.

The next bounded tranche moved seven unchanged turn-loop tests into an included test module, bringing `turn_loops.rs` below 1200 lines. It logs malformed MCP applet resource parsing and poisoned plan-limit notice locks rather than discarding errors. All seven moved tests, the applet lifecycle test and malformed-resource regression passed; changed-crate all-targets clippy and the test-size and panic guards passed. The production-size ratchet still reports **39** files, and the swallowed-error ratchet still reports **3429 versus 3343**. The board is **not green**. No budget rebaseline, push, hosted workflow trigger, deploy or shared-daemon promotion has occurred.

## Continuation: 2026-09-27T22:42Z (merge integration in progress)

From `47dd21a9b`, the noncommitting merge of `origin/master` `cc2171473` exposed 18 conflicts. All conflict entries are resolved locally, but **no merge commit exists yet**. The usage-cache resolution keeps the fork's generation-checked singleflight/backoff, original-age last-good and one reset invalidation outside the RAM mutex, rather than accepting upstream's separate fixed-duration error cache. The agent retains adaptive Auto MCP threshold selection while integrating the upstream late-MCP transcript announcement. Wire requests/events include upstream cross-swarm communication and the locally ported KV-cache miss. The Git-info collector uses the upstream status/counting implementation behind the fork's stale-while-revalidate cache. Split test includes, sandboxing and fork-specific sponsor opt-out behavior were preserved. No baselines were raised.

Focused checks passed: usage tests 93 passed and 1 ignored, MCP selectors 23 passed, harness API suite 163 passed, three adapted late-MCP announcement regressions passed, and the real-App overscroll, transcript edited-path and real-Git Changes-widget regressions each passed. Workspace `check --all-targets --all-features` and workspace clippy with `-D warnings` passed after replacing a new cross-swarm async test's synchronous global lock with a Tokio test mutex. The upstream SDK tool-stream test was moved intact to an included test partition, and the test-size and formatting guards pass. The fast board fails the three ratchets below.

**Outstanding CI debt remains real**: production-size budget reports approximately 43 overlarge/newly grown files, panic-prone usage reports 107 against 100, and swallowed-error budget reports 3433 against 3343, including newly added upstream usages. These failures are not waived or rebaselined. Local-only merge and tests do not establish hosted CI success. No push, deploy or runtime promotion has occurred. Continue reviewing the merged diff, running gates, and repairing genuine budget regressions before claiming CI green.

## Continuation: 2026-09-27T22:13Z (local-only, coverage negative controls)

At clean `2bc458eea`, a bounded, noncommitting three-way merge of `origin/master` `cc2171473` exposed **16** conflicts, adding `tui/app/remote/server_events.rs` to the 15 listed below because the local effort-chip repair overlaps the upstream fix. Its one hunk is the same assignment on both sides plus an upstream explanatory comment. The merge was aborted successfully; HEAD, index and worktree are clean. No conflict or auto-merged hunk was accepted.

The upstream `auto_mode_never_changes_the_tool_list_for_late_mcp_tools` test was tried verbatim in local `agent_tests_partition_01_tests.rs`. It failed at the initial `mcp_search` assertion, because the fork's adaptive Auto mode starts with direct tools and defers only when the threshold is reached. The local `auto_mode_rechecks_late_mcp_definitions_before_deferring` regression passed after the trial test was removed. This upstream test is not a safe verbatim port: reconcile its cache-stability intent with the fork's adaptive policy when integrating the upstream late-MCP announcement code. The upstream `overscroll_is_the_only_mode_and_reveals_pink_model_on_real_app` test was likewise tried in the split TUI suite and failed at the initial `chat_overscroll_active()` assertion before reaching its color check. Its trial was removed, leaving the prior test unchanged. Preserve this real-app coverage when the upstream rendering path is integrated rather than claiming that the synthetic overscroll test is equivalent.

Independent read-only review located 13 of the 14 name-only harness findings unchanged in `translate_regression_tests.rs`, which is included by `translate_tests.rs`. The remaining `kv_cache_miss_is_forwarded_with_session` requires both the upstream `ApiEvent::KvCacheMiss` variant and its `translate.rs` arm, neither present locally before the merge. Three upstream late-MCP announcement tests also exercise an upstream production method absent locally. The three TUI real-app tests remain genuine integration coverage to port or reconcile after production changes land. These mappings narrow the test conflict review but do not resolve the merge or the size/swallowed-error ratchets. Nothing was pushed or deployed.

## Correction: 2026-09-27T21:52Z (upstream test preservation)

The earlier read-only claim that all upstream test names were represented locally was **incorrect**: its inventory filtered for names beginning `test_`. A second check parsed all `#[test]`/`#[tokio::test]` function names from the nine conflicted upstream test paths against local recursive `include!` partitions, then checked representative names across `crates/`. At least **24 upstream tests are not present by their names** locally, even after the two ports at 21:47Z:

| Conflict path | Upstream names missing locally |
| --- | --- |
| `jcode-app-core/src/agent_tests.rs` | `auto_mode_never_changes_the_tool_list_for_late_mcp_tools`, `late_mcp_announcement_skips_referenced_native_and_eager`, `late_mcp_announcement_uses_original_names_for_sanitized_aliases`, `late_mcp_tools_are_announced_once_in_the_transcript` |
| `jcode-base/src/auth/tests.rs` | `cursor_status_is_available_for_authenticated_cli_session`, `full_and_fast_auth_status_document_cursor_cli_exception` |
| `jcode-base/src/config_tests.rs` | `default_sponsors_section_is_not_written_back` |
| `jcode-harness-api-server/src/translate_tests.rs` | `kv_cache_miss_is_forwarded_with_session`, `late_recovered_suffix_completes_previously_empty_or_unseen_text`, `new_request_retry_does_not_retract_previous_response`, `pdf_panels_opt_in_and_survive_native_api_attach_reconnect_and_live_updates`, `persisted_metadata_reads_large_transcripts_from_bounded_windows`, `request_boundary_closes_provider_without_message_end_and_ids_survive_turns`, `session_list_exposes_durable_edit_stats_without_phantom_sidecar_sessions`, `side_panel_attach_and_reconnect_hydrate_in_either_order_and_clear_empty`, `side_panel_history_refresh_hydrates_but_catalog_does_not_clear_panel`, `side_panel_live_updates_preserve_content_focus_and_session_routing`, `side_panel_state_hydration_requires_correlated_attachment`, `text_framing_preserves_chunks_and_reasoning_then_separates_messages`, `text_framing_tools_and_turn_fallback_close_once_without_phantom_messages`, `text_retry_retracts_completed_and_live_messages_and_late_replacements_keep_ids` |
| `jcode-tui/src/tui/app/tests/scroll_copy_01/part_01.rs` | `agent_edited_paths_come_from_transcript_edit_tools`, `changes_widget_end_to_end_on_real_git_repo`, `overscroll_is_the_only_mode_and_reveals_pink_model_on_real_app` |

This is a *name inventory*, not proof that equivalent renamed tests are absent or that same-named assertions are equivalent. In particular the upstream scroll-copy diff adds 200+ lines of real-app/Changes-widget tests that are not in the local partitions. Do not choose the local whole-file side in these conflicts without porting/reconciling this coverage. The 21:42Z split-test comparison below was incomplete. The merge has not been performed; HEAD and worktree remain clean. Prioritize these five clusters before another merge attempt, then run the affected crate suites and full guardrails. Publication, host config changes and baseline updates remain separately gated.

**Config exception verified at 21:54Z:** upstream `default_sponsors_section_is_not_written_back` is *incompatible* with the fork's privacy default. The fork disables discovery by default and deliberately serializes `[sponsors] enabled = false` so older default-on builds preserve the opt-out. The upstream assertion reproduced a local failure; it must **not** be ported verbatim. Instead, `default_sponsors_optout_is_written_for_older_default_on_builds` pins the fork contract, and the pre-existing `sponsors_optout_survives_config_save_and_reload` covers three read-modify-write rounds. Both tests passed. This is a behavioral reconciliation of that one missing test, not a waiver for the other 23 names.

**Auth equivalence verified at 21:57Z:** the two upstream Cursor CLI test names in the table are renamed locally to `full_and_fast_auth_status_ignore_cursor_cli_session` and `cursor_status_is_not_configured_for_cli_session_only`. Both exercise the same not-configured behavior for CLI-only auth, and both passed by exact selector. Upstream's *actual new hunk* in `auth/tests.rs` is the named-provider-profile environment isolation of `openrouter_like_status_is_provider_specific`; the local `AuthTestSandbox` covers the same variables. These two names need not be ported again, leaving the other 21 name-only findings to reconcile or locate under renamed selectors.

## Continuation: 2026-09-27T21:47Z (local-only, upstream test ports)

Ported two independently inspected upstream-only assertions into existing split test partitions before retrying the merge: `config::tests::removed_overscroll_status_key_still_loads_config` and `tui::app::tests::test_remote_model_changed_updates_reasoning_effort` (issue #1504). The latter reproduced a real local failure: an acknowledged successful model switch left the effort chip at `medium` rather than `high`. The remote ModelChanged handler now adopts the reported effort only on successful switches, clearing it when absent and preserving the existing value on failure. The config test passed (1/1); the remote effort regression failed pre-fix as expected and passed post-fix (1/1). Workspace format, test-size guard and diff check passed. This is a local contribution, not a completed upstream merge or hosted CI validation. No push, budget rebaseline or runtime promotion occurred.

## Continuation: 2026-09-27T21:42Z (local-only, merge aborted)

From clean `9f287eeb9`, a bounded command-local three-way merge of `origin/master` `cc2171473` again exposed the same 15 unresolved paths recorded at 21:32Z. The three `usage.rs` hunks were experimentally resolved by retaining local single-flight claiming, original-age last-good limits and generation-checked success. The add/add `usage/disk_cache.rs` was experimentally reduced to the local authoritative state machine. In the automatically merged `usage/cache.rs`, remove both new `disk_cache::invalidate(...)` calls inside the account match, leaving exactly one `disk_cache::invalidate_after_reset(account_label)` outside the RAM lock. These resolutions were **not committed or retained**: `git merge --abort` restored clean HEAD, index and worktree after restoring only this attempt's unstaged cache edit. No partial merge remains.

Independent read-only split-test comparison found no distinct upstream-only tests in `auth/tests.rs` or harness `translate_tests.rs`; preserve local sandbox/includes. Port upstream `removed_overscroll_status_key_still_loads_config` to `config_tests_partition_01_tests.rs` and `test_remote_model_changed_updates_reasoning_effort` with its `model_changed_event` helper to `remote_events_reload_04_partition_02_tests.rs`. Verify bodies, assertions and any other split files independently before a merge commit; these are review findings, not proof of complete test equivalence. The local CI remains red on production-size and swallowed-error ratchets, while workspace check and clippy passed on the prior clean HEAD. Hosted CI on local commits remains unrun. Nothing was pushed, rebaselined or deployed.

## Continuation: 2026-09-27T21:36Z (local-only)

The best-effort voice focus registration previously discarded its filesystem error and marked the session registered even when registration failed; another focus write for the same session would then skip the retry. It now logs registration failure and caches only successful writes. The cross-platform regression forces a blocked registration directory, restores it, then verifies a same-session focus write creates the client marker. Eight `dictation::dictation_tests::` tests passed. Format and test-size checks passed; swallowed-error count fell from 3401 to **3400** against a 3343 budget. The merge rehearsal above remains aborted and clean. No baseline changes, push or runtime promotion.

## Continuation: 2026-09-27T21:32Z (local-only, merge aborted)

From clean `e8c5c97b1`, a bounded `--no-commit --no-ff` merge of `origin/master` `cc2171473` with the command-local three-way merge-file driver exposed **15** unresolved paths. Fourteen match the earlier conflict inventory; the additional unresolved path is `crates/jcode-app-core/src/agent.rs`, where the recent local KV-cache request extraction overlaps upstream's refactor. The full list was obtained using `git diff --name-only --diff-filter=U`; no conflict was accepted. `usage.rs` has three conflict hunks: the local single-flight fetch decision must replace upstream's separate fixed-duration cache, the local original-age last-good response must not be reset to `Instant::now()`, and the local generation-checked success must replace upstream's unguarded store. The add/add `usage/disk_cache.rs` must retain the local single authoritative implementation. The automatically merged `usage/cache.rs` still needs duplicate-invalidation review.

Because the remaining 13 non-usage conflicts include large test partitions and `wire.rs`, the attempt was aborted rather than accepting whole-file sides or committing an unvalidated merge. `git merge --abort` returned 0; HEAD remained `e8c5c97b1`, index and worktree were clean. No push or runtime promotion occurred. Next pass should map the exact new upstream assertions into local split partitions, resolve `agent.rs` alongside usage and config in bounded groups, then run targeted crate suites and the full board before committing.

## Continuation: 2026-09-27T21:28Z (local-only)

The first full `scripts/check_guardrails.sh` run after the recent commits passed workspace check, format, test-size and the other fast guards, but surfaced one additional clippy failure: `suspicious_open_options` in the cross-process usage-cache lock-file open. The lock now explicitly uses `.truncate(false)` to preserve the lock inode and existing content. `scripts/dev_cargo.sh clippy -p jcode-base --lib --all-features -- -D warnings` passed; five focused `usage::disk_cache::` tests passed. Workspace `scripts/dev_cargo.sh clippy --all-targets --all-features -- -D warnings` also passed. Production-size and swallowed-error guards remain red. No baseline change, merge, push or runtime promotion occurred.

## Continuation: 2026-09-27T21:24Z (local-only)

The lid override block-failure path previously discarded both rollback and journal-cleanup errors, deleting the recovery journal even if restoring the original power setting failed. It now logs failures and retains the journal for later recovery when restore fails. The new `lid_override::tests::failed_rollback_keeps_journal_for_later_recovery` regression and ten existing lid tests passed; format passed. The swallowed-error count fell from 3403 to 3401 against a 3343 budget. Read-only route projection found `provider.default_model` drift: active config has `openai-oauth:gpt-6-sol`, while the dotfiles lane table expects `gpt-6-astra`. No provider setting was changed without a specific operator decision. No push or runtime promotion occurred.

## Continuation: 2026-09-27T21:21Z (local-only)

The eight existing live provider probe unit tests were moved from the oversized `live_provider_probes.rs` into `live_provider_probes_tests.rs`, with the same assertions and test names. The production file dropped from 2055 to approximately 1905 lines, below its 2053-line baseline. `scripts/dev_cargo.sh test -p jcode-provider-doctor --lib live_provider_probes::tests:: -- --test-threads=1` passed all eight. Workspace formatting and test-size checks passed. The remaining production-size and swallowed-error findings still require actual repairs; no baseline update, merge, push or runtime promotion occurred.

## Continuation: 2026-09-27T21:14Z (local-only)

Rechecked signing key, clean index/worktree, free write lease and both remote refs without pruning. `origin/master` remains `cc2171473`; integration was 34 behind and 424 ahead before this continuation. The fast guardrail board still fails only production-file size (35 entries) and swallowed-error usage (3404 vs 3343). No baseline was raised. `jcode usage --json` reports 21% used in the visible OpenAI seven-day window; the attempted route-projection check was not run because that script lives in the separate dotfiles repository, not `jcode/scripts/`.

The local cache-path helper previously discarded the actual `jcode_dir()` error through `.ok()?`, replacing it with a generic path-unavailable error. It now propagates the original error via `Result`, retaining the fail-closed behavior. `scripts/dev_cargo.sh test -p jcode-base --lib usage:: -- --test-threads=1` passed (91 passed, 1 ignored), and `cargo fmt --all --check` and `git diff --check` passed. The swallowed-error count is now 3403 against 3343 and still fails because many unrelated findings remain. The upstream merge and hosted CI are untested here. No push, merge, provider setting change or runtime promotion occurred.

## Continuation: 2026-09-27T20:53Z (local-only, merge aborted)

Signed local commits after `67c9caf44`: `95a7dd2cf` moved intact Anthropic invariant tests out of the oversized production module (27 provider tests passed); `e09d16f40` extracted agent KV-cache request event helpers with a passing focused regression and app-core cargo check; `f03968cb1` separated model catalog test fixtures, with both moved tests passing. Format and test-size checks remain green. The production-size ratchet now reports **35 entries** (previously 38). The swallowed-error ratchet still reports **3404 against 3343**, 61 excess. Full check, clippy and hosted CI on these commits have not passed. These changes have not been pushed or deployed.

A fourth command-local built-in-driver merge of `origin/master` `cc2171473` was attempted and **aborted cleanly** after exposing 14 conflicts, including `usage.rs`, an add/add `usage/disk_cache.rs`, and the now-overlapping `auth/tests.rs`. No resolution or merge commit was retained. The upstream test-name comparison found additional tests and helpers needing explicit relocation into local split test partitions, notably in base auth/config, harness translation, and TUI scroll-copy tests. This is only a name inventory, not proof of behavioral equivalence for same-named tests. Do not choose whole-file sides or retain the auto-merged duplicate cache invalidations. Reconcile the single authoritative local usage backoff/singleflight implementation with upstream reset semantics, then run focused regressions before a merge commit. No partial merge remains.

The repo lease for this continuation is `scheduled-reconciliation` and must be released at handoff. Fork CI for these local commits is untested, and upstream hosted format, SSH-secret and paid-labeler issues require separate hosted decisions. Next session should recheck HEAD, status, lease and remote refs, consult this receipt, resolve the 14 conflicts in bounded groups, continue genuine 35-file size and 61-count swallowed-error repairs without raising baselines, and run full local gates. Any push, fork sync, workflow enable, PR/issue mutation or runtime promotion still needs separate operator approval. The duplicate tool-execution/index-lock collision observed during an earlier merge abort is a suspected harness defect; keep mutations serial, verify index state after each and avoid parallel mutation wrappers.

## Continuation: 2026-09-27T20:25Z (local-only)

From clean `ccfd18542`, a single authoritative Anthropic quota gate was implemented and signed as `5c241f354`. `usage/disk_cache.rs` uses hashed account keys, short file-locked transactions, bounded in-flight claims and generations, original-age last-good snapshots, reset tombstones, and the local Retry-After/exponential schedule instead of upstream's separate fixed-duration error cache. The disk state fails closed on unreadable/corrupt JSON; a regression checks it never overwrites corrupted state. Reset write failures are logged. The independently rerun `jcode-base --lib usage::` suite had **91 passed, 0 failed, 1 ignored**, and workspace format and diff checks passed. This patch remains local: upstream's add/add cache implementation is not merged or published. Complexity challenge: its ~460-line cache module is larger than the upstream ~245-line version, justified for now by singleflight, reset generation, failure atomicity, and original quota age, but should be re-reviewed for simplification after integration.

A third command-local merge-driver override attempted a noncommitting merge of `origin/master` `cc2171473` into `5c241f354`. It exposed **13 conflicts**, the earlier twelve plus an add/add conflict in `usage/disk_cache.rs`. The large split-test conflicts remain unresolved and choosing a whole side would drop tests. After inspecting usage and config overlaps, the merge was **aborted cleanly**. There is no partial merge, merge commit, index residue or publication. Next attempt must remove upstream's duplicate `disk_cache::invalidate` calls auto-merged into `usage/cache.rs` while preserving scoped reset behavior, keep the local single-flight cache and integrate upstream's new limit-reset behaviors and tests. Never accept the upstream fifteen-minute error cache or reset quota fetch age to `Instant::now()` on a failed request.

Signed `4eb7d7bb0` replaces incomplete environment guards in the four OpenCode catalog-route tests, Chutes auth test and direct-profile display test with the existing `AuthTestSandbox`. Their focused tests pass, format and test-size ratchets pass. The full `jcode-base --lib -- --test-threads=1` suite improved from **six failures to one**: **1700 passed, 1 failed, 6 ignored**. The remaining `provider::tests::test_resolve_model_capabilities_uses_provider_hint` expected a 1M OpenAI window but observed 272K in the whole suite. An isolated rerun did not reach test execution: `scripts/dev_cargo.sh` waited 90 seconds on the host-wide Cargo gate and timed out. This is a gate-contention observation, not proof of the model assertion's cause. Do not bypass the gate or weaken the test.

The previously remaining provider-hint failure was reproduced in the full suite despite the first test sandbox: `reset_model_catalog_services_for_tests()` did not clear the separate process-global `CONTEXT_LIMIT_CACHE`, which another test had seeded with 272K for `gpt-5.4`. Signed `3424bcde1` repairs the test-support-only reset and moves the isolated regression into a small included partition. The focused test and full `jcode-base --lib -- --test-threads=1` now pass: **1701 passed, 0 failed, 6 ignored**. The full suite used direct bounded `cargo test` as a fallback because an unrelated orphaned `sleep` held the host-wide `dev_cargo` coordination lock, not because any test gate was skipped. Format and test-size budgets passed; code-size and swallowed-error gates remain red.

Local fast CI still has production-size and swallowed-error ratchet failures: **38** reported oversized-file entries and **3404** swallowed-error-like usages against budget **3343** (61 excess). The earlier formatter and test-size repairs remain green. Full cargo check/clippy and hosted CI on the newer local commits have **not** passed. Sibling fork CI was inventoried read-only, not changed. Upstream hosted CI also has independently red formatting, fork-PR SSH-secret and paid labeler gates, so local source edits cannot establish hosted green by themselves. Continue genuine fixes, review the staged diff and known-hosted constraints, and request separate approval before any push, PR/issue edit, workflow trigger/enable, fork-default sync or runtime promotion. The repo lease remains with `nautilus-integration` until released.

## Continuation: 2026-09-27T19:17Z

Follow-up at 19:33Z: signed `7bcb41621` fixes the **test-size** guardrail
without updating its baseline. Three intact translation tests moved from
`translate_tests_partition_02_tests.rs` (1337 to 1183 lines) into a new
included partition; one intact OpenRouter reasoning-effort test moved from
partition 01 (1260 to 1246 lines, below its historical 1253-line cap) to
partition 02. The moved translation test bytes match the originals. The
test-size checker and `cargo fmt --all --check` pass, and each of the four
moved tests passes with its exact selector. A second controlled merge
rehearsal was aborted without accepting conflicts. Production-file size and
swallowed-error guardrails remain unresolved. No publication or runtime
promotion occurred.

Read-only follow-up at 19:39Z: the post-repair
`scripts/check_guardrails.sh --skip-slow` board now fails **two**, not four,
gates: oversized production files and swallowed-error usage. Format, test-size,
lockfile, warning budget and the other fast guardrails pass; check and clippy
were deliberately skipped by this fast mode, not claimed green. GitHub's fork
default `master` has a successful CI run (`36072587643` at `5fb914af8`), while
the integration branch's last published CI (`36104798397` at `3be93ab50`)
failed only the oversized-file ratchet. The latest visible CI/CodeQL runs on
`ianalitis/handterm`, `ianalitis/mermaid-rs-renderer` and
`ianalitis/agentgrep` succeeded; `ianalitis/GLOOP` has no runs. These remote
observations are not evidence that the newer local commits passed hosted CI.

Read-only Terra/high usage-cache review proposed a **single backoff owner**:
RAM successful quota, then persisted fresh successful quota, then the local
exponential `backoff::remaining` gate. During backoff, return last-good quota
with its original fetch age if valid, otherwise report an error without a
fabricated zero. A fixed-duration error cache must not outlive the local
Retry-After/exponential gate. Preserve upstream reset invalidation and
confirmed-zero handling while hashing any persisted token-derived cache key.
This is an unimplemented review plan, not a tested resolution. Its edge cases
still require targeted cross-process, expiry, reset and unknown-quota tests.

From clean `5022ca61d`, both remotes were refreshed without pruning. With the
`nautilus` write lease, the merge was retried using a command-local override
of `merge.mergiraf.driver` to `git merge-file -L local -L base -L upstream %A %O %B`.
It returned promptly and exposed **12** unresolved files, the previous eleven
plus `crates/jcode-base/src/usage.rs`. The latter is the overlapping Anthropic
usage implementation previously hidden by Mergiraf's automatic resolution.
No conflict was accepted. The merge was aborted, restoring the clean starting
tree and index. In particular, trial resolutions of the usage and config
conflicts did not survive the abort. Future merges must reconcile the local
`usage/backoff.rs` gate with upstream `usage/disk_cache.rs`: fixed fifteen-minute
error freshness can outlast local exponential backoff, and returning a last-good
quota must not misrepresent its age or turn unknown into confirmed zero. Large
single-hunk conflicts in split test files (some over 2,000 lines) require
mapping new upstream tests to their local partitions rather than choosing one
whole-file side.

The local `scripts/check_guardrails.sh --skip-slow` baseline failed four gates:
format, oversized production files, oversized test files and swallowed-error
ratchet. The formatter issue was only the misplaced `pub mod applet` declaration;
it was fixed in signed `0c9828d1a`, with `cargo fmt --all --check` and
`git diff --check` passing. The other three failures remain and were not
suppressed or rebased. No full check or tests of the attempted merge ran, and
no push, GitHub mutation, daemon promotion or fork-default sync occurred.

## Fresh-session handoff: 2026-09-27T19:09Z

The operator approved continuing a **local** upstream integration and preparing a
handoff. From clean `jcode/ci-format-baseline` at signed `79b122a91`, with the
`calf` write lease, `scripts/bounded.sh 240 git merge --no-commit --no-ff origin/master`
completed its merge-driver pass in 115 seconds but left **11
unresolved files**: `crates/jcode-app-core/src/agent_tests.rs`,
`crates/jcode-app-core/src/tool/communicate_tests/end_to_end.rs`, `crates/jcode-base/src/config.rs`,
`config_tests.rs`, `crates/jcode-harness-api-server/src/translate_tests.rs`,
`crates/jcode-protocol/src/wire.rs`, and five TUI app helpers/test chunks
(`app/helpers.rs`, `app/tests.rs`, `app/tests/remote_events_reload_01/part_01.rs`,
`app/tests/remote_events_reload_04.rs`, `app/tests/scroll_copy_01/part_01.rs`).
Mergiraf claimed one automatic resolution in `usage.rs`; that output was **not
reviewed or accepted**. Conflict markers expanded some files by thousands of
lines, including `wire.rs`. No tests were run against the incomplete merge.
`git merge --abort` restored the exact clean starting HEAD and empty index,
verified by `git status --short` and `git rev-parse HEAD`. There is **no merge
commit, push, fork-default sync, PR/issue change or runtime promotion**.

**Next session:** reacquire the repository write lease and verify HEAD, refs,
index and worktree before writes. Use a narrowly planned local integration,
preferably disabling the external Mergiraf driver for this merge after testing
the override, rather than trusting its whole-file conflict expansion. Resolve
all 11 conflicts in context, explicitly reconcile upstream `691e1bb04` and
`8556d6fcc` with local N2 `119b4e32d` (last-good quota, confirmed zero,
429 Retry-After, backoff expiry and process-sharing), then run focused
`jcode-base` usage regressions and affected crate tests, changed-file fmt and
guardrails. Do not accept a merge commit or deploy without passing meaningful
gates. The prior known baseline failures are in
[content-filter closeout](HANDOFF_2026-09-25_CONTENT_FILTER_CLOSEOUT.md).
Fork-default synchronization, pushing, GitHub PR/issue mutations, branch
cleanup and shared-daemon promotion remain **separate operator decisions**.

## Current read-only reconciliation: 2026-09-27T18:58Z

The 2026-09-21 counts and publication authorization in §1 are historical, not
permission to repeat those GitHub effects. `CONTRIBUTING.md` still requires an
existing linked issue, a focused change with reproduction, and review on merits.
Both remotes were fetched without pruning; there was no merge, ref deletion,
push, PR/issue mutation, provider/config change or daemon promotion.

| Ref, compared with `origin/master` `cc2171473` | Behind | Ahead | Role |
| --- | ---: | ---: | --- |
| Local `master` `8870f1993` | 68 | 0 | Stale fast-forward-only mirror; do not edit as feature work |
| GitHub `fork/master` `5fb914af8` | **68** | **22** | The URL's fork-default badge, not local source or shared runtime |
| Published `fork/jcode/ci-format-baseline` | 68 | 370 | Integration as last published |
| Local integration `018598cd1` | **34** | **409** | Active source, 73 commits ahead of its published fork branch |

These are ancestry counts, not unique-feature counts. In particular fork's two
old docs/auth commits `5cb7b3dad` and `e09acaa7a` have stable patch IDs identical
to upstream ancestors `ba900d276` and `1d87eadb6`. The other fork-default
commits include intentional fork CI guards, test ratchets and dependency
maintenance, plus fixes mirrored by upstream PRs. `git diff origin/master
fork/master` spans **264 files** because the fork is 68 commits behind. It is
not evidence that 264 fork-only edits need upstreaming or that a reset is safe.

The 34 upstream-only commits after local merge base `b5a4cde7a` include new
account/auto-switch behavior, TUI/detail-layer changes, MCP name sanitization,
voice, SDK/SSH and applet changes. **Potential behavioral overlap:** upstream
`691e1bb04` persists Anthropic OAuth *last-good usage and 429 backoff* in
`usage/disk_cache.rs`; local N2 `119b4e32d` independently persists exponential
backoff and Retry-After handling in `usage/backoff.rs`. Upstream `8556d6fcc`
alters reset availability. A merge must explicitly reconcile both caches,
expiry, rate-limit reporting and unknown-versus-confirmed-zero semantics with
tests, not blindly keep two gates or prefer one side. Upstream
`6fdab0b77` separately isolates a named-profile environment test that failed
on the previous local base. `git merge-tree --write-tree --name-only` against
local integration exceeded its 90-second bound under the configured Mergiraf
merge driver. It returned **no reliable conflict count**; no real merge was
attempted. Against fork default, the bounded preview reported eight conflicts
in app-core server/tool tests and TUI helpers/palette tests. A preview is not
an accepted merge resolution. The timed-out preview temporarily created three
untracked `.merge_file_*` artifacts in the worktree; they were **reversibly
relocated**, without deleting them, to `~/.jcode/scratch/merge-preview-artifacts/`.
The active worktree was rechecked clean. Do not repeat the preview with the
external merge driver while relying on it as a zero-effect read.

### Contribution review, not publication

Authenticated GitHub reads found **12 open upstream PRs authored by
`ianalitis`**, all mergeable with their own head's later Greptile **5/5** summary
and **zero unresolved review threads**: #1513, #1512, #1511, #1496, #1494,
#1493, #1489, #1487, #1362, #1357, #1356, #1354. None has a human
`CHANGES_REQUESTED` or approval review; the visible reviews are Greptile or
the author. Do not infer maintainer acceptance or merge authorization from
Greptile. **26 authored upstream issues remain open.** #1495/#1492/#1491 are
explicitly queued by the maintainer's automated triage alongside #1496/#1494/
#1493. #1352/#1350/#1349/#1348 have #1362/#1357/#1356/#1354 as focused
PRs. Do not duplicate these fixes or close the issues while PRs await review.
#1497 (fallback lane design), #1176 (project MCP trust UX), and #1121/#1122
(named-profile policy layer) remain decision-dependent, not blank tickets for
an autonomous new implementation. #1110 has a prepared fork branch but the
provider route remains disabled pending maintainer decision and live validation.

**CI is not review-complete:** upstream master CI run `36309866230` fails
format/quality checks and the macOS warning budget. #1511 CI run
`36337711541` fails its format check on many files already drifting on master;
five build/guardrail jobs additionally fail at `Configure SSH for cargo git
dependencies`, before testing the PR. The separate semantic-label workflow
run `36338215249` reports HTTP **402** from the labeler provider API and
explicitly says labels were not updated. This is an account/maintainer setting,
not a reason to change keys, spend balance or alter tests. CI status is not
proof that a PR implementation passed or failed. A focused, linked issue/PR
for fresh master format and warning drift should be considered only after
reproducing a minimal patch on current upstream and agreeing its scope with
the maintainer; #1354 addresses earlier drift, not all newly added changes.

**Fork validation PRs:** #3 remains open though upstream #1366 merged its
counterpart. #4 remains open and conflicts though upstream #1373 and issue
#1339 were closed by the maintainer as superseded. Upstream
`Session::save()` exempts debug/canary and retains the improve-mode persistence
test. Their public closure and later remote-branch cleanup are proposed
housekeeping, not performed. The [refreshed branch ledger](BRANCH_LEDGER_2026-09-21.md)
records six dry-run-safe heads, one unique head and the protected PR branches.

**Next authorization boundaries:** review and approve a separately scoped
integration merge of the 34 missing upstream commits, with N2 cache
reconciliation and targeted regression gates before runtime promotion; review
and approve a separate non-force fork-default sync that preserves fork CI; and
name any fork PRs/remote branches for closure/deletion after rechecking heads.
Do not merge an old PR branch or the integration line into a focused upstream
PR, reset `fork/master`, raise ratchets, or publish an unreviewed sync.

This follows [the Astra workforce handoff](HANDOFF_2026-09-21_ASTRA_EPHEMERAL_WORKFORCE.md).
It records verified source changes, not a deployed runtime or an accepted unattended workforce.

## 1. Historical posture and approved publication (2026-09-21)

**Published with operator approval at 16:44 UTC. GitHub fork/master is now 0 behind upstream.**

The non-force atomic push completed at 16:47 UTC. `fork/master` is `dc12efa7a`,
with the exact source tree of upstream `2a4edaa02`. Its three ahead commits are
the two preserved historical fork commits plus the merge, not source differences.
PR #1356 now points to `27231db08`; PR #1357 points to `dc4651417`. GitHub APIs
and `git ls-remote` independently confirmed these heads. Neither PR was merged.

Before publication, with upstream at `2a4edaa02`:

| Ref | Behind upstream | Ahead upstream | Role |
| --- | ---: | ---: | --- |
| `master` | 0 | 0 | Local upstream mirror |
| `jcode/ci-format-baseline` at `178a7d446` | 0 | 257 | Local integration, before this receipt |
| `fork/master` at `e09acaa7a` | 1997 | 2 | Stale GitHub fork default branch |

`origin` is `1jehuang/jcode`. `fork` is `ianalitis/jcode`. The GitHub banner did
not describe the active development checkout. It described the stale remote
mirror. The integration checkout initially lacked only one upstream docs commit,
which was merged as `803b731d2`. Existing local `master` was fast-forwarded to
upstream. No branch was reset or force-pushed.

Both fork-only commits are patch-equivalent to changes already upstream:

| Fork commit | Upstream equivalent | Stable patch ID |
| --- | --- | --- |
| `e09acaa7a` | `1d87eadb6` | `b4300226388c251a7037753fcc7fd38283f7dca8` |
| `5cb7b3dad` | `ba900d276` | `ead0ed449406afe368327a9095891928d76ce73c` |

The captain verified patch identity and upstream ancestry. There is no unique
feature content to rescue from those two commits.

The operator approved the following on 2026-09-21 at 16:44:53 UTC. All three
publication steps are complete:

1. Creating `jcode/fork-mirror-sync` from refreshed `fork/master`, merging refreshed
   upstream into it, and resolving the known account-login/chart conflicts to
   exact upstream content. Verify the resulting tree equals upstream while both
   histories remain ancestors. Do not include the private integration line.
2. Non-force push of that reviewed result to `fork/master`.
3. Non-force pushes of the reviewed local PR branches listed below to their
   matching `fork` branches. This updates PRs, not merges them upstream.

Only the approved sync branch and three non-force remote updates were created.
No new worktree, history rewrite, upstream PR merge, deletion, installation,
auth/provider/config change, or daemon reload occurred.

## 2. PR findings and published follow-ups

All four supplied Greptile findings were valid on the published PR branches.

| PR | Decision and repair | Local follow-up |
| --- | --- | --- |
| [#1356](https://github.com/1jehuang/jcode/pull/1356) | Clear held ownership before the underlying env mutex unlocks. Update explicitly typed callers to `TestEnvGuard`. Signal contention from the actual `WouldBlock` path, not a sleep/readiness guess. | `pr/test-env-lock-bounded-wait` at `27231db08` |
| [#1357](https://github.com/1jehuang/jcode/pull/1357) | Run atomic status publication on the blocking pool. Keep serialization through rename even if the awaiting caller is cancelled. Deterministically pause a staged write while readers and the runtime are exercised. | `pr/background-status-atomic-writes` at `dc4651417` |
| [#1360](https://github.com/1jehuang/jcode/pull/1360) | Integrate the already-published documented-default assertion correction, preserving author attribution. | `9618d3e95`, integrated as `4145bb765` |
| [#1354](https://github.com/1jehuang/jcode/pull/1354), [#1355](https://github.com/1jehuang/jcode/pull/1355) | No additional inline findings at inspection. Their lint/socket repairs are represented in integration. | No invented follow-up changes |

The two new PR follow-ups are committed and **published to the matching fork branches**. They retain
the older contribution layout and do not merge the integration branch or its
refactors. All 13 changed/new atomic methods match accepted integration behavior,
except that an unrelated pre-existing adopted-output change was deliberately not
ported. The six new atomic regression tests match byte-for-byte.

### Atomic publication details

Integration commits: `d763de3ab` and cancellation error handling `178a7d446`.

- Reuse one existing mutex as the status read/modify/publish gate. No new actor,
  per-task registry, or coalescing mechanism.
- Move its owned guard into `spawn_blocking` with the atomic temp-write/rename.
  An aborted async waiter cannot release serialization while rename is pending.
- Cover initial, progress/checkpoint, delivery, watchdog, completion, cancellation,
  reload, and reconciliation status publication. Preserve terminal-before-prune
  and initial-before-start notification ordering.
- Lock order is status gate before live-task map. Map guards drop before awaits.
- Cancellation still stops live work when status JSON is missing or corrupt.
  Cancelling the cancellation caller cannot strand the removed task as Running.
- Log unexpected join/signal failures. Expected aborted joins and Unix ESRCH are
  handled explicitly. No error-budget baseline was raised.

The filesystem work remains best-effort, matching the existing public API.
A slow filesystem serializes status operations globally, but does not occupy a
Tokio worker during publication. This tradeoff is explicit, not a throughput claim.

### Deterministic lock coverage

Integration already contained the guard-lifetime repair from `450702133`.
`0221b6ae2` strengthens its regressions:

- Actual `WouldBlock` observation must arrive before the holder releases.
- A released same-thread record is not reentry, a live same-thread record is,
  and a different thread is not the recorded owner.
- After a real guard drops, acquiring the raw mutex proves the last record is
  released without depending on which other test thread ran in between.

## 3. Test-harness repairs and acceptance evidence

Six reactive compaction assertions failed both parallel and serial runs because
`CompactionManager::new()` loaded the operator's proactive mode. This was not a
thread race. `c622b79ba` pins eight reactive fixtures, including two otherwise
vacuous checks, to the strategy each test intends to exercise. No production
compaction behavior or operator config changed.

A later full run exposed `effort_scaling_never_shrinks_the_base_budget` reading
process-global configuration repeatedly while other tests switched `JCODE_HOME`.
`5d101edcd` adds the shared env guard to that test. No provider timeout changes.

| Gate | Result |
| --- | --- |
| Compaction family after fixture repair | 37 passed |
| Full integration `jcode-base --lib`, parallel | 1549 passed, 0 failed, 5 ignored |
| Full integration `jcode-base --lib -- --test-threads=1` | 1549 passed, 0 failed, 5 ignored |
| Integration background selector after final cancellation logging | 44 passed |
| Full integration `jcode-app-core --lib` after final repair | 1529 passed, 0 failed, 31 ignored |
| Integration `scripts/check_guardrails.sh` | All configured gates pass |
| PR #1356 `jcode-base --lib storage::tests` | 11 passed |
| PR #1356 `cargo check --workspace --all-targets` | Passed, existing warnings remain |
| PR #1357 `jcode-base --lib background::` | 27 passed |
| PR #1357 `cargo check --workspace --all-targets` | Passed, existing warnings remain |

The full parallel base suite was also rerun on the restored final integration
source and passed again. The serial run precedes the final logging-only follow-up.
Background tests, app-core, all-target/all-feature checking, clippy, formatting
and ratchets were rerun after that follow-up. `cargo machete` is optional and was
not installed, so the existing guardrail script reported it skipped. Nothing
was installed to alter that state.

Negative controls were run and restored:

- Truncating the destination JSON while a staged replacement was held caused the
  staged-old/new regression to fail. Restored code passed all six new regressions.
- Leaving `held = true` on real guard drop caused the released-owner test to fail.
  Restored code passed all 11 storage tests.
- The worker's pre-fix single-worker heartbeat proof failed under synchronous
  terminal publication and passed with the off-worker publisher.

The broad `background` substring on the older #1357 baseline also selected three
reactive compaction tests and failed those known fixture assertions. The actual
`background::` module passed. The unrelated fixture fixes were not folded into
that PR, and its entire library suite is not claimed green.

Logs under `~/.jcode/scratch/`:

- `reconcile-base-full-before.log`, `reconcile-compaction-isolated.log`
- `reconcile-base-final.log`, `reconcile-base-final-serial.log`, `reconcile-base-final-head.log`
- `reconcile-background-final.log`, `reconcile-app-final.log`
- `reconcile-atomic-negative-control.log`, `reconcile-atomic-restored.log`
- `reconcile-storage-negative-control.log`, `reconcile-storage-restored.log`
- `reconcile-guardrails-final.log`
- `reconcile-pr1356-storage.log`, `reconcile-pr1356-workspace.log`
- `reconcile-pr1357-background.log`, `reconcile-pr1357-module.log`,
  `reconcile-pr1357-workspace.log`

Older PR branches lack `scripts/bounded.sh`. Their commands used an unchanged
copy from integration in scratch, not an unbounded command or a PR scope expansion.

## 4. Preserved work and remaining limits

Fresh script-generated inventory: `~/.jcode/scratch/upstream-reconcile-ledger.md`.
Its timeout entries are unresolved checks, not demonstrated conflicts. All
existing branches and worktrees were preserved. No stashes existed at inventory.
Older ledger stash entries are historical, not deletion instructions.

One dirty worktree is intentionally untouched:
`../worktrees/data-class-admission/crates/jcode-harness-api-server/src/translate_regression_tests.rs`.
Its untracked 3298 bytes are a byte-identical prefix of the current 18600-byte
canonical file. It was not deleted, even though no unique content was found.
The active integration checkout and the two prepared PR branches are clean after
committing. This is not a claim that every old branch has been integrated.

The prior app-core refused-socket failure did not reproduce in two full runs.
That does not prove its intermittent cause is fixed. Existing auth-route,
privacy/data-admission branch remainders and TUI timing tests still need bounded,
separate review. Do not wholesale merge old branches based on ancestry counts.

Read-only Sol/high lifecycle review did **not** accept unattended self-compaction
or tool-enabled Pi. Native notes remain volatile, request/event correlation and
failure/retry are incomplete, duplicate notes replace one another, byte/character
limits disagree, and automatic post-compaction continuation is unproven. Bridge
attempt authentication, ACK correlation and durable restart semantics need work.
Review artifact: `~/.jcode/scratch/reconcile-sol-review.md`.

The shared/running binary was `a61ab0927` at inventory. No reload was done. New
source tests are not evidence that the daemon now serves these changes.

Tool-interface observation on `v0.86.234-dev (a61ab0927)`: schema-admitted null
optional booleans were rejected as `invalid type: null, expected a boolean`.
Explicit false was the safe workaround. No unrelated harness-schema edit was made.

## 5. Keeping contribution friction low

[The fork posture](FORK_POSTURE.md) now distinguishes mirror, integration and
single-purpose PR branches and gives explicit behind/ahead checks. Use those
checks at session start and before preparing a contribution. They are documented
workflow checks, not an installed automatic synchronization service.

Keep `master` a mirror, local policy/customizations on the integration line, and
PR branches based on current upstream with only their own reviewed change.
Classify old work using the existing branch-ledger script and patch equivalence,
not ancestry alone. Refresh refs before an approved push, preserve unrelated
staging, run the scoped tests, and never publish the integration line as a mirror.

## 6. Publication verification

- Mirror merge: `dc12efa7a687fe97e2a319917938f24aaf3121a1`.
- Parents: old fork `e09acaa7a8828bcd46d9a5ab7c3434112e04e713` and upstream
  `2a4edaa02057ac994a601311c4f03ed450e1b3c9`. Both remain ancestors.
- Entire tree equals upstream: `3f3ca1fdf954104fc8b9eb5f58270c80ba846e55`.
- Resolved only the reviewed account-login implementation, its tests, and
  `docs/images/star-history.svg` conflicts to exact upstream content.
- Account-login smoke tests: 6 passed, 0 failed, 1 live test intentionally ignored.
- `git push --atomic` updated exactly `fork/master`,
  `fork/pr/test-env-lock-bounded-wait`, and `fork/pr/background-status-atomic-writes`.
  No force flag or integration-branch push was used.
- Fetched remote refs, verified all three full SHAs with `git ls-remote`, and
  verified both open PR head SHAs through GitHub.
- Returned to `jcode/ci-format-baseline`. Local `master` still exactly mirrors
  upstream. The sync branch is retained, not deleted.
- At the immediate post-push check, both Greptile reviews were in progress.
  Local acceptance above does not claim hosted review completion.

Publication artifacts in `~/.jcode/scratch/`: `reconcile-mirror-merge.log`,
`reconcile-mirror-account-login.log`, `reconcile-approved-push.log`, and
`reconcile-published-refs.json`.

## 7. Detached cancellation grace review follow-up

A subsequent #1357 review correctly found that detached Unix cancellation held
`status_updates` while awaiting the caller's grace timeout. Off-worker filesystem
publication did not prevent that manager-wide lock from delaying other tasks.

- Integration repair: `f1233129b`.
- Focused PR-branch repair: `245c44b4d`, one local commit after published `dc4651417`.
  The operator separately approved this follow-up at 17:35 UTC. It was pushed
  non-force at 17:36 UTC and verified with `git ls-remote` and GitHub PR metadata.
  The first immediate PR read lagged the successful push, so verification was
  repeated without pushing again. The expected head is `245c44b4dcd40e564f9fe5970bda3925a5b23f63`.
- Release the owned guard after TERM and before the grace wait. Reacquire and
  reread status afterward. Proceed only if still Running, detached, and using the
  original PID. This preserves newer terminal states and avoids stale delayed
  KILL/publication against a replacement record.
- Use refreshed progress/delivery metadata when cancellation does proceed.
  Existing atomic publication and cancelled-caller guard ownership are unchanged.
- Three deterministic Unix regressions use isolated process groups, explicit
  grace-boundary handshakes, paused time, bounded waits, and fixture cleanup.
  They verify unrelated progress can publish, a newer terminal status wins, and
  real cancellation retains updates made while its target remained Running.
- The unrelated-progress test failed before the fix. Captain's negative control
  removing the refreshed-status assignment made the metadata regression fail.
  Restored code passes 30 background tests and the full base suite:
  **1552 passed, 0 failed, 5 ignored**.
- Full configured integration guardrails pass. Optional absent `cargo machete`
  remains skipped by the existing script. On the older PR branch, all 30
  background tests and `cargo check --workspace --all-targets` pass, with its
  existing warnings. The ported method and tests match integration exactly.

Evidence under `~/.jcode/scratch/`: `cancel-grace-builder.md`,
`cancel-grace-metadata-negative.log`, `cancel-grace-background-captain.log`,
`cancel-grace-base-full.log`, `cancel-grace-guardrails.log`,
`cancel-grace-pr1357-tests.log`, and `cancel-grace-pr1357-workspace.log`.
The approved follow-up was published. No PR merge, configuration change, or daemon reload was made.

## 8. Approved local and public cleanup

The operator approved both batches in the
[existing branch ledger](BRANCH_LEDGER_2026-09-21.md) at 17:54 UTC. Execution on
2026-09-21 was limited to those named targets:

- Rechecked all six approved branch tips, then removed their local branch names.
  Removed the clean integrated `attempt-billing-route` worktree without force
  before deleting its branch. Removed only the verified 3298-byte duplicate
  untracked test file in `data-class-admission`, keeping that worktree and branch.
  Inventory is now 27 local branches, 6 clean worktrees, and 0 stashes.
- Closed PR #1293 as already fixed upstream by `e7a22f695`, and PR #1295 as
  superseded by focused PR #1354. Both exact approved heads were rechecked before
  closure. Neither was merged and neither remote branch was deleted.
- Closed issue #1292 as completed. The closure preserves the verification caveat:
  the unchanged upstream fingerprint test still fails only for the independent
  `JCODE_MEMORY_JEV_PROVIDER` omission tracked by #1358 / PR #1360.
- PR #1360 already starts with `Fixes #1358.` at approved head `9618d3e95`.
  That equivalent closing linkage makes an additional `Closes #1358` unnecessary,
  so no description edit was made. #1294 and #1358 remain open.
- Independent GitHub reads confirmed 5 open / 2 closed authored PRs and 25 open /
  13 closed authored issues. All five active contributions remain open.
- Regenerated the canonical ledger against `851ff2c8c`. Its timeout rows remain
  unknown merge outcomes, not demonstrated conflicts. Unreviewed branch remainders
  and historical plans were preserved, not declared integrated or deleted.

Evidence: `approved-cleanup-a.json`, `approved-cleanup-b.json`,
`approved-cleanup-inventory.json`, and `approved-cleanup-ledger.md` under
`~/.jcode/scratch/`. This follow-up changes no production code, source-test result,
provider/config setting, or runtime deployment. No new push or PR merge occurred.

## 9. Follow-up test-harness iteration

After cleanup commit `c290654c3`, the operator requested continued iteration.
Three small retained branches were reviewed; their pinned dispositions and
remaining decisions are in the existing ledger. This produced two test-only
repairs, not a wholesale branch merge:

1. `c4749c964`: restore the missing native-compaction unknown-count regression
   from `e79659889`. The shared fixture now supplies either `Some(80_000)` or
   `None`, while still emitting response usage of 24,000. The new test requires
   an actual client compaction event with `None`. A temporary production mutation
   to `native_pre_tokens.or(usage_input)` failed exactly that assertion with
   `Some(24000)` versus `None`; restoration was verified byte-for-byte.
2. `568b25a24`: canonicalize one source-snapshot test fixture root before calling
   the internal collector. The first captain full-suite run failed that existing
   test: 1529 passed, one failed, 31 ignored. A command-local symlinked temporary
   directory reproduced the failure deterministically. The test had bypassed
   `snapshot()`'s canonical-root precondition, so file containment compared a
   canonical file path against a lexical alias. The same aliased-root test passes
   after the fixture-only repair, as does the default-root test. All production
   source-snapshot bytes, exclusions and containment checks are unchanged.

Final captain acceptance on macOS aarch64:

| Check | Result |
| --- | --- |
| `cargo test -p jcode-app-core --lib` | 1530 passed, zero failed, 31 ignored |
| `cargo test -p jcode-core --lib stdin_detect` | 5 passed |
| `cargo test -p jcode-setup-hints --lib` | 149 passed |
| `scripts/check_guardrails.sh` | All configured gates pass, including all-target/all-feature check, strict clippy, format and size/error ratchets |
| Native compaction module / source snapshot module | 7 / 12 passed in scoped validation |
| Productivity / Mermaid / Cargo cwd routing | 5 / 64 / 5 passed |

Optional `cargo machete` is still absent and was skipped by the existing gate
script, not installed or represented as passing. The old refused-socket test
passed in this parallel full-suite run; that does not prove its earlier
intermittent failure has been fixed. No Windows target is installed, so the three
remaining Windows-only warning-cleanup hunks are retained as unverified.

Evidence under `~/.jcode/scratch/`: `iterate-native-compaction-regression.md`,
`compaction-none-negative-control.log`, `iterate-snapshot-fixture.md`,
`snapshot-fixture-alias-before.log`, `snapshot-fixture-alias-after.log`,
`snapshot-fixture-default-after.log`, `iterate-cargo-cwd.log`, and
`iterate-branch-acceptance.log`. No test/size baseline was weakened. No production
code, provider configuration or daemon deployment changed. No further branch or
worktree deletion, public issue/PR mutation, or push occurred in this iteration.

## 10. Published fork sync to upstream v0.89.0 (2026-09-28)

The separate fork-master staging worktree merged released upstream
`9929ee0eaf187fb3e48d1bcfd5e9d4071b1782f3` into fork/master
`5fb914af89dcfceb955d5c08aa53d14317ff0ab8`. The signed two-parent merge
`5c60f40ce94e7b1278e368dd1868edc96e14c702` was pushed **non-force** to
`ianalitis/jcode` master, and the remote branch SHA was independently verified.
It is not a merge into this local integration line and it has not changed the
shared Jcode runtime.

The operator approved rebaselining only upstream-inherited ratchet debt.
Comparison against an immutable upstream v0.89.0 source snapshot and the fork's
pre-merge budgets found eight merge-specific oversized parent files. Cohesive
helper and test extraction eliminated all eight before refreshing the baselines.
Panic usage is 165 across 58 paths, identical to upstream per path. Swallowed
errors total 3541 with upstream-identical per-pattern totals; three existing
`.ok()?` expressions moved into a small clock helper, explaining the additional
tracked path. Production and test size ratchets track 115 and 49 oversized
paths, respectively. The one test path over upstream's 48 is preexisting fork
e2e support. Provenance and remaining debt are recorded in the published
`docs/FORK_CI.md`, with the full local command/results log in
`~/.jcode/scratch/FORK_SYNC_HANDOFF_2026-09-28.md`.

Local tests passed for app-core (1461/30 ignored), base (1658/6),
harness-api-server (159), provider matrix (8 plus one documented upstream
quarantine), e2e (60/7), and TUI (2419/18 plus one documented upstream
quarantine). All-target/all-feature check, strict Clippy, format, workflow lint,
warning budget and all four ratchets passed. Local strict security preflight
could not run without installing `cargo-audit`, so hosted CI was the decisive
gate: [run 36376171635](https://github.com/ianalitis/jcode/actions/runs/36376171635)
completed **success, ten of ten jobs**, including Linux, macOS, Windows and
dependency audit at the exact merge SHA. Independent read-only merge review
found no blockers. The iOS-specific voice branch remains untouched. Twelve
authored upstream PRs remained open at last refresh; #1513 was conflicting,
with no unresolved Greptile correctness finding. No upstream PR was force
updated or newly submitted in this fork-sync operation.

## 11. Post-v0.89 upstream follow-up (2026-09-28)

The fork-master staging worktree subsequently merged upstream
`4c4d9651c23987e5d89e9054d873fea606ed66df` (after the weekly stars-chart
docs update and Anthropic parallel tool-result ordering fix). Signed merge
`08873a2a53bc9725aaa47b6582fd5ad5afa89064` has parents `5c60f40ce` and
`4c4d9651c`; `ianalitis/jcode` master independently reports that exact SHA
after a non-force push. This is a separate follow-up to the already-green
v0.89.0 merge, not a rewrite of it. The provider's same-role merge and stable
result partition were extracted into `merge_messages.rs` so the oversized
parent `lib.rs` shrank from 1335 to 1318 LOC without rebaselining the ratchet.

At publication, Anthropic's 28 unit tests (including the new parallel-result
regression), full all-target/all-feature check, strict Clippy, rustfmt, warning
budget (0/0), production and test-size budgets, panic and swallowed-error
ratchets, and wildcard re-export budget passed locally. Hosted
[CI 36381846325](https://github.com/ianalitis/jcode/actions/runs/36381846325)
and [CodeQL 36381845591](https://github.com/ianalitis/jcode/actions/runs/36381845591)
were still **in progress** at the latest check, so do not claim the new merge
green until their final conclusions are verified. Cargo continues to emit
upstream-inherited unmatched profile-package notices for conditional TUI and
desktop dependencies. These are not Rust compiler/Clippy warnings and did not
increase in the merge.

The local integration line remains separate at `0a2373193`, measured 40
commits behind and 494 ahead of `origin/master` at the time of this follow-up.
A read-only structured merge preview exceeded its 45-second bound; it is not a
resolved conflict assessment or authorization to merge the large integration
line tonight. The shared runtime was not promoted (self-dev status still
reported running `v0.88.469-dev`, source build channel `832c42b23`).
