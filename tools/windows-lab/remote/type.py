#!/usr/bin/env python3
"""Send keystrokes to the lab VM through the QEMU monitor.

Usage: type.py [-n] TEXT      (-n: do not press Enter at the end)
Env:   LAB_MON=/path/to/monitor.sock

Monitor `sendkey` names are qcodes, not characters. The table below is the
complete set we have needed; anything unlisted must be added here rather than
hoped for, because a name QEMU does not know fails silently.
"""
import os
import socket
import sys
import time

MON = os.environ["LAB_MON"]
PRESS_ENTER = "-n" not in sys.argv

MAP = {
    " ": "spc", "\\": "backslash", "/": "slash", ".": "dot", "-": "minus",
    ":": "shift-semicolon", "[": "bracket_left", "]": "bracket_right",
    ">": "shift-dot", "<": "shift-comma", "~": "shift-grave", "'": "apostrophe",
    "|": "shift-backslash", "&": "shift-7", "=": "equal", ";": "semicolon",
    "(": "shift-9", ")": "shift-0", '"': "shift-apostrophe", "@": "shift-2",
    "#": "shift-3", "$": "shift-4", "%": "shift-5", "^": "shift-6", "*": "shift-8",
    "_": "shift-minus", "+": "shift-equal", "{": "shift-bracket_left",
    "}": "shift-bracket_right", "!": "shift-1", "?": "shift-slash",
}
for c in "abcdefghijklmnopqrstuvwxyz0123456789":
    MAP.setdefault(c, c)

text = " ".join(a for a in sys.argv[1:] if a != "-n")

s = socket.socket(socket.AF_UNIX)
s.connect(MON)


def send(name):
    s.sendall(("sendkey %s\n" % name).encode())
    time.sleep(0.06)


for ch in text:
    if ch in MAP:
        send(MAP[ch])
    elif ch.isupper():
        send("shift-" + ch.lower())
    else:
        sys.stderr.write("no qcode for %r\n" % ch)
        sys.exit(2)

if PRESS_ENTER:
    s.sendall(b"sendkey ret\n")
