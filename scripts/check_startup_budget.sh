#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
binary=${1:-"$repo_root/target/release/jcode"}

if [[ ! -x "$binary" ]]; then
  echo "Binary not found or not executable: $binary" >&2
  echo "Build it first with: cargo build --release" >&2
  exit 1
fi

args=(--runs 3)
if [[ $(uname -s) == Linux ]]; then
  args+=(--check)
else
  echo "Startup budgets are calibrated for Linux; reporting measurements without enforcing them."
fi

exec python3 "$repo_root/scripts/bench_startup.py" "$binary" "${args[@]}"
