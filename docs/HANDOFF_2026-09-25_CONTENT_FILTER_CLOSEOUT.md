# Handoff: content-filter review complete, Phase 2 source work integrated

Continuation prompt and line state for a fresh session. Plan of record:
[plans/2026-09-25-ARCHITECT_ASSESSMENT_AND_PHASED_PLAN.md](plans/2026-09-25-ARCHITECT_ASSESSMENT_AND_PHASED_PLAN.md).
Receipt for everything below:
[measurements/2026-09-25-openrouter-content-filter-false-positive.md](measurements/2026-09-25-openrouter-content-filter-false-positive.md).

## Current closeout: 2026-09-27 (session calf)

This section supersedes the historical continuation instructions below. Source work is
integrated locally, not promoted to the shared daemon. Do not replay completed nodes.

### Review and integration

- **PR #1511:** latest head `6feb8610e` (17:38:43Z), Greptile **5/5**, **0 unresolved
  threads**, summary **17:47:04Z**, rechecked at 18:09Z. No merge performed. CI remains
  red on the separately known infrastructure/format/budget failures, so this is review
  completion, not a claim of green CI or upstream integration.
- Final review corrections: `7edc75697` removes the loopback PAN exemption because a
  local server can forward remotely. `a65ada032` preserves direct, no-proxy loopback
  transport without exempting it from PAN checks. `6feb8610e` permits bounded
  loopback-only 307/308 redirects, never redirects to non-loopback hosts, and isolates
  the opt-out environment in tests. Local ports: `b6fb12ad5`, `a41815b11`, **`8d65320c6`**.
  The earlier `6ee407fd5` port is `92ca3b129`; its automatic exemption is superseded.
- **Final contract:** every endpoint is PAN-checked by default. Deliberate synthetic-card
  testing against a trusted local service requires explicit process-wide
  `JCODE_DISABLE_PAN_CHECK=1`. No inferred exemption for loopback or local forwarders.
  The local single-send metered path retains its constrained supplied transport.
- PRs **#1496, #1494, #1493, #1489, #1487, #1362, #1357, #1356, #1354, #1512,
  #1513** already completed their review closeout in the preceding sessions. No new
  changes to those PRs were needed here. Only the approved #1511 head was pushed,
  via `https://github.com/ianalitis/jcode.git`.

### Phase 2 source results

| Node | Commit | Result |
| --- | --- | --- |
| N1 | `88f14b00e`, subsequent review ports recorded below | LaTeX warning deduplication, upstream #1512 reviewed |
| N2 | `119b4e32d` | Anthropic usage backoff persists in `usage-backoff.json`, exponential jitter, Retry-After respected, 15-minute cap; OpenAI gate deliberately unchanged |
| N3 | `e8c5ce5fe`, `64211c405` | Forced refresh honors `model_catalog=false`; picker cache accepts the supported `chatgpt-web` method without accepting arbitrary `Other` methods |
| N4 | `a2066deae` | MCP stderr banners log at INFO unless the line looks like an error |
| N5 | `d99f5b80b` | Per-worktree target stamp with explicit `JCODE_SHARED_TARGET_DIR=1` override |
| N6 | `ba05a2a70` | SSH signing-agent preflight documented |
| N7 | **`5bc975ef8`** | Browser session/assets extracted, JEV tests split with existing `include!` convention, size allowances removed |

**N3 cause correction:** a read-only live snapshot contained 826 routes with safe text
fields, but the legitimate `chatgpt-web` route was rejected as `Method::Other`. Disabling
catalog refresh alone did not explain or repair that persistence warning. Tests now use
`build_chatgpt_web_route()`, preserve serde roundtrip, and reject invented methods. The
separate named-profile refresh regression proves a disabled catalog never connects and
keeps static models. Both fixes have pre-fix failing and post-fix passing regressions.
No claim is made that the still-old running daemon has stopped emitting the warning.

**N7 simplicity and preservation:** no new abstraction or public API. Browser function
bodies match their originals apart from visibility, imported path spelling and rustfmt.
JEV production code is unchanged, both test chunks match rustfmt of the original bodies,
and fully qualified test names are preserved. Sizes: `browser.rs` **1190**, session
**125**, assets **498**, `jev.rs` **803**, routing tests **640**, transport tests **841**.
Browser swallowed-error allowances are redistributed **25 = 18 + 3 + 4**, with every
pattern total conserved. No total increase, no `--update`, no allowance for existing JEV
overages. This is relocation of unchanged code, not acceptance of new swallowed errors.

### Verification and remaining limits

