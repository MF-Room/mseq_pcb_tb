#!/usr/bin/env bash
# Runs a command with a time limit in seconds, like GNU timeout(1), on Linux and macOS.
# GNU timeout is used when it is installed (Linux, Homebrew coreutils). macOS does not ship it,
# so the fallback is a small perl program with the same behaviour: it starts the command, sends
# it SIGTERM when the limit expires (SIGKILL 5 s later if it is still there) and exits with 124,
# forwards SIGTERM/SIGINT/SIGHUP to it, and otherwise exits with the command's own status.
# Usage: ./with_timeout.sh <seconds> <command> [args...]
set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "Usage: $0 <seconds> <command> [args...]" >&2
    exit 2
fi

if command -v timeout >/dev/null 2>&1; then
    exec timeout "$@"
fi

exec perl -e '
    my $secs = shift @ARGV;
    my $pid = fork();
    defined $pid or die "with_timeout.sh: fork: $!\n";
    if ($pid == 0) {
        exec @ARGV;
        print STDERR "with_timeout.sh: $ARGV[0]: $!\n";
        exit 127;
    }
    my $timed_out = 0;
    $SIG{ALRM} = sub { kill($timed_out++ ? "KILL" : "TERM", $pid); alarm 5 };
    $SIG{TERM} = $SIG{INT} = $SIG{HUP} = sub { kill "TERM", $pid };
    alarm $secs;
    1 until waitpid($pid, 0) == $pid;
    exit 124 if $timed_out;
    exit(($? & 127) ? 128 + ($? & 127) : $? >> 8);
' "$@"
