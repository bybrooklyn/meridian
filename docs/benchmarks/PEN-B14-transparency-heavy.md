# PEN-B14 — Transparency-Heavy Scene

[Suite](README.md) · [Penumbra](../../MERIDIAN_SPECOMENT.md)

version 1.0 · `DefinitionOnly` · `Uncalibrated`

Purpose: stress sorted/blended surfaces, particles, smoke, decals, alpha-tested
layers, local lights, shadows, depth interactions, and temporal reconstruction.
Independent sweeps vary layer count, coverage, material/light diversity,
particle count, and camera motion. The recipe fixes seed/counts/paths/hashes,
profile, warmup, and cache. Reports include complete shared fields plus overdraw,
sort/submit cost, blend artifacts, lighting correctness, temporal instability,
memory, and declared tier/fallback policy.
