# Work plan

Ordered, with feasibility and duration. "Agent-hours" are focused hours of one
session like the ones so far; wall-clock is longer wherever a download, a human
decision or an upstream project is in the path.

Baseline throughput measured today: **10 bodies in about two focused hours**,
all ≤32 bytes, 2 of them instruction-identical. Small bodies will not stay this
cheap, but nothing below assumes better than 5 bodies/hour.

**State 2026-11-24: steps 1–5 are done.** The toolset question is answered
(`docs/decomp-toolset-pin.md`), the gap sweep and call graph are built
(`docs/decomp-gaps.md`, `decomp/map/graph/`), strings are catalogued, and
`revi diff` is the loop. Next is 6, the oracle harness, which needs the
package.

The plan is ordered so that each step removes a blocker for the next. Steps 1–7
are worth doing even if the effort stops after them, because each produces
something the field can use.

## The ladder

| # | Work | Why it is here | Feasibility | Agent-h | Wall |
|---|---|---|---|---|---|
| ✓1 | **Resolve the toolset** — try 19.3x/19.4x minors on the ten bodies already done, record which reproduces the encoding | Two functions differ only in `89 D0` vs `8B C2` and a spill slot; that is compiler, not source. Wrong pin poisons every later comparison | High. `msvc-wine` takes any installer manifest; the checker already diffs | 3–5 | 1 day |
| ✓2 | **Sweep the 7.5 MB with no `.pdata` record** — linear disassembly from known code, classify jump-target vs junk vs data-in-code | 7.5 MB is 7.5% of the section and invisible to the map. Some is leaf code reachable only by jump table | Medium. Needs a disassembler pass and a rule for "looks like data" (`iced-x86` + heuristics) | 8–12 | 2 days |
| ✓3 | **Call graph** — direct `E8` targets, import call sites, pointer-table edges, per-function in/out degree | Without callers, roles are guesses; with them, names and types follow. Also finds the true entry roots for 2 | High, mechanical. Risk is size, not difficulty: 299k nodes | 10–14 | 3 days |
| ✓4 | **Decode `.rdata` (H 7.95)** — identify the encoding over the known string pool | If it is compressed or keyed, the literal inventory is a fraction of the truth and naming stalls | Medium. Needs the scheme recognised; possibly RAGE's own packing | 6–10 | unknown tail |
| ✓5 | **`revi diff`** — compile a body, normalise, print a relocation-aware instruction diff with a score | The loop today is `objdump` by hand. This is the daily tool | High. `objdiff` already implements the diff core; ours is a CLI wrapper | 6–8 | 1–2 days |
| 6 | **Oracle harness v1** — trap at RVA, dump args/regs, single-step out, hash writes, replay | Nothing can be `matching` without it. It is also how the 50 leaves get proven | Medium-high. VIctory has a loader and diagnostics switches; this is loader surgery | 20–30 | 4–6 days, needs the package |
| 7 | **Fifty leaves end to end** — claim, read, port, diff, three runs each | Proves the whole loop at a size where mistakes are cheap | High once 1–6 exist | 15–20 | 3–4 days |
| 8 | **Type and frame reconstruction** — per function: stack frame, struct offsets, vtable slots, argument widths | This is where decompiled C becomes C++ that compiles into the same bytes. RTTI is stripped, so it is all derived | Medium. Slow, needs judgement; the call graph makes it tractable | 3–6 h per *class* of pattern, then per site | ongoing |
| 9 | **EH funclets and cold blocks** — template `__CxxFrameHandler4` shapes, teach the scanner to pair them | 3,204 bodies are split; they are unreadable until the pattern is known | Medium. A finite set of handler shapes | 15–25 | 4–5 days |
| 10 | **Allocator and file/streaming subsystem** — the first subsystem that must link | Dependency root: everything touches memory and archives. Proves P2's gate | Medium. Large but shallow; oracle covers most of it | 80–120 | 2–3 weeks |
| 11 | **Relink path** — objects link against the shims, oracle runs with the original code replaced per function | Turns "readable" into "runs our code" | Medium-high. Needs symbol/ABI discipline, not new science | 30–40 | 1 week |
| 12 | **Script VM and streaming graph** | Largest behavioural surface; needed for a session that does anything | Low-medium. Generated dispatch code, 84 giants | 120–200 | 3–4 weeks |
| 13 | **Renderer and GPU-visible paths** | Needed for a picture, last on purpose: it depends on 10–12 being solid | Low. Interacts with every earlier layer | 200–300 | 4–8 weeks |
| 14 | **Long tail** — everything the boot path does not reach | Can stay binary-only indefinitely; only worth it once the game boots from our objects | High per body, endless in aggregate | 0.2 h each | months |

## Sequence

```
week 1   [1]──[2]──[3]──[4]
              └──[5]──┐
week 2        [6]─────┴──[7]
week 3   [8][9]──────────────┐
week 4-6 [10]─────────────[11]
week 7-10 [12]──────────[13]
after     [14]
```

1–7 are strictly ordered. 8 and 9 interleave with 10 and have no hard
dependency on 11. 14 has no dependency on anything and can start any time.

## Feasibility, in one line each

* **1, 5:** easy, high value, do first.
* **2, 3, 4:** mechanical analysis, real but bounded work; 4 has an unknown tail.
* **6, 7:** the risky pair, because the harness touches the loader and needs the package. If 6 slips, 7 cannot start and everything downstream stops at `ported`.
* **8, 9:** judgement work, no unknowns, but they set the pace.
* **10–13:** the actual port. Estimates assume one focused session; a second contributor halves them roughly, not exactly.
* **14:** never finishes, never blocks.

## What each gate means

* after 1: comparisons are trustworthy — **reached, with a null result: the
  minor is not the variable, so byte-identity stays a score**
* after 3: roles are inferable — **reached: 825,041 edges, 82,223 reachable,
  queue now ranks by in-degree**
* after 5: the daily loop is one command
* after 7: `matching` is a claim we can defend
* after 9: split bodies are cheap
* after 10: one subsystem survives being ours
* after 11: the binary is no longer the only copy of its own code
* after 13: a rebuild is plausible

## Risks, with the counter

| Risk | Counter |
|---|---|
| Harness slips (6) | Build the diff tool (5) first and keep the 50 as `ported` until the oracle exists; publish the queue either way |
| Package never lands | Oracle vectors can come from a contributor's machine; the harness is the blocker, not our box |
| Compiler never pinned | Try older minors first (1); if none match, drop byte-identity to a score and lean on behaviour |
| `.rdata` scheme unknown | Name from strings only where decoded; the call graph still gives roles |
| Scope creep into 14 | Track `matching` on the boot path separately from total coverage |

## The number to watch

Not 299,257. It is **coverage of the boot path**: the fraction of functions the
title actually executes that have a `matching` body. The map can say how much
code exists; only the oracle can say how much of it matters.
