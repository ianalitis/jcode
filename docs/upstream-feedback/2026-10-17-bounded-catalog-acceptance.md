# Bounded native swarm catalog: local acceptance, 2026-10-17

This is source acceptance evidence for the main coordinator's canonical handoff, not a second roadmap or a live activation receipt.

## Scope

Against accepted HEAD `d03603e52a1d770cfe136f7a8792bf6eb1c9f64f`, change only the native swarm tool input/schema/execution, its private catalog presentation module, and its tests:

- `crates/jcode-app-core/src/tool/communicate.rs`
- `crates/jcode-app-core/src/tool/communicate/model_catalog.rs`
- `crates/jcode-app-core/src/tool/communicate_tests.rs`

The operator authorized bounded shared-system improvements. Dotfiles dog initially dispatched coordination, then operator-appointed bird retained the single main shared-system coordinator and canonical handoff. Common Git lease `catalog-bat-20261017` covered these source paths and this dated receipt. No competing writer or Cargo build was admitted.

Native `list_models` gains optional provider/query filters and an action-local limit: default 25, valid 1..100. Family, credential route, compatible profile and canonical account identity remain distinct. Query is a literal ASCII-case-insensitive substring over model/provider/raw api_method, AND the provider filter. Unknown filters return no matches, not fallback or admission. Server order, duplicates, unavailable rows and complete auth/detail/cost facts are preserved.

Complete formatted catalog text, including headers/counts/footer, is bounded to 16 KiB UTF-8 bytes. Only whole route rows are emitted, stopping at the first non-fitting row. Counts and omission reason distinguish row limits, byte limits, an empty catalog, no filter matches, an oversized first row and header overflow. Existing spawn/summary behavior, request/response wire shape, server catalog production and usage metadata remain unchanged.

## Independent review and source identity

Astra/high designer dove traced native ownership before implementation. Astra/high builder buffalo completed its prescribed two iterations. Independent Astra/high reviewer mushroom found and then verified the exact-profile correction and concise schema descriptions. The later one-file Option pricing clarification received separate explicit captain admission and independent Astra/high review. Final verdict: **ACCEPT SOURCE and confirmed recorded validation**, with no blocking finding remaining.

Final SHA-256 values, verified by captain and reviewer from the actual release-sync root:

| File | SHA-256 |
| --- | --- |
| communicate.rs | `c699a22ec7fb09c683414d8bad44accd0c1d0138642d42275efce7b764f84eb7` |
| communicate/model_catalog.rs | `343754e7fd4b1c9a47e23b2695eb3c1bc66a6ab55e0f6530fcce05af65dadeb6` |
| communicate_tests.rs | `88742e26b51218822e0eb7ccb84c065943398707688068a0c3053735a9c45742` |

No verification script, detector, ratchet baseline, allowance, dependency, provider/auth configuration or usage budget changed. Complexity challenge: one cohesive private presentation module, existing provider identity helpers, explicit missing-price policy and focused tests suffice. No new controller, router, helper abstraction or global lint refactor was added.

## Final local validation

Captain task `090533ptl9` completed with exit 0 in 196.41 seconds.

- Native evidence: `/Users/ianalitis/.jcode/scratch/catalog-captain.wqBjix`.
- Gate evidence: `/Users/ianalitis/.jcode/scratch/release-sync-acceptance.ztXwiv`.
- Runner: `/Users/ianalitis/.jcode/scratch/catalog-captain-acceptance.sh`, then the unchanged existing 36-gate runner.
- Full app-core suite: **1,826 passed, 0 failed, 33 ignored, 0 filtered**; protocol roundtrip 1 passed; real deliberate panic/catch regression passed.
- **36 unique local gates all exit 0**, including unchanged swallowed-error ratchet, all-target/all-feature check and strict clippy. Provider 9 and E2E 60 passed, with 7 E2E ignored.

