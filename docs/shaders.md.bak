# Shaders

Target: **WGSL**, ultimately through WebGPU for the browser build. Naga can do the
last hop (SPIR-V → WGSL) and validates the result, so the shader work is not
blocked on hand-writing WGSL — but the *information* has to be captured rich
enough that the translation is a table lookup and not a guess.

## Pipeline

```
container in the image        DXBC (SM4/5 bytecode) or DXBC-wrapped DXIL
        │  extract by RVA, keep the exact bytes + sha256
        ▼
disassemble / reflect         dxc -dumpbin, spirv-cross --reflect (tables, cbuffers, bindings)
        │
        ▼
SPIR-V                        dxil-spirv (DXIL) or spirv-cross (SM4/SM5)
        │
        ▼
WGSL                          naga CLI (`naga input.spv out.wgsl`), then `naga` validation
```

Every stage keeps its artifact: `.dxbc`, `.spv`, `.wgsl`, `.reflect.jsonl`. Nothing
is discarded, because a later naga or wgpu revision may need a different path and
re-extraction from the image is the only reliable repair.

## What must be captured beyond the bytes

A shader without its pipeline state is unusable, so extraction records, per blob:

- the exact container and its part list (`DXIL` parts carry name and size);
- **root signature** — CBV/SRV/UAV/sampler table widths, root constants, static
  samplers, and the flags (deny/allow stages, IA, vertex/instance limits);
- **PSO description** like the title creates it: blend (per-RT, alpha-to-coverage,
  independent blend), rasterizer (fill, cull, depth clip, conservative, forced
  sample), depth-stencil (formats, ops, masks), sample count/quality, strip cut,
  node mask, stream-output strides, render target formats;
- **input layout** and the vertex stream strides for VS-stage pipelines;
- resource bind counts, and whether `SHADER` mode is ray-tracing/DXIL library;
- which RVA in the image holds the blob, so it can be re-found, and which call
  site built the PSO.

That is the interesting half of the work: the translation is mechanical, the
pipeline description is what makes it run.

## Where they live

**Not all in the exe.** The first scan found 11 DXBC containers in `.data`
(10 carrying `DXIL`) inside a single ~12 KB active region, which is a small DXIL
library rather than the whole shader set. Container hunting must therefore run
over the package too (archives, `.pkg` members), not just the executable, and the
same scanner is reused there by file path.

## Tooling

- `tools/shaders/scan.py` — byte scan for `DXBC`, reports RVA/section/size/hash
  and whether the blob is a DXIL container. Cheap and deliberately tolerant; a
  false positive costs a failed disassembly, a miss costs a shader.
- `docs/` note per shader family once extracted: reflection, pipeline state, and
  the WGSL with the naga validation result.

## Rules

1. WGSL is the checked-in form; `.dxbc`/`.spv` are build inputs.
2. Nothing ships that naga rejects.
3Parent. A shader that cannot be translated keeps its DXBC plus a note saying
   which construct blocked it — never a hand-guessed WGSL.
4. WebGPU limits (binding counts, uniform buffer sizes) are checked at extraction
   time, so an untranslatable shader is known before the port, not during it.
