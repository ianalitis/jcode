# Privacy-safe development and DeepSeek iteration

## Enforced boundary

`jcode-base::message::{sanitize_outbound_text, sanitize_outbound_json}` is the shared,
fallible sanitizer. It recognizes selected credential formats and assignments,
PEM private-key blocks (including a truncated block without its closing marker),
and ASCII payment-card candidates with a supported issuer, conventional grouping,
and valid Luhn checksum. A checksum match is a heuristic, not evidence that a
number belongs to a real account. Ordinary four-digit identifiers, invalid
checksums, and unsupported issuer prefixes are not blanket-redacted.

The JSON visitor checks every string value and key, nested objects/arrays, and
JSON-encoded argument/extension strings. Unsigned integer JSON card candidates
use the same issuer/checksum rule. Other numeric values and booleans remain intact.
Rule compilation, invalid custom credential names, excessive nesting, or key
collisions fail closed with a fixed error containing no input or matched value.

* **OpenRouter-compatible chat:** the entire request is checked after `extra_body`
  merging and before canonical fingerprints and retry snapshots. The actual SSE
  send boundary checks it again. This covers direct DeepSeek and OpenCode Go
  profiles using this runtime, including restored tool arguments, reasoning,
  tool definitions, system text, and extension fields. Sanitizer failure prevents
  sending the request. HTTP errors expose status and fixed hints, not response
  bodies. Transport errors discard their original potentially sensitive chains.
  Authentication headers still carry the credentials required by the configured
  endpoint. No provider, model fallback, or authentication configuration is changed.
* **Tools:** successful output text, titles, metadata, and image labels are checked
  before post-tool observers. Error chains are sanitized and flattened before
  those observers and result logging. A sanitized copy of input goes to post-tool
  hooks and telemetry. Feedback is checked again before context truncation.
  SDK callback completion and manual result/error insertion also check results.
  Actual local tool execution inputs and source files are not rewritten.
* **Persistence:** existing calls to `message::redact_secrets` reuse this sanitizer
  and store a fixed withholding marker on failure. This is not a migration or
  erasure of old sessions, logs, or external copies.

## Safe discovery and fixtures

Start discovery with metadata: paths, counts, or a pass/fail result. Do not send
raw matching lines, fixture contents, request bodies, or unsanitized failure logs
to a model. `scripts/security_preflight.sh` now reports only the secret-scan result,
using path-only matching internally with output discarded. It writes no match
file and preserves scan-error failure and the `grep` fallback. Its existing file
type exclusions and dependency advisory policy are unchanged.

Construct sensitive-looking regression inputs at runtime from fragments or
checksum generators. Assert absence and fixed markers without printing the input,
captured payload, or a value-bearing assertion diff. Never add fixture exemptions,
provider-specific sanitizer bypasses, or DLP-disable switches to make a test pass.
The shared sanitizer applies the same rules to fixtures and ordinary content.

## Local regression commands

These commands use synthetic data and loopback transports, not hosted DeepSeek
requests, real credentials, or provider switching. Run from the repository root:

```bash
scripts/dev_cargo.sh test -p jcode-base --no-default-features --lib message::outbound::tests -- --test-threads=1
scripts/dev_cargo.sh test -p jcode-provider-openrouter-runtime --lib privacy_tests -- --test-threads=1
scripts/dev_cargo.sh test -p jcode-provider-openrouter-runtime --lib openrouter_sse_stream::tests -- --test-threads=1
scripts/dev_cargo.sh test -p jcode-app-core --no-default-features --lib tool::privacy::tests -- --test-threads=1
scripts/dev_cargo.sh test -p jcode-app-core --no-default-features --lib tool::sdk::tests::privacy_sdk_completion -- --test-threads=1
python3 scripts/test_security_preflight.py
```

The preflight test covers both scan engines, matching-source failure, scanner
errors, and clean-source success. Existing persistence compatibility tests can
also be run with `--lib redact_secrets`, but capture their output locally and
report only test names/status if diagnosing legacy fixture failures.

## Limits

This is not universal PII removal. Names, addresses, email, phone numbers,
government identifiers, unknown credential formats, obfuscation, malformed or
non-JSON embedded encodings, uncommon card groupings/issuers, floating-point or
scientific-notation identifiers, and credentials split across separate fields
are not comprehensively detected. A benign identifier indistinguishable from a
supported checksum-valid card may be redacted. Do not treat a passing scan as
permission to disclose private source.

Images, screenshots, PDF/binary attachments, base64 content, and encrypted
reasoning are not decoded or OCR-scanned. Encoded tool image bytes are preserved,
not certified safe. Request string scanning cannot inspect what those bytes
represent. Other provider runtimes, tool-internal network calls, pre-tool hooks,
lower-level logs, and early registry validation errors are not comprehensively
guarded by this repair. Use metadata-only workflows for these surfaces.

Tests compile test targets but do not activate a new daemon. Build/reload and
live-runtime validation remain separate, operator-controlled acceptance steps.
