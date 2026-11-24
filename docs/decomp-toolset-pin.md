# Toolset pin: the result

Tested 2026-11-24. Question: do the residual byte differences come from the
compiler minor? Answer: **no, not on these bodies.** Three toolchains from
opposite ends of the range produce identical bytes.

## Method

Eight bodies with a source arrive from the first session. Each was compiled
with `/O2 /GS-` (plus `/arch:AVX2` for the AVX body) under the three
toolchains below and diffed against the original extent with `revi diff`;
`REVI_CC` pointed at a per-toolset wrapper.

| Toolchain | `0x1117e0` | `0x66b380` | `0xb4a090` | `0x1a34480` | `0x1fa5270` | `0x220ea90` | `0x5647190` | `0x56a1ec8` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| MSVC 19.30 (14.30) | 61 | 0 | 71 | 77 | 100 | 100 | 28 | 0 |
| MSVC 19.36 (14.36) | 61 | 0 | 71 | 77 | 100 | 100 | 28 | 0 |
| MSVC 19.52 (current) | 61 | 0 | 71 | 77 | 100 | 100 | 28 | 0 |

Byte-level spot check on the one body where the difference is only encoding
(`0xb4a090`), all three give:

```
c5fb1081080100008bc2c5fb11442408c5fa10448408c3
```

against the original

```
50c5fb10810801000089d0c5fb110424c5fa10448408c3
```

So `8B C2` for `mov eax,edx` and the `[rsp+8]` spill are stable across 19.30 →
19.52, as is the missing `push rax`.

## What that means

1. **Keep 19.52.** There is no evidence for an older pin, and the newest is
   better supported. `docs/decomp-toolchain.md` records the pin.
2. **The remaining gap is source shape or build mode**, not the minor:
   * whole-program (`/GL`, LTCG) and PGO change inlining and frame layout, and a
     retail build plausibly uses both;
   * the `push rax` + `[rsp]` form appears when MSVC wants an aligned spill, and
     six source shapes tried (temp array, union, member reference, `volatile`,
     two-float hoist, index mask) did not produce it.
3. **Byte-identity stays a score, not a gate.** Two of eight bodies are already
   at 100 across every toolset; the rest sit at 61–77 with equal instructions
   and different placement. That is the normal shape of a non-LTCG
   reconstruction and is enough for `matching`.

## Still open

* Try `/GL` + `/LTCG` on one body. If the whole-program flag alone closes
  `0xb4a090`, the build mode is the answer and it changes how the matching loop
  must run (per-translation-unit link, not per-object). Cost: half a day.
* Ask whether the retail build was PGO'd. A `.pgd`/`.pgc` would show in
  `decomp/map/strings.csv` if the names survived; they do not, so this needs a
  test, not a text search.
