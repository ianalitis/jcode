# Upstream CI gate reproduction, 2026-10-07

Existing issue: https://github.com/1jehuang/jcode/issues/1740

Refreshing two already-open contributions against upstream `a61c38ee9` removed
the former missing `DEPLOY_KEY` setup failure, thanks to upstream
`02743439929e92e8e93c53583c70b027a82f1bc9`. Both now reproduce master formatting
drift under hosted stable `rustc 1.99.0 (b940084d7 2026-09-28)`:

- Pairing security #1494, head `75a6c94eeac195020a4673ff2afd4ba386e12798`,
  CI https://github.com/1jehuang/jcode/actions/runs/37689253524,
  completed Format job `113024914363`.
- Atomic background status #1357, head `ee9c3d84584fb45161fae9b47dbbefaaceea90de`,
  CI https://github.com/1jehuang/jcode/actions/runs/37689255021,
  completed Format job `113024919712`.

Expected: `cargo fmt --all -- --check` succeeds on the current upstream base.
Observed: both Format and Quality Guardrails fail on the same two upstream files.
The changes in those files are outside both contribution diffs:

1. `crates/jcode-provider-openai-runtime/src/lib.rs:966`,
   `reload_cached_reasoning_efforts`: rustfmt wants the `let cached =` binding
   split before `jcode_base::provider::cached_openai_reasoning_efforts_for_scope`,
   with `&self.catalog_scope()` kept on the call line.
2. `src/cli/commands_tests.rs:284`: rustfmt wraps the
   `anyhow::anyhow!("the provider label test never sends a request")` argument.

The second location was already reported in #1740. The first is additional
current-head evidence. No duplicate issue or broad formatting PR is needed.
No claim is made that these are the only remaining Clippy or test failures:
Quality Guardrails stops at formatting, and build/test jobs were still running
when this reproduction was captured.

Read the finished job independently:

```sh
gh api repos/1jehuang/jcode/actions/jobs/113024914363/logs --allow-escape-sequences
gh api repos/1jehuang/jcode/actions/jobs/113024919712/logs --allow-escape-sequences
```

Local stable is still 1.98.1. Hosted 1.99 evidence is not a local toolchain
upgrade or proof of local reproduction. Do not install a toolchain or weaken
checks to close this report.
