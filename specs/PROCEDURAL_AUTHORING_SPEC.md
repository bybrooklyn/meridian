# Procedural Authoring Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Assets/world/save/package formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Weather/environment](WEATHER_ENVIRONMENT_AND_SIMULATION_SPEC.md) · [Audio/acoustics](AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) · [Cargo/build/team workflows](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Implementation phases](IMPLEMENTATION_PHASES.md)

Status: version 0.2 normative architecture for planned procedural-authoring work, 2026-07-14.

Current implementation status: procedural authoring is not implemented. Existing terrain, vegetation, weather, editor, and game-content crates are scaffolds or early foundation crates. This document specifies planned graph/compiler/document behavior and opening-slice constraints.

## 1. Context

Project Meridian needs procedural authoring to create and revise a curated opening forest, not to generate an infinite world at runtime. The released world is authored and fixed, with safe variants only where explicitly designed. Version 0.2 requires shared graph/compiler infrastructure, domain-specific documents, deterministic randomness, content-addressed caches, partial regeneration, non-destructive overrides, and clear "why placed/rejected" debugging. It explicitly rejects one giant graph containing terrain, dialogue, UI, weather, and gameplay.

## 2. Goals and Non-Goals

Goals:

- Provide typed graph/compiler infrastructure reusable by terrain, vegetation, weather masks, acoustic regions, validation, and later procedural packs.
- Keep domain documents separate: terrain graphs, vegetation graphs, weather masks, building grammars, material graphs, and gameplay/narrative logic are not one universal graph.
- Support deterministic candidate generation and partial regeneration.
- Preserve manual edits through an override stack.
- Show provenance and cost for every generated object.
- Make beginner workflows playable and expert workflows inspectable.

Non-goals for the opening slice:

- No procedural cities, full building/interior generator, ecosystems lifecycle, terrain destruction, runtime infinite generation, or complete universal content generator.
- No AI-only opaque scoring.
- No mandatory Blender, command-line, cloud, or AI service.
- No generation system that overwrites the only editable source layer.
- No claim that generated content is production-quality without traversal, visual, accessibility, streaming, and performance evidence.

## 3. Ownership and Crate Boundaries

Authoritative data:

- Procedural source documents are authoritative.
- Built meshes, masks, terrain tiles, vegetation placements, acoustic portals, navigation hints, and streaming metadata are generated artifacts or caches unless explicitly committed as source layers.
- Manual overrides and locks are authoritative source layers.

Planned ownership:

| Area | Owner | Status | Notes |
|---|---|---|---|
| Graph document schema and stable IDs | future procedural authoring module/tooling | planned | May start in editor tools before engine crate split. |
| Compiler core and diagnostics | future `meridian-procedural` or editor tooling | planned | Shared infrastructure, no universal domain graph. |
| Terrain/vegetation node libraries | `meridian-terrain`, `meridian-vegetation`, procedural tools | planned | Opening focus. |
| Weather/acoustic mask nodes | `meridian-weather`, `meridian-audio`/acoustics tooling | planned | Consume shared compiler contracts. |
| Candidate scoring and validation | editor tools + validation spec | planned | Score components are inspectable. |
| Built artifacts and cache storage | `meridian-assets`, world/package tools | planned | Content-addressed, recoverable. |
| Game-specific procedural rules | external consumer-game repository | planned | Must not leak into engine graph core. |

Allowed dependencies:

- Procedural tooling may depend on core IDs/math, assets, world documents, diagnostics, task workers, and domain node libraries.
- Domain node libraries depend on graph contracts, not on each other through hidden cycles.
- Runtime may load generated artifacts; it does not need editor graph execution unless a game opts into runtime generation.

Invalid dependencies:

- Invalid: a terrain node importing consumer-game runtime types.
- Invalid: dialogue or UI logic embedded in the terrain/vegetation graph.
- Invalid: renderer-only resources used as authoritative generated geometry.
- Invalid: weather simulation directly mutating procedural source layers during play.

Dependency direction:

```text
procedural source documents
  -> shared graph/compiler contracts
  -> domain node libraries
  -> generated artifacts and validation reports
  -> built asset/world/package outputs

manual overrides and locks
  -> source document layer stack
  -> partial regeneration inputs
```

## 4. Public Types and Data Structures

Rust-like schemas, not current implementation:

