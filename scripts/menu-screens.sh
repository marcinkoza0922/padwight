#!/usr/bin/env bash
# Screenshots of the menu scenes in src/menu/suite/screens.rs, drawn by the real overlay process
# on a headless KWin. No daemon runs: a stand-in on the overlay's socket hands it one frame per
# scene, and spectacle captures the screen. Needs kwin_wayland, dbus-run-session and spectacle.
#
# Usage: scripts/menu-screens.sh [out dir]     (default: screenshots/menus, which git ignores)
#   PADWIGHT_HEADLESS_SIZE=1920x1080   the virtual screen's size
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=$(realpath -m "${1:-$root/screenshots/menus}")
size=${PADWIGHT_HEADLESS_SIZE:-1920x1080}
for tool in kwin_wayland dbus-run-session spectacle python3; do
    command -v "$tool" >/dev/null || { echo "$tool is missing" >&2; exit 1; }
done

home=$(mktemp -d "${TMPDIR:-/tmp}/padwight-screens.XXXXXX")
trap 'rm -rf "$home"' EXIT
mkdir -p "$home/run" "$home/config" "$home/state" "$out"
chmod 700 "$home/run"

cd "$root"
PADWIGHT_SCREEN_FRAMES="$home/frames" RUST_BACKTRACE=0 \
    cargo nextest run --profile agent --hide-progress-bar --cargo-quiet --run-ignored only menu::suite::screens
cargo build --quiet --bin padwight
bin="${CARGO_TARGET_DIR:-$root/target}/debug/padwight"

# The stand-in daemon: answers the overlay's watch request with each scene's frame in turn,
# and captures the screen once it has settled.
cat >"$home/play.py" <<'PY'
import json, os, socket, subprocess, sys, time
frames, out, bin = sys.argv[1:4]
path = os.path.join(os.environ["XDG_RUNTIME_DIR"], "padwight.sock")
server = socket.socket(socket.AF_UNIX)
server.bind(path)
server.listen(1)
overlay = subprocess.Popen([bin, "overlay"])
conn, _ = server.accept()
conn.makefile().readline()
def send(frame):
    conn.sendall((json.dumps(frame) + "\n").encode())
time.sleep(2)
for line in open(frames):
    name, frame = line.rstrip("\n").split("\t", 1)
    send({"info": [], "active": None})
    time.sleep(0.8)
    conn.sendall((frame + "\n").encode())
    time.sleep(1.5)
    shot = os.path.join(out, name + ".png")
    subprocess.run(["spectacle", "-b", "-n", "-f", "-o", shot], check=False, timeout=30)
    print("captured" if os.path.exists(shot) else "missing", shot, flush=True)
overlay.terminate()
PY

socket_name="padwight-screens-$$"
session="$home/session.sh"
{
    printf '#!/bin/sh\nexec env WAYLAND_DISPLAY=%q QT_QPA_PLATFORM=wayland PADWIGHT_NO_DAEMON=1' "$socket_name"
    printf ' %q' XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state" XDG_RUNTIME_DIR="$home/run" \
        python3 "$home/play.py" "$home/frames" "$out" "$bin"
    printf '\n'
} >"$session"
chmod +x "$session"
# A session of its own, so the helpers spectacle starts on the private bus (the portal, the
# secrets service) can be stopped with it rather than left holding the terminal.
XDG_RUNTIME_DIR="$home/run" setsid --wait dbus-run-session -- \
    kwin_wayland --virtual --socket "$socket_name" --width "${size%x*}" --height "${size#*x}" \
    --no-lockscreen --no-global-shortcuts --exit-with-session "$session" &
sid=$!
wait "$sid" || true
pkill -s "$sid" 2>/dev/null || true
echo "screenshots in $out"
