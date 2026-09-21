# `bg wait` reports a running task as missing during a status-file write

Date: 2026-09-21. Status: locally reproduced, fixed, and covered by a regression
test on `jcode/ci-format-baseline`. Not yet published upstream. Runtime reported
`v0.84.255-dev (97a62b6b5)`. The defect exists upstream at `e589cbe5a`
(v0.86.0), where status files are still written with a plain truncating write.

## Expected and observed

`BackgroundTaskManager::status` reads `<task-id>.status.json` and reports an
unreadable file as "task missing", so `wait` returns `None`. The completion path
rewrites that same file with `tokio::fs::write` / `std::fs::write`, which
truncates before writing. A reader that lands in that window observes an empty
file and the task appears to vanish.

Reproduction without any test harness:

```rust
// one manager writes the status file repeatedly while another reads the same
// path; the reader asserts the task is still tracked
for _ in 0..400 { writer.write_status_file(&path, &status).await; }
for _ in 0..400 { assert!(reader.status("atomic-status").await.is_some()); }
```

Observed before the fix: 8 of 8 runs fail with
`a concurrent status write made the task look missing`.

Observed as a real flake: `jcode-base --lib` failed 3 of 15 consecutive full-suite
runs on
`background::tests::adopted_output_is_readable_while_running_and_preserves_final_result`
with `adopted task is tracked`. A temporary diagnostic in `read_status_file`
printed exactly one failure per failing run:

```text
DIAG read_status_file parse len=0 path=.../035325k0o4.status.json
  err=EOF while parsing a value at line 1 column 0 head=""
```

Expected: a reader sees either the previous complete file or the new complete
file, and `wait` returns a task state rather than `None`.

## Proposed fix

Publish every status-file update through a uniquely named sibling plus rename, so
the rename is the only visible transition:

```rust
fn write_status_file_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
    std::fs::write(&temp, contents)?;
    std::fs::rename(&temp, path)
}
```

Applied here to the spawn/adopt initial writes, the completion writes, the cancel
write, and the reconciliation rewrites, with the regression test
`background::tests::status_reads_never_observe_a_partially_written_status_file`.
That test fails 8 of 8 runs against the truncating write and passes 10 of 10
after the change; the full `jcode-base --lib` suite then passed 8 of 8
consecutive runs and 15 of 15 in a second series.

Note for a contributor: a failed publish can leave one uniquely named temp file
behind, and the reconcile sweep and `bg cleanup` both key off the `.json`
extension, so it is inert. Rename-over-existing works on Windows because Rust
opens files with `FILE_SHARE_DELETE`.

## Evidence

- Regression test and helper: `crates/jcode-base/src/background/tests.rs`,
  `crates/jcode-base/src/background.rs`.
- Local receipt with the A/B numbers: `docs/HARNESS_CONTINUATION_PLAN.md`,
  "Update 2026-09-20 22:20 UTC".
- Diagnostics and stress logs: `~/.jcode/scratch/stress-prefix/`,
  `~/.jcode/scratch/merge-tree-full.txt`.
