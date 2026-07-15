# Benchmark Records

[Normative validation policy](../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)

This directory owns committed workload definitions and human-readable result summaries. Machine-readable results validate against `schemas/benchmark-result.schema.json` and record the complete EvidenceRecord context: source checkpoint, BuildId, corpus/artifact hashes, hardware/software, backend, profile/capabilities, cache/warmup state, samples/statistics, raw artifacts, thresholds, status, and known limits.

- [B01 — Midnight Forest Flashlight](B01-midnight-forest-flashlight.md) is the integrated opening traversal and recovery workload.
- [B02 — Field Horizon and Vegetation Stress](B02-field-horizon-sunset.md) is the generated/redacted rendering and streaming stress workload.

Neither workload is executable or calibrated yet. Their qualitative gates are provisional, and unsupported or occluded outcomes are never counted as visual passes.
