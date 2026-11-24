# Reverse-engineering tooling

What is installed, what each part is for, and the rule that keeps its output
honest.

## Ghidra headless

`tools/ghidra/decomp.sh <exe> <out.jsonl> [rva-list]` runs `analyzeHeadless`
with `ExportDecomp.java`, which decompiles functions to one JSON object per RVA.

* **Output goes to notes context, never to `src/`.** Its C is a draft: no real
  types, wrong struct widths, invented names, and it does not compile. It exists
  to remove the blank page, not to be committed.
* Analysis of 120 MB dominates the first run; the project is kept so later runs
  only decompile.
* Ghidra 12.2 needs **JDK 25** (`/usr/lib/jvm/java-25-openjdk-amd64`). JDK 21 is
  rejected as unsupported.
* Run it in the background and collect later; it is not interactive.

## Idiom detection

`revi scan` now labels whole classes of body by what they compile from, so
families surface as families instead of being found by hand:

| Signal | Pattern | Count |
|---|---|---:|
| `tls` | `gs:0x58` thread-slot read | 7,243 |
| `trylock` | `lock cmpxchg` | 3,336 |
| `atomic` | `lock xadd` | 119 |
| `fastfail` | `int 0x29` | 80 |
| `wait` | calls `Wait*` | 16 |
| `close` | calls `CloseHandle` | 10 |
| `semaphore` | calls `CreateSemaphore*` | 6 |
| `unwind` | calls `Rtl*` | 5 |

These are hints, not verdicts: `tls` alone says the body touches a thread slot,
not that it is interesting. But the semaphore chain that took three sessions to
trace by hand would have been a `semaphore`/`wait`/`close` triangle from the
start.

## Also in use

* `objdump` for disassembly and the diff.
* `revi diff` for the compile-and-compare loop, `revi call` for execution.
* `decomp.me` has no MSVC x64 preset, so it is not used; `objdiff` is the plan
  for larger diff runs.

## Not used, and why

* IDA + classinformer: licence cost, and Ghidra covers the decompiler need.
* `recompiled`/`reccmp`: assumes 32-bit pointers.
* `m2c`: MIPS/PPC/ARM only.
