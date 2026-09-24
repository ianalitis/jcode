# Headless-first browser policy and jehuang dependency/fork assessment

Date: 2026-09-24. Status: approved by the operator 2026-09-24 and partly
executed (see "Adopted" at the end). Forks `ianalitis/firefox-agent-bridge` and
`ianalitis/jev-pr-labeler` exist.

## 1. What jcode actually depends on today

Verified against the integration line `a7fa819e4`.

| Repo | Kind | Pin / invocation | Where | Our fork | Fork drift vs upstream |
| --- | --- | --- | --- | --- | --- |
| `1jehuang/agentgrep` | build dep (Rust) | tag `v0.1.7`, `Cargo.lock:65` | `crates/jcode-app-core/Cargo.toml:109` | yes | +2 commits (CI only), 0 behind |
| `1jehuang/mermaid-rs-renderer` | build dep (optional feature `renderer`) | tag `v0.3.1` | `crates/jcode-tui-mermaid/Cargo.toml:22` | yes | +3 commits (CI only), 0 behind |
| `1jehuang/handterm` | runtime terminal backend (optional) | launched as `handterm --backend gpu --exec ...` | `crates/jcode-terminal-launch/src/lib.rs:722` | yes | +4 ahead / 14 behind |
| `1jehuang/firefox-agent-bridge` | runtime dependency (downloaded) | `releases/latest` (currently v0.10.0), host binary + browser extension | `crates/jcode-base/src/browser.rs:7` | **no** | - |
| `1jehuang/jev-pr-labeler` | CI dependency | GitHub Action in | `.github/workflows/label-pr.yml` | **no** | - |
| `1jehuang/ffp` | **not a dependency** | zero references in the tree | - | no | - |
| `1jehuang/jcode-bench` | **not a dependency** | zero references; benchmarking prior art | - | no | - |

`GLOOP` is a fork of `redacktion/GLOOP`, unrelated to jcode or jehuang.

## 2. The browser layer today

