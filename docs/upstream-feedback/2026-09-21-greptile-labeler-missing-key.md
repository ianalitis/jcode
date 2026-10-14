# `Semantic PR labels` fails when the labeler's provider key is absent

Date: 2026-09-21. Status: fix staged on the integration line
(`jcode/ci-format-baseline`); not yet published to the fork default branch and
not yet filed upstream. Upstream-side defect is separate (see below).

## Expected and observed

`label-pr.yml` triggers on Greptile's completed check runs and check suites
(`app.id 867647`) and on `workflow_dispatch`. Its only precondition is that
`1jehuang/jev-pr-labeler` can call OpenRouter with
`secrets.OPENROUTER_API_KEY`. Nothing checks that precondition before the run.

On a repository without that secret the action exits 1, so **every** Greptile
review produces a failing check named `Semantic PR labels`:

```
"requested_head_sha": "603e3b5898297d3f6e2774125d106e4da469b79b",
"matched_open_prs": 1,
"results": [ { "pull_request": 3, "status": "error",
               "reason": "OPENROUTER_API_KEY is required." } ]
##[error]Process completed with exit code 1.
```

Confirmed on `ianalitis/jcode` runs `35666684356` and `35666684019` (2026-09-21).
A permanently red check on a workflow that is not the author's fault trains
readers to ignore that check name, which is the real cost: it sits next to
warnings that matter.

Upstream `1jehuang/jcode` is also red, for a different and unrelated reason:

```
"pull_request": 1366, "status": "error",
"reason": "API request failed: HTTP 402. No labels should be assumed updated."
```

That is the labeler's OpenRouter account needing attention, run `35666984626`.
The two must not be conflated: the fork's failure is a missing secret, upstream's
is a provider-account failure, and only the first is a code defect.

## Fix

Add a `gate` job that decides whether labeling is viable and have the `label` job
depend on its output. Secrets are not available in `jobs.<job_id>.if`, so the
decision needs its own job and an output rather than an inline condition.

```yaml
  gate:
    outputs:
      enabled: ${{ steps.decide.outputs.enabled }}
    steps:
      - id: decide
        env:
          LABELER_KEY: ${{ secrets.OPENROUTER_API_KEY }}
        run: |
          if [[ -z "${LABELER_KEY}" ]]; then
            echo "enabled=false" >> "$GITHUB_OUTPUT"
            echo "::notice::OPENROUTER_API_KEY is not configured for this repository; skipping semantic PR labels."
          else
            echo "enabled=true" >> "$GITHUB_OUTPUT"
          fi
  label:
    needs: gate
    if: needs.gate.outputs.enabled == 'true'
```

The gate carries the same event filter as the old `label` job, so unrelated
`check_run`/`check_suite` events still do no work, and the manual
`workflow_dispatch` path (which passes an explicit PR number) is unchanged. The
`label` job's own event filter is dropped because `needs.gate.outputs.enabled`
already implies it.

`continue-on-error: true` on the action was rejected: it would also hide real
labeler failures such as upstream's HTTP 402, which is exactly the signal that
should stay visible.

## Evidence

- `actionlint -no-color -shellcheck=shellcheck` passes on the whole workflow set
  after the change (exit 0, no findings).
- Expected runtime behavior: the check transitions from `failure` to `skipped`
  on the next Greptile review, with a `::notice::` annotation naming the missing
  secret. Not yet observed, because publishing to the fork default branch needs
  operator approval.

## Not covered

- Upstream's HTTP 402 is not fixed by this and should not be: it is an
  operator-side account problem. It deserves its own issue so the red upstream
  check is understood, with no code change proposed.
- The gate proves the *key exists*, not that the provider accepts it. A present
  but invalid or unpaid key still fails the run. Detecting that without spending
  a request is not possible from the workflow, so it stays a visible failure by
  design.
