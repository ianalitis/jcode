# Phase routing table, 2026-09-21

One frozen treatment per phase. Route sticky per job. Failure closes the
treatment and hands off visibly; no silent substitution. Every worker receipt
opens with its model and effort.

| Phase | Writer lane | Effort | Reader lane (read-only) | Tool surface | Verification bar | Blocked on |
| --- | --- | --- | --- | --- | --- | --- |
| 1 picker family env lock | `opencode-go:glm-5.3-flash` | medium | none | Read, Edit, `cargo test -p jcode-tui`, `git diff`, `git commit --only` | 6 parallel runs, family (b) failures 12 -> 0, wall < 60s/run, guardrails `--skip-slow` | O1 only (docs commit) |
| 2 flicker history thread-local | `opencode-go:glm-5.3-flash`; escalate `openai-oauth:gpt-5.6-terra` medium if diff leaves one file | medium | `opencode-go:mimo-v2.5` none: list readers of the four frame-metrics statics | Read, Edit, `cargo test -p jcode-tui`, `cargo clippy -p jcode-tui` | 6 parallel runs, 0 failures, single-thread count unchanged, full guardrails | Phase 1 landed |
| 3 upstream #1358 port | `openai-oauth:gpt-5.6-terra` | medium | `opencode-go:mimo-v2.5` none: map the 11 tests across both layouts | worktree only; `cargo test -p jcode-base`, `git` | 3 parallel runs 11 -> 0 in a pristine worktree, no new clippy findings, PR body with A/B | O2 (branch), O3 (push) |
| 4 routing measurement contract | `openai-oauth:gpt-5.3-codex-spark` or `opencode-go:mimo-v2.5` | low | `openai-oauth:gpt-5.6-terra` low: one-question review | Read, Write to one docs file | grep-verifiable checklist, < 600 words, Terra answers "no" | none |

Captain (this lane, or Sol/high for integration review) owns: baseline logs,
updating `docs/TUI_TEST_FLAKINESS.md` and `docs/FORK_POSTURE.md` after each
phase, commits of docs, and the decision to reload (O4).

## Concurrency

- Phases 1 and 2: sequential, same tree, one writer lease.
- Phase 3: parallel with 1 or 2, in its own worktree, once O2 is given.
- Phase 4: parallel with anything; docs only.
- Live workers at any time: at most 2 writers (main tree + worktree) and 2
  readers. Within the configured concurrency cap.

## Lanes deliberately not used

- `claude-oauth:claude-fable-5-1`: Stage A is done; the 7-day Fable window is
  at 60%. Re-enter only for a new architecture question, not for revisions to
  these plans (Terra/medium can revise them).
- OpenRouter metered: no phase needs it, and O5 (spend cap) is unresolved.
- `opencode-go:minimax-m2.7`: known HTTP 500 through this route.
- `mlx-serve`: no closed-set classification task in these phases.

## Budget summary

| Phase | Expected | Lane cost class |
| --- | --- | --- |
| 1 | < 30 min wall, < 500K Go tokens | included |
| 2 | < 40 min wall, < 500K Go tokens (+ Terra 100K if escalated) | included |
| 3 | < 45 min wall, Terra 150K | ChatGPT 7-day window (30% used; keep 20% headroom) |
| 4 | < 70K tokens total | included / cheapest |

ChatGPT exposure across all four phases stays well under the 20% headroom
rule; Go exposure is unknown at the provider window level (console
authoritative) but local spend is $3.82 this month.
