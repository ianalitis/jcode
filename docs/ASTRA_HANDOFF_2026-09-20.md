# Astra handoff: source reconciliation and budget-first harness

Prepared 2026-09-20 around 17:10 UTC. This is a continuation packet, not a claim that integration or routing is finished.

## 1. Operator objective and authority

Continue iterating on Jcode source reconciliation across dirty work, worktrees, branches, issues, PRs and upstream drift. Preserve and validate researched improvements rather than dropping unfinished work. Highest product priority is now rapid, safe improvement in token economy through cheap open-weight models and intelligent task routing.

The operator expects to retain approximately the $100/month ChatGPT plan as a supplement, not the main harness dependency. ChatGPT usage is depleted. Anthropic Max will not be renewed. Do not build the operating model around expiring-credit burn or routine premium subscriptions. No subscription purchase/cancellation action is requested.

User requested a fresh Astra agent handoff. This document does not itself launch a session or change the active provider. Existing provider policy still governs execution until explicitly reconciled: no hidden fallback, no unadmitted metered writable swarm, no privacy downgrade. User's strategic request is not an unlimited spend authorization. Obtain a concrete budget before funded autonomous experiments if none is already valid.

Earlier operator approved continuing integration and pushing to the fork. No approval to merge/push upstream, close PRs, delete branches/worktrees, reset baselines, discard changes, or weaken security. Read current AGENTS.md and applicable policy before mutation. Commit only inspected, verified scoped changes.

## 2. Fresh evidence at handoff

Canonical cwd: `/Users/ianalitis/.jcode/source/jcode`.
Branch: `jcode/ci-format-baseline`. HEAD before this document: `d027491f6` (`fix(storage): show scan progress`). Tracks `fork/jcode/ci-format-baseline`, with no ahead/behind marker at inspection. Canonical working tree was clean.

Remotes: `origin` is `1jehuang/jcode`; `fork` is `ianalitis/jcode`. Do not assume an `upstream` remote. Local `master` at `be248c641` is 75 commits behind locally known origin/master. Earlier origin/master observation: `e589cbe5a`, v0.86.0. Fetch/recompute before integration. Large historical divergence and overlapping modifications make blind merging unsafe.

CI: https://github.com/ianalitis/jcode/actions/runs/35524397694 was still `in_progress`, no conclusion, at 17:09 UTC. Check it directly with `gh run view ... -R ianalitis/jcode`. Do not infer success from a watcher exiting zero. Previous watch tasks timed out at 600 seconds. Avoid repeated pushes that cancel running builds.

Fork open PR query returned none. Upstream PRs #1293 and #1295 are OPEN and CONFLICTING. Earlier evidence suggested equivalent fixes already in canonical, but verify exact equivalence before proposing disposition. Do not close them without authorization. Full upstream issue/PR inventory remains outstanding.

### Runtime identity blocker

Fresh symlink inspection:
- `~/.local/bin/jcode` -> `~/.jcode/builds/current/jcode`.
- `builds/current/jcode` -> `builds/versions/d027491f6/jcode`.
- `builds/stable/jcode` -> `builds/versions/d027491f6/jcode`.
- **`builds/shared-server/jcode` still -> `builds/versions/77cdac8ff-dirty-f4f975f58de9/jcode`.**

Background build/install task `751518uf3r` succeeded in 287 seconds using canonical `scripts/install_release.sh --fast`. Its text says stable updated, but both current and stable now resolve to d027491f6. Reload interruptions do not prove the running daemon is current. Inspect actual process/socket identity before behavior claims.

`selfdev build-reload` previously targeted the wrong checkout, `source/worktrees/data-class-admission`, and refused publication because its binary hash `f4e4d463e` did not match source `77cdac8ff`. Do not keep building that checkout or bypass the mismatch. Inspect selfdev status/config and repair scope through supported mechanisms. Private-socket runtime tests are the safe fallback, not silent shared-daemon disruption.

### Worktree inventory

All listed worktrees were clean except the one explicitly marked:

| Worktree under ~/.jcode/source/ | HEAD | Branch / state |
|---|---|---|
| jcode | d027491f6 | jcode/ci-format-baseline |
| jcode-browser-owned-session | ae62c4fb2 | jcode/browser-owned-session |
| worktrees/ci-env-verify | 091618754 | detached |
| worktrees/data-class-admission | 77cdac8ff | jcode/data-class-admission; UNTRACKED file below |
| worktrees/focus-report-loop | 108caabd2 | jcode/focus-report-loop |
| worktrees/focus-runtime-safe | f25eb9d11 | jcode/focus-runtime-safe |
| worktrees/fork-ci-secretless | 32961d965 | jcode/fork-ci-secretless |
| worktrees/macos-terminal-tail-match | 5d501a7cb | jcode/fix-macos-terminal-tail-match |
| worktrees/notification-child-reaping | 535a479d1 | jcode/notification-child-reaping |
| worktrees/plugin-manifest-discovery | ba5201141 | jcode/plugin-manifest-discovery |
| worktrees/privacy-no-collection | c81735fdf | jcode/privacy-no-collection |
| worktrees/route-receipts | 13d3b3caf | jcode/route-receipts |
| worktrees/wake-mode-fingerprint | 510f7c3ff | jcode/fix-wake-mode-fingerprint |
| worktrees/workflow-shellcheck | 98cee29c7 | jcode/workflow-shellcheck |

Preserve and inspect untracked `worktrees/data-class-admission/crates/jcode-harness-api-server/src/translate_regression_tests.rs`. Its provenance and integration status have NOT been established.

## 3. Router investigation: evidence and limits

Earlier session live smoke used canonical `target/debug/jcode` with an isolated socket, not shared daemon:

| Requested model | Result | Interpretation |
|---|---|---|
| openrouter/auto | OK; ~17,313 input tokens | Canonical path works, trivial prompt still expensive in context |
| openrouter/auto-beta | OK; ~17,345 input | Same |
| openrouter/pareto-code | OK; ~12,607 input | Valid route |
| openrouter/pareto | HTTP 400 | Invalid model ID |
| unbiased/pareto | HTTP 404 | Account AND guardrail ZDR exclude sole endpoint |
| openrouter/free | HTTP 404 | ZDR excludes endpoints, some also excluded by free-training policy |
| openrouter/fusion | HTTP 402 | 65,536 requested output exceeds key budget; expensive Opus routing unsuitable |

These are prior-session smoke observations, not repeated during this documentation turn. They prove basic responses, NOT safe spend enforcement, tools, useful coding quality, privacy guarantees, caching, or unattended readiness.

Old installed runtime reproduced: `nested router model openrouter/auto is disabled on the normal provider path until frozen router admission is integrated end to end; pin a concrete model ID`. That guard belongs to the deferred data-class-admission branch, not canonical HEAD. Therefore distinguish stale-runtime guard, invalid ID, privacy-filtered availability, and budget error. Do not solve all four by disabling a guard or relaxing ZDR.

Public catalog also exposed `openrouter/bodybuilder`, but no validation was recorded. Similar names do not imply compatibility. Consult current public router documentation before using fields such as cost_tier, allowlists or coding-score filters.

Operator enabled two OpenRouter classifiers and logs activity using Jev. Exact classifiers, logs, data handling and integration are not verified. Do not infer that account-side classification is local routing authority or that logged repository data is approved for reuse.

## 4. Existing architecture and implementation footholds

Read, reconcile and amend rather than duplicate:
- `docs/SOURCE_RECONCILIATION_2026-09-20.md` (receipt, some historical counts may be inaccurate).
- `docs/plans/TOKEN_ECONOMY_PLAN.md`.
- `docs/HARNESS_LOOP_ARCHITECTURE.md`.
- Applicable current provider/upstream/skill policy and the referenced token-economy measurements. Locate them rather than assume all are in this checkout.

