# Basalt Terrain and Large-World Geometry Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Rendering](RENDERING_AND_GRAPHICS_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Normative Basalt architecture

Documentation maturity: `ResearchReady`. Implementation maturity: `Scaffold`
with partial large-world/streaming precursors in other crates. Governing IDs:
`REQ-BAS-001`, `WP-BAS-001`, `RG-BAS-001`.

Basalt owns terrain runtime/source schemas, vegetation geometry contracts,
large-world spatial precision, origin rebasing, geometry residency, and
path-independent terrain snapshots. Alluvium owns procedural terrain recipes,
field evaluation, generation, overrides, and cooking. Current implementation
status is Partial precursor plus Planned:
64-bit world positions, default cells, origin rebasing, spatial records,
residency, deterministic cell request/priority/cancellation, and bounded
activation have foundations. Production terrain, vegetation, large-world
geometry generation, and renderer-ready Basalt snapshots remain planned.

Schema/API blocks are planned contracts, not current runnable examples.

## 1. Scope

Owns:

- terrain source documents, terrain cells, large-world coordinate contracts,
  origin rebasing, vegetation placement geometry, geometry residency requests,
  and terrain/vegetation snapshot contracts;
- path-independent source data that can support the opening route without being
  hard-coded to that route;
- terrain/vegetation diagnostics, fallback tiers, and native precision evidence.

Does not own:

- Alluvium recipes, field evaluation, generated identity, override
  reconciliation, or procedural cooking;
- Penumbra render backend internals or shader pipelines;
- Isobar weather authority;
- Torsant fire/fluid/thermal solvers;
- private Project Meridian creative route details beyond engine-facing
  acceptance constraints.

## 2. Goals and non-goals

Goals:

- Represent large worlds with stable persistent IDs and high-precision source
  coordinates.
- Publish renderer/physics/audio/gameplay snapshots without exposing source
  document internals.
- Keep origin rebasing explicit and safe for temporal rendering, physics,
  audio, saves, and network-ready IDs.
- Provide terrain and vegetation geometry for Penumbra through measured
  residency and LOD contracts.
- Preserve manual overrides and deterministic regeneration.
- Consume typed Alluvium terrain/field outputs without making generated caches
  source authority.

Non-goals:

- No virtual-geometry production path before a preregistered post-MS-05 gate.
- No world-scale planet renderer for the opening slice.
- No hidden dependency on private game route data.
- No claim that marker terrain/vegetation crates implement Basalt.

## 3. Status

| Area | Status | Evidence and limit |
|---|---|---|
| Large-world positions | Partial precursor | 64-bit world positions, default cells, origin rebasing, spatial records, and residency foundations exist. |
| Streaming requests | Partial precursor | Deterministic cell requests, priority, cancellation, and bounded activation exist. |
| Terrain source documents | Planned | Authoritative source-world directory and terrain schemas remain planned. |
| Terrain rendering geometry | Planned | No production terrain mesh/LOD/vegetation snapshot path is implemented. |
| Vegetation placement | Scaffold/planned | Vegetation crates are marker/scaffold only. |
| Virtual geometry | Research | Post-MS-05 only; not an MS-04/MS-07 baseline. |

## 4. Public contracts

```text
TerrainCellId(u128 persistent_uuid)
TerrainSourceId(u128 persistent_uuid)
VegetationSetId(u128 persistent_uuid)
GeometryArtifactHash([u8; 32])

WorldPosition {
  cell: TerrainCellId,
  meters_from_cell_origin: DVec3,
}

RenderRelativeTransform {
  origin_epoch: u64,
  camera_relative_position: Vec3,
  previous_camera_relative_position: Vec3,
  rotation: Quat,
  scale: Vec3,
}

BasaltGeometrySnapshot {
  epoch: u64,
  origin_epoch: u64,
  visible_cells: [TerrainCellRuntimeHandle],
  terrain_batches: [TerrainBatchRef],
  vegetation_batches: [VegetationBatchRef],
  residency_requests: [GeometryResidencyRequest],
}
```

