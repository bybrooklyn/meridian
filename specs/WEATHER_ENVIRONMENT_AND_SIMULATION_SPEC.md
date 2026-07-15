# Weather, Environment, and Simulation Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Runtime/tasks](CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md) · [Rendering](RENDERING_AND_GRAPHICS_SPEC.md) · [Audio/acoustics](AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) · [Procedural authoring](PROCEDURAL_AUTHORING_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Implementation phases](IMPLEMENTATION_PHASES.md)

Status: version 0.2 normative architecture for planned weather/environment/simulation work, 2026-07-14.

Current implementation status: `engine/meridian_weather`, `engine/meridian_terrain`, and `engine/meridian_vegetation` are scaffolds. Existing renderer code has diffuse environment lighting, but no weather solver, wind field, precipitation, wetness, fluid, fire, snow, erosion, or planetary system is implemented.

## 1. Context

Weather and environmental simulation serve the opening forest before they serve a broad sandbox. The first playable slice needs basic wind, fog/visibility data, weather state hooks, vegetation response, rain-ready flashlight/fog interfaces, and diagnostics. Expensive fluid, fire, snow, erosion, thermal, ecosystem, and planetary features are later optional packs with zero-cost-disabled behavior.

Version 0.1 established weather as globally simulated and locally modulated, with deterministic seeds, visible wind, rain, wetness, fog, and cloud ambitions. Version 0.2 refines that into tiled fields, immutable snapshots, explicit clocks, deterministic artist forcing, sparse surface states, and research gates for advanced coupled simulation.

## 2. Goals and Non-Goals

Goals:

- Provide shared immutable wind/environment snapshots consumed by vegetation, rendering, audio, particles, physics, and gameplay.
- Separate authored weather transitions from hidden random mutation.
- Keep regional weather slower than render frequency; interpolate fields for rendering.
- Persist authoritative coarse state and events while allowing deterministic regeneration of caches.
- Support beginner presets and expert field debugging.
- Make advanced coupled simulation optional and measurable.

Non-goals for the opening slice:

- No planetary weather.
- No full CFD, industrial fluid solver, or engineering-grade validation.
- No runtime flooding, high-quality local fluids, volumetric fire, deep snow simulation, or dynamic erosion.
- No claim that weather is physically validated beyond the specific fixtures that pass measured tests.
- No universal simulation graph that absorbs procedural authoring, rendering, audio, and gameplay logic.

## 3. Ownership and Crate Boundaries

Authoritative data:

- Weather source documents, climate regions, forcing volumes, wind presets, surface-environment maps, and event timelines are source-world documents.
- Runtime field tiles are immutable snapshots or caches.
- Render froxels, cloud textures, vegetation animation buffers, and audio ambience parameters are projections.
- Saves persist weather seed, active transition IDs, coarse field summaries, and gameplay-relevant surface state deltas.

Planned ownership:

| Area | Owner | Status | Notes |
|---|---|---|---|
| Weather documents and public IDs | `meridian-weather` | planned | Source schema and runtime handles. |
| Field service traits and descriptors | `meridian-core` or `meridian-weather` | planned | Must stay renderer/backend independent. |
| Wind snapshot generation | `meridian-weather` | planned | Shared by vegetation, audio, particles, gameplay. |
| Surface wetness/snow/mud fields | `meridian-weather` | planned | Sparse page storage, material coupling. |
| Terrain/vegetation consumers | `meridian-terrain`, `meridian-vegetation` | planned | Consume snapshots; do not own weather. |
| Render fog/cloud/rain integration | `meridian-renderer` | planned | Consumes optical/weather fields. |
| Advanced fire/fluid/snow/erosion packs | future simulation feature crates | research/planned | Optional, gated by benchmarks. |

Allowed dependencies:

- `meridian-weather` may depend on core math/units, diagnostics, tasks, assets/world documents, and save contracts.
- Renderer, audio, vegetation, physics, and gameplay may depend on weather public snapshot/query APIs.
- Weather must not depend on renderer, audio, game, editor UI, or physics internals.

Invalid dependencies:

- Invalid: `meridian-weather -> meridian-renderer` to allocate cloud textures.
- Invalid: `meridian-audio` running its own wind RNG separate from `WindSnapshot`.
- Invalid: Project-specific weather event enums in `meridian-weather`.

Dependency direction:

```text
source world/weather documents
  -> meridian-weather field compiler/runtime
  -> immutable WeatherSnapshot / WindSnapshot / SurfaceEnvironmentSnapshot
  -> renderer, vegetation, audio, particles, physics, gameplay consumers

advanced simulation packs
  -> field service contracts
  -> publish optional source/sink fields
```