```text
struct ProcGraphId(u128 persistent_uuid);
struct ProcNodeId(u128 persistent_uuid);
struct ProcPortId(u64 stable_hash);
struct ProcArtifactId(u128 content_or_document_id);

struct ProcNodeDef {
    type_id: StableTypeId,
    version: u32,
    inputs: Vec<PortDef>,
    outputs: Vec<PortDef>,
    parameters: ParameterBlock,
    domain: ExecutionDomain,
    purity: Purity,
}

enum ExecutionDomain {
    Scalar,
    PerPoint,
    Raster2D,
    Volume3D,
    Mesh,
    Graph,
    WorldRegion,
    ExternalWorker,
}

struct ProcOutputDescriptor {
    bounds: Aabb,
    resolution: Option<UVec3>,
    coordinate_space: CoordinateSpace,
    seed_lineage: SeedLineage,
    provenance: ProvenanceId,
    affected_margin: Vec3,
}

struct DirtyRegion {
    bounds: Aabb,
    reason: DirtyReason,
    required_halo: Vec3,
    dependent_domains: SmallSet<ExecutionDomain>,
}

struct OverrideLayer {
    id: LayerId,
    kind: OverrideKind,
    provenance: ProvenanceId,
    conflict_policy: ConflictPolicy,
}

struct CandidateScore {
    candidate: CandidateId,
    traversal: f32,
    sightlines: f32,
    accessibility: f32,
    streaming_cost: f32,
    simulation_cost: f32,
    visual_composition: f32,
    repetition_penalty: f32,
    narrative_constraints: f32,
}
```

Graph storage:

- Stable node IDs do not change when the visual layout changes.
- Ports are typed and versioned.
- Parameters are serialized in canonical source documents with lossless editor metadata.
- Edge lists are stored explicitly; visual node positions are editor metadata.

Runtime/bake data:

- Dense field operations use SoA buffers or GPU textures where appropriate.
- Mesh outputs use immutable artifact records with source hashes and bounds.
- Generated object tables store object ID, generator node, seed lineage, source inputs, bounds, and override status.
- Debug names/provenance are retained in editor builds and compacted/stripped where shipping builds do not need them.

## 5. Compiler and Authoring Pipeline

Graph compile pipeline:

```text
resolve graph document versions
-> resolve node type definitions
-> migrate nodes if required
-> type-check ports and parameter blocks
-> detect cycles and legal feedback nodes
-> infer spatial domains, coordinate systems, units, and resolution
-> propagate dirty regions and halos
-> partition CPU, GPU, and external-worker stages
-> fuse compatible field operations
-> compute cache keys
-> plan intermediate storage and artifact writes
-> emit execution graph, diagnostics, and validation hooks
```

Opening forest generation pipeline:

1. Load source terrain/forest graph and manual override layers.
2. Generate base heightfield mesh and route concealment constraints.
3. Apply drainage/erosion bakes only at authoring time if enabled.
4. Place hero trees from authored anchors and ordinary trees from deterministic placement fields.
5. Generate undergrowth, roots, fallen logs, grass masks, and field-edge transition.
6. Generate fog/weather exposure masks and acoustic region hints.
7. Bake collision, visibility, LOD, streaming, and performance metadata.
8. Run traversal, boundary, visibility, accessibility, and benchmark validation.
9. Lock accepted region artifacts and source layers into version control.

Partial regeneration:

```text
user edits road spline or forest corridor mask
-> compute changed bounds
-> expand by algorithm halos
-> find dependent terrain, vegetation, weather, acoustic, navigation, and streaming regions
-> invalidate only matching cache keys
-> preserve locked objects and manual overrides
-> regenerate candidates inside dirty regions
-> run validation only for affected evidence
```

## 6. Threading, Memory, and Lifetime

Execution:

- CPU handles topology, constraints, graph traversal, file operations, irregular search, and candidate scoring.
- GPU handles dense fields, masks, texture-like operations, erosion bakes, scatter placement, and previews when useful.
- External worker processes handle crash-prone importers, heavyweight bakes, and optional native tools.
- Distributed workers are later and must preserve cache identity and deterministic seed behavior.

Synchronization:

- Source graph edits are transactional.
- Running bakes observe immutable source snapshots.
- Cancellation is stage- and region-granular.
- Partial artifacts are written to temp/cache locations and committed atomically.
- Editor preview consumes immutable artifact snapshots, not mutable compiler internals.

Memory:

