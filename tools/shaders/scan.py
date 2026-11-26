#!/usr/bin/env -S uv run --quiet --script
"""Find shader containers in a PE image.

    tools/shaders/scan.py EXE [--json target/shaders.json]

Reports DXBC (SM4/SM5 DXIL or SM4 bytecode) magic hits with their RVA, section,
size and sha256, so extraction and translation can be batched later. Container
walking is deliberately byte-level: a false positive is cheap, a missed blob is
not.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

MAGIC = b"DXBC"
# DXIL announces itself in the container; SM4/SM5 bytecode does not.
DXIL = b"DXIL"


def sections(d: bytes):
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, pe + 6)[0]
    optsz = struct.unpack_from("<H", d, pe + 20)[0]
    at = pe + 24 + optsz
    for i in range(nsec):
        o = at + 40 * i
        name = d[o:o + 8].rstrip(b"\0").decode("latin1")
        vs, va, rs, ra = struct.unpack_from("<IIII", d, o + 8)
        yield name, va, vs, ra, rs


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("exe", type=Path)
    ap.add_argument("--json", type=Path)
    ns = ap.parse_args()
    d = ns.exe.read_bytes()

    found = []
    for name, va, vs, ra, rs in sections(d):
        body = d[ra:ra + rs]
        at = 0
        while True:
            i = body.find(MAGIC, at)
            if i < 0:
                break
            at = i + 4
            # a container starts with magic, then a 16-byte digest and a size
            if i + 20 > len(body):
                continue
            total = struct.unpack_from("<I", body, i + 24)[0] if i + 28 <= len(body) else 0
            if not total or total > 0x100000 or i + total > len(body):
                continue
            blob = body[i:i + total]
            found.append({
                "rva": va + i,
                "section": name,
                "size": total,
                "dxil": DXIL in blob[:0x200],
                "sha256": hashlib.sha256(blob).hexdigest(),
            })
    out = {"exe": str(ns.exe), "count": len(found), "shaders": found}
    if ns.json:
        ns.json.parent.mkdir(parents=True, exist_ok=True)
        ns.json.write_text(json.dumps(out, indent=1))
    for s in found[:15]:
        print(f"0x{s['rva']:08x} {s['section']} {s['size']}B {'dxil' if s['dxil'] else 'sm4/5'} {s['sha256'][:12]}")
    print(f"shaders {len(found)} ({sum(1 for s in found if s['dxil'])} dxil)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
