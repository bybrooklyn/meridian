# B02 — Field Horizon and Vegetation Stress

[Benchmark policy](../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md) · [Rendering](../../specs/RENDERING_AND_GRAPHICS_SPEC.md)

Status: version 0.2 workload definition; executable scene and calibration remain Planned.

## Purpose

B02 is a synthetic/redacted stress corpus derived from the proving game's field requirements. It isolates grass coverage, distant LOD, wind coherence, shadow policy, fog/atmosphere, fill rate, overdraw, visibility, streaming uploads, and temporal stability without publishing closed-source world layout.

## Workload dimensions

Run independent sweeps rather than one opaque maximum scene:

- vegetation instance and rasterized-triangle density;
- near/mid/far LOD distribution and transition rate;
- wind-field resolution, sampling tier, and synchronized consumer count;
- sun/shadow cascade coverage and update policy;
- fog/atmosphere quality and render resolution;
- camera speed, horizon exposure, and temporal history invalidation;
- cold and warm streaming/upload pressure;
- material, light, and pipeline diversity.

Each sweep has an immutable generated seed, source recipe, artifact hash, camera path, and expected object counts.

## Required measurements

- all common EvidenceRecord environment/build/corpus fields;
- frame and pass CPU/GPU distributions;
- memory, residency, upload, IO/decode/activation, and cache churn;
- logical/rasterized vegetation, visible objects/draws/triangles;
- LOD transitions, culled instances, shadow casters, cluster occupancy;
- overdraw/fill estimates and temporal rejection/history resets;
- wind queries by consumer and field tier;
- pipeline/shader creation and warmup status;
- visible captures plus LOD/overdraw/residency/debug overlays.

## Provisional acceptance

- no unreported runtime pipeline creation after warm-up;
- LOD transitions and temporal artifacts satisfy capture review at the tested tier;
- overdraw, streaming, and memory remain bounded by the calibrated profile;
- wind remains coherent across vegetation, audio, weather, and particles where enabled;
- lower tiers change content/algorithms intentionally rather than silently dropping correctness.

Initial calibration uses the exact M4 MacBook Air 16 GB record and the available main Windows/Linux-class PC. M1, Steam Deck, representative GPUs, XR, and server tiers remain later evidence rows.

## Evidence

Every sweep produces machine-readable metrics, raw traces, captures, source recipe and seed, hardware/profile record, and a human comparison. A result applies only to the measured dimension, corpus, hardware, and capability set.
