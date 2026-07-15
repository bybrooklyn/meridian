# Torsant Fire, Fluids, and Thermal Simulation Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Rendering](RENDERING_AND_GRAPHICS_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Normative Torsant architecture

Documentation maturity: `ResearchReady`. Implementation maturity: `Research`.
Governing IDs: `REQ-TOR-001`, `WP-TOR-001`, `RG-TOR-001`. No Torsant crate is
created until its first real implementation package starts.

Torsant owns optional fire, fluids, smoke, heat, thermal material state, and
coupled simulation contracts. Current implementation status is Planned/Research:
no Torsant solver, fire, fluid, smoke, heat, thermal propagation, flooding,
snow/granular, erosion, or renderer integration is implemented. Torsant must add
zero work when disabled.

Schema/API blocks are planned contracts, not current runnable examples.

## 1. Scope

Owns:

- optional fire, smoke, fluid, water, wetness coupling, thermal material, heat
  source/sink, and coupled-solver snapshots;
- solver diagnostics, stability limits, downgrade behavior, research fixtures,
  and disabled-pack proof;
- declared coupling with Isobar wind/moisture and Basalt terrain/surface data.

Does not own:

- Alluvium-authored initial-condition recipes, source-field generation,
  generated identity, overrides, or authoring-time bake orchestration;
- Isobar regional weather authority;
- Basalt terrain source geometry;
- Penumbra render graph or shader ownership;
- production algorithm selection before research gates.

## 2. Goals and non-goals

Goals:

- Keep fire/fluid/thermal systems optional and zero-cost when disabled.
- Use shared field/snapshot contracts rather than a universal simulation graph.
- Support authored, approximate, and research solvers behind stable seams.
- Declare coupling latency and stability before exchanging source fields.
- Provide recovery and downgrade behavior for unstable or unsupported solvers.

Non-goals:

- No opening-slice dependency on Torsant.
- No full CFD, engineering fire model, flood simulator, or production thermal
  solver without `RG-TOR-001` evidence.
- No external native/cloud solver without explicit capability and provenance.
- No claim that planned contracts are implemented.

## 3. Status

| Area | Status | Evidence and limit |
|---|---|---|
| Fire/smoke | Research/planned | No solver, source field, renderer integration, or fixture implementation exists. |
| Fluids/water | Research/planned | No shallow-water, particle, APIC, flooding, or puddle simulation exists. |
| Thermal material state | Planned | Material thermal facets are contracts only. |
| Coupling with weather/terrain | Planned | Isobar/Basalt seams exist only as specifications. |
| Disabled behavior | Planned requirement | Must be proven before any pack ships. |

## 4. Public contracts

```text
TorsantPackId(u128 persistent_uuid)
ThermalMaterialProfileId(u128 persistent_uuid)
ThermalSnapshotId { slot: u32, generation: u32, epoch: u64 }

ThermalMaterialState {
  temperature_c: f16,
  heat_capacity: f16,
  conductivity: f16,
  ignition_state: IgnitionState,
  wetness_coupling: f16,
}

FireSmokeSnapshot {
  epoch: u64,
  heat_field: FieldHandle<Scalar>,
  flame_field: FieldHandle<Scalar>,
  smoke_density: FieldHandle<Scalar>,
  source_events: [ThermalEventRef],
  stability_report: SolverStabilityReport,
}

FluidSurfaceSnapshot {
  epoch: u64,
  water_depth_mm: FieldHandle<Scalar>,
  velocity: FieldHandle<Vec2>,
  foam_or_sediment: FieldHandle<Scalar>,
  boundary_conditions: [FluidBoundaryRef],
  stability_report: SolverStabilityReport,
}
```

Snapshots are optional. Consumers must handle absence as a supported disabled
state, not as a failure.

## 5. Dependency direction

Allowed:

- Torsant may depend on core math/units, diagnostics, tasks, assets/world
  documents, saves, package/cache contracts, Isobar public weather snapshots,
  and Basalt public terrain/surface snapshots.
- Torsant may consume validated Alluvium initial conditions, material facets,
  baked fields, and bounded runtime-recipe inputs while retaining solver authority.
- Penumbra may consume Torsant visual source fields through renderer-owned
  resources when the pack is enabled.

Invalid:

- Torsant directly issuing Penumbra render passes.
- Torsant mutating Isobar weather or Basalt terrain source documents without
  typed operations.
- Gameplay depending on private solver internals.
- External native/cloud solver execution without explicit capability,
  provenance, and recovery controls.