- Large intermediate fields use tiled storage and content-addressed cache entries.
- Dense GPU intermediates have explicit lifetime plans.
- Candidate sets stream results to the editor rather than requiring all candidates in memory.
- Cache eviction is deterministic and never deletes source documents or the only editable layer.

Determinism:

- Random streams derive from project seed, graph ID, node ID, region/cell, user seed, and generator version.
- Execution order must not affect generated output.
- Deliberately nondeterministic nodes are marked and excluded from deterministic acceptance tests unless a project explicitly permits them.

## 7. Persistence, Versioning, and Compatibility

Source document requirements:

- Graph documents have a schema version, graph ID, node IDs, node type versions, edge list, parameter blocks, editor metadata, and migration history.
- Override layers preserve generated base, regional overrides, painted masks, locked objects, manual geometry, and final non-destructive modifiers.
- Flattening creates a new source layer and checkpoint; it never destroys the only editable history.

Cache identity includes:

- node type/version;
- parameters;
- input content hashes;
- spatial region;
- resolution/quality;
- target platform where relevant;
- generator/compiler version;
- random seed lineage;
- external tool versions.

Compatibility:

- New editor versions migrate old graphs through explicit node migrations.
- Missing optional node libraries show unavailable-node diagnostics and keep serialized data.
- Runtime loading of built artifacts does not require graph source unless runtime generation is enabled.

## 8. Editor, CLI, MCP, and Workflows

Beginner workflow:

1. Choose an opening-forest terrain/vegetation workspace.
2. Adjust route shape, density, slope, boundary concealment, and field-edge transition with high-level controls.
3. Press Generate Candidate.
4. Inspect playable preview in the final renderer.
5. Accept, tweak, or regenerate selected regions without losing manual edits.
6. Run "Validate Opening Route" and fix listed issues.

Expert workflow:

1. Open graph, live viewport, node inspector, output/errors/cost/provenance panel, and cache/debug views.
2. Inspect dirty regions, seed lineage, memory/GPU cost, generated object counts, and deterministic status.
3. Author domain-specific nodes and migrations.
4. Compare ranked candidates by score components, not opaque totals.
5. Trace any generated object back to generator node, input mask, seed, candidate, override, and cache artifact.

CLI commands, planned:

```text
meridian proc inspect <project> --graph <id>
meridian proc bake <project> --region <id>
meridian proc dirty <project> --since <checkpoint>
meridian proc validate <project> --opening-forest
meridian proc explain <project> --object <id>
meridian proc recover-cache <project>
```

MCP/agent surface:

- Agents may inspect graphs, propose parameter changes, generate candidates, run validation, and create checkpointed edits.
- Agents must show candidate score components and validation failures.
- Agents cannot flatten, delete source layers, enable external workers, or run cloud/AI generation without explicit permission.

## 9. Diagnostics, Failure Recovery, and Security

Diagnostics:

- graph compile errors and warnings;
- missing or deprecated node libraries;
- dirty regions and halos;
- cache hit/miss and artifact size;
- CPU/GPU stage cost;
- generated object count;
- deterministic status;
- validation score components;
- why placed/why rejected traces.

Failure recovery:

- Compile failure keeps the previous valid bake and identifies broken nodes.
- Worker crash discards partial artifacts and leaves source docs untouched.
- Cache corruption invalidates affected entries and rebuilds from source.
- Missing optional node library preserves serialized node data and blocks affected outputs only.
- Bad candidate generation can be rejected without losing prior accepted candidate.

Security:

- External tools and importers are untrusted.
- Generated content can include hostile metadata or paths; all source references are normalized and sandboxed by project policy.
- Agent/MCP edits require capability checks, operation logs, and recovery checkpoints.
- Optional distributed or cloud generation is disabled by default.

## 10. Capability Tiers and Zero-Cost-Disabled Behavior

Opening playable tier:

- Terrain and vegetation graph documents.
- Deterministic forest/field candidate generation.
- Manual overrides and object locks.
- Partial regeneration for route, terrain, and vegetation masks.
- Basic fog/weather exposure and acoustic region outputs.
- Validation hooks for traversal, streaming, visibility, and performance.

Later optional packs:

- Buildings/interiors grammar.
- Procedural materials across visual/physical/acoustic/thermal facets.
- Ecosystem lifecycle.
- Advanced erosion/drainage.
- Runtime safe variants.
- Settlement generation.
- Distributed generation.

Zero-cost-disabled behavior:

- Projects that do not use procedural authoring ship only generated artifacts, not editor graph compilers.
- Disabled building/ecosystem/material packs add no nodes, bakes, validation stages, or runtime dependencies.
- Runtime generation has no scheduler, memory, or network presence unless explicitly enabled by a game.

## 11. Algorithm Alternatives and Research Gates

Terrain:

- Heightfield mesh baseline: matches Project Meridian opening needs and existing v0.1 direction.
- Voxel/SDF terrain: useful for caves/destruction, explicitly not opening-slice scope.
- Mesh patch terrain: supports ditches/bridges/roots as separate meshes.

Vegetation:

- Deterministic scatter from suitability/density fields: opening baseline.
- Branching skeleton graph for hero plants: useful for high-detail trees, higher authoring cost.
- Ecosystem simulation: later optional pack, not needed to ship the opening.

Candidate generation:

- Rule/constraint search: inspectable and deterministic, good baseline.
- Optimization/evolutionary search: explores broader spaces, needs cost controls and reproducibility.
- AI-assisted generation: optional assistant surface only; cannot be sole authority or hidden scorer.

Erosion/drainage:

- Authoring-time hydraulic/thermal erosion bake: acceptable when measured and cached.
- Runtime erosion: later optional simulation pack.
- Painted drainage masks: practical opening fallback.

Research gates:

- Phase 11 selects opening forest terrain/vegetation graph shape and validation fixtures.
- Phase 14 establishes shared graph/compiler platform and candidate workflow.
- Phase 26 evaluates buildings/interiors/materials/ecosystems.
- Phase 27 evaluates advanced erosion, flooding, snow, and simulation coupling.

## 12. Tests, Benchmarks, and Acceptance Evidence

Tests:

- Graph migration and canonical serialization.
- Type checking and illegal cycle rejection.
- Deterministic seed replay under parallel execution.
- Dirty-region propagation and halo expansion.
- Override preservation and flatten safety.
- Cache key stability and corruption recovery.
- Missing optional node library behavior.

Benchmarks:

- Opening forest candidate bake: terrain, vegetation, fog/weather exposure, acoustic hints.
- Dirty edit benchmark: road/route mask changes invalidate only affected cells plus halos.
- Candidate comparison benchmark: score components and validation time.
- Preview memory/GPU budget fixture.

Acceptance evidence:

- End-to-end opening route generated, manually adjusted, baked, validated, and playable.
- "Why placed/rejected" works for representative tree, grass cluster, fog region, and boundary blocker.
- Partial regeneration proof after changing a route spline.
- Generated artifacts can be rebuilt from source docs and cache can be deleted safely.

## 13. Phased Implementation

- Phase 8: use minimal hand-authored/procedural tools needed to make the opening playable.
- Phase 11: weather fields, vegetation, and procedural forest authoring.
- Phase 14: shared graph/compiler platform, candidate workflow, cache identity, object provenance.
- Phase 15: optional live-link/native content-tool foundations, not required for opening.
- Phase 26: procedural buildings, interiors, materials, ecosystems.
- Phase 27: advanced erosion/flooding/snow/simulation coupling.

## 14. Examples

End-to-end opening example:

```text
Designer opens Forest Route graph.
-> Adjusts route negative-space spline and tree density target.
-> Generates three deterministic candidates for cells A-D.
-> Candidate 2 passes traversal and boundary concealment but exceeds vegetation cost.
-> Designer locks hero trees, lowers undergrowth density in distant ring, and regenerates dirty regions.
-> Build emits terrain meshes, vegetation placements, fog exposure masks, acoustic hints, and streaming metadata.
-> Opening slice loads built artifacts without runtime world generation.
```

Failure/recovery example:

```text
Vegetation node library version 4 is missing after branch switch.
-> Graph opens with unavailable-node placeholder preserving parameters and edges.
-> Outputs depending on that node are marked stale.
-> Previous accepted bake remains usable for Play.
-> Restoring the library or running migration rebuilds only affected vegetation artifacts.
```

Performance-debug example:

```text
B01 benchmark reports traversal hitch entering Zone C.
-> Procedural diagnostics show generated object count spike and streaming cell overlap.
-> Expert selects dense undergrowth cluster and runs explain.
-> Trace shows suitability field plus missing exclusion mask near the field-edge preload.
-> Designer paints exclusion mask, dirty propagation invalidates one cell plus halo, and validation confirms lower streaming cost.
```
