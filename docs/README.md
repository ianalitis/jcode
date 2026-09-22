# jcode Docs

Reference documentation for the jcode codebase.

## Layout

- `docs/*.md` — architecture, feature, and behavior docs (current state of the system).
- `docs/plans/` — forward-looking plans, roadmaps, and TODO trackers. May be partially implemented or stale.
- `docs/audits/` — point-in-time audits and reviews. Historical snapshots, not kept up to date.
- `docs/proposals/` — design proposals not yet committed to.
- `docs/dev/` — developer-facing process and testing notes.

## Key entry points

- Current continuation prompt and line state: [HANDOFF_2026-09-22_DECISION_ARM_AND_LINE_STATE.md](HANDOFF_2026-09-22_DECISION_ARM_AND_LINE_STATE.md)
- Local decision arm (W3, laya), plan and gates: [plans/2026-09-22-LAYA_LOCAL_DECISION_ARM.md](plans/2026-09-22-LAYA_LOCAL_DECISION_ARM.md)
- Current CI/CD and upstream-contribution continuation prompt: [HANDOFF_2026-09-21_CI_AND_UPSTREAM_CONTRIBUTIONS.md](HANDOFF_2026-09-21_CI_AND_UPSTREAM_CONTRIBUTIONS.md)
- Current Go-primary continuation prompt: [HANDOFF_2026-09-21_GO_CONTINUOUS_IMPROVEMENT.md](HANDOFF_2026-09-21_GO_CONTINUOUS_IMPROVEMENT.md)
- Current fork state and evidence: [UPSTREAM_RECONCILIATION_2026-09-21.md](UPSTREAM_RECONCILIATION_2026-09-21.md)
- Branch/worktree retirement and unresolved work: [BRANCH_LEDGER_2026-09-21.md](BRANCH_LEDGER_2026-09-21.md)
- Workflow and configuration boundaries: [FORK_POSTURE.md](FORK_POSTURE.md)
- CI/CD strategy for the public fork portfolio: [plans/2026-09-21_OSS_CICD_STRATEGY.md](plans/2026-09-21_OSS_CICD_STRATEGY.md)
- Fork branch inventory and cleanup plan: [FORK_BRANCH_CLEANUP_2026-09-22.md](FORK_BRANCH_CLEANUP_2026-09-22.md)
- CI/CD rollout results (what was applied and observed): [OSS_CICD_ROLLOUT_2026-09-21.md](OSS_CICD_ROLLOUT_2026-09-21.md)
- Fork CI shape and trust boundary: [FORK_CI.md](FORK_CI.md)
- A dated handoff is a snapshot, not standing authority or current runtime state.

- Architecture: `SERVER_ARCHITECTURE.md`, `MODULAR_ARCHITECTURE_RFC.md`, `CRATE_OWNERSHIP_BOUNDARIES.md`
- Swarm: `SWARM_ARCHITECTURE.md`, `SWARM_TASK_GRAPH.md`
- Memory: `MEMORY_ARCHITECTURE.md`, `MEMORY_BUDGET.md`, `MEMORY_INCIDENT_RUNBOOK.md`
- Refactoring and quality: `REFACTORING.md`, `plans/CODE_QUALITY_10_10_PLAN.md`
- Desktop app: maintained in a separate repository; desktop architecture files are not in this checkout.
- Providers: `PROVIDER_DOCTOR.md`, `AWS_BEDROCK_PROVIDER.md`
- Platform: `WINDOWS.md`, `TERMINAL_CAPABILITIES.md`

## Conventions

- Docs describing current behavior live at the top level; anything speculative goes in `plans/` or `proposals/`.
- Prefer updating an existing doc over adding a near-duplicate.
- Root of the repo should only hold README, CONTRIBUTING, RELEASING, AGENTS, LICENSE, and similar meta files. Put everything else here.
