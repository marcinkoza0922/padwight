#!/usr/bin/env bash
# OVL-10 (CR H-13) as pass or fail: with no audio output at all, overlay cues are dropped and the
# daemon logs "no audio device" once; when an output comes back, the next cue plays without a
# restart. The daemon's ALSA configuration (ALSA_CONFIG_PATH, a file of its own, so the machine's
# sound stays untouched) first points the default device at a sound card that does not exist, then,
# in the same running daemon, at a `file` device that records what is played. No sound server, card
# or speaker is involved, so this checks the daemon's handling of a missing and returning device,
# not a particular sound server (ENV-6 is that, by hand). A `--debug --fake-clock` daemon runs in
# bwrap with /dev/input hidden, so it takes no real controller.
#
# Usage: scripts/audio-recovery-check.sh        (about 20 seconds; PADWIGHT_AUDIO_KEEP=1 keeps the logs)
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
for tool in bwrap; do
    command -v "$tool" >/dev/null || { echo "$tool is missing" >&2; exit 2; }
done
cd "$root"
cargo build --quiet --bin padwight
bin="${CARGO_TARGET_DIR:-$root/target}/debug/padwight"
# Short on purpose: unix socket paths have a length limit.
dir=$(mktemp -d /tmp/pwaudio.XXXXXX)
mkdir -p "$dir/run" "$dir/config" "$dir/state" "$dir/data"
chmod 700 "$dir/run"
daemon=
session=
cleanup() {
    kill $daemon $session 2>/dev/null || true
    if [ -n "${PADWIGHT_AUDIO_KEEP:-}" ]; then echo "kept $dir" >&2; else rm -rf "$dir"; fi
}
trap cleanup EXIT

# The default device with nothing behind it.
missing() {
    cat >"$dir/asound.conf.new" <<CONF
pcm.!default { type hw card 99 }
ctl.!default { type hw card 99 }
CONF
    mv "$dir/asound.conf.new" "$dir/asound.conf"
}
# The default device back: a file device that records the samples it is given.
back() {
    cat >"$dir/asound.conf.new" <<CONF
pcm.sink { type null }
pcm.!default {
    type file
    slave.pcm "sink"
    file "$dir/played.raw"
    format "raw"
}
CONF
    mv "$dir/asound.conf.new" "$dir/asound.conf"
}
missing

export XDG_RUNTIME_DIR=$dir/run XDG_CONFIG_HOME=$dir/config XDG_STATE_HOME=$dir/state XDG_DATA_HOME=$dir/data
export TMPDIR=$dir/run PADWIGHT_DEBUG_SOCKET=$dir/run/debug.sock PADWIGHT_NO_DAEMON=1 ALSA_CONFIG_PATH=$dir/asound.conf
unset WAYLAND_DISPLAY DISPLAY PULSE_SERVER

hide=(--tmpfs /dev/input)
for node in /dev/hidraw*; do
    [ -e "$node" ] && hide+=(--ro-bind /dev/null "$node")
done
bwrap --die-with-parent --dev-bind / / "${hide[@]}" -- "$bin" daemon --debug --fake-clock >"$dir/daemon.log" 2>&1 &
daemon=$!
for _ in $(seq 100); do [ -S "$dir/run/padwight.sock" ] && break; sleep 0.1; done
"$bin" debug --headless --attach >"$dir/session.log" 2>&1 &
session=$!
pad() { "$bin" debug --wait 5000 "$@" >/dev/null; }
pad ping

no_device() { grep -c 'no audio device for overlay sounds' "$dir/daemon.log" || true; }
played() { stat -c %s "$dir/played.raw" 2>/dev/null || echo 0; }
# One overlay cue: the Guide overlay's cursor takes a step.
cue() { pad tap dpad-down 30; pad advance 100; }

fail=0
check() { # check <what> <got> <wanted>
    if [ "$2" = "$3" ]; then echo "  ok    $1: $2"; else echo "  FAIL  $1: $2, wanted $3"; fail=1; fi
}

echo "== no audio device"
pad tap guide 60
pad advance 400
for _ in 1 2 3; do cue; sleep 0.3; done
check "'no audio device' lines in the daemon log" "$(no_device)" "1"
check "bytes played" "$(played)" "0"

echo "== the device comes back, the daemon keeps running"
back
recovered=0
for _ in $(seq 10); do
    cue
    sleep 0.5
    if [ "$(played)" -gt 0 ]; then recovered=1; break; fi
done
check "a cue played into the returned device (1 = yes)" "$recovered" "1"
check "'no audio device' lines still" "$(no_device)" "1"
grep -i 'audio' "$dir/daemon.log" | head -3 | sed 's/^/  log: /'
pad quit || true
exit $fail
