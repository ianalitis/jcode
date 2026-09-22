# CodeQL Rust alert backlog triage: all 166 open alerts on `ianalitis/jcode`

Date: 2026-09-22. Triage of the backlog the strategy's gap-register item 12 asked
for (`docs/plans/2026-09-21_OSS_CICD_STRATEGY.md` §4). Read-only: **no alert was
dismissed and no code was changed.** The alerts are against the fork default
branch, which is a mirror; every file cited here is byte-identical on
`origin/master` at `2a4edaa02057ac994a601311c4f03ed450e1b3c9` and on
`fork/master` at `43a2e7539`, verified with `git rev-parse <ref>:<path>`, so this
is a triage of **upstream's code**, not of fork-local work.

## 1. Headline

**166 open alerts. Zero confirmed leaks of secret material.** The backlog is
dominated by a single classifier artifact: CodeQL's Rust rule taints an entire
struct once any field of it is credential-shaped, so every *identifier*, *env var
name*, *file path*, and even the *return value of a write function* derived from
that struct is reported as "written to a log file."

Two genuinely actionable items came out of it, neither of which is a leak:
a misnamed function that a reviewer would reasonably mistake for a redactor, and
the only workflow in the repository with no job permissions block.

## 2. What is open, as measured

```sh
gh api "repos/ianalitis/jcode/code-scanning/alerts?state=open&per_page=100" \
  --paginate \
  --jq '.[] | [.rule.security_severity_level, .rule.id,
    .most_recent_instance.location.path,
    (.most_recent_instance.location.start_line|tostring)] | @tsv'
```

| Count | Severity | Rule |
| --- | --- | --- |
| 143 | high | `rust/cleartext-logging` |
| 10 | high | `py/clear-text-logging-sensitive-data` |
| 6 | critical | `rust/hard-coded-cryptographic-value` |
| 4 | high | `rust/cleartext-transmission` |
| 1 | medium | `py/overly-large-range` |
| 1 | medium | `js/stack-trace-exposure` |
| 1 | medium | `actions/missing-workflow-permissions` |

Split by kind of file: **45 in test or tooling paths**
(`crates/*/tests/`, `*_tests.rs`, `scripts/`), **121 in production paths**.
The production concentration is the authentication CLI: `src/cli/login.rs` 24,
`src/cli/commands.rs` 13, `src/cli/commands/provider_setup.rs` 10,
`src/cli/login/scriptable.rs` 5. That concentration is why the strategy document
said "start there rather than dismissing the rule", and it is the right instinct:
the finding is that the *cluster* is not a leak, not that the file is safe to
ignore.

The strategy document and the handoff both say "100+ alerts" and the handoff gives
the split as 6 `critical` plus 94 `high`. The measured figures are **166 open**,
6 `critical` plus 158 `high`. The earlier readings are lower across the board,
which is consistent with a count taken while the Rust analysis was still opening
alerts — the same mid-run snapshot trap the strategy document already records in
its CodeQL finding — but that cause was not re-measured here, so the only durable
statement is the count in the table above and the command that produced it.

## 3. Why the `rust/cleartext-logging` cluster is not a leak

`rust/cleartext-logging` reports a *source expression* in its message. Across the
153 Rust alerts there are 50 distinct named sources. The most frequent:

| Alerts | Named source | What it actually is |
| --- | --- | --- |
| 26 | `oauth_app` | a **local variable in one TUI test file**, `App`-typed, holding `ResolvedCredential::Oauth` |
| 22 | `session_id` | a session identifier |
| 25 | `resolve_openai_compatible_profile_with_api_key_hint(...)` | the *profile*, whose printed fields are the env var **name** and the model |
| 9 | `read_api_key(...)` | the key-carrying function; every observed sink prints a sibling field of its enclosing report struct |
| 9 | `resolve_api_key_env(...)` | the env var **name** |
| 9 | `write_text_secret(...)` | the **return value of a write call**, which carries no value at all |
| 6 | `...::write_json_secret(...)` | same: the return of a write call |
| 6 | `ordered_uuids(...)` | a UUID ordering |
| 5 | `sanitize_secret_value(...)` | see §5, item A |
| 4 | `next_account_label(...)`, `canonical_account_label(...)` | account labels |
| 4 | `trusted_gateway_base(...)` | a base URL |

