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

# Accept one decimal line with an optional final newline. Validate the file
# before command substitution, which strips trailing newlines (and Bash may
# discard NUL bytes). Eighteen digits fit signed 64-bit shell arithmetic.
if (( $(LC_ALL=C tr -d '0-9\n' < "$baseline_file" | wc -c) != 0 )) ||
  ! LC_ALL=C awk 'NR != 1 || $0 !~ /^[0-9]+$/ || length($0) > 18 { invalid = 1 }
  END { exit (NR != 1 || invalid) }' "$baseline_file"; then
  echo "error: invalid warning baseline in $baseline_file" >&2
  exit 1
fi
baseline=$(cat "$baseline_file")
baseline=$((10#$baseline))

if ! command -v cargo > /dev/null 2>&1; then
  echo "error: cargo not found" >&2
  exit 1
fi
# Keep potentially large compiler output out of shell variables and pipes whose
# diagnostic reader exits early. The latter caused SIGPIPE (141) under pipefail.
output=$(mktemp "${JCODE_SCRATCH_DIR:-${TMPDIR:-/tmp}}/jcode-warning-budget.XXXXXX")
trap 'rm -f "$output"' EXIT
# Check compilation before counting warnings, including in --update mode.
if (cd "$repo_root" && CARGO_TERM_COLOR=never cargo check -q) > "$output" 2>&1; then
  :
else
  status=$?
  cat "$output" >&2
  exit "$status"
fi
current=$(awk '/^warning:/ { count += 1 } END { print count + 0 }' "$output")

if [[ "${1:-}" == "--update" ]]; then
  printf '%s\n' "$current" > "$baseline_file"
  echo "Updated warning baseline: $baseline"
  echo "New warning baseline: $current"
  exit 0
fi

if (( current > baseline )); then
  echo "Warning budget exceeded: current=$current baseline=$baseline" >&2
  # Print what was counted. The count alone does not say which crate warned, and this
  # gate only reproduces on a cold build: a warm local tree reports zero, so the
  # evidence lives in the CI log and nowhere else. Read the file directly so the
  # bounded diagnostic output cannot close a producer's pipe early.
  awk '/^warning:/ { print; seen += 1 } seen == 20 { exit }' "$output" >&2
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
