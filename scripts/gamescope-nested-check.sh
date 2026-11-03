#!/bin/sh
# ENV-3, the nested half: runs a program inside a real nested gamescope on a virtual X server and
# asks Padwight's own lookup what it makes of it. Shows two things the plan wants written down:
# what the outer desktop sees (one window, gamescope's), and that the game inside is still found
# as a running process by its executable.
#
# Needs gamescope, Xvfb and xwininfo (x11-utils) and a Vulkan driver that works without a GPU
# (Mesa's lavapipe: mesa-vulkan-drivers). In the Debian container the tests use:
#   apt-get install -y gamescope xvfb x11-utils mesa-vulkan-drivers
# Steam's game mode session (gamescope as the compositor) and gamescope nested in Plasma or
# Hyprland are not covered: those need the desktops.
#
# Usage: scripts/gamescope-nested-check.sh [padwight binary, default: target/debug/padwight]
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
bin=${1:-$root/target/debug/padwight}
for tool in gamescope Xvfb xwininfo; do
    command -v "$tool" >/dev/null || { echo "$tool is missing" >&2; exit 1; }
done

work=$(mktemp -d)
pids=""
cleanup() {
    for p in $pids; do kill "$p" 2>/dev/null || true; done
    rm -rf "$work"
}
trap cleanup EXIT

# A program with the executable name the library test pack's rule matches.
mkdir -p "$work/config" "$work/state" "$work/run"
cp "$(command -v sleep)" "$work/scenario-scenario-input.exe"
export XDG_CONFIG_HOME=$work/config XDG_STATE_HOME=$work/state XDG_RUNTIME_DIR=$work/run
chmod 700 "$work/run"
"$bin" debug install-pack "$root/tests/scenarios/packs/input.padpack" Keys

display=:77
Xvfb "$display" -screen 0 1280x720x24 >/dev/null 2>&1 &
pids="$pids $!"
sleep 1
export DISPLAY=$display

gamescope -W 640 -H 480 -- "$work/scenario-scenario-input.exe" 120 >"$work/gamescope.log" 2>&1 &
scope=$!
pids="$pids $scope"

echo "waiting for gamescope's window on the outer display"
seen=""
for _ in $(seq 1 30); do
    seen=$(xwininfo -root -tree | grep -i gamescope || true)
    [ -n "$seen" ] && break
    kill -0 "$scope" 2>/dev/null || { cat "$work/gamescope.log"; echo "FAIL: gamescope exited" >&2; exit 1; }
    sleep 1
done
[ -n "$seen" ] || { cat "$work/gamescope.log"; echo "FAIL: no gamescope window appeared" >&2; exit 1; }
echo "the outer display sees: $seen"
if xwininfo -root -tree | grep -qi 'scenario-scenario-input'; then
    echo "note: the game's own window is visible on the outer display too"
else
    echo "the game's own window is not on the outer display: only gamescope's is"
fi

echo "the focus lookup for gamescope's window:"
"$bin" debug focus "$scope" gamescope | tee "$work/window.txt"
grep -q 'program  gamescope' "$work/window.txt" || { echo "FAIL: the window was not named as gamescope" >&2; exit 1; }

echo "the running-process lookup:"
"$bin" debug focus processes | tee "$work/processes.txt"
grep -q 'scenario-scenario-input.exe' "$work/processes.txt" || { echo "FAIL: the game inside gamescope was not found as a process" >&2; exit 1; }

echo "gamescope nested: the window is gamescope's, the game is found by its executable"