Commands use `CARGO_TARGET_DIR=~/.jcode/scratch/target-n1` locally, `target-up` upstream,
and `scripts/bounded.sh` with 180-300 second limits. Changed-file rustfmt and
`git diff --check` pass.

- Upstream final OpenRouter runtime suite: **147 passed, 1 ignored**; scoped clippy
  `--no-deps -- -D warnings` passed. Local final suite at `8d65320c6`: **198 passed,
  1 ignored**, same scoped clippy passed. Local proxy/redirect regression has red/green
  evidence upstream; the reviewed behavior and its tests are ported locally.
- N3: catalog regression and six catalog tests pass; all four
  `remote_model_catalog_cache_` tests pass after the validator fix.
- N7 focused: **26 browser passed**, **50 JEV passed, 1 ignored**. Full base suite:
  **1691 passed, 7 failed, 6 ignored**, versus pre-extraction **1690 passed, 8 failed,
  6 ignored**. All seven remaining failures already occurred before extraction:
  `auth::tests::{browser_suppressed_inside_test_harness_without_env_overrides,
  openrouter_like_status_is_provider_specific}`;
  `provider::catalog_routes::tests::{current_compatible_profile_accepts_only_cataloged_slash_models,
  remote_compatible_route_marks_static_model_list_fallback,
  remote_compatible_route_uses_live_cache_and_does_not_mark_fallback,
  slash_model_fallback_prefers_matching_compatible_profile}`;
  `provider_catalog::provider_catalog_tests::auth_issue_runtime_display_name_tracks_direct_compatible_profiles`.
  The earlier `provider::tests::test_resolve_model_capabilities_uses_provider_hint`
  failure did not recur. No tests were removed, weakened or ignored for this work.
- Panic budget passes (**100 sites, 37 files**). Whole-tree code-size, test-size and
  swallowed-error gates retain existing unrelated failures. N7 introduces no size
  violations; global swallowed count remains **3408** against baseline **3343**,
  including the unchanged JEV **6 versus 2**. Dependency-inclusive clippy remains
  blocked by the known `jcode-core` `needless_range_loop`; no lint was suppressed.
- Final `scripts/bounded.sh 300 scripts/check_guardrails.sh --skip-slow` completed
  in 250 seconds with four failures: whole-tree format (existing applet module ordering
  in `jcode-app-core/src/tool/mod.rs`), oversized production files, oversized tests and
  swallowed errors. Module resolution, lockfile freshness, warning budget, panic budget,
  dependency boundaries, wildcard reexports and onboarding state-space invariants pass.
  No `--fix`, `--update`, failure suppression or unrelated formatting was applied.
- Dotfiles G1 was already done in `adbf93e`, G5 in `f6c28a8`; provisioning credential
  verification is `f9e5e83`. No credential or account settings changed in this continuation.
- Preflight refreshed origin/fork without pruning (`origin/master` `cc2171473`).
  Runtime is separate: session build `c232ad32d`, shared-server channel
  `e7f83a9c1-dirty` when inspected. **No rebuild/reload/promotion was performed.**
  Runtime acceptance and existing broad gate failures remain explicit follow-up work,
  not grounds to replay completed source fixes or increase budgets.

## 1. Historical line state (2026-09-25 21:01Z)

| Fact | Value |
| --- | --- |
| Branch | `jcode/ci-format-baseline` (integration line) |
| HEAD | this handoff's commit, on top of `07644ca7e` |
| `origin/master` | `74577fe83`, 5 behind / 376 ahead (phase 0 merge still pending, approval A2) |
| Running build | `v0.88.432-dev (e7f83a9c1)`, self-dev reloaded, G2 and G3 live |
| Worktree | clean except untracked `scripts/local_model_probe.py` (not ours, leave it) |
| Write lease | free. SSH signing key is loaded (`ssh-add -l` shows ED25519) |
| Pushed | nothing. No PR or issue mutations this session |

## 2. What this session did

