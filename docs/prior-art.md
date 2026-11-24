# Prior art and watch list

Checked 2026-11-24. Nobody else is decompiling: every live effort runs or lifts
the binary. That is the premise of the repository, and it has to be re-checked
because it can stop being true.

## What exists

| Effort | Lane | Use to us |
|---|---|---|
| vice-ledger (`wer33321/vice-ledger`) | Ghidra ledger of names, signatures, structs, keyed by RVA, no bytes | potential name source; our `map/names.jsonl` uses the same key |
| `dariulone/gta6-vr-port` | IDA notes, renderer/camera/culling RVAs | renderer bodies, later |
| `Jake-Jensen/GTAVI_Xbox_ReverseEngineering` | import audit, PE inventory | import classification |
| VIctory / XCL / TuxBox / CGBJ | runtime, boots further every hour | correctness oracle for behaviour, and where our rebuild must eventually plug in |
| `Ahmet-Dedeler/awesome-gta6-reverse-engineering` | index | discovery |
| PortWatch VI | tracker, refreshed 3-hourly | discovery of new decomp efforts |

## Watch cadence

Every working session, before choosing targets:

1. `gh api "search/repositories?q=gta6+decomp"` and the same for `gta vi decompilation`, `scarlett decompile`;
2. `vice-ledger` commits (`gh api repos/wer33321/vice-ledger/commits?per_page=5`);
3. the PortWatch tracker `news` array for anything with "decomp" in it;
4. PortWatch's candidate list for bodies of work we have not seen.

If a decompilation appears, the first question is whether it publishes RVAs —
if it does, the two maps can be cross-referenced and names merged. If it
publishes only a tree, it competes; the tie-breaker is evidence per body.

## Findings this pass

* 0 repositories named or described as a GTA VI decompilation.
* PortWatch tracks 29 efforts; the two "decomp" hits are `ACreTeam/ac-decomp`
  (Army Men, unrelated) and a Ghidra skill repo.
* Newest progress is runtime-side: CGBJ reports in-game movement in an unlit
  world, XCL reports 30 fps through the menus, TuxBox drew the first screen.
  All of it widens the oracle surface, none of it overlaps the decomp.
