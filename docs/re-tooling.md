# Reverse-engineering tooling

What is installed, what each part is for, and the rule that keeps its output
honest.

## Drafting: kuna

The drafting decompiler is kuna (fork `sharifhsn/kuna`, Apache-2.0, Rust),
built from source on `aster`; see `docs/decomp-kuna.md` for the invocation, the
address convention and the map-to-boundaries conversion. Ghidra is retired.

* **Output goes to notes context, never to `src/`.** Its C is a draft: invented
  types, and it does not compile as-is. It exists to remove the blank page, not
  to be committed.
* Batch, do not loop: a single function reloads the 120 MB image per call.
* Feed it our `.pdata` extents with `--only-declared`; its own discovery
  over-reports by ~130 k starts.
* Its `warnings` array marks every place the printer rewrote control flow; read
  those bodies by hand, not from the draft.

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
