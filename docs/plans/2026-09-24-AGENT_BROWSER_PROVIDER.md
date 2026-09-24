# Agent browser provider decision (2026-09-24)

Status: decided, guard landed (`08d491015`). Backend work is P1/P2 in
[`2026-09-24-PHASE_PLAN_POST_088.md`](2026-09-24-PHASE_PLAN_POST_088.md).

## Problem

The built-in `browser` tool only speaks the extension bridge. Its `auto` target
comes from `browser_detect::resolve_detection`: env, saved preference, system
default, then the first installed browser. On macOS `BrowserKind::Safari`
always reports installed, so with Firefox uninstalled and Comet unmapped, auto
lands on Safari, the operator's personal browser. Explicit `browser=chrome|edge|
brave|safari` also opens the user's own install. Only the Gecko branch runs in
jcode's dedicated agent profile.

## Decision

1. **Hard guard, now (landed).** `require_isolated_agent_browser` in
   `crates/jcode-app-core/src/tool/browser.rs` rejects every non-Gecko target for
   all actions except read-only `status`, before setup, launch or bridge
   contact. Tests: `agent_browser_guard_admits_only_the_isolated_gecko_profile`,
   `explicit_safari_action_is_refused_before_any_bridge_contact`.
2. **Tool stays disabled on this machine** (`[tools].disabled += "browser"`) until
   a CDP backend exists: with Firefox removed there is no admissible target.
3. **Retire the extension bridge for agents; do not add a CDP backend to the
   built-in tool yet.** The MCP ladder already covers the need and keeps the
   browser out of the harness binary:
   - `webfetch` for static pages;
   - `lightpanda` MCP (pinned 0.4.1, checksum-verified launcher, telemetry off)
     for JS-light pages;
   - headless Chromium over CDP (Playwright's cached `chrome-headless-shell`,
     throwaway profile) for everything else.
   Revisit only if a measured task class fails on the ladder but would pass with
   a built-in tool (acceptance: two real task failures with receipts).
4. **If a CDP backend is ever built**, it must: launch only a headless binary
   with a fresh temp `--user-data-dir`; never attach to an existing CDP port;
   never read `~/Library/Application Support/{Comet,Google,BraveSoftware,
   Microsoft Edge}`; and keep the guard above as the single admission point.

## Upstream

The Safari fall-through is upstream behavior (`e1576e9e3`). Worth an issue: an
agent tool that silently drives the user's personal Safari is a surprise even
for upstream users. File after the guard has soaked on the fork.

## Validation

```sh
cargo test -p jcode-app-core --lib -- tool::browser
```
