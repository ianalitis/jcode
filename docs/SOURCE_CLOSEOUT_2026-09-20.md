# Source closeout checkpoint: privacy and budget safety

Observed: 2026-09-20 00:43 UTC. Branch: `jcode/ci-format-baseline`.
This is a verified source checkpoint, not a release or whole-workspace acceptance receipt.

## Accepted local commits

| Commit | Scope | Verification |
| --- | --- | --- |
| `a86dcfe46` | Cargo wrapper respects foreign checkout ownership | Synthetic cwd regression, job-sizing checks, actual foreign locate-project |
| `459784734` | Bedrock test extraction with per-test state isolation | Default/no-default suites, strict package Clippy, formatting, sentinel preservation |
| `3ba9f8a76` | Local Cargo test state isolated by default | 11 synthetic isolation cases, cwd/job fixtures |
| `a1a5d213c` | Shared ledger preserves actual costs and rejects overflow | 48 attempt-types tests, 11 loopback caller tests, strict types Clippy |
| `96411b62d` | Format ledger re-export | Exact one-file rustfmt correction; no other native-gate result changed |
| `312f9b37a` | Remove optional collection from fork source | Independent review plus exact staged-candidate checks below |

Privacy implementation originated in the approved isolated branch commit `c81735fdf`.
The canonical integration preserves unrelated J5 and extraction changes. Only the reviewed
44-path privacy delta and a two-line synthetic fixture funding correction were committed.
No blanket staging, reset, branch switch, publication, installation, or shared-daemon restart occurred.

## Exact privacy candidate evidence

The index was exported with `git archive` into scratch, without registering or switching a
worktree. The final tested candidate tree was `0691c1993822fc6acfc412c7795a178b985fef74`.
All changed staged file bytes were compared against the tested export before committing.

Commands used `--locked --offline`, existing dependencies, and isolated test state through
`scripts/dev_cargo.sh`. No external inference or telemetry probes were sent.

- Telemetry core all-target tests: pure-helper unit test and no-collection integration with three subprocess scenarios passed.
- Strict telemetry core all-target Clippy with `-D warnings`: passed.
- Base `config::tests`: 79 passed.
- App-core `tool::discover::tests`: 33 passed in the exact candidate (34 in the separately mapped dirty tree).
- TUI `telemetry` filter: 7 passed.
- OpenRouter `router_shaping_tests`: initially 9/10, then 10/10 after the correction below.
- OpenRouter `attempt_caller::tests`: 11 passed.
- Installer fake-curl fixtures and module-resolution check: passed.
- Staged diff whitespace check: passed.

The banned-family fixture reported 2,070 micro-USD against a 500 reservation and 1,000
ledger cap. Correct full-cost settlement therefore failed before its intended receipt
assertion. Both synthetic limits are now exactly 2,070. The billed amount, served model,
and banned-family assertion are unchanged. This does not relax a production spending cap.

The first archive module check could not find Git metadata. It was rerun successfully
using read-only `GIT_DIR`/`GIT_WORK_TREE` context against the export. The failed invocation
was not counted as a pass. Lockfile regeneration encountered missing cached agentgrep tag
metadata. The reviewed telemetry dependency removal was reconciled exactly instead;
locked offline metadata and all named Cargo checks passed without cache/network changes.

Additional captain checks on the combined working tree passed configuration, support,
discovery, privacy routing, TUI policy and installer tests. The broad concurrency filter
had 21 passes and one failure in `communicate_fill_slots_tops_up_to_concurrency_limit`:
unfinished J5 admission cannot establish billing for the test provider `openai`.
Independent review found that admission seam unrelated to the privacy delta.

## Boundaries and remaining work

- Privacy is committed source behavior, not active-binary certification. Inference, auth,
  remote MCP, requested exports/support and other functional service traffic remain.
  No historical local or remote data was purged.
- The working tree still has extensive uncommitted extraction, protocol and J5 work.
  Preserve it. Privacy assertions have been mapped into extracted test files, but those
  broader extraction changes were not silently adopted in the privacy commit.
- J5 is not ready: request-level maximum-bill proof and whole-child enforcement remain
  incomplete. Its current all-OpenRouter rejection is not a usable routing feature.
- Last full native-gate inventory predates privacy integration. Module/dependency/reexport
  passed; format, production/test size, panic and swallowed-error gates remained blocked.
  Do not reuse those historical counts as a fresh post-integration acceptance result.
- Native ancestor-AGENTS work is separately owned by the existing source writer. Approved
  scope is `prompt.rs`, `prompt_tests.rs`, and `SYSTEM_PROMPT_CONFIG.md`, repository-only
  traversal plus global, with outer-workspace trust/config expansion deferred. It is not
  complete at this checkpoint.
- Earlier unsafe Bedrock fixtures touched the host App Support `bedrock.env`. Source
  isolation repairs prevent recurrence but do not restore that file. No credential contents
  or backup were inspected. Named credential-file/backup recovery approval is still required.

Detailed temporary artifacts: `~/.jcode/scratch/privacy-integration-20260920T0012Z/`
and `~/.jcode/scratch/privacy-index-validation-20260920/`. Re-run checks rather than
assuming scratch artifacts survive. No claim that all highest-priority tasks are complete.