| Node | Commit | Result |
| --- | --- | --- |
| A1 guardrail PATCH | receipt `a95382205` | OpenRouter workspace guardrail `1c7da278-dad7-552d-ab12-97d5bf2e34df`: builtin `credit-card` block -> **flag**. Three custom `[PAN]` block filters (flat patterns, because the server rejects any quantified group as "catastrophic backtracking"). Other builtins and `limit_usd 20 daily` unchanged. Live: the issue list passes, the contiguous test PAN still 403s. Before/after JSON: `~/.jcode/scratch/or_guardrail_{before,after}.json`. Applied by `~/.jcode/scratch/or_guardrail_fix.sh` |
| G3 no-retry + legible error | `5feca93e6` | **Root cause correction:** the transport never retried 403s (`attempt 1/8` on every block). The loop was the **TUI auto-retry continuation** resending empty turns every 2-7 s. `previous_errors` x24 is OpenRouter's own provider fan-out. Fix: `jcode_provider_core::failover::content_filter_block_label`, plus `crates/jcode-tui/src/tui/app/remote/terminal_errors.rs` (the fail-fast branch, which now also holds the #387 model/endpoint case moved out of `server_events.rs`), declared via `#[path]` from `server_events.rs`. Also `openrouter_sse_stream::should_retry`: `upstream_provider_shared_pool` 429s retry at most twice |
| G2 local PAN check | `e7f83a9c1` | `jcode_provider_core::failover::pan_check` (`#[path]` child of `failover.rs`, to keep `lib.rs` under its size budget). Checked before send in `run_stream_with_retries` and `run_stream_once`: 15/16 contiguous digits or 4-4-4-4 / 4-6-5 groups, plus issuer prefix, brand length and Luhn. Loopback exempt. `JCODE_DISABLE_PAN_CHECK=1` escape hatch. The error names the message index and role, never the value, and reuses the "blocked by content filter: [PAN]" wording so G3 stops auto-retry. FP scan of 1,500 session files: 0 real false positives. Live check passed on an isolated socket |
| Doc hygiene | this commit | Literal test PANs removed from the plan and receipt. With G2 live, **any literal test card number in a file an agent reads makes that session unable to send** until the text leaves context. Write them as `"4242".repeat(4)` |

Tests at close: `jcode-provider-core` 150 passed, `jcode-provider-openrouter-runtime` 192 passed,
`jcode-tui` remote/fail-fast subset 9 passed, clippy `-D warnings` clean on the three crates.

## 3. Known, not caused by this session (do not "fix" by weakening)

- Resolved in `ec63aaa46`: the Comtegra/FPT picker tests failed only when run inside a jcode
  session. The parent exports `JCODE_NAMED_PROVIDER_PROFILE`, so
  `openai_compatible_profile_is_configured()` short-circuited and the route fell back to Copilot.
  `with_temp_jcode_home` now clears it. It was not a code regression, so no bisect was needed.
- Still intermittent under full `tui::app::tests` runs, and also on clean HEAD: the
  `test_model_picker_copilot_*` and inline-image tests (`test_alt_shift_i_is_inert…`,
  `test_real_draw_click_on_body_anchored…`, `skill_invocation_with_prompt_attaches…`).
  They pass in isolation, so they depend on test order.
- `scripts/check_code_size_budget.py` already fails at `3be93ab50`:
  `jcode-provider-openrouter-runtime/src/lib.rs` (2982 -> 3028) and
  `openrouter_provider_impl.rs` (1264 -> 1268). `scripts/check_test_size_budget.py`:
  `openrouter_tests_partition_01_tests.rs` (1253 -> 1260). All three came from the provider-profile-headers
  commit. Shrink them, don't `--update`.
- Guard helper quirk: the destructive-command gate refuses `mv` into scratch and `VAR=$(mktemp)`
  reassignment. Use `git stash push -u -- <paths>` and `mkdir -p "$JCODE_SCRATCH_DIR/x"` instead.

## 4. Next nodes, in order

**Status 2026-09-26T08:52Z (session calf):**
- **Done:**
  - **G1:** dotfiles `adbf93e`.
  - **G5:** dotfiles `f6c28a8`. A 40k-case differential fuzz against the Rust rule found 0 mismatches. All four policy gates pass, and the three agent surfaces are rendered.
  - **Node 5 (A2):** done by session wyvern (`c232ad32d`).
- **G4:** closed as a no-op.
  - The Together 502s were a single burst (09-24, 18:27 to 18:29). About 640 successful calls since.
  - jcode `ProviderRouting` has no `ignore` field.
  - The guardrail's `ignored_providers` field is account-wide.
- **Node 4:**
  - Upstream PR https://github.com/1jehuang/jcode/pull/1511, branch `ianalitis:pr/pan-precheck-content-filter`, based on upstream `b5a4cde7a`.
  - Tests: 145 + 142 provider tests pass, the TUI regression passes, and clippy reports nothing in the changed files.
  - No separate issue was filed because the PR body states the problem.
- **Node 6 (N1):** local `88f14b00e`, upstream PR https://github.com/1jehuang/jcode/pull/1512.
- **Node 7:** `ec63aaa46` (test env isolation, see §3). The tests fail on the baseline and pass with the fix.
  Upstream port: PR #1513 (`c266e1d02`). The upstream helper had no session-var clearing, so the port clears only `JCODE_NAMED_PROVIDER_PROFILE`.
