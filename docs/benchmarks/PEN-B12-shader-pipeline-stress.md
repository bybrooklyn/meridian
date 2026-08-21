# PEN-B12 — Shader and Pipeline Compilation Stress

[Suite](README.md) · [Penumbra](../../MERIDIAN_SPECOMENT.md)

version 1.0 · `DefinitionOnly` · `Uncalibrated`

Purpose: stress `MaterialIr`/`ShaderIr` lowering, reflection/binding generation,
specialization, cache identity, backend output, pipeline creation, warmup, and
source-map diagnostics. Cold, warm, invalid-source, cache-corruption, and device-
recreation variants fix source/compiler/backend/capability/build hashes and
variant counts. Reports include complete shared fields plus compile/create
distributions, cache hits/misses/size, runtime creation, permutation inventory,
diagnostic mapping, recovery, and provenance. Shipping traversal permits no
undeclared runtime pipeline creation after its warmup contract.
