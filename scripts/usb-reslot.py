#!/usr/bin/env python3
# Usage: usb-reslot.py /dev/bus/usb/BBB/DDD INTERFACE HOLD_SECONDS
# Detach the pad's driver, create a stand-in uinput pad while the real one is gone, reattach the
# driver while keeping the stand-in, then hold it so Steam's log shows who got which index.
import ctypes, fcntl, os, struct, sys, time
from evdev import UInput, AbsInfo, ecodes as e
def _ioc(d, nr, size): return (d << 30) | (size << 16) | (ord('U') << 8) | nr
class DisconnectClaim(ctypes.Structure):
    _fields_ = [("interface", ctypes.c_uint), ("flags", ctypes.c_uint), ("driver", ctypes.c_char * 256)]
class UsbIoctl(ctypes.Structure):
    _fields_ = [("ifno", ctypes.c_int), ("ioctl_code", ctypes.c_int), ("data", ctypes.c_void_p)]
DISCONNECT_CLAIM = _ioc(2, 27, ctypes.sizeof(DisconnectClaim)); RELEASE = _ioc(2, 16, 4)
IOCTL = _ioc(3, 18, ctypes.sizeof(UsbIoctl)); CONNECT = _ioc(0, 23, 0)
node, iface, hold = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
stick = AbsInfo(0, -32768, 32767, 16, 128, 0); trig = AbsInfo(0, 0, 255, 0, 0, 0)
caps = {e.EV_KEY: [e.BTN_A, e.BTN_B, e.BTN_X, e.BTN_Y, e.BTN_TL, e.BTN_TR, e.BTN_SELECT, e.BTN_START, e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR],
        e.EV_ABS: [(e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick), (e.ABS_Z, trig), (e.ABS_RZ, trig)]}
fd = os.open(node, os.O_RDWR)
fcntl.ioctl(fd, DISCONNECT_CLAIM, DisconnectClaim(iface, 0, b""))
print("detached", flush=True)
time.sleep(1.0)
ui = UInput(caps, name="Reslot Stand-in", vendor=0x045e, product=0x028e, version=0x110, bustype=e.BUS_USB)
print("stand-in up", flush=True)
time.sleep(1.5)
fcntl.ioctl(fd, RELEASE, struct.pack("I", iface))
fcntl.ioctl(fd, IOCTL, UsbIoctl(iface, CONNECT, None))
print("reattached", flush=True)
os.close(fd)
time.sleep(hold)
ui.close()
print("stand-in gone", flush=True)
