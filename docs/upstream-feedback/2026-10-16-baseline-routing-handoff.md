# Release baseline and routing closeout

Checkpoint: 2026-10-16 23:52 UTC. The operator requested a stopping point before losing connection. No further implementation or profile promotion is being attempted. This document records accepted local code, remaining work, and evidence for a fresh session.

## State and authority

- Primary checkout `/Users/ianalitis/.jcode/source/jcode`, branch `pr/fu149-content-filter-hint`, is clean at `921807b98d74f2fcd102570327547189e70dd8f3`. It has NOT been fast-forwarded.
- Approved isolated checkout `/Users/ianalitis/.jcode/scratch/release-sync-20261016`, branch `jcode/release-sync-20261016`, has accepted code at `ba9a018c91f1d67d25586f9529330ec009773d64`. This closeout is a documentation-only successor, not a new runtime-tested code revision.
- No push, PR publication, upstream submission, release, runtime activation, installation, provider/auth change, host configuration write or banked reset redemption occurred. Local integration is not remote synchronization.
- Delegated writers are stopped. Captain is closing the remaining reviewer and releasing common Git lease `release-sync-20261016` using the existing dotfiles `scripts/repo-lease.sh`. A fresh session must verify ownership, worktrees, HEAD and index before writing.
- Use configured Git identity and preserve unrelated changes. Publication, hosted dispatch and runtime activation remain separately gated.

## Accepted source commits

| Purpose | Commit |
| --- | --- |
| Starting fork | `921807b98d74f2fcd102570327547189e70dd8f3` |
| Baseline recovery | `ed544dfc200b8c398d07ea0f246f5927cb48bf94` |
| Stable integration merge | `c5d5e5ab49ca87714accec053434630e5ee3cefc` |
| Actual secret PTY CI owner/filter and nonzero guard | `d9cb7c53bce0c4ce31f395f960255c589dd64eb3` |
| Discriminating hotkey numeric positive control | `a988dfb88934ad89dda8f1d709f83fab498b1a8d` |
| Four isolated fixture repairs | `c3240dba10a12a08c48c9ea87ef27cfd3882af83` |
| App-core test profile and exact hook-test ownership | `ba9a018c91f1d67d25586f9529330ec009773d64` |

Stable v0.98.1 annotated tag object is `61ed7d9802058e8d93894e5931f620732f7b349a`, peeled commit `1c7426eb1925a1167eb56011fa479ad707cc28b6`, pinned ref `refs/upstream/release-20261016`. Original merge base is `94392e100dd05868172c9884944d97bd1eee5d8e`. Merge c5d5 has exact parents ed544 and 1c7426, tree `bd61e00e64da0ab9846327d8139f818d84af0972`; both ancestry checks passed.

Baseline recovery retained assertions and test inventory while extracting cohesive existing owners, replacing three cloned-plan requeue expects before persistence, and logging startup acknowledgement/heartbeat outcomes without canceling admitted work. Independent Astra dove reviewed all14 files and passed289 focused tests. Captain22 gates passed;1827 test leaves unchanged, nine qualified relocations.

Astra fish resolved11 integration conflicts. Astra otter independently reviewed semantics and panda reviewed budgets/workflows. Preserved: single tokio test-util dev-dependency; complete upstream subscribe-context owner with live/busy cwd semantics, explicit load causes and no daemon-cwd fallback; companion fallible shutdown; distinct account-route regressions; copy-pane extraction; saved-account/deferred-tool ownership; upstream message/name-suggestion implementations without duplicate local definitions; strict hotkey identity parsing. All incoming CI/release jobs remain, including mandatory Windows ARM64, pinned actions, credential persistence disabled, telemetry opt-outs, locked checks, quality cancellation guards and browser timeout/nonzero assertions.

Secret CI now selects actual `jcode-terminal-prompt --lib secret::pty_tests` with a nonzero guard. The inherited zero-selection command was exposed, not counted as passing. Hotkey numeric positive control uses a strictly later timestamp so event deduplication cannot make it vacuous. Linux execution is still unverified on this Mac.

Budget reconciliation used actual source attribution, not JSON arithmetic. Actual B/L/U/C panic counts179/178/162/161 and swallowed3884/3881/3714/3711. Final ceilings remain upstream162 and3714; pattern ceilings1429/1355/930, actual1429/1354/928. Three existing helpers `.ok()` sites moved to helpers_clock with no global increase. Four detectors unchanged. Six production-size and one test-size retained differences remain within historical local ceilings. No budget inflation or suppression.

## Final accepted local validation

Runner `/Users/ianalitis/.jcode/scratch/release-sync-acceptance.sh`, shared target `/Users/ianalitis/.jcode/source/jcode/target`, host-wide Cargo gate and telemetry off.

- ed544:22 gates passed, evidence `release-sync-acceptance.MDTckZ`.
- c5d5:35/36, correctly failed zero-test secret selection, evidence `release-sync-acceptance.vWT6Yb`.
- a988:36/36 passed, evidence `release-sync-acceptance.VFcJNM`.
- **ba9:36/36 passed**, task783466ko80,162.31seconds, evidence `/Users/ianalitis/.jcode/scratch/release-sync-acceptance.KHVTm2`.

