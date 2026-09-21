#!/usr/bin/env bash
# attempt_worktree_test.sh: self-contained round-trip test for
# scripts/attempt-worktree.sh. Builds a throwaway git repo under
# $JCODE_SCRATCH_DIR and exercises create / commit / list / discard.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
helper="$here/attempt-worktree.sh"

scratch=${JCODE_SCRATCH_DIR:-${TMPDIR:-/tmp}}
mkdir -p "$scratch"
tmp=$(mktemp -d "$scratch/attempt-worktree-test.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

export JCODE_WORKTREE_ROOT="$tmp/root"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

git init -q "$tmp/repo"
cd "$tmp/repo"
git config user.email test@example.com
git config user.name test
echo one > file.txt
git add file.txt
git commit -qm init

# --- create ---------------------------------------------------------------
"$helper" create t1 >/dev/null
wt="$JCODE_WORKTREE_ROOT/attempt-t1"
[ -d "$wt" ] || fail "worktree dir missing: $wt"
got_branch=$(git -C "$wt" rev-parse --abbrev-ref HEAD)
[ "$got_branch" = "attempt/t1" ] || fail "branch is $got_branch, want attempt/t1"

# refuses duplicates
if "$helper" create t1 >/dev/null 2>&1; then
    fail "duplicate create succeeded"
fi

# --- commit ---------------------------------------------------------------
echo two > "$wt/new.txt"
hash=$("$helper" commit t1 -m "test commit" -- new.txt)
[ -n "$hash" ] || fail "commit printed no hash"
hash_len=${#hash}
[ "$hash_len" -eq 40 ] || fail "commit hash not 40 chars: $hash"
git -C "$wt" rev-parse --verify "$hash^{commit}" >/dev/null || fail "commit $hash not in repo"
branch_head=$(git -C "$wt" rev-parse attempt/t1)
[ "$branch_head" = "$hash" ] || fail "attempt/t1 head $branch_head != $hash"
git -C "$wt" show --stat --oneline "$hash" | grep -q new.txt || fail "commit does not touch new.txt"

# --- list -----------------------------------------------------------------
listing=$("$helper" list)
echo "$listing" | grep -q "^t1	attempt/t1	" || fail "list missing t1 row: $listing"

# --- discard without --yes exits 2 ---------------------------------------
set +e
"$helper" discard t1 >/dev/null 2>&1
code=$?
set -e
[ "$code" -eq 2 ] || fail "discard without --yes exited $code, want 2"
[ -d "$wt" ] || fail "discard without --yes removed the worktree"
git -C "$tmp/repo" rev-parse --verify --quiet refs/heads/attempt/t1 >/dev/null \
    || fail "discard without --yes deleted the branch"

# --- discard with --yes ---------------------------------------------------
"$helper" discard t1 --yes >/dev/null
if [ -d "$wt" ]; then fail "worktree not removed: $wt"; fi
if git -C "$tmp/repo" rev-parse --verify --quiet refs/heads/attempt/t1 >/dev/null 2>&1; then
    fail "branch attempt/t1 not deleted"
fi

echo PASS
