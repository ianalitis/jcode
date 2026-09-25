# Voice contribution, model routing overhaul, fork steering (2026-09-25)

Operator request (06:23Z): refine the voice feature and contribute it without feature
creep; steer fork, upstream-sync and stale-PR work; overhaul model delegation with
primaries and fallbacks per specialty and complexity, kept current as models and prices
change weekly; decide on OpenRouter routers, our own router, or a gateway such as
Cloudflare AI Gateway. Admission, freeze and receipts follow `docs/HARNESS_LOOP_ARCHITECTURE.md`.
Nothing here authorizes a push, PR, issue, install, credential or provider change.

## 1. Voice: what is native, what is ours

| Piece | Owner | Where |
| --- | --- | --- |
| `[dictation]` contract (run any command that prints a transcript, key toggles it) | **upstream, native** | `crates/jcode-base/src/config*`, `alt+;` in `~/.jcode/config.toml` |
| `turn_end` hook with reply text | **upstream, native** | hooks config |
| Gateway (WebSocket, pairing, Tailscale) + iOS app | **upstream, native** | `crates/jcode-base/src/gateway/`, `ios/` |
| Nari cloud voice | upstream, not admitted (cloud audio, metered) | |
| `jvoice` (SpeechAnalyzer STT, 324-line Swift), `jvoice-speak` (Kokoro briefs), `!voice` modes | **ours** | `~/dotfiles/home/dot_local/private_share/jvoice/`, `home/dot_local/bin/` |
| iOS hold-to-talk + spoken briefs | **ours**, 1 commit on upstream | branch `jcode/ios-voice` `e6c6b3e65`, local only |

Headless shape (Mac mini or Hetzner): speech stays on the edge devices (Mac `jvoice`,
iOS on-device SpeechAnalyzer), the server only runs jcode + the gateway and exchanges
text. A Linux box therefore needs no audio stack, which is why this split is right.

### Contribution plan, no creep

| # | Item | Why upstream | Size | Gate |
| --- | --- | --- | --- | --- |
| V1 | Gateway `/pair` attempt limit: invalidate pending codes after N failed guesses | Security defect: 6-digit code, 5-min TTL, unlimited guesses (`registry.rs:64`). No existing issue | small | issue first, then PR |
| V2 | iOS hold-to-talk + spoken brief | Already contribution-shaped (79/79 `swift test`) | medium | needs Xcode build + device trial (handoff §3), then issue + PR |
| V3 | Docs: "bring your own dictation command" example using the `[dictation]` contract | Upstream contract is generic; a documented example is the whole value | tiny | after V2 |
| - | `jvoice` binary, Kokoro briefs, `!voice` modes | **Stay in dotfiles.** macOS-only Swift + local TTS server; the upstream contract already makes them pluggable | - | - |

Explicitly out: a native `/voice` command, server-side audio streaming, realtime
speech-to-speech. Revisit only with a measured need from the ADR 033 trial.

## 2. Model routing overhaul

### Decision

1. **Do not build our own router.** Selection logic is small (a table); the hard parts
   are spend enforcement, observability and failover, which a gateway already does.
2. **Cloudflare AI Gateway as the single metered egress.** Every *metered* call (OpenRouter,
   Zen overflow, any future API key) goes through one gateway. Verified 2026-09-25 from
   Cloudflare docs: spend limits block with 429 per provider, model or custom metadata
   (for example `agent_id`, `lane`), work with BYOK, and can fall back to a cheaper model via
   dynamic routing; OpenRouter is a supported provider endpoint. Caveats from the same page:
   enforcement is eventually consistent (concurrent bursts can overshoot), cost is estimated,
   max 20 rules per gateway. This closes the standing "no per-key cap" blocker
   (`policy/providers.md`) better than a capped OpenRouter key alone, and applies to all
   harnesses and apps, not just jcode.
3. **Subscription OAuth lanes stay direct** (ChatGPT, Claude, Go). They are tied to their
   clients and quota windows; proxying them adds ToS risk for no enforcement gain. Their
   budget signal stays `jcode usage --json`.
