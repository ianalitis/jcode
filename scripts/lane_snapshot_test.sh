#!/usr/bin/env bash
# Tests for scripts/lane-snapshot.sh: two fixture snapshots under
# $JCODE_SCRATCH_DIR (openai at 30% and 90% used) must differ only in the
# recommendation the packet pins down. Read-only; no network.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script="$here/lane-snapshot.sh"

scratch=${JCODE_SCRATCH_DIR:-}
if [ -z "$scratch" ]; then
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/lane-snapshot-test.XXXXXX")
fi

write_fixture() {
    local path=$1 openai_used=$2 claude_used=$3
    cat >"$path" <<JSON
{
  "providers": [
    {
      "provider_name": "Anthropic (Claude) (i***s@example.com)",
      "limits": [
        {"name": "5-hour window", "usage_percent": ${claude_used}, "resets_at": "2026-09-21T19:00:00+00:00", "reset_in": "4h 28m"},
        {"name": "7-day window", "usage_percent": 43.0, "resets_at": "2026-09-25T07:00:00+00:00", "reset_in": "3d 16h"}
      ],
      "extra_info": [],
      "error": null
    },
    {
      "provider_name": "OpenCode Go (API key)",
      "limits": [],
      "extra_info": [["Key status", "valid"]],
      "error": null
    },
    {
      "provider_name": "OpenRouter",
      "limits": [{"name": "Credits", "usage_percent": 19.8, "resets_at": null, "reset_in": null}],
      "extra_info": [["Balance", "\$92.16 / \$115.00"]],
      "error": null
    },
    {
      "provider_name": "OpenAI (ChatGPT) (i***s@example.com)",
      "limits": [{"name": "7-day window", "usage_percent": ${openai_used}, "resets_at": "2026-09-27T17:08:16+00:00", "reset_in": "6d 2h"}],
      "extra_info": [["Plan", "prolite"]],
      "error": null
    }
  ]
}
JSON
}

fail() {
    echo "FAIL: $1" >&2
    exit 1
}

contains() {
    printf '%s\n' "$1" | grep -q "$2"
}

headroom_fixture="$scratch/lane-snapshot-openai-30.json"
stressed_fixture="$scratch/lane-snapshot-openai-90.json"
write_fixture "$headroom_fixture" 30 77
write_fixture "$stressed_fixture" 90 85

headroom_out=$(LANE_SNAPSHOT_JSON="$headroom_fixture" bash "$script")
stressed_out=$(LANE_SNAPSHOT_JSON="$stressed_fixture" bash "$script")

echo "=== openai at 30% used ==="
printf '%s\n' "$headroom_out"
echo "=== openai at 90% used ==="
printf '%s\n' "$stressed_out"
echo

contains "$headroom_out" 'frontier: openai-oauth:gpt-5.6-terra' \
    || fail "frontier lane missing when openai headroom is 70"
if contains "$stressed_out" 'frontier: openai-oauth:gpt-5.6-terra'; then
    fail "frontier lane offered when openai headroom is 10"
fi

contains "$headroom_out" 'worker: opencode-go:deepseek-v4.1-flash' \
    || fail "base worker missing (openai 30%)"
contains "$stressed_out" 'worker: opencode-go:deepseek-v4.1-flash' \
    || fail "base worker missing (openai 90%)"

if contains "$headroom_out" 'claude: pause'; then
    fail "claude pause recommended with 23 headroom on the 5h window"
fi
contains "$stressed_out" 'claude: pause' \
    || fail "claude pause missing with 15 headroom on the 5h window"

contains "$headroom_out" 'go.*unknown' \
    || fail "go lane must be unknown (windows not exposed)"
contains "$headroom_out" "\$92.16" \
    || fail "openrouter dollars missing"
contains "$headroom_out" 'local ' \
    || fail "local hour missing"
contains "$headroom_out" 'utc ' \
    || fail "utc hour missing"

echo "PASS"