Source data remains persistent and high precision. Penumbra consumes
camera-relative runtime transforms and must invalidate temporal histories on
origin rebase.

## 5. Dependency direction

Allowed:

- Basalt may depend on core math/units, assets/world/source documents,
  diagnostics, tasks, saves, package/cache contracts, and Isobar public field
  contracts for shelter/wind inputs.
- Basalt may consume versioned Alluvium geometry, field, semantic-spline, and
  provenance artifacts through PRC contracts.
- Penumbra, physics, audio, gameplay, and navigation may depend on Basalt
  public snapshots/query APIs.

Invalid:

- Basalt depending on `meridian-renderer` to allocate mesh buffers.
- Penumbra owning terrain source documents.
- Vegetation running a separate wind field instead of Isobar snapshots.
- Torsant mutating terrain source geometry directly without a declared Basalt
  operation and recovery record.
- Basalt invoking Alluvium editor/compiler internals from its runtime snapshot
  or residency path.

## 6. Runtime pipeline

1. Read source-world terrain documents, cell manifests, deterministic seeds,
   and accepted Alluvium outputs where authored.
2. Resolve active cells from camera, gameplay relevance, acoustics, and preload
   reasons.
3. Build or fetch terrain/vegetation artifacts by source hash and quality tier.
4. Apply manual overrides and author-approved regeneration deltas.
5. Publish immutable Basalt geometry snapshot for the frame/fixed epoch.
6. Penumbra resolves visual facets and uploads changed geometry.
7. Physics/audio/gameplay consume their own facets from the same persistent
   source identity.
8. Diagnostics record active cells, origin epoch, precision, residency, upload
   pressure, LOD, and fallback decisions.

## 7. Capability tiers and disabled behavior

| Tier | Behavior |
|---|---|
| Disabled | Terrain/vegetation packs omitted; no cells, geometry tasks, renderer resources, or package chunks. |
| Opening baseline | Source cells, camera-relative transforms, conventional terrain meshes, conservative vegetation LODs. |
| Standard large world | Multi-reason streaming, origin rebasing, deterministic regeneration, manual override preservation. |
| High vegetation | Denser vegetation, wind animation, improved LOD/overdraw controls after PEN-B02/PEN-B06 evidence. |
| Research | Virtual geometry, meshlets, sparse terrain pages, and deformable terrain candidates. |

## 8. Diagnostics, recovery, and security

Diagnostics:

- active cells and request reasons;
- origin epoch/rebase history;
- precision/floating-origin warnings;
- terrain/vegetation artifact hash and source version;
- LOD/residency/upload pressure;
- stale snapshot and missing artifact;
- disabled-pack zero-work assertions.

Recovery:

- If a terrain artifact is corrupt, invalidate only the artifact and rebuild or
  fall back to a lower tier.
- If rebase metadata is inconsistent, stop publishing new snapshots and preserve
  source data.
- If streaming activation is cancelled, keep previous valid cell state until a
  complete replacement exists.

Security:

- Terrain imports, heightfields, vegetation masks, and sidecars are untrusted.
- Validate counts, bounds, path references, nesting, compression, hashes, and
  units before allocation.
- External DCC/live-link data cannot become authoritative without provenance and
  reversible operations.

## 9. Accessibility

Terrain and vegetation must not hide critical route readability, recovery UI,
or navigation feedback. Wind/vegetation animation must respect motion settings.
Editor terrain diagnostics need keyboard and screen-reader reachable
alternatives to color-only heatmaps.

## 10. Tests, evidence, research, and delivery

Tests:

- origin rebase transform and temporal invalidation;
- cell request priority/cancellation/recovery;
- deterministic terrain/vegetation regeneration;
- source/artifact hash and corrupt cache recovery;
- renderer snapshot stale epoch rejection;
- disabled-pack zero-work tests.

Workloads: PEN-B01, PEN-B02, PEN-B03, PEN-B06, PEN-B09, PEN-B10, PEN-B11,
PEN-B13, and PEN-B15.

