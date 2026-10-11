#!/usr/bin/env bash
# Drives the real daemon and overlay on a headless KWin with the debug controller, and records
# what happens: every frame the daemon sends the overlay, and the overlay's own timing log (each
# frame received, each build, and how each menu was drawn in it). For debugging overlay timing
# and motion without a monitor or a controller.
#
# The daemon runs with a copy of a config folder, its own runtime folder (so it can't reach the
# real daemon, desktop or speakers) and no /dev/input or hidraw devices (so it can't take the real
# controllers). Needs kwin_wayland, dbus-run-session, bwrap and python3.
#
# Usage: scripts/overlay-harness.sh <scenario> [out dir]     (default: screenshots/harness)
#   The scenario is a shell script run in the session once the controller is attached. `pad`
#   runs `padwight debug` against the session (pad press South, pad tap up 40, ...), `mark
#   <text>` notes something in the logs, and `shot <name>` saves a screenshot. Screenshots show
#   layout only: KWin's virtual output lags and part-updates what the overlay draws, so they
#   don't show a fade as it plays (src/overlay/fade_frames.rs draws fades in-process instead).
#   scripts/harness/analyze.py sums up the logs; scripts/harness/*.sh are scenarios.
#   PADWIGHT_HARNESS_CONFIG=<dir>   the config folder to copy (default scripts/harness/config:
#                                   South holds a directional menu, East a radial one)
#   PADWIGHT_HEADLESS_SIZE=1920x1080
#
# Out: daemon.log, frames.log (`<ns> <frame json>` from the daemon), overlay.log (the overlay's
# timing log, with `mark` lines), and any screenshots.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
scenario=$(realpath "${1:?usage: overlay-harness.sh <scenario> [out dir]}")
out=$(realpath -m "${2:-$root/screenshots/harness}")
config=${PADWIGHT_HARNESS_CONFIG:-$root/scripts/harness/config}
size=${PADWIGHT_HEADLESS_SIZE:-1920x1080}
for tool in kwin_wayland dbus-run-session bwrap python3; do
    command -v "$tool" >/dev/null || { echo "$tool is missing" >&2; exit 1; }
done

cd "$root"
cargo build --quiet --bin padwight
bin="${CARGO_TARGET_DIR:-$root/target}/debug/padwight"

home=$(mktemp -d "${TMPDIR:-/tmp}/padwight-harness.XXXXXX")
trap 'rm -rf "$home"' EXIT
mkdir -p "$home/run" "$home/config" "$home/state" "$home/data" "$home/tmp" "$out"
chmod 700 "$home/run"
cp -r "$config" "$home/config/padwight"
rm -f "$out"/*.log "$out"/*.png

# Logs each frame the daemon sends the overlay, with when it was received.
cat >"$home/frames.py" <<'PY'
import json, os, socket, sys, time
path = os.path.join(os.environ["XDG_RUNTIME_DIR"], "padwight.sock")
for _ in range(100):
    try:
        conn = socket.socket(socket.AF_UNIX)
        conn.connect(path)
        break
    except OSError:
        time.sleep(0.1)
conn.sendall((json.dumps("WatchOverlay") + "\n").encode())
with open(sys.argv[1], "a") as log:
    for line in conn.makefile():
        log.write(f"{time.time_ns()} {line}")
        log.flush()
PY

hide=(--tmpfs /dev/input)
for node in /dev/hidraw*; do
    [ -e "$node" ] && hide+=(--ro-bind /dev/null "$node")
done

socket="padwight-harness-$$"
cat >"$home/session.sh" <<SH
#!/bin/bash
set -u
export WAYLAND_DISPLAY=$socket QT_QPA_PLATFORM=wayland
export XDG_CONFIG_HOME=$home/config XDG_STATE_HOME=$home/state XDG_DATA_HOME=$home/data
export XDG_RUNTIME_DIR=$home/run TMPDIR=$home/tmp PADWIGHT_NO_DAEMON=1
export PADWIGHT_OVERLAY_TIMING=$out/overlay.log PADWIGHT_DEBUG_SOCKET=$home/run/debug.sock
pad() { "$bin" debug --wait 5000 "\$@"; }
mark() { echo "mark \$(date +%s%N) \$*" >>"$out/overlay.log"; }
shot() { spectacle -b -n -f -o "$out/\$1.png" >/dev/null 2>&1; }
bwrap --die-with-parent --dev-bind / / ${hide[*]} -- "$bin" daemon --debug >"$out/daemon.log" 2>&1 &
daemon=\$!
python3 "$home/frames.py" "$out/frames.log" &
watcher=\$!
for _ in \$(seq 200); do [ -S $home/run/padwight.sock ] && break; sleep 0.1; done
sleep 1
"$bin" debug --headless --attach >"$out/session.log" 2>&1 &
pad ping >/dev/null
sleep 1
. "$scenario"
pad quit >/dev/null 2>&1 || true
kill \$watcher \$daemon 2>/dev/null
wait \$daemon 2>/dev/null
SH
chmod +x "$home/session.sh"

XDG_RUNTIME_DIR="$home/run" setsid --wait dbus-run-session -- \
    kwin_wayland --virtual --socket "$socket" --width "${size%x*}" --height "${size#*x}" \
    --no-lockscreen --no-global-shortcuts --exit-with-session \
    "$home/session.sh" >"$out/kwin.log" 2>&1 &
sid=$!
wait "$sid" || true
pkill -s "$sid" 2>/dev/null || true
echo "logs in $out"
