# What is possible before the package lands

Measured 2026-11-24: session download ~1.7 MB/s, 83 GB on disk of 224 GB, 97
peers connected, 52 at ≥99%. Transmission's own `Percent` and `Have` are broken
in this beta build, and neither `du` nor file length moves because the files are
preallocated and the bytes are filling holes — so progress has to be read from
the session counter, not the filesystem.

**ETA at the measured rate: about 23 hours** for the remaining ~141 GB, call it
one to two days with swarm variance. If the near-seeds thin out it is longer;
the tail of a 224 GB swarm is the slow part.

None of that blocks work. Ordered by value:

| # | Work | Needs package? | Est. | Why it matters |
|---|---|---|---|---|
| 1 | Continue the slot/semaphore thread: `0x569f0`'s callers, the remaining `g_slots` writers, the handler at `0x2a1be0`'s target chain | no | 2–4 h | completes a mechanism end to end |
| 2 | Stateful semaphore stubs (create/wait/release/close against a host table) so waits actually block and release | no | 2–3 h | turns 0-value stubs into a fake kernel; reusable in harness tests |
| 3 | Type reconstruction for what is already read: `g_slots` size and element type, the packed word in `0x37a50`, `CONTEXT` | no | 2 h | settles the open size question; feeds `DECOMP_SIZE_ASSERT`s |
| 4 | Harness code that needs no run: trap list parsing, vector schema + validator, replay binary against `revi call`, write-hash capture | no | 4–6 h | the harness's data path, unit-testable today |
| 5 | More argument-pure bodies from the queue (the 3,302 clean leaves) | no | ~5/h | volume where every result is provable now |
| 6 | Names: promote `proposed` → `confirmed` where a run backs it | no | ½ h | keeps the map honest |
| 7 | Loader hooks: a `trap` subcommand that patches an RVA with `int3` in a mapped image and reports the hit | no | 2 h | the trap half of the oracle without the game |
| 8 | First real oracle run, `matching` claims, subsystem integration | **yes** | — | everything 1–7 converges here |
| 9 | Boot attempt under WARP, then the failure list | **yes** | — | tells us which subsystem actually blocks a run |

## Rules while waiting

* Nothing in 1–7 may assume a game file. If it needs one, it waits.
* Every new stub that returns a handle or a state must have its failure branch
  exercised too (`0x01662430` shows why).
* Keep `revi check` green; the automerger stays on.

## What we are not doing

Chasing the 84 giants or the renderer before the oracle exists: they are where
plausible-but-unprovable work accumulates.