**`oauth_app` is the proof of mechanism.** That name appears in exactly one place
in the tree, as a test-local binding of `create_test_app()` in
`crates/jcode-tui/src/tui/app/tests/remote_events_reload_04_partition_01_tests.rs`.
One field of that test app is `remote_resolved_credential = Some(ResolvedCredential::Oauth)`.
CodeQL treats that enum variant as a credential source, taints the whole `App`,
and then flags every downstream hop that reaches a log sink. That is why alerts
land on lines that cannot possibly log anything:

```
$ gh api ... --jq '.[] | select(.most_recent_instance.location.path
    =="crates/jcode-tui-core/src/stream_buffer.rs")'
{"line":248,"msg":"This operation writes oauth_app to a log file."}
{"line":167,"msg":"This operation writes oauth_app to a log file."}
{"line":144,"msg":"This operation writes oauth_app to a log file."}
{"line":391,"msg":"This operation writes oauth_app to a log file."}
```

Line 248 of `stream_buffer.rs` is `self.queue.push_back(QueuedOp::Chunk { .. })`
and line 391 is `series.push_back(JitterEvent { .. })`. `stream_buffer.rs`
contains **no logging call at all** (`grep -n "logging::" ... → no match`), so the
alert location is an interprocedural hop through the buffered payload, not the
sink. The same shape covers `queue_recovery.rs` (7 alerts whose locations are
`remove()` calls next to the real `logging::info`).

Representative production sites read directly, all false positives:

| Site | Prints |
| --- | --- |
| `src/cli/login.rs:889` | `resolved.api_key_env`, the env var **name**, gated by `is_safe_env_key_name` |
| `src/cli/login.rs:1095` | the GitHub **username** returned by `fetch_github_username` |
| `src/cli/login.rs:1426` | `auth::google::credentials_path()?.display()`, a file **path** |
| `src/cli/login.rs:790` | `azure::load_endpoint()`, the configured **base URL** |
| `src/cli/commands/provider_setup.rs:85-94` | the report's `run_command`, `auth_test_command`, `api_key_env`, `env_file_path` |
| `src/cli/login/existing_key_notice.rs` | the provider display name and the **source** of the key (env var name or config path) |

The last row is the one worth having checked by hand, because it is the only sink
whose name suggests it handles key material. It does not:

```rust
Some(format!(
    "A {} API key is already configured from {}.\nPaste a new key to replace it, or press Ctrl+C to keep it.\n",
    resolved.display_name,
    credential_source(resolved)
))
```

`credential_source` returns `"the {ENV_VAR} environment variable"` or the config
file path. No key material.

Finally, a whole-tree search for a *direct* print of a secret value finds nothing:

```sh
grep -rn --include=*.rs -E \
  '(eprintln!|println!|print!|warn!|info!|error!|debug!|trace!)[^;]*\b(api_key|access_token|refresh_token|client_secret|password|secret)\b\s*[},)]' . \
  | grep -v '_env\b' | grep -v '_path' | grep -v test
# no matches
```

## 4. The other rules

