# PEN-B11 — Low-Memory Stress

[Suite](README.md) · [Validation](../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · `DefinitionOnly` · `Uncalibrated`

Purpose: apply reproducible memory pressure to unified and discrete-memory
profiles while traversing representative scenes. Variants isolate texture,
geometry, temporal, shadow, pipeline, field, staging, and CPU-cache pressure.
The recipe fixes pressure schedule, required/optional assets, path, hashes,
profile, warmup, and cache. Reports include complete shared fields plus domain
budgets/peaks, allocation failures, eviction order, churn, stalls, quality
downgrades, required-data preservation, recovery, and post-pressure stability.
