# PEN-B02 — Dense Grass Field

[Suite](README.md) · [Validation](../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md) · [Vegetation](../../specs/VEGETATION_ECOSYSTEM_SPEC.md)

version 0.5 · Status `DefinitionOnly` · Calibration `Uncalibrated`

## Purpose

Isolate dense grass and distant vegetation across coverage, logical/rasterized
instance count, triangle/meshlet policy, near/mid/far LOD distribution,
transition rate, alpha overdraw, fill, shadow cascades/casters, Isobar wind,
fog/atmosphere, streaming uploads, origin precision, memory, and temporal
stability.

## Recipe contract

Independent deterministic Alluvium sweeps vary one dimension at a time. Every sweep
records recipe/version/provenance hashes, determinism level, evaluation mode,
seed, source/build/artifact hashes, camera path, expected counts,
capability/profile, resolution/upscaler, warmup/cache, and debug-overlay state.
The corpus is generated and contains no private world layout or assets.

## Required evidence

Use the complete report contract in
[`workloads.json`](../../specs/registry/workloads.json). Required overlays include
LOD, overdraw, shadow residency/update, geometry/texture residency, temporal
rejection/history reset, wind-field tier, and pipeline state.

Acceptance thresholds remain provisional until calibration. Lower tiers must
change content/algorithms intentionally and preserve declared correctness rather
than silently omitting required state.