- **Review fixes (09-26):** #1511 head `9c85d8e28`, #1512 head `22a928616`, all threads resolved.
  - Both fixes have red/green proof: the #1512 dedup mutation logs 1100 times instead of 1.
  - Local ports are `de3de5f0f` and `45350c77d`.
  - `f04a7f58a` isolates two auto-poke tests that raced on config under parallel runs.
- **Second review round (09-26):** #1512 `e2be52fd2` logs a one-time notice when the 256-reason cap drops a new reason.
  - #1513 `6f39a1d37` restores the named profile from a drop guard, so a caught panic restores it too.
  - Both regression tests fail without their fix, and the local ports are `642174343` and `9404b48ed`.
  - The local port guards all three session-provider keys, not just the named profile.
  - Third round: #1513 `871229e55` (local `8663903fd`) moves all cleanup, including `JCODE_HOME`, into one drop guard.
    Without the guard, the test fails with `JCODE_HOME` still pointing at the deleted temp dir.
  - Fourth round: #1513 `d4132078d` (local `82839396e`) holds the env lock for the whole panic probe through a `_locked` helper.
    Picker cache tests flake about 1 run in 3 on the unchanged base as well.
  - #1511 P1: a loopback base could redirect a card-bearing POST to a remote host.
    `139cb43e7` (local `87d3eec4d`) drops the loopback exemption; the local port also fixes the single-send caller.
- **Credentials:** `openrouter-provisioning` re-verified by a read-only guardrails call (dotfiles `f9e5e83`).
- **Next:** track #1511/#1512/#1513 Greptile, then the remaining Phase 2 N2-N7.

1. **G1** (dotfiles, approved): fold `~/.jcode/scratch/or_guardrail_fix.sh` into
   `~/dotfiles/scripts/openrouter-admin.sh` as `guardrails` (read), `guardrail-set <id> --builtin slug=action`,
   `guardrail-add-filter`, `keys-create --limit --reset`, `keys-delete`. Each mutation prints the
   body and needs `--yes`. The key is read from Keychain `openrouter-provisioning-key`, sent by
   stdin header, never printed. Custom filter patterns must be flat (no quantified groups). The
   PATCH replaces whole collections, so always send the full builtin set. Check: `guardrails`
   output matches `or_guardrail_after.json`. Update `config/infra-credentials.v1.json` `lastVerified`.
2. **G5** (dotfiles): `policy/global.md` outbound-data sentence ("four-digit identifier lists
   are not PAN; PAN is issuer prefix plus Luhn, and the harness checks that before send").
   Then `scripts/generate-agent-surfaces.sh`. Add the same rule plus a self-test to
   `scripts/pii-scrub.py`, reusing G2's grouping rules (`[15]|[16]|[4,4,4,4]|[4,6,5]`, issuer, Luhn).
3. **G4**: `cf-openrouter` provider pinning for `openrouter/auto`, `provider.ignore=["Together"]`
   (the 502 Invalid URL), set through `config/model-lanes.v1.json` +
   `scripts/render-model-lanes.py --write`, never by hand-editing `~/.jcode/config.toml`.
   One live `jcode run` to confirm.
4. **Upstream packet for G2+G3** (per `CONTRIBUTING.md`, issue first): "OpenAI-compatible
   providers: pre-send PAN check and legible content_filter errors". Port only
   `pan_check.rs`, `content_filter_block_label`, `terminal_errors.rs`, `should_retry` and
   the tests onto a `pr/*` branch from `origin/master`. Filing the issue or opening the PR needs its own approval.
5. **Phase 0** (approval A2): merge `origin/master` `74577fe83`, `cargo check --workspace`,
   `selfdev build-reload`, push to `fork`.
6. **Phase 2** N1-N7 from the plan, §6 (LaTeX warning spam first: about 17k lines/day).
7. Bisect the two Copilot-routing test failures (§3).

## 5. Continuation prompt

> Continue from the current closeout at the top of this file, not the historical node
> sequence. PR #1511 review and Phase 2 source changes are complete. Recheck HEAD,
> runtime identity, lease and review freshness before new work. Runtime promotion,
> unrelated gate repairs, optional upstream publication of N2-N4 or `f04a7f58a`, and
> Phase 3 require their own scoped authority. Keep literal PANs out of files and
> prompts, sign scoped commits, preserve other sessions' changes and release leases.
