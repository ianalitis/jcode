# 2026-09-25 OpenRouter content-filter false positive: A1 applied

Plan: `docs/plans/2026-09-25-ARCHITECT_ASSESSMENT_AND_PHASED_PLAN.md` section 3 (root cause) and
phase 1 node G1/G6. Operator approval A1 granted 2026-09-25.

## Change

Workspace default guardrail `1c7da278-dad7-552d-ab12-97d5bf2e34df`, `PATCH /api/v1/guardrails/{id}`
with the provisioning key read from Keychain over stdin (never printed).

| Field | Before | After |
| --- | --- | --- |
| builtin `credit-card` | block | **flag** |
| builtin `ssn` | block | block |
| builtin `secrets` | redact | redact |
| builtins `regex-prompt-injection`, `email`, `phone`, `ip-address` | flag | flag |
| `content_filters` | none | 3 x `[PAN]` block (below) |
| `limit_usd` / `reset_interval` | 20 / daily | 20 / daily |

The only fields that changed were `content_filter_builtins` and `content_filters`
(diffed from full before/after GET snapshots).

Custom block patterns (issuer prefix, contiguous or dash-grouped only):

```
\b(?:4\d{15}|5[1-5]\d{14}|2[2-7]\d{14}|6011\d{12}|65\d{14}|3[47]\d{13})\b
\b(?:4\d{3}|5[1-5]\d{2}|2[2-7]\d{2}|6011|65\d{2})-\d{4}-\d{4}-\d{4}\b
\b3[47]\d{2}-\d{6}-\d{5}\b
```

## Deviation from the plan

The plan's single pattern was rejected with HTTP 400 `guardrails_invalid_regex_pattern`
("catastrophic backtracking ... nested quantifiers") because of `(?:-\d{4}){3}` (a quantified group). It was split into the three flat patterns above with
the same match set. No partial state was written: a GET after the failed PATCH showed no
changed fields.

## Validation

- Local regex tests (Python `re`): not matched: `for n in 1113 1114 1115 1116`,
  `1487 1489 1354 1356`, `1000 1200 1366 1440`, a hex sha, a 16-digit timestamp starting
  `17`, and space-grouped `4242 4242 4242 4242` (deliberately left to the G2 local Luhn
  check). Matched: Visa, Visa dashed, Mastercard, Mastercard 2-series, Discover, Amex,
  Amex dashed test numbers.
- Live, `jcode run --provider-profile cf-openrouter -m deepseek/deepseek-v4-flash-0731`:
  - prompt containing `for n in 1113 1114 1115 1116` returned `OK` (previously 403 `[CREDIT_CARD]`).
  - prompt containing the Stripe test PAN `4242424242424242` returned 403
    `Request blocked by content filter: [BLOCKED]`, so real card shapes still fail closed.

## G3 (commit `5feca93e6`)

Root cause of the repeated sends, from `~/.jcode/logs/jcode-2026-09-25.log`: every block
was `API stream attempt 1/8`. The transport layer already refused to retry a 403. The
repeats were new zero-content client messages, one every 2 to 7 seconds: the TUI
auto-retry continuation (`schedule_pending_remote_retry`, 3 attempts) resending a turn
the guardrail would always block. `previous_errors` (24 entries) is OpenRouter's own
fan-out across providers, not a harness retry.

- `jcode_provider_core::content_filter_block_label` recognises `blocked by content filter: [LABEL]`.
- `remote/terminal_errors.rs` holds the fail-fast branch, next to the #387 model/endpoint
  case: clear the retry, stop auto-poke, show `Not retrying ... [LABEL]` with what to do,
  and offer a route switch.
- `openrouter_sse_stream::should_retry`: 429s with `limit_source=upstream_provider_shared_pool`
  retry at most twice.
- Tests: `test_remote_content_filter_block_fails_fast_without_retry_budget`,
  `content_filter_block_label_reads_the_openrouter_entity`,
  `shared_pool_429_retries_at_most_twice`, `content_filter_block_is_never_retried`.
  `jcode-provider-core` 146 passed, `jcode-provider-openrouter-runtime` 191 passed,
  clippy `-D warnings` clean on all three crates.
- Existing, unrelated failures: `test_model_picker_remote_comtegra_model_uses_comtegra_route_not_copilot`
  and `test_remote_current_fpt_live_model_uses_fpt_route_not_copilot_without_cache` fail
  on clean HEAD `a95382205` too, even with an empty `JCODE_HOME`: a Copilot route wins over
  the provider-specific route. They need their own reproduction.
- The code-size budget already reports `openrouter-runtime/src/lib.rs` and
  `openrouter_provider_impl.rs` as over budget at HEAD `3be93ab50`. This change did not
  touch either file.

## Still open (phase 1)

- G2: local pre-send PAN plus Luhn check with message index, which also covers space-grouped PANs.
- G1: fold `scratch/or_guardrail_fix.sh` into `~/dotfiles/scripts/openrouter-admin.sh`
  (`guardrails`, `guardrail-set`, `--yes` gating), then I1 manages it as code.
