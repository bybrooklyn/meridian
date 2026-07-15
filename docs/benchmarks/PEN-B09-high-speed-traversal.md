# PEN-B09 — High-Speed Traversal

[Suite](README.md) · [Basalt](../../specs/BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md)

version 0.5 · `DefinitionOnly` · `Uncalibrated`

Purpose: stress world/asset/geometry prediction, IO/decode/upload/activation
deadlines, visibility/LOD transitions, temporal behavior, and frame pacing at
speeds above normal gameplay. The generated path fixes velocity/acceleration,
cell graph, asset families, expected deadlines/counts, hashes, profile, warmup,
and cache. Reports include complete shared fields plus request reasons, queue
depth, misses/cancellation, bytes/times per stage, residency, required-content
failures, hitches, and recovery. It is not a gameplay speed commitment.
