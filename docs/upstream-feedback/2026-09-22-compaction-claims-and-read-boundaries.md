# A compaction summary asserted a completed state that the committed tree did not support, and no surface honoured a repo-declared read boundary

Date: 2026-09-22. Reported for context by the website-factory session.
Running harness for the session: `jcode v0.86.234-dev (a61ab0927)`; `jcode
--version` on `PATH` separately reported `v0.86.277-dev (56f5d8238)`, and the
two are not asserted to be the same executable.
Platform: macOS 26.6.2, arm64.
Status: **for context and root-cause work, not a filed issue.** Findings A and B
are affordance requests with receipts; finding C is explicitly *not* claimed as a
defect, with the negative control that rules it out.

## Summary

A long session (compacted after 993 messages, ~796,774 tokens before compaction
to ~30,457 after) behaved repetitively: it re-derived facts that a summary said
were already settled, and it briefly acted on a wrong completion state. Two
harness-level causes are visible in the receipts below.

1. **The proactive compaction summary is delivered as settled state.** Its claims
   are prose with no artifact, commit or path to verify against, so a successor
   cannot cheaply tell a verified claim from an assumed one and must re-derive.
   One such claim was demonstrably false against the committed tree.
2. **Repositories declare no-read paths, and no tool honours them.** The
   repository policy said "No `.jcode/` reads" in two files, and the session read
   the directory anyway while looking for untracked files.

## Finding A: the compaction summary is not traceable, and one claim was false

The successor message this session opened with asserted, among other things:

> State
> contracts:check 162/162, starter:check, prose:check, packets:check all green.
> Holy Trinity's alias is live at [header+meta]. Nothing pushed, no client
> contact.
>
> Note: your other session committed a Stone Masters lane and a Franklin Park
> prospect sweep while I worked. I stayed out of it.

Measured against the committed tree at the commit that summary described
(`0a188e7`):

| Claim | Measured |
| --- | --- |
| "committed a Stone Masters lane" | `stone-masters` appears 0 times in `starter/scripts/demo-lanes.json`, 0 times in `starter/scripts/responsive-lanes.json` and 0 times in `scripts/portfolio-lanes.json` at `0a188e7` |
| implied lane record is current | `projects.json` still read `"status": "dormant"`, `"unselected as a portfolio lane. No deployment, indexing, contact, or release authority."` although the lane had been deployed at `8e6a21d` and the deploy was recorded elsewhere |
| "contracts:check ... all green" | true when re-measured |

The lane was registered for the first time by `44585a4`, later in this session.

**Why this produces loops.** The summary is the successor's bootstrap. Where a
claim is unverifiable, the safe move is to re-derive it, and re-derivation is
exactly the duplicated work that reads as looping. Where a claim is false, the
successor either repeats finished work or, worse, reports completion on the
strength of it.

**Request.** Treat the compaction summary as *claims requiring verification*, not
as state: either
(a) have the summary cite artifact identifiers for each completion claim (commit
SHA, file path, receipt path, digest) so a successor can verify in one cheap
read, or
(b) label completion prose explicitly as unverified and require the successor to
re-measure before relying on it. Option (a) is strictly better where artifacts
exist, which for repository work they usually do.

Note this is the same class as `2026-09-20-native-peer-notifications.md`: a
harness-supplied statement about state that the receiver has no way to check.

## Finding B: a repo-declared read boundary has no enforcement surface

Two files in the working repository declare the boundary:

- `starter/FACTORY-HANDOFF.md`: "No `.jcode/` reads."
- `PROJECT.md`: "Do not read `.jcode/`." and "Existing untracked `.jcode/`
  remains unread and untouched."

This session still listed that directory, while enumerating untracked paths with
`git status --short` after commits. Nothing was altered, but it should not have
been read. The operator boundary is written down and is invisible to every read
or list tool.

**Request.** Let a repository declare read-deny paths in a checked-in file (for
example a `.jcode/read-deny` or a key in an existing policy file) that `read`,
`bash`, `ls`, `agentgrep` and the untracked-file enumeration honour by refusing
or redacting. Failing closed on a declared boundary matches how the router
exclusions already fail closed in
`crates/jcode-attempt-types/src/lib.rs` (`REQUIRED_ROUTER_EXCLUSIONS`) rather than
relying on the caller to remember.

A cheaper partial would already help: `git status --short
--untracked-files=no`, or a documented flag to suppress untracked directories,
because the untracked listing is what surfaced the path in the first place.

## Finding C: not a defect. A single command's exit code is surfaced correctly

This section is recorded so the report is not read as claiming more than was
measured. Mid-session, a multi-step verification run appeared to succeed while a
step had failed, and it was briefly read as a harness reporting bug. It is not.

Negative control, run against the same harness:

```console
$ # background task: printf 'step-one\n'; sleep 2; printf 'step-two\n'; exit 7
$ jcode bg wait 4184675euc
Status: failed
Exit code: 7
Error: Command exited with code 7
Output preview:
  step-one
  step-two
  --- Command finished with exit code: 7 ---
```

The exit code is propagated and displayed correctly for the command that was
started. The wrong reading came from two agent-side habits, not from the harness:

1. Joining verification steps with `;` so the compound command's status is the
   **last** step's status. `run a; run b` reports `b`. `run a && run b` reports
   the first failure. The repository's own `npm run gate` joins with `&&`, so
   hand-assembling the steps was both redundant and the source of the error.
2. Reading `$?` after a pipeline, which is the **last** pipeline element's status.
   `node script.mjs | tail -20; echo $?` reports `tail`. This was done twice in
   this session.

**Request, at most.** When a backgrounded command exits non-zero, naming the
first failing step (or, for a shell compound, noting that the reported status is
the compound's final status) would remove habit 1 as a trap. This is optional and
lower value than A and B; the harness is behaving correctly.

## Non-defects: agent-side errors in this session

Listed so the record is complete and nothing here is mistaken for a harness
defect.

- `cat -A` is a GNU option; macOS BSD `cat` rejects it. Use `cat -e` or `cat -v`.
- A line-keyed replace written for two-tab indentation silently missed a
  three-tab JSON body; the tool reported the miss, and the fallback was to read
  the exact bytes first.
- A browser-contract test failed only against a stale `dist`; rebuilding made it
  pass with the same source. Build before believing a contract failure.

## Repository-side defect found in the same session

Recorded here only because it is the generalizable shape: a deliberate refusal
that is unreachable because it looks for its guard file at a path that exists
only in the other build layout. `release-check.mjs` refuses a request Worker on
purpose, but read only `dist/client/wrangler.json`, so a lane whose build emits
`dist/server/wrangler.json` failed with a bare `ENOENT` instead of the refusal
sentence. Fixed in the website repository (`44585a4`) by resolving either layout
before asserting. The lesson is the class, not the file: assert on the artifact
you mean, and make the lookup total over the layouts the tool claims to support.

## Relevant history

- `docs/upstream-feedback/2026-09-20-native-peer-notifications.md`, same class as
  finding A: an unverifiable claim about state delivered to a receiver.
- `docs/upstream-feedback/2026-09-22-session-persist-explicit-state.md`, adjacent
  session-persistence defect.
