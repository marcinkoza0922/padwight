"""Times the overlay extension's Present (it blocks the Shell's main loop) on a sealed memfd.
Run it like probe.py: docker run -v <extension>:/extension:ro -v scripts/gnome-versions/bench.py:/home/dev/probe.py:ro padwight-gnome:<release>"""
import fcntl, os, sys, time
from gi.repository import Gio, GLib
NAME, PATH = "io.github.marcinkoza0922.Padwight.Shell", "/Overlay"
IFACE = "io.github.marcinkoza0922.Padwight.Overlay"
UUID = "padwight-focus@io.github.marcinkoza0922"
bus = Gio.bus_get_sync(Gio.BusType.SESSION)
def call(n, p, i, m, a=None, fds=None):
    if fds:
        return bus.call_with_unix_fd_list_sync(n, p, i, m, a, None, 0, 10000, Gio.UnixFDList.new_from_array(fds), None)[0]
    return bus.call_sync(n, p, i, m, a, None, 0, 10000, None)
for _ in range(90):
    try:
        if call("org.gnome.Shell", "/org/gnome/Shell", "org.gnome.Shell.Extensions", "GetExtensionInfo", GLib.Variant("(s)", (UUID,))).unpack()[0].get("state") == 1.0: break
    except GLib.Error: pass
    time.sleep(1)
pid = int(os.popen("pgrep -x gnome-shell").read().split()[0])
def cpu():
    f = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
    return (int(f[11]) + int(f[12])) / os.sysconf("SC_CLK_TCK")
call(NAME, PATH, IFACE, "Geometry")
for (w, h) in ((1280, 800), (1920, 1080)):
    fd = os.memfd_create("b", 2); os.write(fd, bytes([0, 200, 0, 255]) * (w * h)); fcntl.fcntl(fd, fcntl.F_ADD_SEALS, fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL)
    call(NAME, PATH, IFACE, "SetBuffer", GLib.Variant("(hii)", (0, w, h)), fds=[os.dup(fd)])
    for _ in range(30): call(NAME, PATH, IFACE, "Present")
    for run in range(3):
        n = 400; c0 = cpu(); ts = []
        t0 = time.perf_counter()
        for _ in range(n):
            a = time.perf_counter(); call(NAME, PATH, IFACE, "Present"); ts.append((time.perf_counter() - a) * 1000)
        wall = time.perf_counter() - t0; used = cpu() - c0
        ts.sort()
        print(f"RESULT {w}x{h} run{run}: Present mean {sum(ts)/n:.2f} p50 {ts[n//2]:.2f} p95 {ts[int(n*.95)]:.2f} ms; shell CPU {used:.2f}s over {wall:.2f}s ({100*used/wall:.0f}%)", flush=True)
    call(NAME, PATH, IFACE, "Hide")
