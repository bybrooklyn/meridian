# Penumbra Benchmark Definitions

version 0.5 · All workloads are `DefinitionOnly` / `Uncalibrated`

The machine-readable authority is
[`specs/registry/workloads.json`](../../specs/registry/workloads.json); the
[validation specification](../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)
owns evidence and report policy. Markdown files explain scenario intent. They
are not benchmark results.

| ID | Definition |
|---|---|
| PEN-B01 | [Midnight forest](PEN-B01-midnight-forest.md) |
| PEN-B02 | [Dense grass field](PEN-B02-dense-grass-field.md) |
| PEN-B03 | [Flashlight through alpha-tested foliage](PEN-B03-alpha-tested-foliage.md) |
| PEN-B04 | [Redacted AMI interior](PEN-B04-redacted-ami-interior.md) |
| PEN-B05 | [Heavy Isobar storm](PEN-B05-heavy-isobar-storm.md) |
| PEN-B06 | [Large Basalt terrain vista](PEN-B06-basalt-terrain-vista.md) |
| PEN-B07 | [Torsant coupled simulation](PEN-B07-torsant-coupled-simulation.md) |
| PEN-B08 | [Rapid camera rotation](PEN-B08-rapid-camera-rotation.md) |
| PEN-B09 | [High-speed traversal](PEN-B09-high-speed-traversal.md) |
| PEN-B10 | [World-streaming transition](PEN-B10-world-streaming-transition.md) |
| PEN-B11 | [Low-memory stress](PEN-B11-low-memory-stress.md) |
| PEN-B12 | [Shader/pipeline stress](PEN-B12-shader-pipeline-stress.md) |
| PEN-B13 | [Shadow-heavy scene](PEN-B13-shadow-heavy.md) |
| PEN-B14 | [Transparency-heavy scene](PEN-B14-transparency-heavy.md) |
| PEN-B15 | [Temporal disocclusion](PEN-B15-temporal-disocclusion.md) |
| PEN-B16 | [VR-oriented stereo](PEN-B16-vr-stereo.md) |

Results are separate typed records. Before/after comparisons require identical
checkpoint, corpus, hardware, capability profile, settings, cache/warmup, and
statistical method.

The v0.5 report contract also records Alluvium recipe hashes/version,
determinism level, evaluation mode, and provenance-manifest hash. Workloads not
using Alluvium record explicit `NotApplicable` values. The new fields do not
change any workload's `DefinitionOnly` / `Uncalibrated` status.