Old token plan treats subscriptions as sunk capacity and includes frontier burn scheduling. Operator direction supersedes that objective. Keep its accepted-work cost framing and measured-context emphasis, replace subscription dependence and unvalidated assumptions. Do not treat its proposed router fields or model prices as current facts.

Historical measurement in that plan: 17,718 OAuth requests, 2.08B input tokens, 95.5% cache reads, average 117,580 input / 542 output. API-equivalent cost is NOT an actual bill. Replaying that much context on cheap uncached endpoints can still be expensive. Historical OpenRouter balances/limits are stale; inspect fresh metadata without printing credentials before experiments.

Current config observations: default `openai-oauth:gpt-6-astra`; worker `openai-oauth:gpt-5.6-terra`; headless workers; concurrency 3; memory sidecar off. Empty `[agents.swarm_effort_models]` and `[agents.swarm_effort_tools]` tables exist, but canonical AgentsConfig lacked those fields. Do not assume they work.

Deferred branch commits provide possible bounded reuse:
- `5c1528295`: effort-to-worker-model mapping.
- `f4e4d463e`: effort-to-worker-tools narrowing.
These are not a task classifier or complete admission system. Inspect diffs, tests and dependencies individually. Do not cherry-pick the entire data-class branch just to obtain them.

Local advisory lane observed at `http://127.0.0.1:11234/v1`: Ornith 9B, gemma-4-e2b, Qwen3.8-27B MLX profiles. Availability now is unverified. Local models can label/rank/extract or abstain within closed sets, never widen authority or independently dispatch arbitrary routes. Do not change auth/provider configuration just for a handoff.

## 5. Prioritized next phases and acceptance contracts

### Phase 0: stabilize evidence and preserve work (first)
1. Read this packet and repo rules. Inspect status/rebase/cherry-pick state in canonical and dirty worktree. No destructive reset.
2. Check CI run and failed job logs. Repair only reproduced failures, not a blanket baseline refresh.
3. Resolve selfdev checkout and runtime identity. Confirm binary hash, private socket and loaded code. No unsupported claim that a build updated the daemon.
4. Inspect untracked regression test before any checkout cleanup.
Acceptance: trustworthy runtime receipt, explicit CI result, preserved/inventoried local changes.

### Phase 1: budget-first minimal slice (highest feature priority)
Make a small deterministic routing/admission core before classifiers or broad automation. Select concrete candidate models from current verified catalog and policy, biased toward cheap open-weight DeepSeek, GLM/Z, Qwen/Alibaba, Tencent and comparable options. Lab preference is not a substitute for endpoint quality, licensing, pricing or privacy evidence.

Initial task classes: read-only discovery/extraction; summarization/docs; mechanical changes; narrow bug/test fix; routine implementation; independent review; architecture/security/ambiguous recovery. Reuse existing task_class definitions if present instead of inventing a second taxonomy.

Inputs: explicit task class, data eligibility, allowed tools/files, risk, exact verification command, maximum output/context, deadline, concurrency and task budget. Classifier may only label or abstain. Deterministic admission owns effects and route eligibility. Missing provenance remains private. Freeze requested model/router, effort and tools for each attempt. Failure closes the attempt with a visible receipt, never silently changes provider/account/model.

Implement with synthetic fixtures and mock transport first. Exact first code change should follow inspection of current admission/spawn code, not this document's nouns. Prefer one narrow end-to-end vertical slice over generic routing infrastructure.

Acceptance tests: permitted selection; unknown class abstention; private packet rejection on unapproved route; classifier cannot widen tools; unknown price denied; output cap enforced; budget reserve/release including failure/cancellation/concurrent attempts; no network dispatch after denial; no silent fallback. Do not enable writable/unattended metered workers until whole-task spend and account/model admission are actually enforced.

### Phase 2: shrink packets and measure accepted-work economics
Measure context bytes by source and token/cache usage. Send only bounded task packet, relevant file snippets and required tool schemas. Cap tool outputs with explicit continuation. Preserve stable cache prefix. Do not apply arbitrary global compaction changes before retention tests.

