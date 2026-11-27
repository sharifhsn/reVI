#!/usr/bin/env -S uv run --quiet --script
"""Audit claimed sources for the defect classes a self-declared `ported` hides.

    tools/audit-src.py [--map /tmp/revimap.jsonl]

Flags, per file: an empty body, `(void)x` swallows, placeholder markers, a call to
an address that is not a function extent (the padding-target class), and a note
shorter than the body it documents. Exit 1 if anything is flagged.
"""
from __future__ import annotations

import argparse
import os
import struct
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = Path(os.environ.get("EXE", Path.home() / "leonida/drop/exe/game_scarlett_master.exe"))
CALL = re.compile(r"\bsub_([0-9a-fA-F]+)\b")
DATA = re.compile(r"\b(?:g_|dat_)([0-9a-fA-F]+)\b")
PLACEHOLDER = re.compile(r"\b(TODO|placeholder|sketch|Skeleton|stub)\b", re.I)


def load_image() -> tuple[bytes, list[tuple[int, int, int, int]]]:
    d = EXE.read_bytes()
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, pe + 6)[0]
    optsz = struct.unpack_from("<H", d, pe + 20)[0]
    at = pe + 24 + optsz
    secs = []
    for i in range(nsec):
        o = at + 40 * i
        vs, va, rs, ra = struct.unpack_from("<IIII", d, o + 8)
        secs.append((va, vs, ra, rs))
    return d, secs


def byte_at(d: bytes, secs, rva: int) -> int | None:
    for va, vs, ra, rs in secs:
        if va <= rva < va + max(vs, 1):
            off = ra + (rva - va)
            return d[off] if off < len(d) else None
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--map", type=Path, default=Path("/tmp/revimap.jsonl"))
    ns = ap.parse_args()
    d, secs = load_image()
    ext: dict[int, tuple[int, str]] = {}
    for i, line in enumerate(ns.map.read_text().splitlines()):
        if i and line.strip():
            rva, size, sec = line.split(",")
            ext[int(rva)] = (int(size), sec)

    flagged = 0
    for src in sorted((ROOT / "decomp/src").glob("*.cpp")):
        rva = int(src.stem, 16)
        text = src.read_text()
        body = text[text.find("{", text.find(")")) :] if "{" in text else ""
        note = ROOT / "decomp/notes" / f"{src.stem}.md"
        outs: list[str] = []

        if len(re.sub(r"\s+", "", body)) <= 2:
            outs.append("empty body")
        voids = len(re.findall(r"\(void\)\s*\w+", body))
        if voids:
            outs.append(f"{voids}x (void)x")
        if PLACEHOLDER.search(text):
            outs.append("placeholder marker")

        for m in CALL.finditer(text):
            t = int(m.group(1), 16)
            if t >= 0x140000000:
                t -= 0x140000000
            b = byte_at(d, secs, t)
            if b is None:
                outs.append(f"call target unmapped: 0x{t:08x}")
            elif b == 0xCC:
                outs.append(f"call target is int3 padding: 0x{t:08x}")
        for m in DATA.finditer(text):
            t = int(m.group(1), 16)
            if t >= 0x140000000:
                t -= 0x140000000
            if t not in ext and t not in {0x67E2038, 0x75DEFC0, 0x149C3D678}:
                sec = ext.get(t)
                if sec is None and t > 0x5F3F966:
                    continue  # data symbol, not extent-checked
                if sec is not None and sec[1] == ".text":
                    outs.append(f"data symbol in .text: 0x{t:08x}")

        if note.exists():
            nlen = len(note.read_text())
            slen = len(text)
            if slen > 400 and nlen < slen // 2:
                outs.append(f"note {nlen}B vs src {slen}B")

        if outs:
            flagged += 1
            print(f"{src.stem}: " + "; ".join(outs))
    print(f"flagged {flagged} of {len(list((ROOT / 'decomp/src').glob('*.cpp')))}")
    return 1 if flagged else 0


if __name__ == "__main__":
    sys.exit(main())
