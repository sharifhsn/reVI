# The exe-only oracle

`revi call <exe> <rva> [--arg …]` maps the image at its preferred base, applies
base relocations, allocates a stack and a scratch buffer, and calls the body.
No package, no loader, no init.

## What it is good for

Bodies whose behaviour is a function of their arguments: arithmetic, bit tests,
struct field access, small switch and table lookups. Those are the `leaf` class
the queue starts with, and for them this is a **real run**, not a reading — the
same evidence the game oracle would give, minus the scene.

Two vectors recorded this way already:

```
0x00b4a090  double 50.0 at +0x108, index 0 -> low 4 bytes  (0.0f)
                                 index 1 -> high 4 bytes (3.140625f)
0x001117e0  a = 0x16            -> al = 0, no call
            a = 0x00            -> al = 0 given the callee's result in this image
```

## What it is not

* **No init.** Globals are whatever the file says, usually zero. A body that
  dereferences a singleton faults: `0x01fa5270` does (`mov rax,[global]; call
  [rax+30h]`, null) and exits 139. That is a clean, useful failure.
* **No imports.** IAT slots point at nothing; calling one jumps to address 0.
* **No threads, heap or TLS.** A body that allocates will fault.

## Rules of use

1. A vector from this counts as an `exe_run` in `tests/`, never a `game_run`.
   `matching` still needs the live title.
2. Record the argument buffer layout in the note, not just the result.
3. A fault is information: it says the body needs state, which is a P2 fact.

## Safety

The image is mapped read-execute outside the Rust heap, and a bad body can
scribble anywhere. Run it in the same throwaway VM as everything else.
