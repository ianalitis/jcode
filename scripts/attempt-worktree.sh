#!/usr/bin/env bash
# attempt-worktree.sh: per-attempt worktree lifecycle for ephemeral workers.
#
#   scripts/attempt-worktree.sh create <id> [base]
#   scripts/attempt-worktree.sh commit <id> -m <message> -- <paths...>
#   scripts/attempt-worktree.sh discard <id> --yes
#   scripts/attempt-worktree.sh list
#
# One attempt is one worktree under $JCODE_WORKTREE_ROOT (default
# ~/.jcode/source/worktrees/attempt-<id>) on branch attempt/<id>, created from
# <base> (default HEAD). Proven work is committed there with `git commit
# --only` so a concurrent session's staged files never land in the attempt's
# commit (see AGENTS.md). `discard` is destructive: without --yes it prints the
# plan and exits 2. `list` is read-only. Nothing here pushes, merges, or
# touches the branch ledger.
set -euo pipefail

ROOT=${JCODE_WORKTREE_ROOT:-$HOME/.jcode/source/worktrees}

usage() {
    sed -n '2,14p' "$0" >&2
    exit 2
}

die() {
    echo "attempt-worktree: $*" >&2
    exit 1
}

# Attempt ids become path and branch components; reject anything that could
# escape the root or wedge git.
check_id() {
    case "$1" in
        '') die "empty attempt id" ;;
        *[!A-Za-z0-9._-]*) die "bad attempt id: $1" ;;
        .|..) die "bad attempt id: $1" ;;
    esac
}

wt_path() { echo "$ROOT/attempt-$1"; }

repo_root() {
    git rev-parse --show-toplevel 2>/dev/null || die "not inside a git repository"
}

branch_exists() {
    git -C "$1" rev-parse --verify --quiet "refs/heads/attempt/$2" >/dev/null 2>&1
}

cmd=${1:-}
[ -n "$cmd" ] || usage
shift

case "$cmd" in
    create)
        [ $# -ge 1 ] || usage
        id=$1; shift
        base=${1:-HEAD}
        check_id "$id"
        wt=$(wt_path "$id")
        repo=$(repo_root)
        if branch_exists "$repo" "$id"; then
            die "branch attempt/$id already exists"
        fi
        if [ -e "$wt" ]; then
            die "path already exists: $wt"
        fi
        mkdir -p "$ROOT"
        git -C "$repo" worktree add -b "attempt/$id" "$wt" "$base" >&2
        echo "$wt"
        ;;

    commit)
        [ $# -ge 3 ] || usage
        id=$1; shift
        check_id "$id"
        [ "$1" = "-m" ] || usage
        msg=$2; shift 2
        [ "${1:-}" = "--" ] || usage
        shift
        [ $# -ge 1 ] || die "no paths given after --"
        wt=$(wt_path "$id")
        [ -e "$wt/.git" ] || die "no worktree at $wt"
        git -C "$wt" add -- "$@"
        # commit's summary goes to stderr so stdout is exactly the hash.
        git -C "$wt" commit --only -m "$msg" -- "$@" >&2
        git -C "$wt" rev-parse HEAD
        ;;

    discard)
        [ $# -ge 1 ] || usage
        id=$1; shift
        check_id "$id"
        wt=$(wt_path "$id")
        if [ "${1:-}" != "--yes" ]; then
            echo "would remove worktree: $wt" >&2
            echo "would delete branch:   attempt/$id" >&2
            echo "re-run with --yes to discard" >&2
            exit 2
        fi
        repo=$(repo_root)
        if [ -e "$wt" ]; then
            git -C "$repo" worktree remove --force "$wt"
        fi
        git -C "$repo" worktree prune
        if branch_exists "$repo" "$id"; then
            git -C "$repo" branch -D "attempt/$id" >&2
        fi
        echo "discarded attempt-$id"
        ;;

    list)
        [ $# -eq 0 ] || usage
        [ -d "$ROOT" ] || exit 0
        for wt in "$ROOT"/attempt-*; do
            [ -d "$wt" ] || continue
            [ -e "$wt/.git" ] || continue
            id=${wt##*/attempt-}
            branch=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "?")
            head=$(git -C "$wt" rev-parse --short HEAD 2>/dev/null || echo "?")
            dirty=$(git -C "$wt" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
            printf '%s\t%s\t%s\t%s\n' "$id" "$branch" "$head" "$dirty"
        done
        ;;

    -h|--help|help)
        usage
        ;;

    *)
        echo "attempt-worktree: unknown command: $cmd" >&2
        usage
        ;;
esac
