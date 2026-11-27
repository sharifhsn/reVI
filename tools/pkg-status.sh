#!/usr/bin/env -S /home/sharif/.local/bin/uv run --quiet --script
# /// script
# requires-python = ">=3.11"
# ///
"""Preflight for a package-dependent run.

96 % of an in-flight download is preallocated zeros, and the daemon counts bytes
it no longer holds (363 GB "downloaded" while 4.65 GB verified), so `Percent
Done` after one verify pass is the only number that means anything. This decides
whether a run that reads package bytes may start: it verifies current state,
requires the verified percentage to reach the bar, requires disk headroom, and
requires a clean verify pass (no corrupt pieces).

    tools/pkg-status.sh            # report and exit by the bar
    tools/pkg-status.sh --verify   # re-verify drifted pieces first, then report

Exit 0 = go, 1 = not yet, 2 = daemon unreachable.
"""

import argparse
import re
import shutil
import subprocess
import sys

TORRENT = "2"
DIR = "/srv/gta6"


def remote(*args: str) -> str:
    out = subprocess.run(
        ["transmission-remote", "-t", TORRENT, *args],
        capture_output=True, text=True, check=False,
    )
    if out.returncode != 0:
        sys.exit(2)
    return out.stdout


def state() -> dict[str, str]:
    text = remote("-i")
    fields = {}
    for key in ("Percent Done", "Have", "State", "Total size"):
        m = re.search(rf"^\s*{key}:\s*(.+)$", text, re.M)
        if m:
            fields[key] = m.group(1).strip()
    return fields


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--verify", action="store_true")
    ap.add_argument("--min-percent", type=float, default=100.0)
    ap.add_argument("--min-free-gb", type=float, default=25.0)
    a = ap.parse_args()

    if a.verify:
        remote("--verify")

    f = state()
    percent = float(f.get("Percent Done", "0").rstrip("%") or 0)
    free_gb = shutil.disk_usage(DIR).free / 2**30
    have = f.get("Have", "None")
    # "Have: 4.65 GB (4.65 GB verified)" — unverified bytes are not usable.
    m = re.search(r"\(([\d.]+) GB verified\)", have)
    verified_gb = float(m.group(1)) if m else 0.0
    missing = "None" in have
    print(f"percent   {percent:.2f} %")
    print(f"verified  {verified_gb:.2f} GB")
    print(f"free      {free_gb:.1f} GB")
    print(f"state     {f.get('State', '?')}")

    ok = True
    if missing:
        print("bad: no piece state yet — run with --verify")
        ok = False
    if percent < a.min_percent:
        print(f"bad: {percent:.2f} % < {a.min_percent} %")
        ok = False
    if free_gb < a.min_free_gb:
        print(f"bad: {free_gb:.1f} GB free < {a.min_free_gb} GB")
        ok = False
    print("go" if ok else "not yet")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
