#!/bin/sh
# Soak and chaos run (tests/soak): a --debug daemon driven by seeded random steps in a container,
# with invariant checks (nothing held in the kernel, no panic, memory/fds/threads bounded).
# Results (soak.csv, steps.log, daemon.log) go to $PADWIGHT_SOAK_OUT (default soak-runs/ here,
# which is untracked).
#
# Usage: scripts/soak.sh              60 s run, random seed (printed)
#   PADWIGHT_SOAK_SECONDS=600 PADWIGHT_SOAK_CHECK_SECONDS=60 scripts/soak.sh
#   PADWIGHT_SOAK_SEED=123 PADWIGHT_SOAK_STEPS=400 scripts/soak.sh    replay a failure
# Overnight: PADWIGHT_SOAK_SECONDS=28800 scripts/soak.sh. More settings: PADWIGHT_SOAK_PADS,
# _RSS_KB, _FDS, _THREADS, _FOCUS_HOOK, _FOCUS_CLASSES (see the header of tests/soak/main.rs).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out=${PADWIGHT_SOAK_OUT:-$root/soak-runs}
mkdir -p "$out"
for dev in /dev/uinput /dev/uhid; do
    [ -e "$dev" ] || { echo "$dev is missing: sudo modprobe ${dev#/dev/}" >&2; exit 1; }
done
find "$root/src" "$root/tests" -name '*.rs' -exec touch {} +
status=0
docker run --rm \
    --device /dev/uinput --device /dev/uhid --device-cgroup-rule 'c 13:* rwm' \
    -v /dev/input:/dev/input -v "$root":/src -v "$out":/out \
    -v padwight-kernel-target:/target -v padwight-kernel-cargo:/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/target -e RUST_BACKTRACE=0 -e PADWIGHT_SOAK_OUT=/out \
    -e PADWIGHT_SOAK_SECONDS -e PADWIGHT_SOAK_STEPS -e PADWIGHT_SOAK_SEED -e PADWIGHT_SOAK_CHECK_SECONDS \
    -e PADWIGHT_SOAK_CHECK_STEPS -e PADWIGHT_SOAK_PADS -e PADWIGHT_SOAK_RSS_KB -e PADWIGHT_SOAK_FDS \
    -e PADWIGHT_SOAK_THREADS -e PADWIGHT_SOAK_FOCUS_HOOK -e PADWIGHT_SOAK_FOCUS_CLASSES \
    -w /src -u 0 rust:1 \
    sh -c "apt-get update -qq && apt-get install -y -qq pkg-config libxkbcommon-dev libwayland-dev libudev-dev libfontconfig1-dev libdbus-1-dev libasound2-dev >/dev/null \
        && cargo test --test soak -- --ignored --nocapture soak_run; s=\$?; chown -R $(id -u):$(id -g) /out; exit \$s" || status=$?
exit $status
