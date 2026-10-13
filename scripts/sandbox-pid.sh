#!/bin/sh
# Runs a program in a pid namespace of its own, as a Flatpak or Steam runtime sandbox does, and
# prints the pid it has in there next to its pid here. A window from such a program can report
# the inner pid, which names some unrelated process here; check what Padwight makes of it with
#   padwight debug focus <inner pid> <window class>
# Usage: scripts/sandbox-pid.sh [program [args...]]   (default: sleep 60)
# Needs unprivileged user namespaces (unshare --user). Ctrl+C stops the program.
[ $# -eq 0 ] && set -- sleep 60
tag="padwight-sandbox-$$"
PADWIGHT_SANDBOX_TAG=$tag unshare --user --map-root-user --pid --fork --kill-child --mount-proc "$@" &
outer=$!
# unshare --fork ignores TERM and INT; KILL it, and --kill-child takes the program down too.
trap 'kill -KILL $outer 2>/dev/null; exit' INT TERM
# The program is the process carrying the tag that is in a namespace below ours.
for _ in 1 2 3 4 5 6 7 8 9 10; do
    for status in /proc/[0-9]*/status; do
        dir=${status%/status}
        grep -qs "PADWIGHT_SANDBOX_TAG=$tag" "$dir/environ" 2>/dev/null || continue
        nspid=$(awk '/^NSpid:/ { $1 = ""; print }' "$status")
        set -- $nspid
        [ $# -gt 1 ] || continue
        eval "inner=\${$#}"
        echo "pid here: $1  pid in the sandbox: $inner  ($(cat "$dir/comm"))"
        echo "try: cargo run -q -- debug focus $inner $(cat "$dir/comm")"
        wait $outer
        exit 0
    done
    sleep 0.1
done
echo "no sandboxed process found (are unprivileged user namespaces switched off?)" >&2
kill -KILL $outer 2>/dev/null
exit 1
