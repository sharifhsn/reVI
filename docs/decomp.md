# Decompilation

Goal: a source-level, readable reconstruction of `game_scarlett_master.exe`
that rebuilds to a playable title without the original binary. Nobody has
started one. Every other effort either runs the binary (VIctory, XCL, TuxBox)
or lifts it to machine code (Leonida, Palmetto, Xbox2Windows).

Two things make it tractable and one makes it cheap to start:

* the console CPU is x86-64, so the source is ordinary MSVC C++ with no
  instruction-set rewrite between decompiled and shipping code;
* VIctory already boots the title to the world, so every function can be
  checked against the live game instead of against a guess.

## Legal posture

The spin-off ships derived material, which VIctory's asset policy previously
forbade; `decomp/` is now exempt (`xtask/src/policy.rs`, prefix exception) and
the rest of the repository is unchanged.

Three rungs, in order, so a threat costs days and not the project:

1. **Ship it.** Public decompiled C here, GPL-3.0-or-later, no game files, no
   assets, "you still need your own copy". This is the re3/reVC posture, and
   both of those only ever lost their hosting, never a judgment.
2. **Notes-only.** `notes/` is written so it stands alone: signatures, field
   layouts, branch conditions, constants, call graphs. If rung 1 is taken
   down, the tree is regenerated from notes by the maintainers (and by any
   agent reading them) and the repository closes again.
3. **Counter-notice.** GitHub's process lets a developer contest; the
   re3/reVC precedent is that this forces the claimant to sue. That is a
   decision for a human, not a bot.

We are a single-team, notes-first effort. We do not claim clean-room: the same
people read the binary and write the C. The notes layer exists so the work
survives a takedown, not to win an argument about provenance.

Non-negotiables: no exe, no DLLs, no archives, no extracted assets, no keys.
One exception, already in the policy: interop constants that cannot be derived
(BCPack tables) stay in `victory-formats`, untouched.

## Matching

"Decompiled" means the C in `src/0x<rva>.c` reproduces the original's
observable behaviour on the same inputs. Three levels, and a function only
claims the highest one it earns:

1. `ported` — compiles in the harness, reads like the original, matches the
   notes. No proof yet.
2. `matching` — passes three recorded oracle runs, byte-for-byte on the
   function's return value, out-parameters and every memory range it writes.
3. `linked` — the harness runs it in place of the original in a live session
   with no divergence, and the frame still comes out.

Byte-identical codegen is a separate, optional signal (same MSVC major, same
flags, `revi diffcodesize`), never the bar. Different valid codegen is fine.

## The oracle

VIctory is both the loader and the recorder. The harness runs the untouched
title under a build with `victory-loader` patched to trap at one RVA, dump
arguments, registers and the `RUNTIME_FUNCTION` extent, single-step out, and
hash everything the function touches. `tests/0x<rva>.json` stores the trimmed
vectors; they are small JSON, no code bytes.

Rejected during harness work, with reasons kept in `docs/decomp-harness.md`:
whole-frame traces (too big), screenshot diffing (too many false alarms),
emulator traces (no CPU emulation in this path at all).

## Work order

Leaf functions in the loader, allocator and math libraries first: no scene, no
GPU, no timing, so the harness is provable before it is trusted. Then the
subsystems in dependency order — files/streaming, script VM, renderer — with
the scene last, because it needs the GPU alive.

Phases:

| Phase | Out | Done when |
|---|---|---|
| 0 | map, claims, check, harness spec | `revi check` green on an empty tree, one function round-trips |
| 1 | 200 leaf functions matching | the harness is trusted: three independent contributors reproduce one run each |
| 2 | one subsystem matching | the subsystem links and a session survives it |
| 3 | rebuild | the title runs from our object files with the original exe absent |

## Parallelism and distribution

The unit of work is one RVA, one file set. Nothing shared is edited, so PRs do
not conflict and merges do not need ordering. Claims are files, so the only
coordination state is `decomp/claims/`, and it merges itself.

Three hosting rungs, matching the legal ones:

1. the self-hosted Forgejo (canonical, on the author's server);
2. mirrors (GitHub `sharifhsn/reVI`, and a bundle on archive.org);
3. any fork. Forks are not authoritative: releases, the map and `upstream`
   point at the canonical remote, and the map is regenerated from the hash in
   `build.json`, so a lost host is an inconvenience, not a restart.

Upstream VIctory stays a remote. Runtime work keeps flowing in; decomp work
lives here and reaches it as patches when it is ready, so neither project
blocks the other.

## What we will not do

* decompile assets, formats or script bytecode (other projects own those);
* take the original exe's bytes into the tree;
* accept a "matching" claim without runs;
* review runtime code in a decomp PR — that goes upstream.