Start a small operator-budgeted public/synthetic evaluation with pinned cheap route versus constrained dynamic router. Record requested router AND served model, endpoint, requested envelope, actual billed cost where available (otherwise explicit unknown), cache usage, latency, verification result and review burden. No secrets/raw private packets in receipts.

Acceptance: paired baseline and candidate on identical fixtures, cost per accepted task and failure rate, context reduction without correctness loss, enforceable daily/task ceilings. Cheap token price alone is not success. No two-week discovery bureaucracy is necessary before a safe bounded pilot, but promotion needs real evidence.

### Phase 3: adaptive routing without adaptive authority
Use verified allowlisted routes on the measured cost/quality frontier for each class. Deterministic explicit labels first, local closed-set classifier second, Jev only on eligible sanitized packets. Handle abstention visibly. Update future-attempt selection based on verified receipts, never change an in-flight frozen treatment. Test nested routers against current documented constraint semantics and reject routers that cannot enforce required privacy/budget bounds.

OpenAI is optional deliberate supplement for selected difficult work, not automatic overflow. Anthropic is not assumed available after subscription expiry. Preserve current policy until approved changes are made to canonical policy sources and regenerated surfaces, not ad hoc prompt/config drift.

### Phase 4: finish source integration and upstream alignment
Run this alongside bounded routing work with at most one writer plus narrowly scoped readers, not a costly broad swarm. Build branch ledger: patch IDs/equivalent canonical commit, remaining unique work, tests, disposition and approval needed. Read dirty work before cherry-picking. Do not declare a branch integrated because one of its commits landed.

Refresh remote refs only through normal authorized workflow, then review upstream drift in cohesive groups. Large fork divergence merits explicit integration plan and regression matrix before merge/rebase. Keep fork CI and upstream publication distinct. Retire branches/worktrees only after explicit deletion approval.

Acceptance: every unique change accounted for, every kept behavior has evidence, CI and platform checks pass, known failures recorded honestly, upstream PR decisions separately approved. Not merely a clean canonical `git status`.

## 6. Reconciliation already performed (verify source for exact state)

Earlier work integrated ~200-file dirty-tree extraction/module closure and many branch slices. Extraction admission used rustfmt-normalized parent/include splicing equivalence, not blanket acceptance. Key receipts:
- `4784b7a1a`: module/extraction closure, protocol/spawn budget and provenance fields, dependency edges.
- `666ca37c6`: explicitly approved swallowed-error baseline reset. Do not reset again without asking.
- `113c0cd06`: headless/cleared session persistence invariant fix.
- `68b341633`: secretless fork CI; `50db42c64`, `5dfb9a674`, `8225b6ace`, `fdc197dc9`: platform/test/dependency fixes.
- `629332543`: scanner-safe redaction fixtures, not broad tests exemption.
- `ed2189011`: size-ratchet extraction repairs.
- `d834ffc05` / `14bcf8b21`: prompt dedup and optional-files extraction.
- `f7a67a795`: macOS stdin false-positive fix.
- `26439069c`: bounded 256-entry working-git-state cache.
- `f5604cb98`: Ctrl+5 regression.
- `34f2fbd2d`: TUI lock/deadlock work. Review panic-budget regex broadening if touched.
- `043ee50f3`: compaction metrics regression.
- `bc05abad1`: h2 0.4.19 and rustls 0.23.45 security updates (RUSTSEC-2026-0258/0285).
- `28f45c066` / `af5d8f28c`: memory-off avoids eager embeddings and swarm-GC extraction.
- `5f66db7ff`: resvg/usvg alignment; `d4dfb21d1`: agentgrep v0.1.7.
- `2427fe337`: Anthropic metadata test; `b3efd3d98`: decimal-version fallback parsing.
- `28be582aa`: compatible-provider auth aggregation; `406dfd6df`: explicit model on resume.
- `2ee8049e0`: socket permission fail-closed; `93f057abb`: webfetch SSRF checks.
- `3b40c0c62`, `48f64eddf`, `88d6c40c7`: cache/journal/serialization improvements.
- `bbdd552bf`, `d027491f6`: read-only storage reporting/progress.
Other slices include git-probe isolation, background output adoption, focus-report loop, notification-child reaping, missing-provider failover, runtime socket preservation, cached-token accounting, project MCP trust and gateway loopback. Resolve exact commit attribution with git history rather than reconstructing hashes from memory.