4. **`openrouter/auto` and friends stay discovery lanes** until the A/B/C arms in
   `2026-09-24-ROUTING_AND_DELEGATION.md` measure them. They route through the gateway so
   their spend is capped even while unadopted.
5. **Self-hosted fallback if Cloudflare falls short** (logging, data class, latency):
   Bifrost or LiteLLM on the Mac mini, same role. Reported 2026 supply-chain incident on
   LiteLLM's PyPI package is unverified here; pin by hash if chosen.

### Lane matrix (one registry, primaries + ordered fallbacks)

One machine-readable file, `~/dotfiles/config/model-lanes.v1.json`, keyed by task class
and complexity, rendered into `policy/providers.md` and jcode config (`agents.swarm_model`,
`provider.quota_fallback`, per-spawn model choice). Starting content is today's policy:

| Class | Complexity | Primary | Fallback 1 | Fallback 2 |
| --- | --- | --- | --- | --- |
| Captain / architecture / security | hard | Astra high (OAuth) | Opus 5 (OAuth, while quota lasts) | stop and report |
| Scoped frontier impl / review | medium-high | Terra (OAuth) | Astra | stop |
| Routine impl, tests, fixes | low-medium | Go DeepSeek V4.1 Flash | Go GLM-5.3 Flash | Zen overflow via gateway |
| Hard iteration / escalation | medium-high | Go GLM-5.3 Flash | Terra | stop |
| Readers, verifiers, triage | none-low | Go MiMo-V2.5 | local mlx (advisory only) | Go DeepSeek |
| Long-context review | medium | Go Qwen3.7-plus | Terra | stop |
| Extraction, fixtures (public data) | low | OpenRouter DeepSeek 0731 via gateway | Go DeepSeek | stop |
| Local advisory (classify, rerank) | none | mlx-serve | none | - |

Fallbacks are **declared and visible**, never silent (policy): a quota or failure stop
records the escalation; the next spawn picks the next row entry.

### Keeping it current (weekly, proposal-only)

A scheduled weekly job (jcode `ScheduleWakeup` or launchd on the mini) that:
1. pulls OpenRouter `/models` (price, context), the Go/Zen catalogs, `jcode usage --json`
   windows, and new benchmark receipts under `docs/measurements/`;
2. writes a **proposed diff** to `model-lanes.v1.json` with the evidence per changed row;
3. never promotes: availability and vendor benchmarks are priors, adoption needs a
   task-class measurement or an operator yes (ADR 061).

### Upstream-worthy piece

Jcode has one `swarm_model` default plus per-spawn overrides and `quota_fallback`. A small,
generic `[agents.lanes]` table (class -> ordered model list) that the swarm tool resolves
by label would remove our per-spawn bookkeeping. Issue first, no PR until upstream agrees.

## 3. Fork and upstream steering (ordered)

| # | Item | State | Next |
| --- | --- | --- | --- |
| F1 | Commit H3 test-home isolation + U6 labeler | done, uncommitted: commit signing blocked | operator runs `ssh-add --apple-use-keychain ~/.ssh/id_ed25519` |
| F2 | Upstream H3 as a PR (root fix supersedes #1487/#1489 per-suite patches) | ready after F1 | issue + PR, then close #1487/#1489 as superseded if upstream prefers |
| F3 | #1354 | code-complete, CI red only on the SSH-secret step for fork PRs | comment explaining the infra failure; ask for a maintainer rerun |
| F4 | #1356 conflicting, #1357, #1362 mergeable, stale 3+ days | | rebase #1356 (Terra after 09-27), ping the other two once |
| F5 | 22 open issues, none fixed upstream (U4) | | close none; batch-comment only where new evidence exists |
| F6 | Gateway pairing limit (V1) | new | issue |
| F7 | `fork/master` 22 ahead of upstream; integration line 365 ahead | | keep; re-run `scripts/branch_ledger.sh` after F1-F2 land |
| F8 | Capped metered spend | blocked on a key | stand up the Cloudflare gateway (operator: account + token), then move OpenRouter traffic behind it |
