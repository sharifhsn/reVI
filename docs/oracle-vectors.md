# Oracle vectors and traps

Two files types, both per function, both checked by tools so a claim carries its
own expectations.

## Traps — `decomp/traps/0x<rva>.jsonl`

Where a run should stop or record. One JSON object per line:

```json
{"rva": 1120224, "kind": "entry", "mode": "trap", "note": "family byte 0x16 returns before the call; proved"}
{"rva": 1120224, "kind": "call", "mode": "record", "note": "other family bytes call 0x450879e0 with (a1)"}
```

* `kind`: `entry`, `call`, `store`, `branch`, `ret`, `fail`. A `fail` trap is a
  path that must be taken and needs `returns` with the expected value.
* `mode`: `trap` (stop the title), `step` (single-step to the return),
  `record` (write a vector, then continue).
* `at`: `entry` or `+0x<off>` inside the extent.

Validate with `tools/oracle/traps.py decomp/traps/*.jsonl`.

## Vectors — not committed, generated

A vector is one recorded execution, replayable on any machine. Schema:
`tools/oracle/vector.schema.json`. Rules the schema cannot state live in
`tools/oracle/validate.py`: the RVA must be a function extent, `rva:` spans must
sit inside the image, blobs are capped.

Record and replay:

```sh
# spec in, vectors out (deliberate re-baseline on a new build)
tools/oracle/replay.py spec.jsonl --exe <exe> --record
tools/oracle/validate.py spec.jsonl
# replay against our build
tools/oracle/replay.py spec.jsonl --exe <exe> --revi ~/revi-map-src/target/release/revi
```

Only hashes of writes are stored, never the contents; image bytes appear only in
`mem` spans where a body needs them preloaded, which is the one exemption.

## Why both

A trap with no vector says "this path matters". A vector with no trap has no
provenance. A claim earns `matching` only when its traps fire on a real run and
three machines replay its vectors to the same result — see
`docs/decomp-harness.md` for the trust rule.
