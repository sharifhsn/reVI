# The bytes with no record

`revi sweep` walks every gap between `.pdata` records in an executable section,
decodes it linearly and classifies it. Result for build 1.0.2.1:

| Class | Gaps | Bytes |
|---|---:|---:|
| `data` | 67,727 | 5,675,350 |
| `mixed` | 2,841 | 186,997 |
| `code` | 291 | 25,015 |

So 7.5 MB of `.text` outside the records is 95% data-in-code and alignment —
jump tables, literals and padding — and only **25 KB across 291 gaps looks like
clean code**. Those 291 are the ones worth reading; the rest is why `.text` is
99.9 MB while the records cover 92.4 MB.

Samples, all real and all without an unwind record:

```
0x003d1e0  mov qword ptr [rcx+74h], 0 ; ret                    a setter
0x00fbd10  mov rax,rdx ; dec r8d ; lea rdx,[table]            a table dispatch
0x001b6060 cmp dword ptr [rcx+48h], 0 ; jne … ; jmp …          a tail
0x00253fa0 mov eax,edx ; lea r8,[table] ; shl rax,5 ; vmovups  a vector copy
```

## How it classifies

`code` needs ≥95% of the bytes to decode, a terminator at the end, and a
prologue-shaped first instruction. `data` is under 60% decoding or a first
instruction no function starts with. `mixed` is everything between. The
classifier is deliberately conservative; a `data` verdict can still hide code.

## Use

`decomp/map/gaps.csv` (`rva,size,class,first`). A `code` gap is not a map row,
so claim it by RVA with its own size and name the gap in the note. Folding gaps
into `functions.csv` is a separate decision; it would stop the map from being a
pure `.pdata` reading.

## What the sweep does not do

* Follow jump tables. It decodes a gap linearly from its first byte, so a gap
  that starts mid-instruction (cold block entered by offset) classifies as
  `data`.
* Decide ownership. A `code` gap may belong to the function on either side; the
  call graph is the place to resolve that.
