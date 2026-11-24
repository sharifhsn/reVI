# Toolchain

How to produce objects that can be compared with the retail binary. Set up on
the lab box (`aster`) on 2026-11-24; the licence was accepted by the owner.

## Installed

* `msvc-wine` (`mstorsjo/msvc-wine`, current main), Wine 10.0, `msitools`
* MSVC **19.52.37029** x64 (`~/msvc/bin/x64`)
* Windows SDK **10.0.26100.0**

Pinned here because a toolchain bump changes every function's bytes and
invalidates comparisons. The pin was tested against 19.30 and 19.36 on eight
bodies and they are byte-identical, see `docs/decomp-toolset-pin.md`:

```
msvc    19.52.37029
sdk     10.0.26100.0
wine    10.0
flags   /O2 /GS- (add /GS only when the function has a stack cookie in the original)
```

## Compiling

```sh
export BIN=$HOME/msvc/bin/x64
export PATH=$BIN:$PATH
. ~/msvc-wine/msvcenv-native.sh
cl /nologo /O2 /GS- /c decomp/src/0x<rva>.c
objdump -d 0x<rva>.obj
```

`cl` is a Wine wrapper; `msvcenv-native.sh` sets `INCLUDE`/`LIB` so the same
environment also drives `clang-cl` and `lld-link` if a native path is needed.

## `revi diff`

```sh
REVI_CC=~/srv/revi/bin/cc REVI_CFLAGS=/arch:AVX2 \
  revi diff <exe> decomp/map/functions.csv 0x<rva> decomp/src/0x<rva>.c
```

Compiles, normalises both sides (every hex displacement becomes `----`; they
are link-time in the original and zero in the object) and prints a
side-by-side. The score is the longest common subsequence of instruction text
over the longer side, so an insertion does not zero out the rest. 100% exits 0.

`REVI_CC` is the compiler wrapper (Wine + `msvcenv-native.sh`), `REVI_CFLAGS`
the per-call extras. The baseline flags cannot be changed per call: they are
the comparison contract.

## Comparing

Byte comparison is per function: dump the extent from the user's exe with
`revi map` RVAs, and diff it against the object's `.text`. `objdiff` gives
relocation-aware, normalised output and is the intended tool once several
functions are in flight; a one-liner suffices for single functions.

A mismatch is not a failure. Record the byte counts and the shape of the
difference in the note, as `notes/0x001117e0.md` does. Only an oracle run
promotes a claim to `matching`.

## Import calls

An unlinked object emits `call rel32`; the linked original emits
`call QWORD PTR [rip+iat]`. `revi diff` normalises `call [rip]` and a plain
`call` to the same token, so an import call does not read as a mismatch. It
does mean `revi call` cannot run such a body: the IAT is null.

## Known gaps

* `/GS` differs per function: the scans show stack cookies in some bodies.
  Compare with cookies present, not by stripping them.
* COMDAT folding (`/OPT:ICF`) and function-level linking (`/Gy`) affect symbol
  layout, not bytes; do not chase object-level symbol order.
* The retail minor is probably not the variable (19.30 = 19.36 = 19.52 on every
  test). Whole-program build (`/GL`) and PGO are the open suspects.
