# kuna as the draft decompiler

Ghidra is out. kuna (`Noelo-Lab/kuna`, Apache-2.0, Rust) is the drafting tool:
it is fast, agent-shaped (`--json`, `--option`, per-function CLI, an installable
skill), and it produces structure our hand loop does not (TEB typing, packed
constants, `LOCK`/`UNLOCK`, record layout synthesis).

Everything it emits is still a **draft**. A body only enters `decomp/src` after
it has been read against the disassembly and then against a run.

## Build

Use our fork (`sharifhsn/kuna`, branch `lain/kuna-completion`), on `aster`:

```
~/kuna-fork/decompiler/target/release/kuna     # built from source
~/kuna-specs/specs                             # compiled SLEIGH specs, pass with --sleighpath
```

The release binary at `~/kuna/bin/...` predates the fork's boundary, RVA-file,
PDB and class work; do not use it for new drafts.

## Addresses

Base is `0x140000000`; a canonical claim RVA `0x00037a50` becomes the VMA
`0x140037a50`. Names are `sub_<VMA>`. `--rva` does not exist yet, so the
addition is done in the wrapper, not by hand.

## Function boundaries

Never let kuna discover the inventory: it over-reports (~430 k starts here
against 299,257 real `.pdata` extents). Feed the map instead.

```
revi map --out /tmp/revimap.jsonl <exe>
# rva,size -> VMA range with an exclusive end
python3 - <<'PY'
out = open('/tmp/bounds.txt', 'w')
for i, line in enumerate(open('/tmp/revimap.jsonl')):
    if not i:
        continue
    r, s, _ = line.strip().split(',')
    b = 0x140000000 + int(r)
    out.write('0x%x-0x%x\n' % (b, b + int(s)))
PY
```

Then always `--only-declared`. A one-byte extent has no legal spelling today
(`end` must exceed `start`), so the 10 such extents in this image drop out.

## Drafting a batch

```
A=$(sed 's/^/--addr /' /tmp/rvas.txt | tr '\n' ' ')
kuna decompile-project <exe> -o /tmp/kexp --layout rva-file --only-declared \
  --define-function @/tmp/bounds.txt --max-fn-seconds 30 \
  --sleighpath ~/kuna-specs/specs $A
```

400 functions take ~80 s. Output is `funcs/0x<bucket>/0x<rva>.c` plus
`index.jsonl` (`rva`, `end_rva`, `c_file`, `c_offset`, `c_len`), so a claim RVA
maps to a file without slicing a 750 MB `.c`.

For one function, `kuna decompile <exe> 0x<vma> --addr` is enough; it costs
30–90 s because the image is reloaded, so batch rather than loop.

`--layout turso` plus `kuna query <db> --rva/--range` is the same data as a
database when a run needs random access after the fact.

## Reading drafts

Take the `warnings` array seriously: `return-dupe`, `early-return`,
`tailcallframe` and `int3-pad` mark where the printer rewrote control flow, and
those are exactly the places to check by hand.

Known weak spots, so check these before believing a body:

* rotate chains (`rol`/`ror` by a constant) collapse to a shift of the original,
  and a value returned only through a tail callee is typed `void`;
* a spill and indexed reload (`VMOVSS xmm0, [rsp + rax*4]`) loses the scalar
  type, the index parameter and the arithmetic;
* virtual dispatch prints as `(**(void (**)(void))(*a0 + 0x40))()` — no slot
  name, no `this`;
* class recovery (`kuna_msvcclasses`) is gated behind RTTI, so it is inert on
  `/GR-` images.

Upstream: issues 1968, 1969 (both weak spots above), 1970 (no-RTTI vftables),
1972 (outlining). 1971 (boundaries, per-RVA export, 300 k scale) is done and
verified on this image.

## Skill

`kuna install-skill --dir ~/.agents/skills` is already installed on `aster`.
Put the binary on `PATH` (`~/srv/revi/bin/kuna`) before a session that will use
it, or the skill's own examples fail.

## Fix loop

`tools/kuna-watch.sh [minutes]` watches `sharifhsn/kuna` main; when it moves it
rebuilds on aster and runs `tools/kuna-repro.sh`, which prints a PASS/PARTIAL/FAIL
verdict per open issue (1968, 1969, 1970, 1972) plus an export/query regression
check. It never closes an issue.

Fixes are accepted only when the verdict flips and the regression block stays
green; then the issue gets a comment with the new output and the weak-spot list
above is pruned.
