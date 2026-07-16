# Isobar Weather and Atmosphere Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Rendering](RENDERING_AND_GRAPHICS_SPEC.md) · [Competitive quality](COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Normative Isobar architecture

Documentation maturity: `ResearchReady`. Implementation maturity: `Scaffold`.
Governing IDs: `REQ-ISO-001`, `REQ-ISO-002`, `WP-ISO-001`, `RG-ISO-001`,
and post-1.0 `PRG-REL-001`.

Isobar owns weather, wind, atmosphere, visibility, precipitation hooks, and
surface-environment state contracts. Current implementation status is Planned:
`engine/meridian_isobar` is a renamed empty scaffold, and no weather solver, wind field,
precipitation, wetness, cloud, or atmosphere system is implemented. Existing
renderer diffuse environment lighting is renderer evidence, not Isobar weather
evidence.

Schema/API blocks are planned contracts, not current runnable examples.

## 1. Scope

Owns:

- deterministic weather state, regional fields, wind, fog/visibility,
  precipitation hooks, atmosphere optical summaries, and surface wetness/snow
  interfaces;
- immutable snapshots consumed by rendering, vegetation, audio, particles,
  physics, and gameplay;
- weather document IDs, clocks, diagnostics, cache/provenance metadata, and
  fallback behavior.

Does not own:

- Alluvium recipe evaluation, authored climate/exposure field generation,
  generated identity, overrides, or baking;
- Penumbra render passes or GPU texture allocation;
- Basalt terrain source geometry;
- Torsant fire/fluid/thermal simulation;
- Project-specific narrative event enums.

## 2. Goals and non-goals

Goals:

- Provide shared path-independent weather snapshots instead of hidden
  per-system random weather.
- Separate authored transitions from hidden mutation.
- Keep regional weather slower than render frequency and interpolate for
  presentation.
- Persist authoritative coarse state and deterministic regeneration metadata.
- Support beginner presets and expert field debugging.
- Keep advanced atmosphere/weather simulation optional and measurable.

Non-goals for the opening slice:

- No planetary weather.
- No full CFD, engineering-grade atmosphere, or unbounded volumetrics.
- No runtime flooding, high-quality local fluids, volumetric fire, deep snow, or
  erosion; those belong to Torsant/Basalt research packs.
- No claim that Isobar is implemented from scaffold crates.

## 3. Status

| Area | Status | Evidence and limit |
|---|---|---|
| Weather crate | Scaffold only | Marker crate presence is not implementation evidence. |
| Wind snapshot | Planned | No runtime wind field or shared consumer path exists. |
| Fog/visibility | Planned | Renderer can later consume fields; no Isobar field generation exists. |
| Precipitation/wetness | Planned | Surface interface only; no rain, puddle, snow, or wetness simulation exists. |
| Atmosphere optical fields | Planned | No cloud/fog/rain projections implemented. |
| Opening hooks | Planned for MS-04/MS-05 | Basic weather/wind/fog hooks are future representative-forest work. |

## 4. Public contracts

```text
WeatherRegionId(u128 persistent_uuid)
WeatherSnapshotId { slot: u32, generation: u32, epoch: u64 }
FieldHandle<T> { slot: u32, generation: u32, level: u8, type_id: TypeId }

AtmosphereSnapshot {
  epoch: u64,
  pressure: FieldHandle<Scalar>,
  temperature: FieldHandle<Scalar>,
  humidity: FieldHandle<Scalar>,
  wind: FieldHandle<Vec3>,
  cloud_water: FieldHandle<Scalar>,
  precipitation: FieldHandle<Scalar>,
  visibility: FieldHandle<Scalar>,
  optical_summary: AtmosphereOpticalSummary,
}

WindSnapshot {
  epoch: u64,
  coarse_field: FieldHandle<Vec3>,
  gust_field: FieldHandle<Vec3>,
  local_overrides: [WindVolumeRef],
  query_lod_policy: WindLodPolicy,
}

SurfaceEnvironmentState {
  wetness: f16,
  water_depth_mm: f16,
  snow_depth_mm: f16,
  ice_fraction: f16,
  mud_saturation: f16,
  temperature_c: f16,
}

EnvironmentalTilePolicy {
  simulation_clock,
  presentation_clock,
  spatial_level,
  active_bounds,
  influence_horizon,
  work_quota,
  memory_quota,
  update_debt_limit,
  deterministic_envelope,
  downgrade,
}

SurfaceFluidHandoff {
  region_id,
  source_epoch,
  target_epoch,
  owner: IsobarCoarse | TorsantDynamic,
  initial_or_settled_state: SurfaceEnvironmentState,
  mass_or_depth_error,
  stale_after,
  fallback,
}
```

