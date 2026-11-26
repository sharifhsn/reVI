#!/usr/bin/env -S uv run --quiet --with pillow --script
"""Render docs/img/exe-structure.png from the image and /tmp/dlls.txt."""
import math, random
from PIL import Image, ImageDraw, ImageFont

W, H = 3200, 2600
BG = (11, 15, 20)
FG = (225, 232, 242)
DIM = (122, 136, 154)
RULE = (40, 49, 62)
ACC = {"code": (92, 176, 255), "ro": (142, 208, 138), "rw": (238, 166, 92),
       "meta": (194, 148, 244), "reloc": (118, 128, 144), "shader": (255, 118, 168),
       "hot": (255, 206, 92)}
MONO, SANS, SANSB = "/System/Library/Fonts/Menlo.ttc", "/System/Library/Fonts/Supplemental/Arial.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf"


def F(p, s):
    return ImageFont.truetype(p, s)


ft = {"title": F(SANSB, 92), "sub": F(MONO, 32), "h": F(SANSB, 42), "hl": F(MONO, 25),
      "body": F(MONO, 25), "small": F(MONO, 20), "tiny": F(MONO, 17), "num": F(SANSB, 50)}

img = Image.new("RGB", (W, H), BG)
d = ImageDraw.Draw(img)


def t(xy, s, k, fill=FG, anchor="la"):
    d.text(xy, s, font=ft[k], fill=fill, anchor=anchor)


def wrap(s, k, width):
    out, line = [], ""
    for w in s.split():
        c = (line + " " + w).strip()
        if d.textlength(c, font=ft[k]) <= width:
            line = c
        else:
            out.append(line)
            line = w
    out.append(line)
    return out


def para(x, y, s, k, fill, width, lead=8):
    for ln in wrap(s, k, width):
        t((x, y), ln, k, fill)
        y += ft[k].size + lead
    return y


def hr(y, x0=130, x1=W - 130, col=RULE, w=2):
    d.line([(x0, y), (x1, y)], fill=col, width=w)


def box(x0, y0, x1, y1, col, fill=None, r=12, w=2):
    d.rounded_rectangle([x0, y0, x1, y1], radius=r, outline=col, width=w, fill=fill)


def dot(x, y, r, col, w=2, fill=None):
    d.ellipse([x - r, y - r, x + r, y + r], outline=col, width=w, fill=fill)


SEC = [
    ("MZ/PE", 0x400, 0x400, "meta", 6.10, "headers"),
    (".text", 0x5F3F966, 0x5F3A000, "code", 6.75, "299,257 functions"),
    (".rdata", 0x8A0C0C, 0x8A0C00, "ro", 5.90, "76,201 strings, imports, 2 vtables"),
    (".data", 0x43EC398, 0x43EC400, "rw", 2.38, "6.4 MB raw + 65 MB BSS"),
    (".pdata", 0x36CBAC, 0x36CC00, "meta", 6.88, "rva/end/unwind per function"),
    (".reloc", 0x12A350, 0x12A400, "reloc", 5.64, "type 3/10 entries"),
    (".tls", 0x1481, 0x1600, "meta", 0.17, "slot byte at +0x1294"),
    (".xbld", 0x6B9, 0x800, "meta", 4.54, "build stamp"),
    (".voltbl", 0x724, 0x800, "reloc", 5.93, "guard CF targets"),
    (".xbld2", 0xDC, 0x200, "meta", 2.87, "second stamp"),
]
DLLS = sorted(((int(l.split(None, 1)[0]), l.split(None, 1)[1].strip()) for l in open("/tmp/dlls.txt")), reverse=True)


def cls(n):
    if n.startswith("api-ms-win-crt") or n.startswith(("MSVCP", "VCRUNTIME", "ucrtbase")):
        return "crt"
    if ".GDK" in n or n.startswith(("AcpHal", "xmem", "XboxBvh", "Xbl", "GameCore", "xgameruntime")):
        return "gdk"
    if n.startswith(("api-ms-win-core", "ntdll", "kernel32", "KERNEL32")):
        return "os"
    return "third"


CC = {"crt": ACC["ro"], "gdk": ACC["hot"], "os": ACC["code"], "third": ACC["shader"]}

# ---- header
t((130, 92), "game_scarlett_master.exe", "title")
t((130, 208), "x86-64 PE   base 0x140000000   entry 0x1456a1ec8   120,181,760 bytes   blake3 b99f876b…", "sub", DIM)
t((130, 250), "MSVC 19.52 /O2 /GR-   Xbox Series GDK   no PDB, no symbols, no RTTI", "sub", DIM)
hr(320)

