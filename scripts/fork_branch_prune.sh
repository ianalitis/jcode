#!/usr/bin/env bash
# Prune branches on a fork remote, using evidence rather than a hand-written list.
#
#   scripts/fork_branch_prune.sh                 # dry run, tier `safe`
#   scripts/fork_branch_prune.sh --tier unique   # the branches that hold unique work
#   scripts/fork_branch_prune.sh --archive       # also keep each tip in this clone
#   scripts/fork_branch_prune.sh --keep <branch> # spare a named branch (repeatable)
#   scripts/fork_branch_prune.sh --apply         # actually delete (needs approval)
#
# Tiers, derived live at every run (never from a stored list):
#   safe    every unique-by-patch commit is either absent (integrated) or present
#           by subject (relanded) in `origin/master`, in the integration line, or
#           in both. Deleting loses nothing that the bases do not already carry.
#   unique  at least one commit is unique against both bases, so the branch holds
#           unmerged content and is only deletable on an explicit decision.
#
# Refused in every mode: the remote's default branch, the current branch, the
# integration line, and the head of any open PR against upstream or against the
# fork itself. Those are read from the API at run time, not from a list.
#
# Read-only unless --apply. Even with --apply it never force-pushes and never
# rewrites an existing ref.
set -uo pipefail

REMOTE=fork
UPSTREAM_REPO=1jehuang/jcode
LINE=jcode/ci-format-baseline
TIER=safe
DEFAULT_BRANCH=master
ARCHIVE=false
APPLY=false
EXTRA_KEEP=()

usage() { sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --remote) REMOTE=$2; shift 2 ;;
        --tier) TIER=$2; shift 2 ;;
        --line) LINE=$2; shift 2 ;;   # empty string means the repo has no integration line
        --upstream) UPSTREAM_REPO=$2; shift 2 ;;
        --keep) EXTRA_KEEP+=("$2"); shift 2 ;;
        --default-branch) DEFAULT_BRANCH=$2; shift 2 ;;
        --archive) ARCHIVE=true; shift ;;
        --apply) APPLY=true; shift ;;
        -h|--help) usage ;;
        *) die "unknown arg: $1" ;;
    esac
done

case "$TIER" in safe|unique) ;; *) die "--tier must be safe or unique" ;; esac

