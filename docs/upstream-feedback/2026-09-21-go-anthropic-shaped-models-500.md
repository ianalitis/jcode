# `opencode-go` Anthropic-shaped models fail with an opaque provider 500

Date: 2026-09-21. Status: locally reproduced against the live provider. Not yet
published upstream. Runtime reported `v0.84.255-dev (97a62b6b5)`; canonical
source at observation `11ab2d532` (pre-upstream-merge tip).

## Expected and observed

`jcode model list -p opencode-go` lists every model OpenCode Go advertises,
including the MiniMax and Qwen families. Selecting one of those models sends the
request to the OpenAI-compatible chat-completions endpoint:

```sh
jcode run --no-update --socket /tmp/jcode-minimax.sock -p opencode-go \
  -m minimax-m2.7 'reply with the single word: ok'
```

Observed:

```text
Error: OpenAI-compatible chat request failed
  endpoint: https://opencode.ai/zen/go/v1/chat/completions
  model: minimax-m2.7
  auth: OPENCODE_GO_API_KEY
  status: 500 Internal Server Error
  response: {"type":"error","error":{"type":"error","message":"Internal server error"}}
```

Expected: either a successful response, or a local, named error such as
"`minimax-m2.7` is served on the Anthropic-shaped Go endpoint, which this
profile does not implement". A provider 500 is indistinguishable from a
transient outage, so callers retry and report the wrong cause.

`glm-5.3-flash` through the same profile returns normally, so the credential,
network path, and session header are all fine.

## Cause

OpenCode Go publishes a per-model endpoint table
(`https://opencode.ai/docs/go`, fetched 2026-09-21):

| Model family | Go endpoint | SDK shape |
| --- | --- | --- |
| GLM, Kimi, MiMo, LongCat, DeepSeek, Hy3 | `/zen/go/v1/chat/completions` | OpenAI-compatible |
| MiniMax (M2.5/M2.7/M3), Qwen3.6/3.7/3.8 | `/zen/go/v1/messages` | Anthropic |

The `opencode-go` profile is a single OpenAI-compatible profile, so every model
id is routed to `chat/completions`. The model list is populated from the live
catalog, which does not carry the endpoint shape, so the picker offers ids the
profile cannot serve.

## Proposed fix

One of:

1. Split the profile: keep `opencode-go` for chat-completions models and add an
   Anthropic-compatible sibling for the `/messages` family, with model
   discovery filtering ids the selected profile cannot serve.
2. Carry the per-model endpoint shape from the Go catalog (or a small static
   family table) into route selection, and choose the client per request.
3. At minimum, fail closed locally for known `/messages` ids with a named error
   instead of sending a request that returns 500.

## Evidence

- Live call output above; `https://opencode.ai/zen/go/v1/models` lists both
  families in one list and does not expose the endpoint shape.
- Fetched provider table: `https://opencode.ai/docs/go` (2026-09-21), section
  "Endpoints".
- Local routing receipt: `~/dotfiles/docs/measurements/2026-09-20-opencode-go-glm-reroute.md`
  records the surrounding reroute and the same chat-completions assumption.