# ---- section strip
t((130, 366), "IMAGE LAYOUT", "h")
t((480, 378), "virtual sizes; the strip is to scale", "small", DIM)
SX0, SX1, SY = 130, W - 130, 468
BAR_H = 120
span = sum(s[1] for s in SEC)
x = SX0
segs = []
MINW = 48
for name, vs, rs, kind, ent, note in SEC:
    w = max(MINW, (SX1 - SX0 - MINW * (len(SEC) - 1)) * vs / span)
    segs.append((name, x, x + w, kind, vs, ent, note))
    x += w + 2
d.rectangle([SX0, SY, SX1, SY + BAR_H], fill=(20, 26, 34), outline=RULE)
for name, x0, x1, kind, vs, ent, note in segs:
    col = ACC[kind]
    d.rectangle([x0 + 2, SY + 2, x1 - 2, SY + BAR_H - 2], fill=tuple(int(c * .20) for c in col), outline=col, width=2)
    if name == ".data":
        raw_frac = 0x43EC400 / 0x43EC398
        split = x0 + 2 + (x1 - x0 - 4) * 0.09
        d.rectangle([split, SY + 2, x1 - 2, SY + BAR_H - 2], fill=(30, 24, 16))
    if x1 - x0 > 74:
        t(((x0 + x1) / 2, SY + 20), name, "hl", col, "ma")
        t(((x0 + x1) / 2, SY + 54), (f"{vs/1048576:.1f}M" if vs > 1 << 20 else f"{vs}B"), "small", DIM, "ma")
        t(((x0 + x1) / 2, SY + 84), f"H {ent:.1f}", "tiny", DIM, "ma")
    else:
        d.line([((x0 + x1) / 2, SY + BAR_H), ((x0 + x1) / 2, SY + BAR_H + 10)], fill=col)

# narrow segments are listed instead of labelled in place
t((SX0, SY + BAR_H + 18), "tail (too narrow to label in place, same scale): "
  ".pdata  .reloc  .tls  .xbld  .voltbl  .xbld2", "tiny", DIM)
ay = SY + BAR_H + 50
t((SX0, ay), "· .data is 92% BSS: 71.2 MiB virtual against 6.4 MiB on disk — the hatched part is uninitialised", "small", FG)
t((SX0, ay + 36), "· .pdata holds exactly one record per function, which is why the map can be built without heuristics", "small", FG)
t((SX0, ay + 72), "· entropy per section (H) separates code (6.8) from the zero page (0.2 in .tls); .data's 2.4 is BSS", "small", DIM)

# ---- import panel
PY = 800
COLW = 940
C1, C2, C3 = 130, 1180, 2230
t((C1, PY), "IMPORT SURFACE", "h")
t((C1, PY + 46), "587 symbols over 68 DLLs — each is a seam to fill", "small", DIM)
y = PY + 92
for g, label in (("crt", "C/C++ runtime"), ("os", "OS surface"), ("gdk", "Xbox GDK"), ("third", "third-party")):
    rows = [r for r in DLLS if cls(r[1]) == g]
    if not rows:
        continue
    col = CC[g]
    t((C1, y), f"{label} — {sum(n for n, _ in rows)} symbols / {len(rows)} DLLs", "hl", col)
    y += 34
    xx = C1
    for n, name in rows[:10]:
        r = min(12, 4 + 1.2 * math.sqrt(n))
        if xx + 2 * r > C1 + COLW - 40:
            break
        dot(xx + r, y + r, r, col, 2, tuple(int(c * .22) for c in col))
        t((xx + r, y + 2 * r + 4), str(n), "tiny", col, "ma")
        xx += 2 * r + 10
    t((C1, y + 80), "  ".join(n.replace(".dll", "") for _, n in rows[:5])[:86], "tiny", DIM)
    y += 172

# ---- graph panel
t((C2, PY), "CALL GRAPH", "h")
t((C2, PY + 50), "299,257 nodes, 825,041 edges", "small", DIM)
y = PY + 92
for v, k, col in (("13,140", "roots", ACC["code"]), ("82,223", "reachable (27.5%)", ACC["hot"]), ("217,034", "no live caller (72.5%)", DIM)):
    t((C2, y), v, "num", col)
    t((C2 + 200, y + 22), k, "small", col)
    y += 74
gx0, gy0, gx1, gy1 = C2, y + 6, C2 + COLW - 40, y + 300
random.seed(4)
nds = [(random.uniform(gx0 + 30, gx1 - 30), random.uniform(gy0 + 20, gy1 - 20)) for _ in range(160)]
for p in nds:
    for q in random.sample(nds, 2):
        d.line([p, q], fill=(30, 41, 54), width=1)
for i, p in enumerate(nds):
    col = ACC["hot"] if i < 24 else ACC["code"] if i < 104 else (58, 68, 82)
    dot(p[0], p[1], 3 if i > 104 else 4, col, 1, col)
