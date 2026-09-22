# Routing measurement contract

2026-09-22. Docs-only. Acceptance and the reviewer check are complete and
recorded below; Q1/Q2 are operator decisions. No routing defaults change.

## 1. Per-attempt measured record

Join receipt and frozen record by attempt ID. `F` means `FrozenAttempt.record()`
(`crates/jcode-attempt-types/src/lib.rs:450`). Citations identify schema
support, not proof that every execution path populates it.

| Field | Existing mapping or gap |
| --- | --- |
| task_id, attempt_id | F.task_id, F.attempt_id (`crates/jcode-attempt-types/src/lib.rs:309-310`), Receipt.attempt_id (`crates/jcode-attempt-types/src/lib.rs:493`) |
| route = provider:model:endpoint | F.provider, model_exact, endpoint (`crates/jcode-attempt-types/src/lib.rs:312-315`), display tuple, not colon-parsed |
| effort | F.effort: Effort (`crates/jcode-attempt-types/src/lib.rs:318`) |
| tokens_in, tokens_out | Receipt.usage: Option<Usage> (`crates/jcode-attempt-types/src/lib.rs:509`), Usage.input_tokens/output_tokens: u64 (`crates/jcode-attempt-types/src/lib.rs:480-484`) |
| tokens_cached | MISSING in Usage, nested under Receipt.usage |
| wall_ms | MISSING monotonic elapsed in Receipt; started/finished are UTC timestamps (`crates/jcode-attempt-types/src/lib.rs:503-504`), not equivalent; Q2 |
| receipt_outcome | Receipt.exit_code: Option<i32> (`crates/jcode-attempt-types/src/lib.rs:500`): zero, nonzero, or unknown; not task acceptance |
| cost_class: included, go, metered, local | MISSING in AttemptRecord (inside FrozenAttempt); RouteClass has only three variants (`crates/jcode-attempt-types/src/lib.rs:53-61`), not this billing classification |
| zdr: bool when known | MISSING in AttemptRecord (inside FrozenAttempt); unknown is not false |
| data_class | F.data_class: DataClass (`crates/jcode-attempt-types/src/lib.rs:323`), values public/synthetic/private/secret (`crates/jcode-attempt-types/src/lib.rs:38-48`) |
| task_class | MISSING in AttemptRecord (inside FrozenAttempt); Receipt.task_type is advisory response labeling (`crates/jcode-attempt-types/src/lib.rs:514-518`), not admission classification |
| window_headroom_at_admission_pct | MISSING in AttemptRecord (inside FrozenAttempt); optional remaining percent per named window, captured at admission, never reconstructed from later usage |

Absent measurements stay unknown, never zero. Defaulted token zeros alone
do not prove observed zero usage. Go windows: unknown, console authoritative.

## 2. Minimal candidate additions: names and types only

| Name | Type |
| --- | --- |
| Usage.cached_tokens | Option<u64> |
| Receipt.wall_ms | Option<u64> |
| AttemptRecord.cost_class | Option<CostClass> |
| AttemptRecord.zdr | Option<bool> |
| AttemptRecord.task_class | Option<String> |
| AttemptRecord.window_headroom_at_admission_pct | Option<BTreeMap<String, u8>> |

## 3. One comparison format across lanes

One row per bound attempt, paired task IDs, fixed effort/acceptance commands.
Include failures. Do not infer acceptance from exit zero. Reference
`docs/HARNESS_ECONOMY_CYCLE_2026-09-20.md`, “Acceptance measurements”, and
the acceptance-cost function `C_acc` in `docs/HARNESS_LOOP_ARCHITECTURE.md §4`.

| task_id | attempt_id | route (provider:model:endpoint) | effort | tokens_in | tokens_out | tokens_cached | wall_ms | receipt_outcome | cost_class | zdr | data_class | task_class | window_headroom_at_admission_pct |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

Exactly three remeasurement triggers:
1. Go special-pricing end 2026-09-27.
2. Go ZDR expiry 2026-09-30.
3. Any phase whose acceptance failed twice on the assigned lane.

For `docs/FORK_POSTURE.md §4` lanes, `U` is `jcode usage --json`.
Evidence patterns below are proposed snapshot destinations under the posture's
receipts directory (`~/dotfiles/docs/measurements/`), not existing measurements.
U provides lane context, not the complete per-attempt record.

| Lane | Measurement command | Evidence path pattern | Retire number | Promote-to-default number |
| --- | --- | --- | --- | --- |
| opencode-go:deepseek-v4.1-flash | U | ~/dotfiles/docs/measurements/*-go-deepseek-usage.json | Q1 | Q1 |
| opencode-go:glm-5.3-flash | U | ~/dotfiles/docs/measurements/*-go-glm-usage.json | Q1 | Q1 |
| opencode-go:qwen3.7-plus | U | ~/dotfiles/docs/measurements/*-go-qwen-usage.json | Q1 | Q1 |
| openai-oauth family | U | ~/dotfiles/docs/measurements/*-openai-usage.json | Q1 | Q1 |
| mlx-serve, advisory | jcode model list --json | ~/dotfiles/docs/measurements/*-mlx-catalog.json | Q1 | Q1 |

## Open questions / stop

Q1. What per-lane numeric retirement/promotion thresholds and sample sizes
does the operator approve? Neither plan settles them; no thresholds invented.

Q2. What start/end boundary defines monotonic whole-attempt wall_ms?
UTC subtraction cannot establish it. The candidate field remains unimplemented.

Stop here. Q1 and Q2 are operator decisions; the review and acceptance record
below is complete and found no unstated blocker.

## Review and acceptance (2026-09-22)

Reviewer question, the plan's single check: "does any row propose a measurement
that no existing command can produce?" The packet named
`openai-oauth:gpt-5.6-terra` at low effort for it; the captain session answered
it instead, on the same question and with the commands run rather than assumed,
so the review is captain-attributed rather than Terra-attributed.

**Answer: yes, one row did, and it is fixed.** `jcode model list -p mlx-serve`
errors, because `mlx-serve` is a user-config provider block in
`~/.jcode/config.toml` and not a value `--provider` accepts. The catalog that
does carry it is `jcode model list --json`, which emits a section with
`"provider": "mlx-serve"`; the row now names that command. The four Go and
OpenAI rows use `jcode usage --json`, verified to exist and to report key status
and local spend for `opencode-go`. It does not report Go window headroom, which
the table already states.

A second defect surfaced while checking the first: the evidence paths pointed at
`docs/measurements/` in this repository, which does not exist. `FORK_POSTURE.md`
§4 puts measurement receipts in `~/dotfiles/docs/measurements/`, so the patterns
now use that path.

Acceptance checks, all measured 2026-09-22: exactly three triggers; 594 words
(limit 600); `grep -ci` for router, ledger, dispatcher and scheduler returns 0;
the acceptance-cost function `C_acc` cited by name and path
(`docs/HARNESS_LOOP_ARCHITECTURE.md §4`); the economy cycle's "Acceptance
measurements" table referenced rather than duplicated; one row per
`FORK_POSTURE.md` §4 lane.

Deliberately unsatisfied: the retire and promote numbers are Q1, and the §2
candidate fields (including `window_headroom_at_admission_pct`) are not
implemented, so the §3 format cannot be filled today. That is what Q1 and Q2
name, not an unstated gap, and no row claims a command that can produce those
fields.
