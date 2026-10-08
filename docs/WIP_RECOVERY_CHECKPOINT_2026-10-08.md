# Reviewed WIP recovery checkpoint, 2026-10-08

## Disposition: archival WIP, not accepted integration

At 22:45 UTC the operator explicitly chose a reviewed work-in-progress submission after the final failures and publication limits were explained. This is the separate archival disposition contemplated by the earlier portfolio receipt. It permits a normal signed checkpoint preserving the reviewed candidate and its existing merge parents, with normal hooks and no gate suppression. It does not declare the merge accepted, authorize a default-branch overwrite, or promote a runtime.

Original source HEAD: `d7848d7d3b2c03afbdade4deb031f8795bfafb3c`.
Pending upstream parent: `a61c38ee9dd945fe9f930fa09c7f3454a2ae4110`.
Latest upstream mirror: `21eb960a2fb9b9ffada350138f01ec1be5c67349`.
The four newer upstream commits are not silently claimed as merged into this checkpoint. The local mirror is current, and the published fork default has its own intentional CI history. No force push, reset, merge-parent manipulation, alternate-index workaround or commit-tree bypass is part of this submission.

## Reviewed work retained

The staged upstream integration and subsequent scoped repairs include provider policy/cache/collector extractions, Cursor malformed decoding and real h2 task cleanup, gateway consumption/registration persistence failure handling, recorder error reporting, prompt guidance fixture alignment, TUI fixture environment isolation and render/environment lock ordering, and conserved test-module/MIME/presentation extractions. Existing upstream attribution is preserved by normal Git history. Source review and focused checks are not represented as final global acceptance.

The five latest fixture/lock-contract files received independent static review. Ten final targeted regressions passed. The immutable original 63-entry formatting freeze receipt was retained separately from a candidate with exactly one explicitly admitted benchmark entry supersession. No unowned bytes, budget baselines, scanners or production assertions were changed to make gates pass.

## Remaining acceptance issues

| Gate | Actual final evidence |
| --- | --- |
| Full TUI library | 2496 passed, 1 failed, 19 ignored, exit 101 |
| Failing regression | `smoothness_plain_text_commit_preserves_the_live_viewport`, `smoothness_benchmark.rs:251`: `big_pop_events` was 1, expected 0 |
| Ten focused regressions | Passed |
| Subsequent final check, Clippy, formatting and guardrails | Unrun after the full-package failfast result; prior scoped passes are historical evidence only |
| Production-size ratchet | 32 violations, 91 total oversized files |
| Swallowed-pattern ratchet | 3514 matches against baseline 3343, across 544 paths |

The viewport failure has not been diagnosed as a production bug, fixture race or measurement-contract issue. Error-pattern counts are candidates for meaningful review, not a claim that every ignored result is a runtime defect. No tests were excluded, serialized for acceptance, weakened or retried to manufacture a green result. No ratchet allowance was increased.

## Functional environment and safe continuation

The session's running harness was `v0.88.469-dev (c232ad32d)` at closeout. Its installed/shared-server channels were not replaced by this candidate. No build, release, deployment, daemon restart or activation was performed for this WIP publication. Continue ordinary project development on the working runtime. Later candidate promotion requires its own exact-tree check and isolated runtime validation.

A complete local-only preservation archive, including verified Git bundle, separate staged/worktree patches and all 18 new source files, was created before publication edits. SHA-256: `db6105080ee6df596f62677c6606f23ce55f3af07f61651d50be92e5610aa20a`. Its bytes have not been uploaded or represented as public attachments. Three personal workstation-path receipt lines were made portable for publication, while the original local archive remains intact.

Before pushing, audit the actual outgoing reachable objects, including merge-resolution content and final metadata. An ordinary patch log alone does not prove complete historical coverage. Known synthetic test fixtures require provenance rather than blanket scanner suppression. Destination is the existing fork integration branch only, with a normal non-force explicit refspec.

## Upstream and housekeeping

Current own upstream PRs and the dependency forks were inspected before choosing further work. No contribution was closed merely for age, failed CI or a stale local branch. No giant integration PR was opened upstream. Relevant new fixture-isolation evidence was submitted to existing upstream PR #1493. Existing linked-issue requirements remain in force. Already merged fixes must not be resubmitted.

Current source-layout policy keeps products under `~/src/<repo>`, system configuration in `~/dotfiles`, and existing harness/dependency source exceptions under `~/.jcode/source/`. Broader automation is a staged dotfiles task, not permission to relocate checkouts, change accounts, install packages, delete worktrees or run unattended production deployment. The detailed local agent brief is `DOTFILES_SOURCE_DEVELOPMENT_AGENT_PROMPT_2026-10-08.md`; no system-wide scaffolding was silently activated.

The next integration task should be finite: explain the viewport failure, distinguish inherited ratchet debt from genuine new regressions, retain the accepted scoped behavior, and reconcile the four newer upstream commits on their exact merits. This checkpoint is progress preserved for iteration, not an invitation to restart an unbounded recovery loop.
