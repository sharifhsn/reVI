#!/usr/bin/env python3
"""One monitor command, with output.

Usage: mon.py "info usernet" [more commands...]
Env:   LAB_MON=/path/to/monitor.sock

Exists because raw monitor output is otherwise never seen: the earlier
helpers wrote commands and closed, so `info usernet` and friends looked empty.
"""
import os
import socket
import sys
import time

s = socket.socket(socket.AF_UNIX)
s.connect(os.environ["LAB_MON"])

for cmd in sys.argv[1:]:
    s.sendall((cmd + "\n").encode())
    time.sleep(0.4)
    s.settimeout(1.5)
    out = b""
    try:
        while True:
            chunk = s.recv(65536)
            if not chunk:
                break
            out += chunk
            if out.endswith(b"(qemu) "):
                break
    except socket.timeout:
        pass
    print(out.decode(errors="replace").replace("\r", ""))
