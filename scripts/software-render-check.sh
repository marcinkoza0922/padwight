#!/bin/sh
# ENV-7 without a GPU: the overlay scenes and the settings window's layout check run with no display
# server, no GPU access and every Mesa/Vulkan/GL route to hardware turned off, and must still draw.
# This shows that nothing in drawing needs a graphics card; it does not test NVIDIA's driver, a
# hybrid laptop, 144 Hz, VRR, HDR or a rotated panel (those stay manual, see qa-plan.md ENV-7).
#
# Usage: scripts/software-render-check.sh [padwight binary, default: cargo run]
# Pictures go to screenshots/software-render/ (untracked). Exit status 1 on any failure.
#
# For the real settings window under a virtual X server with the same GPU-less environment, run
# `scripts/kernel-test-docker.sh gui_software` (Docker, uses Xvfb).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
out=screenshots/software-render
rm -rf "$out"
mkdir -p "$out"

if [ -n "${1:-}" ]; then
    bin=$1
    padwight() { "$bin" "$@"; }
else
    padwight() { cargo run -q -- "$@"; }
fi

# No display, software GL, no Vulkan ICD, and wgpu told to use GL only.
nogpu() {
    env -u DISPLAY -u WAYLAND_DISPLAY \
        LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe VK_ICD_FILENAMES=/nonexistent \
        WGPU_BACKEND=gl ICED_BACKEND=tiny-skia "$@"
}

echo "overlay scenes with no GPU and no display"
nogpu padwight debug overlay all glyphs --out "$out" >"$out/overlay.log" 2>&1 || { cat "$out/overlay.log"; echo "FAIL: overlay scenes did not draw" >&2; exit 1; }

pictures=$(find "$out" -name '*.png' | wc -l)
[ "$pictures" -ge 10 ] || { echo "FAIL: only $pictures pictures drawn" >&2; exit 1; }

# A picture of one flat colour means nothing was drawn: every scene must have some variation.
python3 -I - "$out" <<'PY'
import struct, sys, zlib
from pathlib import Path

flat = []
for path in sorted(Path(sys.argv[1]).glob("*.png")):
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        flat.append(path.name + " (not a PNG)")
        continue
    # Concatenate the IDAT chunks and look at the raw (filtered) scanlines: a one-colour picture
    # compresses to almost nothing compared with its size.
    pos, idat, width, height = 8, b"", 0, 0
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height = struct.unpack(">II", body[:8])
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    if len(idat) < max(2000, width * height // 2000):
        flat.append(f"{path.name} ({len(idat)} bytes of image data for {width}x{height})")
if flat:
    print("FAIL: pictures that look empty:", *flat, sep="\n  ")
    sys.exit(1)
PY

echo "settings window layout with no GPU and no display"
nogpu padwight debug gui-check >"$out/gui-check.log" 2>&1 || {
    # gui-check exits non-zero on findings with --strict only; any failure here is a crash.
    cat "$out/gui-check.log"
    echo "FAIL: the settings window checks did not run" >&2
    exit 1
}

echo "software rendering: $pictures overlay pictures drawn, settings window checked"
