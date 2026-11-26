# Strategy, draft 2 (after adversarial review)

## What the two reviews established

1. **No vector has ever executed our C.** `replay.py` runs `revi call`, which maps
   the original and calls the original bytes. `matching` today would be satisfied
   by running the exe three times. Everything downstream (traces, slice gates,
   coverage) hangs off a gate that does not exist.
2. **The vector data path is dead wherever a write exists**: `--dump` is a single
   value, `rva:`/`abs:` spans are unparsed, addresses are not rebased, `mem`
   preloads are applied by nothing, `--f64`/`--u32` are never forwarded, and
   `REVI_STUBS=0` is still truthy. The example vectors have no writes, so the gate
   never touched any of it.
3. **`ported` is hollow.** `build-src.sh` is not called with `--strict`, so the
   compile gate cannot fail; `check.rs` compares filenames only, so a claim with no
   `src/`, `notes/` or `tests/` passes; two claims in that state are `ported`; the
   entry point's body is `void entry(void) {}`; `tls_set` is `(void)` casts; one
   note documents 60% of a body that the source omits.
4. **"Callerless" is a graph artefact.** There are no indirect edges, so every
   vtable and table-dispatched target reads callerless — including the renderer and
   both recovered vtables. Deferring callerless work defers the class system.
5. **CI never runs any of this.** `ci.yml` is cargo on Windows only.
6. **The swap is a mini-linker.** An object's `.text` has zeroed operands for every
   `call rel32`, every `__imp_X`, every absolute. Nothing parses COFF relocations.
7. **Globals have their address only in a comment** (`extern char *g; /* 0x… */`),
   and file-scope `static` copies (e.g. `pool_up`) shadow the real word, so a
   second body sees a different flag.

## Order

## Gates, owners, stop-loss

One owner per gate, reviewer sign-off before the next gate starts. Each gate is
timeboxed; missing the box triggers the named rollback rather than extension.

### G0a — vector path correctness (3 days)

- `revi call --obj <obj>`: after mapping and relocating, copy the object's `.text`
  over the extent (pad to size, `mprotect` RW then RX) and call the patched copy.
  `--swap` becomes the replay default; `--original` keeps today's behaviour for
  comparison.
- One span parser shared by CLI and scripts: `buf`, `rva:0x…`, `abs:0x…` →
  resolved pointer. `--dump` repeatable. `--mem FILE` preloads spans. Forward
  `--f64`/`--u32`. `var_os` for the stub/TEB variables.
- Vector schema v2: `stub` table (symbol → return), `map` mode, `mem` spans,
  `branches` bitmask; `stubs: false` must actually disable stubs.
- Dump a page-rounded *superset* of every declared write and hash the superset, so
  over-writes cannot hide; fix `--u32` writing 8 bytes for a 4-byte word.
- Smoke vectors move onto claimed bodies that have sources; replay **hard-fails**
  on `--swap` when an object is missing (no silent fallback to the original).
- `check-all`: `--strict`, require `src`+`notes` for `ported`, require a stored
  `revi diff` score and `covered: n/N`.
- Committed sabotage fixture: right body passes, one-wrong-store body fails, empty
  body fails.

Gate: the three sabotage cases behave; a claim with no source fails the checker.

Rollback: none needed, this is all local.

### G0b — swap (5 days, stop-loss 1 day of measurement first)

- `revi call --obj`: parse the object's COFF relocations, whitelist
  `REL32/ADDR32/ADDR32NB/SECREL/SECREL7`, resolve `sub_<rva>` and `__imp_<iat>`
  against a generated symbol table, copy the patched `.text`, `mprotect`.
- Global rule, lint-enforced: a swapped body reaches a global either through an
  absolute constant or a resolver symbol; any object `.data`/`.bss` symbol that
  does not resolve refuses the swap.
- `--original` stays available for A/B.

Gate: `0x001117e0` passes on swap with bytes patched over the original (proved by
corrupting the original extent and still passing); a body calling an import and a
global (`0x01662430`) passes.

Rollback: if more than six reloc kinds appear, or resolution needs more than a
symbol table, abandon per-object swap and build a real link step
(`revi link` → stub lib → `--raw` image), keeping swap only for leaves.

### G1 — audit the 24 (4 days)

Re-read every claim for the three defects. Fill or demote (`ported` → `claimed`)
`0x056a1ec8`, `0x043c4140`, `0x043c38a0`, and any note whose source is shorter than
its body. Every claim that writes gets at least one write-bearing vector.

Gate: 24/24 either compile with `covered ≈ 100%` and pass their vectors, or are
demoted. Rollback: any body not fillable in two hours is demoted without debate
and returns through the normal queue.

### G2 — graph truth (5 days)

Add indirect edges: vtable slots (`vtables.jsonl`), pointer-table dispatch,
data-held code pointers. Recompute reachability. `callerless` may only be claimed
when no pointer path exists. Finish both known vtables' slot maps.

Gate: pointer edges move the reachable set by more than 10% and the added hot
bodies enter the queue; callerless is published with its caveat. Rollback: if the
move is under 10%, freeze the graph and proceed — do not gold-plate.

### G3 — loader and first real run (3 weeks)

The PE-under-ELF loader (fork issue 4): map, relocate, TLS, table-driven import
stubs, entry with a stop condition. Trap list from G2's hot set. First trace.

Gate: 5 minutes of execution, a captured observed-body list, no unknown-import
crash. Rollback: cut to "call the entry-adjacent set with a prepared state" and
validate by traps only.

### G4 — slice (per cluster, 1–2 weeks each)

Order: CRT init → TLS/slot pool → allocator → job queue → GPU submit/present →
input. Per cluster: traps, vectors, port with mutate-to-match, verify on `--swap`.
A cluster is done when its vectors pass and its notes stand alone.

Gate per cluster; slice coverage published, not function count over 299k.
Rollback: a cluster that cannot pass vectors in three days is split or deferred.

### G5 — data model, from G4 on, in parallel

Start with the renderer globals already found (`0x149c3d6*|7*`, `g_slots`,
`0x1471bfa10`). Recover base, size, writer set, and type; register vtables. Bodies
in the slice are read against these records.

## Do not

bulk-draft; port unreached code; grind codegen on rotate/float bodies before the
upstream fixes land; treat CRT, OS, GDK and third-party imports as one problem.
