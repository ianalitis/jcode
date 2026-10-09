# Jev paid-vs-free routing, settled with live evidence, 2026-10-09

Fork HEAD `df9792506` (`jcode/ci-format-baseline`). Binary under test:
`target/selfdev/deps/jcode_base-343a1efc98042d06` (prebuilt, matches HEAD).

This receipt settles the open question "should Jev route through the free Zen
model or a paid route?" with live probes rather than policy reading. All probes
used the synthetic acceptance payload from `memory_jev::tests` (no private
data).

## 1. Route inventory as configured

`crates/jcode-base/src/jev.rs` `resolve_with`:

- `auto` -> `resolve_providers(&[JevProvider::Jcode, JevProvider::TypeSafe])`,
  and `JevProvider::resolve_providers` appends the remaining credentialed
  providers in doc order: Jcode, TypeSafe, OpenRouter, AI/ML API. Zen is
  explicit-only.
- `~/.jcode/config.toml`: `memory_jev_provider = "auto"`,
  `memory_jev_threshold = 0.8`.

Credentials actually present on this machine (env files live in
`~/Library/Application Support/jcode`, not `~/.jcode`):

| Provider | Credential file | Present |
| --- | --- | --- |
| `jcode` (subscription) | `jcode-subscription.env` | **no** |
| `typesafe` | (none) | **no** |
| `openrouter` | `openrouter.env` | yes |
| `aimlapi` | (none) | **no** |
| `opencode` / `opencode-free` | `opencode.env` | yes |

`~/.jcode/auth.json` holds only `anthropic_accounts`. `jcode usage -p
openrouter` reports OpenCode Go (API key) and OpenAI (ChatGPT) valid. The Jcode
subscription `/me` block that reports "Memory recall (daily)" / "Browser
automation (daily)" allowances
(`crates/jcode-base/src/usage/provider_fetch.rs:891`) never renders here,
consistent with no subscription credential.

## 2. Live probes

`JCODE_MEMORY_JEV_LIVE_TEST=1 <binary> live_synthetic_relevance_acceptance
--nocapture --ignored`, three windows, expected selection counts 1/0/1.

| Selector | Result | Latency |
| --- | --- | --- |
| `auto` (configured) | ok, `provider=openrouter`, counts 1/0/1 | 250-916 ms |
| `opencode` (Zen paid, `jev-1.13`) | ok, `provider=opencode`, counts 1/0/1 | 295-792 ms |
| `opencode-free` (Zen free, `jev-1.13-free`) | **HTTP 400** | n/a |

The free-route 400 is model-specific, not a credential or endpoint problem:
`GET https://opencode.ai/zen/v1/models` with the same key returns 200 and lists
45 models including **both** `jev-1.13-free` and `jev-1.13`, while
`POST /v1/systemone` with the identical key, endpoint and payload succeeds for
`jev-1.13` and fails for `jev-1.13-free`. The only difference is the model id,
so the free model is offered to the workspace but refused at request time, which
matches the code's stated cause: the workspace privacy setting that admits free
endpoints is off.

So on this machine `auto` = **OpenRouter BYOK** (`typesafe/jev-1.13` via the
OpenRouter Decisions API), and the free Zen route is **not currently usable**
without an operator changing the Zen workspace setting.

## 3. Cost of the paid routes

Jev is a decision model, not a chat model: $0.042 per 1M input tokens, output
free (TypeSafe, OpenRouter and Zen all publish the same price; OpenRouter P50
latency 0.17 s, 3-day uptime 99.29%).

A memory-recall decision sends roughly 1-3k input tokens:

- 1 decision ~= $0.0001
- 1,000 decisions ~= $0.04-0.08
- 100 recalls/day for a month ~= $0.25

The free route therefore saves on the order of **eight cents per thousand
decisions**. That is not a measurable saving, and it buys a workspace consent
setting plus a model Zen documents as "available for a limited time".

## 4. Decision

1. **Keep `auto`.** It already prefers the included Jcode subscription route
   when that credential exists, then TypeSafe, then OpenRouter. On this machine
   it resolves to a working paid BYOK route with no config change needed.
2. **Do not adopt `opencode-free` for cost reasons.** At $0.042/M input the
   saving is ~$0.08 per 1,000 decisions; the free model is limited-time and
   currently refused by the workspace setting. Keeping Zen explicit-only (as
   `resolve_with` already does, because its key is shared with chat models) is
   correct.
3. **If zero metered spend is wanted**, the only real levers are (a) log in to
   the Jcode subscription, which moves `auto`'s first choice ahead of
   OpenRouter with no config edit, or (b) enable Zen's free-endpoint workspace
   setting. Both are explicit operator decisions, not routing defaults.

## 5. Open discrepancy (recorded, not silently changed)

`crates/jcode-base/src/jev.rs:109`, `:746-751` and
`crates/jcode-base/src/config/default_file.rs:529-530` state that Zen's
free-endpoint setting "permits training on request data". Zen's published
privacy page (last updated 2026-10-08) lists per-provider terms and says for
Jev: "Prompts and other inputs are not used for training. Data is retained in
accordance with TypeSafe AI's Privacy Policy." The free-model entries that do
train (Big Pickle, Exo, MiMo, Ling, Muse Spark Contributor) each say so
explicitly; `Jev 1.13 Free` carries no such caveat.

Two halves, different evidence states:

- The **workspace gate** half is confirmed: the free model is listed for this
  key yet returns 400 (section 2).
- The **training** half is unverified against the console wording and
  contradicted by the published policy.

No code text was changed. The current wording errs toward warning the operator
about a consent decision, which is the safe direction, and the authoritative
wording lives in Zen's authenticated console that was not driven for this
receipt. Proposed replacement if the operator confirms the policy reading:
"Zen serves jev-1.13-free only when the workspace privacy setting allows free
endpoints; Zen's free-model data terms apply (see Zen's privacy policy)."

## Reproduce

```bash
B=target/selfdev/deps/jcode_base-343a1efc98042d06
JCODE_MEMORY_JEV_LIVE_TEST=1 ./scripts/bounded.sh 180 "$B" \
  live_synthetic_relevance_acceptance --nocapture --ignored
JCODE_MEMORY_JEV_LIVE_TEST=1 JCODE_MEMORY_JEV_PROVIDER=opencode \
  ./scripts/bounded.sh 180 "$B" live_synthetic_relevance_acceptance --nocapture --ignored
JCODE_MEMORY_JEV_LIVE_TEST=1 JCODE_MEMORY_JEV_PROVIDER=opencode-free \
  ./scripts/bounded.sh 180 "$B" live_synthetic_relevance_acceptance --nocapture --ignored
KEY=$(sed -n 's/^OPENCODE_API_KEY=//p' "$HOME/Library/Application Support/jcode/opencode.env")
curl -sS https://opencode.ai/zen/v1/models -H "Authorization: Bearer $KEY"
```
