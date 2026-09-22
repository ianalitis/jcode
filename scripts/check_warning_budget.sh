#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
baseline_file="$repo_root/scripts/warning_budget.txt"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check_warning_budget.sh            # fail if warnings exceed baseline
  scripts/check_warning_budget.sh --update   # update baseline to current warning count

Notes:
  - Counts Rust compiler lines that begin with "warning:" from `cargo check -q`
  - Baseline is stored in scripts/warning_budget.txt
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ ! -f "$baseline_file" ]]; then
  echo "error: missing baseline file: $baseline_file" >&2
  exit 1
fi

if ! command -v cargo > /dev/null 2>&1; then
  echo "error: cargo not found" >&2
  exit 1
fi
# Check compilation before counting warnings, including in --update mode.
if output=$(cd "$repo_root" && CARGO_TERM_COLOR=never cargo check -q 2>&1); then
  :
else
  status=$?
  printf '%s\n' "$output" >&2
  exit "$status"
fi
# grep is available on CI without ripgrep; exit 1 means no warnings, not failure.
current=$(printf '%s\n' "$output" | grep -c '^warning:' || [[ $? -eq 1 ]])
current=$(printf '%s' "${current:-0}" | tr -d '[:space:]')
baseline=$(tr -d '[:space:]' < "$baseline_file")

if [[ "${1:-}" == "--update" ]]; then
  printf '%s\n' "$current" > "$baseline_file"
  echo "Updated warning baseline: $baseline"
  echo "New warning baseline: $current"
  exit 0
fi

if ! [[ "$baseline" =~ ^[0-9]+$ ]]; then
  echo "error: invalid warning baseline in $baseline_file: '$baseline'" >&2
  exit 1
fi

if (( current > baseline )); then
  echo "Warning budget exceeded: current=$current baseline=$baseline" >&2
  # Print what was counted. The count alone does not say which crate warned, and this
  # gate only reproduces on a cold build: a warm local tree reports zero, so the
  # evidence lives in the CI log and nowhere else. Naming the warnings here is the
  # difference between one CI round trip and a guess. awk rather than `grep | head`
  # because `set -o pipefail` turns the closed pipe into a gate failure.
  printf '%s\n' "$output" | awk '/^warning:/ { print; seen += 1 } seen == 20 { exit }' >&2
  if (( current > 20 )); then
    echo "... and $((current - 20)) more" >&2
  fi
  echo "Run scripts/check_warning_budget.sh --update only after intentional cleanup." >&2
  exit 1
fi

if (( current < baseline )); then
  echo "Warning budget improved: current=$current baseline=$baseline"
  echo "Consider running: scripts/check_warning_budget.sh --update"
else
  echo "Warning budget OK: current=$current baseline=$baseline"
fi
