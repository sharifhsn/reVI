#!/usr/bin/env -S uv run --quiet --script
"""Tiny HTTP file channel between the guest and the host.

GET  serves FILES_DIR (guest -> host downloads)
PUT  writes into FILES_DIR (guest -> host uploads; this is how logs come back)

Usage: upload-server.py FILES_DIR PORT
Bind address is 0.0.0.0 so the guest can reach it at 10.0.2.2 under QEMU
user-mode networking. Loopback-only would be invisible to the guest.
"""
import http.server
import os
import sys

root = os.path.abspath(sys.argv[1])
port = int(sys.argv[2])
os.makedirs(root, exist_ok=True)
os.chdir(root)


class Handler(http.server.SimpleHTTPRequestHandler):
    def do_PUT(self):
        name = os.path.basename(self.path.lstrip("/"))
        length = int(self.headers.get("Content-Length", 0))
        with open(name, "wb") as f:
            f.write(self.rfile.read(length))
        self.send_response(201)
        self.end_headers()
        self.wfile.write(b"ok\n")

    do_POST = do_PUT

    def log_message(self, fmt, *args):
        sys.stderr.write("%s\n" % (fmt % args))


http.server.ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()
