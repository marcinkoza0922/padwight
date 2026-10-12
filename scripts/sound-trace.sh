#!/usr/bin/env bash
# Shows which overlay cue a real daemon plays for each input, with no display and no speakers
# needed. It starts a `--debug` daemon on its own config and runtime folders (your session and
# daemon are left alone), attaches a virtual controller, drives the Guide overlay and the menus
# it leads to, and prints the daemon's `sound:` lines: overlay, cue, kind and settings.
#
# Usage: scripts/sound-trace.sh [config.toml|-]     (default: the built-in defaults)
#   A config file is copied in as the App settings, so its [sounds.*] tables apply. A setup's own
#   sounds go in the setup's file instead: set SETUP_TOML to a file appended to General's setup.toml.
#   PADWIGHT_BIN=path   the binary to run (default: target/debug/padwight, built if missing)
#   PADWIGHT_TRACE_KEEP=1   keep the folders and print where they are
set -euo pipefail
cd "$(dirname "$0")/.."
bin=${PADWIGHT_BIN:-target/debug/padwight}
[ -x "$bin" ] || cargo build -q
bin=$(realpath "$bin")
# Short on purpose: a unix socket path has a length limit.
dir=$(mktemp -d /tmp/pwtrace.XXXXXX)
mkdir -p "$dir/run" "$dir/cfg/padwight/general"
chmod 700 "$dir/run"
export XDG_RUNTIME_DIR=$dir/run XDG_CONFIG_HOME=$dir/cfg
daemon=
cleanup() {
    [ -n "$daemon" ] && kill "$daemon" 2>/dev/null || true
    [ -n "${PADWIGHT_TRACE_KEEP:-}" ] && echo "kept $dir" >&2 || rm -rf "$dir"
}
trap cleanup EXIT
if [ "${1:--}" != "-" ]; then cp "$1" "$dir/cfg/padwight/config.toml"; fi
if [ -n "${SETUP_TOML:-}" ]; then
    printf 'name = "General"\nrules = []\n' >"$dir/cfg/padwight/general/setup.toml"
    cat "$SETUP_TOML" >>"$dir/cfg/padwight/general/setup.toml"
fi
setsid "$bin" daemon --debug >"$dir/daemon.log" 2>&1 &
daemon=$!
for _ in $(seq 100); do [ -S "$dir/run/padwight.sock" ] && break; sleep 0.1; done
setsid "$bin" debug --headless --attach >"$dir/session.log" 2>&1 &
d() { "$bin" debug --wait 10000 "$@"; sleep 0.4; }
echo "# Guide overlay: open, step, mode, close" >&2
d tap guide 100; d tap dpad-down 100; d tap dpad-right 100; d tap east 100
echo "# Guide overlay: pick the first item (opens Quick Settings), step in it, close" >&2
d tap guide 100; d tap south 100; d tap dpad-down 100; d tap east 100
echo "# Guide + Start menu" >&2
d press guide; d tap start 100; d release guide; d tap dpad-down 100; d tap east 100
grep -a "sound:" "$dir/daemon.log" || echo "no cues played" >&2
