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

## Still open (phase 1)

- G3: the blocked request shows 24 `previous_errors` entries for one turn. OpenRouter retries
  a deterministic block across providers, and the harness then adds its own retries. A 403
  `content_filter` must not be retried and should surface the entity type.
- G2: local pre-send PAN plus Luhn check with message index, which also covers space-grouped PANs.
- G1: fold `scratch/or_guardrail_fix.sh` into `~/dotfiles/scripts/openrouter-admin.sh`
  (`guardrails`, `guardrail-set`, `--yes` gating), then I1 manages it as code.
