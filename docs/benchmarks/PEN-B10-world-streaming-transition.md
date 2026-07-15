# PEN-B10 — Large World-Streaming Transition

[Suite](README.md) · [Data formats](../../specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md)

version 0.5 · `DefinitionOnly` · `Uncalibrated`

Purpose: isolate a deterministic multi-cell/room indoor-outdoor transition with
asset, geometry, material, light, audio, collision, and gameplay facets. Cold,
warm, cancellation, corruption, and memory-pressure variants fix source/build/
package hashes, cell/room graph, expected dependencies, path, profile, warmup,
and cache. Reports include complete shared fields plus IO/decode/upload/
activation distributions and bytes, request reasons/deadlines, residency,
fallbacks, source-to-artifact trace, and retained authority after failure.
