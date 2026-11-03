#!/bin/sh
# Runs the kernel-level uinput, daemon and GUI tests in a container, under a virtual X server (Xvfb),
# so the test user doesn't need to be in the host's `input` group. The container runs as root and reads the virtual keyboard and mouse
# nodes through a bind mount of /dev/input. Your host account is unchanged.
#
# Caveat: the container can read every host input device, including your real keyboard. The
# test only opens its own virtual nodes, but the mount makes the others visible to it.
#
# Caveat: every checkout is mounted at /src and shares the padwight-kernel-target volume, and Cargo
# rebuilds by file times. After running from another checkout, touch this one's sources first, or
# the run tests the other checkout's build (the test count gives it away). A padwight daemon on
# the host can also make the virtual-keyboard tests watch its keyboard; see docs/development/qa-plan.md.
#
# Usage: scripts/kernel-test-docker.sh [test filter, default: nothing_stays] [more cargo test args]
#   scripts/kernel-test-docker.sh daemon_   runs the daemon-exit scenarios
#   scripts/kernel-test-docker.sh hw_ --test hardware   runs the hardware-check replicas (HW-1, 3, 8, 10, 11)
#   scripts/kernel-test-docker.sh touchpad_ --test touchpad_modes   runs the touchpad modes (HW-5)
# PADWIGHT_KERNEL_VOLUME=name uses its own build volume (default padwight-kernel-target), so a run
# from another checkout doesn't make this one test stale code. The run takes an exclusive lock
# (PADWIGHT_DOCKER_LOCK, default /tmp/padwight-kernel-docker.lock): two runs at once would fight
# over /dev/uhid, /dev/input and each other's pads.
set -eu

filter=${1:-nothing_stays}
[ $# -gt 0 ] && shift
volume=${PADWIGHT_KERNEL_VOLUME:-padwight-kernel-target}
lock=${PADWIGHT_DOCKER_LOCK:-/tmp/padwight-kernel-docker.lock}
root=$(cd "$(dirname "$0")/.." && pwd)

for dev in /dev/uinput /dev/uhid; do
    if [ ! -e "$dev" ]; then
        echo "$dev is missing: load its module first (sudo modprobe ${dev#/dev/})" >&2
        exit 1
    fi
done

# Build output and the cargo registry live in named volumes, so the container doesn't write
# root-owned files into the checkout.
run() { docker run --rm \
    --device /dev/uinput \
    --device /dev/uhid \
    --device-cgroup-rule 'c 13:* rwm' \
    -v /dev/input:/dev/input \
    -v "$root":/src \
    -v "$volume":/target \
    -v padwight-kernel-cargo:/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/target \
    -e RUST_BACKTRACE=0 \
    -w /src \
    -u 0 \
    rust:1 \
    sh -c "apt-get update -qq && apt-get install -y -qq pkg-config libxkbcommon-dev libwayland-dev libudev-dev libfontconfig1-dev libdbus-1-dev libasound2-dev xvfb xauth libxcursor1 libxrandr2 libxi6 libxinerama1 libxkbcommon-x11-0 x11-utils >/dev/null \
        && xvfb-run -a cargo test $* -- --ignored $filter"
}

# Takes the lock when flock is there (it is on Linux).
if command -v flock >/dev/null 2>&1; then
    exec 9>"$lock"
    flock 9
fi
run "$@"
