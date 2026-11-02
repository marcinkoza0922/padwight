#!/usr/bin/env python3
"""Drives the daemon's tray icon through D-Bus, the way a panel does (qa-plan INS-10).

    dbus-run-session -- python3 -I scripts/tray-test.py [path/to/padwight]

It starts a private session bus (dbus-run-session), stands in for the desktop's
StatusNotifierWatcher and host, starts `padwight daemon --debug --fake-clock` in a throwaway
home, reads the tray's menu over com.canonical.dbusmenu, and clicks each entry: Remapping (twice),
Open settings, Close overlays and Quit. Needs python3-dbus and PyGObject. The host's real session
bus, home and systemd are not touched.

Not covered: the real virtual devices (a --fake-clock daemon has none, so "Quit releases every key"
is shown only by the log line that says the release ran), and how a particular panel draws the menu.
"""

import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

BINARY = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "target/debug/padwight")
passed = failed = 0


def check(what, ok, detail=""):
    global passed, failed
    if ok:
        passed += 1
        print(f"ok: {what}")
    else:
        failed += 1
        print(f"FAIL: {what}" + (f"\n    | {detail}" if detail else ""))


def wait_for(fn, seconds=10.0, step=0.05):
    end = time.time() + seconds
    while time.time() < end:
        value = fn()
        if value:
            return value
        time.sleep(step)
    return None


class Watcher(dbus.service.Object):
    """The desktop's tray host: remembers who registered."""

    IFACE = "org.kde.StatusNotifierWatcher"

    def __init__(self, bus):
        self.items = []
        self.registered = threading.Event()
        super().__init__(dbus.service.BusName(self.IFACE, bus), "/StatusNotifierWatcher")

    @dbus.service.method(IFACE, in_signature="s")
    def RegisterStatusNotifierItem(self, service):
        self.items.append(str(service))
        self.registered.set()

    @dbus.service.method(IFACE, in_signature="s")
    def RegisterStatusNotifierHost(self, service):
        pass

    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature="ss", out_signature="v")
    def Get(self, iface, name):
        return self.GetAll(iface)[name]

    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature="s", out_signature="a{sv}")
    def GetAll(self, iface):
        return {
            "IsStatusNotifierHostRegistered": dbus.Boolean(True),
            "ProtocolVersion": dbus.Int32(0),
            "RegisteredStatusNotifierItems": dbus.Array(self.items, signature="s"),
        }


def ipc(socket_path, request):
    """One request to the daemon's control socket; its reply as JSON."""
    s = socket.socket(socket.AF_UNIX)
    s.settimeout(5)
    s.connect(socket_path)
    s.sendall(json.dumps(request).encode() + b"\n")
    reply = s.makefile().readline()
    return json.loads(reply)


def menu_entries(menu):
    """(id, label, toggle-state or None) for each entry of the dbusmenu, separators as label None."""
    _, layout = menu.GetLayout(0, -1, dbus.Array([], signature="s"), dbus_interface="com.canonical.dbusmenu")
    entries = []
    for child in layout[2]:
        cid, props = int(child[0]), child[1]
        if props.get("type") == "separator":
            entries.append((cid, None, None))
        else:
            entries.append((cid, str(props.get("label", "")), int(props["toggle-state"]) if "toggle-state" in props else None))
    return entries


def main():
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    watcher = Watcher(bus)
    threading.Thread(target=GLib.MainLoop().run, daemon=True).start()

    root = tempfile.mkdtemp(prefix="pwtray.", dir="/tmp")  # short: socket paths are limited to 107 bytes
    for d in ("home", "config", "state", "data", "cache", "tmp"):
        os.makedirs(f"{root}/{d}")
    os.makedirs(f"{root}/run", mode=0o700)
    env = {
        "PATH": "/usr/bin:/bin", "HOME": f"{root}/home", "XDG_CONFIG_HOME": f"{root}/config", "XDG_STATE_HOME": f"{root}/state",
        "XDG_DATA_HOME": f"{root}/data", "XDG_CACHE_HOME": f"{root}/cache", "XDG_RUNTIME_DIR": f"{root}/run", "TMPDIR": f"{root}/tmp",
        "DBUS_SESSION_BUS_ADDRESS": os.environ["DBUS_SESSION_BUS_ADDRESS"], "LANG": "C",
    }
    log_path = f"{root}/daemon.log"
    log = open(log_path, "w")
    daemon = subprocess.Popen([BINARY, "daemon", "--debug", "--fake-clock"], env=env, stdout=log, stderr=log, stdin=subprocess.DEVNULL)
    sock = f"{root}/run/padwight.sock"
    try:
        run(watcher, daemon, sock, log_path)
    finally:
        if daemon.poll() is None:
            daemon.kill()
        daemon.wait()
        shutil.rmtree(root, ignore_errors=True)
    print(f"\n{passed} passed, {failed} failed")
    return 1 if failed else 0


