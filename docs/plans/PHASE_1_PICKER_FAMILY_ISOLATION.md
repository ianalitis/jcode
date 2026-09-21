# Phase 1: picker/catalog family takes the env lock

Workstream W2 family (b). Sequencing: first. Independent of Phase 2 but must not
run concurrently with it in the same tree.

## 1. Goal and non-goals

Goal: the twelve picker/catalog tests and `shortcut_hints` stop failing under
parallel `cargo test -p jcode-tui --lib`.

Operator-visible check: six consecutive default-thread runs report zero failures
from the family listed in `docs/TUI_TEST_FLAKINESS.md` "Parallel failure set
measured 2026-09-21". Render-family failures (Phase 2) are allowed to remain.

Non-goals: touching `create_test_app`, `lock_test_env`, or any file outside the
permitted list. No reentrant guard. No blanket lock.

## 2. Acceptance test

```sh
cd ~/.jcode/source/jcode
for i in 1 2 3 4 5 6; do
  JCODE_TEST_ENV_LOCK_TIMEOUT_SECS=20 cargo test -p jcode-tui --lib 2>&1 \
    | tee "$JCODE_SCRATCH_DIR/phase1-run-$i.log" | grep -E '^test result|FAILED|failed'
done
grep -h '^test .*FAILED' "$JCODE_SCRATCH_DIR"/phase1-run-*.log | sort | uniq -c
```

Number that must change: family (b) failures across six runs, from 12 (measured
2026-09-21, sum of the frequency table) to **0**. Suite wall time must stay
under 60s per run (baseline 21 to 30s); over 90s is a fail regardless of
failures.

Baseline the same command once before editing and keep that log.

## 3. Freeze

| Field | Value |
| --- | --- |
| Lane, model | `opencode-go:glm-5.3-flash` |
| Effort | medium |
| Writer | one; holds `scripts/repo-lease.sh` on this repo |
| Readers | none needed |
| Tool surface | Read, Edit, Bash limited to `cargo test -p jcode-tui`, `git diff`, `git commit --only` |
| Permitted files | `crates/jcode-tui/src/tui/app/tests/state_model_poke_02/*.rs`, `crates/jcode-tui/src/tui/app/tests/state_model_poke_03.rs`, `crates/jcode-tui/src/tui/app/tests/state_model_poke_03_partition_0{1,2,3}_tests.rs`, `crates/jcode-tui/src/tui/app/shortcut_hints.rs` (test module only), and if step 2 is needed, one new helper in `crates/jcode-tui/src/tui/app/tests/support_failover/part_02.rs` |
| Iteration cap | 2 |

## 4. Method (the packet body)

Step 1: in each named test, replace the leading `ensure_test_jcode_home_if_unset();`
with `let _env = crate::storage::lock_test_env(); ensure_test_jcode_home_if_unset();`
so the test holds exclusion for its duration. Keep the helper call: it still sets
`JCODE_HOME` when unset. Run the acceptance loop.

Step 2, only if step 1 leaves family (b) failures: add a catalog reset at the top
of each affected test using the same call the passing siblings use (grep
`state_model_poke_03.rs` for the sibling pattern that clears
`model_picker_cache` or the provider catalog cache; reuse it, do not invent one).
Run the acceptance loop again. That is the cap.

Hard rule: if any test in the family now times out on the env lock (the bounded
wait reports a holder), stop. That is a nested acquisition and belongs to a
Sol/high review, not this packet.

## 5. Gate and rollback

Gates: `bash scripts/check_guardrails.sh --skip-slow` (format, ratchets: the
oversized-test ratchet matters because these files are large), then the
acceptance loop. `cargo clippy -p jcode-tui --all-targets -- -D warnings`.

Rollback: `git checkout -- <permitted files>`. No state outside the tree.

## 6. Budget and stop

Expected: one cargo build of `jcode-tui` tests (~4 to 5 min cold), twelve runs
of ~30s each, two iterations. Under 30 minutes wall, well under 500K Go tokens.

Stop conditions: iteration cap reached with family (b) still failing; suite time
over 90s; any env-lock timeout; any diff outside permitted files.

## 7. Criticism

- Over-engineered? No. Twelve edits of one line each.
- Missing evidence: whether family (b) is `JCODE_HOME` only or also the
  in-process catalog cache. Step 1 measures that. Step 2 exists for the second
  case.
- Regression: a family test that already transitively locks would self-deadlock.
  The 20s timeout makes that a named failure in 20s, not a wedge.
- Would abandon if: step 2 still leaves failures. Then the shared state is
  somewhere not yet named and the fix needs a reader pass first.
- The `jcode-app-core` spill test is deliberately excluded: it already locks;
  its flake needs a sibling named first. A `mimo-v2.5` read-only packet may grep
  `crates/jcode-app-core/src/tool/*tests*.rs` for `JCODE_HOME` writers that do
  not lock and return the list. Not part of this phase's acceptance.

## 8. Handoff to Stage B

Start with: baseline log at `$JCODE_SCRATCH_DIR/phase1-baseline.log`, then the
packet. Commit as `test(tui): picker/catalog tests hold the env lock for their
duration` with the six-run counts before and after in the body. Update the
frequency table in `docs/TUI_TEST_FLAKINESS.md` (captain does this, not the
worker). Phase 2 may start when this commit is on the branch.
