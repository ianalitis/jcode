# Phase 4: routing measurement contract (docs only)

Workstreams W5 and W6. Independent; can run in parallel with any other phase
because it touches only `docs/`. No code, no config, no spend.

## 1. Goal and non-goals

Goal: one short document, `docs/ROUTING_MEASUREMENT_CONTRACT.md`, that says
which measurement changes which routing default, and on which trigger. It
replaces "re-measure lanes in every handoff" with three dated or event-driven
triggers and a fixed evidence format.

Non-goals: no new metric machinery, no classifier, no router, no ledger, no
change to `~/.jcode/config.toml` or `~/dotfiles` (another agent owns
dotfiles). No lane promotion or retirement; this phase writes the rule, the
operator applies it.

## 2. Acceptance test

Doc exists and contains, verifiably by grep:

- Exactly three triggers: (1) Go special-pricing end 2026-09-27, (2) Go ZDR
  expiry 2026-09-30, (3) any phase whose acceptance failed twice on the
  assigned lane.
- For each lane in `docs/FORK_POSTURE.md §4`, one row: measurement command
  (must be an existing command: `jcode usage --json`, `jcode model list -p`,
  `scripts/openrouter-admin.sh` read path, or a receipt file path), evidence
  path pattern, the number that would retire the lane, the number that would
  promote a lane to default.
- The acceptance-cost function is cited by name and path
  (`docs/HARNESS_LOOP_ARCHITECTURE.md §4`), not restated.
- The `HARNESS_ECONOMY_CYCLE_2026-09-20.md` "Acceptance measurements" table is
  referenced, not duplicated.
- Word count under 600. `grep -c` on the doc for "router", "ledger",
  "dispatcher", "scheduler" returns 0 except inside a "not proposed" sentence.

Reviewer check: `openai-oauth:gpt-5.6-terra` low, read-only, answers one
question: "does any row propose a measurement that no existing command can
produce?" A yes fails the phase.

## 3. Freeze

| Field | Value |
| --- | --- |
| Writer | `openai-oauth:gpt-5.3-codex-spark` or `opencode-go:mimo-v2.5`, effort low. This is extraction and tabulation from four existing docs |
| Reader | the Terra low reviewer above, after the draft |
| Permitted files | `docs/ROUTING_MEASUREMENT_CONTRACT.md` (new) and a one-line pointer added to `docs/FORK_POSTURE.md §4` |
| Iteration cap | 1 draft, 1 revision after review |

## 4. Gate and rollback

`bash scripts/check_guardrails.sh --skip-slow` (only to prove no source
change slipped in). Rollback: delete the file, revert the pointer line.

## 5. Budget and stop

Under 50K tokens on the cheap lane, under 20K for the Terra review. Stop if
the draft exceeds 600 words or proposes machinery.

## 6. Criticism

- Over-engineered? The temptation is a measurement *system*. The contract is a
  table. If the writer returns more than a table and three triggers, reject it.
- Missing evidence: Go's provider windows are not exposed to Jcode, so the Go
  rows can only cite the console and local spend. The doc must say "unknown,
  console authoritative" rather than invent a number.
- Regression: none possible; docs only.
- Abandon if: the operator decides routing policy stays entirely in
  `~/dotfiles/policy/providers.md` and the repo should not carry a second
  statement. Then this phase becomes a one-line pointer to that file and is
  done in five minutes.

## 7. Handoff

Commit `docs: routing measurement contract` when the Terra review passes.
Captain adds the three trigger dates to whatever calendar the operator uses
(O6 is the operator's).