Hot data uses tiled SoA field pages. Cold data contains debug names, source
document provenance, author comments, palettes, and rejected candidate states.

## 5. Dependency direction

Allowed:

- Isobar may depend on core math/units, diagnostics, tasks, assets/world
  documents, saves, and package/cache contracts.
- Isobar may consume versioned Alluvium-authored weather profiles, exposure,
  shelter, climate, and weathering inputs without allowing Alluvium to advance
  live weather state.
- Renderer, audio, vegetation, particles, physics, and gameplay may depend on
  Isobar public snapshots/query APIs.
- Torsant may consume Isobar wind/moisture snapshots through declared latency.

Invalid:

- `meridian-isobar -> meridian-renderer` to allocate cloud textures.
- `meridian-audio` running separate hidden wind RNG for ambience.
- Project-specific story weather enums in Isobar core.
- Per-route hard-coded weather behavior inside Penumbra.
- Isobar runtime invoking Alluvium editor/compiler internals or writing recipe
  source during simulation.

## 6. Runtime pipeline

Opening-slice pipeline:

1. Read active weather state, authored forcing terms, deterministic seed, and
   accepted Alluvium source artifacts where present.
2. Advance low-frequency weather clock when due.
3. Generate coarse wind/fog/visibility state for active regions.
4. Apply terrain shelter, canopy attenuation, and authored gust volumes.
5. Publish immutable `WindSnapshot` and `AtmosphereSnapshot`.
6. Penumbra interpolates fog/visibility and optional rain/fog projections.
7. Vegetation samples wind LODs for trunk/branch/leaf/grass motion.
8. Audio derives ambience parameters from the same snapshots.
9. Diagnostics record update cost, query cost, active tiles, and fallbacks.

No subsystem may create a hidden per-frame feedback loop without declaring
stability, latency, and downgrade behavior.

### 6.1 Sparse, multirate scheduling

Isobar field pages are hierarchical, sparse where evidence justifies it, and
updated on typed simulation clocks separate from presentation. Every active
region declares spatial level, work and memory quota, update-debt limit,
influence horizon, deterministic envelope, and downgrade. Relevance combines
visibility, distance, predicted influence, gameplay importance, authored
priority, and state-change magnitude; camera visibility alone cannot discard an
offscreen storm that may affect gameplay.

Distant regions may use authored or analytic state, relevant regions coarse
tiled fields, and bounded local regions higher resolution. Penumbra interpolates
published state; it does not force Isobar to simulate at render frequency. GPU
acceleration is optional and must retain a CPU/simple fallback and measured
transfer/recovery policy.

### 6.2 Surface water authority and Torsant handoff

Without Torsant, Isobar remains authoritative for its coarse wetness and shallow
surface-environment state. When a region is promoted to dynamic fluid:

1. Isobar publishes an epoch-tagged initial state and stops advancing dynamic
   water for the handed-off region.
2. Torsant validates and accepts ownership through a typed barrier.
3. Torsant advances dynamic fluid while Isobar may continue non-conflicting
   atmosphere and precipitation source publication.
4. On demotion, Torsant publishes a settled summary, error/conservation report,
   and target epoch.
5. Isobar accepts the summary atomically or retains its last valid coarse state
   and reports the failed handoff.

Exactly one system advances dynamic water in a region and epoch. Eviction,
stale state, solver failure, package disablement, and save/load define explicit
fallbacks; no presentation transition may hide an ownership conflict or
gameplay discontinuity.

### 6.3 Shared media source publication

Isobar maps fog, cloud, precipitation projection, and atmosphere optical fields
into Penumbra's planned `ParticipatingMediaSourceSnapshot`. Isobar owns source
meaning and evolution; Penumbra owns GPU representation, lighting, temporal
history, and compositing. Isobar never allocates renderer volume hierarchies or
creates a separate raymarch path.

## 7. Capability tiers and disabled behavior

