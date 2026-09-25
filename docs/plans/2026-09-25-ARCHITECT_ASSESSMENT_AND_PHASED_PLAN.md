# Fable 5.1 architect assessment and phased plan for the Opus 5.5 implementation agent (2026-09-25)

**Author:** Fable 5.1 (Claude OAuth), chief architect and analyst for the harness source
configuration. **Requested by:** operator, 2026-09-25 15:20Z, with explicit authorization to
identify and address the OpenRouter blocking errors, any other hanging work, and anything
preventing regular automatic upstream sync plus continued contributions.

A dated snapshot, not standing authority. Admission, freeze, receipt and gate follow
`docs/HARNESS_LOOP_ARCHITECTURE.md`. Nothing here overrides `policy/global.md` hard stops:
pushes, PR mutations, credential or provider changes, installs, and daemon promotion each
still need their named approval, and section 7 lists the ones this plan requests.

## 1. Headline findings

1. **There is no credit card number in the fork, the config, the memory store, the skills,
   or any blocked session.** The "[CREDIT_CARD]" 403s come from the OpenRouter workspace
   default guardrail (`Workspace … Default`, builtin `credit-card` in **block** mode), whose
   builtin detector matches any four groups of four digits separated by spaces. Every one of
   the 27 blocks in the logs (2 on 09-20, 16 on 09-24, 9 on 09-25) correlates with a prefix
   that contains a list of issue or PR numbers or a viewport-width list such as
   `1113 1114 1115 1116`, `1487 1489 1354 1356`, `1319 1328 1334 1312`, `1000 1200 1366 1440`,
   or a hexdump line `2020 2020 2020 2020`. None is Luhn-valid. Sessions with zero such
   groups (the successful `penguin` runs on the same route) were never blocked. Evidence and
   method are in section 3.
2. The far larger OpenRouter disruption on 09-24 was **394 upstream 429s** (`glm-5.3-flash`
   and `deepseek-v4.1-flash` "temporarily rate-limited upstream", `limit_source:
   upstream_provider_shared_pool`, providers Crusoe/BaseTen/Together/Decart/CoreWeave) and
   **16 Together `502 Invalid URL`** responses, all on `openrouter/auto` and `auto-beta`.
   The routers pick shared-pool endpoints that are throttled; the blocks were 6% of the
   failures.
3. The fork's operational state is healthy: integration line `jcode/ci-format-baseline` at
   `3be93ab50`, clean tree except the untracked `scripts/local_model_probe.py` (not ours),
   write lease free, 5 upstream commits behind (`74577fe83`), 370 ahead, 9 open upstream PRs
   all `MERGEABLE`. The remaining hanging nodes from the 09-24 plan of record are listed in
   section 4 with what changed since.
4. The main throughput blocker for an implementation agent is not code: it is
   `commit.gpgsign=true` with an SSH signing key that is now loaded (`ssh-add -l` shows the
   ED25519 key), so commits work again in this session; the 06:20Z status note about the
   locked key is resolved. Confirm at the start of each session with `ssh-add -l`.
5. Infra management is already agentic in shape (Keychain-only credentials,
   `scripts/infra-cred.sh`, read-only Pulumi estate stack, `scripts/openrouter-admin.sh`,
   Cloudflare AI Gateway with spend caps). The gaps are: OpenRouter guardrails and keys are
   read-only from the CLI, the estate stack has no OpenRouter or Cloudflare AI Gateway
   provider, and nothing renders the lane table into the guardrail. Section 6 makes that
   the third phase.

## 2. Current state (measured 15:20Z-15:38Z)

