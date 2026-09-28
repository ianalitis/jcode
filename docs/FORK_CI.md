# Fork CI

This fork keeps upstream validation commands while making ordinary branch CI work without repository secrets.

## What runs

- `CI` runs on every branch push and on pull requests targeting `main` or `master`.
- `Workflow Lint` runs independently when files under `.github/workflows/` change on a push or on a pull request targeting `main` or `master`. It installs actionlint 1.7.12 and uses the standard Ubuntu runner's shellcheck.
- Windows Smoke remains manual. iOS tests and unsigned simulator compilation retain their existing triggers, but the signing and TestFlight upload job runs only in `1jehuang/jcode`.
- `Semantic PR labels` still runs on Greptile's completed checks, but only after a `gate` job confirms `OPENROUTER_API_KEY` is configured. This fork has no such secret, so the labeler job is skipped instead of failing. Before the gate, every Greptile review produced a red `Semantic PR labels` check from the labeler's `OPENROUTER_API_KEY is required.` exit, which buried real failures under a known-bad result. See `upstream-feedback/2026-09-21-greptile-labeler-missing-key.md`.

The existing CI build and test commands are retained, including formatting, all-target/all-feature checks, clippy, dependency and size ratchets, SDK checks, Unix and Windows builds, targeted cohorts, integration tests, installer checks, and script syntax checks. Workflow-only validation does not imply that those source gates pass.

## Cost and trust boundary

Automatic fork validation uses only standard `ubuntu-latest`, `macos-latest`, and `windows-latest` hosted runners. Standard hosted runner usage is free for public repositories, but cache and artifact storage have separate allowances. The fork reuses existing caches and does not upload diagnostic artifacts; failure details remain in job logs.

Validation workflows request read-only repository contents, do not persist checkout credentials, do not read `DEPLOY_KEY`, and do not use `pull_request_target`. Cargo's locked git sources are public HTTPS repositories. Anonymous advertised-tag checks matched the two locked revisions, but that evidence is not a full cold Cargo fetch. Optional telemetry is disabled in validation with `JCODE_NO_TELEMETRY=1` and `DO_NOT_TRACK=1`. The installer conversion test alone overrides these flags: it replaces `curl` with a local stub and must exercise both collection and opt-out assertions without making telemetry requests.

These changes do not enable release tags, signing, publishing, deployment, TestFlight upload, paid review actions, larger runners, or self-hosted runners for the fork. The existing release workflow is unchanged except for a shell-formatting lint fix and is not part of automatic fork validation.

## Local workflow validation

From the repository root, with actionlint 1.7.12 and shellcheck available:

```sh
actionlint -no-color
git diff --check
```

The recorded baseline covered workflow syntax and shellcheck findings only. It did not run Cargo, platform suites, release scripts, or publishing paths. This branch began from the published fork state at `0735c75317e644ecb440e0c3dddb7a6b3cd0d8bf`; unrelated changes in the separate dirty local main checkout are neither included nor modified.

## Local integration-line CI continuation, 2026-09-28

From clean `5c174c804`, commit `1ef5886be` moved the unchanged interactive
capture/REPL and transcript memory-extraction methods out of the 1404-line
`agent/turn_execution.rs`. The latter is now 1180 lines; the new modules are
119 and 110 lines. The only change inside the moved method makes the absent
project-directory fallback explicit. `MemoryManager::default()` calls `new()`,
so this keeps the same fallback. No ratchet baseline was updated.

Local checks: `cargo fmt --all -- --check`, strict app-core all-feature Clippy,
and the four `agent::model_usage_tests` pass. Source comparison confirmed the
moved capture/REPL bodies are identical and the memory method differs only in
the equivalent manager fallback. The production-size ratchet now reports **29**
violations (down from 30), and swallowed-error-like usage **3401/3343** (down
from 3402/3343). Both ratchets remain red.

The full `cargo test -p jcode-app-core --lib` run compiled and ran **1600
passing, 2 failing, 31 ignored** tests. The failures are in unchanged tool
code: `tool::bash::tests::sleeping_command_does_not_request_stdin` reproduced
alone, and `tool::tests::tool_parameter_descriptions_stay_under_token_cap`
reproduced alone with three swarm parameter descriptions over its 25-token
limit (`label`, `tldr`, `to_swarm`). These are separately scoped defects to
investigate, not grounds to relax a test or claim a passing full suite.

Hosted CI has not run on this unpublished integration-line commit. No push,
baseline increase, daemon promotion, or upstream PR/issue mutation occurred.