Inventory is 1,859 tests versus 1,847 before this packet: 12 private catalog tests plus 2 new tool tests replace 2 old formatter tests. All other names are unchanged and the 33 ignored tests are byte-identical. Tests cover 10k routes at25/100, exact credential/profile/account identities, punctuation collisions, literal AND filtering, invalid limits before request, default/null/blank behavior, whole-row multibyte byte limits, oversized rows/headers and absent/present pricing.

Final artifact is a cached matching worker build (`fresh=true`), not a fresh or cold compilation. Recorded host is aarch64-apple-darwin, features default/bedrock/pdf, opt1/debug0, assertions and overflow checks enabled. `__eh_frame` is `0x7dc0f0`, below 16 MiB, with unwind information retained. Source and executable hashes match the final receipt. Hooks were enabled and runtime homes isolated for the full native suite.

Compiler warning budget remains 0/0. SDK parity passes its 3 tests and still visibly reports the explicitly triaged TypeScript-only `close` diagnostic. That diagnostic was not hidden and is distinct from the prior resolved Apple linker warning.

## Failed attempts and STOP boundaries retained

1. Initial independent review found a real compatible-profile punctuation collision (`local-gpu` versus `local_gpu`). The first full suite also rejected the 3 new descriptions exceeding the existing 25-token cap: 1,822 passed, 1 failed, 33 ignored. Neither failure was suppressed.
2. Builder iteration two made compatible-profile matching terminal and shortened the descriptions under the unchanged cap. The next complete suite passed 1,825 tests, but only 35/36 gates passed: the per-file swallowed-error ratchet correctly rejected the moved `.unwrap_or_default()` cost formatter in a new production file. The two-iteration builder treatment closed with STOP.
3. A fresh Astra/high recovery worker closed with a zero-execution STOP after a read-only bash preflight was refused by the safety gate. No source mutation, build or test occurred in that treatment. No gate override was used.
4. Captain explicitly admitted a final fresh Astra/high one-file/one-iteration recovery using native file tools and literal command arguments. `estimated_reference_cost_micros()` returns `Option<u64>`, not a Result. Explicit Some/None expresses the missing-price policy without inventing a price or consuming an error. One exact absent-price row test was added, while the existing present-price positive control stayed unchanged. The unchanged ratchet and 12 catalog tests passed, then the complete fresh acceptance above passed.
5. An auxiliary captain hash check initially ran from the old primary checkout and therefore compared the wrong files. The corrected explicit-cd check passes all 3 source hashes and the executable hash. Both authoritative acceptance scripts had already used the correct checkout. This was not a changed candidate.

Earlier evidence remains retained: `catalog-captain.vocy5U`, `catalog-captain.nd7OSH`, `release-sync-acceptance.4SlqyJ`, and `catalog-bat-20261017-builder/iteration2/` under the Jcode scratch root. Full final source and raw receipts supersede failed receipts only for acceptance, not for the historical record.

## Limits and next gates

The 16 KiB guarantee covers formatted catalog text only. It does not bound generic deserialization/transport errors, transient heap allocations, daemon catalog allocation or wire transfer. No new successful `list_models` execute/transport integration test or active-daemon end-user workflow was validated. Synthetic formatter fixtures and full native project regressions must not be mislabeled as live activation.

No push, publication, installation, host/provider/auth changes, infrastructure apply, runtime reload or activation occurred. The primary source checkout remains at `921807b98d74f2fcd102570327547189e70dd8f3`. Cross-platform and hosted release validation remain unexecuted. This nested receipt is outside app-core's embedded README/direct-docs corpus and does not change embedded documentation contents.

After this catalog packet closes, the separately requested Pi P0 intake is read-only: verify source custody/current revision and propose the existing carrier/replay/task-generation read allowlist for independent high review. It grants no RED test, source writer, duplicate adapter, installation or activation authority. Broader modernization stays in the main coordinator's existing canonical roadmap and handoff.