Delivery: MS-01 carries source/streaming precursors; MS-04 defines Basalt
snapshots and conventional geometry; `WP-PRC-003` supplies the Alluvium terrain
and field authoring handoff; MS-05 proves the representative terrain and
vegetation renderer; MS-07 validates the opening slice; MS-08 may expand
large-world systems. `RG-BAS-001` owns later geometry hierarchy
selection and remains separate from `RG-PEN-001` renderer-path research.

## 10.1 Work package brief (medium — Scaffold)

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md)
for the next real step beyond the current Scaffold-plus-partial-precursor
state. No status change.

**`WP-BAS-001` — Basalt terrain and large-world geometry foundation**
Result: terrain runtime/source schemas, large-world spatial precision,
origin rebasing, geometry residency, and path-independent terrain snapshots
become real (§1) — MS-04's "Basalt snapshots and conventional geometry"
(§10). Owning crate: `meridian-basalt`. Entry conditions: the existing
Partial precursors already in place (64-bit world positions, default cells,
origin rebasing, spatial records, residency, deterministic cell request/
priority/cancellation, bounded activation — current-status line) — this
package completes rather than starts from zero; `WP-PRC-003` supplies the
Alluvium terrain/field authoring handoff (§10) as a parallel input, not a
blocker for the geometry/residency machinery itself. Deliverables: the
public contracts in §4, the dependency direction in §5, the runtime
pipeline in §6, and disabled-pack zero-work behavior (§7). Non-goals: no
Alluvium recipe/field evaluation or generated-identity ownership (§1 —
Basalt receives typed geometry, it does not author it, matching the
forbidden-edge check in `PROCEDURAL_AUTHORING_SPEC.md` §15's
`RISK-PRC-007`); no Penumbra render-backend internals; no Isobar weather or
Torsant simulation ownership (§1); terrain-hierarchy algorithm selection
stays `RG-BAS-001`, separate from `RG-PEN-001` renderer-path research
(§10, §11). Tests: the §10 list (origin rebase transform and temporal
invalidation, cell request priority/cancellation/recovery, deterministic
terrain/vegetation regeneration, source/artifact hash and corrupt-cache
recovery, renderer snapshot stale-epoch rejection, disabled-pack zero-work).
Stop condition: a stale renderer-snapshot epoch must be rejected, never
consumed as current (§10) — matches Isobar's equivalent rule, since both
feed Penumbra's path-independent snapshot contracts. Next unblocked: MS-05's
representative terrain/vegetation renderer proof (§10), which this package
is the direct prerequisite for.

## 11. Adopted decisions

[ADR-0008](../docs/architecture/decisions/ADR-0008-isobar-basalt-torsant-boundaries.md)
owns subsystem boundaries; [ADR-0005](../docs/architecture/decisions/ADR-0005-shared-renderer-systems.md)
owns renderer-path independence. `RG-BAS-001` requires a future ADR when a
production terrain/geometry portfolio is selected.
Alluvium authoring ownership is governed by
[ADR-0017](../docs/architecture/decisions/ADR-0017-alluvium.md).

## 12. End-to-end, failure, and performance-debug examples

End-to-end: an artist edits a terrain source cell and preserves a hand-authored
route override. Basalt rebuilds only affected terrain and vegetation artifacts,
publishes a new geometry epoch, and lets Penumbra, Cairn, audio, and gameplay
resolve their own facets from the same stable source identities.

Failure/recovery: a streamed cell references a corrupt derived mesh. Basalt
keeps the previous valid cell active, invalidates the corrupt artifact, rebuilds
or chooses a declared lower tier, and never rewrites source terrain or manual
overrides as a recovery shortcut.

Performance debug: PEN-B10 reports a traversal hitch. The trace attributes cell
selection, generation, decode, upload, vegetation expansion, and residency
churn separately; the fix is accepted only after identical route and cache-state
runs improve distributions without route holes, precision faults, or memory
regression.
