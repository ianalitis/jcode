#!/usr/bin/env bash
# Branch ledger: classify every local branch, stash and worktree against a base
# so a closeout session starts from a disposition table instead of re-deriving
# one by hand. Read-only: nothing is merged, deleted, or checked out.
#
#   scripts/branch_ledger.sh [--base <ref>] [--timeout <secs>] [--max-behind <n>] [--no-merge-check]
#   scripts/branch_ledger.sh --refs refs/remotes/fork/ --base origin/master
#
# --refs classifies any ref glob with the same columns and dispositions, which
# is how a fork's pushed branches are audited before deleting any of them. The
# worktree and stash sections are local-only and are skipped in that mode.
#
# Columns per branch:
#   unique   commits not in base by patch content (git cherry), over total
#   merge    CLEAN | CONFLICT(n) | TIMEOUT | large | skipped   (git merge-tree dry run;
#            `large` means the branch is more than --max-behind commits behind base,
#            where merge-tree takes minutes and a merge is not the right tool anyway)
#   files    files the clean merge would touch
#   age      last commit date
#   wt       worktree path when checked out somewhere, and * when dirty
#
# Disposition is a suggestion, not authority:
#   integrated   0 unique commits: safe to delete after operator approval
#   relanded     every unique commit's subject already exists in base history:
#                the fix landed via another path; verify by test name, then treat as integrated
#   merge-ready  clean dry-run merge with unique work: candidate for `git merge`
#   conflicts    needs a human or a bounded worker packet per branch
#   cherry       far behind base: cherry-pick the unique commits, do not merge
#   stale        >30 days and conflicts: probably superseded; confirm, then drop
set -euo pipefail

BASE=HEAD
TIMEOUT=60
MAX_BEHIND=500
MERGE_CHECK=true
REFS='refs/heads/'
while [ $# -gt 0 ]; do
    case "$1" in
        --base) BASE=$2; shift 2 ;;
        --refs) REFS=$2; shift 2 ;;
        --timeout) TIMEOUT=$2; shift 2 ;;
        --max-behind) MAX_BEHIND=$2; shift 2 ;;
        --no-merge-check) MERGE_CHECK=false; shift ;;
        -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# A trailing `*` is accepted for convenience, but git's pattern matcher only
# reaches one level with it, so `refs/remotes/fork/*` would miss every branch
# whose name contains a slash. Normalize to the prefix form, which matches all
# of them.
REFS=${REFS%\*}
local_mode=false
[ "$REFS" = refs/heads/ ] && local_mode=true

base_sha=$(git rev-parse --verify "$BASE^{commit}")
current=$(git rev-parse --abbrev-ref HEAD)
now=$(date +%s)

declare -A WT DIRTY
if $local_mode; then
while IFS= read -r line; do
    case "$line" in
        "worktree "*) path=${line#worktree } ;;
        "branch refs/heads/"*)
            b=${line#branch refs/heads/}
            WT[$b]=$path
            n=$(git -C "$path" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
            [ "$n" != "0" ] && DIRTY[$b]=$n
            ;;
    esac
done < <(git worktree list --porcelain)
fi

with_timeout() {
    "$(dirname "$0")/bounded.sh" "$TIMEOUT" "$@"
}

printf '# Branch ledger: base %s (%s), refs %s, %s\n\n' "$BASE" "${base_sha:0:9}" "$REFS" "$(date -u +%FT%TZ)"
printf '| disposition | branch | unique/total | relanded | behind | merge | files | age | worktree |\n'
printf '| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n'

# --exclude drops the per-remote HEAD symref, whose short name is the bare
# remote name (`fork`, not `fork/HEAD`) and which is not a branch.
for b in $(git for-each-ref --format='%(refname:short)' --exclude='refs/remotes/*/HEAD' "$REFS" | sort); do
    [ "$b" = "$current" ] && continue
    sha=$(git rev-parse "$b")
    [ "$sha" = "$base_sha" ] && continue
    mb=$(git merge-base "$base_sha" "$b")
    total=$(git rev-list --count "$mb..$b")
    behind=$(git rev-list --count "$mb..$base_sha")
    unique=$(git cherry "$base_sha" "$b" "$mb" 2>/dev/null | grep -c '^+' || true)
    # unique-by-patch commits whose subject line also exists in base history:
    # usually the same fix re-landed on a different tree (rebase, upstream copy)
    relanded=0
    for u in $(git cherry "$base_sha" "$b" "$mb" 2>/dev/null | awk '/^\+/{print $2}'); do
        subj=$(git log -1 --format=%s "$u")
        if [ -n "$(git log -1 --format=%h --fixed-strings --grep="$subj" "$base_sha")" ]; then
            relanded=$((relanded + 1))
        fi
    done
    age_s=$(git log -1 --format=%ct "$b")
    age_d=$(( (now - age_s) / 86400 ))
    merge=skipped; files=""
    if [ "$unique" = "0" ]; then
        disp=integrated; merge=n/a
    elif [ "$relanded" = "$unique" ]; then
        disp=relanded; merge=n/a
    elif [ "$behind" -gt "$MAX_BEHIND" ]; then
        disp=cherry; merge=large
    elif $MERGE_CHECK; then
        set +e
        r=$(with_timeout git merge-tree --write-tree "$base_sha" "$b" 2>&1)
        rc=$?
        set -e
        if [ $rc -eq 0 ]; then
            tree=$(printf '%s\n' "$r" | head -1)
            files=$(git diff --name-only "$base_sha" "$tree" | wc -l | tr -d ' ')
            merge=CLEAN; disp='merge-ready'
        elif [ $rc -eq 124 ]; then
            merge=TIMEOUT; disp=conflicts
        else
            c=$(printf '%s\n' "$r" | grep -c '^CONFLICT' || true)
            merge="CONFLICT($c)"; disp=conflicts
            [ $age_d -gt 30 ] && disp=stale
        fi
    else
        disp=unknown
    fi
    rm -f "$(git rev-parse --show-toplevel)"/.merge_file_* 2>/dev/null || true
    wt=${WT[$b]:-}
    [ -n "${DIRTY[$b]:-}" ] && wt="$wt *(${DIRTY[$b]} dirty)"
    printf '| %s | `%s` | %s/%s | %s | %s | %s | %s | %sd | %s |\n' \
        "$disp" "$b" "$unique" "$total" "$relanded" "$behind" "$merge" "$files" "$age_d" "$wt"
done

if $local_mode; then
echo
echo '## Stashes'
echo
i=0
git stash list | while IFS= read -r line; do
    ref="stash@{$i}"
    if git stash show -p "$ref" 2>/dev/null | git apply --check --reverse >/dev/null 2>&1; then
        state="already in base (reverse-applies)"
    elif git stash show -p "$ref" 2>/dev/null | git apply --check >/dev/null 2>&1; then
        state="applies cleanly"
    else
        state="conflicts"
    fi
    printf -- '- `%s` %s: %s\n' "$ref" "$line" "$state"
    i=$((i + 1))
done

echo
echo '## Detached worktrees'
git worktree list --porcelain | awk '/^worktree /{p=$2} /^detached$/{print "- " p}'
fi
