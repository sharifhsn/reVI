# Language and build contract

The title is MSVC C++. Decompiled bodies are **C++20**, compiled by the same
toolset family with RTTI off. Everything that compares runs through these
flags — they are measured as byte-neutral on the sample bodies, so they cost
nothing and stop stylistic drift.

```
/nologo /O2 /GS- /GR- /EHsc /std:c++20 /c
```

* `/GR-` because the binary's own RTTI is stripped; only `std::`'s remains.
  Declaring classes with RTTI would change the layout.
* `/EHsc` because the title is built with exceptions and funclets appear in the
  code. Bodies without `try` compile the same either way.
* `/GS` is absent by default: only bodies whose original has a cookie get it,
  added per call with `REVI_CFLAGS=/GS`.
* `/O2` is the level; `/O1` and `/Ox` produce identical code on the samples,
  `/Os` and `/Ot` do not.

## Files

* one body per file, `src/0x<8 hex>.cpp`, same RVA as claim, note and tests;
* `extern "C"` on the entry symbol so the linker name stays `sub_<rva>`;
* `sub_<rva>` is the **symbol**: mechanical, stable, greppable. Roles live in
  `map/names.jsonl` and in the notes, and are promoted here only with evidence;
* no `using namespace`, no iostream, no headers beyond `<stdint.h>`, `<stddef.h>`,
  `<string.h>`, `<intrin.h>`;
* reconstruct the original's types. If a member offset is known, say it in a
  comment (`/* 0x108 */`) and give the struct the right size.

## Names

`decomp/map/names.jsonl` holds proposed roles, one JSON object per line:
`rva`, `name`, `role`, `status`, `evidence`. `status` is `proposed` until a run
or a caller proves it, then `confirmed`. Symbols in `src/` stay `sub_<rva>`
until confirmed; the notes may use the role freely. Never write a role name into
a symbol on a hunch — a wrong name is worse than a number, because callers get
searched for.

## Why not C

Half the value is virtual calls, member layout and class identity, and the
original was C++. C also drifts: a `struct`-with-function-pointer-table version
of `0x01fa5270` costs four bytes more than the member-call spelling.

## Compiler

MSVC 19.52 through `msvc-wine`, pinned. 19.30 and 19.36 are byte-identical on
every sample (`docs/decomp-toolset-pin.md`), so the minor is not a variable; a
later one may be re-tested once, and only with evidence.

## Language level, measured

`/std:c++14` (default), `/std:c++20` and `/permissive-` produce identical bytes
on the sample. C++20 is chosen for source comfort, not for codegen.
