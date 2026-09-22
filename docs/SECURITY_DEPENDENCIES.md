# Dependency Security Triage

Last reviewed: 2026-09-22

This file tracks the current `cargo audit` findings for jcode and the intended remediation path.
It is not an allowlist. It is a triage record so advisories are visible and actionable.

## Current advisories

| Advisory | Crate | Dependency path | Affected area in jcode | Triage | Planned action |
|---|---|---|---|---|---|
| `RUSTSEC-2025-0141` | `bincode` | `syntect -> bincode` | Markdown/code highlighting in the TUI | Unmaintained transitive dependency. No direct exposure in the provider/auth flow. | Track `syntect` upgrades or replace `syntect` if upstream does not move off `bincode` soon. |
| `RUSTSEC-2024-0436` | `paste` | `ratatui -> paste`, `tokenizers -> paste`, `tract-* -> paste` | TUI rendering, tokenizers, embedding/model support | Widely transitive. Not isolated to one module. | Prefer upstream dependency upgrades before any local workaround. Re-evaluate after bumping `ratatui`, `tokenizers`, and `tract-*`. |
| `RUSTSEC-2026-0002` | `lru` | `ratatui -> lru` | TUI rendering/cache internals | Unsoundness warning in a UI dependency. Not in auth/provider logic, but still ships in-process. | Upgrade `ratatui` / `ratatui-image` together once compatible. |
| `RUSTSEC-2026-0141` | `lettre` | `jcode-notify-email -> lettre` | Notification email sending | Vulnerability applies to the Boring TLS backend hostname verification path. Jcode's `lettre` dependency uses rustls/native-tls features, not `boring-tls`, so this is not believed exploitable in the current build. | Keep ignored in `scripts/security_preflight.sh`; remove ignore after `lettre` ships a patched release or if feature use changes. |
| `RUSTSEC-2026-0098` | `rustls-webpki` | `rustls` dependency stack | TLS certificate validation in rustls consumers | Name constraints for URI names incorrectly accepted. Transitive via TLS libraries. | Upgrade rustls/webpki stack when compatible releases are available. |
| `RUSTSEC-2026-0099` | `rustls-webpki` | `rustls` dependency stack | TLS certificate validation in rustls consumers | Name constraints accepted for wildcard certificates. Transitive via TLS libraries. | Upgrade rustls/webpki stack when compatible releases are available. |
| `RUSTSEC-2026-0104` | `rustls-webpki` | `rustls` dependency stack | TLS certificate revocation list parsing | Reachable panic in CRL parsing. Transitive via TLS libraries. | Upgrade rustls/webpki stack when compatible releases are available. |
| `RUSTSEC-2026-0049` | `rustls-webpki` | `rustls` dependency stack (`aws-smithy` rustls 0.21, `imap`/`rustls-connector` rustls 0.22) | TLS certificate revocation list handling | CRLs not considered authoritative by Distribution Point due to faulty matching logic. Transitive via the older rustls stacks; fix needs rustls-webpki >=0.103.10, which requires major bumps of the `aws-sdk`/`imap` stacks. | Upgrade rustls/webpki stack when compatible releases are available. |
| `RUSTSEC-2026-0187` | `lopdf` | `jcode-pdf -> pdf-extract 0.8.2 -> lopdf 0.34` | PDF text extraction (`/pdf`, image/PDF reads) | Stack overflow parsing deeply nested PDF objects. Only reached when extracting text from a (potentially malicious) PDF the user opens; not in the auth/provider/network path. `pdf-extract 0.8.2` pins `lopdf 0.34`, so it cannot be bumped to the fixed `>=0.42` without an upstream `pdf-extract` release. | Upgrade once `pdf-extract` ships a release depending on `lopdf >=0.42`; remove the ignore then. |
| `RUSTSEC-2023-0086` | `lexical-core` | `imap -> imap-proto -> lexical-core` | Gmail/IMAP support path | Old unsound transitive dependency in the mail stack. Higher priority than the UI-only findings because it touches network-parsed data. | Investigate upgrading or replacing `imap` / `imap-proto`. If no maintained path exists, isolate or remove the IMAP dependency. |

## Priority order

1. `rustls-webpki` TLS advisories via rustls stack
2. `lexical-core` via `imap-proto`
3. `lettre` if Jcode ever enables `boring-tls`
4. `lru` via `ratatui`
5. `bincode` via `syntect`
6. `paste` via multiple transitive dependencies