| Ref | Value |
| --- | --- |
| Integration line `jcode/ci-format-baseline` | `3be93ab50` "fix(openrouter-runtime): send named provider profile headers" (PR #1496) |
| `origin/master` | `74577fe83`, 5 ahead of our merge base (`applet-types`, `protocol reasoning effort on model_changed`, hooks Windows fix #1490, star chart) |
| Worktrees | `data-class-admission` (`77cdac8ff`), `ios-voice` (`e6c6b3e65`), `privacy-no-collection` (`c81735fdf`) plus the main checkout |
| Stashes | none |
| Write lease | free |
| Shared daemon | `3be93ab50-dirty-f507f8f6c2c4` (matches HEAD) |
| Open upstream PRs (all MERGEABLE) | #1354 lint drift, #1356 test-env lock, #1357 bg status atomic writes, #1362 swarm stop quiesce, #1487 copilot test home, #1489 communicate test home, #1493 test home sandbox (H3 root fix), #1494 gateway pairing limit (V1), #1496 openai-compatible profile headers |
| OpenRouter workspace guardrail | `limit_usd: 20 daily`; builtins: regex-prompt-injection flag, email flag, phone flag, ssn **block**, credit-card **block**, ip-address flag, secrets redact; no custom filters, no ZDR flag on the guardrail object |
| Cloudflare AI Gateway `jcode-metered` | live, `cf-openrouter` profile in config, `cf-aig-collect-log-payload=false`, metadata `agent_id=jcode`; 2 gateway 429s on 09-25 (the caps working) |
| Commit signing | SSH key loaded in agent this session |

## 3. OpenRouter blocking errors: diagnosis and evidence

### 3.1 Method

- Pulled every `openrouter` error line from `~/.jcode/logs/jcode-2026-09-2{0,1,2,4,5}.log`
  and bucketed by error body. Counts above.
- For each `[CREDIT_CARD]` 403, took the session id and timestamp from the preceding
  `Processing task completed with error` line, loaded the session file, kept only messages
  before the block, and scanned message *content* (ids and timestamps excluded) for:
  4x4 digit groups, 13-19 digit runs, Amex/Visa/MC prefixed runs, all with a Luhn check.
- Captured a real outbound request body by pointing a copy of the real config at a local
  fake endpoint (`~/.jcode/scratch/capture.py`, `capture_run2.sh`, request in `req3.bin`):
  system prompt, swarm prompt, skills and tools contain **no 12+ digit run at all**.
- Read the workspace guardrail through the provisioning key (`GET /api/v1/guardrails`).

### 3.2 Result table (content before the block; message ids excluded)

| Session | Block (UTC) | 4x4 groups found | Luhn-valid | Example context |
| --- | --- | --- | --- | --- |
| sauropod | 09-20 10:02, 15:17 | 2 | no | hexdump `00000000: 2020 2020 …` while investigating a flagged line |
| buffalo | 09-24 21:07-21:28 (x5) | 3 | no | `bun run measure.ts 320 360 768 1000 1200 1366 1440` |
| giraffe | 09-24 21:14, 21:28 | 0-1 | no | `for l in 188 2007 6166 6758 6823` (loop over line numbers) |
| piglet | 09-25 00:25, 00:41 | 7 | no | `for n in 1113 1114 1115 1116 …` (U4 issue recheck) |
| humpback | 09-25 00:50 | 1 | no | `for n in 1487 1489 1354 1356` |
| guppy | 09-25 00:57, 01:30 | 1-2 | no | `for p in 1489 1487 1362 1357`; later the literal Stripe test card while investigating this very block |
| dog | 09-25 07:00, 07:06 (x7) | 10 | no | `Issues: 1110 1112 1113 1114 …` |
| shark | 09-25 08:47 (x2) | 1 | no | `for n in 1319 1328 1334 1312 1314 1315` |
| **control** penguin (901 and 693 msgs, same route, never blocked) | | 0 | | |

The only Luhn-valid 13-19 digit runs anywhere are millisecond timestamps and u64 hashes
inside message/memory ids, which are not sent as content. The only real card-shaped
literal in the repository is the Stripe public test number in
`crates/jcode-app-core/src/tool/discover_tests_body_tests.rs:585`, a fixture for the
discovery tool's own PII detector; it is not in any prompt path.

### 3.3 Why it presents as "a credit card number that somehow ended up in here"

The guardrail's error string names the entity type, not the match, and OpenRouter's
builtin uses a shape regex without Luhn. Our repository work naturally produces
four-digit issue numbers in runs of four. Once such a line is in a session's history it
is resent on every turn, so the session is blocked until it compacts or the route changes,
which is exactly the "keeps hitting it" pattern reported.

### 3.4 Remediation (phase 1, section 6)

- Set the workspace default guardrail's `credit-card` builtin to **flag** (keep `ssn` block,
  keep `secrets` redact) through the management API, and add a **custom** regex that blocks
  only real card shapes. Luhn cannot be expressed in regex, so the server-side filter
  matches issuer prefix plus a contiguous or dash-grouped body only (tested against every
  blocked line above and the Stripe/Amex/MC test numbers, `~/.jcode/scratch/or_guardrail_fix.sh`):
  `\b(?:(?:4\d{3}|5[1-5]\d{2}|2[2-7]\d{2}|6(?:011|5\d{2}))(?:\d{12}|(?:-\d{4}){3})|3[47]\d{2}(?:\d{11}|-\d{6}-\d{5}))\b`.
  Space-grouped PANs are deliberately left to the local pre-send check with Luhn, because
  that is the exact shape issue lists take. The three tests: `for n in 1113 1114 1115 1116`
  is not blocked, `4242424242424242` and `4111-1111-1111-1111` are, and a key-shaped
  string is redacted.
- Add a local outbound pre-check to the OpenRouter/gateway request path that runs the
  same detector with Luhn before send and, on a real match, fails closed with a message
  naming the message index, so the operator learns *which* content tripped it rather
  than reading an opaque 403. Same rule as `scripts/pii-scrub.py`: report location, never
  the value.
- On a 403 `content_filter` response, the provider runtime should surface the
  `matched_entity_types` and the pipeline summary from `openrouter_metadata` in the error
  text, and must **not** retry (the log shows 8 attempts per turn against a deterministic
  block, and `previous_errors` growing per attempt).

## 4. Hanging work inventory and disposition

From `docs/plans/2026-09-24-PHASE_PLAN_POST_088.md` and
`docs/plans/2026-09-25-VOICE-ROUTING-AND-FORK-STEERING.md`, rechecked against HEAD and
GitHub:

| Node | State at 15:38Z | Disposition |
| --- | --- | --- |
| U1, U2, H4, H3 root fix, V1 | done: #1489, #1354 head `07db0864d`, #1493, #1494 | keep the PRs alive (section 5) |
| U3 rebase 1354/1356 | both now MERGEABLE (upstream moved) | nothing to do until a review lands |
| U4 issue recheck against 0.88 | done, 22 open, none fixed upstream | the U4 run is what produced the `for n in 1113 …` lines that got blocked; keep it, it is a valid read |
| U5 browser auto-detect issue | soak still running (guard landed 09-24) | file 2026-10-01 |
| U6 jev-pr-labeler skip-without-key | branch in `~/.jcode/scratch/jpl`, tests 116/116, unpushed | signing is unblocked: push + PR (approval item A3) |
| H1 split `browser.rs` / `jev.rs` | not started | Go/medium once the Go window is confirmed; otherwise Opus 5.5 |
| H2 TUI/SDK test races | not started; #1356 is the bounded-wait half | after #1356 review |
| F2 upstream H3 root fix | done as #1493 | ask upstream whether #1487/#1489 should close as superseded once #1493 merges |
| F3 #1354 red on SSH-secret step | infra-only red on fork PRs | comment once (approval A4) |
| F5 22 open issues | no action | batch-comment only with new evidence |
| F7 fork/master 22 ahead | unchanged; integration line now 370 ahead | re-run ledger after phase 2 |
| F8 capped metered spend | **done differently**: Cloudflare AI Gateway caps + workspace guardrail `limit_usd 20/day` | close F8; record in `policy/providers.md` that OpenRouter's own workspace guardrail is now the second cap |
| M1 jcode-bench baseline, M2 routing arms | not started | after phase 3; needs quota ≥40% |
| Firefox retirement | in `~/.Trash/jcode-firefox-retire-20260924/` | operator empties Trash to finalize |
| `worktrees/data-class-admission`, `ios-voice`, `privacy-no-collection` | present, not merged | ios-voice is V2 (needs Xcode device trial); the other two need a fresh ledger row before any decision |

## 5. Other errors and warnings that disrupt operations

Ranked by impact on an implementation agent.

| # | Symptom | Count (09-25 log) | Cause | Fix |
| --- | --- | --- | --- | --- |
| E1 | `LaTeX image rendering fell back to Unicode: DVI renderer failed (start latex: No such file)` and `LaTeX source is empty` | 13,410 lines on 09-25, 17,971 on 09-24 | the TUI probes for `latex`/`pdflatex` on every math block render and logs at WARN each time | probe once per process, cache the absence, log once at INFO; skip the renderer entirely when the source is empty (upstream-worthy, small) |
| E2 | `Usage fetch error: Usage API error (429)` from Anthropic | 160 | usage poller has no backoff when the OAuth usage endpoint rate-limits | exponential backoff with jitter, cap at 15 min; treat a 429 as "window unknown", never as zero headroom |
| E3 | `openrouter/auto` upstream 429 shared-pool storms and Together `502 Invalid URL` | 394 + 16 on 09-24 | the routers select throttled shared-pool endpoints; `is_byok:false`; 8 retries per turn | pin `provider.order`/`ignore` in the `cf-openrouter` profile for the auto routes (ignore Together until the 502 is fixed upstream), and cap retries at 2 for a 429 whose `limit_source` is `upstream_provider_shared_pool` |
| E4 | `Refusing to persist an invalid remote model catalog` (20) and `handle_get_model_catalog: session busy` (15) | 35 | the picker hydration for `~openai/gpt-astra-latest` returns 0 providers on the cf-openrouter profile | skip catalog refresh for profiles with `model_catalog = false`; already declared in config, not honored by the hydration path |
| E5 | `MCP [context7] stderr: … running on stdio` / `MCP [lightpanda] stderr: config tips` at WARN | 288 | server banners logged at WARN | demote MCP stderr lines that are not errors to INFO |
| E6 | `jcode TUI requires an interactive terminal` (15) | 15 | scripts invoking `jcode` without `run` from non-TTY | make the error name the `run` subcommand; audit `~/dotfiles/home/dot_local/bin` callers |
| E7 | `Failed to send queued continuation message` (3), `Client error: Broken pipe` (3) | 6 | client detach during a queued continuation | existing known flake; keep the reproduction in this receipt, no change yet |
| E8 | `commit.gpgsign` waits on a locked SSH key (06:20Z status) | blocked commits and the unpushed U6 | key not in agent | loaded now; add `ssh-add -l` to the session preflight in `AGENTS.md` and fail fast with the exact `ssh-add --apple-use-keychain` command |
| E9 | Shared `CARGO_TARGET_DIR` across worktrees produces phantom E0599/E0609 | recurring | documented in the 09-24 handoff | give each worktree its own target dir in `scripts/dev_cargo.sh` |

Bedrock catalog refresh failures (2) are expected: no AWS credentials, and the provider is
not admitted.

## 6. Phased plan for the Opus 5.5 implementation agent

Every phase: one frozen treatment (`claude-oauth:claude-opus-5-5`, effort high for phase 1
and 2 judgment nodes, medium for the mechanical ones), one writer holding the lease,
`git commit --only -- <paths>`, narrow crate tests after each edit,
`scripts/check_guardrails.sh --skip-slow` before a phase closes. Workers, when the Go
window is confirmed by `jcode usage --json`, take only the nodes marked Go.

### Phase 0: preflight (captain, 15 min)

- `ssh-add -l`; `scripts/repo-lease.sh status`; `git fetch origin fork` bounded; recheck the
  counts in section 2; `selfdev status`.
- Merge `origin/master` `74577fe83` into the integration line (5 commits, applet-types is
  additive, expect no conflicts), `cargo check --workspace`, `selfdev build-reload`.
- Gate: daemon reports the merged hash; `cargo test -p jcode-app-core --lib` green.

### Phase 1: stop the guardrail false positives and make the failure legible (captain, high)

| Node | Writable paths | Gate | Validation |
| --- | --- | --- | --- |
| G1 `scripts/openrouter-admin.sh` grows `guardrails`, `guardrail-set <id> --builtin credit-card=flag …`, `guardrail-add-filter`, `keys-create --limit --reset`, `keys-delete`; every mutation prints the intended PATCH body and requires `--yes`; the key stays stdin-only | `~/dotfiles/scripts/openrouter-admin.sh`, `~/dotfiles/config/infra-credentials.v1.json` (`lastVerified`) | dry run shows the exact body; **operator approval A1** before the first `--yes` | `guardrails` output matches section 2 before; after: credit-card=flag, custom filter present |
| G2 Local pre-send PAN check in the OpenAI-compatible request path: issuer-prefix regex plus Luhn over `messages[*].content` text parts, fail closed with the message index and role, value never logged. Off for local/loopback bases | `crates/jcode-provider-openrouter-runtime/src/` (request build), one new `pan_check.rs` in `jcode-provider-core`, tests | unit tests: issue list passes, Stripe test card fails, hexdump passes | `cargo test -p jcode-provider-core -p jcode-provider-openrouter-runtime` |
| G3 403 `content_filter` handling: no retry, error text carries `matched_entity_types` and the pipeline summary; 429 with `limit_source=upstream_provider_shared_pool` retries at most twice | `openrouter_sse_stream.rs`, `openrouter_provider_impl.rs`, tests | the 09-25 log shape reproduces in a fixture and yields one attempt | crate tests |
| G4 Provider pinning for the auto routes in `cf-openrouter`: `provider.ignore=["Together"]` until the 502 clears, `allow_fallbacks=true`; documented in the profile comment | `~/.jcode/config.toml` via `scripts/render-model-lanes.py --write`, `config/model-lanes.v1.json` | render `--check` clean | one live `jcode run` on `openrouter/auto` through the gateway |
| G5 Policy: `policy/global.md` outbound-data rule gains one sentence: "four-digit identifier lists are not PAN; PAN is issuer prefix plus Luhn, and the harness checks that before send"; `scripts/pii-scrub.py` gains the same PAN rule with Luhn and a self-test | `~/dotfiles/policy/global.md`, `scripts/pii-scrub.py`, `scripts/generate-agent-surfaces.sh` output | `pii-scrub --self-test` passes | chezmoi apply regenerates `~/AGENTS.md` |
| G6 Receipt: `docs/measurements/2026-09-25-openrouter-content-filter-false-positive.md` with the table in section 3 and the before/after guardrail JSON (ids only) | `docs/measurements/` | committed | n/a |

G2 is upstream-worthy (issue first: "OpenAI-compatible providers: pre-send PAN check and
legible content_filter errors"). G1 and G5 are dotfiles.

### Phase 2: operations noise and throughput (Opus 5.5 medium; E1, E4, E5 are Go-eligible)

| Node | Writable paths | Validation |
| --- | --- | --- |
| N1 (E1) LaTeX renderer probe once per process, skip empty source, single INFO line | `crates/jcode-tui*/` math render module + tests | log line count for a 100-block render fixture is ≤1 |
| N2 (E2) usage poller backoff with jitter; 429 = unknown window | `crates/jcode-base/src/usage*` or wherever `Usage fetch error` originates | unit test on the backoff schedule |
| N3 (E4) honor `model_catalog=false` in picker hydration | catalog refresh path | no "Refusing to persist" line on a cf-openrouter session |
| N4 (E5) MCP stderr banner severity | MCP client logging | grep count |
| N5 (E9) per-worktree target dir | `scripts/dev_cargo.sh` | two worktrees build without phantom errors |
| N6 (E8) `ssh-add -l` preflight line in `AGENTS.md` "Development Workflow" | `AGENTS.md` | n/a |
| N7 H1 split `browser.rs` and `jev.rs` under 1200 lines, ratchet budgets | those files, `scripts/code_size_budget.json` | `cargo test -p jcode-base --lib`; budget script |

Each of N1-N4 is a small upstream PR (issue first per `CONTRIBUTING.md`).

### Phase 3: agentic infra management (captain-led design, Opus 5.5 implements)

Goal: the harness manages OpenRouter (keys, guardrails, credits), OpenCode Go/Zen
(catalog, key status), and Cloudflare AI Gateway (caps, metadata) from one interface, with
Pulumi as the record where a provider exists and scripts where it does not.

| Node | What | Where |
| --- | --- | --- |
| I1 OpenRouter in the estate stack | Pulumi has no OpenRouter provider; use a `pulumi.dynamic.Resource` (TypeScript) wrapping `/guardrails` and `/keys` with the provisioning key from `infra-cred.sh`. Resources: `WorkspaceGuardrail` (builtins, custom filters, `limit_usd`, `reset_interval`, ZDR flags), `InferenceKey` (name, limit, reset, guardrail assignment). Read-only `preview` first; `apply` behind `scripts/estate.sh` and ADR 063 | `~/dotfiles/infra/estate/` |
| I2 Cloudflare AI Gateway in the estate stack | `@pulumi/cloudflare` has AI Gateway resources in recent versions; verify the provider version supports `AiGateway` and the rate-limit/spend-limit rules, else dynamic resource over the REST API. Import `jcode-metered` and its caps | same |
| I3 Lane table → guardrail rendering | `scripts/render-model-lanes.py` grows a `--guardrail` target that emits the OpenRouter `allowed_models` list from `config/model-lanes.v1.json` so the workspace guardrail's allowlist matches the lanes; proposal-only by default (`--check`), apply through I1 | `~/dotfiles/scripts/` |
| I4 OpenCode Go/Zen | no management API is known; `jcode usage -p opencode-go` plus the console stay authoritative. Node: a read-only `scripts/opencode-go-status.sh` that probes the key and catalog and records the 09-27 pool re-verification | `~/dotfiles/scripts/` |
| I5 Jcode surface | a `jcode infra` (or `/infra`) read-only command that runs `estate-inventory`, `openrouter-admin all`, gateway status and prints one table; mutations stay in the scripts behind approval | `crates/jcode-cli` (fork-only until measured useful) |
| I6 Receipt + ADR | ADR 065 "OpenRouter and AI Gateway join the estate stack via dynamic resources"; measurement receipt for the first preview | `~/dotfiles/docs/decisions/`, `docs/measurements/` |

Order: I1 preview → A1-gated apply of the phase 1 guardrail change through I1 (so the
manual G1 fix and the IaC state agree) → I2 → I3 → I4 → I5 → I6.

### Phase 4: regular automatic upstream sync and contribution cadence

The posture (`docs/FORK_POSTURE.md`) already forbids automatic pushes and rebases, so
"automatic" means: automatic **fetch, merge-preview, gate, and proposal**, with one
approval per publish.

| Node | What | Gate |
| --- | --- | --- |
| S1 `scripts/upstream_sync.sh` | bounded `git fetch origin fork`; counts per the posture; `git merge-tree --write-tree origin/master HEAD` (bounded) to detect conflicts without touching the tree; if clean, create a temporary merge in a scratch worktree with its own target dir, run `cargo check --workspace` and `scripts/check_guardrails.sh --skip-slow`; write `docs/measurements/sync/<date>.md` with counts, conflict files, gate result, and the exact `git merge` command to run | runs read-only; never merges into the integration line itself |
| S2 Schedule | jcode `ScheduleWakeup` daily at 06:00 local targeting `spawn` with a `verifier` allowlist, or launchd on the Mac mini; the wake reads S1's receipt and reports in 3 lines; a clean receipt is the operator's "approve merge + push" prompt | one approval per publish (A5 standing pattern) |
| S3 Fork branch hygiene | after each merged/closed PR, `scripts/fork_branch_prune.sh` dry run in the same wake; deletion still needs approval | n/a |
| S4 PR keepalive | weekly: rebase-preview each open `pr/*` head against `origin/master` in the scratch worktree, report which are still clean; a conflicting head is a node for the next session | n/a |
| S5 Contribution intake | keep the existing flow (issue first, `pr/*` on a recent upstream base, port only the fix and tests). Add to `AGENTS.md`: a session that fixes a bug on the integration line records the candidate PR node in the plan of record before closing | doc only |

### Phase 5: measurement backlog (unchanged, after quota permits)

M1 jcode-bench baseline, M2 routing arms A/B/C, U5 browser issue after soak, V2 iOS voice
after the device trial. No change to their definitions in the 09-24 plans.

## 7. Operator approvals this plan needs (nothing below happens without a named yes)

| Id | Action | Why |
| --- | --- | --- |
| A1 | PATCH the OpenRouter workspace default guardrail: `credit-card` block → flag, add the Luhn-shaped custom block filter | ends the false-positive 403s |
| A2 | Merge `origin/master` `74577fe83` into the integration line and push to `fork` | phase 0 |
| A3 | Push U6 branch in `~/.jcode/scratch/jpl` and open the jev-pr-labeler PR | signing unblocked |
| A4 | One comment on #1354 explaining the SSH-secret step failure is infra-only | F3 |
| A5 | Standing pattern: each daily sync receipt that is clean is a request to merge and push; approval is per receipt | phase 4 |
| A6 | First `pulumi up` on the estate stack with OpenRouter dynamic resources (after a clean preview) | phase 3 |
| A7 | Empty `~/.Trash/jcode-firefox-retire-20260924/` and `~/.Trash/jcode-prune-20260925/` | finalize retirements |

## 8. Continuation prompt for the Opus 5.5 agent

> You are the captain. Read `docs/plans/2026-09-25-ARCHITECT_ASSESSMENT_AND_PHASED_PLAN.md`.
> Run phase 0, then phase 1 in node order G1-G6; G1's first mutation and the merge/push in
> phase 0 each need the named approval in §7 (A1, A2). Then phase 2 N1-N7, filing an
> upstream issue before each PR-shaped node. Phase 3 starts with a read-only preview of I1.
> Freeze one treatment per node, hold the write lease, commit with `--only`, run the narrow
> crate test after each edit, and report in under 5 lines with the model and effort named.
