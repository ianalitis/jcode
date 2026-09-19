# Jcode source closeout and branch-resolution plan

Snapshot: 2026-09-19 UTC. This is an integration plan, not permission to bypass gates or a claim that all branches are merged.

## Outcome and authority

The operator requested iterative commit/integration with the personal fork and planning only for upstream issues/PRs. No upstream issue or PR was created, modified or closed. No force-push, branch deletion, worktree mutation, install, daemon promotion or baseline increase occurred. Remote refs were fetched without pruning. No source branches were merged or pushed because the live working tree and repository acceptance remain blocked. Do not mistake local checkpoint commits for release acceptance.

Completed local packets: async test lint cleanup, todo predicates, TUI predicate and fixture lint cleanup, failed ModelCall Verify receipt rejection, and coverage-preserving Verify-receipt extraction. Pending independently reviewed A1 Pi terminal-outcome fix was committed unchanged as `0f216f754521e74df99052ae780c67ffbc31597d`; fresh offline checks pass 16/16 plus strict crate Clippy, format and integration. The preceding receipt/extraction commits are `fabc9a5aebcab1c9847f789158bc4c969a024990` and `e0b7782378baca3ef62d6a7792dec834e16c48cd`.

## Critical reconciliation facts

- Source snapshot: `0f216f754521e74df99052ae780c67ffbc31597d` on `jcode/ci-format-baseline`.
- Personal fork master: `e09acaa7a8828bcd46d9a5ab7c3434112e04e713`.
- Upstream master after fetch: `eebc4996fbb57e9534bb6a030a5ac232bf90681c`.
- Fetch reported upstream master force-updated from `5cb7b3dad` to `eebc4996f`. At the pre-closeout-commit snapshot, upstream/source divergence was 1921/49 commits, whereas fork/source was 4/49. A further Pi commit was then added. These are ancestry counts, not counts of unique behavior or proof of a bad upstream update. Do not merge the rewritten history wholesale or use `ours` to conceal it.
- The source worktree still has a large concurrent extraction and unfinished protocol-axes work. The latest inventory records 281 dirty status entries after the Pi checkpoint (directory-collapsed porcelain counts). Those changes were preserved, not blanket-staged. Linked worktrees are inventoried below and were read only.
- The upstream and fork remotes are intentionally named `origin` and `fork`, respectively. Never push to origin under this closeout authorization.

## Next execution order and acceptance

