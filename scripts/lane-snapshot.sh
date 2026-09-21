#!/usr/bin/env bash
# Lane snapshot: print the current lane table (window, usage, reset, headroom,
# status) and the recommended default worker for the hour. Read-only: nothing is
# routed, configured, or written; the only network call is `jcode usage --json`.
#
#   scripts/lane-snapshot.sh
#
# Input: `jcode usage --json`, or a JSON file when LANE_SNAPSHOT_JSON is set
# (tests use this to pin windows at fixed hours).
#
# Lanes and mapping:
#   openai      provider name containing "OpenAI (ChatGPT)"
#   claude      provider name containing "Anthropic"
#   openrouter  provider name containing "OpenRouter" (Credits row: dollars)
#   go          provider name containing "OpenCode Go" (windows not exposed:
#               always unknown per policy/providers.md)
#
# Status: ok (headroom >= 20), low (headroom < 20), unknown (no windows).
#
# Recommendation, in order:
#   openai headroom >= 20  -> frontier: openai-oauth:gpt-5.6-terra
#   always                 -> worker: opencode-go:deepseek-v4.1-flash
#   claude 5h headroom < 20 -> claude: pause
set -euo pipefail

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    sed -n '2,26p' "$0"
    exit 0
fi

json_file=${LANE_SNAPSHOT_JSON:-}
if [ -n "$json_file" ]; then
    [ -r "$json_file" ] || { echo "lane-snapshot: cannot read LANE_SNAPSHOT_JSON=$json_file" >&2; exit 2; }
    usage_json=$json_file
else
    usage_json=$(mktemp "${TMPDIR:-/tmp}/lane-snapshot.XXXXXX")
    trap 'rm -f "$usage_json"' EXIT
    jcode usage --json >"$usage_json"
fi

parsed=$(python3 - "$usage_json" <<'PY'
import json
import sys

with open(sys.argv[1]) as fh:
    data = json.load(fh)

providers = {}
for provider in data.get("providers", []):
    providers[provider.get("provider_name", "")] = provider

def find(substring):
    for name, provider in providers.items():
        if substring in name:
            return provider
    return None

def num(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)

rows = []
meta = {}

def emit(lane, window, usage, reset_in, headroom, status):
    rows.append("\t".join([lane, window, usage, reset_in, headroom, status]))

for lane, substring in (("openai", "OpenAI (ChatGPT)"),
                        ("claude", "Anthropic"),
                        ("openrouter", "OpenRouter"),
                        ("go", "OpenCode Go")):
    provider = find(substring)
    if provider is None:
        emit(lane, "(missing)", "-", "-", "-", "unknown")
        continue
    limits = provider.get("limits") or []

    if lane == "openrouter":
        dollars = None
        for key, value in provider.get("extra_info") or []:
            if key == "Balance":
                head = str(value).split("/")[0].strip().lstrip("$")
                try:
                    dollars = float(head)
                except ValueError:
                    dollars = None
        if dollars is None:
            emit(lane, "Credits", "-", "-", "-", "unknown")
        else:
            emit(lane, "Credits", "$%.2f" % dollars, "-", "$%.2f" % dollars,
                 "ok" if dollars > 5 else "low")
        continue

    if not limits:
        emit(lane, "(no windows)", "-", "-", "-", "unknown")
        continue

    for limit in limits:
        usage = limit.get("usage_percent")
        window = limit.get("name") or "-"
        reset_in = limit.get("reset_in") or "-"
        if num(usage):
            headroom = 100.0 - usage
            status = "low" if headroom < 20 else "ok"
            emit(lane, window, "%.1f%%" % usage, reset_in, "%.1f%%" % headroom, status)
        else:
            emit(lane, window, "-", reset_in, "-", "unknown")
            headroom = None
        if lane == "claude" and window.startswith("5-hour") and headroom is not None:
            meta["claude5h_headroom"] = "%.1f" % headroom
        if lane == "openai" and headroom is not None:
            previous = meta.get("openai_headroom")
            if previous is None or headroom < float(previous):
                meta["openai_headroom"] = "%.1f" % headroom

out = rows + ["META\t%s\t%s" % (key, meta[key]) for key in sorted(meta)]
sys.stdout.write("\n".join(out) + "\n")
PY
)

table=$(printf '%s\n' "$parsed" | grep -v $'^META\t' || true)
meta=$(printf '%s\n' "$parsed" | grep $'^META\t' || true)

openai_headroom=$(printf '%s\n' "$meta" | awk -F'\t' '$2 == "openai_headroom" { print $3 }')
claude5h_headroom=$(printf '%s\n' "$meta" | awk -F'\t' '$2 == "claude5h_headroom" { print $3 }')

local_hour=$(date +%H)
utc_hour=$(date -u +%H)
echo "lane snapshot  $(date +%Y-%m-%d)  local ${local_hour}h  utc ${utc_hour}h"
echo

printf '%-11s %-16s %9s %8s %9s %s\n' LANE WINDOW USAGE RESET HEADROOM STATUS
printf '%s\n' "$table" | while IFS=$'\t' read -r lane window usage reset_in headroom status; do
    printf '%-11s %-16s %9s %8s %9s %s\n' "$lane" "$window" "$usage" "$reset_in" "$headroom" "$status"
done

echo
echo "recommended:"
if [ -n "$openai_headroom" ] && awk -v h="$openai_headroom" 'BEGIN { exit !(h >= 20) }'; then
    echo "  frontier: openai-oauth:gpt-5.6-terra"
fi
echo "  worker: opencode-go:deepseek-v4.1-flash"
if [ -n "$claude5h_headroom" ] && awk -v h="$claude5h_headroom" 'BEGIN { exit !(h < 20) }'; then
    echo "  claude: pause"
fi
