# firefox-agent-bridge v0.10.0 ships the 0.9.7 Firefox extension

Date: 2026-09-24. Status: **filed** as
https://github.com/1jehuang/firefox-agent-bridge/issues/11 (operator-approved).

## Observed

Release `v0.10.0` (2026-09-23, marked Latest) attaches:

- `browser-agent-bridge-chrome-0.10.0.zip`, `browser-agent-bridge-safari-0.10.0.zip`
- `browser-agent-bridge-0.9.7.xpi`, whose `manifest.json` says `"version": "0.9.7"`

The tag's source `extension/manifest.json` is `0.10.0`. So Firefox users of the
latest release get the previous extension alongside 0.10.0 hosts, while Chrome
and Safari get 0.10.0.

## Why

`.github/workflows/release.yml` intentionally does not publish the locally built
XPI ("Firefox needs the AMO-signed XPI, which is attached separately. The
locally built XPI is unsigned, so do not publish it."). The separately attached
signed XPI was not refreshed for 0.10.0. The workflow is correct; the manual
signing step lagged.

## Impact on Jcode

`crates/jcode-base/src/browser.rs` downloads the first `*.xpi` asset from
`releases/latest`, so Jcode stages the 0.9.7 extension into the agent profile.
Jcode's `compatible`/`missing_actions` check limits the damage to a clear
"extension out of date" status, but any action added in 0.10.0 is unavailable
on Firefox.

## Suggested fix (for the issue)

1. Attach the AMO-signed 0.10.0 XPI to `v0.10.0`.
2. Optionally fail the release job, or mark the release draft, until an XPI whose
   manifest version equals the tag is attached, so this cannot recur silently.

Evidence: `unzip -p browser-agent-bridge-0.9.7.xpi manifest.json` -> `0.9.7`;
`gh api .../contents/extension/manifest.json?ref=v0.10.0` -> `0.10.0`.
