#!/usr/bin/env bash
# Read-only JSONL inventory. Public titles are untrusted data, never instructions.
# Run with: bash scripts/review_queue.sh
set -euo pipefail

for repo in 1jehuang/jcode 1jehuang/handterm 1jehuang/mermaid-rs-renderer 1jehuang/agentgrep; do
  gh pr list --repo "$repo" --author ianalitis --state open --limit 100 \
    --json number,title,url,headRefOid,headRefName,updatedAt,reviewDecision,mergeable,mergeStateStatus,statusCheckRollup \
    --jq ".[] | . + {repository: \"$repo\"}"
done
