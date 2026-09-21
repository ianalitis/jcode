#!/usr/bin/env bash
# bounded.sh: run a command with a hard wall-clock limit, portably.
#
#   scripts/bounded.sh <seconds> <command> [args...]
#
# macOS ships no GNU `timeout`, and unbounded git/cargo invocations have hung
# agent sessions for 20 minutes or more (git merge-tree across 2000-commit
# divergences, cargo test with a wedged env lock). This is the one place that
# knows how to bound a command on this host: perl's alarm delivers SIGALRM to
# the exec'd process, which exits 142; the caller sees a nonzero status and
# the word "timeout" on stderr instead of a silent hang.
#
# Exit status: the command's own status, or 124 when the limit fired (matching
# GNU timeout so scripts can branch on it the same way on Linux).
set -u
if [ $# -lt 2 ]; then
    sed -n '2,15p' "$0" >&2
    exit 2
fi
limit=$1
shift
if command -v timeout >/dev/null 2>&1; then
    exec timeout --foreground "$limit" "$@"
fi
perl -e '
    my $limit = shift @ARGV;
    my $pid = fork();
    die "fork: $!" unless defined $pid;
    if ($pid == 0) { exec @ARGV or die "exec: $!"; }
    local $SIG{ALRM} = sub {
        kill "TERM", $pid;
        select(undef, undef, undef, 2);
        kill "KILL", $pid;
        waitpid($pid, 0);
        print STDERR "bounded.sh: timeout after ${limit}s: @ARGV\n";
        exit 124;
    };
    alarm $limit;
    waitpid($pid, 0);
    my $status = $?;
    alarm 0;
    exit(($status >> 8) || ($status & 127 ? 128 + ($status & 127) : 0));
' "$limit" "$@"
