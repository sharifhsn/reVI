# Contributing

Two kinds of work, two paths. Read `docs/decomp.md` before the second one.

## Runtime work (shaders, loader, GPU, formats)

Goes to [VIctory](https://github.com/Marvisak/VIctory) and arrives here through
the `upstream` remote. Nothing about that flow changes: open it there, keep it
there. This repository only carries the decompilation and the tooling for it.

## Decompilation work

One function per pull request is normal and welcome. The unit of work is one
RVA, so nobody edits a shared file and PRs never conflict.

```sh
revi claim /tmp/revimap.jsonl decomp 0x056a1ec8 --owner yourhandle
# write decomp/notes/0x056a1ec8.md, decomp/src/0x056a1ec8.cpp,
#       decomp/tests/0x056a1ec8.json, decomp/traps/0x056a1ec8.jsonl
MAP=/tmp/revimap.jsonl EXE=$HOME/leonida/drop/exe/game_scarlett_master.exe tools/check-all.sh
git add decomp && git commit -m "decomp: 0x056a1ec8 <what it does>"
```

`tools/check-all.sh` runs, in order: the structural checker, trap validation,
vector validation and replay, then a compile of every body. A body that does not
compile alone is not done.

Rules the bot enforces, and a human does not have to:

1. only files under `decomp/` (a PR touching `Cargo.toml`, `tools/` or CI is
   queued for a human instead of auto-merged);
2. the RVA exists in `decomp/map/functions.csv` and the claim is yours;
3. the claim status matches what you actually did (`claimed` → `ported` →
   `matching`, and `matching` needs three recorded runs);
4. every file named `0x<8 hex>.c|md|json` has a claim, and every claim has a
   note;
5. files are text, small, and nothing carries game bytes.

Nothing else is reviewed for style. Decompiled C is ugly by nature; make it
readable, name locals, comment the conditions, and leave the original
instruction sequence findable in the notes.

### Claim etiquette

* Check `decomp/claims/` first. `revi claim` refuses a live claim.
* Claim the smallest useful extent. A `--size` smaller than the function is
  fine for a first slice; claim the rest separately later.
* A claim untouched for 30 days is fair game: set `status` to `stale`, or take
  it with `revi claim` (which allows stale).
* Abandoned work is worse than none. Close it, or say so in the notes.

### Notes first

`decomp/notes/0x<rva>.md` must stand alone: callers, argument meaning, branch
conditions, struct fields and constants, what you could not explain. If the C
ever has to come out of the tree (see the legal ladder in `docs/decomp.md`),
the notes are all that is left.

## What not to send

* the exe, DLLs, archives, extracted assets, decrypted anything;
* whole-frame dumps or screenshots (link, do not commit);
* a claim without a note;
* upstream runtime changes.

## Sign-off

Commits say `decomp: 0x<rva> <one line>`. If you want credit beyond the handle
in the claim, add a `Signed-off-by` trailer.
