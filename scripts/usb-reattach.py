#!/usr/bin/env python3
"""Gives a USB interface back to its kernel driver: usb_reattach.py /dev/bus/usb/BBB/DDD IFACE"""
import ctypes, fcntl, os, sys

class UsbIoctl(ctypes.Structure):
    _fields_ = [("ifno", ctypes.c_int), ("ioctl_code", ctypes.c_int), ("data", ctypes.c_void_p)]

IOCTL = (3 << 30) | (ctypes.sizeof(UsbIoctl) << 16) | (ord('U') << 8) | 18
CONNECT = (ord('U') << 8) | 23

fd = os.open(sys.argv[1], os.O_RDWR)
try:
    fcntl.ioctl(fd, IOCTL, UsbIoctl(int(sys.argv[2]), CONNECT, None))
    print("reattached")
finally:
    os.close(fd)
