# Fork policy: no optional data collection

This fork removes the CLI's optional telemetry implementation, not merely its
opt-out preference. It has no telemetry HTTP client, endpoints, upload queues,
retry workers, installation-ID store, usage accumulator, or transcript uploader.
The compatibility API in `crates/jcode-telemetry-core` discards recording calls.
There is no build feature, environment switch, or consent marker to enable it.

Removed paths include install/upgrade events, auth/onboarding analytics,
feedback uploads, session/turn/crash metrics, concurrency tracking, todo and
workflow aggregates, and full-transcript uploads. Support drafts no longer read
or include old telemetry IDs. Optional OpenRouter application-attribution headers
(`HTTP-Referer` and `X-Title`) are removed; functional routing metadata and
provider session/caching identifiers are separate service traffic.
The installer does not report
success/failure or persist conversion IDs. The maintainer-feedback agent tool
is not registered. `/feedback` reports that nothing was sent.

`jcode telemetry status --json` reports:

```json
{
  "enabled": false,
  "content_sharing_enabled": false,
  "opt_out_source": "build_policy",
  "telemetry_id": null
}
```

`jcode telemetry enable` fails with an explanation. `disable` is idempotent and
does not write or delete files. Old consent markers and environment opt-ins
have no effect. Existing IDs and historical records are not read by this API
or silently erased; remote deletion needs the recipient's cooperation.

## Discovery and sponsor reporting

Discovery is disabled by default. Configuration loading never turns a stored
opt-out into an opt-in. Sponsor provenance tagging, usage counters and `/usage`
uploads are removed even if discovery is explicitly enabled. Discovery requests
no longer include internal session IDs, session correlation IDs, or build/session
metadata headers.

An explicitly enabled discovery service still receives functional requests:
queries, reasons, selection/detail/suggestion fields, per-request IDs, ordinary
HTTP metadata, and an explicit benchmark marker when requested. Such a service
can keep its own operational records. Do not send private material to it.

## What this policy does not remove

- Prompts/context/tool results sent to a selected cloud inference provider.
- Requests to explicitly used remote tools, MCP services, auth, billing, model
  catalogs, downloads, update services, or web search.
- Local sessions, recovery state, logs, performance diagnostics, token/cost
  displays, local IPC, or user-requested exports. These are not vendor telemetry.
- Collection by independently installed tools, OS/desktop apps, remote services,
  or legacy server-side infrastructure. The separate `telemetry-worker/` sources
  are not linked into the CLI; this change neither deploys nor decommissions a
  remote worker, deletes its records, nor changes any remote account policy.

Opt-out settings are not an OS/network isolation boundary. A zero-unapproved-
egress requirement needs a separate allowlist/sandbox and provider/tool decisions.
Changing source alone does not update a running shared daemon. Build and test an
isolated binary first; installation and shared-daemon restart are separate steps.

## Regression verification

```sh
cargo test --offline -p jcode-telemetry-core --all-targets
cargo test --offline -p jcode-base --lib sponsors
cargo test --offline -p jcode-base --lib discovery_provenance_end_to_end
cargo test --offline -p jcode-app-core --lib tool::discover::tests
cargo test --offline -p jcode-app-core --lib maintainer_feedback
cargo test --offline -p jcode-app-core --lib concurrency
cargo test --offline -p jcode-tui --lib telemetry
cargo test --offline -p jcode-provider-openrouter-runtime --lib router_shaping_tests
bash scripts/test_install_conversion.sh
```

The telemetry integration test uses the normal library build in subprocesses,
with fresh and legacy-consent homes. It verifies rejection of enable requests,
no ID disclosure, no new tracking files, no modification of historical files,
and no proxy connection while exercising former collection entry points.
Removed collector-internal payload/queue tests are replaced by these no-
collection assertions, not hidden behind ignores or a test-only policy switch.
Installer fixtures intercept downloads and reporting; they install no real tool.
