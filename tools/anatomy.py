#!/usr/bin/env -S uv run --quiet --script
"""Render the executable anatomy as an SVG. Pure stdlib."""
import json, sys
from html import escape

J = json.load(open(sys.argv[1]))
OUT = sys.argv[2]

W, H = 1900, 1380
BG = "#0e131b"
FG = "#e6edf5"
DIM = "#8b98ab"
GRID = "#1d2635"
PANEL = "#131a24"

C = {
    "unique": "#39465c",
    "leaf": "#1f6f5c",
    "tail": "#55607a",
    "split": "#a56a1c",
    "giant": "#6b3fa0",
    "alias": "#2f6fa8",
    "claimed": "#2ecc71",
}

buckets = {int(k): v for k, v in J["buckets"].items()}
keys = sorted(buckets)
MB = J["bucket"]
B = 12.0
X0, Y0 = 190, 200
BAR_H = 250

def mmb(n):
    return f"{n/1e6:.1f} MB" if n >= 1e6 else f"{n/1e3:.1f} kB"

out = []
add = out.append
add(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="ui-monospace,SF Mono,Menlo,monospace">')
add(f'<rect width="{W}" height="{H}" fill="{BG}"/>')

# header
add(f'<text x="70" y="62" fill="{FG}" font-size="30" font-weight="700">game_scarlett_master.exe — anatomy</text>')
add(f'<text x="70" y="92" fill="{DIM}" font-size="14">build 1.0.2.1 · {J["size"]:,} bytes · sha256 e95020b2…a08993 · blake3 b99f876b…239e93 · image base {hex(J["image_base"])} · entry {hex(J["entry"])}</text>')
add(f'<text x="70" y="116" fill="#f0b429" font-size="14">boundary = every .pdata RUNTIME_FUNCTION, 299,257 records. type info is stripped; vtables are inferred by shape, not RTTI.</text>')

# axis labels
add(f'<text x="{X0}" y="{Y0-14}" fill="{DIM}" font-size="12">{hex(J["code_start"])}</text>')

add(f'<text x="{X0+len(keys)*B/2}" y="{Y0-14}" text-anchor="middle" fill="{DIM}" font-size="12">.text · {len(keys)} buckets of {mmb(MB)} · height = sqrt(function count), clipped at p98</text>')
for i in range(0, len(keys), 8):
    x = X0 + i * B
    add(f'<line x1="{x}" y1="{Y0-8}" x2="{x}" y2="{Y0+BAR_H+112}" stroke="{GRID}"/>')
    add(f'<text x="{x}" y="{Y0+BAR_H+128}" fill="{DIM}" font-size="10" text-anchor="middle">+{i}M</text>')
# entry marker
ex = X0 + (J["entry"] // MB) * B
add(f'<polygon points="{ex-4},{Y0-24} {ex+4},{Y0-24} {ex},{Y0-16}" fill="#f0b429"/>')
add(f'<text x="{ex}" y="{Y0-13}" fill="#f0b429" font-size="10" text-anchor="middle">entry</text>')

import math
mx = sorted(v["n"] for v in buckets.values())[int(len(buckets) * 0.98)]
def hscale(n):
    return math.sqrt(n / mx) * BAR_H
claimed = {int(k, 16): v for k, v in J["claimed"].items()}

# main columns
for idx, k in enumerate(keys):
    v = buckets[k]
    va = k * MB
    x = X0 + idx * B
    y = Y0 + BAR_H
    for cls in ("alias", "unique", "tail", "split", "giant", "leaf"):
        n = v["cls"].get(cls, 0)
        if not n:
            continue
        h = hscale(n)
        y -= h
        add(f'<rect x="{x:.1f}" y="{y:.1f}" width="{B-1:.1f}" height="{h:.1f}" fill="{C[cls]}"/>')
    # signal rows: claims, imports, pointer tables, strings
    if [r for r in claimed if va <= r < va + MB]:
        add(f'<rect x="{x:.1f}" y="{Y0+BAR_H+6}" width="{B-1:.1f}" height="10" fill="{C["claimed"]}"/>')
    if v["imp"]:
        add(f'<rect x="{x+3:.1f}" y="{Y0+BAR_H+30}" width="{B-7:.1f}" height="{min(math.sqrt(v["imp"]/8)*26,26):.1f}" fill="#c98a2b" opacity=".9"/>')
    if v["vt"]:
        add(f'<rect x="{x+3:.1f}" y="{Y0+BAR_H+68}" width="{B-7:.1f}" height="{min(math.sqrt(v["vt"]/80)*22,22):.1f}" fill="#7a5cff" opacity=".9"/>')
    if v["strings"]:
        add(f'<rect x="{x+3:.1f}" y="{Y0+BAR_H+100}" width="{B-7:.1f}" height="{min(math.sqrt(v["strings"]/60)*16,16):.1f}" fill="#3aa0a0" opacity=".9"/>')

# baseline
add(f'<line x1="{X0}" y1="{Y0+BAR_H}" x2="{X0+len(keys)*B}" y2="{Y0+BAR_H}" stroke="{GRID}"/>')
for label, dy, col in (("claims", 14, C["claimed"]), ("import calls", 40, "#c98a2b"),
                       ("pointer-table refs", 78, "#7a5cff"), ("string literals", 108, "#3aa0a0")):
    add(f'<text x="{X0-12}" y="{Y0+BAR_H+dy}" fill="{col}" font-size="11" text-anchor="end">{label}</text>')

# coverage ribbon
RY = Y0 + BAR_H + 150
covered = sum(buckets[k]["bytes"] for k in keys)
add(f'<text x="{X0-12}" y="{RY+11}" fill="{DIM}" font-size="11" text-anchor="end">record coverage</text>')
add(f'<text x="{X0+len(keys)*B}" y="{RY+46}" text-anchor="end" fill="{DIM}" font-size="11">end {hex(J["code_end"])}</text>')
add(f'<text x="{X0}" y="{RY+46}" fill="{DIM}" font-size="12"> — {mmb(covered)} of .text in records, {mmb(J["sections"][0]["vsize"]-covered)} bytes with no record</text>')
for idx, k in enumerate(keys):
    v = buckets[k]
    x = X0 + idx * B
    cov = min(v["bytes"] / MB, 1.0)
    add(f'<rect x="{x:.1f}" y="{RY}" width="{B-1:.1f}" height="14" fill="#1b2230"/>')
    add(f'<rect x="{x:.1f}" y="{RY}" width="{B-1:.1f}" height="14" fill="{C["claimed"] if any(va := k*MB <= r < k*MB+MB for r in claimed) else "#3d7f5f"}" opacity=".25"/>')
    add(f'<rect x="{x:.1f}" y="{RY+14:.1f}" width="{B*cov:.1f}" height="4" fill={chr(34)}#5f8f7a{chr(34)}/>')


# ---------------- left column: sections ----------------
LX, LY = 70, 720
add(f'<rect x="{LX-16}" y="{LY-34}" width="700" height="300" fill="{PANEL}"/>')
add(f'<text x="{LX}" y="{LY-10}" fill="{FG}" font-size="16" font-weight="600">sections</text>')
for i, s in enumerate(J["sections"]):
    y = LY + 18 + i * 26
    col = "#2ecc71" if s["exec"] else DIM
    add(f'<text x="{LX}" y="{y}" fill="{col}" font-size="12">{escape(s["name"]):<12}</text>')
    add(f'<text x="{LX+110}" y="{y}" fill="{DIM}" font-size="12">{hex(s["va"])}</text>')
    add(f'<text x="{LX+200}" y="{y}" fill="{DIM}" font-size="12">{mmb(s["vsize"]):>10}</text>')
    add(f'<rect x="{LX+310}" y="{y-9}" width="{s["entropy"]/8*180:.0f}" height="9" fill="#7a5cff" opacity=".55"/>')
    add(f'<text x="{LX+500}" y="{y}" fill="{DIM}" font-size="12">H {s["entropy"]:.2f}</text>')
add(f'<text x="{LX}" y="{LY+282}" fill="{DIM}" font-size="11">entropy bar: 8.0 = looks compressed/encrypted, &lt;4 = structured</text>')

# ---------------- right column: imports ----------------
RX, RY2 = 830, 720
add(f'<rect x="{RX-16}" y="{RY2-34}" width="760" height="300" fill="{PANEL}"/>')
add(f'<text x="{RX}" y="{RY2-10}" fill="{FG}" font-size="16" font-weight="600">imports — {J["imports_total"]} symbols in {J["modules"]} modules</text>')
top = J["modules_top"][:10]
m = top[0][1] if top else 1
for i, (name, n) in enumerate(top):
    y = RY2 + 16 + i * 26
    add(f'<text x="{RX}" y="{y}" fill="{DIM}" font-size="11">{escape(name[:44])}</text>')
    add(f'<rect x="{RX+340}" y="{y-9}" width="{n/m*240:.0f}" height="9" fill="#c98a2b" opacity=".8"/>')
    add(f'<text x="{RX+590}" y="{y}" fill="{DIM}" font-size="11">{n}</text>')

# ---------------- stats ----------------
SY = 1100
add(f'<rect x="54" y="{SY-34}" width="{W-108}" height="196" fill="{PANEL}"/>')
add(f'<text x="70" y="{SY-10}" fill="{FG}" font-size="16" font-weight="600">what the records alone tell us</text>')
stats = [
    (f"{J['functions']:,}", "functions with an unwind record"),
    (f"{mmb(covered)}", "of .text inside records"),
    (f"{mmb(J['sections'][0]['vsize']-covered)}", ".text with no record (thunks, data-in-code, padding)"),
    (f"{J['eh_functions']:,}", "functions with an EH handler"),
    (f"{J['pointer_tables']:,}", "pointer tables (vtable-shaped)"),
    (f"{J['vtable_functions']:,}", "functions referenced from one"),
    (f"{J['strings_total']:,}", "string literals found"),
    (f"{J['rtti_classes']}", "RTTI names, all std:: — game RTTI is stripped"),
]
for i, (n, label) in enumerate(stats):
    x = 70 + (i % 4) * 450
    y = SY + 34 + (i // 4) * 54
    add(f'<text x="{x}" y="{y}" fill="{FG}" font-size="26" font-weight="700">{n}</text>')
    add(f'<text x="{x}" y="{y+18}" fill="{DIM}" font-size="11">{escape(label)}</text>')

# ---------------- legend + unmapped ----------------
add(f'<text x="70" y="{SY+160}" fill="{DIM}" font-size="12">legend</text>')
add(f'<text x="{W-70}" y="{SY+160}" fill="{DIM}" font-size="12" text-anchor="end">claims cover 0.0004% of record bytes</text>')
lx = 140
for cls, label in (("claimed", "claimed"), ("leaf", "leaf / no calls"), ("alias", "duplicate body"),
                   ("split", "split (EH / cold)"), ("giant", ">16 kB"), ("tail", "epilogue fragment"),
                   ("unique", "ordinary, unclaimed")):
    add(f'<rect x="{lx}" y="{SY+149}" width="12" height="12" fill="{C[cls]}"/>')
    add(f'<text x="{lx+18}" y="{SY+160}" fill="{DIM}" font-size="12">{label}</text>')
    lx += 150

add(f'<text x="70" y="{SY+214}" fill="#f0b429" font-size="13">Unmapped is the whole semantic layer: {J["functions"]-len(J["claimed"]):,} bodies with no reading, no callers, no types, no names, and no run behind any claim.</text>')

add(f'<text x="70" y="{H-14}" fill="{DIM}" font-size="11">generated by tools/anatomy.py from the header, .pdata, import table, pointer tables and string pools of the build above · no code bytes are stored</text>')
add('</svg>')

open(OUT, "w").write("\n".join(out))
print("wrote", OUT, len(out), "elements")
