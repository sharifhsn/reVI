#!/usr/bin/env python3
"""Replay oracle vectors through `revi call` and compare.

    tools/oracle/replay.py VECTORS.jsonl --exe game.exe [--revi revi] [--limit N]
    tools/oracle/replay.py SPEC.jsonl --exe game.exe --record

Replay: each vector runs in a fresh process under the stub/TEB settings it was
recorded with; rax and every declared write range are compared. Exit 0 only when
every vector matches.

Record: the input is a spec of `{rva, args, argbuf?, dump?: ["abs:0x..:12"]}` and
the output is a full vector file, so a new build is re-baselined deliberately
rather than silently.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

RAX = re.compile(r"-> rax 0x([0-9a-f]+)")
HEX = re.compile(r"^[0-9a-f]+$")


def invoke(revi: str, exe: str, vec: dict, dumps: list[tuple[str, int]]) -> str:
    args: list[str] = []
    if vec.get("argbuf"):
        args += ["--arg-buf", str(vec["argbuf"])]
    for a in vec["args"]:
        args += ["--arg", str(a)]
    for span, ln in dumps:
        args += ["--dump", f"{span}:{ln}"]
    env = {**os.environ, "REVI_STUBS": "1" if vec.get("stubs", True) else "0"}
    if vec.get("gs"):
        env["REVI_GS"] = "1"
    proc = subprocess.run([revi, "call", exe, hex(vec["rva"]), *args],
                          capture_output=True, text=True, env=env, timeout=180)
    return proc.stdout


def hexlines(out: str) -> list[str]:
    return [l for l in out.splitlines() if l and HEX.match(l)]


def replay_one(revi: str, exe: str, vec: dict) -> list[str]:
    dumps = [(w["span"], w["len"]) for w in vec.get("writes", [])]
    out = invoke(revi, exe, vec, dumps)
    if not out:
        return ["no output from revi call"]

    fails: list[str] = []
    m = RAX.search(out)
    got = f"0x{m.group(1)}" if m else "<none>"
    if got != vec["returns"]:
        fails.append(f"returns: want {vec['returns']} got {got}")

    lines = hexlines(out)
    for w, line in zip(vec.get("writes", []), lines):
        raw = bytes.fromhex(line)
        got_hash = hashlib.sha256(raw).hexdigest()
        if got_hash != w["sha256"]:
            fails.append(f"write {w['span']}: want {w['sha256'][:12]} got {got_hash[:12]}")
    if len(lines) < len(dumps):
        fails.append(f"only {len(lines)} dumps for {len(dumps)} write ranges")
    return fails


def record(revi: str, exe: str, spec: dict) -> dict:
    dumps = []
    for sp in spec.get("dump", []):
        span, ln = sp.rsplit(":", 1)
        dumps.append((span, int(ln)))
    out = invoke(revi, exe, spec, dumps)
    m = RAX.search(out)
    vec = {
        "rva": spec["rva"],
        "build": spec["build"],
        "args": spec["args"],
        "returns": f"0x{m.group(1)}" if m else "0x0",
        "stubs": spec.get("stubs", True),
        "writes": [],
    }
    if spec.get("argbuf"):
        vec["argbuf"] = spec["argbuf"]
    if spec.get("note"):
        vec["note"] = spec["note"]
    for (span, ln), line in zip(dumps, hexlines(out)):
        raw = bytes.fromhex(line)
        vec["writes"].append({"span": span, "len": ln,
                              "sha256": hashlib.sha256(raw).hexdigest()})
    return vec


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("vectors", type=Path)
    ap.add_argument("--exe", required=True)
    ap.add_argument("--revi", default="revi")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--record", action="store_true")
    ns = ap.parse_args()

    lines = [l for l in ns.vectors.read_text().splitlines()
             if l.strip() and not l.lstrip().startswith("#")]
    if ns.limit:
        lines = lines[: ns.limit]
    vecs = [json.loads(l) for l in lines]

    if ns.record:
        out = [json.dumps(record(ns.revi, ns.exe, v)) for v in vecs]
        ns.vectors.write_text("\n".join(out) + "\n")
        print(f"recorded {len(out)} vectors")
        return 0

    bad = 0
    for i, v in enumerate(vecs, 1):
        fails = replay_one(ns.revi, ns.exe, v)
        print(f"{i:4d} 0x{v['rva']:08x} {'FAIL' if fails else 'ok'}")
        for f in fails:
            print(f"      {f}")
        bad += bool(fails)
    print(f"replayed {len(vecs)}, {bad} failed")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
