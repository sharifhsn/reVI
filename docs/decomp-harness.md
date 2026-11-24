# Oracle harness

Specification for the thing that turns "this C looks right" into evidence.
Not implemented yet; phase 1 is what implements it. Nothing here may depend on
game bytes being committed.

## Shape

A diagnostic build of the VIctory loader (`--profile diag --features diagnostics`,
plus `victory-decomp` hooks) exposes:

```
VICTORY_ORACLE=0x<rva>[,0x<rva>...]   arm these
VICTORY_ORACLE_MODE=trap|step|record   what to do when hit
VICTORY_ORACLE_OUT=<dir>               where the vectors go
```

On a hit: read the arguments from rcx/rdx/r8/r9 and the shadow space, dump the
extent's first 32 bytes of instructions as mnemonics, snapshot the memory the
function reads, single-step to the return, snapshot every range it wrote, and
take the hash of (return value, out-params, written ranges). Trap mode stops
the title, step mode continues, record mode writes a JSON vector.

## Vector format

```json
{
  "rva": 90840776,
  "build": "1.0.2.2", 
  "args": ["0x1", "0x7ff6..."],
  "returns": "0x0",
  "writes": [["0x2af0a10", "sha256:..."]],
  "caller": 90840896,
  "frame": 1421
}
```

No code bytes, no pointer values that only exist in one ASLR layout: store RVAs
for pointers into the image, and hashes for memory contents. A vector must
replay on a different machine, so anything layout-dependent is normalised.

## Two directions

1. **Record** from the untouched title, then run our C against the vector. The
   C side is a small harness binary that links the decompiled function, sets
   the same arguments against a scratch memory image, calls, and compares.
2. **Swap** the C in for the original at runtime (a trampoline at the RVA) and
   compare live. This is `linked` in `docs/decomp.md` and needs the function to
   be free of re-entrancy hazards first.

## Rejections, with reasons

* whole-frame traces — hundreds of MB, useless for isolating one function;
* screenshot diffing — dominated by timing and GPU nondeterminism;
* emulating the CPU — the point is that it is x86-64 and native;
* comparing against compiled C alone — proves our compiler agrees with itself.

## Trust

The harness is trusted when three contributors, on three machines, replay the
same vector and get the same result. Until then, only `ported` is honest and
`matching` means "my machine agrees".

## What already exists (2026-11-24)

The import and environment layers are built and proven in `revi call`, ahead of
the loader harness:

* `REVI_STUBS=1` fills every IAT slot with a recording trampoline that returns 0
  and logs `symbol, arg1, arg2, return-address-as-RVA`;
* `REVI_GS=1` fabricates a TEB (two indirections, slot byte at `+0x1294`);
* `RtlCaptureContext` writes the caller address into `CONTEXT.Rip`;
* `RtlLookupFunctionEntry` answers from the map by range and publishes the image
  base; the walk in `0x056006b4` now completes both iterations.

That is the harness's import, TEB and unwind halves minus the real APIs and the
live title. What remains is trapping inside the running game and replaying
vectors against it.
