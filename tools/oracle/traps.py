#!/usr/bin/env -S uv run --quiet --script
"""Validate and summarise trap lists.

    tools/oracle/traps.py decomp/traps/*.jsonl

A trap is one address inside one claimed function where a run should stop, step,
or record. Kept per function so a claim carries its own expectations.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

KINDS = {"entry", "call", "store", "branch", "ret", "fail"}
MODES = {"trap", "step", "record"}


def check(path: Path, extents: dict[int, int]) -> list[str]:
    errs: list[str] = []
    lines = path.read_text().splitlines()
    traps = [l for l in lines if l.strip() and not l.lstrip().startswith("#")]
    if not traps:
        return [f"{path.name}: empty"]
    seen: set[tuple[int, str, str]] = set()
    for i, line in enumerate(traps, 1):
        where = f"{path.name}:{i}"
        try:
            t = json.loads(line)
        except json.JSONDecodeError as e:
            errs.append(f"{where}: json: {e}")
            continue
        if t.get("kind") not in KINDS:
            errs.append(f"{where}: kind {t.get('kind')!r} not in {sorted(KINDS)}")
        if t.get("mode") not in MODES:
            errs.append(f"{where}: mode {t.get('mode')!r} not in {sorted(MODES)}")
        if t.get("kind") == "fail" and t.get("returns") is None:
            errs.append(f"{where}: a fail trap needs the expected value")
        at = t.get("at", "entry")
        if not (at == "entry" or (at.startswith("+0x") and int(at[3:], 16) < 0x1000)):
            errs.append(f"{where}: at {at!r} must be entry or +0x<off>")
        key = (t.get("rva", 0), at, str(t.get("kind")))
        if key in seen:
            errs.append(f"{where}: duplicate {t.get('kind')} trap at {at}")
        seen.add(key)
        rva = t.get("rva")
        if not isinstance(rva, int) or rva not in extents:
            errs.append(f"{where}: rva {rva!r} is not a function extent")
            continue
        off = 0 if at == "entry" else int(at[3:], 16)
        if off >= extents[rva]:
            errs.append(f"{where}: offset {off:#x} past the {extents[rva]}-byte extent")
        note = t.get("note", "")
        if not note:
            errs.append(f"{where}: no note")
        elif len(note) > 200:
            errs.append(f"{where}: note {len(note)} chars, max 200")
    return errs


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+", type=Path)
    ap.add_argument("--map", type=Path, default=Path("/tmp/revimap.jsonl"))
    ns = ap.parse_args()
    extents: dict[int, int] = {}
    for i, line in enumerate(ns.map.read_text().splitlines()):
        if i and line.strip():
            rva, size, _ = line.split(",")
            extents[int(rva)] = int(size)
    total = 0
    bad = 0
    for f in ns.files:
        errs = check(f, extents)
        n = len([l for l in f.read_text().splitlines() if l.strip()])
        total += n
        print(f"{f.name}: {n} traps, {'ok' if not errs else 'bad'}")
        for e in errs:
            print(f"  {e}")
        bad += bool(errs)
    print(f"traps {total}, files {len(ns.files)}, {bad} bad")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