url=$(git remote get-url "$REMOTE") || die "no such remote: $REMOTE"
owner_repo=$(printf '%s' "$url" | sed -E 's#(git@[^:]+:|https://[^/]+/)##; s#\.git$##')
OWNER=${owner_repo%%/*}
FORK_REPO=$owner_repo

[ -n "$OWNER" ] || die "cannot derive an owner from $url"

printf 'remote %s (%s), integration line %s, tier %s\n\n' "$REMOTE" "$FORK_REPO" "$LINE" "$TIER"

# Heads of open PRs anywhere: upstream PRs opened from this owner, plus PRs open
# on the fork itself. Both are live work and must survive.
open_pr_heads() {
    gh pr list --repo "$UPSTREAM_REPO" --state open --limit 300 \
        --json headRefName,headRepositoryOwner \
        --jq ".[] | select(.headRepositoryOwner.login == \"$OWNER\") | .headRefName" 2>/dev/null
    gh pr list --repo "$FORK_REPO" --state open --limit 300 --json headRefName --jq '.[].headRefName' 2>/dev/null
}

declare -A KEEP
KEEP[$DEFAULT_BRANCH]=1
[ -n "$LINE" ] && KEEP[$LINE]=1
while IFS= read -r h; do
    [ -n "$h" ] && KEEP[$h]=1
done < <(open_pr_heads)
for h in ${EXTRA_KEEP[@]+"${EXTRA_KEEP[@]}"}; do KEEP[$h]=1; done

line_sha=$(git rev-parse --verify "refs/heads/$LINE" 2>/dev/null || git rev-parse --verify "$LINE" 2>/dev/null || true)
upstream_sha=$(git rev-parse --verify origin/master 2>/dev/null || true)
[ -n "$upstream_sha" ] || die "origin/master is not fetched; run git fetch origin master"

# unique-by-patch count in <base>..<ref>, then how many of those commit subjects
# already exist in <base> (the same fix landed by another path).
classify_against() {
    local base=$1 ref=$2
    local mb unique relanded subj
    mb=$(git merge-base "$base" "$ref" 2>/dev/null) || { echo "0 0"; return; }
    unique=$(git cherry "$base" "$ref" "$mb" 2>/dev/null | grep -c '^+' || true)
    relanded=0
    if [ "$unique" != "0" ]; then
        while read -r u; do
            [ -n "$u" ] || continue
            subj=$(git log -1 --format=%s "$u" 2>/dev/null)
            # Test the captured output, not git log's exit status: git log exits 0
            # even when --grep matches nothing, so an exit-status test would call
            # every branch relanded and delete branches holding unique work.
            if [ -n "$subj" ] && [ -n "$(git log -1 --format=%h --fixed-strings --grep="$subj" "$base" 2>/dev/null)" ]; then
                relanded=$((relanded + 1))
            fi
        done < <(git cherry "$base" "$ref" "$mb" 2>/dev/null | awk '/^\+/{print $2}')
    fi
    echo "$unique $relanded"
}

stamp=$(date -u +%Y-%m-%d)
safe=(); unique=()
refused=()
for ref in $(git for-each-ref --format='%(refname)' --exclude='refs/remotes/*/HEAD' "refs/remotes/$REMOTE/" | sort); do
    b=${ref#refs/remotes/$REMOTE/}
    if [ -n "${KEEP[$b]:-}" ]; then refused+=("$b"); continue; fi
    read -r u_up r_up <<<"$(classify_against "$upstream_sha" "$ref")"
    if [ -n "$line_sha" ]; then
        read -r u_ln r_ln <<<"$(classify_against "$line_sha" "$ref")"
    else
        u_ln=0; r_ln=0
    fi
    safe_here=false
    if [ "$u_up" = "0" ] || [ "$r_up" = "$u_up" ]; then safe_here=true; fi
    # The integration line is optional. When it is absent, u_ln/r_ln are both 0
    # and this clause would otherwise be trivially true and call every branch
    # integrated: a repo without the line must be judged on upstream alone.
    if [ -n "$line_sha" ] && { [ "$u_ln" = "0" ] || [ "$r_ln" = "$u_ln" ]; }; then safe_here=true; fi
    if [ "$safe_here" = true ]; then
        safe+=("$b")
    else
        unique+=("$b")
    fi
done

printf 'safe   (integrated or relanded in a base): %s\n' "${#safe[@]}"
printf 'unique (unmerged content, needs a decision): %s\n' "${#unique[@]}"
printf 'refused (default branch, integration line, open PR heads): %s\n\n' "${#refused[@]}"

case "$TIER" in
    safe) selected=("${safe[@]:-}") ;;
    unique) selected=("${unique[@]:-}") ;;
esac

if [ "${#selected[@]}" -eq 0 ] || [ -z "${selected[0]:-}" ]; then
    echo "nothing to do"
    exit 0
fi

for b in "${selected[@]}"; do
    [ -n "$b" ] || continue
    sha=$(git rev-parse "refs/remotes/$REMOTE/$b" 2>/dev/null || echo '?')
    printf '%-58s %s\n' "$b" "$sha"
done

echo
if [ "$ARCHIVE" = true ]; then
    echo "archive refs that would be created (refs/archive/$REMOTE-$stamp/<branch>):"
    for b in "${selected[@]}"; do
        [ -n "$b" ] || continue
        printf '  refs/archive/%s-%s/%s\n' "$REMOTE" "$stamp" "$b"
    done
fi

if [ "$APPLY" != true ]; then
    echo
    echo "dry run: nothing deleted. Re-run with --apply (and operator approval) to delete."
    exit 0
fi

echo
for b in "${selected[@]}"; do
    [ -n "$b" ] || continue
    sha=$(git rev-parse "refs/remotes/$REMOTE/$b" 2>/dev/null) || continue
    if [ "$ARCHIVE" = true ]; then
        git update-ref "refs/archive/$REMOTE-$stamp/$b" "$sha" || die "archive failed for $b"
    fi
    if git push "$REMOTE" --delete "$b" >/dev/null 2>&1; then
        printf 'deleted  %s\n' "$b"
        git update-ref -d "refs/remotes/$REMOTE/$b" >/dev/null 2>&1 || true
    else
        printf 'FAILED   %s (left in place)\n' "$b"
    fi
done
