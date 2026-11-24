# decomp/

Derived material for the source-level decompilation. Everything here is
allowed to be derived from the user's own copy of the title; nothing here is a
game file. See [docs/decomp.md](../docs/decomp.md) for the method and
[docs/asset-policy.md](../docs/asset-policy.md) for why this directory is
exempt from rule #1.

| Path | What | Hand-written |
|---|---|---|
| `map/functions.csv` | every function the compiler emitted: `rva,size,section` | no, `revi map` |
| `map/candidates.csv` | per-RVA class and cheapness score | no, `revi scan` |
| `QUEUE.md` | the cheapest 200 unclaimed functions | no, `revi scan` |
| `map/names.jsonl` | proposed name and signature per RVA | yes |
| `claims/0x<rva>.json` | who took an RVA, its status, its extent | yes, one file per function |
| `src/0x<rva>.c` | the decompiled body | yes |
| `notes/0x<rva>.md` | behaviour, callers, evidence, open questions | yes |
| `tests/0x<rva>.json` | input/output vectors recorded from a live run | yes, generated then trimmed |
| `build.json` | which build the map belongs to | once per build |

## The rules

1. **One RVA, one file, one owner.** A claim is a file named after the RVA, so
   two contributors cannot collide by editing a shared index.
2. **No RVA is decompiled twice.** Check `claims/` before you start; `revi
   claim` refuses a taken RVA unless the claim is `stale`.
3. **Status is evidence, not mood.** `claimed` → someone is on it; `ported` →
   the C compiles and matches the notes; `matching` → oracle traces pass
   (three runs); `stale` → untouched for 30 days and free again.
4. **Delete the claim and the files together.** An abandoned claim is worse
   than none.
5. **Know your extent.** A `.pdata` record can be a fragment of a function.
   `revi scan` merges fragments into bodies and says how many it joined; claim
   a split body with `--chunks N`, never a bare fragment.
6. **Aliases count once.** `revi scan` marks a body that already exists
   elsewhere as `alias` with its twin in `alias_of`; decompile the twin, claim
   the alias with a note pointing at it, do not duplicate the work.
5. **Additive only.** A PR usually adds whole files under `decomp/`. If it has
   to touch `Cargo.toml`, CI or `tools/`, a human reads it first.
