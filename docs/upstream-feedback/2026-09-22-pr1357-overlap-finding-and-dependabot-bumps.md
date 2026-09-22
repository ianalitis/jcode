# PR #1357's overlap finding, and the Dependabot alerts behind the dependency bump

**Date:** 2026-09-22. **Status:** #1357 needs no code change, a reply is drafted
and not posted. The dependency bumps are applied locally in `5bedaf6ea` and not
pushed. Publications are listed in §6 and need operator approval.

**Scope:** the review findings still open across the twelve upstream PRs this
account has against `1jehuang/jcode`, and the seven open Dependabot alerts on
`ianalitis/jcode`.

## 1. Which findings were actually still open

Every open PR was checked against its current head, not against the review
timestamp: greptile re-anchors its inline comments to the newest head, so a
comment's `commit_id` and line can postdate the body it carries.

| PR | Head | Finding | State |
| --- | --- | --- | --- |
| [#1356](https://github.com/1jehuang/jcode/pull/1356) | `27231db08` | P1 stale holder metadata, P2 unsynchronized waiter | **Resolved at head.** `TestEnvGuard::drop` marks the holder record released and `is_held_by_current_thread` requires `held`; the contended-wait test waits on a readiness channel. Greptile's own re-review at this head reports both as done. Inline text is stale. |
| [#1357](https://github.com/1jehuang/jcode/pull/1357) | `245c44b4d` | P2 "Race test lacks overlap" | **Still listed by greptile's latest review at head.** No code change needed, see §2. |
| [#1360](https://github.com/1jehuang/jcode/pull/1360) | `9618d3e95` | P2 assert the sidecar default, not the env var name | **Resolved** by the applied suggestion commit; greptile re-reviewed and says the sidecar default is now asserted directly. |
| [#1362](https://github.com/1jehuang/jcode/pull/1362) | `2a8c4e8dc` | P2 stopping-gate bypass | **Resolved** by `2a8c4e8dc` and replied to; greptile's single review is the pre-fix one. |
| [#1366](https://github.com/1jehuang/jcode/pull/1366) | `211067fc4` | outside-diff P1, numeric account labels | **Split into #1368** and replied to; outside this PR's scope by design. |
| [#1368](https://github.com/1jehuang/jcode/pull/1368) | `9b4f9940e` | P2 vacuous switch assertion | **Resolved**: two accounts, active-before assertion, negative control recorded. Greptile: "No findings outside the diff remain." |
| #1354, #1355, #1364, #1371, #1372, #1373 | unchanged | none | Only summaries, 5/5, no inline findings. |

So one item was genuinely open: #1357's P2.

## 2. #1357's P2, measured rather than argued

The finding says the spawned writer "has no suspension point, so on the default
current-thread test runtime it can complete all 400 writes before the reader
resumes", making the test unable to catch a return to truncating status writes.
That was written against `13612fd2f`, whose write path was:

```rust
async fn write_status_file(&self, path: &std::path::Path, status: &TaskStatusFile) {
    if let Ok(json) = serde_json::to_string_pretty(status) {
        let _ = write_status_file_atomic(path, &json);
    }
}
```

No `await` in the loop body, so the finding is right about that commit.
`dc4651417` changed exactly that: publication moved to `spawn_blocking`, and
`background/status_write_tests.rs` was added.

Three controls were run on this checkout's integration line, whose
`write_status_file`, `write_status_file_locked`, `publish_status_file` and
`write_status_file_atomic` are **byte-identical to `pr/background-status-atomic-writes`
at `245c44b4d`** (compared function by function, not by diffstat). Each control
replaces only the atomic publication with an in-place truncating write, and the
third also inlines it to reconstruct `13612fd2f`'s no-suspension shape.

| Writer shape | #1357's loop test (as at head) | The strengthened loop test on the integration line |
| --- | --- | --- |
| atomic + `spawn_blocking` (head) | passes | passes |
| truncating + `spawn_blocking` | **fails 8/8**, "a concurrent status write made the task look missing" | **fails 5/5** |
| truncating + inline (the `13612fd2f` shape) | **passes 8/8** (vacuous, the finding reproduced) | fails 2/5 (not deterministic either) |

Two conclusions follow:

- The finding was correct for the commit it reviewed, and the commit that
  addressed it is `dc4651417`, which is in this PR.
- At the current head the test is no longer vacuous: a truncating publication is
  detected 8/8 on the default current-thread runtime, because each publication
  now suspends. Adding synchronization to this loop test is not what makes the
  guard real.

The deterministic guard the finding asks for is in this PR already:
`background/status_write_tests.rs::staged_atomic_publication_exposes_only_complete_old_then_new_json`
installs a pre-rename hook, lets a publication stage its temp file, reads both
the live path (old complete JSON) and the staged file (new complete JSON), then
resumes. Under a regression to an in-place write it cannot pass vacuously: with
the truncating-plus-inline control above it fails closed in 3s with `status
write did not reach the pre-rename hook: timed out waiting on channel`. All six
tests in that module pass on the restored tree.

## 3. Why no third variant was added to #1357

The strengthened loop test (multi-thread flavor plus a `writes > 1` overlap
assertion plus a fails-closed check for an empty file) exists on the integration
line from `450702133`, a review follow-up for #1356 and #1357. Porting it to
#1357 would add a third overlapping guard without making the property any more
deterministic than the staged hook already is, and the ported file was never run
against that branch's tree. A reply with the measurements above is the smaller,
checkable answer. If a reviewer prefers the extra guard, the port is a
mechanical copy of one function.

## 4. The dependency alerts

Seven Dependabot alerts are open on `ianalitis/jcode` (alerts were enabled on the
five public forks on 2026-09-21). All seven are patchable inside ranges the graph
already allows, so no manifest change is involved.

| Alert | Package | Range | Patched | Path in this tree |
| --- | --- | --- | --- | --- |
| GHSA-3pv8-6f4r-ffg2 | `tar` | `<= 0.4.45` | 0.4.46 | direct `jcode-app-core` dependency, release-archive extraction |
| GHSA-3rjw-m598-pq24 / CVE-2026-50185 | `cmov` | `< 0.5.4` | 0.5.4 | `ctutils -> digest -> hmac -> aws-sigv4`, Bedrock signing |
| GHSA-cq8v-f236-94qc / RUSTSEC-2026-0097 | `rand` | `>= 0.7.0, < 0.8.6` | 0.8.6 | `ratatui-image` and `tungstenite` |
| GHSA-5jgf-p345-68v8 + 3 more | `fast-uri` | `>= 3.0.0, < 3.1.6` | 3.1.6 | `ajv` (a direct runtime dependency of the SDK), `sdk/typescript/package-lock.json` |

Applied in `5bedaf6ea` on `jcode/ci-format-baseline`, lockfiles only:

- `tar` 0.4.45 -> 0.4.46, `cmov` 0.5.3 -> 0.5.4, `rand` 0.8.5 -> 0.8.6, one
  `cargo update --precise` each. The `rand` 0.9.3 and 0.10.1 lines already carry
  the patch for their ranges, so the advisory is now closed across the graph.
- `sdk/typescript`: `fast-uri` 3.1.5 -> 3.1.8, written as three lines of the
  lockfile from registry-verified `version`/`resolved`/`integrity` rather than by
  regenerating the lock, because `npm audit fix` and `npm update` both also
  reorder the platform-specific optional packages (63 lines of unrelated churn).
- `docs/SECURITY_DEPENDENCIES.md` updated: the resolved `rand` row left the
  current-advisory table, the four resolutions were added to its notes, and the
  register now says it is fed by Dependabot in addition to `cargo audit`.

cargo re-derived seven target-gated `windows-sys` edges among versions already
present in the lock (`arboard`, `errno`, `quinn-udp`, `rustix` twice,
`rustls-platform-verifier`, `tempfile`, all of which allow `>=0.52.0, <0.61.0`).
No crate version was added or removed by that.

Verified: `cargo check --workspace` clean, `cargo test -p jcode-base --lib` 1555
passed / 0 failed, `npm install --dry-run` and `npm audit` clean in
`sdk/typescript`. `scripts/security_preflight.sh` was **not** run: `cargo-audit`
is not installed in this environment, so the crate side rests on the alert
ranges and the resolved versions rather than on a local audit.

The fork's default branch is upstream's mirrored tree, so these seven alerts stay
open on GitHub until the same versions move upstream. Clearing them on the fork
would mean diverging the mirror, which the posture forbids; the honest remedies
are an upstream PR (needs approval, §6) or leaving them as visibility.

## 5. Draft reply for #1357, not posted

> The finding was right for the commit it reviewed, and I reproduced it: with
> `13612fd2f`'s write path (no `await` in the loop, so no suspension point) and
> the atomic rename replaced by an in-place write, this test passes 8/8 without
> ever reading during a write.
>
> `dc4651417` is the commit that addresses it. Publication now runs on
> `spawn_blocking`, so each write suspends, and the same truncating-write control
> against the current head fails 8/8 on the default current-thread runtime with
> "a concurrent status write made the task look missing" (measured on a tree
> whose `write_status_file`, `write_status_file_locked`, `publish_status_file`
> and `write_status_file_atomic` are byte-identical to this branch's head).
>
> The deterministic synchronization this asks for is also in this PR already:
> `background/status_write_tests.rs::staged_atomic_publication_exposes_only_complete_old_then_new_json`
> holds a publication at its pre-rename hook, reads the live path and the staged
> temp file while it is held, then resumes. It cannot pass vacuously either: with
> the same truncating-plus-inline control it fails closed in 3s with `status
> write did not reach the pre-rename hook: timed out waiting on channel`, and it
> is one of six passing tests in that module.
>
> I did not add synchronization to this loop test as well. A strengthened version
> exists on my integration line, and under the old inline shape it is not
> deterministic either (2/5 failing, 3/5 passing), so it would add a third
> overlapping guard without making the property deterministic. Happy to port it
> if you would rather have the belt and braces.

## 6. Publications this receipt does not perform

1. Post §5 as a comment on #1357 (and optionally a one-line reply on #1356
   pointing at `27231db08`, whose inline comments are stale).
2. Push `5bedaf6ea` (`jcode/ci-format-baseline`) to `fork` if the fork is meant
   to carry the dependency bumps on its integration line.
3. Open the upstream contribution for the same lockfile bumps
   (`pr/dependency-patched-bumps` off `origin/master`, then a PR) if clearing the
   fork's seven alerts matters more than zero mirror divergence.
4. Nothing was pushed, no branch or worktree was created, no PR or comment was
   posted, and the shared daemon was not reloaded while preparing this.

## 7. Local gate state after this pass, and a corrected attribution

`scripts/check_guardrails.sh --skip-slow` failed three gates before this pass:
`cargo fmt --all --check`, the oversized-file ratchet, and the swallowed-error
ratchet. The format gate is now green: rustfmt reported exactly one offender on
this line, inside the test added by `386320cb1`, and `319799dc7` formats it with
no behavior change (39 jev tests still pass).

The other two are still red, and "upstream-red" is only half right for this line.
Measured against `origin/master` rather than assumed:

| Offender | Baseline | HEAD | `origin/master` | Owner |
| --- | --- | --- | --- | --- |
| `crates/jcode-base/src/jev.rs` | new oversized | 1206 | 1109 | ours, `386320cb1` (the choice/score validation) |
| `crates/jcode-tui/src/tui/app/navigation.rs` | 2003 | 2066 | 2003 | ours, `56f5d8238` |
| `crates/jcode-tui/src/tui/ui.rs` | 3626 | 3627 | 3720 | ours by one line, and upstream is 94 above the same baseline |
| `crates/jcode-tui/src/tui/ui/url.rs` `.ok()` sites | 2 | 4 | 2 | ours, `56f5d8238` (two regex-compile sites following the file's established idiom) |

Clearing the size ratchet means either removing 63 lines from a fork feature or
updating `scripts/code_size_budget.json`, and clearing the swallowed-error one
means either a baseline update or turning two `Regex::new(..).ok()` sites into
something more explicit than the file's own convention. Both scripts say to
rebaseline only after intentional cleanup, and neither is a drive-by, so this
pass recorded them instead. The one thing it did not do is leave an offender of
our own that a mechanical tool could have fixed.
