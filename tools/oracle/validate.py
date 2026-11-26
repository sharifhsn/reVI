#!/usr/bin/env -S uv run --quiet --script
"""Validate an oracle vector file against the schema and the image map.

    tools/oracle/validate.py VECTORS.jsonl [--map map.jsonl] [--exe-size N]

Rules the schema cannot express are here: the RVA must be a mapped function
extent, every `rva:` span must land inside a section, and the arg count must be
consistent with the declared scratch buffer. Exit 1 on any problem.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

SECTION_LIMIT = 0x1000       # bytes per rva: span, generous on purpose
TOTAL_BLOB_LIMIT = 64 << 10  # per vector


def load_map(path: Path) -> dict[int, int]:
    extents: dict[int, int] = {}
    for i, line in enumerate(path.read_text().splitlines()):
        if i == 0 or not line.strip():
            continue
        rva, size, _ = line.split(",")
        extents[int(rva)] = int(size)
    return extents


def problems(vec: dict, index: int, extents: dict[int, int], image_size: int):
    def bad(rule: str, msg: str):
        yield f"line {index}: {rule}: {msg}"

    rva = vec.get("rva")
    if not isinstance(rva, int):
        yield from bad("rva", "missing or not an integer")
        return
    if rva not in extents:
        yield from bad("rva", f"0x{rva:x} is not a function extent start")
    build = vec.get("build", "")
    if build.count(".") != 3 or not all(p.isdigit() for p in build.split(".")):
        yield from bad("build", f"{build!r} is not a.b.c.d")

    args = vec.get("args")
    if not isinstance(args, list) or not args:
        yield from bad("args", "must be a non-empty list")
        args = []
    if len(args) > 8:
        yield from bad("args", f"{len(args)} entries, max 8")
    for a in args:
        if not isinstance(a, int) or a < 0 or a > 0xFFFF_FFFF_FFFF_FFFF:
            yield from bad("args", f"{a!r} is not a u64")

    buf = vec.get("argbuf")
    if buf is not None:
        if not isinstance(buf, int) or buf < 1:
            yield from bad("argbuf", "must be a positive size")
        if not args:
            yield from bad("argbuf", "declared without an arg1 to put it in")

    total = 0
    for span in vec.get("mem", []):
        s = span.get("span", "")
        try:
            kind, hexaddr = s.split(":", 1)
            addr = int(hexaddr, 16)
        except ValueError:
            yield from bad("mem", f"{s!r} is not rva:0x.. or abs:0x..")
            continue
        if kind == "rva" and addr >= image_size:
            yield from bad("mem", f"0x{addr:x} is past the image")
        raw = __import__("base64").b64decode(span.get("bytes", ""), validate=False)
        total += len(raw)
        if kind == "rva" and len(raw) > SECTION_LIMIT:
            yield from bad("mem", f"{len(raw)} bytes at one rva span, max {SECTION_LIMIT}")

    for w in vec.get("writes", []):
        if not isinstance(w.get("len"), int) or not 1 <= w["len"] <= 4096:
            yield from bad("writes", f"len {w.get('len')!r} outside 1..4096")
        h = w.get("sha256", "")
        if len(h) != 64 or any(c not in "0123456789abcdef" for c in h):
            yield from bad("writes", f"sha256 {h[:8]!r} malformed")

    if total > TOTAL_BLOB_LIMIT:
        yield from bad("mem", f"{total} bytes of blobs, max {TOTAL_BLOB_LIMIT}")

    ret = vec.get("returns", "")
    if not ret.startswith("0x") or len(ret) > 18 or any(c not in "0123456789abcdef" for c in ret[2:]):
        yield from bad("returns", f"{ret!r} is not 0x<hex>")
    note = vec.get("note")
    if note is not None and len(note) > 400:
        yield from bad("note", f"{len(note)} chars, max 400")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("vectors", type=Path)
    ap.add_argument("--map", type=Path, default=Path("/tmp/revimap.jsonl"))
    ap.add_argument("--exe-size", type=int, default=120_181_760)
    ns = ap.parse_args()
    if not ns.map.exists():
        print(f"no map at {ns.map}; run `revi map --out` first", file=sys.stderr)
        return 2
    extents = load_map(ns.map)

    bad = 0
    good = 0
    for i, line in enumerate(ns.vectors.read_text().splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        try:
            vec = json.loads(line)
        except json.JSONDecodeError as e:
            print(f"line {i}: json: {e}")
            bad += 1
            continue
        errs = list(problems(vec, i, extents, ns.exe_size))
        if errs:
            bad += 1
            print("\n".join(errs))
        else:
            good += 1
    print(f"vectors {good} ok, {bad} bad")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
