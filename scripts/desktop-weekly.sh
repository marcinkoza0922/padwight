#!/usr/bin/env bash
# The weekly desktop run: scripts/desktop-versions.sh, then scripts/desktop-test.sh on every VM in
# vms/ (each vms/<name>.d with a `de` file), then the table in docs/development/qa-plan.md
# section 8 gets each desktop's result, date and kernel (scripts/update-desktop-table.py).
#
# VMs that are not running are started with vms/vm.sh (niri with VM_GL=1) and stopped again after
# their run. Only ever use throwaway VMs: the daemon grabs gamepads and creates input devices.
#
# Usage: scripts/desktop-weekly.sh [--dry-run] [--no-table] [--only name,name] [--vms dir]
#   --dry-run   print what would run for each VM and change nothing (no VM is started)
#   --no-table  leave qa-plan.md alone
#   --only      run just these VMs (names as in `vms/vm.sh list`)
#   --vms       the VM folder, default vms/ in the checkout (DESKTOP_VMS_DIR)
# Logs go to target/desktop-weekly/<date>/ (DESKTOP_WEEKLY_OUT). Exit status is the number of
# desktops that failed.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
vms=${DESKTOP_VMS_DIR:-$root/vms}
out=${DESKTOP_WEEKLY_OUT:-$root/target/desktop-weekly/$(date +%F)}
dry=0 table=1 only=
while [ $# -gt 0 ]; do
    case $1 in
    --dry-run) dry=1 ;;
    --no-table) table=0 ;;
    --only) only=,${2:?--only needs names},; shift ;;
    --vms) vms=${2:?--vms needs a folder}; shift ;;
    *) sed -n '2,17p' "$0"; exit 2 ;;
    esac
    shift
done

# The ssh port vms/vm.sh gives a VM.
port() { echo $((2200 + $(cksum <<<"$1" | cut -d' ' -f1) % 400)); }
alive() { [ -f "$vms/$1.d/pid" ] && kill -0 "$(cat "$vms/$1.d/pid")" 2>/dev/null; }
# What desktop-test.sh calls the desktop of a VM made for `de`.
test_name() { case $1 in ubuntu-*) echo "${1#ubuntu-}" ;; *) echo "$1" ;; esac; }

[ -d "$vms" ] || { echo "no VM folder at $vms" >&2; exit 2; }
mkdir -p "$out"

echo "== desktop versions"
if [ $dry = 1 ]; then
    echo "would run: scripts/desktop-versions.sh"
else
    scripts/desktop-versions.sh 2>&1 | tee "$out/versions.log" || echo "(desktop-versions.sh flagged something; see above)"
fi

declare -A seen
results=()
failed=0
for dir in "$vms"/*.d; do
    [ -f "$dir/de" ] || continue
    name=$(basename "$dir" .d)
    de=$(tr -d '[:space:]' <"$dir/de")
    [ -z "$only" ] || [[ $only == *",$name,"* ]] || continue
    # Several VMs for one desktop: the one named after it wins, otherwise the first.
    if [ -n "${seen[$de]:-}" ] && [ "$name" != "$de" ]; then
        echo "== $name: skipped, $de already has ${seen[$de]}"
        continue
    fi
    seen[$de]=$name
    desktop=$(test_name "$de")
    p=$(port "$name")
    ssh_opts="-p $p -i $vms/id_ed25519 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"
    running=yes
    alive "$name" || running=no
    echo "== $name ($de, desktop-test.sh $desktop, ssh port $p, running: $running)"
    if [ $dry = 1 ]; then
        [ $running = yes ] || echo "would run: $([ "$de" = niri ] && echo 'VM_GL=1 ')vms/vm.sh start $name, and stop it afterwards"
        echo "would run: DESKTOP_TEST_HOST=dev@127.0.0.1 DESKTOP_TEST_SSH_OPTS=\"$ssh_opts\" DESKTOP_TEST_QMP=$vms/$name.d/qmp.sock scripts/desktop-test.sh $desktop > $out/$name.log"
        continue
    fi
    started=0
    if [ $running = no ]; then
        if [ "$de" = niri ]; then export VM_GL=1; fi
        "$vms/vm.sh" start "$name"
        unset VM_GL
        started=1
        sleep 30 # the session needs to finish logging in
    fi
    status=0
    DESKTOP_TEST_HOST=dev@127.0.0.1 DESKTOP_TEST_SSH_OPTS="$ssh_opts" DESKTOP_TEST_QMP="$vms/$name.d/qmp.sock" \
        scripts/desktop-test.sh "$desktop" >"$out/$name.log" 2>&1 || status=$?
    tail -n 12 "$out/$name.log"
    [ $started = 0 ] || "$vms/vm.sh" stop "$name" || true
    [ $status = 0 ] || failed=$((failed + 1))
    kernel=$(sed -n 's/^kernel //p' "$out/$name.log" | head -1)
    results+=("$de	$status	${kernel:-unknown}	$(date +%F)")
done

if [ $dry = 1 ]; then
    echo "== dry run: nothing was started or changed"
    exit 0
fi
printf '%s\n' "${results[@]}" >"$out/results.tsv"
if [ $table = 1 ] && [ ${#results[@]} -gt 0 ]; then
    python3 -I scripts/update-desktop-table.py docs/development/qa-plan.md "$out/results.tsv"
fi
echo "== $failed desktop(s) failed; logs in $out"
exit "$failed"
