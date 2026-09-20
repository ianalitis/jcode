# Source reconciliation and auto-beta root cause: 2026-09-20 ~06:42 UTC

Continuation of `docs/HANDOFF_2026-09-20_DIRTY_CLOSEOUT.md` and
`docs/SOURCE_CLOSEOUT_REVIEW_2026-09-20.md`. This is a fresh observed receipt, not a
release or whole-workspace acceptance claim. Canonical checkout:
`/Users/ianalitis/.jcode/source/jcode`, branch `jcode/ci-format-baseline`.

## 1. Why `openrouter/auto-beta` is refused (resolved)

The refusal did not come from the canonical source branch. Two independent causes:

1. **Runtime binary is a different branch.** `~/.local/bin/jcode`,
   `~/.jcode/builds/current/jcode` and `~/.jcode/builds/shared-server/jcode` all
   resolve to `~/.jcode/builds/versions/77cdac8ff-dirty-f4f975f58de9/jcode`, built from
   commit `77cdac8ff` on `jcode/data-class-admission`. That binary contains
   `ensure_nested_router_admission_is_integrated` in
   `crates/jcode-provider-openrouter-runtime/src/openrouter_provider_impl.rs`, added by
   `1cb2479fa` "reject unadmitted routers before provider dispatch". It fails closed on
   any nested router slug (`auto`, `auto-beta`, `pareto-code`, `:free`) because dynamic
   router pricing cannot be enforced as a per-request hard bound. That guard is **not**
   present in canonical HEAD, and neither is `jcode_attempt_types::is_nested_router`.
2. **System configuration asked for a router.** The `/model` switch in the operator
   session wrote `default_model = "openrouter/auto-beta"` and
   `default_provider = "openrouter"` into `~/.jcode/config.toml` (line 127).

Resolution applied: `default_model`/`default_provider` reverted to the prior concrete
lane `openai-oauth:gpt-6-astra` / `openai-oauth` (matching
`config.toml.pre-effort-routes-20260919.bak`). Backup:
`~/.jcode/config.toml.pre-autobeta-revert-20260920.bak`. No provider code was changed.

Enabling a nested-router default for real still requires the frozen router admission
(J2/J5) integration, which is unmerged and explicitly not admitted end to end. Until
then a concrete model ID is the correct default.

## 2. Branch state (read-only inventory)

Remotes: `origin` = `1jehuang/jcode` (upstream), `fork` = `ianalitis/jcode`.

- 44 local branches; the local `origin/master` ref is stale (upstream was force-updated),
  so `ahead/behind` counts are not trustworthy.
- The **active build channel is not canonical**: version `77cdac8ff` comes from the
  `jcode/data-class-admission` worktree, not from `jcode/ci-format-baseline`.
- `jcode/data-class-admission` carries 9 commits not present in HEAD:
  `ea15b57be`, `0b7d83048`, `e60efd965`, `5c1528295`, `f4e4d463e`, `107f55b3b`,
  `68a6a4093`, `1cb2479fa`, `77cdac8ff` (data-class admission table, bounded nested-router
  admission, served-model receipts/spend ledger, swarm effort routing).
- `git cherry HEAD <branch>` reports 48 branches with commits not patch-equivalent to
  HEAD. These counts conflate ordinary divergence with genuinely unmerged work and must
  not be read as a merge list: `master` and `jcode/browser-owned-session` alone show
  ~1800 each from the stale shared ancestor.

No merge, rebase, branch deletion, worktree change, fetch/push, or upstream convergence
was performed. Those remain separately gated.

## 3. Dirty-tree verification (automated)

A deterministic check compared every non-blank deleted line in `git diff` against the
union of added lines and untracked file contents:

- 24,581 non-blank deleted lines; **8,444 (34.4%) are not reproduced verbatim**. The
  dirty tree is therefore **not** a pure mechanical extraction; it contains real edits
  (route hooks, description trims, production splits). Blanket commits remain unsafe.
- `include!("…")` moves were classified as `EXACT` (byte-identical) or `DEDENT`
  (whitespace-only dedent). Only those were adopted; multi-target splits and files with
  residual edits were left untouched.

## 4. Accepted commits this session (branch `jcode/ci-format-baseline`)

| Commit | Scope | Verification |
| --- | --- | --- |
| `e57cd174b` | provider-metadata: move profile constants into `catalog_profiles.rs` | byte-exact; `check --all-targets` + 18 tests |
| `dbbef6a4a` | base: 6 include! moves (oauth, provider_catalog, session, config/provider tests) | byte-exact; `check --all-targets`; 1406 lib tests listed |
| `c40897bf3` | app-core: memory sampling + 4 test partitions | byte-exact; `check --all-targets`; 1370 lib tests listed |
| `4eb619733` | tui: 8 test partitions | byte-exact; `check --all-targets` |
| `381bfcc63` | e2e: reload-cycle helper | byte-exact; `check --test e2e` |
| `44afc1ed9` | tui-mermaid: width-selection tests | dedent-only; `check --all-targets` |
| `81802521e` | base: skill tests | dedent-only; verified in combined check |
| `00b469a7e` | provider-doctor: provider e2e tests | dedent-only; verified in combined check |
| `3af3bc779` | tui: 5 more test partitions | dedent-only; `check --all-targets` |

All new files are rustfmt-clean under edition 2024. Dirty footprint dropped from 159
tracked-modified + 112 untracked to **129 tracked + 82 untracked**.

## 5. Remaining work (unchanged in kind, smaller in count)

- 32 `include!` splits with real edits or multi-target fan-out still need per-file
  normalized review (production splits for agent/turn loops, client lifecycle, tool
  registry/zls, update, session, translate, anthropic/openrouter tests, tui app input /
  inline image / header, ui_messages, session_picker).
- Non-move edits: J5 spawn-envelope/route-contract work, route-class hook, description
  trims, and the `jcode-provider-openrouter-runtime` changes.
- Native gates (production/test size, panic-prone, swallowed-error) still fail for
  whole-tree debt; no baseline was relaxed.
- Branch integration, upstream PRs, push, install, and shared-daemon restart remain
  separately gated and were not performed.
- Host Bedrock credential-file recovery (`~/Library/Application Support/jcode/bedrock.env`)
  still requires named operator approval.

## 6. Concrete next steps

1. Continue adopting verified `EXACT`/`DEDENT` moves crate by crate; stop at multi-target
   splits that need semantic review.
2. Decide whether to integrate the 9 `jcode/data-class-admission` commits into canonical,
   or to rebuild the active channel from canonical so runtime identity stops diverging.
3. Keep a concrete `default_model` until frozen router admission is actually admitted.
