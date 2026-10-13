#!/usr/bin/env python3
"""Experiment: take a wired Xbox One (GIP) pad away from its kernel driver, without root, and feed a uinput pad.

Usage: scripts/usb-takeover.py /dev/bus/usb/BBB/DDD INTERFACE IN_ENDPOINT SECONDS
See docs/development/steam-input.md.

Detaches the kernel driver (xpad, xone) from the pad's interface through usbfs, sends xpad's init
request, reads the pad's GIP reports and writes them to a uinput Xbox 360 pad. On exit (SIGTERM,
Ctrl+C, timeout) it gives the interface back to the kernel driver. After a SIGKILL it can't: run
scripts/usb-reattach.py.
"""
import ctypes, errno, fcntl, os, signal, struct, sys, time

from evdev import UInput, AbsInfo, ecodes as e

def _ioc(d, nr, size):
    return (d << 30) | (size << 16) | (ord('U') << 8) | nr

class DisconnectClaim(ctypes.Structure):
    _fields_ = [("interface", ctypes.c_uint), ("flags", ctypes.c_uint), ("driver", ctypes.c_char * 256)]

class Bulk(ctypes.Structure):
    _fields_ = [("ep", ctypes.c_uint), ("len", ctypes.c_uint), ("timeout", ctypes.c_uint), ("data", ctypes.c_void_p)]

class UsbIoctl(ctypes.Structure):
    _fields_ = [("ifno", ctypes.c_int), ("ioctl_code", ctypes.c_int), ("data", ctypes.c_void_p)]

class CtrlTransfer(ctypes.Structure):
    _fields_ = [("bRequestType", ctypes.c_uint8), ("bRequest", ctypes.c_uint8), ("wValue", ctypes.c_uint16),
                ("wIndex", ctypes.c_uint16), ("wLength", ctypes.c_uint16), ("timeout", ctypes.c_uint32),
                ("data", ctypes.c_void_p)]

CONTROL = _ioc(3, 0, ctypes.sizeof(CtrlTransfer))
DISCONNECT_CLAIM =_ioc(2, 27, ctypes.sizeof(DisconnectClaim))
RELEASEINTERFACE = _ioc(2, 16, 4)
BULK = _ioc(3, 2, ctypes.sizeof(Bulk))
IOCTL = _ioc(3, 18, ctypes.sizeof(UsbIoctl))
CONNECT = _ioc(0, 23, 0)

node, iface, ep, seconds = sys.argv[1], int(sys.argv[2]), int(sys.argv[3], 0), int(sys.argv[4])

stick = AbsInfo(0, -32768, 32767, 16, 128, 0)
trigger = AbsInfo(0, 0, 255, 0, 0, 0)
hat = AbsInfo(0, -1, 1, 0, 0, 0)
caps = {
    e.EV_KEY: [e.BTN_A, e.BTN_B, e.BTN_X, e.BTN_Y, e.BTN_TL, e.BTN_TR, e.BTN_SELECT, e.BTN_START,
               e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR],
    e.EV_ABS: [(e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick),
               (e.ABS_Z, trigger), (e.ABS_RZ, trigger), (e.ABS_HAT0X, hat), (e.ABS_HAT0Y, hat)],
}

GIP_BUTTONS = {2: e.BTN_START, 3: e.BTN_SELECT, 4: e.BTN_A, 5: e.BTN_B, 6: e.BTN_X, 7: e.BTN_Y,
               12: e.BTN_TL, 13: e.BTN_TR, 14: e.BTN_THUMBL, 15: e.BTN_THUMBR}

stop = False
def on_signal(*_):
    global stop
    stop = True
signal.signal(signal.SIGTERM, on_signal)
signal.signal(signal.SIGINT, on_signal)

fd = os.open(node, os.O_RDWR)
claim = DisconnectClaim(iface, 0, b"")
fcntl.ioctl(fd, DISCONNECT_CLAIM, claim)
print(f"detached the kernel driver from interface {iface} and claimed it", flush=True)
ui = None
try:
    # What xpad sends when it opens a wired 360 pad: third-party pads stay silent without it.
    dummy = ctypes.create_string_buffer(20)
    ctrl = CtrlTransfer(0xC1, 0x01, 0x0100, 0, 20, 2000, ctypes.cast(dummy, ctypes.c_void_p))
    try:
        print(f"init request answered with {fcntl.ioctl(fd, CONTROL, ctrl)} bytes", flush=True)
    except OSError as err:
        print(f"init request failed: {err}", flush=True)
    led = ctypes.create_string_buffer(bytes([0x01, 0x03, 0x02]), 3)
    try:
        fcntl.ioctl(fd, BULK, Bulk(ep & 0x7F, 3, 500, ctypes.cast(led, ctypes.c_void_p)))
        print("LED command sent", flush=True)
    except OSError as err:
        print(f"LED command failed: {err}", flush=True)
    ui = UInput(caps, name="USB Takeover Test Pad", vendor=0x045e, product=0x028e, version=0x110, bustype=e.BUS_USB)
    print(f"uinput pad up: {ui.device.path}", flush=True)
    buf = ctypes.create_string_buffer(64)
    end = time.monotonic() + seconds
    reports = 0
    seen_other = 0
    while not stop and time.monotonic() < end:
        bulk = Bulk(ep, 64, 500, ctypes.cast(buf, ctypes.c_void_p))
        try:
            n = fcntl.ioctl(fd, BULK, bulk)
        except OSError as err:
            if err.errno in (errno.ETIMEDOUT, errno.EINTR):
                continue
            raise
        d = buf.raw[:n]
        if n >= 6 and d[0] == 0x07:  # GIP Guide button
            ui.write(e.EV_KEY, e.BTN_MODE, d[4] & 1)
            ui.syn()
            continue
        if n < 18 or d[0] != 0x20:  # GIP input report
            continue
        reports += 1
        if reports == 1:
            print("first report read from the pad", flush=True)
        b = d[4] | d[5] << 8
        for bit, code in GIP_BUTTONS.items():
            ui.write(e.EV_KEY, code, (b >> bit) & 1)
        ui.write(e.EV_ABS, e.ABS_HAT0X, ((b >> 11) & 1) - ((b >> 10) & 1))
        ui.write(e.EV_ABS, e.ABS_HAT0Y, ((b >> 9) & 1) - ((b >> 8) & 1))
        lt, rt = struct.unpack_from("<HH", d, 6)
        ui.write(e.EV_ABS, e.ABS_Z, lt >> 2)
        ui.write(e.EV_ABS, e.ABS_RZ, rt >> 2)
        lx, ly, rx, ry = struct.unpack_from("<hhhh", d, 10)
        ui.write(e.EV_ABS, e.ABS_X, lx)
        ui.write(e.EV_ABS, e.ABS_Y, -ly - 1)
        ui.write(e.EV_ABS, e.ABS_RX, rx)
        ui.write(e.EV_ABS, e.ABS_RY, -ry - 1)
        ui.syn()
    print(f"stopping after {reports} reports", flush=True)
finally:
    if ui:
        ui.close()
    try:
        fcntl.ioctl(fd, RELEASEINTERFACE, struct.pack("I", iface))
    except OSError as err:
        print(f"release failed: {err}", flush=True)
    try:
        fcntl.ioctl(fd, IOCTL, UsbIoctl(iface, CONNECT, None))
        print("gave the interface back to the kernel driver", flush=True)
    except OSError as err:
        print(f"reconnect failed: {err} (replug the pad to restore it)", flush=True)
    os.close(fd)