def run(watcher, daemon, sock, log_path):
    def log_text():
        return open(log_path).read()

    if not wait_for(lambda: os.path.exists(sock), 15):
        check("the daemon starts", False, log_text())
        return
    if not watcher.registered.wait(15):
        check("the tray icon registers with the watcher", False, "no registration; the daemon log says:\n" + log_text())
        return
    check("the tray icon registers with the watcher", True)
    service = watcher.items[0]
    bus = dbus.SessionBus()
    item = bus.get_object(service, "/StatusNotifierItem")
    menu = bus.get_object(service, "/MenuBar")
    props = dbus.Interface(item, dbus.PROPERTIES_IFACE)

    check("the item's title is Padwight", str(props.Get("org.kde.StatusNotifierItem", "Title")) == "Padwight")
    entries = menu_entries(menu)
    labels = [e[1] for e in entries]
    check("the menu is Remapping, Open settings, Close overlays, -, Quit", labels == ["Remapping", "Open settings", "Close overlays", None, "Quit"], str(labels))
    ids = {label: cid for cid, label, _ in entries if label}

    def click(label):
        menu.Event(ids[label], "clicked", dbus.Int32(0, variant_level=1), dbus.UInt32(int(time.time())), dbus_interface="com.canonical.dbusmenu")

    def enabled():
        return ipc(sock, "Status")["Status"]["enabled"]

    def tooltip():
        return str(props.Get("org.kde.StatusNotifierItem", "ToolTip")[3])

    def toggle_state():
        return [e[2] for e in menu_entries(menu) if e[1] == "Remapping"][0]

    # Remapping: a checkmark that flips the daemon's switch.
    check("Remapping starts on, checked, and the tooltip says so", enabled() is True and toggle_state() == 1 and "Remapping on" in tooltip())
    click("Remapping")
    check("Remapping click: the daemon switches off", wait_for(lambda: enabled() is False, 5) is not None)
    check("Remapping click: the checkmark and tooltip follow", wait_for(lambda: toggle_state() == 0 and "Remapping off" in tooltip(), 5) is not None)
    click("Remapping")
    check("Remapping clicked again: back on", wait_for(lambda: enabled() is True and toggle_state() == 1, 5) is not None)

    # Close overlays: open the numpad overlay first, then close it from the tray.
    ipc(sock, "ToggleNumpad")
    status = lambda: ipc(sock, "Status")["Status"]
    check("(setup) the numpad overlay is up", wait_for(lambda: status()["numpad_visible"], 5) is not None)
    click("Close overlays")
    check("Close overlays click: the numpad closes", wait_for(lambda: not status()["numpad_visible"], 5) is not None)
    check("Close overlays click: the log says it came from the tray", "closing every overlay: from the tray" in log_text())

    # Open settings: the daemon starts the settings window, a child of its own.
    def children():
        out = subprocess.run(["pgrep", "-P", str(daemon.pid)], capture_output=True, text=True).stdout.split()
        return out

    before = set(children())
    seen = threading.Event()

    def watch():
        end = time.time() + 6
        while time.time() < end and not seen.is_set():
            if set(children()) - before:
                seen.set()
            time.sleep(0.01)

    watcher_thread = threading.Thread(target=watch)
    watcher_thread.start()
    click("Open settings")
    watcher_thread.join()
    check("Open settings click: the daemon starts the settings window as a child process", seen.is_set())
    # Left click on the icon does the same (Activate).
    before = set(children())
    seen.clear()
    watcher_thread = threading.Thread(target=watch)
    watcher_thread.start()
    item.Activate(0, 0, dbus_interface="org.kde.StatusNotifierItem")
    watcher_thread.join()
    check("Activate (a left click on the icon): the same, or the window is already up", seen.is_set() or bool(before))

    # Quit: releases and exits, and the icon goes.
    try:
        click("Quit")
    except dbus.exceptions.DBusException as e:
        # The daemon exits before it answers: the menu's reply never comes.
        check("Quit click: the daemon went away without replying", "NoReply" in str(e) or "disconnected" in str(e), str(e))
    code = wait_for(lambda: daemon.poll() is not None and 1, 10)
    check("Quit click: the daemon exits", code is not None)
    check("Quit click: it exits with status 0", daemon.returncode == 0, f"status {daemon.returncode}")
    text = log_text()
    check("Quit click: the log says it came from the tray", "quit from the tray" in text)
    check("Quit click: the log says what was held was released before exiting", "released what was held; exiting" in text)
    # The settings windows it started have no display here and panic in winit (their output shares the log); that is not the daemon.
    panics = [l for l in text.splitlines() if "panicked" in l and "iced_winit" not in l]
    check("Quit click: no 'output error' and no panic of the daemon's in the log", "output error" not in text and not panics, "\n".join(panics))
    names = [str(n) for n in dbus.SessionBus().list_names()]
    check("Quit click: the tray icon's bus name is gone", service not in names)


if __name__ == "__main__":
    sys.exit(main())
