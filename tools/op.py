#!/usr/bin/env -S uv run --quiet --script
"""One entry point, batched verbs, compact output.

Code-mode rule: a caller picks a verb, the verb does N operations and prints a
short summary; per-item detail goes to files under target/. Exit code is the
worst outcome of the verb.

    op.py gate                      every gate, in order
    op.py claim RVA...              create claims from the map
    op.py draft RVA...              decompile to target/drafts/
    op.py vectors record|replay [f] oracle vectors
    op.py traps                     validate trap lists
    op.py shaders scan              find shader containers in the image
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAP = os.environ.get("MAP", "/tmp/revimap.jsonl")
EXE = os.environ.get("EXE", str(Path.home() / "leonida/drop/exe/game_scarlett_master.exe"))
REVI = os.environ.get("REVI", str(Path.home() / "revi-map-src/target/release/revi"))
KUNA = os.environ.get("KUNA", str(Path.home() / "kuna-fork/decompiler/target/release/kuna"))
SPECS = os.environ.get("SPECS", str(Path.home() / "kuna-specs/specs"))
OUT = ROOT / "target"


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, text=True, capture_output=True, **kw)


def extents() -> dict[int, int]:
    out = {}
    for i, line in enumerate(Path(MAP).read_text().splitlines()):
        if i and line.strip():
            rva, size, _ = line.split(",")
            out[int(rva)] = int(size)
    return out


def verb_gate(ns) -> int:
    p = run([str(ROOT / "tools/check-all.sh")], env={**os.environ, "MAP": MAP, "EXE": EXE, "REVI": REVI})
    tail = (p.stdout + p.stderr).strip().splitlines()[-8:]
    print("\n".join(tail))
    return p.returncode


def verb_claim(ns) -> int:
    ext = extents()
    bad = 0
    for rva in ns.rva:
        if rva not in ext:
            print(f"0x{rva:08x} not an extent")
            bad += 1
            continue
        path = ROOT / f"decomp/claims/0x{rva:08x}.json"
        if path.exists():
            print(f"0x{rva:08x} already claimed")
            continue
        path.write_text(json.dumps({"rva": rva, "size": ext[rva], "chunks": 1,
                                    "status": "claimed", "owner": ns.owner,
                                    "claimed_at": ns.date}, indent=2) + "\n")
        print(f"0x{rva:08x} claimed ({ext[rva]} bytes)")
    return 1 if bad else 0


def verb_draft(ns) -> int:
    (OUT / "drafts").mkdir(parents=True, exist_ok=True)
    ok = 0
    for rva in ns.rva:
        f = OUT / f"drafts/0x{rva:08x}.c"
        p = run([KUNA, "decompile", EXE, hex(0x140000000 + rva), "--addr",
                 "--sleighpath", SPECS], timeout=600)
        text = p.stdout or p.stderr
        f.write_text(text)
        first = next((l for l in text.splitlines() if l.strip() and not l.startswith("note:")), "")
        print(f"0x{rva:08x} {'ok' if p.stdout else 'fail'} {f.relative_to(ROOT)} {len(text)}B {first[:60]}")
        ok += bool(p.stdout)
    return 0 if ok == len(ns.rva) else 1


def verb_vectors(ns) -> int:
    if ns.action == "record":
        f = ns.file or ROOT / "tools/oracle/examples/smoke.jsonl"
        p = run(["uv", "run", str(ROOT / "tools/oracle/replay.py"), str(f), "--exe", EXE,
                 "--revi", REVI, "--record"])
        print(p.stdout.strip() or p.stderr.strip())
        return p.returncode
    p = run(["uv", "run", str(ROOT / "tools/oracle/replay.py"),
             str(ns.file or ROOT / "tools/oracle/examples/smoke.jsonl"),
             "--exe", EXE, "--revi", REVI])
    print("\n".join((p.stdout + p.stderr).strip().splitlines()[-10:]))
    return p.returncode


def verb_traps(ns) -> int:
    files = sorted((ROOT / "decomp/traps").glob("*.jsonl"))
    p = run(["uv", "run", str(ROOT / "tools/oracle/traps.py"), "--map", MAP, *map(str, files)])
    print("\n".join((p.stdout + p.stderr).strip().splitlines()[-8:]))
    return p.returncode


def verb_shaders(ns) -> int:
    if ns.action != "scan":
        print("only `scan` exists yet")
        return 2
    p = run(["uv", "run", str(ROOT / "tools/shaders/scan.py"), EXE])
    print("\n".join((p.stdout + p.stderr).strip().splitlines()[-15:]))
    return p.returncode


def main() -> int:
    ap = argparse.ArgumentParser(prog="op.py")
    sub = ap.add_subparsers(dest="verb", required=True)

    sub.add_parser("gate")

    c = sub.add_parser("claim")
    c.add_argument("rva", type=lambda s: int(s, 16), nargs="+")
    c.add_argument("--owner", default="sharif")
    c.add_argument("--date", default="2026-11-26")

    d = sub.add_parser("draft")
    d.add_argument("rva", type=lambda s: int(s, 16), nargs="+")

    v = sub.add_parser("vectors")
    v.add_argument("action", choices=["record", "replay"])
    v.add_argument("file", nargs="?")

    sub.add_parser("traps")

    s = sub.add_parser("shaders")
    s.add_argument("action", choices=["scan"])

    ns = ap.parse_args()
    return {"gate": verb_gate, "claim": verb_claim, "draft": verb_draft,
            "vectors": verb_vectors, "traps": verb_traps,
            "shaders": verb_shaders}[ns.verb](ns)


if __name__ == "__main__":
    sys.exit(main())
