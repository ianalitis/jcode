# Upstream merge: v0.86.0 into `jcode/ci-format-baseline`

**Date:** 2026-09-20/21. **Pre-merge tip:** `11ab2d532`. **Upstream target:**
`origin/master` = `e589cbe5a` (`v0.86.0`, released 2026-09-20 10:42).
**Merge base:** `74e7a4be5` (2026-09-15).

This records how the local line was moved onto upstream while keeping the local
improvements, which side won each class of conflict, and what remains open.
It is a merge, not a rebase: the orientation receipt
(`~/dotfiles/docs/measurements/2026-09-20-upstream-v0.86.0-orientation.md`)
recommended merging because local history is already published on
`fork/jcode/ci-format-baseline` and this repository treats history as a record
rather than something to tidy.

## 1. Scope

| Direction | Commits |
| --- | ---: |
| Upstream has, we did not (`74e7a4be5..e589cbe5a`) | 1,998 |
| We have, upstream does not (`74e7a4be5..11ab2d532`) | 207 |
| Files touched by both sides | 109 |
| Conflicted files (`UU`) | 48 |

## 2. How conflicts were resolved

`git merge origin/master` with `core.attributesfile` pointed at `/dev/null`, so
the global Mergiraf structured-merge driver did not add minutes per file and no
driver temp files were left in the tree. Every hunk was decided by hand:

- **Adopt upstream** when upstream solved the same problem better or later:
  - prompt overlay dedupe (`d834ffc05` was our fix for upstream #1092; v0.85.0
    shipped the same behavior, so upstream's loader won).
  - `scripts/dev_cargo.sh` cwd guard (upstream implemented the same physical-path
    guard) and `reapply_terminal_modes_to`'s `focus_change` parameter plus the
    tmux extended-key request.
  - `cache_ttl_for_provider_model` / `cache_ttl_is_estimate`: auth-route aware,
    so upstream's policy replaced our module-local copy.
  - server startup: upstream removed the eager embedding preload because Jev
    recall does not need it; our memory-enabled gate and its test were deleted
    with it.
  - todo ownership/confidence gates: upstream tightened them to actionable gaps
    only (metadata alone is not unfinished work) and to extreme recorded
    confidence jumps (three levels). Our tests were rewritten to that contract.
  - Conifer catalog: upstream excludes the undocumented
    `nemotron-3-ultra-together` route rather than lending it the base model's
    window; we adopted the stricter table.
  - Protocol additions (`ServerEvent::TextDone`, `Request::Subscribe
    supports_pdf_panels`), MCP alias-aware dispatch, and OpenAI usage
    accounting (`jcode_compaction_core::effective_context_tokens_from_usage`)
    came from upstream.
- **Retain ours** where the local behavior is the point of the branch:
  ancestor-layered `AGENTS.md` loading, atomic background status-file writes
  (`write_status_file_atomic`), truncated tool output spilled to disk, bounded
  `list_models`, included-subscription route pinning, the fail-closed session
  tool-policy dispatch guard, the split test layout for the size budgets, and
  the fork's privacy removals (feedback tool, todo telemetry collection).
- **Combine** where both sides changed different axes of one function:
  the MCP policy helpers now keep our fail-closed `checked_session_tool_policy`
  and ctx-based signature while adding upstream's alias/legacy disable checks;
  `provider/mod.rs` keeps our `cache_ttl` module split with upstream's imports;
  `sdk/auth.rs::cancel` keeps our poisoned-lock recovery and takes upstream's
  callback clear; `agent.rs` keeps both added predicates.
- **Port** upstream's new tests into our split files rather than restoring the
  inline modules: 4 config, 8 todo, 14 harness-api, 6 app-core agent, 3
  provider-core, 3 openrouter, 2 anthropic, 1 lifecycle, 1 session-cases, 1
  tui `mod`, 1 `ui_input`, 1 `state_model_poke_03` — plus the helpers they use.
- **Drop** what the merge made dead: `should_preload_embedding_model` and its
  test, and the `diff_line_wrap` override in `tui_bench` after upstream removed
  the trait method.

## 3. Fixes the merge surfaced

- `crates/jcode-base/src/jev.rs`: the mock HTTP server set the listener
  non-blocking and then read the accepted socket with a timeout. On BSD/macOS an
  accepted socket inherits `O_NONBLOCK`, so the read returned `WouldBlock` and
  the test failed 2 of 3 runs. The socket is now switched back to blocking after
  `accept`, and passed 5 of 5 runs.
- New fields on shared types reached our test initializers: `CacheTtlInfo::
  is_estimate`, `SidePanelPage::pdf_data`, `SidePanelSnapshot::focus_revision`,
  `TokenUsageTotals::cache_prompt_tokens`, `tool::Registry::mcp_policy` and
  `Request::Subscribe::supports_pdf_panels`.
- `cap_tool_output_for_history` takes the session id and tool name; the
  `panel` suite now registers the session policy an agent turn would have,
  because this fork denies `AgentTurn` dispatch without one.
- macOS renders the Option key as `⌥`, so tests that hard-coded `Alt+…` were
  made platform-aware via `jcode_tui_core::keybind::alt_chord`.
- `compile_remote::source::tests::rejects_non_utf8_git_paths` is Linux-only:
  APFS refuses to create the invalid-UTF-8 path the case needs.
- clippy 1.98 lint drift failed the quality gate on twelve sites that are
  unchanged from upstream (needless_borrow, too_many_arguments,
  manual_is_multiple_of, collapsible_if, match_like_matches_macro,
  unnecessary_fold). `browser_fast.rs` keeps full traversal on purpose: clippy's
  suggested `any(..)` would short-circuit and skip credential redaction for the
  remaining array items. The full list and the exact fixes are in
  `docs/upstream-feedback/2026-09-21-clippy-1.98-lint-drift.md`.
- The shared test-env lock now bounds its wait and names the last successful
  acquirer, and reports a re-entrant acquisition in about 200ms instead of
  hanging. It was unbounded and had wedged a `jcode-tui --lib` run for fifty
  minutes with eleven threads in `__psynch_mutexwait`; see
  `docs/TUI_TEST_FLAKINESS.md`.

## 4. Verified after the merge

| Suite | Result |
| --- | --- |
| `jcode-base --lib` | 1527 passed, 0 failed, 5 ignored |
| `jcode-app-core --lib` | 1517 passed, 0 failed, 31 ignored |
| `jcode --lib` (root) | 285 passed, 0 failed |
| `jcode-provider-core --lib` | 134 passed, 0 failed |
| `jcode-provider-openrouter-runtime --lib` | 184 passed, 0 failed, 1 ignored |

## 5. Known issues carried forward

- `jcode-tui --lib` runs 2351-2354 of ~2373 tests green, with 1-4 failures that
  differ per run and all pass in isolation. The affected files
  (`tests/remote_events_reload_05.rs`, `tests_input_scroll.rs`) hold no
  `lock_test_env` guard on either side of the merge and write reload state under
  the process-wide `JCODE_HOME`, so parallel tests redirect each other. This is
  pre-existing, not merge-induced. A blanket env lock was tried and reverted: it
  serialized the suite from ~27s to over 10 minutes. The correct fix is a
  per-test temporary `JCODE_HOME` for those two files.
- The stale-test sweep (see `scripts`-adjacent scratch tooling used during the
  merge) flags same-named tests whose bodies differ from upstream, mostly nested
  helper methods and the tests deliberately rewritten above. It is a hint list,
  not a defect list: every suite listed in section 4 is green.

## 6. Evidence commands

```sh
cd ~/.jcode/source/jcode
git log --oneline -1 11ab2d532          # pre-merge tip
git log --oneline -1 origin/master      # merge target (v0.86.0)
git merge-base HEAD origin/master
git diff --name-only --diff-filter=U    # empty once the merge is committed
```
