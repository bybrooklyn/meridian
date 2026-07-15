# B01 — Midnight Forest Flashlight

[Benchmark policy](../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md) · [Engine-facing slice](../../specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)

Status: version 0.2 workload definition; executable scene and calibration remain Planned.

## Purpose

B01 is the deterministic five-minute opening traversal workload. It exercises the Phase 8 integration boundary without publishing the full closed-source route or creative documents: dense forest visibility, flashlight and shadow stability, fog/readability, vegetation, world/asset streaming, grounded movement, simple interaction, spatial forest audio, basic wind/weather, save checkpoint, and title transition.

## Corpus contract

The private game repository owns the source route. The engine evidence bundle records only:

- an immutable source checkpoint and redacted corpus hash;
- a semantic camera/input recording with named capture markers;
- required world-cell and asset-family expectations;
- weather/wind/audio/logic event IDs needed for correlation;
- build profile, capability set, quality tier, and cache state.

The recording starts before first movement and ends after the title/return transition. Completion must remain possible without optional discoveries.

## Run variants

| Variant | Cache | Purpose |
|---|---|---|
| B01-COLD | empty derived/runtime caches allowed by the profile | import/package/stream startup and first traversal |
| B01-WARM | validated artifacts and warmed required pipelines | steady traversal and frame consistency |
| B01-RECOVERY | interrupted save at the declared checkpoint | transaction recovery and resumed traversal |
| B01-SCALE | each supported quality tier | algorithm/content scaling and readability |

## Required measurements

- hardware, OS, driver/runtime, backend, display mode, power/thermal state;
- source checkpoint, BuildId, artifact/package/corpus hashes;
- fixed-tick, presentation, input-to-frame, frame-time distribution, worst windows;
- pass-level CPU encoding and optional GPU duration with unsupported reason;
- queue submit/present/surface outcome and runtime pipeline creation;
- memory by domain, allocations, upload/staging, and peak;
- file IO, read/decode/upload/activation time and bytes by cell/asset;
- visible cells/objects, draws, triangles, vegetation, lights, shadow casters, overdraw estimate;
- Cairn/controller step, query, contact, and correction;
- audio callback/stream/voice/underrun metrics;
- weather/wind snapshot, script/state-flow, UI/semantics, and save transaction;
- captures at each named marker and known limitations.

## Provisional acceptance

- no required runtime pipeline creation after warm-up;
- no missing required asset, collision, or route cell;
- no save corruption after the induced interruption;
- no unsupported/occluded render outcome counted as visible-quality evidence;
- no unexplained traversal hitch or callback underrun;
- reduced-effects/accessibility settings preserve completion and readability.

Numeric gates remain provisional until the workload is executable and calibrated under the validation policy. Historical M1 and Steam Deck ideas are later comparison tiers, not initial Phase 8 certification.

## Evidence

Each run emits the versioned benchmark JSON plus raw trace, render captures, memory capture, audio report, save/recovery log, and a human summary. Before/after optimization claims use identical checkpoint, workload, hardware, profile, and statistical method.