Final gates cover modules/four ratchets/dependency and wildcard boundaries/fmt/actionlint; sync28/CI guards5/release and recovery Python cohorts; wrapper and warning contracts; locked offline metadata; all-target/all-feature check; clippy with warnings denied; compiler warning budget0; SDK3/retention3/actual PTY2/browser4+3/stdin2/provider9/E2E60 plus7 ignored; whitespace. Check/clippy warning0 does NOT establish warning-free final binary linking.

**ba9 full app-core matched comparison passed**, task593062fity, evidence `/Users/ianalitis/.jcode/scratch/app-core-corrected-match.LGRzWO`: explicit opt0 control1814 passed/33 ignored in78.58seconds; native opt1 same1814/33 in79.31seconds. Complete and ignored inventories identical. Exact two-name relocation mapping is saved in `inventory-relocation-check.txt`. Native artifact fresh=false, debug0/assertions/overflow retained, unwind frame0xae53bc below16MiB, no raw compiler/linker warnings, deliberate panic-catch regression passed both.

Four c324 repairs were fixture-only and independently reviewed: prepared persistence plus existence assertion for empty observer snapshot; temporary working directory for empty skill-list fixture; hook configuration invalidation after setting AND restoring environment in privacy/post-feedback tests. No assertions or timeouts weakened. At ba9, a redundant parent-supplied Tokio import was removed and two complete attributed hook functions moved byte-identically into private Unix-gated `tool/tests/hooks.rs`; only those two qualified names gain `hooks::`. App-core-only test opt1 was measured and independently approved. No production/profile-wide change or allowance increase was made.

Earlier bad harness runs disabled hooks, inherited a mise shim under isolated HOME and used long socket paths. Corrected matched harness uses real rustup bin, identical cwd, short isolated HOME/JCODE_HOME/TMPDIR/XDG_RUNTIME_DIR, hooks enabled, telemetry off. `JCODE_HOOKS_DISABLED` must be absent, not `0`. Browser runtime uses XDG_RUNTIME_DIR or /tmp. One earlier browser hang was canceled; mechanism remains unproven, later isolated and full runs passed. Do not claim a production browser fix.

## Remaining CLI linker warning and completed experiment

Committed ba9 eliminates app-core libtest's Apple compact-unwind warning but leaves the supporting root CLI warning in native provider/E2E builds: native root frame0x1506924 exceeds16MiB. SDK parity also intentionally prints one explicitly triaged TypeScript-only `close` diagnostic; do not hide it or invent an API to silence it.

No-source root-only opt1 and root+TUI opt1 measurements (`cli-unwind-experiment.L6QGhn`, task055299vmjp) still exceeded16MiB at0x1474c00 and0x12cda70. Exit0 meant measurement completion, not acceptance. No corresponding source settings were promoted.

Astra otter admitted ONE no-source global test-profile opt1 candidate, only `--config 'profile.test.opt-level=1'`, on clean ba9, retaining explicit package/dependency overrides. **This experiment completed successfully just as closeout began**, task336660no40,301.37seconds, evidence `/Users/ianalitis/.jcode/scratch/global-test-opt1.SRGu0r`, script `/Users/ianalitis/.jcode/scratch/global-test-opt1.sh`.

Reported candidate unwind frames: CLI0xd1a040, provider0x7954c0, E2E0x73854c, app-core0x7db3c4, each below16MiB. Candidate full tests:1814 passed/33 ignored, provider9 passed, E2E60 passed/7 ignored. Script exited0. Raw artifacts, diagnostics, profile and inventory checks are preserved in the evidence directory. This is a completed candidate experiment, **NOT a committed global profile fix, independent final review, or full36-gate acceptance under a new permanent default**. Inherited dev package wildcard0 may leave non-workspace dependencies unoptimized; do not describe global opt1 as optimizing everything. Fresh session should inspect all raw receipts, exact artifact profiles/features/assertions/overflow/unwind and rebuild scope before admitting a minimal permanent change, independently reviewing it and rerunning acceptance. No further package/linker/codegen tuning is authorized by the experiment itself.

## Automatic synchronization remains proposal-only

Reuse existing `.github/workflows/upstream-release-sync.yml`, scheduled every2h. Last read-only observation23:26UTC: enabled=true, lateststable v0.98.1, fork master `d2a6737885074b143f211b3fd61dd83a5a050cc3`. Run38967979155 created21:14:30UTC failed validate at pinned stable merge and skipped publish. Remote status is NOT fixed by these local changes.

Existing isolated validator and clean proposal publisher pin/check base, tag object, peeled commit and tree, reject drift, and do not force-push, write the default branch or auto-merge. GITHUB_TOKEN proposals do not automatically trigger ordinary CI. Exact-head dispatch/publication needs separate approval. No second scheduler/controller was added. Primary fast-forward and remote publication remain undone.

## Companion routing/configuration work

Dotfiles captain dog completed and released ownership. Accepted independently Astra-reviewed commits:

- `d39021103e880df5e9d85a61bc7b9b1efd508ace`: native six-key renderer, static ChatGPT defaults, stale clock/hold and enforcement doctrine removed. Renderer25, generator parity and full53 fixture suites plus specialist gates passed.
- `1aa9d0da98d7a5ffd14c67ee28986071d40a3d96`: removed unsafe execution of model-response Python in captain benchmark process and three executable tasks; obsolete suites reject before pricing/directories/dispatch. Benchmark18/dogwalk6/full53 passed. No live benchmark or sandbox added. Seven-task text scores are not comparable to historical ten-task aggregates or accepted implementation cost.
- `dba6a9f0add40f9dea36997ee7b7f50bae1d6510`: current FU143/147/173 and September28 document reconciliation, review/policy/link439/whitespace passed.

Preserve original foreign modified `docs/sessions/2026-10-13-r2-version-gc-cleanup-root-redproof-verdict.md` and untracked `docs/sessions/2026-10-13-r1-r9-integration-merge-closeout-receipt.md`. No host/auth/provider/spend/runtime activation occurred.

Real native failover fields are `provider.cross_provider_failover="manual"` and `same_provider_account_failover=false`. There is NO quota_fallback field; phantom empty keys do not enforce disabled fallback. Old policy claims about absent attempt-types/router-exclusion code were corrected. Real Config5 probe passed task742936d5cf using path dependency and existing lockfile, not a copied parser. It is representative proof, not a committed regression suite. Host selected values were already safe, no host write needed.

Operator direction: ChatGPT majority, Sol/low interactive starting route, Luna/medium routine scoped implementation, separately admitted Astra/high hard judgment and substantive review. Active treatments remain pinned. Go first for admitted planned supplemental phases, then separately admitted prepaid metered capacity, never hidden fallback/automatic spill. Actual Go tier/Use balance and authoritative windows remain unknown; missing/partial/stale windows never mean zero or admission, and rolling exhaustion does not prove monthly exhaustion. Strict spend/data/privacy/retry/resume/concurrency gates still apply. Operator upgraded to $500/month ChatGPT and authorized baseline work without previous subscription usage stops. Last native observation23:37UTC: promax, seven-day6% used, errornull, reset2026-10-23T21:07:12Z. No reset redeemed or paid fallback used.

## Deferred bounded native work

1. Go usage: `crates/jcode-base/src/usage/{api_keys.rs,tests.rs}` currently lacks quota branch; TUI may label empty limits '+ available'. Add pure validated complete/partial/error parsing only after endpoint/auth evidence. Observation freshness is not cache insertion time. Schema/cache/TUI/CLI changes require traced initializers and separate scope; unknown must remain non-admitting.
2. Bounded swarm list_models provider/query/limit: current full catalog is approximately77k tokens. Reuse native schema/server, not a wrapper/router.
3. Real per-spawn tools: schema exposes model/effort but no tools field. Prompt species is not enforced containment or filesystem/network sandboxing. Trace persistence/resume/customtools before claims.
4. Consolidate approximately22KB swarm doctrine through canonical projections without weakening safety. Reuse native context/cache/taskgraph/pins, not another orchestrator.
5. OrcaRouter already exists in native registry, CLI/login/docs and stable source. Pure metadata test571396vmja passed. Do not build a duplicate adapter. Registration is not live auth/privacy/cost proof; establish intended provider identity first.

Separate retrieval benchmark owner sloth recommends retaining agentgrep, no source/runtime/config change. Native and Xerj definition traces tied18/18 rank1; prose comparisons did not match evidence budgets/full workflows; synthetic manifest275vs274 blocks comparative relevance. Xerj indexing19-23seconds/RSS306MiB are costs, not accepted-task gains. Existing MCP boundary suffices for isolated experiments. Owner receipt `docs/measurements/2026-10-16-retrieval-backend-benchmark.md` was being gated, not accepted as source work here. External synthetic symlink observation remains a separate policy followup.

## Fresh-session execution order

1. Read this receipt and verify both checkout HEADs/index/worktrees, current lease and active workers. Acquire existing lease before mutation. Do not overwrite new user work.
2. Inspect completed global-profile experiment receipts and seek independent Astra judgment. If admitted, make one minimal permanent change, validate actual artifact profiles/unwind and rerun36 gates/full suites. Otherwise keep warning explicitly unresolved. Do not repeat completed fixture repair or invent further knobs.
3. Only after accepted baseline, consider approved clean local fast-forward. Publication, exact-head hosted CI and runtime activation remain separate approvals.
4. Keep Linux hotkey, Windows ARM64, FreeBSD and hosted release validation explicitly unexecuted on this Mac. Only aarch64-apple-darwin target is installed and Docker daemon unavailable; do not install/start implicitly. Rust SDK parity is not npm packaging acceptance.
5. Take deferred routing work as individually bounded packets with review, not scope expansion or a new controller. Preserve dotfiles foreign changes and existing review-only sync design.

No destructive scratch cleanup is part of this closeout. Evidence paths are intentionally retained for the next session.
