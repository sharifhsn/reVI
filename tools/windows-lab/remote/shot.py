#!/usr/bin/env python3
"""Trigger a screendump in the VM.

Usage: shot.py OUT.ppm
Env:   LAB_MON=/path/to/monitor.sock
"""
import os
import socket
import sys
import time

s = socket.socket(socket.AF_UNIX)
s.connect(os.environ["LAB_MON"])
s.sendall(("screendump %s\n" % os.path.abspath(sys.argv[1])).encode())
time.sleep(1.0)
s.settimeout(1.5)
try:
    while s.recv(65536):
        pass
except socket.timeout:
    pass
