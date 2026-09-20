# Fork CI

This fork keeps upstream validation commands while making ordinary branch CI work without repository secrets.

## What runs

- `CI` runs on every branch push and on pull requests targeting `main` or `master`.
- `Workflow Lint` runs independently when files under `.github/workflows/` change on a push or on a pull request targeting `main` or `master`. It installs actionlint 1.7.12 and uses the standard Ubuntu runner's shellcheck.
- Windows Smoke remains manual. iOS tests and unsigned simulator compilation retain their existing triggers, but the signing and TestFlight upload job runs only in `1jehuang/jcode`.

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
