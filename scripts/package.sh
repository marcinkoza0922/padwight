#!/bin/sh
# Builds the Linux release tarball into release/ (untracked): the binary, an installer, and the
# files the installer copies. Run on the oldest distro you want to support, since the binary needs
# at least the glibc it was built against; the highest version it asks for is printed at the end.
set -eu
cd "$(dirname "$0")/.."

#
# For testing the update and install paths (scripts/update-test.sh), three variables stand in for a
# real release: PADWIGHT_PACKAGE_BINARY packages that binary instead of building one,
# PADWIGHT_PACKAGE_VERSION names the tarball for another version, and PADWIGHT_PACKAGE_OUT is the
# output folder.
version=${PADWIGHT_PACKAGE_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}
out=${PADWIGHT_PACKAGE_OUT:-release}
arch=$(uname -m)
name=padwight-$version-linux-$arch
stage=$out/$name

if [ -z "${PADWIGHT_PACKAGE_BINARY:-}" ]; then cargo build --release --locked; fi
rm -rf "$stage"
mkdir -p "$stage/share"
install -m755 "${PADWIGHT_PACKAGE_BINARY:-target/release/padwight}" "$stage/padwight"
install -m755 dist/install.sh "$stage/install.sh"
install -m644 dist/padwight.service dist/70-padwight-motion.rules dist/70-padwight-uinput.rules dist/72-padwight-hidraw.rules dist/padwight-uinput.conf "$stage/share/"
install -m644 dist/io.github.marcinkoza0922.Padwight.desktop dist/io.github.marcinkoza0922.Padwight.svg "$stage/share/"
install -m644 LICENSE "$stage/LICENSE"
cat > "$stage/README.txt" <<TEXT
padwight $version

Run ./install.sh. It installs padwight for your user and, with your OK, three small system files
(udev rules and a module list) that let it create a virtual gamepad and read your controller.
Afterwards, \`padwight\` opens the settings window, and \`padwight doctor\` checks that everything
it needs is in place. ./install.sh --uninstall removes it again.
TEXT

tar -C "$out" -czf "$out/$name.tar.gz" "$name"
(cd "$out" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
echo "built $out/$name.tar.gz"
if command -v objdump >/dev/null 2>&1; then
    glibc=$(objdump -T "$stage/padwight" | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1)
    echo "needs ${glibc:-an unknown glibc} or newer"
fi