t((gx0, gy1 + 8), "amber: the 24 claimed bodies", "tiny", ACC["hot"])

# ---- idiom panel
t((C3, PY), "IDIOM CLASSES", "h")
t((C3, PY + 50), "what the scan recognises without names", "small", DIM)
y = PY + 100
for label, n, col in (("TLS slot read", 7243, ACC["code"]), ("lock cmpxchg", 3336, ACC["hot"]),
                      ("lock xadd", 119, ACC["ro"]), ("int 0x29 fastfail", 80, ACC["meta"])):
    w = 620 * n / 7243
    d.rectangle([C3, y, C3 + max(6, w), y + 20], fill=tuple(int(c * .55) for c in col))
    t((C3 + max(6, w) + 10, y - 3), f"{n:,} {label}", "small", col)
    y += 40
y += 30
t((C3, y), "HARD FACTS", "h")
y += 52
for line in ("guard CF: .voltbl present", "TLS template: 5.2 KiB", "DXIL: 11 containers, 10 DXIL",
             "vtables found: 2 (59 and 97 slots)", "strings: 76,201 in .rdata"):
    t((C3, y), "·", "small", ACC["ro"])
    t((C3 + 18, y), line, "small", DIM)
    y += 30

# ---- subsystem chain
CY = 1706
t((C1, CY), "RENDERER SLOT POOL — read end to end", "h")
t((C1, CY + 48), "the one subsystem with a complete, verified reading", "small", DIM)
box(C1, CY + 92, W - 130, CY + 300, RULE)
CW = (W - 260 - 26 * 2 - 40 * 5) // 6
steps = [("harness alloc", "0x1662430", "CreateSemaphoreExW\n(0, n, 0x7fff)\nOOM -> trim, retry, trap"),
         ("g_slots[5]", "0x75defc0", "five handles\ninitially == 0"),
         ("pool init", "0x43c38a0", "5 threads\nSubrender:%02u\nclamps on failure"),
         ("worker", "0x43c4140", "TLS slot <- id\nwait, pop, run"),
         ("job runner", "0x43c4370", "pass begin/commit\ninvoke, release"),
         ("signal", "0x7330", "ReleaseSemaphore\n(h, n, 0)")]
for i, (a, rva, body) in enumerate(steps):
    x0 = C1 + 26 + i * (CW + 40)
    box(x0, CY + 120, x0 + CW, CY + 272, ACC["hot"], (36, 27, 18))
    t((x0 + 18, CY + 134), a, "hl", ACC["hot"])
    t((x0 + 18, CY + 168), rva, "small", FG)
    for j, ln in enumerate(body.split("\n")):
        t((x0 + 18, CY + 200 + j * 24), ln, "tiny", DIM)
    if i < len(steps) - 1:
        d.line([(x0 + CW + 6, CY + 196), (x0 + CW + 32, CY + 196)], fill=ACC["hot"], width=3)
t((C1 + 26, CY + 288), "signal is the release half of the same protocol, not a sixth stage", "tiny", DIM)

# ---- limits
LY = 2116
t((C1, LY), "WHAT IS NOT KNOWN", "h")
y = LY + 56
for line in ("no PDB: names, types and class layout are inferred; vtable names are manual",
             "shaders are DXIL blobs — target WGSL through SPIR-V + naga, never a hand guess",
             "package not finalised: no oracle run, so every claim stops at `ported`",
             "68 DLLs of platform surface must be shimmed before anything executes"):
    t((C1, y), "·", "body", ACC["hot"])
    para(C1 + 22, y, line, "body", FG, 2600)
    y += 42

# ---- footer
hr(2366)
t((130, 2392), "LEGEND", "hl", DIM)
x = 300
for label, col in (("code", ACC["code"]), ("ro", ACC["ro"]), ("rw / BSS", ACC["rw"]), ("metadata", ACC["meta"]),
                   ("relocation", ACC["reloc"]), ("shader", ACC["shader"]), ("claim", ACC["hot"])):
    d.rectangle([x, 2388, x + 32, 2408], fill=col)
    t((x + 42, 2386), label, "small", DIM)
    x += 210
para(130, 2448, "Measured 2026-11-26 from the image and its map alone: section table, LLVM import reader, "
                ".pdata extents, scan classes, call graph, claimed bodies. Sizes are virtual. No game bytes are "
                "stored in the repo.", "small", DIM, 2200)
t((W - 130, 2386), "reVI", "h", ACC["hot"], "ra")
t((W - 130, 2442), "structure of the shipped executable", "small", DIM, "ra")

img.save("/tmp/exe-structure.png")
print("ok", img.size)