- `crates/jcode-app-core/src/tool/browser.rs` defines a `BrowserProvider`
  trait, but exactly **one** impl exists (`FirefoxBridgeProvider`) and
  `resolve_provider` errors for anything but `auto`/`firefox` ("not wired into
  the built-in browser tool yet").
- Headless is already the default: `JCODE_BROWSER_HEADLESS=0` merely *shows*
  the agent browser instance. The bridge already runs it headless.
- Upstream 0.88 `e1576e9e3` is the big browser update: it detects the user's
  browser and supports Firefox, Chrome, Edge, Brave, Chromium and Safari from
  bridge `v0.10.0`, adds `crates/jcode-base/src/browser_detect.rs` (596 lines)
  plus tests, and reworks `browser.rs` (2111 insertions across 12 files). We do
  not have it, because we are 141 commits behind (see
  `2026-09-24-UPSTREAM_088_RECONCILIATION.md`).
- `docs/BROWSER_PROVIDER_PROTOCOL.md` (draft) already anticipates CDP,
  WebDriver/BiDi and Safari adapters and a `provider.describe` capability
  negotiation. The abstraction we need already exists on paper.
- No Playwright anywhere. `tests/e2e/` is Rust integration tests, not browser
  end-to-end tests.

## 3. Policy: pick the tool from the task class

| Task | Tool | Why |
| --- | --- | --- |
| read / summarize / extract one URL | `webfetch` | no browser process at all |
| many pages, JS-light, crawler-shaped | Lightpanda (CDP) | 16x less memory / ~9x faster than headless Chrome on its own 933-page benchmark |
| real JS, needs a screenshot or a DOM the engine must actually build | headless Chrome or headless Firefox (CDP / bridge) | real engine, no window |
| the user's own logged-in session | bridge (Firefox/Chrome), headless unless the task needs a visible window | only path that carries their cookies and profile |
| goal-driven interactive browsing | existing `browser` handoff / `browser_fast` (Jev) over a headless provider | actions stay trusted, never model-authored JS |
| authored end-to-end tests in CI | Playwright | it is a test framework, not an agent tool |

Rule: headless by default. A headed browser only when the task states it needs
a visible window or the user's live profile.

## 4. Recommended implementation order

1. **Land 0.88** and inherit `browser_detect.rs` + multi-browser support for
   free; it is already written and tested upstream.
2. **One CDP `BrowserProvider`**, not one per product. Lightpanda serves CDP at
   `ws://<host>:9222` (`lightpanda serve --host ... --port 9222`, also WebDriver
   BiDi via `--protocol webdriver`), and headless Chrome/Chromium serve the same
   protocol on the same port shape. A single CDP adapter therefore covers
   Lightpanda *and* headless Chrome, which is the highest-leverage change here
   and is already listed as an intended adapter in the protocol doc.
3. Route in `resolve_provider` from declared capabilities (`provider.describe`),
   so `browser=lightpanda|chrome-headless|firefox|auto` are policy names, not
   separate code paths.
4. Evaluate `vercel-labs/agent-browser` (Rust CLI, headless Chrome for Testing)
   as a subprocess provider **only** if the CDP adapter proves insufficient for
   agentic workflows. Do not integrate it first; it would duplicate the CDP
   adapter's job.
5. Add Playwright as a separate CI job for our own products' E2E tests. It must
   not become the agent's browsing path.
6. Keep `webfetch` first for simple reads; it is the cheapest correct answer.

## 5. Forks: what to create and what to contribute

Have (all three carry only our fork-CI commits so far, and those are the
upstreamable part):

- `agentgrep` +2: `ci: add CI (format, clippy, test...)`, `fix(lint): clear
  rustfmt and clippy drift`.
- `mermaid-rs-renderer` +3: least-privilege workflow permissions, publish only
  from the canonical repository, manual runs.
- `handterm` +4: clippy 1.98 drift, machine-dependent test skips, least-privilege
  permissions. Upstream has moved 14 commits ahead since our fork.

Should fork:

- `firefox-agent-bridge` - we depend on it at runtime, it is the whole browser
  layer, and it has a real defect worth fixing: release `v0.10.0` ships
  `browser-agent-bridge-0.9.7.xpi` while the Chrome and Safari packages are
  `0.10.0`. Headless/multi-browser behaviour also lives here and is exactly what
  this policy wants to change.

Optional:

- `jev-pr-labeler` - our CI dependency. Our side already has the `gate` fix
  (`docs/upstream-feedback/2026-09-21-greptile-labeler-missing-key.md`); the
  upstreamable part is that the labeler should skip cleanly instead of exiting 1
  when its provider key is absent.

Skip: `ffp` (unused), `jcode-bench` (no code dependency; use it as a benchmark,
below).

## 6. Benchmarks

`jcode-bench` is the interesting one: score = `log2(given_cost / your_cost)`
over instruction counts inside the function (callgrind), correctness verified
exhaustively, so it is deterministic and cheat-resistant. Its README also
documents the discipline we should copy: grading noise stdev 0.0040 versus
run-to-run agent noise ~0.10, so a k=1 gap under ~0.1 says nothing and a 0.08
gap needs k >= 11. Requirements are gcc/clang, valgrind, python3, linux x86-64,
so it runs from macOS only inside a Linux container.

Prior art to weigh against it: SWE-bench (pass/fail, contamination-prone),
Terminal-Bench and OSWorld / WebArena (browser and desktop agents), Aider's
polyglot, LiveCodeBench, and METR's long-horizon task suite. Use `jcode-bench`
for the "is the harness faster at real primitives" question and a
WebArena/OSWorld-shaped task set for browser work.

Plan: run `jcode-bench` in a Linux container as the first harness A/B, adopt its
variance rules for every agent comparison we publish, and keep the browser task
set separate from the coding task set.

## 7. Decisions the operator owns

- Create the `firefox-agent-bridge` fork (and optionally `jev-pr-labeler`)?
- Approve the CDP `BrowserProvider` (covers Lightpanda + headless Chrome)?
- Choose a Linux container runtime for `jcode-bench` before it is used.


## Adopted (2026-09-24)

### Personal vs agent browsers

| Who | Browser | Rule |
| --- | --- | --- |
| Operator, daily | Comet (system default, `ai.perplexity.comet`) | Never an automation target |
| Operator, privacy | Safari | Never an automation target |
| Agents | Lightpanda, headless Chromium shell, Firefox agent profile | Headless, own profile, never the operator's |

Upstream 0.88 browser detection maps bundle ids to supported browsers; Comet is
unmapped, so `auto` falls through to installed Firefox, which this fork launches
only with its dedicated headless agent profile. Nothing agent-side opens Comet
or Safari. Safari stays reachable only by explicit `browser=safari`, which the
operator should not request for agent work.

### Agent browser ladder (measured on this Mac)

| Rung | How jcode reaches it | Cost | Covers |
| --- | --- | --- | --- |
| 1. `webfetch` | built-in tool | no browser | static reads |
| 2. **Lightpanda 0.4.1** | MCP server `lightpanda` (`~/dotfiles/scripts/lightpanda-mcp.sh`) | `fetch` 0.31 s / 27 MB; CDP server 54 MB; goto 334 ms, extract 8 ms live | JS pages, extract, click/fill/press/select, forms, screenshots, isolated sessions |
| 3. Headless Chromium (Chrome for Testing 149, Playwright's cached `chrome-headless-shell`) | CDP (Playwright `connectOverCDP`) | 98 MB, 303 ms | full Blink rendering when Lightpanda's engine falls short |
| 4. Firefox bridge, agent profile | built-in `browser` tool | headless unless `JCODE_BROWSER_HEADLESS=0` | extension-driven automation, Jev handoff |

Both CDP rungs passed the same Playwright probe: navigate, read title/h1/href,
`evaluate`, fill + click with a read-back, and a screenshot.

Lightpanda's own MCP server exposes about 30 browser tools, so the rung-2
integration is configuration, not new Rust: one pinned, checksum-verified,
environment-minimized launcher, same pattern as `context7-mcp.sh`. Lightpanda
reports usage to `telemetry.lightpanda.io` by default; the launcher always sets
`LIGHTPANDA_DISABLE_TELEMETRY=true`. The binary lives in
`~/.jcode/browser-tools/lightpanda/0.4.1/` with its `SHA256SUMS`; nothing is
installed into `/Applications` or on `PATH`.

No new GUI browser was installed. The headless Chromium shell already existed
in Playwright's cache and needs no app bundle, so "Chrome for Testing" as an
app is unnecessary. Firefox.app remains only because the bridge rung uses it.

### Revised implementation order

1. Done: Lightpanda via MCP (rung 2).
2. Next: a built-in CDP `BrowserProvider` is now optional, because rung 2 already
   gives agents headless CDP-class automation. Build it only if the built-in
   `browser` tool or Jev handoff needs to drive Lightpanda/Chromium directly.
3. Playwright stays the E2E test runner (validated here against both CDP
   endpoints), not the agent's browsing path.
4. When the operator no longer needs rung 4, Firefox.app can be removed; that
   is an operator decision.
