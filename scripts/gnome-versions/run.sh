#!/usr/bin/env bash
# Checks the GNOME overlay extension against the GNOME Shell of each Fedora release given
# (39 = Shell 45, 40 = 46, 41 = 47, 42 = 48, 43 = 49, 44 = 50), headless, in Docker. The
# extension's frames are made by probe.py, not by the overlay process, so this covers the
# extension and the Shell API it uses, not padwight's Rust side.
# Usage: scripts/gnome-versions/run.sh 44 43 ...        (default: 39 to 44)
set -uo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
releases=("$@")
[ ${#releases[@]} -gt 0 ] || releases=(39 40 41 42 43 44)
failed=0
for release in "${releases[@]}"; do
    echo "===== Fedora $release"
    ctx=$(mktemp -d)
    cp "$root"/scripts/gnome-versions/{Dockerfile,probe.py,localed-stub.py,run-in-container.sh} "$ctx"/
    docker build -q --build-arg FEDORA="$release" -t "padwight-gnome:$release" "$ctx" >/dev/null || { echo "BUILD FAILED"; failed=1; rm -rf "$ctx"; continue; }
    rm -rf "$ctx"
    docker run --rm -v "$root/gnome-extension/padwight-focus@io.github.marcinkoza0922:/extension:ro" "padwight-gnome:$release" || failed=1
done
exit $failed
