"""Drives the overlay extension like the overlay process does, and looks at the screen.

Waits for the extension, asks for the monitor's geometry, shares a picture (an opaque green
square on clear) with SetBuffer, calls Present and screenshots the shell: the square must be on
screen where it was drawn, and the rest must not be. Then Hide, and the square must be gone."""
import os, struct, sys, time
from gi.repository import Gio, GLib, GdkPixbuf

NAME, PATH = "io.github.marcinkoza0922.Padwight.Shell", "/Overlay"
IFACE = "io.github.marcinkoza0922.Padwight.Overlay"
UUID = "padwight-focus@io.github.marcinkoza0922"
bus = Gio.bus_get_sync(Gio.BusType.SESSION)

def call(name, path, iface, method, args=None, fds=None, reply="*"):
    if fds:
        out, _ = bus.call_with_unix_fd_list_sync(name, path, iface, method, args, None, 0, 10000, Gio.UnixFDList.new_from_array(fds), None)
        return out
    return bus.call_sync(name, path, iface, method, args, None, 0, 10000, None)

def fail(why):
    print("FAIL", why)
    sys.exit(1)

# The extension loads once the shell has started.
deadline = time.time() + 90
while time.time() < deadline:
    try:
        info = call("org.gnome.Shell", "/org/gnome/Shell", "org.gnome.Shell.Extensions", "GetExtensionInfo", GLib.Variant("(s)", (UUID,))).unpack()[0]
        if info.get("state") == 1.0:
            break
        if info.get("error"):
            fail(f"the extension failed to load: {info['error']}")
    except GLib.Error:
        pass
    time.sleep(1)
else:
    fail("the extension never loaded")
print("extension enabled")

w, h, scale = call(NAME, PATH, IFACE, "Geometry").unpack()
print(f"geometry {w}x{h} scale {scale}")
device = (round(w * scale), round(h * scale))
square = (100, 120, 300, 320)  # x0 y0 x1 y1 in device pixels
fd = os.memfd_create("probe")
rows = bytearray()
for y in range(device[1]):
    for x in range(device[0]):
        inside = square[0] <= x < square[2] and square[1] <= y < square[3]
        rows += bytes((0, 255, 0, 255)) if inside else bytes(4)
os.write(fd, bytes(rows))
call(NAME, PATH, IFACE, "SetBuffer", GLib.Variant("(hii)", (0, *device)), fds=[fd])
call(NAME, PATH, IFACE, "Present")

def shot(name):
    path = f"/tmp/{name}.png"
    call("org.gnome.Shell.Screenshot", "/org/gnome/Shell/Screenshot", "org.gnome.Shell.Screenshot", "Screenshot", GLib.Variant("(bbs)", (False, False, path)))
    pixbuf = GdkPixbuf.Pixbuf.new_from_file(path)
    px, stride, n = pixbuf.get_pixels(), pixbuf.get_rowstride(), pixbuf.get_n_channels()
    scale_x = pixbuf.get_width() / w
    def at(x, y):
        i = int(y * scale_x) * stride + int(x * scale_x) * n
        return tuple(px[i:i + 3])
    return at

time.sleep(1)
at = shot("shown")
inside, outside = at((square[0] + square[2]) / 2 / scale, (square[1] + square[3]) / 2 / scale), at(w - 40, h - 40)
print("inside", inside, "outside", outside)
if not (inside[1] > 200 and inside[0] < 60 and inside[2] < 60):
    fail(f"the picture isn't on screen: the square reads {inside}")
if outside[1] > 200 and outside[0] < 60:
    fail("the clear part of the picture is drawn green")
call(NAME, PATH, IFACE, "Hide")
time.sleep(1)
after = shot("hidden")((square[0] + square[2]) / 2 / scale, (square[1] + square[3]) / 2 / scale)
print("after Hide", after)
if after == inside:
    fail("Hide left the picture on screen")
print("PASS")
