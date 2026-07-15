# PEN-B05 — Heavy Isobar Storm

[Suite](README.md) · [Isobar](../../specs/ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md)

version 0.5 · `DefinitionOnly` · `Uncalibrated`

Purpose: isolate wind-field updates/queries, heavy rain hooks, fog/visibility,
volumetric layers, wetness interfaces, vegetation/audio coupling, lighting,
temporal stability, and downgrade/recovery. Deterministic sweeps vary field
resolution, active tiles, precipitation/volumetric tier, consumers, and camera
motion. The recipe fixes state graph, forcing, seed, counts, path, hashes,
profile, warmup, and cache. Reports include complete shared fields plus field
age/memory, query cost, coupling latency, stale/downgrade reason, and disabled
cost. No production threshold exists before Isobar implementation/calibration.
