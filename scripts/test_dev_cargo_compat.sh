#!/usr/bin/env bash
set -euo pipefail
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/jcode-shell-test.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
cat > "$tmp/bin/cargo" <<'EOF'
#!/bin/sh
printf 'cargo executed\n' > "$TEST_CARGO_MARKER"
exit "${TEST_CARGO_STATUS:-0}"
EOF
cat > "$tmp/bin/flock" <<'EOF'
#!/bin/sh
# Force the contention branch once, then check the descriptor is open.
if [ "$1" = -n ]; then exit 1; fi
: >&9
EOF
cat > "$tmp/bin/date" <<'EOF'
#!/bin/sh
case "$1" in
  +%s%N) printf '1700000000N\n' ;;
  *) /bin/date "$@" ;;
esac
EOF
chmod +x "$tmp/bin/"*
for shell in /bin/bash "$(command -v bash)"; do
  for status in 0 23; do
    rm -f "$tmp/marker"
    result=0
    PATH="$tmp/bin:$PATH" TEST_CARGO_MARKER="$tmp/marker" TEST_CARGO_STATUS="$status" \
      JCODE_CARGO_GATE_PATH="$tmp/gate" JCODE_RUST_ACTION_LOG_PATH="$tmp/actions.jsonl" \
      JCODE_PARALLEL_FRONTEND=0 JCODE_SCCACHE=off JCODE_REMOTE_CARGO=0 \
      JCODE_BUILD_GIT_HASH=test "$shell" "$repo_root/scripts/dev_cargo.sh" check || result=$?
    test "$result" -eq "$status"
    test -f "$tmp/marker"
  done
done
python3 - "$tmp/actions.jsonl" <<'PY'
import json
import sys
with open(sys.argv[1]) as stream:
    rows = [json.loads(line) for line in stream]
assert [row['exit_code'] for row in rows] == [0, 23, 0, 23], rows
assert [row['success'] for row in rows] == [True, False, True, False], rows
assert all(row['duration_ms'] >= 0 for row in rows), rows
PY
# Simulate leaked local configuration without mutating the real repository.
mkdir -p "$tmp/other-worktree"
rm -f "$tmp/marker"
result=0
PATH="$tmp/bin:$PATH" TEST_CARGO_MARKER="$tmp/marker" \
  GIT_WORK_TREE="$tmp/other-worktree" \
  /bin/bash "$repo_root/scripts/dev_cargo.sh" check 2>"$tmp/mismatch" || result=$?
test "$result" -eq 2
test ! -f "$tmp/marker"
grep -q 'Git worktree differs' "$tmp/mismatch"
result=0
PATH="$tmp/bin:$PATH" TEST_CARGO_MARKER="$tmp/marker" GIT_DIR="$tmp/no-git-dir" \
  /bin/bash "$repo_root/scripts/dev_cargo.sh" check 2>"$tmp/invalid" || result=$?
test "$result" -eq 2
test ! -f "$tmp/marker"
grep -q 'cannot discover Git worktree' "$tmp/invalid"
printf 'Cargo shell compatibility contracts passed\n'