## 7. Important unfinished or uncertain work

- **Discovery privacy gating uncertain**: commit `3e7b937ad` was seen before an aborted cherry-pick sequence; later test count decreased. `493363ad7` test isolation landed, but verify substantive discovery opt-in policy exists before claiming privacy work integrated.
- Full TUI suite was NOT green: prior run ~45s, 2308 passed, 5 failed, 18 ignored. Four deterministic macOS Alt versus option-symbol label mismatches and one parallel changelog flake. Reproduce narrowly. CI success alone may not cover it.
- Local `cargo-audit` absent. Non-strict security preflight pass does not prove audit pass. Do not install tools without approval or silently skip strict checks.
- Full workspace test completion not established. Platform-only cohorts and provider tests need explicit coverage.
- Remaining Fable history commits: `ffcb4a89a`, `1d0653d03`, `5a1ac47c1`, `0d16b4aa2`. Ordered assistant blocks span runtime/core/TUI; not a mechanical pick. Preserve work even if Anthropic is now lower priority.
- `fbeaa226f` bash approval redesign is a security decision, not routine cleanup.
- Gemini retirement branch (4 commits), issue #1110, remains unintegrated. Gemini not an implicit fallback.
- `jcode/auth-preserve-billing-route` includes DeepSeek admission/budget reservation work. Inspect prerequisites and integration rather than reimplement blindly.
- `jcode/privacy-no-collection` and data-class-admission remain substantial deferred work. Classify each unique commit; do not import guard-only behavior and break routers again.
- Browser-owned-session, plugin-manifest-discovery, route-receipts, fork CI/env/shellcheck and all backup/perf/fix branches still require ledger-based disposition. No branches deleted.
- Webfetch ordinary fetch SSRF improvements do not prove the separate evidence `research_client` path has equivalent coverage.
- Storage status reported roughly 637 GiB apparent data (363G source, 266G scratch, 4.5G builds, 2.2G sessions). Reporting is not deletion authorization.

## 8. Verification and practical commands

Start with read-only `git status --short`, `git worktree list`, `git branch -vv`, `git log -5 --oneline`, `gh run view 35524397694 -R ianalitis/jcode`, then targeted source inspection. Inspect untracked file without executing it.

Use repository test/build wrappers and coordinated selfdev only after correcting checkout identity. `cargo` may be wrapped by `scripts/dev_cargo.sh` with isolated test home/scratch. No GNU `timeout` on this Mac. Tool background operations have repeatedly timed out after 600s despite longer requested limits. Prefer bounded commands and bg wait, not endless polling. Preserve nonzero exit status through pipelines.

Verification ladder: narrow failing reproduction -> narrow tests after edit -> related crate/cohort -> six project quality gates plus fmt/clippy as appropriate -> strict security checks on equipped CI -> actual runtime via known binary/private socket -> final diff review -> scoped commit. Check repository scripts for exact current commands. No blanket `git add`, reset --hard, baseline bump or suppressions. Extraction moves require semantic-equivalence proof, not merely compilation.

## 9. Fresh Astra opening packet

“Continue from docs/ASTRA_HANDOFF_2026-09-20.md in /Users/ianalitis/.jcode/source/jcode. First verify runtime/selfdev identity, CI run 35524397694 and the untracked data-class regression test. Preserve every worktree. Highest feature priority is bounded budget-first cheap open-weight routing with smaller context packets, enforced privacy/spend limits and accepted-work measurements. OpenAI is now supplemental and Anthropic Max is not renewing. Reconcile existing plans and admission branches, implement the smallest tested vertical slice, and maintain a complete branch/PR/upstream integration ledger. No upstream publication, deletions, hidden provider fallback, new spend limits or privacy relaxation without approval.”
