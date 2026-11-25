# Decompilation plan

Written 2026-11-24, after the map (299,257 extents) and the gate existed.
Order is deliberate: shrink the problem, then prove a loop on the cheapest
functions, then use the loop on a subsystem, and only then attempt a rebuild.

## What the research says

1. **The precedent that fits is LEGO Island, not Mario 64.** LEGO Island is a
   Windows x86 MSVC target decompiled to source, function-annotated, scored by
   `reccmp`, and eventually reproduced byte for byte with the retail compiler
   under `ReproBit`. `reccmp`'s annotation scheme (`// FUNCTION: BINARY 0x...`,
   `SIZE`, `VTABLE`) and its accuracy report are the model:
   <https://github.com/isledecomp/reccmp>, <https://github.com/isledecomp/reprobit>.
   `reccmp` is written for 32-bit old MSVC (x64 is an open issue), so we borrow
   the method and the annotations, not the binary.
2. **Byte-matching is a late, optional signal.** Console decomp projects
   (`sm64`, `zeldaret`) match because they can pin the exact compiler and the
   code is small. Ours is 92 MB of MSVC x64 with no PDB; forcing byte identity
   before anything runs would stall. Behaviour equivalence against the live
   title is the bar (`docs/decomp-harness.md`); codegen identity is a score.
3. **Assembler diffing already exists for x86-64.** `objdiff` carries an
   `ArchX86_64` with `iced-x86` decoding and relocation-aware diffing:
   <https://github.com/encounter/objdiff>. `decomp.me` has no MSVC x64 preset
   (its `win32` platform stops at msvc6.x), so the loop is local:
   `msvc-wine` + `objdiff` + our oracle.
4. **The retail toolchain is obtainable.** `msvc-wine` installs VS2022 and the
   SDK on Linux through Wine: <https://github.com/mstorsjo/msvc-wine>. Same
   version family, same flags, and `/O2 /GS /guard:cf` are all visible in the
   headers already.
5. **Decompiler output is a skeleton, not a result.** kuna's decompiler plus
   its MSVC class pass (`rtti`/`kuna_msvcclasses`, RTTI-gated) produces
   compilable-shaped C++ and names; `m2c` and `decomp-permuter` fill gaps and
   grind the last instructions. Humans own the structure.
6. **Legal position is better than folklore.** Sega v. Accolade (1992) and
   Sony v. Connectix (2000) protect intermediate copying for interoperability;
   Google v. Oracle (2021) protects reimplementation. Take-Two's 2021 action
   against re3/reVC followed a DMCA takedown that was **restored in June 2021
   after a counter-notice**, and the suit has produced no judgment. Counter-
   notice is therefore a known-working step, not a bluff.

## Phase 0 — shrink the problem (this week)

299,257 functions is not a queue. The scanner (`revi scan`) sorts the map into
classes, none of which need a human:

| Class | What | Roughly |
|---|---|---|
| `thunk` | `jmp [rip+x]` import thunks, `jmp rel32` tail-only stubs | tens of thousands |
| `pad` | `int3`/`nop`/zero runs, < 4 bytes | thousands |
| `dup` | identical normalised bodies (`/OPT:ICF`, template and accessor clones) | thousands |
| `giant` | > 16 KB, generated (script VM, big switches) | 6 |
| `leaf` | no calls, small, string-free | the first real queue |
| `scored` | everything else, ranked by strings, imports, size | the long tail |

**Extents are chunks, not always functions.** MSVC splits bodies around EH
functlets and cold parts, and each piece gets its own unwind record. `revi scan`
joins a piece to the next unless it ends in `ret`/`jmp`/`jmp [rip+x]`, marks the
result `split` with a chunk count, and `revi claim --chunks N` pins that span so
`revi check` can validate it. 3,204 bodies are still split after the join, and a
split body is harder than a whole one, so they sort lower.

Deliverables: `decomp/map/candidates.csv`, generated `decomp/QUEUE.md`, and a
count per class in `STATUS.md`. The classification is also a public good; other
projects can read it.

## Phase 1 — prove the loop

1. `msvc-wine` on the lab box; versions pinned in `docs/decomp-toolchain.md`
   (19.52.37029 / SDK 10.0.26100 / /O2 /GS-).
2. `revi diff <rva>`: compile our file to a COFF object, diff against the
   original extent with `objdiff` semantics, and score it. No GUI.
3. Oracle harness per `docs/decomp-harness.md` — traps, vectors, replay.
4. Fifty `leaf` functions, each with notes, C, vectors, and three runs.
5. Run the toolchain twice on one function with a different flag set and record
   which set matches; that flag set becomes the project default.

Gate: 50 functions where our object and the original agree on every vector.
Until then nobody claims `matching`.

## Phase 2 — a subsystem, in dependency order

Strings first (they name things), then RTTI and vtables (they give classes and
virtual order), then the subsystem itself. Order of subsystems: file/streaming,
allocator, script VM, renderer. EH funclets (`__CxxFrameHandler4`, `.xdata`
scope tables) are a shape, not judgement work: template them once.

## Phase 3 — rebuild

Object files link against the shims, the title runs with the original `.text`
absent, and `revi` reports coverage. Byte-identical reproduction is attempted
per function only where the permuter gets there cheaply; a mismatch is a
comment, not a blocker.

## Easy wins, ranked

1. **Duplicates.** Hash the normalised instruction stream; alias the copies.
   One body can retire dozens of extents. `revi scan` does this first.
2. **Thunks and pads.** Mechanical, and they bulk out any honest percentage.
3. **RTTI harvest.** `_RTTICompleteObjectLocator` in `.rdata` yields class
   names, hierarchy and vtable order without reading a single instruction.
4. **Vtable membership.** A function address in a vtable is a virtual function;
   its slot number is its role. Free structure.
5. **String xrefs.** `lea rcx,[rip+lit]` before a call names the function
   (`"Failed to load %s"`), and `.rdata` literals are a to-do list.
6. **Import adjacency.** A function that calls `xmem`/`dstorage_xs` once is a
   wrapper and its purpose is the import.
7. **Signature clusters** (templates). Same normalised body, different types:
   one template, N instantiations.
8. **The giants.** Recognise the pattern (dispatch tables) once and generate.
9. **RAGE 8→9 carry-over.** RAGE 9 descends from GTA V's engine; public modding
   symbol maps let similarly-shaped functions be named from their GTA V
   counterparts. Names are facts, not code.

## Non-goals

* Byte-identical rebuild as a phase gate.
* Decompiling CRT and third-party middleware beyond identifying it.
* Decompiling assets or script bytecode.
* Chasing the renderer before the loader runs our code.