| Rule | Count | Verdict |
| --- | --- | --- |
| `rust/hard-coded-cryptographic-value` | 6 | **False positive, confirmed.** All six are "hard-coded value is used as a salt". `ui_animations.rs:440` is `choose_animation_variant(IDLE_VARIANTS, 0x4944_4c45_414e_494d)`, an ASCII-packed animation seed reading `IDLEANIM`. The four in `layout_cache_resize_probe.rs` are in an `#[ignore]`d wall-clock benchmark. No cryptography is involved in any of the six. |
| `rust/cleartext-transmission` | 4 | **Accurate dataflow, by design.** `embedding_backend.rs:189`, `jev.rs:221`, `openrouter_provider_impl` catalog fetch at `lib.rs:548`, `gmail.rs:345`. Each sends an API key to a **user-configured** base URL, which is the deliberate feature (self-hosted gateways, localhost, OpenAI-compatible endpoints). The rule is right that the scheme is not enforced; see §5 item C. |
| `py/clear-text-logging-sensitive-data` | 10 | `scripts/test_oauth_usage.py`, a local test script. |
| `py/overly-large-range` | 1 | `ios/TestHarness/reward/scorers/ai_patterns.py:94`. |
| `js/stack-trace-exposure` | 1 | `telemetry-worker/src/worker.js:2127`. |
| `actions/missing-workflow-permissions` | 1 | **True positive, trivially fixable.** See §5 item B. |

## 5. The two actionable findings

**A. `sanitize_secret_value` is a normalizer whose name implies a redactor.**
`crates/jcode-provider-env/src/lib.rs:58`:

```rust
pub fn sanitize_secret_value(raw: &str) -> &str {
    raw.trim_matches(is_invisible_boundary_char)
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches(is_invisible_boundary_char)
}
```

It strips quotes and invisible characters and returns the **live secret**. It does
not redact. Five alerts are literally "writes `sanitize_secret_value(...)` to a log
file", which a reviewer skimming the message will read as *already handled*. That
is a human-factors defect, not a leak: the value still reaches the same log it
would have without the call. Smallest honest fix is either a rename to
`normalize_secret_value`/`trim_secret_value` (10 usages in 3 files, all in
`jcode-base` and `jcode-provider-env`) or a doc comment stating it does not redact.
Not applied here; it is an upstream change and needs its own branch.

**B. `freebsd-smoke.yml` is the only workflow without a permissions block.**
The alert is at the `smoke` job. Ten of the repository's eleven workflow files
declare a workflow-level `permissions:`; `freebsd-smoke.yml` declares none, so the
job inherits the repository default. The fix is one line, `permissions: contents:
read`, matching every sibling workflow and `docs/FORK_CI.md`'s least-privilege
posture. This is the cleanest small upstream contribution the triage produced.

**C. Non-loopback custom endpoints are not required to be HTTPS.** The four
`cleartext-transmission` alerts are by-design, but nothing rejects an `http://`
base URL for a **remote** host when an API key is attached. Loosening nothing and
blocking nothing, this is a stated caveat rather than a finding, and it is the one
item here where the alert is pointing at a real (accepted) trade-off rather than
at noise.

## 6. What was deliberately not done

- **No alerts were dismissed.** Dismissal is a repository mutation and would hide
  the backlog on a mirror fork without improving upstream, where the code lives.
  Note that a dismissal is also *not* obviously correct for the `cleartext-logging`
  cluster: the rule is over-tainting, but it is over-tainting a category that
  genuinely can leak, so the honest disposition is "the rule needs a tighter
  model", not "this alert class is worthless".
- **No `.github/codeql/codeql-config.yml` was added** to exclude test paths, though
  45 of 166 alerts and the single largest named source (`oauth_app`) come from
  tests. Excluding test trees would have suppressed the largest false-positive
  cluster at the cost of never scanning test code, which is a real if modest
  loss. It is worth proposing upstream only with that trade-off stated.
- **The `oauth_app` taint was not "fixed"** by editing the test. The test is
  correct; the classifier is over-broad.

## 7. Recommended next actions

| Priority | Action | Gate |
| --- | --- | --- |
| 1 | Upstream PR: `permissions: contents: read` on `freebsd-smoke.yml`'s `smoke` job | standing `pr/*` approval |
| 2 | Upstream: rename or document `sanitize_secret_value` | standing `pr/*` approval |
| 3 | Optional: propose a CodeQL config with the test-path trade-off stated | upstream discussion first |
| 4 | Operator-only: decide whether to dismiss the fork's 166 alerts as triaged | repository mutation |
