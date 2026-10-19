#!/bin/sh
# Builds the Linux release tarball into release/ (untracked): the binary, an installer, and the
# files the installer copies. Run on the oldest distro you want to support, since the binary needs
# at least the glibc it was built against; the highest version it asks for is printed at the end.
set -eu
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
arch=$(uname -m)
name=padwight-$version-linux-$arch
stage=release/$name

cargo build --release --locked
rm -rf "$stage"
mkdir -p "$stage/share"
install -m755 target/release/padwight "$stage/padwight"
install -m755 dist/install.sh "$stage/install.sh"
install -m644 dist/padwight.service dist/70-padwight-motion.rules dist/70-padwight-uinput.rules dist/72-padwight-hidraw.rules dist/padwight-uinput.conf "$stage/share/"
install -m644 flatpak/io.github.marcinkoza0922.Padwight.desktop flatpak/io.github.marcinkoza0922.Padwight.svg "$stage/share/"
install -m644 LICENSE "$stage/LICENSE"
cat > "$stage/README.txt" <<TEXT
padwight $version

Run ./install.sh. It installs padwight for your user and, with your OK, three small system files
(udev rules and a module list) that let it create a virtual gamepad and read your controller.
Afterwards, \`padwight\` opens the settings window, and \`padwight doctor\` checks that everything
it needs is in place. ./install.sh --uninstall removes it again.
TEXT

tar -C release -czf "release/$name.tar.gz" "$name"
(cd release && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
echo "built release/$name.tar.gz"
if command -v objdump >/dev/null 2>&1; then
    glibc=$(objdump -T "$stage/padwight" | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1)
    echo "needs ${glibc:-an unknown glibc} or newer"
fi