- Torsant writing Alluvium recipe source or allowing authoring-time bakes to
  advance live solver state.

## 6. Runtime pipeline

Optional coupled pipeline:

```text
Isobar publishes wind/moisture snapshot N
-> Basalt publishes terrain/surface snapshot N
-> Torsant consumes N according to declared latency
-> Torsant updates enabled fire/fluid/thermal tiles
-> Torsant publishes source fields for epoch N+1
-> Penumbra, audio, gameplay, or Isobar consume fields at allowed coupling point
-> scheduler records stability, downgrade, skipped coupling, and disabled state
```

No pack may create a hidden per-frame feedback loop. Solver instability disables
the affected tile/pack and preserves source data.

## 7. Capability tiers and disabled behavior

| Tier | Behavior |
|---|---|
| Disabled | No Torsant crates/tasks/field pages/GPU resources/package chunks/listeners/recurring queries. |
| Authored approximation | Hand-authored heat/smoke/wetness events and cheap visual/audio/gameplay cues. |
| Local simple solver | Bounded tile solver with CPU fallback and deterministic replay envelope. |
| GPU-assisted solver | GPU compute only with transfer, determinism, recovery, and unsupported-outcome evidence. |
| Research | Fire, smoke, APIC-like fluids, shallow water, snow/granular, erosion, thermal portfolios. |

## 8. Diagnostics, recovery, and security

Diagnostics:

- enabled packs and active tiles;
- solver tier and algorithm ID;
- CPU/GPU cost, memory, and field pages;
- stability, conservation/error estimates where meaningful;
- coupling latency and skipped coupling reasons;
- source event provenance;
- disabled-pack zero-work assertions.

Recovery:

- If a tile update fails, keep last valid snapshot or disable the tile.
- If a solver becomes unstable, emit diagnostic and fall back to authored/simple
  tier where available.
- If cache is corrupt, invalidate only affected fields and regenerate or skip.
- If external solver capability is revoked, stop solver work and keep source
  data authoritative.

Security:

- Solver inputs, package fields, external references, and caches are untrusted.
- Validate lengths, dimensions, counts, time steps, bounds, units, hashes, and
  compression before allocation.
- External solvers require explicit permission, pinned provenance, sandboxing,
  redaction, and no ambient filesystem/network access.

## 9. Accessibility

Fire, smoke, steam, bright flashes, water motion, and heat-haze visuals must
respect photosensitivity, contrast, reduced-motion, audio cue, and recovery
settings. Gameplay-critical thermal or fluid state must have non-visual
diagnostics/cues where relevant.

## 10. Tests, evidence, research, and delivery

Tests:

- disabled-pack zero-work tests;
- snapshot publication/reclamation and stale-reader rejection;
- solver instability downgrade/recovery;
- corrupt field cache recovery;
- deterministic replay envelope for declared deterministic tiers;
- field dimension/unit validation and fuzzing.

Workloads: PEN-B05, PEN-B07, PEN-B11, PEN-B14, and PEN-B15.

Delivery: Torsant is not required by the prototype or opening slice. Research
may begin in MS-08 through `RG-TOR-001`; only selected, measured, optional
packages may enter later profiles. MS-10 still requires zero-cost-disabled and
supported-profile evidence for every included package.

## 11. Adopted decisions

[ADR-0008](../docs/architecture/decisions/ADR-0008-isobar-basalt-torsant-boundaries.md)
owns subsystem boundaries; [ADR-0014](../docs/architecture/decisions/ADR-0014-optional-capability-packs.md)
owns zero-cost-disabled behavior. `RG-TOR-001` requires future ADRs for each
production solver portfolio.

## 12. End-to-end, failure, and performance-debug examples

End-to-end: an authored ignition event references stable Basalt surface and
Isobar wind epochs. An enabled Torsant package validates capability and units,
updates bounded tiles, publishes heat/smoke source fields at the declared
latency, and lets Penumbra, audio, vegetation, and gameplay consume immutable
results. With the package disabled, none of those tasks or resources exist.

Failure/recovery: a GPU solver tile becomes unstable. Torsant records the
algorithm, time step, field dimensions, and error estimate; disables the tile or
falls back to its declared simple tier; preserves the source event; and prevents
the invalid output from feeding Isobar or gameplay.

Performance debug: PEN-B07 exceeds its memory envelope. The report separates
solver pages, transfer buffers, smoke rendering, vegetation events, and temporal
history. A tier change is accepted only when repeated evidence improves memory
and frame-time distributions without hiding instability or silently changing
the coupled simulation contract.