## 4. Public Types and Data Structures

Rust-like schemas, not current implementation:

```text
struct WeatherRegionId(u128 persistent_uuid);
struct WeatherSnapshotId { slot: u32, generation: u32, epoch: u64 }
struct FieldHandle<T> { slot: u32, generation: u32, level: u8, _type: T }

struct FieldTile<T> {
    coord: TileCoord,
    level: u8,
    epoch: u64,
    resolution: UVec3,
    bounds: Aabb,
    unit: UnitId,
    values: Box<[T]>,
}

struct AtmosphereSnapshot {
    epoch: u64,
    pressure: FieldHandle<Scalar>,
    temperature: FieldHandle<Scalar>,
    humidity: FieldHandle<Scalar>,
    wind: FieldHandle<Vec3>,
    cloud_water: FieldHandle<Scalar>,
    precipitation: FieldHandle<Scalar>,
    visibility: FieldHandle<Scalar>,
}

struct WindSnapshot {
    epoch: u64,
    coarse_field: FieldHandle<Vec3>,
    gust_field: FieldHandle<Vec3>,
    local_overrides: Box<[WindVolumeRef]>,
    query_lod_policy: WindLodPolicy,
}

struct SurfaceEnvironmentState {
    wetness: f16,
    water_depth_mm: f16,
    snow_depth_mm: f16,
    ice_fraction: f16,
    mud_saturation: f16,
    temperature_c: f16,
}

struct SimulationTileReport {
    tile: TileCoord,
    tier: CapabilityTier,
    last_update_epoch: u64,
    estimated_error: Option<f32>,
    conservation_drift: Option<f32>,
    downgrade_reasons: Box<[DowngradeReason]>,
}
```

Hot data:

- active field values, interpolation endpoints, sparse page metadata, and query acceleration structures;
- stored SoA where consumers sample one attribute across many points;
- vector fields aligned for SIMD-friendly interpolation.

Cold data:

- debug names, source document provenance, author comments, rejected candidate weather states, editor visualization palettes.

Lifetime:

- Producers publish immutable snapshots with epochs.
- Consumers may retain snapshots for one frame/audio block/task epoch and must release before reclamation.
- Hot reload creates a new epoch; old snapshots remain until readers drain.
- Device loss only invalidates renderer projections, not authoritative weather snapshots.

## 5. Runtime Pipeline

Opening-slice wind/weather pipeline:

1. Read active weather state, authored forcing terms, and deterministic seed.
2. Advance low-frequency weather clock when due.
3. Generate coarse wind/fog/visibility state for active regions.
4. Apply terrain shelter, canopy attenuation, and authored gust volumes.
5. Publish immutable `WindSnapshot` and `AtmosphereSnapshot`.
6. Renderer interpolates fog/visibility and optional rain/fog projections.
7. Vegetation samples wind LODs for trunk/branch/leaf/grass motion.
8. Audio derives ambience parameters from the same wind/weather snapshots.
9. Diagnostics record update cost, query cost, active tiles, and fallback decisions.

Full planned weather pipeline:

```text
advance regional weather clock
-> exchange tile halos and dormant summaries
-> apply authored fronts, forcing volumes, and boundary conditions
-> update moisture, wind, cloud, precipitation, and visibility fields
-> schedule local refinement tiles only where importance/error requires
-> update sparse surface wetness/accumulation pages
-> publish immutable atmosphere, wind, and surface snapshots
-> render/audio/vegetation/physics consume snapshots at their own latency tolerance
-> record error, downgrade, and persistence summaries
```

Coupled simulation pipeline for optional packs:

```text
weather publishes wind/moisture snapshot N
-> fire/fluid/snow pack consumes N according to declared latency
-> pack publishes heat/smoke/water/snow source fields for epoch N+1
-> weather or renderer consumes source fields at next allowed coupling point
-> scheduler reports stability limits and skipped coupling if overloaded
```

No subsystem may create a hidden per-frame feedback loop without declaring stability, latency, and downgrade behavior.

## 6. Threading, Memory, and Lifetime

Latency classes:

- Player-proximal wind queries and render interpolation are frame-critical.
- Regional weather updates are asynchronous fixed-clock tasks.
- Surface page updates, precipitation accumulation, and cache compaction run in background tasks.
- Local fluid/fire/snow simulations may use GPU compute only when transfer and determinism costs are acceptable.

Core use:

- Small query kernels can run on performance cores when needed for frame deadlines.
- Regional coarse updates, import/bake, and dormant-tile processing can use efficiency cores.
- GPU compute is optional for dense fields and later simulation packs; CPU fallback must exist for validation or unsupported devices.

