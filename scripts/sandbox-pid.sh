#!/bin/sh
# Runs a program in a pid namespace of its own, as a Flatpak or Steam runtime sandbox does, and
# prints the pid it has in there next to its pid here. A window from such a program can report
# the inner pid, which names some unrelated process here; check what Padwight makes of it with
#   padwight debug focus <inner pid> <window class>
# Usage: scripts/sandbox-pid.sh [program [args...]]   (default: sleep 60)
# Needs unprivileged user namespaces (unshare --user). Ctrl+C stops the program.
# Exit status: 0 ok, 1 no sandboxed process found, 77 user namespaces are refused (Ubuntu's AppArmor
# restriction or a kernel switch); the distro matrix reports 77 as a skip.
[ $# -eq 0 ] && set -- sleep 60
userns_refused() {
    echo "user namespaces are refused here: $1" >&2
    exit 77
}
[ "$(cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns 2>/dev/null)" = 1 ] &&
    userns_refused "kernel.apparmor_restrict_unprivileged_userns=1; run 'sudo sysctl kernel.apparmor_restrict_unprivileged_userns=0' or give unshare an AppArmor profile with 'userns,'"
[ "$(cat /proc/sys/kernel/unprivileged_userns_clone 2>/dev/null)" = 0 ] &&
    userns_refused "kernel.unprivileged_userns_clone=0; run 'sudo sysctl kernel.unprivileged_userns_clone=1'"
probe=$(unshare --user --map-root-user true 2>&1) ||
    case $probe in
    *uid_map* | *"Operation not permitted"*) userns_refused "$probe (on Ubuntu: kernel.apparmor_restrict_unprivileged_userns=1, see above)" ;;
    esac
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
