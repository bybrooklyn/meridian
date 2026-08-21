# PEN-B13 — Shadow-Heavy Scene

[Suite](README.md) · [Penumbra](../../MERIDIAN_SPECOMENT.md)

version 1.0 · `DefinitionOnly` · `Uncalibrated`

Purpose: isolate directional cascades and many local shadow casters across
static/dynamic update, cache/atlas policy, alpha-tested casters, large ranges,
and camera/light motion. The recipe fixes geometry/light/caster counts, paths,
hashes, quality profile, warmup, and cache. Reports include complete shared
fields plus shadow passes/draws/texels/memory, updates/cache hits, cluster
interaction, artifacts/leaks/acne/peter-panning, and lower-tier fallback.