| Tier | Behavior |
|---|---|
| Disabled | No weather tasks, field pages, package chunks, renderer resources, or recurring queries. |
| Opening hooks | Deterministic state, wind snapshot, fog/visibility field, light-rain hooks, sparse wetness interface. |
| Regional fields | Tiled weather fields, transition graph, shelter masks, surface wetness, field debugger. |
| Advanced atmosphere | Volumetric fog/cloud/rain projections, measured against PEN-B05. |
| Research | Planetary or full local flow candidates with explicit corpus and owner. |

## 8. Diagnostics, recovery, and security

Diagnostics:

- active weather tiles;
- per-stage CPU/GPU cost where GPU compute is used;
- query cost by consumer;
- snapshot age and interpolation alpha;
- field memory by tier;
- downgrade reasons;
- determinism status;
- error/conservation estimates where meaningful;
- disabled-pack zero-work assertions.

Recovery:

- If tile update fails, keep last valid snapshot and report stale age.
- If cache is corrupt, invalidate affected regions and regenerate or fall back.
- If GPU compute fails, fall back to CPU/simple tier where available.
- If document migration fails, keep the original and emit a repair report.

Security:

- Weather imports are untrusted input.
- Optional atmosphere solvers cannot run external native/cloud compute without
  explicit capability grants.
- MCP commands must respect project permissions and redact local machine
  details unless explicitly exported.

## 9. Accessibility

Weather effects must expose photosensitivity, motion, contrast/readability,
audio cue, and subtitle/caption implications. Fog/rain presets need accessible
fallbacks that preserve gameplay route readability and do not hide recovery UI.

## 10. Tests, evidence, research, and delivery

Tests:

- deterministic seed replay under parallel scheduling;
- snapshot publication/reclamation and stale-reader safety;
- wind query LOD consistency;
- surface page serialization and migration;
- disabled-pack zero-work tests;
- cache corruption and recovery.
- sparse/multirate quota, influence-horizon, update-debt, and downgrade tests;
- `SurfaceFluidHandoff` promotion, single-owner, demotion, stale/failure,
  conservation/error, save/load, and disabled-Torsant tests;
- participating-media source epoch, bounds, budget, and absence tests.

Workloads: PEN-B01, PEN-B02, PEN-B05, PEN-B07, PEN-B10, PEN-B11, and PEN-B15.

Delivery: MS-04 defines the Isobar/Penumbra snapshot seam; `WP-PRC-004`
provides typed authoring inputs for environmental vegetation coupling; MS-05 requires basic
forest wind, visibility, fog, and weather evidence; MS-07 validates the opening
slice; MS-08 may expand regional fields and Torsant/Basalt coupling through
bounded packages. `RG-ISO-001` owns algorithm selection.

After MS-10, `PRG-REL-001` may competitively optimize the sparse/multirate,
surface-fluid-handoff, and shared-media contracts. It creates no pre-1.0
requirement, does not select an algorithm, and cannot promote Isobar maturity.

## 11. Adopted decisions

[ADR-0008](../docs/architecture/decisions/ADR-0008-isobar-basalt-torsant-boundaries.md)
owns subsystem boundaries; [ADR-0005](../docs/architecture/decisions/ADR-0005-shared-renderer-systems.md)
owns path-independent renderer consumption; [ADR-0026](../docs/architecture/decisions/ADR-0026-environmental-performance-contracts.md)
owns sparse/multirate, media, and water-handoff convergence. `RG-ISO-001` requires a future ADR
when it selects production algorithms.

## 12. End-to-end, failure, and performance-debug examples

End-to-end: an editor command changes an authored storm transition. Validation
checks units, bounds, seed policy, and required capabilities; the build creates
new field artifacts; Isobar publishes an epoch-tagged snapshot; Penumbra,
vegetation, and audio consume that same snapshot without owning weather state.

Failure/recovery: a regional field cache fails its hash check. Isobar discards
only that derived tile, retains source forcing and the last valid snapshot,
reports snapshot age, and regenerates or selects the documented simple tier.

Performance debug: PEN-B05 shows a GPU spike during a storm transition. The
trace separates field update, atmosphere projection, vegetation sampling, and
Penumbra volumetrics; a rerun with identical corpus, profile, warmup, and cache
state proves whether the selected downgrade removes the spike without changing
gameplay-visible wind authority.