## Notes

- Dependabot alerts are enabled on the public fork (`ianalitis/jcode`), so this
  register is no longer fed by `cargo audit` alone: it also covers advisories
  reported against the crate graph and against `sdk/typescript/package-lock.json`,
  which `cargo audit` cannot see. An alert is not automatically a finding here;
  each is triaged the same way, by reachability and by whether a patched release
  exists inside the range the graph already allows.
- Resolved on 2026-09-22 by bumping only the affected lockfile entries, no
  manifest changes and no `cargo update` beyond `--precise` per crate:
  - `GHSA-3pv8-6f4r-ffg2` (`tar <= 0.4.45`, PAX header desynchronization) ->
    `tar` 0.4.45 -> 0.4.46. `tar` is a direct `jcode-app-core` dependency
    (archive extraction over paths the user names), so the parse path is
    reachable from a user-supplied archive.
  - `GHSA-3rjw-m598-pq24` / `CVE-2026-50185` (`cmov < 0.5.4`, aarch64 may
    select on stale high register bits) -> `cmov` 0.5.3 -> 0.5.4. Reached
    through `ctutils -> digest -> hmac -> aws-sigv4` in the AWS Bedrock signing
    path, not the UI, and only on aarch64.
  - `RUSTSEC-2026-0097` / `GHSA-cq8v-f236-94qc` (`rand`, unsound with a custom
    logger using `rand::rng()`) -> the 0.8 line moved 0.8.5 -> 0.8.6. The other
    two lines in the graph (0.9.3 and 0.10.1) are already at or above the
    patched release for their ranges, so no `rand` version now falls inside an
    affected range. Reached through `ratatui-image` (TUI images) and
    `tungstenite` (websocket transport). This replaces the previous plan of
    waiting for a `rand` 0.9-era migration, which the 0.8.6 patch made
    unnecessary for this advisory.
  - `GHSA-5jgf-p345-68v8`, `GHSA-f65p-4m7j-42xc`, `GHSA-fph4-wmhf-6fwf` and
    `GHSA-jqff-g426-hqxp` (`fast-uri >= 3.0.0, < 3.1.6`, host confusion and SSRF
    in URI parsing) -> the lockfile's transitive `fast-uri` moved 3.1.5 -> 3.1.8
    inside `ajv`'s existing `^3.0.1` range. `ajv` is a direct runtime dependency
    of `@1jehuang/jcode-sdk`, so this sits in the SDK's own schema-validation
    path rather than in test tooling; `npm audit` reports zero vulnerabilities
    against the SDK lock after the change.
- Those four bump the fork's crate graph and SDK lock only. The fork's mirror of
  `master` is upstream's tree, so its Dependabot alerts stay open until the same
  entries move upstream; a fork-side bump does not clear them.

- None of the advisories above were introduced by the provider-auth refactor.
- The provider/auth hardening work should continue independently of these dependency upgrades.
- `RUSTSEC-2026-0217` (`tract-nnef` 0.21.10, integer overflow in the NNEF tensor
  parser) was resolved on 2026-07-30 by moving `jcode-embedding` to `tract` 0.23.
  The in-line `0.21.16` fix was unreachable: `tract-data 0.21.16` pins
  `half ^2.5`. The 0.23 line drops that pin. This mattered because the parser
  runs over a model downloaded at runtime rather than one shipped in the binary,
  so an ignore would not have been clearly safe. See #657.
- `RUSTSEC-2024-0320` (`yaml-rust`) was removed from the dependency graph on 2026-03-05 by trimming `syntect` features to built-in syntax/theme dumps instead of YAML loading.
- `RUSTSEC-2026-0194` / `RUSTSEC-2026-0195` (`quick-xml` 0.39.2): reached only through `wayland-scanner`, a build-time proc-macro in the desktop crate's winit stack. It parses trusted, vendored Wayland protocol XML during compilation and never touches untrusted input at runtime. Remediation is upstream: `wayland-scanner` needs to move to `quick-xml >= 0.41`. Triaged and ignored in `scripts/security_preflight.sh` on 2026-07-04.
- `scripts/security_preflight.sh` ignores the vulnerability advisories that are explicitly triaged above (`lettre` and `rustls-webpki`) so CI can remain actionable. New vulnerabilities still fail CI by default.
- Before changing dependency versions, run:
  - `cargo check`
  - `cargo test -j 1`
  - `scripts/security_preflight.sh`
