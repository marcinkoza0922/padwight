#!/bin/sh
# Validates the files a user installs: the systemd units, the udev rule, the desktop entry, the
# AppStream metainfo. A missing validator is an error, not a skip, so a
# run can't pass without checking anything.
set -eu
cd "$(dirname "$0")/.."

need() {
    command -v "$1" >/dev/null 2>&1 || { echo "missing tool: $1 (install it, or run this in CI)" >&2; exit 1; }
}
need systemd-analyze
need udevadm
need desktop-file-validate
need appstreamcli
need python3

# `systemd-analyze verify` also checks that the ExecStart program exists, and that depends on the
# machine (the daemon is in ~/.cargo/bin here, and not on a CI runner). So each unit is checked as a
# copy whose ExecStart is /bin/true; the directives around it are what's being checked.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for unit in dist/padwight.service; do
    copy="$tmp/$(basename "$unit")"
    sed -E 's|^ExecStart=[^ ]+|ExecStart=/bin/true|' "$unit" > "$copy"
    systemd-analyze verify --man=no "$copy"
    echo "ok: $unit"
done

for rules in dist/70-padwight-motion.rules dist/70-padwight-uinput.rules dist/72-padwight-hidraw.rules; do
    udevadm verify "$rules"
    echo "ok: $rules"
done

sh -n dist/install.sh
sh -n dist/get.sh
sh -n scripts/package.sh
echo "ok: install scripts"

desktop-file-validate dist/io.github.marcinkoza0922.Padwight.desktop
echo "ok: desktop entry"

# --no-net: no network access in CI. Pedantic hints are not failures.
appstreamcli validate --no-net dist/io.github.marcinkoza0922.Padwight.metainfo.xml
echo "ok: metainfo"