Synchronization:

- Field snapshots are immutable and epoch-tagged.
- Ordinary mutexes are acceptable for editor document mutation and offline bakes.
- Runtime readers use snapshot handles and avoid locking producer state.
- Cancellation is tile-granular; partially computed snapshots are not published.

Memory:

- Tiled fields use clipmap/sparse-page allocation, not one giant world array.
- Dormant regions store summaries and deterministic seeds.
- Surface state uses sparse pages or virtual-texture-like storage.
- Optional packs allocate no persistent field pages when disabled.

Determinism:

- Default weather is deterministic from seed, source docs, save state, and authored event inputs.
- Parallel execution must not change random streams.
- Artist overrides record whether they preserve deterministic replay.

## 7. Persistence, Versioning, and Compatibility

Source documents:

- Weather regions, climate presets, fronts, gust volumes, surface maps, and transition graphs are versioned source documents.
- Units and coordinate systems are explicit in every field descriptor.
- Unknown fields are preserved across editor migrations where supported.

Runtime saves:

- Persist seed, active transition, regional summaries, scheduled events, and gameplay-relevant surface pages.
- Do not persist every cloud voxel or render froxel.
- Local refinement tiles may be regenerated when deterministic and not gameplay-modified.

Built caches:

- Field caches include source hash, generator version, region bounds, resolution, quality tier, and platform where relevant.
- Cache corruption falls back to regeneration or simpler weather state; it does not corrupt source documents.

Compatibility:

- Unsupported advanced pack fields must be skipped with warnings when the project declares them optional.
- Shipping builds reject saves requiring missing mandatory simulation packs.

## 8. Editor, CLI, MCP, and Workflows

Beginner workflow:

1. Choose a weather preset such as calm midnight forest, light rain, fog, or distant storm.
2. Paint or place simple wind/fog/rain zones.
3. Press Play and preview vegetation, fog, ambience, and surface wetness.
4. Use readable warnings: "rain enabled but no shelter masks", "wind gust too strong for low preset", "feature pack disabled".

Expert workflow:

1. Inspect tiled pressure, humidity, wind, visibility, precipitation, and surface state fields.
2. Edit forcing volumes, boundary conditions, clocks, deterministic seeds, and LOD/error policies.
3. Compare algorithm tiers and visualize query latency, update cost, halo exchange, conservation drift, and downgrade reasons.
4. Run benchmark fixtures before accepting new simulation algorithms.

CLI commands, planned:

```text
meridian weather inspect <project> --region <id>
meridian weather bake <project> --region <id> --tier <tier>
meridian weather validate <project> --opening-forest
meridian weather diff-snapshots <a> <b>
meridian sim run-fixture <fixture> --algorithm <name>
```

MCP/agent surface:

- Agents may list weather documents, read diagnostics, propose presets/forcing changes, and run validation.
- Agents may not enable optional expensive packs or external compute/cloud services without explicit permission.
- Every agent edit creates an auditable project operation and recovery checkpoint.

## 9. Diagnostics, Failure Recovery, and Security

Diagnostics:

- Active weather/simulation tiles.
- Per-stage CPU/GPU cost.
- Query cost by consumer.
- Snapshot age and interpolation alpha.
- Field memory by tier.
- Downgrade reasons.
- Determinism status.
- Error/conservation estimates where meaningful.
- Disabled-pack zero-work assertions.

Failure recovery:

- If a tile update fails, keep the last valid snapshot and report stale age.
- If a cache is corrupt, invalidate only affected regions and regenerate or fall back.
- If GPU compute fails, fall back to CPU/simple tier when available.
- If a weather document migration fails, keep the original document and emit a repair report.
- If a simulation becomes unstable, disable that optional tile/pack and preserve source data.

Security:

- Weather/procedural imports are untrusted input.
- Optional simulation packs cannot run native external solvers or cloud compute without capability grants.
- MCP commands must respect project permissions and avoid leaking local paths or machine details in shared diagnostics unless explicitly exported.

## 10. Capability Tiers and Zero-Cost-Disabled Behavior

Baseline opening tier:

- Deterministic weather state.
- Shared wind snapshot.
- Basic fog/visibility fields.
- Optional light-rain hooks without full precipitation simulation.
- Sparse surface wetness interface where needed for materials/audio.
- Vegetation/audio/render consumers.

Intermediate tier:

- Regional tiled weather fields.
- Precipitation events, shelter masks, wetness, puddles.
- Volumetric clouds/fog projections.
- Weather transition graph and field debugger.

Advanced optional packs:

- Local fluid/flooding.
- Fire/smoke/thermal.
- Snow/granular deformation.
- Erosion authoring bake and limited runtime response.
- Ecosystem/vegetation lifecycle.
- Planetary or very large-world weather.

Zero-cost-disabled tests:

- Disabled fluids/fire/snow/erosion allocate no fields, schedule no tasks, create no GPU resources, and add no scene traversal.
- Standalone apps can omit terrain/weather packs entirely.
- Headless servers include only the weather authority required by their game mode.

## 11. Algorithm Alternatives and Research Gates

Weather:

- Rule-based fronts/cells: controllable and cheap; lower physical plausibility.
- Layered shallow-atmosphere approximation: better flow continuity; more stability and tuning work.
- Full local 3D flow: useful for hero effects; too expensive as default.
- Data-driven procedural fields: art-directable and stable; must avoid hidden nondeterminism.

Wind:

- Procedural noise plus authored gusts: opening baseline candidate.
- Terrain-adjusted field solve: better local plausibility, higher preprocessing/runtime cost.
- Local obstruction wakes: useful near buildings/vehicles, later tier.

Clouds/fog:

- Analytical height fog: cheap fallback.
- Froxel/volume fields: opening-friendly when bounded; can shimmer if temporal handling is poor.
- Full volumetric cloud raymarch: high visual quality, expensive and not mandatory for opening.

Fluids/fire/snow/erosion:

- Shallow-water heightfields versus simplified routing for flooding.
- Semi-Lagrangian versus MacCormack/BFECC advection for smoke.
- Surface fire graph versus local volumetric tile.
- Snow depth fields versus particle/MPM-like hero snow.
- Hydraulic erosion bake versus runtime incremental erosion.

Research gates:

- Phase 11 selects opening wind/fog/weather-field baseline using forest fixtures.
- Phase 21 prototypes fire/thermal and advanced vegetation coupling without production commitment.
- Phase 27 evaluates fluids, flooding, erosion, snow, and coupled simulation on shared fixtures with error, cost, determinism, and failure-mode reporting.

## 12. Tests, Benchmarks, and Acceptance Evidence

Tests:

- Deterministic seed replay under parallel scheduling.
- Snapshot publication/reclamation and stale-reader safety.
- Wind query LOD consistency.
- Surface page serialization and migration.
- Disabled-pack zero-work tests.
- Cache corruption and recovery.

Benchmarks:

- B01 midnight forest: wind, fog, vegetation, audio ambience, flashlight/rain-ready path.
- B02 field horizon: field grass wind, fog/visibility, streaming cells.
- Synthetic gust-front fixture: visible propagation through grass/canopy/audio.
- Optional research fixtures for flooding, fire/smoke, snow, erosion.

Acceptance evidence:

- Opening slice records shared wind snapshot consumed by vegetation and audio, not duplicated systems.
- Diagnostic report lists active tiles, update cost, memory, snapshot age, and fallback decisions.
- Recovery demo shows corrupt weather cache regeneration.
- Performance capture shows weather work bounded and optional packs absent when disabled.

## 13. Phased Implementation

- Phase 8: basic weather/wind/fog hooks for opening forest; no full weather system.
- Phase 11: weather fields, vegetation coupling, and procedural forest authoring integration.
- Phase 14: shared graph/compiler infrastructure for authored fields and partial regeneration, without a universal graph.
- Phase 21: advanced vegetation/fire/thermal research prototypes.
- Phase 27: advanced fluids, flooding, erosion, snow, and simulation coupling.

## 14. Examples

End-to-end opening example:

```text
Designer selects "calm midnight forest" weather preset.
-> Places two gust volumes near Zone C and field edge.
-> Build compiles source weather documents into region field caches.
-> Runtime publishes WindSnapshot epoch 42.
-> Vegetation bends canopy and grass from epoch 42.
-> Audio raises canopy bus and branch creak probability from the same epoch.
-> Renderer interpolates fog/visibility without running regional weather at frame rate.
```

Failure/recovery example:

```text
Compiled wind cache for forest cell B fails checksum.
-> Runtime invalidates that cache entry only.
-> Weather falls back to deterministic preset fields for cell B.
-> Editor diagnostic names source document, cache key, and rebuild command.
-> Rebuild produces a new cache without changing the source weather document.
```

Performance-debug example:

```text
Frame capture shows vegetation wind queries consuming too much CPU.
-> Weather diagnostic groups queries by consumer and LOD.
-> Expert view shows grass sampling high-resolution gust field beyond the near ring.
-> User lowers grass query tier for distant cells.
-> Validation confirms same WindSnapshot authority, lower query count, and no hidden second wind system.
```
