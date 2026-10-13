#!/usr/bin/env python3
"""Experiment: take a Bluetooth Switch Pro-style pad away from BlueZ, without root, and feed a uinput pad.

Usage: scripts/bt-takeover.py MAC SECONDS [full|profile]
See docs/development/steam-input.md.

Asks BlueZ to let go of the pad (its uhid device and hidraw node go away, so Steam drops it), then
opens the pad's HID control (PSM 17) and interrupt (PSM 19) channels itself and writes the reports
to a uinput Xbox 360 pad. `full` (the default) disconnects the pad, which puts a Switch-style pad
to sleep. `profile` only closes BlueZ's HID channels (Device1.DisconnectProfile) and keeps the
Bluetooth link up. On exit it closes the channels and asks BlueZ to connect the pad again;
if that fails, press the pad's Home button.
"""
import socket, struct, subprocess, sys, time, signal

from evdev import UInput, AbsInfo, ecodes as e

SOL_BLUETOOTH, BT_SECURITY, BT_SECURITY_MEDIUM = 274, 4, 2
mac, seconds = sys.argv[1], int(sys.argv[2])
mode = sys.argv[3] if len(sys.argv) > 3 else "full"
HID_UUID = "00001124-0000-1000-8000-00805f9b34fb"

stop = False
def on_signal(*_):
    global stop
    stop = True
signal.signal(signal.SIGTERM, on_signal)
signal.signal(signal.SIGINT, on_signal)

def bluetoothctl(*args):
    out = subprocess.run(["bluetoothctl", *args], capture_output=True, text=True, timeout=15)
    return (out.stdout + out.stderr).strip().splitlines()[-1:] or [""]

def channel(psm, tries=40):
    for attempt in range(tries):
        s = socket.socket(socket.AF_BLUETOOTH, socket.SOCK_SEQPACKET, socket.BTPROTO_L2CAP)
        s.setsockopt(SOL_BLUETOOTH, BT_SECURITY, struct.pack("BB", BT_SECURITY_MEDIUM, 0))
        try:
            s.connect((mac, psm))
            print(f"PSM {psm} connected after {attempt + 1} tries", flush=True)
            return s
        except OSError as err:
            s.close()
            if attempt == tries - 1:
                raise
            if attempt % 10 == 0:
                print(f"PSM {psm}: {err}, retrying", flush=True)
            time.sleep(0.25)

stick = AbsInfo(0, -32768, 32767, 16, 128, 0)
trigger = AbsInfo(0, 0, 255, 0, 0, 0)
hat = AbsInfo(0, -1, 1, 0, 0, 0)
caps = {
    e.EV_KEY: [e.BTN_A, e.BTN_B, e.BTN_X, e.BTN_Y, e.BTN_TL, e.BTN_TR, e.BTN_SELECT, e.BTN_START,
               e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR],
    e.EV_ABS: [(e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick),
               (e.ABS_Z, trigger), (e.ABS_RZ, trigger), (e.ABS_HAT0X, hat), (e.ABS_HAT0Y, hat)],
}
# 0x3F (simple HID) report: byte 1 = B A Y X L R ZL ZR, byte 2 = - + LS RS Home Capture.
# Nintendo B is the south button, A the east one.
SIMPLE = {(1, 0): e.BTN_A, (1, 1): e.BTN_B, (1, 2): e.BTN_X, (1, 3): e.BTN_Y, (1, 4): e.BTN_TL,
          (1, 5): e.BTN_TR, (2, 0): e.BTN_SELECT, (2, 1): e.BTN_START, (2, 2): e.BTN_THUMBL,
          (2, 3): e.BTN_THUMBR, (2, 4): e.BTN_MODE}
HAT = {0: (0, -1), 1: (1, -1), 2: (1, 0), 3: (1, 1), 4: (0, 1), 5: (-1, 1), 6: (-1, 0), 7: (-1, -1)}

if mode == "profile":
    path = "/org/bluez/hci0/dev_" + mac.replace(":", "_")
    out = subprocess.run(["gdbus", "call", "--system", "--dest", "org.bluez", "--object-path", path,
                          "--method", "org.bluez.Device1.DisconnectProfile", HID_UUID],
                         capture_output=True, text=True, timeout=15)
    print("disconnect HID profile:", (out.stdout + out.stderr).strip() or "ok", flush=True)
else:
    print("disconnect:", bluetoothctl("disconnect", mac)[0], flush=True)
time.sleep(1)
ctrl = intr = ui = None
try:
    ctrl = channel(17)
    intr = channel(19)
    intr.settimeout(0.5)
    ui = UInput(caps, name="BT Takeover Test Pad", vendor=0x045e, product=0x028e, version=0x110, bustype=e.BUS_USB)
    print(f"uinput pad up: {ui.device.path}", flush=True)
    start = time.monotonic()
    end, reports, shown, tick = start + seconds, 0, 0, start + 5
    while not stop and time.monotonic() < end:
        if time.monotonic() >= tick:
            tick += 5
            print(f"{tick - start - 5:3.0f}s: {reports} reports", flush=True)
        try:
            d = intr.recv(64)
        except socket.timeout:
            continue
        if shown < 4:
            shown += 1
            print(f"packet ({len(d)} bytes): {d[:16].hex()}", flush=True)
        if len(d) < 12 or d[0] != 0xA1 or d[1] != 0x3F:
            continue
        r = d[1:]
        reports += 1
        if reports == 1:
            print("first 0x3F report", flush=True)
        for (byte, bit), code in SIMPLE.items():
            ui.write(e.EV_KEY, code, (r[byte] >> bit) & 1)
        ui.write(e.EV_ABS, e.ABS_Z, 255 if r[1] & 0x40 else 0)
        ui.write(e.EV_ABS, e.ABS_RZ, 255 if r[1] & 0x80 else 0)
        hx, hy = HAT.get(r[3], (0, 0))
        ui.write(e.EV_ABS, e.ABS_HAT0X, hx)
        ui.write(e.EV_ABS, e.ABS_HAT0Y, hy)
        lx, ly, rx, ry = struct.unpack_from("<HHHH", r, 4)
        for code, v in ((e.ABS_X, lx), (e.ABS_Y, ly), (e.ABS_RX, rx), (e.ABS_RY, ry)):
            ui.write(e.EV_ABS, code, max(-32768, min(32767, v - 0x8000)))
        ui.syn()
    print(f"stopping after {reports} reports", flush=True)
finally:
    if ui:
        ui.close()
    for s in (intr, ctrl):
        if s:
            s.close()
    time.sleep(1)
    print("reconnect:", bluetoothctl("connect", mac)[0], flush=True)
