#!/usr/bin/env -S uv run --quiet --script
"""Click at image coordinates in the lab VM's framebuffer.

Usage: click.py X Y [--double]
Env:   LAB_VNC_PORT=5902, LAB_SHOT_W=1280, LAB_SHOT_H=800

Coordinates are given in the *screenshot* space and scaled to the server's
framebuffer, so a screendump at any resolution can be clicked directly. The
scaling matters: without it, clicks on a smaller framebuffer land short.
"""
import os
import socket
import struct
import sys
import time

PORT = int(os.environ["LAB_VNC_PORT"])
SHOT_W = int(os.environ.get("LAB_SHOT_W", "1280"))
SHOT_H = int(os.environ.get("LAB_SHOT_H", "800"))

x_arg, y_arg = int(sys.argv[1]), int(sys.argv[2])
double = "--double" in sys.argv

s = socket.create_connection(("127.0.0.1", PORT), 10)
s.recv(12)
s.sendall(b"RFB 003.008\n")
n = s.recv(1)[0]
s.recv(n)
s.sendall(b"\x01")          # security type: none
s.recv(4)                   # security result
s.sendall(b"\x01")          # client init, shared
init = b""
while len(init) < 24:
    init += s.recv(24 - len(init))
w, h = struct.unpack(">HH", init[:4])
x, y = int(x_arg * w / SHOT_W), int(y_arg * h / SHOT_H)


def pointer(mask):
    s.sendall(struct.pack(">BBHH", 5, mask, x, y))


pointer(0)
time.sleep(0.15)
for _ in range(2 if double else 1):
    pointer(1)
    time.sleep(0.08)
    pointer(0)
    time.sleep(0.12)

print("server %dx%d sent %d,%d" % (w, h, x, y))