1. **Reconcile the upstream history before choosing an integration base.** Inspect the fork-only four commits, compare merge bases, `git range-diff` and stable patch IDs for candidate topics. Confirm whether upstream history was intentionally rewritten. Select an explicit base with the operator if the old fork lineage cannot safely fast-forward. Preserve backup refs. Never force-push or reset to manufacture convergence.
2. **Close dirty ownership, one packet at a time.** The large extraction is not safe to commit merely because it compiles. Assign its owner, compare each moved body to its source, verify module resolution and exact test inventory, and inspect all staged files. Keep protocol-axes changes separate from extraction. The axes packet is still pending because its wider application integration and owned oversized-test growth are not accepted. No whole-tree staging.
3. **Restore source acceptance without waivers.** Resolve the four remaining native ratchet categories: production size, test size (app-core DAG e2e and swarm persistence), production panic use and swallowed errors. Preserve current baselines. Also fix known explicit crate failures: app-core description caps, base catalog completeness (check existing issue #1274), TUI test failures and five explicit TUI all-target lint errors. Earlier counts are historical evidence, not fresh success claims. Rerun crate tests, strict all-target Clippy, fmt, integration, then native guardrails. Do not conflate the root-package Clippy gate with workspace-wide test-target lint coverage.
4. **Integrate narrow topics first.** For each candidate in the tables, inspect `git log --left-right --cherry-pick`, exact changed paths and any existing PR. Run target-specific tests on the proposed integration result, not only on the topic tip. Merge only reviewed, green topics into an operator-approved clean integration context. Never switch this dirty checkout or modify another owner's worktree. If a new isolated checkout is needed, explicitly establish its ownership and authority before use.
5. **Update the fork non-destructively.** After combined gates pass, fetch fork again, confirm the target is unchanged, and use an explicit non-force refspec to the agreed fork integration branch. Reconcile fork master only after reviewing its four unique commits. Do not turn a blocked source checkpoint into fork master or a release. The current request authorizes fork integration, not deletion, force-update, daemon promotion or upstream publication.
6. **Prepare focused upstream proposals, do not submit tonight.** Every PR requires an existing upstream issue per CONTRIBUTING.md. Check all-author issue/PR duplicates immediately before any later proposal. Today's read-only duplicate inventory queried the operator's submissions only, so it is not an exhaustive duplicate search. Use minimal patches against the selected fresh upstream base, red/green reproduction, compatibility and rollback notes. Large harness architecture should start with an architecture issue, not a giant bundled PR.
7. **Retire branches last.** Ancestry containment is enough to avoid duplicate merging, not enough to delete recovery refs. For cherry-picked/rebased topics verify patch equivalence and resulting behavior. Archive/delete local or remote branches only with separate deletion approval, after upstream/fork state is recorded and linked worktree ownership is clear.

## Proposed upstream packages (planning only)

| Package | Proposed issue / PR content | Required evidence and dependencies |
| --- | --- | --- |
| Existing wake-mode fingerprint | Continue existing PR #1293 linked to issue #1292, not a duplicate | Read latest upstream review, revalidate on selected base, no edits/submission tonight |
| Existing macOS terminal detection lint | Continue existing PR #1295 linked to issue #1294 | Check merge/review state before integrating local branch |
| CI gate integrity and macOS coverage | Separate fail-closed compiler check, warning gate tests, shellcheck and platform validation | Coordinate with open #1177; demonstrate deliberate compiler error is not green; avoid unrelated size baseline changes |
| Terminal focus-report loop | Reconcile focus-report-loop, focus-runtime-safe and installed ancestry before a single proposal | Reproduction with TUI frames/runtime loop, test ownership, no duplicate overlapping fixes |
| Session storage/cache/accounting | Separate journal scaling, storage status, bounded git cache and compaction accounting | Existing issues #1150/#1151/#1154 and any current duplicates; measured performance and correctness evidence, not blanket performance claims |
| Provider/auth and security | Separate route preservation, missing-provider failure, sandboxed Keychain access, project MCP trust | Independent security review, no live credentials; coordinate with open #1176 and current upstream state |
| Task-DAG receipts / frozen attempts | Architecture issue describing native acceptance contract and its limitations | Provenance and artifact freshness remain unproven. Keep route admission, axes, deadline/budget and receipt policy individually reviewable |
| Pi no-tools terminal outcomes | Small follow-up to adapter if that adapter is accepted upstream | Existing 8 failing regressions -> 16 passing; distinguish intermediate retry errors from terminal outcomes. No tool-enabled Pi, sandbox or telemetry expansion |
| Failed ModelCall Verify acceptance | Small follow-up depending on receipt types and DAG acceptance seam | Same-attempt exits 1/124/130: 93/3 -> 96/0; unchanged gate state; success controls; extracted test file within existing budget |
| Mechanical extraction | Separate, ownership-reviewed code movement only | Byte/token comparison, identical named-test inventory, integration and ratchets. Do not hide functional changes or baseline increases inside movement |

## Per-branch resolution index

All 49 local branches and 99 non-symbolic fork branches are listed. `S/F/U` means tip is an ancestor of source snapshot / fork master / upstream master. This is an ancestry classification only, not a semantic review. `fork-only/topic-only` counts show divergence relative to fork master. Remote-only branches may be inherited historical topics, not operator-authored work, and must not be bulk-merged. Linked-worktree counts are read-only snapshots.

### Local branches

| Branch | Tip | S/F/U | Fork/topic divergence | Disposition |
| --- | --- | --- | --- | --- |
| `backup/kv-cache-pre-rebase-v0.81.7` | `0d008c720d01c3676877246e953384365c611005` | no/no/no | 174/59 | Retain recovery ref; never bulk-merge. Delete only with separate approval after patch reconciliation. |
| `backup/kv-cache-pre-rebase-v0.83.0` | `1a55fafeb6310f6c56b0a75d54a7c8a6c8627140` | no/no/no | 164/66 | Retain recovery ref; never bulk-merge. Delete only with separate approval after patch reconciliation. |
| `backup/kv-cache-telemetry-single-pass-20260904` | `4560d117b824d3bc7c13b75729b55cd644e37a22` | no/no/no | 198/34 | Retain recovery ref; never bulk-merge. Delete only with separate approval after patch reconciliation. |
| `chore/macos-warning-clean` | `2e09d209762640e0a395132736b3fad270d7a9f2` | no/no/no | 174/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `deps/agentgrep-v0.1.7` | `35a3ad6e373f1722effc9bf10441e17e402a128b` | no/no/no | 174/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `deps/resvg-usvg-align` | `25526f21b8673cc7398129fecaba598680637c59` | no/no/no | 50/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `feat/storage-status` | `5ca058ebf4e919b8ad5899d6a2b1dcebc59a325c` | no/no/no | 174/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/anthropic-fable-history` | `0d16b4aa2c43541b88e08229ac3244eeb0a116f0` | no/no/no | 174/5 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/ci-gate-integrity` | `85f04d98e957cb4698b291d0c09f8cb5184449ee` | no/no/no | 174/36 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/compaction-token-accounting` | `e79659889eb852715bf14c2c1774d200c6c42512` | no/no/no | 174/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/macos-ctrl5-prompt-rank` | `7ad6bab90df7ebc49920f81460dbb13eb0d5bae4` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/macos-stdin-false-positive` | `04d8bb30d1795143ab7c9ba7a00725b431e9fbdc` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/menubar-dark-streaming-icon` | `6b9a340003d48701117f1dedafad90b46add3024` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/persist-session-with-title` | `89c13f6172bb2db7b68da800f51fc5b65f325bb4` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/prompt-overlay-home-dedupe` | `c7e5d46d916aa5a810b59c5ca66127f0777fdde7` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/provider-cli-routing` | `3481e810ae06808975f854e554954da969b563ee` | no/no/no | 174/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/sandboxed-home-keychain` | `91b492629a4d8f8e9b4cd8f2bf06159decad0b75` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/test-ambient-queue-isolation` | `cfb8a7504f60f42c26d259df2d9aabbfd4b1513a` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/test-git-probe-isolation` | `01d60e5aa178e0ae9ffb40097c5f295133e26ec4` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/tui-suite-deadlock` | `bff4f2d946521067646c772571295fb098099d7f` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/adopted-output-artifact` | `cba16813a2cf2381672376e1ba67f74d2d754f65` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/api-translation-lints` | `74e7a4be54ae1db736bbd0e32ae1e6f59b475044` | yes/yes/yes | 4/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `jcode/auth-preserve-billing-route` | `11be474fe8870b65e7360c9aba8e621219949906` | no/no/no | 17/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/ci-env-dedup` | `091618754399f1ef6150c3ae9e6d4449346ae911` | no/no/no | 51/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/ci-format-baseline` | `0f216f754521e74df99052ae780c67ffbc31597d` | yes/no/no | 4/50 | OWNERSHIP HOLD: dirty linked worktree. Contained in source integration lineage. Do not remerge; publish only after dirty-work and acceptance gates close. |
| `jcode/dev-cargo-cwd-guard` | `a3995a0609f2bc625b81df0036fb2c386ca95942` | no/no/no | 50/30 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fix-fresh-improve-bootstrap` | `f7daf185cd163b3289e8c4b39e4b8304ad073c53` | no/no/no | 199/136 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fix-gemini-individual-oauth-status` | `b3c8cb5817a374c3216ca702474aa8ae1ba27776` | no/no/no | 200/4 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fix-macos-terminal-tail-match` | `5d501a7cbfeff68224e89bc138973a36721b302d` | no/no/no | 1/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. Existing upstream PR #1295 (OPEN); do not duplicate. |
| `jcode/fix-wake-mode-fingerprint` | `510f7c3ffdab972d8b92e8a0b3c36a0081d03849` | no/no/no | 1/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. Existing upstream PR #1293 (OPEN); do not duplicate. |
| `jcode/focus-report-loop` | `108caabd22034e78a5e221fc3a3901118f53de3f` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/focus-runtime-safe` | `f25eb9d11fedad0ee3d0d7dbf830929d39cf85c6` | no/no/no | 72/73 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fork-ci-secretless` | `32961d96534f4c23e6445b74ad8d4c1640a9bab5` | no/no/no | 2/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/macos-stdin-validation` | `74e7a4be54ae1db736bbd0e32ae1e6f59b475044` | yes/yes/yes | 4/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `jcode/missing-provider-failover` | `8ff08c9c21b321fa11461dbe53971436522ae610` | no/no/no | 5/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/notification-child-reaping` | `535a479d144bb2a172b2033eb0ea2e47e02b33b0` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/plugin-manifest-discovery` | `ba5201141696938f30917d8f204ca2de83395d33` | no/no/no | 49/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/route-receipts` | `13d3b3cafddad0b64f49bf6be0690d73f976ffb8` | no/no/no | 51/21 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/scratch-runtime-socket` | `a8bdfbe55a85c5fbf2c2fa75224e161f521741d8` | no/no/no | 5/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/token-churn-accounting` | `cd299a935ffba77b33b093a0d1e810e3b10936f0` | no/no/no | 5/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/workflow-shellcheck` | `98cee29c7ff5b3c03760fd2126951561eddd0a21` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `master` | `74e7a4be54ae1db736bbd0e32ae1e6f59b475044` | yes/yes/yes | 4/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `perf/bound-git-state-cache` | `6691edf820ad1ca1f5f46ecfdd6344ef3919b02f` | no/no/no | 198/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `perf/cache-and-journal` | `12dc573b93029b0bde3837c44015f663cd9b5267` | no/no/no | 174/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `perf/cache-journal-storage` | `f7c4630d1d4be74c5a4b1b09d2ef057239c87f7e` | no/no/no | 174/4 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `perf/kv-cache-telemetry-single-pass` | `669a1df51199f3d7994ba29a5a4493aca8258a17` | no/no/no | 72/91 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `perf/memory-off-agentgrep-v017` | `2215f0aa47b4e372edfc9d0a96e7a39b62e67c3d` | no/no/no | 198/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `security/harden-network-and-approval-boundaries` | `64e764f67dc2a22293d1df3e6e7fac50b6742113` | no/no/no | 174/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `security/project-mcp-trust` | `07fc5a0abbc955c5bf0edea33b2e1f98476febc7` | no/no/no | 174/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |

### Fork branches

| Branch | Tip | S/F/U | Fork/topic divergence | Disposition |
| --- | --- | --- | --- | --- |
| `add-fpt-ai-marketplace-provider` | `271546419d2139a431841e27ded53a8701a2dad8` | no/no/no | 4355/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `agent/fix-739-742` | `f044816bf624a913fc66899bf3bc8de7fdfd380b` | yes/yes/yes | 904/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/issue-699-ctrl-d` | `02f4afc00edb608ac6629538d03c7f487b921380` | yes/yes/yes | 953/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/jcode-triage-20260814-2203` | `3309ad37bbdf2a3774a7173512e54d26cc087448` | yes/yes/yes | 442/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/release-v0.71.1` | `431ee121915e595854cf41e6331c593e06c33619` | no/no/no | 664/21 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `agent/sdk-release-followup` | `2f534cf14ddff8a8c9593d29f49126e262b446ce` | no/no/no | 665/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `agent/triage-2026-08-02-clean` | `30de2a2a1bf8dfa2590de5f15c6bbadeeb789ccd` | yes/yes/yes | 927/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-2026-08-06` | `09d328b1c636d5fd706bf23050eb795c5270f47a` | yes/yes/yes | 621/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-2026-08-07` | `9445e7211f974010c17003a5eabfbf562884d8da` | no/no/no | 613/12 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `agent/triage-2026-08-07-pr` | `47e8819ce8f2c24e59d52f6e1289c9813851ebc3` | no/no/no | 666/11 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `agent/triage-2026-08-14` | `5ffba9482ec144723f514a2497fb6fea81759a5d` | yes/yes/yes | 440/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-2026-08-24` | `3f199fa0f156a0177252e19a7df2e62ad513022f` | yes/yes/yes | 282/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-20260816` | `d790cf279e3f2eb034a977e9fec9b96f2443d3f1` | yes/yes/yes | 375/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-20260818` | `3d993cc74a9e16091c873380f8b37c8662461aa2` | yes/yes/yes | 322/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-fixes-20260728` | `111ee842de012511a481064e40964c28bfbb2309` | yes/yes/yes | 1207/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-open-issues-20260809` | `44ffa55281fad71c02be984c0674d92412210452` | yes/yes/yes | 652/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-safe-fixes-20260809` | `7522663b10ffc3a7b7606d7740ae2dc1a84580cd` | yes/yes/yes | 573/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-safe-fixes-20260813` | `5c8b207cb06627288399d221996a3d75d95bdeb1` | no/no/no | 480/7 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `agent/triage-safe-fixes-20260827` | `e24dd976cc54745ba30797566fb1820a7221d448` | yes/yes/yes | 222/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `agent/triage-verification-tests` | `35e692bc8758d5c21292b097b27a1f456728d2cc` | yes/yes/yes | 919/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `arch/app-decomp` | `7634a9ed8941ce7e41e75f997a05b00ce3ef03cf` | no/no/no | 3283/5 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `arch/integration` | `46929ad081b2b749c1dcbe6155cb42e7986774ad` | no/no/no | 3272/11 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `arch/server-svc` | `8bb66ae13b14cf7b89e16ddd9130d83e586d54dd` | no/no/no | 3283/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `backup/kv-cache-telemetry-single-pass-20260904` | `4560d117b824d3bc7c13b75729b55cd644e37a22` | no/no/no | 198/34 | Retain recovery ref; never bulk-merge. Delete only with separate approval after patch reconciliation. |
| `chick/compacted-history-visible-window` | `c37dccf34dc4eadf29e419b14261cf4d13928500` | no/no/no | 3937/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `chore/macos-warning-clean` | `2e09d209762640e0a395132736b3fad270d7a9f2` | no/no/no | 174/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `deps/agentgrep-v0.1.7` | `35a3ad6e373f1722effc9bf10441e17e402a128b` | no/no/no | 174/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `dioxus-gui` | `7abad06cf0803654bec9bb18fbe60824d00ccf19` | no/no/no | 7293/364 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `dioxus-gui-local` | `e914f97abac50a57180fe254b806c9f89d099e0f` | no/no/no | 7293/382 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `docs/release-base-clarity` | `f1e9af276addc86e3f71516da877b94fcf2ba97b` | no/no/no | 5277/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `feat/computer-observability-348` | `941b44b28190698023e2c0132dca0be5b5a265a8` | no/no/no | 3082/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `feat/issue-664-auto-poke-config` | `8acc3081b91f80409d4a6ec1b15a9283cf3cde43` | no/no/no | 1195/9 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `feat/storage-status` | `5ca058ebf4e919b8ad5899d6a2b1dcebc59a325c` | no/no/no | 174/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `feat/windows-setup-copilot-key` | `526af67e0fff6809de5b272df6d83b1422e76790` | no/no/no | 1851/32 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix-browser-setup-201` | `f5d3630958b0ce04ca7a1695c761b556c4665ed2` | no/no/no | 4151/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix-set-route-model-alias` | `1252aaf2b871643665a76295de75e4ae256161b6` | no/no/no | 3000/11 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/acp-mcpservers-tolerated` | `0517f01f2c665cc577c501ac433dd0e359530628` | yes/yes/yes | 526/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/anthropic-fable-history` | `0d16b4aa2c43541b88e08229ac3244eeb0a116f0` | no/no/no | 174/5 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/compaction-token-accounting` | `e79659889eb852715bf14c2c1774d200c6c42512` | no/no/no | 174/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/computer-tool-schema-and-element-at` | `1677d567af831bec5cfb30c66a5cad7ca9975135` | no/no/no | 3076/8 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/herdr-client-hooks` | `3c57514745bc27f8695dca1c43e829ad32253390` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/installer-path-idempotency` | `c111c10e3aab61048d21a0905a31a5a7707035ad` | no/no/no | 1256/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/issue-543-mcp-format` | `a52ead259cd48c69c0e78d6a9e2974a54efd5894` | no/no/no | 1627/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/issue-657-tract-023` | `49d9dfd620ac96b882238005511829fbd7c1b53d` | yes/yes/yes | 1101/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-662-ci-red` | `c3455d9512e953c8d73dd63022183e6f159e18bb` | yes/yes/yes | 1102/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-754-gemini-mcp-notification` | `3625a023044b354ccceaed061d1d2ccd2f5299b1` | yes/yes/yes | 770/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-759-client-hooks` | `c943b04e8edaca0b3b7946a4bf18a070daa5a16a` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-762-celeris-config` | `d7feb6b54d9aeb5b2145d36bfb5b25bbfd13d2af` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-763-power-inhibitor` | `dc59f652190ad85a83d06c83fb9d66e6249ed874` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-767-favorite-cycle` | `0251e0803a8cacca8c34831f12cf1dde091fdbaf` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-768-menubar-ci` | `1e80a4033216064b7eebda1495251d9976a603ea` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/issue-779-acp-resume-subscribe` | `8d46f643c68e21d70ef5d57c6cf547d4ee528eea` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/latest-remote-session-bugs` | `fefae76c2f9d14c6fd7923b3adefe8ecec4bd3fa` | yes/yes/yes | 525/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/macos-ctrl5-prompt-rank` | `7ad6bab90df7ebc49920f81460dbb13eb0d5bae4` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/macos-stdin-false-positive` | `04d8bb30d1795143ab7c9ba7a00725b431e9fbdc` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/menubar-dark-streaming-icon` | `6b9a340003d48701117f1dedafad90b46add3024` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/menubar-root-sessions` | `aac9555e88b8a31db3e9caec1bed0d26b4572454` | no/no/no | 1751/5 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/openai-quota-window-dedup` | `30ec7b33719889d61de23aab15c481fd155a1eb9` | yes/yes/yes | 512/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/persist-session-with-title` | `89c13f6172bb2db7b68da800f51fc5b65f325bb4` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/pinned-todos-config-cache-isolation` | `446601bc3c9337aa67d3e638814e7ee1c05a9253` | yes/yes/yes | 525/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/prompt-overlay-home-dedupe` | `c7e5d46d916aa5a810b59c5ca66127f0777fdde7` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/provider-cli-routing` | `3481e810ae06808975f854e554954da969b563ee` | no/no/no | 174/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/report-alternate-keyboard-keys` | `a1d7db4542515030447ba35951665af26aecd442` | yes/yes/yes | 521/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/sandboxed-home-keychain` | `91b492629a4d8f8e9b4cd8f2bf06159decad0b75` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/skill-invocation-multi-word-619` | `477cb86c6acc6ab390381d18ade4176a34ed39d0` | no/no/no | 1256/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/soft-interrupt-images` | `9ea612f9db91d10070847dbe9dbc39f7c1ccc68b` | no/no/no | 1256/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/stream-first-byte-timeout` | `2b0e28b3591543a8a8f145c10443ee867bfc948b` | no/no/no | 1256/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/stream-read-error-retry` | `437c6610a8e7e3d532f158343461cd0ed51bfd7a` | yes/yes/yes | 525/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/test-ambient-queue-isolation` | `cfb8a7504f60f42c26d259df2d9aabbfd4b1513a` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/test-git-probe-isolation` | `01d60e5aa178e0ae9ffb40097c5f295133e26ec4` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/transport-retry-classification` | `664c04142c1909d6d67070442f0537a62a4f4554` | no/no/no | 3090/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/tui-suite-deadlock` | `bff4f2d946521067646c772571295fb098099d7f` | no/no/no | 199/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `fix/unique-fallback-tool-call-ids` | `e0aea418ef45d801f241ef29aba936179d14e585` | yes/yes/yes | 526/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `fix/windows-global-jcode-path` | `0bb4d1084afca9afbc00bc9ec48ff0f3ff3474ad` | no/no/no | 1858/4 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `ios/mobile-real-nav` | `673ff4f3e25974f6fd7a9662ef8aebc1d698f8f4` | no/no/no | 664/22 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `ios/ux-production` | `887194bdf6be366876905cd762476885c8cbbc6e` | no/no/no | 1621/9 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/adopted-output-artifact` | `cba16813a2cf2381672376e1ba67f74d2d754f65` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/ci-env-dedup` | `091618754399f1ef6150c3ae9e6d4449346ae911` | no/no/no | 51/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/configurable-colors` | `53c5c0a2f9a1b3557318b8a841b63314b0b8b554` | no/no/no | 1195/24 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/dev-cargo-cwd-guard` | `d3f3c6c45f45ed2b531300ed7ba8c95fbf63c128` | no/no/no | 50/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fix-gemini-individual-oauth-status` | `c8b2ec774985163ebf06d7c7c8fd20288a8676e3` | no/no/no | 200/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fix-macos-terminal-tail-match` | `5d501a7cbfeff68224e89bc138973a36721b302d` | no/no/no | 1/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. Existing upstream PR #1295 (OPEN); do not duplicate. |
| `jcode/fix-wake-mode-fingerprint` | `510f7c3ffdab972d8b92e8a0b3c36a0081d03849` | no/no/no | 1/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. Existing upstream PR #1293 (OPEN); do not duplicate. |
| `jcode/focus-report-loop` | `108caabd22034e78a5e221fc3a3901118f53de3f` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/fork-ci-secretless` | `32961d96534f4c23e6445b74ad8d4c1640a9bab5` | no/no/no | 2/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/notification-child-reaping` | `535a479d144bb2a172b2033eb0ea2e47e02b33b0` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `jcode/workflow-shellcheck` | `98cee29c7ff5b3c03760fd2126951561eddd0a21` | no/no/no | 51/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `logical-commits-20260322` | `2e89a6b457a6bc0aa97272f52be9759c20ed2866` | no/no/no | 5967/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `master` | `e09acaa7a8828bcd46d9a5ab7c3434112e04e713` | no/yes/no | 0/0 | Already contained in fork master. Compare against rewritten upstream for patch equivalence before any PR. |
| `perf/cache-and-journal` | `12dc573b93029b0bde3837c44015f663cd9b5267` | no/no/no | 174/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `perf/kv-cache-telemetry-single-pass` | `a0f77ccd43b8fd8f852196b9ca98df54a28588e9` | no/no/no | 174/33 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `perf/memory-off-agentgrep-v017` | `2215f0aa47b4e372edfc9d0a96e7a39b62e67c3d` | no/no/no | 198/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `release-workflow-fixes` | `6dd45bf5d01a323ce21fa48e3fef8e76cdba3538` | no/no/no | 5922/2 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `security/harden-network-and-approval-boundaries` | `64e764f67dc2a22293d1df3e6e7fac50b6742113` | no/no/no | 174/6 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `security/project-mcp-trust` | `07fc5a0abbc955c5bf0edea33b2e1f98476febc7` | no/no/no | 174/1 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |
| `test/preserve-iteration-maturity-fixtures` | `9fc15d499de7d85b26b7d365b20d2204c5ebc74f` | yes/yes/yes | 803/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `triage/issues-2026-07-29` | `fe77ce9c7ec1bc767b4f29489918026805742cca` | yes/yes/yes | 1187/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `triage/open-issues-20260812` | `6fa94a882486866b172d3035230a486d97896eb9` | yes/yes/yes | 501/0 | Already contained upstream by ancestry. Verify current behavior and PR state; no duplicate PR. Archive only after approval. |
| `windows-lifecycle-e2e` | `63c77688f1edebdcb949873bb5434855ede4e7d6` | no/no/no | 4756/3 | Unintegrated candidate. Review unique commits and patch equivalence against source/fork/upstream, run owned tests, then integrate one topic at a time. |

## Linked worktrees

| Branch | Dirty status entries | Disposition |
| --- | --- | --- |
| `refs/heads/jcode/ci-format-baseline` | 281 | Read only; ownership hold |
| `detached 091618754399f1ef6150c3ae9e6d4449346ae911` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/focus-report-loop` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/focus-runtime-safe` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/fork-ci-secretless` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/fix-macos-terminal-tail-match` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/notification-child-reaping` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/plugin-manifest-discovery` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/route-receipts` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/fix-wake-mode-fingerprint` | 0 | Read only; clean snapshot, not integration authorization |
| `refs/heads/jcode/workflow-shellcheck` | 0 | Read only; clean snapshot, not integration authorization |

## Upstream-only tracking inventory

All fetched upstream branch refs are listed for completeness. Disposition for each: read-only upstream context, not a branch to merge indiscriminately or delete. Compare only when its commits are relevant to a selected topic.

| Upstream branch | Tip |
| --- | --- |
| `acceptance/greptile-order` | `d8b0f8e3f1d8538252bac64e9ea63cc3171e1af8` |
| `acceptance/jev-labeler-live` | `9601870602155b29a7769a185eb864d0043d9905` |
| `add-fpt-ai-marketplace-provider` | `271546419d2139a431841e27ded53a8701a2dad8` |
| `agent/fix-739-742` | `f044816bf624a913fc66899bf3bc8de7fdfd380b` |
| `agent/issue-699-ctrl-d` | `02f4afc00edb608ac6629538d03c7f487b921380` |
| `agent/jcode-triage-20260814-2203` | `3309ad37bbdf2a3774a7173512e54d26cc087448` |
| `agent/release-v0.71.1` | `431ee121915e595854cf41e6331c593e06c33619` |
| `agent/sdk-release-followup` | `2f534cf14ddff8a8c9593d29f49126e262b446ce` |
| `agent/triage-2026-08-02-clean` | `30de2a2a1bf8dfa2590de5f15c6bbadeeb789ccd` |
| `agent/triage-2026-08-06` | `09d328b1c636d5fd706bf23050eb795c5270f47a` |
| `agent/triage-2026-08-07` | `9445e7211f974010c17003a5eabfbf562884d8da` |
| `agent/triage-2026-08-07-pr` | `47e8819ce8f2c24e59d52f6e1289c9813851ebc3` |
| `agent/triage-2026-08-14` | `5ffba9482ec144723f514a2497fb6fea81759a5d` |
| `agent/triage-2026-08-24` | `3f199fa0f156a0177252e19a7df2e62ad513022f` |
| `agent/triage-20260816` | `d790cf279e3f2eb034a977e9fec9b96f2443d3f1` |
| `agent/triage-20260818` | `3d993cc74a9e16091c873380f8b37c8662461aa2` |
| `agent/triage-fixes-20260728` | `111ee842de012511a481064e40964c28bfbb2309` |
| `agent/triage-open-issues-20260809` | `44ffa55281fad71c02be984c0674d92412210452` |
| `agent/triage-safe-fixes-20260809` | `7522663b10ffc3a7b7606d7740ae2dc1a84580cd` |
| `agent/triage-safe-fixes-20260813` | `5c8b207cb06627288399d221996a3d75d95bdeb1` |
| `agent/triage-safe-fixes-20260827` | `e24dd976cc54745ba30797566fb1820a7221d448` |
| `agent/triage-verification-tests` | `35e692bc8758d5c21292b097b27a1f456728d2cc` |
| `arch/app-decomp` | `7634a9ed8941ce7e41e75f997a05b00ce3ef03cf` |
| `arch/integration` | `46929ad081b2b749c1dcbe6155cb42e7986774ad` |
| `arch/server-svc` | `8bb66ae13b14cf7b89e16ddd9130d83e586d54dd` |
| `chick/compacted-history-visible-window` | `c37dccf34dc4eadf29e419b14261cf4d13928500` |
| `ci/greptile-before-jev` | `daaf1f04bf19d9112698def2ee4bb17cf08f62c1` |
| `ci/jev-live-base-evidence` | `d03d6502764397ad010106d7275780f2a69f45f4` |
| `ci/jev-semantic-pr-labels` | `88c6fe031a443956a3ff76794d804d0c42432373` |
| `compact-pinned-todos` | `7d08697437871e08abe38ca774dd47bde3b3cacf` |
| `dioxus-gui` | `7abad06cf0803654bec9bb18fbe60824d00ccf19` |
| `dioxus-gui-local` | `e914f97abac50a57180fe254b806c9f89d099e0f` |
| `docs/release-base-clarity` | `f1e9af276addc86e3f71516da877b94fcf2ba97b` |
| `docs/welcome-all-contributors-20260913` | `ea44b6d03f75377e7d3fd9d27ab104d87e6f285e` |
| `feat/computer-observability-348` | `941b44b28190698023e2c0132dca0be5b5a265a8` |
| `feat/desktop-account-login` | `7dd973912a7acb8792c7d6caf9254270b0dd1b02` |
| `feat/issue-664-auto-poke-config` | `8acc3081b91f80409d4a6ec1b15a9283cf3cde43` |
| `feat/jev-browser-handoff` | `582ec115fd9469a14bf55279a1f33284309480c9` |
| `feat/native-nari-streaming` | `8d5953314f4e4482f8b1cbd0e1384126d3e7b797` |
| `feat/sdk-local-worktrees` | `fcbcf7fd1ceb61cfd7aae8038e15180c640d6e59` |
| `feat/sdk-local-worktrees-api` | `0c6056425bf36fd7e1d9584cfd12fc3d4a801041` |
| `feat/session-edit-stats` | `93f9f4b7b4ca79448090876d12a7947b30cd45b0` |
| `feat/shared-subscription-voice` | `2d67b49cb24d0efc1b8566b3e1901196d6f6fd64` |
| `feat/swarm-root-effort` | `b17fdf85903584f43ac34eed18c852622659dbaf` |
| `feat/windows-setup-copilot-key` | `526af67e0fff6809de5b272df6d83b1422e76790` |
| `fix-browser-setup-201` | `f5d3630958b0ce04ca7a1695c761b556c4665ed2` |
| `fix-set-route-model-alias` | `1252aaf2b871643665a76295de75e4ae256161b6` |
| `fix/1179-relative-doc-links` | `0fb8827f223a495ba13201111978e31a419a76b3` |
| `fix/912-source-update-status` | `dd717ce25ea399eecbca790c05c93678dfa83f9e` |
| `fix/acp-mcpservers-tolerated` | `0517f01f2c665cc577c501ac433dd0e359530628` |
| `fix/api-attach-preserve-cwd` | `c80d69097a458dbe03228c67dc85ebb3ed23d326` |
| `fix/authoritative-edit-diffs` | `5541e4584b8b3a0271d5fc1cb2b0f02d7acd5598` |
| `fix/computer-tool-schema-and-element-at` | `1677d567af831bec5cfb30c66a5cad7ca9975135` |
| `fix/desktop-edit-stats-compat` | `a19b1e9eeee7a89940d6d4164d816ae24cb91c52` |
| `fix/desktop-kitty-detection` | `66acfc57ef660c43db4ab7508ea3b3b224001f21` |
| `fix/desktop-native-login-sdk` | `950e231445af43bbfeccfdc99f5f97256c7dc4b2` |
| `fix/desktop-observer-turn-completion` | `fbed5b91b28d43268e3e50e007a0e97a915e5b77` |
| `fix/early-tool-streaming` | `639459e33a066bca3ca5e0f95da08dc039afa92a` |
| `fix/harness-session-recovery` | `641d6bcff6c03cac764f8777f86375fe8d95fea6` |
| `fix/harness-side-panel` | `10c3391931ea9f600bc2fb8e2b3f17603b85f977` |
| `fix/herdr-client-hooks` | `3c57514745bc27f8695dca1c43e829ad32253390` |
| `fix/image-transcript-position` | `200b11ec5b1af5e25ae8aa6b691b6dcf2ebc0c9c` |
| `fix/installer-path-idempotency` | `c111c10e3aab61048d21a0905a31a5a7707035ad` |
| `fix/issue-1240-prompt-dedup` | `cb2178be67f666c3e65e1907a871a8b3b687ac15` |
| `fix/issue-543-mcp-format` | `a52ead259cd48c69c0e78d6a9e2974a54efd5894` |
| `fix/issue-657-tract-023` | `49d9dfd620ac96b882238005511829fbd7c1b53d` |
| `fix/issue-662-ci-red` | `c3455d9512e953c8d73dd63022183e6f159e18bb` |
| `fix/issue-754-gemini-mcp-notification` | `3625a023044b354ccceaed061d1d2ccd2f5299b1` |
| `fix/issue-759-client-hooks` | `c943b04e8edaca0b3b7946a4bf18a070daa5a16a` |
| `fix/issue-762-celeris-config` | `d7feb6b54d9aeb5b2145d36bfb5b25bbfd13d2af` |
| `fix/issue-763-power-inhibitor` | `dc59f652190ad85a83d06c83fb9d66e6249ed874` |
| `fix/issue-767-favorite-cycle` | `0251e0803a8cacca8c34831f12cf1dde091fdbaf` |
| `fix/issue-768-menubar-ci` | `1e80a4033216064b7eebda1495251d9976a603ea` |
| `fix/issue-779-acp-resume-subscribe` | `8d46f643c68e21d70ef5d57c6cf547d4ee528eea` |
| `fix/issue-triage-20260907` | `6ac6b2ed2f9b2fa75bbdee12a23263f89c7c4263` |
| `fix/issue-triage-20260910-pr` | `0e84b661a1c437b2376398a9be4409b5def6664b` |
| `fix/issue-triage-20260912` | `3b94a54c963594dcc6b31cec28526368a5dcbf26` |
| `fix/issue-triage-20260912-pr` | `e2b9d7a1aeae7f0e951af6328ce1e58c13d27cff` |
| `fix/issue-triage-20260915` | `c515991f28e8be677385c6632489c3927ebc3a4d` |
| `fix/latest-remote-session-bugs` | `fefae76c2f9d14c6fd7923b3adefe8ecec4bd3fa` |
| `fix/light-terminal-contrast` | `6c79900de78afe7be0cf4b2eec55b72bc218147c` |
| `fix/menubar-root-sessions` | `aac9555e88b8a31db3e9caec1bed0d26b4572454` |
| `fix/native-reasoning-markdown` | `f6764ddb6e6f59808fc100e3fa037753be35b854` |
| `fix/openai-early-tool-streaming-639459e33` | `639459e33a066bca3ca5e0f95da08dc039afa92a` |
| `fix/openai-quota-window-dedup` | `30ec7b33719889d61de23aab15c481fd155a1eb9` |
| `fix/pinned-todos-config-cache-isolation` | `446601bc3c9337aa67d3e638814e7ee1c05a9253` |
| `fix/report-alternate-keyboard-keys` | `a1d7db4542515030447ba35951665af26aecd442` |
| `fix/sdk-cache-creation-usage-6ee9ce5f1` | `6ee9ce5f1affefac3e84adc7ce7ab6a90fb9ca9c` |
| `fix/skill-invocation-multi-word-619` | `477cb86c6acc6ab390381d18ade4176a34ed39d0` |
| `fix/soft-interrupt-images` | `9ea612f9db91d10070847dbe9dbc39f7c1ccc68b` |
| `fix/stream-first-byte-timeout` | `2b0e28b3591543a8a8f145c10443ee867bfc948b` |
| `fix/stream-read-error-retry` | `437c6610a8e7e3d532f158343461cd0ed51bfd7a` |
| `fix/transport-retry-classification` | `664c04142c1909d6d67070442f0537a62a4f4554` |
| `fix/unique-fallback-tool-call-ids` | `e0aea418ef45d801f241ef29aba936179d14e585` |
| `fix/windows-global-jcode-path` | `0bb4d1084afca9afbc00bc9ec48ff0f3ff3474ad` |
| `history-response-stats` | `827eee1c096c9ca06c1e49c977e7a1a6f0334ff5` |
| `ios/mobile-real-nav` | `673ff4f3e25974f6fd7a9662ef8aebc1d698f8f4` |
| `ios/ux-production` | `887194bdf6be366876905cd762476885c8cbbc6e` |
| `jcode/configurable-colors` | `53c5c0a2f9a1b3557318b8a841b63314b0b8b554` |
| `jcode/model-usage-metadata-20260912` | `31d14f86b565b0047dd08ae0d3aa2dd57cd343af` |
| `logical-commits-20260322` | `2e89a6b457a6bc0aa97272f52be9759c20ed2866` |
| `master` | `eebc4996fbb57e9534bb6a030a5ac232bf90681c` |
| `oauth-api-equivalent-usage-20260907` | `9e5522626b493d4fe0fe1b2640dfa35171dcb3c2` |
| `release-workflow-fixes` | `6dd45bf5d01a323ce21fa48e3fef8e76cdba3538` |
| `sdk/swarm-session-metadata` | `10cf5c74dbe16ccf604084ca438ae3d0048cc691` |
| `test/preserve-iteration-maturity-fixtures` | `9fc15d499de7d85b26b7d365b20d2204c5ebc74f` |
| `triage/issues-2026-07-29` | `fe77ce9c7ec1bc767b4f29489918026805742cca` |
| `triage/open-issues-20260812` | `6fa94a882486866b172d3035230a486d97896eb9` |
| `windows-lifecycle-e2e` | `63c77688f1edebdcb949873bb5434855ede4e7d6` |

## Existing operator-authored upstream issue inventory

Read-only snapshot, at most 100 results, returned 32. Closed does not necessarily mean a patch is merged. Do not recreate these without reading their resolution.

| Issue | State | Title |
| --- | --- | --- |
| #1294 | OPEN | macOS: terminal-launch Clippy fails on unnecessary tail return |
| #1292 | OPEN | fix(config): JCODE_WAKE_MODE is missing from the config-cache fingerprint |
| #1275 | CLOSED | macOS: copy-badge margin test assumes the non-macOS Alt-label width |
| #1274 | OPEN | Conifer static fallback lacks context limits for 25 models and fails catalog completeness test |
| #1204 | CLOSED | The exported `cargo` shim silently builds jcode when a Bash command cds into another Rust checkout |
| #1189 | CLOSED | `bg output` fails while a timeout-promoted command is still Running (output artifact created only at completion) |
| #1188 | CLOSED | A hand-written `[sponsors] enabled = false` is undone after one config save and reload |
| #1178 | CLOSED | Remote compaction notices can report the wrong before-token count and hide dropped messages |
| #1177 | OPEN | CI's zero-warning budget is Linux-only; 2,700+ core tests are compiled but never run |
| #1176 | OPEN | Project-local .mcp.json executes arbitrary commands with no trust prompt (fires even when the session has no provider) |
| #1154 | OPEN | Self-dev immutable build versions have no retention policy and grow storage indefinitely |
| #1153 | CLOSED | macOS handterm spawn tests compare /var and /private/var literally, then poison ENV_LOCK |
| #1152 | CLOSED | idle_live_agent drops its try_lock guard before spawn, allowing duplicate wake reservations |
| #1151 | OPEN | session_search count-only scoring cap permits large concurrent full-session deserialization |
| #1150 | OPEN | Fixed 512 KiB journal checkpoint cap causes snapshot rewrite amplification on long sessions |
| #1149 | OPEN | Native tool calls in one model response execute serially; OpenAI disables parallel_tool_calls |
| #1147 | OPEN | websearch remains unreliable when DDG and Bing HTML are blocked; fallback diagnostics hide attempts |
| #1146 | OPEN | macOS stdin detection reports every sleeping command as waiting for input |
| #1144 | CLOSED | Session::save silently skips the write for a session created with a title, so later lookups by id find nothing |
| #1142 | CLOSED | Test isolation: create_test_app deletes ambient state from another test's $JCODE_HOME |
| #1141 | OPEN | cargo test -p jcode-tui --lib deadlocks at the default thread count (ABBA between the render-state lock and the env lock) |
| #1133 | CLOSED | Test isolation: the background git probe leaks the host repository's live state into unrelated TUI frame assertions |
| #1132 | CLOSED | macOS: a sandboxed $JCODE_HOME still reads the real user's Keychain, changing onboarding and leaking real credential state into tests |
| #1131 | CLOSED | macOS: Ctrl+5 prompt-rank jump is dead; a legacy Ctrl+] alias consumes it before any handler runs |
| #1122 | OPEN | `provider-doctor` cannot diagnose a configured named profile, and its error message routes users in a circle |
| #1121 | OPEN | `jcode run` silently ignores JCODE_NAMED_PROVIDER_PROFILE and answers from the cloud default |
| #1117 | OPEN | restart restore can resume stale agent/tool work without a new operator prompt |
| #1115 | OPEN | First-run install telemetry is sent before the telemetry notice |
| #1114 | OPEN | Global telemetry opt-out does not suppress sponsor usage metering |
| #1113 | OPEN | Malformed config silently falls back to broad defaults |
| #1112 | OPEN | Auth login can change the provider of later fresh daemon sessions |
| #1110 | OPEN | Gemini OAuth succeeds but individual accounts fail at inference after Code Assist client retirement |

## Evidence location and limits

Local evidence is under `$JCODE_SCRATCH_DIR/night-closeout-20260919/`: ref/worktree inventory JSON, original dirty status and hashes, upstream read-only queries, exact reviewed Pi patch, fresh green and repository logs. Prior packet artifacts are in sibling dated directories. The initial compound fetch/inventory shell exited 1 because its final optional docs glob had no match; both fetches completed. No failed fetch was silently retried.

The closeout documentation itself does not make source acceptance green. No source build was promoted. No upstream proposal was submitted. No broad extraction was adopted or discarded. Future integration remains blocked on the explicitly listed ownership, history and gate conditions.

## Fork-master-only commits at closeout

Review these before choosing an integration target. The presence of these commits does not authorize overwriting the fork.

```text
e09acaa7a8828bcd46d9a5ab7c3434112e04e713 feat(auth): add reusable account-only desktop login
5cb7b3dad6029a6f868e36fd4c13cbd07d1fca50 docs: update weekly stars chart
0735c75317e644ecb440e0c3dddb7a6b3cd0d8bf docs: update weekly stars chart
5f33d6239b56b3d381d6b2c0e9b151eef0b54cbe docs: update weekly stars chart
```

## Fresh closeout verification

- `cargo test -p jcode-executor-pi --offline`: exit 0, log `night-closeout-20260919/green-test.log`.
- `cargo clippy -p jcode-executor-pi --all-targets --offline -- -D warnings`: exit 0, log `night-closeout-20260919/green-clippy.log`.
- `cargo fmt -p jcode-executor-pi -- --check`: exit 0, log `night-closeout-20260919/green-fmt.log`.
- `cargo check -q --offline`: exit 0, log `night-closeout-20260919/green-integration.log`.
- `bash scripts/check_guardrails.sh`: exit 1, log `night-closeout-20260919/repository-guardrails.log`.
- `python3 scripts/test_check_warning_budget.py`: exit 0, log `night-closeout-20260919/repository-warning-gate-tests.log`.
- `cargo test -p jcode-sdk --offline parity -- --nocapture`: exit 0, log `night-closeout-20260919/repository-sdk-parity.log`.
