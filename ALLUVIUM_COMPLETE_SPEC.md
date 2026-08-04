# The Alluvium Engine — Complete Merged Specification

This file is a **generated merge**, assembled on 2026-07-31 to make Alluvium
understandable from one place instead of the ~100 files that reference it
across the repository. It is not itself a source of truth — if the original
documents change, this file goes stale. **Source of truth, in order:**

1. `specs/PROCEDURAL_AUTHORING_SPEC.md` — the canonical Alluvium specification
2. `docs/architecture/decisions/ADR-0017-alluvium.md` — the adoption decision
3. `docs/migrations/V0_4_ALLUVIUM_AMENDMENT.md` — the migration ledger
4. `specs/registry/*.json` — machine-readable requirements/work-packages/gates/risks/evidence
5. Every other spec file listed in the [Cross-References](#6-cross-references-from-other-specs) section, at the boundary paragraph cited

No source files were modified to produce this merge. Implementation/code state
(the `engine/meridian_alluvium` crate, editor CLI, etc.) is intentionally
**excluded** — see [§7](#7-excluded-not-merged). This is a specification/design
reference, not a code or status dump.

## Table of Contents

1. [Core Specification](#1-core-specification-verbatim) — full text of `PROCEDURAL_AUTHORING_SPEC.md`
2. [ADR-0017: The Alluvium Engine](#2-adr-0017-the-alluvium-engine-verbatim) — the adoption decision
3. [v0.4 Alluvium Amendment Migration Ledger](#3-v04-alluvium-amendment-migration-ledger-verbatim)
4. [Registry Entries](#4-registry-entries) — REQ-PRC, WP-PRC, RG-PRC, RISK-PRC, EV-PRC, delivery-plan rows
5. [Delivery Roadmap Milestones Involving Alluvium](#5-delivery-roadmap-milestones-involving-alluvium)
6. [Cross-References From Other Specs](#6-cross-references-from-other-specs) — boundary excerpts, grouped by topic
7. [Excluded (Not Merged)](#7-excluded-not-merged)

---

## 1. Core Specification (verbatim)

*Source: `specs/PROCEDURAL_AUTHORING_SPEC.md`, version 0.5, 2026-07-15. This is
the normative, self-contained Alluvium specification — 18 sections.*

# The Alluvium Engine — Procedural World Authoring and Asset Generation Specification

[Master index](specs/MERIDIAN_MASTER_SPEC.md) · [Migration register](specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Assets/world/save/package formats](specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Basalt](specs/BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md) · [Vegetation](specs/VEGETATION_ECOSYSTEM_SPEC.md) · [Isobar](specs/ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md) · [Torsant](specs/TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md) · [Competitive quality](specs/COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md) · [Validation](specs/TESTING_BENCHMARKS_AND_VALIDATION.md) · [Delivery roadmap](specs/DELIVERY_ROADMAP.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by [ADR-0017](#2-adr-0017-the-alluvium-engine-verbatim). Documentation maturity: `ResearchReady`. Implementation maturity: `ImplementedFoundation`.

Governing IDs: `REQ-PRC-001` through `REQ-PRC-010`; `WP-PRC-001` through
`WP-PRC-010`; `RG-PRC-001`; `RG-PRC-002`; post-1.0 `PRG-REL-001`.

Current implementation status: `WP-PRC-001` is `ImplementedFoundation` after
GitHub Actions run `29511174569` passed governance and Linux, Windows, and
macOS workspace rows for `9c88cc152878b1eb22f18c236c00ad1abd984fa5`. Its
`meridian-alluvium` crate provides canonical pretty JSON
`meridian.procedural-recipe/v1` `.mproc` source, strict scalar reference
evaluation, stable generated IDs, cache integrity recovery, dirty reports,
retained override outcomes, provenance/license audit, structured CLI commands,
and a basic semantic inspector. It does not implement graph authoring, domain
adapters, runtime-safe evaluation, or a production corpus.

### 1. Authority and Position

The Alluvium Engine, shortened to Alluvium, is Meridian's first-party procedural world-authoring, asset-generation, environmental-composition, and simulation-aware cooking system. It is a core editor/build capability rather than an optional external plug-in or a thin importer around proprietary authoring software.

Alluvium is general-purpose, but its first proving requirements come from Project Meridian: a midnight forest, dense tall grass, forest-to-field transitions, drainage and terrain shaping, weather-aware vegetation, overgrown infrastructure, material weathering, and curated environmental composition. Creative rules, AMI facilities, proprietary recipes, private seeds, hero overrides, and game-specific assets remain in the separate private game repository. Engine evidence uses generated surrogates and controlled hashes only.

The `PRC` domain remains the stable governance namespace. The canonical filename remains `PROCEDURAL_AUTHORING_SPEC.md`. A future first implementation package may introduce `meridian-alluvium`; this specification does not create a placeholder crate. Internal components use descriptive names such as `graph`, `fields`, `cache`, `evaluation`, `provenance`, and `overrides` rather than multiplying branded subsystem names.

Alluvium may use permissively licensed or public-domain foundations behind Meridian-owned contracts. Replacement requires measured product benefit, provenance review, maintenance capacity, and an evidence-backed ADR. Meridian ownership means control of public semantics, source authority, diagnostics, and exit strategy—not rewriting proven dependencies for branding.

### 2. Goals and Non-Goals

Goals:

- Let creators author, regenerate, inspect, validate, cook, and ship procedural content using Meridian UI, CLI, and headless workers without requiring proprietary software.
- Provide specialized typed graph domains on one common evaluation, serialization, migration, scheduling, caching, profiling, and provenance foundation.
- Treat spatial fields as a first-class interchange between terrain, vegetation, weathering, infrastructure, acoustics, navigation, and simulation-aware authoring.
- Make every generated result reproducible or explicitly labeled otherwise.
- Preserve manual curation through stable generated identity and non-destructive override layers.
- Rebuild only invalidated regions and dependencies while keeping accepted output available during failure.
- Produce coherent visual, physical, acoustic, thermal, fire, fluid, structural, collision, navigation, and streaming facets from shared authored causes.
- Keep source data inspectable, diffable, migratable, recoverable, and suitable for Git, Meridian VCS, CI, and agents.
- Make cost, provenance, licensing, placement, rejection, cache behavior, and subsystem handoffs explainable.
- Support scalar reference, CPU SIMD, GPU, tiled, sparse, and external-worker execution where evidence justifies each path.

Long-term capability targets include the relevant strengths of Houdini, Gaea, World Machine, SpeedTree, Substance, EmberGen, Unreal Engine PCG, and Geometry Nodes. These names define comparison areas, not implementation, parity, compatibility, or superiority claims. Any comparative claim requires reproducible workload evidence and a recorded product-quality review.

Non-goals for the initial packages:

- No universal graph that mixes terrain, materials, dialogue, UI, gameplay, weather simulation, and every other domain without typed boundaries.
- No infinite-runtime-world promise.
- No full procedural city, unrestricted interior generator, complete ecosystem succession model, runtime terrain destruction, or universal multiphysics solver.
- No full visual node editor before textual recipes, headless evaluation, validation, and a basic inspector work.
- No mandatory Blender, cloud account, AI service, command-line-only workflow, or proprietary DCC.
- No opaque AI-generated binary or hidden scoring model as source authority.
- No procedural system that overwrites the only editable source or silently discards manual curation.
- No duplicate runtime solver inside Alluvium for functionality owned by Basalt, Isobar, Torsant, Cairn, vegetation, audio, navigation, streaming, or Penumbra.
- No production-quality claim from schema validation, structural smoke, occluded output, or definition-only benchmarks.

### 3. Ownership, Dependencies, and Forbidden Edges

Alluvium owns authoring intent and derivation. Runtime systems own live state and execution.

| Boundary | Alluvium owns | Consuming authority owns |
|---|---|---|
| Basalt | terrain recipes, semantic splines, source fields, erosion/drainage bakes, generated geometry inputs | runtime terrain geometry, precision, residency, deformation, and terrain snapshots |
| Vegetation | species-generation inputs, ecological suitability, placement recipes, growth-source parameters, LOD source artifacts | live instances, wind response, interaction, damage, and ecosystem runtime state |
| Isobar | authored weather profiles, exposure/shelter inputs, climate source fields, weathering bake inputs | live atmosphere, weather, wind, visibility, wetness, and forecast state |
| Torsant | authored initial conditions, baked results, bounded runtime-recipe inputs | authoritative fire, fluids, thermal, heat, and smoke simulation |
| Cairn | collision, physical-material, constraint, and structural source facets | live physics, contacts, queries, constraints, destruction, and structural state |
| Penumbra | visual geometry, materials, volumes, impostors, LOD source data, render metadata | GPU resources, visibility, lighting, shadows, temporal state, and presentation |
| Audio/acoustics | acoustic materials, regions, portals, obstruction and propagation source data | mixer, voice, propagation, output, and live acoustic state |
| Navigation | walkability, semantic routes, cost and exclusion fields | runtime navigation representation, queries, and agent state |
| World streaming | authored cells, bounds, dependencies, priorities, and runtime-recipe declarations | residency scheduling, activation, eviction, and memory policy |
| Save/gameplay | stable generated IDs, recipe instance identity, persisted parameters and override schemas | game state authority, save transactions, scripting, and runtime decisions |
| Assets/build | recipe source, generation dependencies, provenance, and cook requests | stable source/artifact identity, package manifests, license tables, and build transactions |

Authority rules:

- `.mproc` recipes, referenced source documents, exposed parameter values, seeds, and override layers are source authority.
- `.mfield` data, meshes, masks, volumes, placements, collision, navigation hints, acoustic regions, impostors, and package chunks are derived artifacts unless explicitly promoted through a recorded source transaction.
- Runtime-safe recipes are source data, but their runtime outputs are owned by the receiving runtime subsystem.
- A bake may sample immutable subsystem snapshots or versioned source inputs. It may not mutate live subsystem internals.
- Story-critical and hero spaces remain authored or tightly constrained. Procedural generation supplies candidates and connected systems; it does not overrule creative acceptance.

Allowed dependencies:

```text
schema IDs, math, diagnostics, tasks, assets, world, build contracts
  -> Alluvium graph, field, evaluation, cache, override, provenance contracts
  -> domain adapters and node libraries
  -> immutable generated artifacts and validation reports
  -> Basalt / vegetation / Isobar / Torsant / Cairn / Penumbra / audio / navigation / streaming
```

Forbidden edges:

- Consumer-game crates or private content types inside engine Alluvium code or public recipes.
- Renderer, GPU-backend, ECS, Rapier, operating-system path, or third-party handles in persistent recipe schemas.
- Alluvium directly advancing live weather, fire, fluid, physics, vegetation, audio, navigation, or streaming state.
- Domain adapters depending on one another through hidden cycles instead of typed shared inputs.
- Editor panels becoming the only way to validate, migrate, bake, or recover a recipe.
- Generated artifacts becoming hidden source authority because the recipe or provenance record is missing.

### 4. Planned Public Contracts

The following are logical contracts, not implemented Rust APIs.

```text
struct RecipeId(u128 persistent_uuid);
struct NodeId(u128 persistent_uuid);
struct GeneratedObjectId(u128 stable_derivation);

struct ProceduralRecipe {
    id: RecipeId,
    schema_version: u32,
    recipe_version: u32,
    graph: TypedGraph,
    exposed_parameters: Vec<ParameterDefinition>,
    dependencies: Vec<RecipeDependency>,
    outputs: Vec<OutputDeclaration>,
    default_seed: u128,
    determinism: DeterminismLevel,
    evaluation_policy: EvaluationPolicy,
    provenance: ProvenanceManifest,
    license_policy: LicensePolicy,
}

enum ProceduralOutputKind {
    Geometry,
    Material,
    Field,
    Volume,
    Collision,
    Navigation,
    Acoustic,
    Physical,
    Structural,
    Vegetation,
    RuntimeRecipe,
    SceneFragment,
}

enum EvaluationMode { InteractivePreview, AuthoritativeBake, RuntimeSafe }
enum DeterminismLevel { Strict, Stable, Opportunistic }

struct EvaluationRequest {
    recipe: RecipeId,
    mode: EvaluationMode,
    parameters: ParameterBlock,
    region_of_interest: Option<WorldBounds>,
    quality: QualityProfile,
    capability_profile: CapabilityProfileId,
    budget: EvaluationBudget,
    cancellation: CancellationToken,
}

struct EvaluationResult {
    outputs: Vec<GeneratedOutput>,
    diagnostics: Vec<EvaluationDiagnostic>,
    cache_report: CacheReport,
    provenance: ProvenanceManifest,
    determinism: DeterminismReport,
    resource_report: ResourceReport,
}
```

Required supporting contracts:

- Typed ports carry exact value class, units, coordinate space, optionality, cardinality, and version.
- Limited implicit conversions are deterministic, visible in compile output, and insert an inspectable conversion node. Ambiguous or lossy conversions require explicit nodes.
- Node IDs survive visual layout changes. Port and parameter identity survive compatible migrations.
- Output declarations include bounds, resolution policy, coordinate space, affected halo, artifact facets, capability requirements, fallback policy, and shipping eligibility.
- `EvaluationBudget` includes wall time, CPU work, GPU work, transient bytes, persistent-cache bytes, output bytes, object count, recursion/iteration limits, and runtime frame allocation where applicable.
- Diagnostics include stable code, severity, recipe/node/port/source span, causal chain, affected outputs, recovery action, and links to provenance and cost traces.

### 5. Graph Domains and Spatial Fields

Alluvium uses specialized graph domains on one common execution foundation:

- field graphs;
- geometry graphs;
- terrain and geology graphs;
- ecosystem and vegetation graphs;
- material and weathering graphs;
- structure and infrastructure graphs;
- validation and composition graphs.

Gameplay, narrative, UI, and live simulation state machines retain their owning systems. They may consume Alluvium outputs through typed schemas but are not folded into a universal procedural graph.

Spatial fields carry values plus semantic metadata:

- scalar, vector, categorical, distribution, signed-distance, occupancy, density, mask, flow, material, biome, moisture, exposure, temperature, fuel, and cost values;
- world/local/asset coordinate spaces, units, bounds, sampling/filter policy, validity, precision, resolution, and provenance;
- analytic, sampled dense, tiled, sparse, mesh-attached, point-cloud, GPU texture/buffer, and volume representations;
- separate preview, bake, and runtime resolutions with explicit resampling and error policy.

An evaluator may change physical representation without changing field semantics. Representation conversion participates in cache identity, diagnostics, memory planning, and determinism evidence.

### 6. Evaluation Modes and Ordered Pipeline

#### 6.1 Interactive Preview

- Optimized for low-latency iteration, bounded quality, cancellation, region-of-interest evaluation, and frequent cache reuse.
- May use approximate kernels only when marked `Opportunistic` or when a `Stable` recipe declares an accepted preview approximation.
- Never replaces an authoritative bake silently.

#### 6.2 Authoritative Bake

- Produces reproducible artifacts, full provenance, license decisions, validation reports, and atomic artifact publication.
- Uses `Stable` determinism by default and `Strict` where cross-machine byte or structural identity is required.
- Fails closed for missing required dependencies, unresolved licensing, incompatible schema, corrupt cache input, or unsupported required capability.

#### 6.3 Runtime-Safe Evaluation

- Must be explicitly authored, bounded, cancellable, streaming-aware, capability-scoped, persistence-aware, and equipped with a deterministic fallback.
- Declares maximum work, memory, output, update frequency, spatial scope, save/network behavior, and failure result.
- Cannot call editor-only nodes, unrestricted file/network APIs, external DCC tools, cloud services, or unbounded search.
- Is absent from shipping builds and schedules no work when no authored runtime recipe requires it.

#### 6.4 Compile and Execute

```text
load immutable recipe/source snapshot
-> migrate one schema step at a time
-> resolve dependencies and license/provenance policy
-> type-check ports, units, coordinates, capabilities, and outputs
-> detect illegal cycles and validate bounded feedback
-> infer bounds, dirty regions, halos, representations, and resolution
-> select strict reference, scalar, SIMD, GPU, or external-worker kernels
-> partition and fuse compatible stages
-> compute cache keys and resource plan
-> execute with cancellation and budgets
-> validate structural/domain outputs
-> atomically publish artifacts, diagnostics, provenance, and cache records
```

Failure leaves the previous accepted bake available. Partial outputs remain quarantined until the entire publication transaction passes.

### 7. Incremental Evaluation, Cache, and Scheduling

Dirty propagation tracks exact source, parameter, dependency, algorithm, region, resolution, capability, and target changes. Spatial invalidation expands by declared algorithm halos and follows typed downstream edges only.

Cache classes:

1. interactive preview;
2. source-authoritative bake;
3. platform-independent derived artifact;
4. platform-specific cooked artifact;
5. runtime transient.

A cache key includes:

- recipe schema and recipe version;
- node type, node implementation, algorithm, migration, and compiler versions;
- canonical parameters and explicit conversion nodes;
- dependency content hashes and provenance-manifest hash;
- deterministic random-stream lineage;
- region, bounds, halo, coordinate space, units, representation, resolution, and quality;
- evaluation mode and determinism level;
- capability profile and target platform where output semantics or bytes differ;
- external tool/library versions and approved donor identifiers.

Scheduling requirements:

- immutable source snapshots and generation-checked task handles;
- bounded queues, explicit priorities, cooperative cancellation, and atomic publication;
- region-of-interest evaluation and streaming of large candidate/result sets;
- memory plans for dense/tiled/sparse CPU fields and GPU intermediates;
- asynchronous GPU work without blocking the editor main loop;
- deterministic tie-breaking and publication order where observable;
- cache eviction that never deletes source, the only editable layer, or the last accepted artifact required for recovery.

### 8. Determinism and Randomness

`Stable` is the default production level.

| Level | Contract |
|---|---|
| `Strict` | Structural or byte identity across supported machines/backends; uses reference kernels or proven deterministic alternatives. |
| `Stable` | Same semantic content, generated identities, topology/category decisions, and accepted error bounds across supported execution paths. |
| `Opportunistic` | Variation is permitted and recorded; never satisfies deterministic cooking, network, or migration requirements. |

Every random stream derives from project seed, recipe ID/version, node ID/version, region/cell, user seed, stream purpose, and algorithm version. Nodes request named substreams. Hidden global randomness, thread-order randomness, wall-clock seeding, and pointer/hash-map iteration as semantic input are prohibited.

CPU scalar, CPU SIMD, and GPU paths use differential structural comparison appropriate to the output: field error bounds, topology and winding, generated identities, material facets, object counts/categories, spatial tolerances, and semantic masks. Visual similarity alone is insufficient.

### 9. Generated Identity and Non-Destructive Overrides

`GeneratedObjectId` derives from stable recipe identity, generating node, semantic source feature, region, and deterministic candidate identity—not array position or execution order.

Supported override operations:

- suppress generated object;
- transform or replace object;
- override exposed or object parameters;
- replace material/facets;
- preserve or lock accepted output;
- attach metadata, gameplay marker, or authored relationship;
- promote a generated result into an explicit authored source transaction.

Regeneration produces an override reconciliation report:

- `Applied`: target identity remains valid;
- `Migrated`: a registered migration maps old target to new identity;
- `Conflicted`: base and override changed incompatibly;
- `Orphaned`: target no longer exists and no deterministic migration is available;
- `Invalid`: override violates schema, capability, ownership, or license policy.

Orphans are retained with provenance and presented for retarget, preserve-as-authored, suppress, or delete. They are never silently dropped. Flattening creates a new checkpointed source layer and keeps recoverable ancestry.

### 10. Provenance, Licensing, Security, and Trust

Every recipe and output records:

- source recipe ID/version and source checkpoint;
- full dependency and input hashes;
- seed lineage and determinism level;
- generator/compiler/node/algorithm versions;
- external tool/library and donor identifiers;
- source licenses, SPDX expressions where applicable, notices, attribution, redistribution constraints, and modification records;
- generated-output license policy and shipping eligibility by target;
- private/restricted source classification and redacted evidence reference;
- operator or agent identity, command transaction, review, and approval where required.

Provenance propagates through every graph edge and merges conservatively. Unknown or incompatible rights cannot become permissive through generation. The cooker rejects outputs that violate target policy, have unresolved required attribution, reference forbidden private material, or lack required source records. Warnings are allowed only for explicitly non-blocking policy and cannot waive private-content leakage or missing authority.

External importers, DCCs, scripts, files, archives, generated metadata, and agents are untrusted inputs. Paths are project-relative and normalized; archives and decoders have size/depth limits; workers are isolated and resource bounded; network/cloud execution is disabled by default; secrets and private corpus material are redacted from logs and public evidence.

AI and agents may create or edit textual recipes, parameters, constraints, tests, and candidate sets only through typed commands, permissions, provenance, validation, undo/checkpoint, and license policy. They may not emit an opaque binary as source authority or bypass cooker eligibility.

### 11. Domain Authoring Requirements

#### 11.1 Terrain and Basalt

Alluvium terrain authoring combines heightfields, meshes, local SDF/voxel patches where justified, semantic splines, and spatial fields. It covers geology, slope, drainage, watersheds, channels, erosion/weathering bakes, soil, moisture, roads, paths, ditches, culverts, embankments, and placement constraints. Basalt receives typed geometry/source artifacts and retains runtime authority.

#### 11.2 Vegetation and Ecosystems

The first major proving target is dense grass and forest vegetation. Authoring includes species/archetype parameters, clumping, ecological suitability, canopy shelter, moisture, slope, soil, disturbance, paths, water, competition, wind source data, LOD/impostor source, collision, and basic damage facets.

The tall-grass proving ground must cover density, species mix, clumping, paths, trampling source data, wind/rain response inputs, optional fire/flood coupling, LOD/streaming, shadows, temporal stability, and acoustic interaction. Later research may add growth, competition, succession, seasonality, fire response, and flood response.

Placement is ecological and explainable, not undifferentiated random scatter. Every accepted or rejected candidate can identify contributing fields, constraints, random stream, score components, and overrides.

#### 11.3 Materials and Weathering

One high-level material source may generate visual, physical, acoustic, thermal, fire, fluid, and structural facets. Owning runtime systems consume their facet and do not infer authoritative nonvisual properties from rendered pixels.

Weathering derives from shared causes—exposure, moisture, drainage, contact, heat, material, age, maintenance, and biological growth—so color, roughness, geometry, friction, absorption, fuel, corrosion, and structural effects remain coherent. Independent random dirt overlays cannot substitute for causal source fields when cross-system behavior depends on them.

Planned post-1.0 source facets include `CombustionMaterialFacet` with ignition
response, available fuel, burn-rate curve, heat release, smoke/soot/char yield,
moisture response, and spread class; and `FluidInteractionFacet` with
permeability, absorption, drainage, buoyancy class, erosion response, wet
friction, and thermal exchange. Fields use declared units, bounds, defaults,
provenance, profile applicability, and migration. They are semantic authored
controls, not engineering-accuracy claims. Torsant validates and consumes them
without giving Alluvium live solver authority.

#### 11.4 Infrastructure and Structures

Semantic splines may generate connected roads, paths, rivers, drainage, fences, poles, cables, and pipes with terrain shaping, support placement, collision, navigation, streaming, material, and weathering outputs. Procedural structures use authored constraints, modules, egress/accessibility rules, structural facets, and explicit hero-space locks. Story-critical layouts remain manually accepted or authored.

### 12. Editor, CLI, Headless, and Accessibility

Initial authoring surface:

- human-readable recipe source and schema-aware text editing;
- recipe/parameter inspector with typed controls and units;
- preview of fields, geometry, placements, dirty regions, dependencies, caches, provenance, licensing, overrides, and cost;
- source spans and fix actions for diagnostics;
- headless validate, migrate, evaluate, bake, explain, diff, provenance, and license-audit operations;
- identical command transactions across editor, CLI, CI, MCP, and agents.

Planned command shape:

```text
meridian alluvium inspect <recipe.mproc>
meridian alluvium validate <recipe.mproc>
meridian alluvium migrate <recipe.mproc> --to <schema>
meridian alluvium preview <recipe.mproc> --region <bounds>
meridian alluvium bake <recipe.mproc> --profile <profile>
meridian alluvium dirty <recipe.mproc> --since <checkpoint>
meridian alluvium explain <recipe.mproc> --object <id>
meridian alluvium provenance <recipe.mproc> --output <id>
meridian alluvium license-audit <recipe.mproc> --target <target>
```

Exact executable spelling remains subordinate to Meridian's common CLI command registry; these are planned semantic operations, not implemented commands.

Beginner workflow: choose a bounded template, adjust labeled parameters, preview a selected region, inspect plain-language warnings and cost, accept a candidate, make a manual adjustment, regenerate, and see the adjustment preserved.

Expert workflow: inspect typed graph/source, representations, kernels, seed lineage, cache keys, dirty regions, memory plan, provenance, license flow, generated identity, override reconciliation, and subsystem artifacts; invoke the same operations headlessly.

The later visual graph editor provides typed ports, nested groups, reusable subgraphs, structural diffs, profiling overlays, CPU/GPU placement, migration previews, keyboard-only operation, semantic accessibility nodes, and text-source round-trip. It is `WP-PRC-007`, not part of the minimum evaluator foundation.

### 13. Persistence, Formats, and Interchange

`meridian.procedural-recipe/v1` is the planned logical source schema. `.mproc` is reserved for recipe source. Encoding is not frozen by this documentation pass; `WP-PRC-001` must select and fixture a canonical, human-readable, deterministic encoding before implementation. YAML examples are illustrative, not an encoding commitment.

`.mfield` is reserved for derived field artifacts or caches. It is never the only source authority. The specification does not reserve `.mspecies`, `.mmat`, or `.mterrain`; independent extensions require an owning schema, migration policy, fixtures, and an ADR if existing asset/facet documents are insufficient.

Every persistent recipe stores stable IDs, schema and recipe versions, node versions, explicit edges, parameters, editor metadata, dependencies, outputs, seed/determinism policy, evaluation policy, provenance, license policy, and migration history. Unknown optional data round-trips when extension policy permits it. Unknown required semantics block mutation and baking while preserving source for inspection and recovery.

[glTF](https://registry.khronos.org/glTF/), [OpenUSD](https://openusd.org/release/intro.html), [MaterialX](https://materialx.org/Specification.html), [OpenEXR](https://openexr.com/en/latest/TechnicalIntroduction.html), [PNG](https://www.w3.org/TR/png-3/), [KTX2](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html), [WAVE](https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/ns-mmeapi-waveformatex), [FLAC](https://xiph.org/flac/format.html), and [OpenVDB](https://www.openvdb.org/documentation/doxygen/overview.html) may be used at import/export boundaries when capability, fidelity, licensing, and maintenance evidence justify them. These primary format authorities were reviewed for this amendment only to validate the interchange boundary; each selected implementation, library, extension, codec, asset, and trademark policy still requires its own versioned provenance and license review. The formats are not automatically canonical source or shipping formats. Meridian owns optimized runtime artifacts and package manifests.

### 14. Performance and Capability Policy

The evaluator selects scalar reference, scalar optimized, architecture SIMD, GPU, tiled/sparse, or isolated worker execution per node/kernel using determinism, data shape, transfer cost, latency, throughput, memory, capability, and failure behavior. A single recipe may use multiple paths without changing its public semantics.

Required metrics:

- compile, preview, clean bake, dirty bake, and runtime-safe latency distributions;
- CPU work, GPU work, transfer bytes/time, worker time, queue delay, and cancellation latency;
- peak/transient/persistent memory and cache size/churn;
- cache hit/miss reasons and invalidated region/output counts;
- generated object/output counts and artifact sizes;
- scalar/SIMD/GPU structural differences and determinism status;
- editor responsiveness and time to first usable preview;
- cook and license-audit time;
- downstream streaming, draw, physics, simulation, and audio cost attributable to generated content.

Every cooked profile may produce a planned `RuntimeCostManifest` containing
predicted geometry, texture, participating-media, pipeline, shadow, light,
vegetation, weather, simulation, upload, streaming, activation, and residency
demand by region and tier. Each prediction records uncertainty, calibration
corpus, model/version, unsupported dimensions, causal source IDs, and proposed
downgrades. It never becomes runtime authority or a fabricated budget gate.

Runtime evidence reconciles predicted and observed cost. The editor explains
which authored causes contribute to a predicted or observed cost and which
change would alter quality, gameplay, accessibility, recovery, or provenance.
Prediction error remains visible calibration evidence rather than being silently
rewritten after a run.

Unsafe optimized kernels follow Meridian's unsafe policy: documented invariants, smallest scope, reference implementation, differential tests, fuzz/property coverage where applicable, sanitizer-capable paths, and measured benefit.

Core editor/build support is not an optional external pack. Domain adapters and runtime-safe evaluation remain capability-scoped. Projects with baked-only content ship generated artifacts and provenance/package records, not the editor, graph compiler, preview caches, or runtime evaluator. Disabled runtime evaluation creates no threads, tasks, allocations, package chunks, listeners, or recurring work.

### 15. Research Gates and Risk Register

`RG-PRC-001` opens after `MS-01`. It selects the initial evaluator representation and kernel portfolio behind stable recipe, field, evaluation, generated-identity, and provenance seams. The strict reference path is mandatory. Scalar/SIMD/GPU choices use differential correctness, determinism, preview latency, clean/dirty throughput, transfer, memory, device coverage, debugging, and maintenance evidence from `PEN-B01`, `PEN-B02`, `PEN-B05`, `PEN-B06`, `PEN-B10`, and `PEN-B11`.

`RG-PRC-002` remains closed until `MS-05`. It controls replacement or deep ownership of permissive third-party foundations. The default is to keep a wrapped dependency. Replacement requires a preregistered material benefit, representative evidence, provenance/license review, migration/compatibility cost, maintenance ownership, and an ADR. Branding or a trivial gain is insufficient.

Terrain hierarchy remains governed by `RG-BAS-001`; weather algorithms by `RG-ISO-001`; coupled fire/fluid/thermal solvers by `RG-TOR-001`. Alluvium does not reopen those decisions under a procedural label.

| Risk | Failure | Required mitigation |
|---|---|---|
| `RISK-PRC-001` | CPU/GPU/platform nondeterminism | reference kernels, structural differential tests, explicit level/report |
| `RISK-PRC-002` | invalidation explosion, stale or poisoned cache | dependency traces, halos, canonical keys, corruption recovery, dirty-bake workloads |
| `RISK-PRC-003` | generated identity drift and orphaned curation | stable derivation, reconciliation reports, migration fixtures, no silent deletion |
| `RISK-PRC-004` | unbounded runtime recipes | declared budgets, capability/fallback contracts, cancellation, shipping validation |
| `RISK-PRC-005` | license/provenance contamination | conservative propagation, target policy, cooker rejection, source audit |
| `RISK-PRC-006` | private Project Meridian corpus leakage | repository boundary, redaction, controlled hashes, unwaivable validation |
| `RISK-PRC-007` | duplicated subsystem authority or universal-graph coupling | typed handoffs, forbidden-edge tests, owning-spec reviews |
| `RISK-PRC-008` | visual authoring complexity and inaccessible workflows | text/CLI foundation first, progressive disclosure, keyboard/semantic tests |
| `RISK-PRC-009` | GPU transfer or unified-memory pressure | representation planner, budgets, sparse/tiled data, measured offload decisions |
| `RISK-PRC-010` | dependency lock-in or unsustainable custom kernels | Meridian seams, donor provenance, exit plans, evidence-gated replacement |

### 16. Tests, Benchmarks, and Evidence

Required unit, property, fixture, integration, and failure tests:

- logical-schema canonicalization, round-trip, unknown-field preservation, and one-step migration;
- duplicate/missing IDs, type/unit/coordinate mismatch, illegal cycles, bounded feedback, and malformed source;
- deterministic random substreams under task reordering and parallel execution;
- scalar/SIMD/GPU structural differential tests with output-specific tolerances;
- cache-key stability, precise invalidation, halo propagation, corruption recovery, and stale-generation rejection;
- cancellation before/during/after stages, budget exhaustion, worker crash, device loss, and atomic publication;
- stable generated identity across unrelated edits;
- override apply/migrate/conflict/orphan/invalid behavior and flatten recovery;
- provenance and license propagation, unresolved-rights rejection, target-policy cooking, and private-content redaction;
- CPU/GPU memory limits, large fields, sparse/tiled regions, result overflow, and queue saturation;
- domain output validation for geometry, terrain, vegetation, materials, collision, navigation, acoustics, structures, and runtime recipes;
- typed handoff tests proving Alluvium cannot mutate live subsystem authority;
- headless/editor/CLI command parity and beginner/accessibility journeys;
- zero-cost-disabled tests for baked-only shipping profiles.
- combustion/fluid facet units, bounds, defaults, migration, provenance,
  coherent cross-facet derivation, and missing-optional-value tests;
- `RuntimeCostManifest` determinism, source attribution, uncertainty,
  unsupported-dimension, prediction-versus-observation, stale-calibration, and
  editor-explanation tests.

Permanent proving workloads use the existing Penumbra corpus where relevant. `PEN-B01`, `PEN-B02`, `PEN-B05`, `PEN-B06`, `PEN-B10`, and `PEN-B11` record recipe hashes, Alluvium version, determinism level, evaluation mode, provenance-manifest hash, cache state, generated-content counts, and downstream costs. All remain `DefinitionOnly/Uncalibrated` until executed evidence exists.

Acceptance for `WP-PRC-001` requires a versioned textual recipe, strict reference evaluation, stable IDs, deterministic substreams, typed fields, precise dirty rebuild, cache recovery, overrides, provenance/license flow, headless commands, and a basic inspector. A visual graph is not required.

Acceptance for `WP-PRC-002` requires a reproducible sanitized forest/field corpus generated from public-safe recipes, manual curation preserved across regeneration, six relevant benchmark definitions carrying recipe/provenance fields, and typed handoff to Basalt, vegetation, Isobar, Penumbra, assets, streaming, navigation, Cairn, and audio as applicable. It does not disclose private AMI or route content.

### 17. Delivery Mapping

| Package | Result | Milestone | Status |
|---|---|---|---|
| `WP-PRC-001` | typed recipe, field, strict evaluator, cache, identity, overrides, provenance, text/headless/basic inspector | `MS-03`, `MS-05` | ImplementedFoundation |
| `WP-PRC-002` | Project Meridian environmental proving recipes and sanitized corpus | `MS-05` | Planned |
| `WP-PRC-003` | Alluvium–Basalt terrain and field production integration | `MS-05`, `MS-08` | Planned |
| `WP-PRC-004` | vegetation and ecosystem production integration | `MS-05`, `MS-08` | Planned |
| `WP-PRC-005` | cross-facet material and causal weathering production | `MS-08` | Planned |
| `WP-PRC-006` | semantic infrastructure and constrained structures | `MS-08` | Planned |
| `WP-PRC-007` | Meridian-native accessible visual authoring | `MS-08` | Planned |
| `WP-PRC-008` | bounded runtime-safe recipes, streaming, persistence, and fallback | `MS-08`, `MS-09` | Planned |
| `WP-PRC-009` | ecosystem growth, competition, succession, season, and disturbance | `MS-09` | Research |
| `WP-PRC-010` | measured dependency replacement and kernel optimization | `MS-09`, `MS-10` | Research |

`MS-05` requires `WP-PRC-001` through `WP-PRC-004` as dependencies of the representative forest corpus. `WP-PEN-011` and `WP-PRJ-001` consume `WP-PRC-002`. The active `WP-PRC-001` source delivery does not activate later packages or change the sequential MS-03 modeler gate.

After MS-10, `PRG-REL-001` may activate bounded work for combustion/fluid source
facets and `RuntimeCostManifest` prediction/reconciliation. It does not change
the current package chain, authorize a runtime solver, or promote Alluvium
implementation maturity.

### 18. Examples

Logical recipe example; YAML is illustrative:

```yaml
schema: meridian.procedural-recipe/v1
id: recipe.public.representative_forest
version: 1
seed: 9917
determinism: stable
evaluation:
  allowed: [interactive_preview, authoritative_bake]
outputs: [geometry, field, vegetation, collision, navigation, acoustic, scene_fragment]
graph:
  - id: slope
    op: basalt.terrain_slope
  - id: route_distance
    op: field.distance_to_spline
    input: route.public_test
  - id: suitability
    op: field.ecological_suitability
    inputs: [slope, route_distance, field.moisture, field.canopy_shelter]
  - id: trees
    op: vegetation.place
    input: suitability
    random_stream: ordinary_trees
overrides: overrides/public_representative_forest.moverride
license_policy: public_engine_fixture
```

End-to-end authoring:

```text
Designer edits the route spline and moisture field.
-> Alluvium validates source and reports affected cells plus halos.
-> Preview evaluates only the selected region with a bounded quality profile.
-> Designer locks hero trees and suppresses one ordinary tree.
-> Authoritative bake reuses valid fields, regenerates dirty terrain/vegetation,
   reconciles overrides, validates subsystem facets, and publishes atomically.
-> Build records recipe, corpus, provenance, and artifact hashes.
-> Shipping runtime loads baked Basalt, vegetation, Penumbra, Cairn, audio,
   navigation, and streaming artifacts without linking runtime Alluvium.
```

Failure and recovery:

```text
A required node library is missing after a branch switch.
-> Recipe opens read-only with source preserved and unavailable-node diagnostics.
-> Dependent outputs are stale; previous accepted bake remains playable.
-> No partial artifact is published and no override is discarded.
-> Restoring the library or applying a registered migration rebuilds only
   affected outputs and emits an override reconciliation report.
```

Performance debugging:

```text
PEN-B02 reports a traversal hitch and excessive grass memory.
-> Correlated evidence links frame, cell, artifact, recipe, node, and cache IDs.
-> Field diagnostics show dense representation selected across a sparse region.
-> RG-PRC-001-approved sparse/tiled execution is tested against the strict path.
-> The change is accepted only if structural output, determinism, frame time,
   memory, editor latency, and lower-tier behavior meet preregistered thresholds.
```

---

## 2. ADR-0017: The Alluvium Engine (verbatim)

*Source: `docs/architecture/decisions/ADR-0017-alluvium.md`*

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.4
- Implementation status: ImplementedFoundation; `WP-PRC-001` passed its CI evidence gate
- Owners: future `meridian-alluvium`, editor/build, data, and procedural workstreams
- Amends: ADR-0008, ADR-0009, ADR-0011, ADR-0014
- Supersedes: none
- Superseded by: none

### Context

Meridian needs first-party procedural authoring that can create coherent terrain, vegetation, materials, weathering, infrastructure, structures, and simulation-aware source data without requiring proprietary tools. The v0.3 procedural specification described useful graph, determinism, cache, and override foundations but treated the work too narrowly and left authoring ownership ambiguous with Basalt and optional capability packs.

### Decision

Adopt The Alluvium Engine as Meridian's core procedural world-authoring, asset-generation, environmental-composition, and simulation-aware cooking system.

Alluvium owns recipes, typed graph and field evaluation, cache/invalidation, generated identity, non-destructive overrides, provenance/license propagation, and cooking of generated outputs. It produces typed source or built artifacts for Basalt, vegetation, Isobar, Torsant, Cairn, Penumbra, audio/acoustics, navigation, world streaming, assets, packages, and saves. Those systems retain authority over live runtime state and behavior.

The editor/build capability is core Meridian functionality, not an optional proprietary plug-in. Runtime-safe evaluation remains content-triggered and capability-scoped. A baked-only project does not ship the editor, graph compiler, preview cache, or runtime evaluator and incurs no recurring Alluvium runtime cost.

`WP-PRC-001` created `meridian-alluvium` as the bounded textual scalar-reference foundation. Internal modules use descriptive names. Third-party foundations remain behind Meridian seams and are replaced only through measured research gates and an ADR.

Project Meridian supplies the first private proving requirements. Engine documents and evidence contain only sanitized functional contracts, generated surrogates, and controlled hashes; AMI content, proprietary recipes, seeds, hero overrides, and assets remain private.

### Amendments to Existing Decisions

- ADR-0008: Basalt retains terrain and large-world runtime authority; Alluvium owns procedural terrain authoring and derived source generation.
- ADR-0009: textual recipes, CLI/headless operation, and a basic inspector precede the full visual graph editor; every surface uses the same typed commands and schemas.
- ADR-0011: recipes, parameters, seeds, and overrides are source authority; generated artifacts and field caches remain derived unless explicitly promoted through a source transaction.
- ADR-0014: Alluvium editor/build support is core. Domain adapters and runtime evaluation still obey capability and zero-cost-disabled rules.

### Consequences

- The `PRC` domain remains stable while the owning specification is retitled in place.
- `MS-05` requires a minimum Alluvium foundation and environmental proving recipes.
- Alluvium cannot become a universal runtime solver or duplicate subsystem authority.
- First-party authoring cannot require proprietary software or an online account.
- AI output remains editable recipe/source data under normal command, provenance, license, and cooker policy.
- Competitive parity and performance claims require evidence; adoption of the architecture is not an implementation claim.

### Current Evidence

- [Alluvium specification](specs/PROCEDURAL_AUTHORING_SPEC.md)
- [Delivery roadmap](specs/DELIVERY_ROADMAP.md)
- [v0.4 migration ledger](docs/migrations/V0_4_ALLUVIUM_AMENDMENT.md)
- [Source data authority](docs/architecture/decisions/ADR-0011-data-authority.md)
- [Repository split](docs/architecture/decisions/ADR-0003-repository-split.md)
- GitHub Actions run `29511174569` passed governance and Linux, Windows, and macOS workspace rows for `9c88cc152878b1eb22f18c236c00ad1abd984fa5`.

### Status Review

Review after `WP-PRC-001`, after the `MS-05` representative forest evidence, and before any runtime-safe evaluator or dependency replacement is promoted.

**Related ADR amendment notices** (each of these ADRs carries a one-line pointer to ADR-0017 in its own header — quoted here so the amendment chain is visible without opening each file):

- **ADR-0008** (Isobar/Basalt/Torsant boundaries): "[ADR-0017] assigns procedural authoring and generated-source ownership to Alluvium. [ADR-0026] adds the shared Penumbra participating-media boundary, sparse/multirate policy, typed surface-fluid authority transfer, and authored material/cost facets. Basalt retains terrain and large-world runtime authority; Isobar and Torsant retain their simulation authority." It also states: "Basalt owns terrain, large-world geometry, ground/rock substrate, material-source facets, and runtime world-surface authority. Alluvium owns procedural authoring and feeds Basalt, renderer, Cairn, audio, and world streaming through schemas."
- **ADR-0009** (editor-first): "[ADR-0017] applies this command/schema parity to Alluvium textual recipes, headless execution, the basic inspector, and the later visual graph editor."
- **ADR-0011** (data authority): "[ADR-0017] defines Alluvium recipes, parameters, seeds, and override layers as source authority. Generated fields and artifacts remain derived unless promoted through an explicit source transaction."
- **ADR-0014** (optional capability packs): "[ADR-0017] makes Alluvium editor/build support a core capability. Domain adapters and runtime-safe evaluation remain capability-scoped and zero-cost when absent."
- **ADR-0022** (native modeler) amends ADR-0009, ADR-0011, **and ADR-0017** directly, with owners including Alluvium: "Alluvium may generate editable model documents and stable generated elements. Manual edits use Alluvium override/reconciliation contracts when regeneration remains live. The modeler owns direct mesh editing; Alluvium owns recipes and generation provenance."
- **ADR-0026** (environmental performance contracts), owners include Alluvium: "...behavior requires coherent authored material facets and cook-time cost insight without moving runtime authority into Alluvium." And: "Alluvium owns authored `CombustionMaterialFacet`, `FluidInteractionFacet`, and `RuntimeCostManifest` semantics. Torsant owns live solver state; Penumbra owns visual resources; observed runtime cost remains evidence rather than source."
- **ADR index** (`docs/architecture/decisions/README.md`): `ADR-0017 | The Alluvium Engine | Adopted | Partial WP-PRC-001 source delivery; CI evidence pending`

---

## 3. v0.4 Alluvium Amendment Migration Ledger (verbatim)

*Source: `docs/migrations/V0_4_ALLUVIUM_AMENDMENT.md`*

Version 0.4 · 2026-07-15 · Active migration record

This ledger maps every v0.3 procedural-authoring heading and every binding Alluvium amendment subject to a v0.4 authority or explicit disposition. Disposition vocabulary remains `Preserved`, `Split`, `Merged`, `Superseded`, or `Retired`.

### 1. v0.3 Procedural Specification Headings

| v0.3 heading | Disposition | v0.4 destination |
|---|---|---|
| Context | Merged | Alluvium sections 1-2, ADR-0017 |
| Goals and Non-Goals | Preserved | Alluvium section 2 |
| Ownership and Crate Boundaries | Superseded | Alluvium sections 1 and 3; subsystem ownership tables |
| Public Types and Data Structures | Superseded | Alluvium sections 4-5 |
| Compiler and Authoring Pipeline | Preserved | Alluvium sections 6-7 |
| Threading, Memory, and Lifetime | Split | Alluvium sections 6-7 and 14 |
| Persistence, Versioning, and Compatibility | Split | Alluvium sections 7, 9, 10, and 13 |
| Editor, CLI, MCP, and Workflows | Superseded | Alluvium section 12; agent/editor specifications |
| Diagnostics, Failure Recovery, and Security | Split | Alluvium sections 6, 10, 12, and 16 |
| Capability Tiers and Zero-Cost-Disabled Behavior | Superseded | Alluvium sections 6 and 14; ADR-0017 |
| Algorithm Alternatives and Research Gates | Superseded | Alluvium section 15; `RG-PRC-001`, `RG-PRC-002` |
| Tests, Benchmarks, and Acceptance Evidence | Preserved | Alluvium section 16; validation and workload registries |
| Delivery Mapping | Superseded | Alluvium section 17; roadmap and delivery-plan registry |
| Examples | Preserved | Alluvium section 18; API/file-format examples |

Mapped rows: 14. Unmapped rows: 0.

### 2. Binding Amendment Subjects

| Amendment subject | Disposition | v0.4 authority |
|---|---|---|
| Alluvium name and core-engine position | Preserved | Alluvium section 1; ADR-0017 |
| Competitive capability targets | Preserved | Alluvium section 2; principles and research specs |
| No proprietary first-party tool requirement | Preserved | Alluvium sections 1-2 and 12-13 |
| Permissive/public-domain dependency and replacement policy | Preserved | Alluvium sections 1, 10, 15; provenance policy |
| Authoring versus runtime authority | Preserved | Alluvium section 3; owning subsystem specs |
| Specialized domains on common evaluator | Preserved | Alluvium sections 4-6 |
| Typed recipes and outputs | Preserved | Alluvium section 4; API examples |
| Spatial field system | Preserved | Alluvium section 5 |
| Preview, bake, and runtime-safe modes | Preserved | Alluvium section 6 |
| Incremental evaluation and cache classes | Preserved | Alluvium section 7 |
| Strict, stable, and opportunistic determinism | Preserved | Alluvium section 8 |
| Explicit random substreams | Preserved | Alluvium section 8 |
| Stable generated identity and non-destructive overrides | Preserved | Alluvium section 9 |
| Provenance, licensing, redistribution, and cooker policy | Preserved | Alluvium section 10; data/security specs |
| Terrain and hybrid world-surface authoring | Split | Alluvium section 11.1; Basalt specification |
| Vegetation and tall-grass proving ground | Split | Alluvium section 11.2; vegetation specification; private game docs |
| Cross-facet materials and causal weathering | Split | Alluvium section 11.3; renderer/Cairn/audio/Isobar/Torsant specs |
| Semantic spline infrastructure and structures | Split | Alluvium section 11.4; Basalt/Cairn/data specs |
| Editable AI-generated recipes | Split | Alluvium sections 10 and 12; agent specification |
| Text/inspector first and visual graph later | Split | Alluvium section 12; editor specification; roadmap |
| CLI, headless, CI, and batch operation | Split | Alluvium section 12; build/agent specs |
| Testing and structural output comparison | Split | Alluvium section 16; validation specification |
| SIMD/GPU/tiled/sparse performance policy | Split | Alluvium section 14; `RG-PRC-001` |
| Open interchange and Meridian runtime formats | Split | Alluvium section 13; data/DCC specs |
| `.mproc` and `.mfield` | Preserved | Alluvium section 13; API examples |
| Avoid premature branded extensions | Preserved | Alluvium sections 1 and 13 |
| Capability progression | Superseded | `WP-PRC-001` through `WP-PRC-010`; delivery roadmap |
| Private Project Meridian target list | Split | private game production/opening docs; public sanitized `WP-PRC-002` |
| Initial non-goals | Preserved | Alluvium section 2 |

Mapped rows: 28. Unmapped rows: 0.

### 3. Contract and Identifier Migration

| v0.3 authority | Disposition | v0.4 destination |
|---|---|---|
| Procedural Authoring Specification title | Superseded | The Alluvium Engine title at the same canonical path |
| future `meridian-procedural` name | Retired | reserved future `meridian-alluvium`; no crate created |
| `meridian.procedural-graph/v1` definition-only example | Superseded | logical `meridian.procedural-recipe/v1`; no implemented compatibility promise |
| single `REQ-PRC-001` coverage | Split | `REQ-PRC-001` through `REQ-PRC-009` |
| single oversized `WP-PRC-001` | Split | `WP-PRC-001` through `WP-PRC-010` |
| no PRC research gate | Superseded | `RG-PRC-001` and `RG-PRC-002` |
| no PRC risk entries | Superseded | `RISK-PRC-001` through `RISK-PRC-010` |
| Alluvium work mostly in MS-08 | Superseded | minimum foundation and proving recipes required by MS-05; later packages remain MS-08/MS-09/MS-10 |
| Penumbra benchmark report v0.3 | Superseded | report v0.4 with recipe/evaluator/provenance fields |

Mapped rows: 9. Unmapped rows: 0.

### 4. Private Boundary

| Material | Disposition | Authority |
|---|---|---|
| General forest, grass, terrain, weathering, infrastructure, and structure capabilities | Preserved | public engine specifications and generated benchmark contracts |
| AMI facilities and proprietary environmental composition | Preserved | private Project Meridian documentation |
| Private recipes, seeds, hero overrides, route constraints, logos, documents, and assets | Preserved | private Project Meridian repository only |
| Public evidence | Preserved | generated surrogate, controlled private source identifier/hash, redacted differences |

Mapped rows: 4. Unmapped rows: 0.

### 5. Validation Contract

`meridian-spec list-unmapped` must report zero. Current normative documents and registries use v0.4. Historical v0.3 ADR and migration records remain legal in `docs/architecture/decisions/` and `docs/migrations/`; independent schema/report identifiers remain at their own version unless explicitly migrated above. Private-content validation remains unwaivable.

Total mapped rows: 55. Total unmapped rows: 0.

---

## 4. Registry Entries

*Source: `specs/registry/{requirements,work-packages,research-gates,risks,evidence}.json`. Every PRC-domain record, reformatted from JSON into readable Markdown with no fields dropped.*

### 4.1 Requirements (`REQ-PRC-001` … `REQ-PRC-010`)

| ID | Title | Status | Work Packages | Evidence Classes |
|---|---|---|---|---|
| `REQ-PRC-001` | Alluvium preserves deterministic provenance and manual overrides | Normative | WP-PRC-001, WP-PRC-002, WP-PRC-005, WP-PRC-006 | determinism, provenance, override-migration |
| `REQ-PRC-002` | Alluvium recipes and fields are typed, versioned, migratable, and surface-neutral | Normative | WP-PRC-001, WP-PRC-003, WP-PRC-004, WP-PRC-007 | schema-roundtrip, migration, editor-cli-headless-parity |
| `REQ-PRC-003` | Alluvium evaluation is incremental, bounded, cancellable, capability-aware, and honest about determinism | Normative | WP-PRC-001, WP-PRC-003, WP-PRC-004, WP-PRC-008, WP-PRC-009, WP-PRC-010 | partial-rebuild, cache-invalidation, cancellation, budget, differential-execution |
| `REQ-PRC-004` | Generated identity and overrides survive regeneration or produce explicit conflicts and orphans | Normative | WP-PRC-001, WP-PRC-002 | stable-identity, regeneration, orphan-override-recovery |
| `REQ-PRC-005` | Provenance, licensing, redistribution, attribution, and shipping eligibility propagate through Alluvium cooking | Normative | WP-PRC-001, WP-PRC-002, WP-PRC-005, WP-PRC-006, WP-PRC-008 | provenance, license-audit, cook-rejection |
| `REQ-PRC-006` | Alluvium produces typed artifacts without duplicating runtime subsystem authority | Normative | WP-PRC-002, WP-PRC-003, WP-PRC-004, WP-PRC-005, WP-PRC-006, WP-PRC-008, WP-PRC-009 | typed-handoff, authority-audit, forbidden-dependency |
| `REQ-PRC-007` | Project Meridian proving recipes are reproducible and curated while public evidence remains sanitized | Normative | WP-PRC-002, WP-PRC-003, WP-PRC-004 | reproducible-corpus, visual-review, benchmark, private-content-audit |
| `REQ-PRC-008` | First-party Alluvium workflows require no proprietary tools and dependency replacement is evidence-gated | Normative | WP-PRC-001, WP-PRC-007, WP-PRC-010 | dependency-audit, headless-build, interchange-roundtrip, research-gate |
| `REQ-PRC-009` | Runtime-safe recipes declare budgets, capabilities, fallbacks, persistence, and zero-cost-disabled behavior | Normative | WP-PRC-008, WP-PRC-009 | bounded-runtime, save-streaming, zero-cost-disabled, fallback |
| `REQ-PRC-010` | Alluvium cooks coherent combustion and fluid facets plus calibrated attributable runtime-cost predictions without owning live state | Normative | *(none — programs: `PRG-REL-001`)* | schema-roundtrip, typed-handoff, cost-calibration, authority-audit, reconciliation |

Two other requirements name Alluvium as a consuming/related boundary rather than owning it:
- `REQ-VEG-001`: "Vegetation consumes Alluvium, Basalt, Isobar, and Torsant contracts without duplicating authority" (work packages: `WP-VEG-001`).
- `REQ-MDL-003`: "Modifiers Alluvium overrides and runtime handoffs preserve subsystem authority" (work packages: `WP-MDL-001`, `WP-MDL-002`).
- `REQ-CORE-001`/`REQ-CORE-002` list `WP-PRC-001`/`WP-PRC-007` among the work packages proving beginner/expert workflow requirements.

### 4.2 Work Packages (`WP-PRC-001` … `WP-PRC-010`)

| ID | Title | Status | Milestones | Requirements | Depends On | Evidence |
|---|---|---|---|---|---|---|
| `WP-PRC-001` | Alluvium typed recipe, field, and evaluator foundation | **ImplementedFoundation** | MS-03, MS-05 | REQ-PRC-001, -002, -003, -004, -005, -008, REQ-CORE-001, REQ-CORE-002 | WP-DAT-002, WP-EDT-001 | `EV-PRC-20260716-001` |
| `WP-PRC-002` | Project Meridian environmental proving recipes | Planned | MS-05 | REQ-PRC-001, -004, -005, -006, -007, REQ-CORE-004 | WP-PRC-001, WP-PRC-003, WP-PRC-004 | — |
| `WP-PRC-003` | Alluvium-Basalt terrain and field production integration | Planned | MS-05, MS-08 | REQ-PRC-002, -003, -006, -007 | WP-PRC-001, WP-BAS-001 | — |
| `WP-PRC-004` | Alluvium vegetation and ecosystem production integration | Planned | MS-05, MS-08 | REQ-PRC-002, -003, -006, -007 | WP-PRC-001, WP-PRC-003, WP-VEG-001, WP-ISO-001 | — |
| `WP-PRC-005` | Alluvium material and weathering production | Planned | MS-08 | REQ-PRC-001, -005, -006 | WP-PRC-001, WP-DAT-001, WP-PEN-004, WP-PHY-001, WP-AUD-001 | — |
| `WP-PRC-006` | Alluvium infrastructure and constrained-structure authoring | Planned | MS-08 | REQ-PRC-001, -005, -006 | WP-PRC-001, WP-BAS-001, WP-PHY-001, WP-DAT-001 | — |
| `WP-PRC-007` | Alluvium native visual authoring | Planned | MS-08 | REQ-PRC-002, REQ-PRC-008, REQ-CORE-002 | WP-PRC-001, WP-UI-001, WP-EDT-001 | — |
| `WP-PRC-008` | Alluvium runtime-safe recipes and streaming integration | Planned | MS-08, MS-09 | REQ-PRC-003, -005, -006, -009, REQ-CORE-003 | WP-PRC-001, WP-RUN-004, WP-DAT-003, WP-DAT-004 | — |
| `WP-PRC-009` | Alluvium ecosystem growth and succession | Research | MS-09 | REQ-PRC-003, -006, -009, REQ-CORE-003 | WP-PRC-004, WP-PRC-008, WP-ISO-001 | — |
| `WP-PRC-010` | Alluvium measured dependency replacement and kernel optimization | Research | MS-09, MS-10 | REQ-PRC-003, REQ-PRC-008 | WP-PRC-001 | — |

`WP-MDL-001` (native editable mesh/beginner modeler) also depends on `WP-PRC-001`.
`WP-EDT-003` (Meridian Creator workspace composition) also depends on `WP-PRC-001`.

### 4.3 Research Gates (`RG-PRC-001`, `RG-PRC-002`)

**`RG-PRC-001` — Alluvium evaluator representation and execution portfolio**
Domain: PRC · Status: Planned · Opens after: MS-01 · Owner: Procedural systems lead
Stable seams: `ProceduralRecipe`, `FieldValue`, `EvaluationRequest`, `EvaluationResult`, `GeneratedObjectId`, `ProvenanceManifest`
Required workloads: `PEN-B01`, `PEN-B02`, `PEN-B05`, `PEN-B06`, `PEN-B10`, `PEN-B11`
Decision rule: "Require a strict reference path; select scalar, SIMD, GPU, tiled, sparse, or worker kernels by structural correctness, determinism, latency, throughput, transfer, memory, recovery, platform coverage, debugging, and maintenance evidence." ADR required on decision.

**`RG-PRC-002` — Alluvium dependency replacement and deep kernel ownership**
Domain: PRC · Status: ClosedUntilMS05 · Opens after: MS-05 · Owner: Procedural systems lead
Stable seams: `ProceduralRecipe`, `FieldValue`, `EvaluationRequest`, `EvaluationResult`, `ProvenanceManifest`
Required workloads: `PEN-B01`, `PEN-B02`, `PEN-B06`, `PEN-B10`, `PEN-B11`
Decision rule: "Keep permissive foundations by default; replacement requires preregistered material product benefit, representative evidence, provenance and license review, migration cost, maintainable ownership, and an ADR." ADR required on decision.

### 4.4 Risks (`RISK-PRC-001` … `RISK-PRC-010`)

| ID | Title | Status | Owner | Milestone |
|---|---|---|---|---|
| `RISK-PRC-001` | Alluvium CPU GPU or platform nondeterminism | Open | Procedural systems lead | MS-05 |
| `RISK-PRC-002` | Invalidation explosion stale cache or cache poisoning | Open | Procedural systems lead | MS-05 |
| `RISK-PRC-003` | Generated identity drift and orphaned curation | Open | Data lead | MS-05 |
| `RISK-PRC-004` | Unbounded runtime-safe recipes | Open | Runtime lead | MS-08 |
| `RISK-PRC-005` | Generated-content license or provenance contamination | Open | Release lead | MS-05 |
| `RISK-PRC-006` | Private Project Meridian recipe or corpus leakage | Open | Security lead | MS-00 |
| `RISK-PRC-007` | Duplicated runtime authority or universal-graph coupling | Open | Architecture lead | MS-05 |
| `RISK-PRC-008` | Inaccessible or unmanageable visual authoring complexity | Open | UI lead | MS-08 |
| `RISK-PRC-009` | CPU GPU transfer and unified-memory pressure | Open | Performance lead | MS-05 |
| `RISK-PRC-010` | Dependency lock-in or unsustainable custom kernels | Open | Procedural systems lead | MS-09 |

### 4.5 Evidence (`EV-PRC-20260716-001`)

Domain: PRC · Kind: test · Status: **Pass**
Source: "GitHub Actions run `29511174569` for `9c88cc152878b1eb22f18c236c00ad1abd984fa5` passed governance and the Linux, Windows, and macOS workspace rows: format, workspace check/test, warning-denied clippy, editor headless/UI-headless smokes, runtime profile smoke, bounded Cargo helper and artifact smokes, and the minimal-runtime dependency audit. The workspace suite includes canonical recipe, deterministic/dirty evaluation, cache recovery, override, CLI/UI parity, and public Creator Alpha recipe journey tests."
Visual: false
Limits:
- CI is headless and establishes no native presented-surface or visual-quality claim.
- This qualifies only the bounded textual scalar-reference Alluvium foundation; graph editing, environment adapters, runtime-safe evaluation, GPU/SIMD evaluation, and a production corpus remain planned.
- The required native Creator Alpha screenshots and keyboard/accessibility review remain MS-03 integration evidence.

Related governance evidence (`EV-GOV-20260715-003`, closing the v0.4 Alluvium amendment itself) records the limit: "Documentation cannot promote Alluvium implementation maturity; Alluvium remains Planned with no crate or runtime/editor implementation." *(Note: this predates `WP-PRC-001` passing CI — see `EV-PRC-20260716-001` above, which supersedes it in time.)*

---

## 5. Delivery Roadmap Milestones Involving Alluvium

*Source: `specs/registry/delivery-plan.json`, rows where an Alluvium (`WP-PRC-*`) package appears in the critical path, a parallel lane, integration checkpoint, exit evidence, or stop conditions. Full row content, reformatted.*

**MS-03** — entry: MS-02 Pass, source/import and build seams ready. Parallel lane "Qualified Creator behavior and domain foundations" includes `WP-PRC-001`. Integration checkpoint: "Production Meridian application shell, World workspace, and consistently composed current Creator workspaces over qualified Creator, Alluvium, build, and editable-model foundations." Exit evidence includes "Recipe editor and headless parity evidence." Stop conditions include "Alluvium source requires an opaque binary editor."

**MS-05** — entry: MS-04 Pass, representative corpus recipes ready. Parallel lane "Alluvium proving chain": `WP-PRC-001`, `WP-PRC-003`, `WP-PRC-004`, `WP-PRC-002`. Integration checkpoint: "Measured representative forest renderer plus accepted native-modeler baseline for prototype asset work." Exit evidence includes "Recipe determinism and provenance hashes." Stop conditions include "Private recipe payload enters public evidence" and "Project prototype starts before native modeler baseline."

**MS-08** — entry: MS-07 Pass, `RG-RHI-001` entry decision, stable common RHI contracts. Parallel lane "Alluvium production authoring": `WP-PRC-003`, `WP-PRC-004`, `WP-PRC-005`, `WP-PRC-006`, `WP-PRC-007`, `WP-PRC-008`. Integration checkpoint: "Engine Alpha with native Metal retained wgpu and bounded general-purpose creator gameplay Artus navigation 2D and Alluvium foundations." Exit evidence includes "Alluvium provenance and cooker audit." Stop conditions include "Procedural authoring duplicates runtime authority."

**MS-09** — entry: MS-08 Pass, mature native Metal and common RHI evidence. Parallel lane "Alluvium research": `WP-PRC-008`, `WP-PRC-009`, `WP-PRC-010`. Exit evidence includes "Alluvium differential evaluator evidence." Stop conditions include "Dependency replacement lacks measured product value."

**MS-10** — entry: MS-09 Pass, declared 1.0 profiles frozen. Parallel lane "Alluvium qualification research": `WP-PRC-010`.

---

## 6. Cross-References From Other Specs

*Every other spec file that mentions Alluvium at a system boundary, grouped by topic. Each excerpt is the paragraph/subsection containing the reference, cited by source path. This is not exhaustive of every sentence — link/index lines and repeated identical boundary phrases are omitted — but every substantive boundary statement is included.*

### 6.1 Terrain — Basalt

*Source: `specs/BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md`*

> ...large-world spatial precision, origin rebasing, geometry residency, and path-independent terrain snapshots. Alluvium owns procedural terrain recipes, field evaluation, generation, overrides, and cooking. Current implementation status is Partial precursor plus Planned: 64-bit world positions, default cells, origin rebasing, spatial records, residency, deterministic cell request/priority/cancellation, and bounded...

Basalt's non-goals explicitly exclude "Alluvium recipes, field evaluation, generated identity, override reconciliation, or procedural cooking." Its goals include: "Consume typed Alluvium terrain/field outputs without making generated caches source authority." Allowed dependencies: "Basalt may consume versioned Alluvium geometry, field, semantic-spline, and provenance artifacts through PRC contracts." Forbidden edges include "Basalt invoking Alluvium editor/compiler internals from its runtime snapshot or residency path."

Runtime pipeline step 1: "Read source-world terrain documents, cell manifests, deterministic seeds, and accepted Alluvium outputs where authored."

Delivery: "`WP-PRC-003` supplies the Alluvium terrain and field authoring handoff; MS-05 proves the representative terrain and vegetation renderer." And: "Alluvium authoring ownership is governed by [ADR-0017]."

### 6.2 Vegetation and Ecosystems

*Source: `specs/VEGETATION_ECOSYSTEM_SPEC.md`*

> ...placement results after generation, growth/damage state, LOD policy inputs, interaction events, and vegetation-specific runtime snapshots. Alluvium owns species/placement/ecosystem recipes, suitability fields, candidate generation, generated identity, overrides, and cooking. Vegetation consumes those outputs, Basalt geometry and surface authority, Isobar wind/moisture/weather fields, and optional Torsant heat/fire/thermal events.

Non-goals: "owning terrain, weather, fire/fluid solvers, renderer resources, Alluvium evaluator/source recipes, game-specific route logic, a universal ecosystem solver..."

Forbidden edges: "vegetation runtime cannot invoke Alluvium editor/compiler internals or mutate recipe/override source."

Authoritative data: "Alluvium recipes and placement/field inputs, manual instances, override stacks, growth/damage state where persistent, and source provenance." Runtime load step 1: "load species, accepted Alluvium placement outputs, seeds, Basalt surface data, and overrides."

Delivery: "MS-08 may activate richer Alluvium/Torsant integrations; `WP-PRC-009` keeps ecosystem growth and succession in research until MS-09."

### 6.3 Weather — Isobar

*Source: `specs/ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md`*

Non-goals: "Alluvium recipe evaluation, authored climate/exposure field generation, generated identity, overrides, or baking..."

> Isobar may consume versioned Alluvium-authored weather profiles, exposure, shelter, climate, and weathering inputs without allowing Alluvium to advance live weather state.

Forbidden: "Isobar runtime invoking Alluvium editor/compiler internals or writing recipe source during simulation."

Runtime pipeline step 1: "Read active weather state, authored forcing terms, deterministic seed, and accepted Alluvium source artifacts where present."

### 6.4 Fire, Fluids, Thermal — Torsant

*Source: `specs/TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md`*

Non-goals: "Alluvium-authored initial-condition recipes, source-field generation, generated identity, overrides, or authoring-time bake orchestration..."

> Torsant may consume validated Alluvium initial conditions, material facets, baked fields, and bounded runtime-recipe inputs while retaining solver authority.

Forbidden: "Torsant writing Alluvium recipe source or allowing authoring-time bakes to advance live solver state."

> Alluvium authors and cooks `CombustionMaterialFacet` and `FluidInteractionFacet`; Torsant validates and consumes them while retaining live solver authority. The facets are semantic gameplay/visual controls with units and bounds, not a claim of engineering-grade material accuracy. Missing optional values select a documented conservative tier rather than being inferred...

### 6.5 Physics — Cairn

*Source: `specs/CAIRN_PHYSICS_SPEC.md`*

Ownership table: "The Alluvium Engine | Collision, physical-material, constraint, structural, and fracture-rule source facets | Live Cairn world, contacts, constraints, solver state, or destruction authority."

Invalid dependencies: "Cairn runtime must not invoke Alluvium editor/compiler internals or infer physical authority from visual artifacts."

### 6.6 Rendering — Penumbra

*Source: `specs/RENDERING_AND_GRAPHICS_SPEC.md`*

Ownership table: "future `meridian-alluvium` / Alluvium | Visual geometry/material/volume/LOD/impostor source artifacts and provenance | GPU resources, visibility, lighting, temporal state, or render-pass ownership."

> Alluvium, Isobar, Basalt, and Torsant source-data crates must not allocate renderer textures or issue render-graph passes directly.

> Alluvium publishes immutable visual artifacts, material facets, volumes, placement/LOD data, and provenance during build or bounded runtime-safe evaluation. Penumbra never invokes editor/compiler internals or treats GPU resources as generated-source authority.

> Alluvium may author and cook the high-level material source and causal weathering inputs. Penumbra lowers only the visual facet; Cairn, audio, Isobar, and Torsant consume their own typed facets rather than inferring them from rendered appearance.

### 6.7 Audio and Acoustics

*Source: `specs/AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md`*

Boundary table: "The Alluvium Engine | Generated acoustic material/region/portal/obstruction source facets and provenance | planned | Audio retains live propagation, mixer, voice, and device authority."

> `meridian-audio` may consume versioned Alluvium acoustic artifacts; it must not invoke authoring/compiler internals from the callback or runtime graph.

### 6.8 Navigation

*Source: `specs/NAVIGATION_AND_AI_INFRASTRUCTURE_SPEC.md`*

Boundary table: "Basalt/Alluvium | compiled navigation surfaces, tiles, links, fields | source terrain, geometry, semantic regions, generation inputs."

> End to end: Alluvium emits walkability fields for a Basalt cell; NAV builds a tile; gameplay requests a path; Artus consumes locomotion targets and proposes movement; Cairn resolves it.

### 6.9 Native Modeling and DCC

*Source: `specs/NATIVE_MODELING_AND_DCC_SPEC.md`*

> The `MDL` domain owns Meridian's native editable model document, stable mesh-element identity, modeling operations, modifiers, mesh validation, UV/normals authoring, collision/LOD source tools, and beginner-first modeling workflows inside the single Meridian application. DAT owns asset identity/import/cook; Penumbra owns preview rendering; Cairn owns runtime collision; ANI owns skeleton/animation semantics; **Alluvium owns procedural recipes and generated identity**; DCC integration owns optional external-tool bridges.

Scope includes "Penumbra material/lighting preview and Alluvium-generated editable documents with override preservation."

> The native document—not a render mesh, imported binary, UI state, or modifier cache—is source authority. ...Every topology-changing operation publishes a `TopologyMap` so selections, overrides, materials, collision facets, **Alluvium identity**, and agent edits can migrate or become explicit orphans.

Ownership table: "Alluvium | editable generated result acceptance and overrides | recipes, evaluation, generated identity, regeneration conflicts."

> Undo/redo stores semantic transactions and required snapshots, not replay of arbitrary UI events. ...Alluvium regeneration enters the same transaction model: unchanged generated IDs update, manual overrides migrate, conflicts are shown, and orphan recovery remains available.

`REQ-MDL-003`: "bounded deterministic modifiers, Alluvium override migration, and Penumbra/Cairn/ANI handoffs without authority duplication."

> Tests cover topology invariants and fuzzing, stable-ID lineage, undo/redo and crash recovery, stale selections, modifiers, UV/collision/LOD fixtures, **Alluvium regeneration conflicts**, import/export loss reports, accessibility, memory, cancellation, and stripped player builds.

Failure example: "a dissolve would orphan a protected Alluvium override. The operation previews the conflict and requires remap, discard, or cancel."

*Also amended directly by ADR-0022 (native modeler) — see §2 above.*

### 6.10 Assets, World, Save, and Package Formats

*Source: `specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md`*

Goals: "Compile world, asset, shader, script, material, collision, acoustic, and navigation data, including **Alluvium-generated facets**, into independently addressable artifacts."

Boundary table: "The Alluvium Engine | Recipe source, generation dependencies, seeds, generated identity, override reconciliation, provenance, and cook requests | Asset/package identity authority, live subsystem state, or hidden artifact-only source."

> Alluvium recipes and overrides use stable source IDs and artifact hashes; `.mfield` data and generated outputs cannot become the only editable authority.

> Override operations address stable element/property/facet IDs. When a source edit, import, or Alluvium regeneration changes topology, the producer publishes identity lineage; unresolved operations become visible orphans.

> Semantic regions are stable volumes, surfaces, paths, portals, zones, biomes, rooms, ownership areas, and authoring masks. Basalt, Alluvium, Isobar, vegetation, NAV, Wavefront, streaming, gameplay, and Collective/NET may consume typed facets or immutable snapshots; none may reinterpret an unversioned display label as authority.

Build pipeline step 3: "Validate Alluvium recipe/provenance/license closure and build missing artifacts reproducibly." Step 7: "Build manifest, dependency index, mount table, patch table, license table, Alluvium provenance references, and recovery index."

Example recipe: `specs/API_AND_FILE_FORMAT_EXAMPLES.md` §14 "Alluvium terrain and vegetation recipe" — the same `recipe.public.representative_forest` YAML reproduced in §1.18 above.

### 6.11 Editor and Meridian UI

*Source: `specs/EDITOR_AND_MERIDIAN_UI_SPEC.md`*

> World editing, modeling, UI authoring, code, materials, **Alluvium**, build, profiling, VCS, diagnostics, documentation, and later tools are workspaces in that application, not separately branded Studio or IDE products.

Application shell row diagram: `World  Modeler  UI  Code  Materials  Alluvium  Build  Profile`.

Workspace table: "Alluvium | Synchronized recipe graph, parameters, canonical source, generated result, provenance, license status, and diagnostics. Text and parameters remain first-class."

> `WP-EDT-003` composes Hub, Settings, contextual/focused Code, Modeler, UI, Materials, **Alluvium**, Build, Profile, and Recovery from the same authored component vocabulary.

The Alluvium editor workspace is also registered in `specs/registry/ui-workspaces.json` as `workspace.alluvium`, owned by capability `PRC`, depending on `WP-EDT-003`, `WP-PRC-001`, `WP-PRC-007`, with five regions: `alluvium.recipe-graph`, `alluvium.parameters`, `alluvium.canonical-source`, `alluvium.generated-result`, `alluvium.provenance-diagnostics`.

The UI design brief review (`docs/production/MERIDIAN_UI_DESIGN_BRIEF_REVIEW.md`) records: "Alluvium compact and full workspace | Adopted | Graph, parameters, canonical source, preview, provenance, license, and diagnostics stay synchronized."

### 6.12 Agents, MCP, and AI

*Source: `specs/AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md`*

> For Alluvium, agents may create or edit textual recipes, parameters, constraints, tests, and candidate sets only through normal typed commands. An opaque generated mesh, field, or binary cannot become source authority without an editable recipe or an explicit reviewed promotion transaction.

Required tests: "Alluvium recipe schema, budget, deterministic replay, provenance/license, private-content redaction, candidate explanation, and no-opaque-source tests."

End-to-end example: "an agent queries a renderer diagnostic, reads selected spec/schema/code, proposes a bounded Alluvium material-recipe fix, previews semantic/source diff, provenance, license impact, cost, and tests, receives approval, executes normal commands, runs validation, and checkpoints the change."

### 6.13 Build, IDE, and Team Workflows

*Source: `specs/CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md`*

> Node kinds include Cargo check/build/test/doc, Meridian Shader Language parse/IR/target/reflect, asset import/facet/variant, native model validate/modifier/derive/interchange, animation import/compress/build, **Alluvium recipe validate/migrate/evaluate/bake/provenance/license-audit**, world/UI/logic compile, package, sign, install, launch, benchmark, and evidence assemble.

> The complete IDE is not one monolithic process. rust-analyzer, compilers, debugger adapters, importers, **Alluvium workers**, model operations, and remote workers may be bounded helper processes.

> Alluvium external tools and kernels receive only declared content-addressed inputs and cannot publish until output schema, provenance, license, budget, and determinism policy pass.

> `WP-PRC-001` integrates Alluvium validation and baking with the same observable build graph before MS-05.

### 6.14 Security, Signing, and Supply Chain

*Source: `specs/SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md`*

Untrusted-input scope: "Alluvium recipes, node libraries, generated metadata/artifacts, external authoring tools, and runtime-safe recipe inputs."

Reproducibility record fields: "Alluvium recipe/output hashes, evaluator/algorithm versions, determinism level, provenance-manifest hash, license disposition, and shipping eligibility when generated content participates."

> Alluvium donor libraries and generated outputs follow the same [provenance] policy; generation cannot erase or loosen an input license.

Required tests: "Alluvium provenance propagation, target-policy cooker rejection, private corpus redaction, runtime budget exhaustion, and hostile recipe limits."

### 6.15 Competitive Performance and Quality

*Source: `specs/COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md`*

Scope: "...environmental-performance contracts owned by Penumbra, Isobar, Torsant, and **Alluvium**; post-1.0 optimization sequencing, regression gates, and stop rules."

> Alluvium owns `CombustionMaterialFacet`, `FluidInteractionFacet`, and the authored/cooked `RuntimeCostManifest` source semantics. `PRG-REL-001` validates their convergence but cannot move authority into REL or create a universal environment solver. One-way snapshot coupling is the default.

### 6.16 Program-Level: Master Spec, Principles, Roadmap, Repository Architecture, Research, Testing, Migration Register

*Source: `specs/MERIDIAN_MASTER_SPEC.md`*

> Meridian is a general-purpose engine for games and interactive applications. Penumbra is its Meridian-owned renderer. **The Alluvium Engine is its adopted procedural world-authoring and asset-generation architecture.** Marquee is its adopted post-1.0 promotional-material authoring...

Full §9.1 "Alluvium direction":

> Alluvium is a core editor/build system for typed recipes, spatial fields, incremental evaluation, generated identity, overrides, provenance, licensing, and cooking. It authors source and derived artifacts; Basalt, vegetation, Isobar, Torsant, Cairn, Penumbra, audio, navigation, streaming, and saves retain live runtime authority. Projects using baked outputs only do not ship an Alluvium runtime evaluator. Private game recipes and creative constraints remain outside this repository.
>
> Post-1.0 Alluvium work may author coherent combustion/fluid facets and a calibrated `RuntimeCostManifest`; runtime systems retain live authority and observed traces remain evidence rather than authored truth.

§9.2 also notes: "Model source uses stable mesh-element identity and explicit topology lineage so **Alluvium generation, overrides, undo, materials, collision, and later animation** can survive edits or report recoverable conflicts."

*Source: `specs/PRINCIPLES_AND_SCOPE.md`*

> Meridian is a local-first, data-oriented engine and one user-facing creator application for games and interactive applications. It combines runtime, editor, IDE, native modeler, **The Alluvium Engine procedural authoring**, asset pipeline, build service, documentation, version control, collaboration, optional agents, packaging, deployment, and the deferred Marquee promotional workspace behind coherent typed contracts.

User persona: "Technical designer: edits typed logic, Rust gameplay data/APIs, optional Luau, materials/shaders, models, animation, UI, **Alluvium recipes/fields**, and profiles with live validation."

In-scope list: "Alluvium textual recipes, typed fields, deterministic evaluation, generated identity, non-destructive overrides, provenance, and cooking."

> Blender, Git hosting, Steam, EOS, Ollama, cloud models, external profilers, and proprietary SDKs are integrations, never prerequisites for core authoring. Meridian's native modeler and Alluvium provide first-party modeling and procedural authoring through editor, CLI, and headless workflows.

*Source: `specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md`*

> meridian-alluvium: `WP-PRC-001` implemented foundation for typed procedural recipe/field evaluation, incremental cache, generated identity, overrides, provenance, licensing, and cooking. It remains editor/build architecture; baked-only player profiles do not depend on it.

> Alluvium is core editor/build architecture, not an optional proprietary plug-in. Its domain adapters and runtime-safe evaluator remain capability-scoped. A baked-only shipping profile depends on generated asset/world/package facets, not on editor, graph compiler, preview cache, or runtime Alluvium code.

The `meridian-editor` crate composition note: "only this crate composes editor-core, UI, source import, **Alluvium**, modeler, build, and runtime-facing adapters."

*Source: `specs/RESEARCH_AND_ALGORITHM_DECISIONS.md`*

> Alluvium is the adopted procedural world-authoring and asset-generation architecture while implementation remains planned (`ADR-0017`).

Research-gate table row: "RG-PRC-001 | after MS-01 | Alluvium evaluator representation and scalar/SIMD/GPU kernel portfolio."

Section 7, "Alluvium, Isobar, Basalt, Torsant, and Wavefront research":

> `RG-PRC-001` keeps `ProceduralRecipe`, `FieldValue`, `EvaluationRequest`, `EvaluationResult`, `GeneratedObjectId`, and `ProvenanceManifest` stable while comparing strict reference, optimized scalar, architecture SIMD, GPU... [and] Alluvium buildings, ecosystems, terrain, material, and weathering work follows its registered package chain and the owning runtime subsystem gates. Sharing evaluation or field infrastructure does not imply a universal graph or solver.

Competitive-claim scope note: it "cannot select a Penumbra, Isobar, Torsant, **Alluvium**, RHI, or platform algorithm; owning research gates retain those decisions."

*Source: `specs/TESTING_BENCHMARKS_AND_VALIDATION.md`*

Report fields include: "source checkpoint, BuildId, corpus/build hashes, **Alluvium recipe hashes/version**, determinism level, evaluation mode, provenance-manifest hash, and raw evidence."

Required test summary: "Alluvium: recipe canonicalization/migration; graph types/units/cycles; strict, stable, and opportunistic determinism; named random substreams; scalar/SIMD/GPU structural differential; exact dirty regions/halos; cache corruption; bounded cancellation and memory; generated identity; override reconciliation/orphan recovery; provenance/license propagation and cooker rejection; typed subsystem [handoffs]."

Coverage claim: "all Alluvium requirements/packages/gates/risks and historical v0.4 migration rows" is part of the validation contract's mapped scope.

Benchmark report contract (`docs/benchmarks/README.md`): "The v0.5 report contract also records Alluvium recipe hashes/version, determinism level, evaluation mode, and provenance-manifest hash. Workloads not using Alluvium record explicit `NotApplicable` values." Individual workload docs (`PEN-B01-midnight-forest.md`, `PEN-B02-dense-grass-field.md`) repeat: "the future sanitized Alluvium recipe fixes recipe/version/provenance hashes, determinism level, evaluation mode, seed..." and "Independent deterministic Alluvium sweeps vary one dimension at a time. Every sweep records recipe/version/provenance hashes..."

*Source: `specs/DELIVERY_ROADMAP.md`*

Domain table row: "World authoring and simulation | PHY, ISO, BAS, VEG, **PRC**, TOR | Cairn, Isobar, Basalt, vegetation, **Alluvium**, and coupled simulation | physics wrapper transitional; named environmental crates scaffold; **Alluvium `WP-PRC-001` is `ImplementedFoundation`**."

MS-03 section: "Alluvium contributes textual recipe, headless validation/evaluation, and basic typed-inspector foundations through `WP-PRC-001`. The complete visual graph editor is not required for Editor Alpha."

MS-04 section: capabilities list includes "Alluvium/Basalt/vegetation/Isobar seams."

MS-05 section: "The critical corpus path includes `WP-PRC-001` through `WP-PRC-004` and `WP-PEN-011`. Alluvium must provide typed recipes/fields, deterministic evaluation, generated identity, overrides, provenance, a sanitized forest/field corpus, and Basalt/vegetation/Isobar handoffs. This requirement does not imply the later visual editor, materials/weathering production, structures, runtime-safe recipes, or ecosystem succession."

MS-08 section: "Alpha work includes the selected later Alluvium packages for materials/weathering, infrastructure/structures, native visual authoring, and runtime-safe recipes when independently ready."

*Source: `specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md`*

Contradiction-resolution table:

> **Environmental material and cost authoring** — Fire/fluid behavior could be inferred from pixels and optimization could begin only after runtime failure. → **Alluvium authors coherent combustion/fluid facets and derived `RuntimeCostManifest` predictions; Torsant retains live state and runtime traces reconcile predictions.**
>
> **Procedural authoring** — Procedural work was narrow, mostly deferred, and overlapped Basalt source ownership. → **Alluvium is adopted as core editor/build procedural authoring. It owns recipes, evaluation, fields, generated identity, overrides, provenance, and cooking; runtime systems retain live authority.**
>
> **Alluvium runtime cost** — "Core" could imply every game ships an evaluator. → **Core first-party authoring is always available; baked-only shipping profiles omit editor/compiler/runtime evaluator and incur zero recurring Alluvium cost.**
>
> **Procedural formats** — A definition-only graph example and many possible branded extensions could become accidental commitments. → **Logical `meridian.procedural-recipe/v1`; `.mproc` recipe source and `.mfield` derived artifacts are reserved. Other extensions require owning schemas and evidence.**
>
> **Procedural game boundary** — Project Meridian targets could leak AMI facilities, private recipes, seeds, or curation into engine fixtures. → **Public engine specs retain sanitized functional targets and hashes only; proprietary recipes and creative constraints remain private.**

Implementation-maturity table: "Alluvium | `Partial` | `WP-PRC-001` has an active source delivery for text recipes, strict scalar evaluation, recovery, CLI, and a basic inspector; its CI evidence and all production/domain work remain open." *(Note: superseded in time by `WP-PRC-001` reaching `ImplementedFoundation` — see §4.2/§4.5 above.)*

Open-research list: "Alluvium evaluator/kernel portfolio (`RG-PRC-001`) and evidence-gated dependency replacement (`RG-PRC-002`)."

*Source: `specs/AGENTS.md`*

> Keep Alluvium normative behavior in `PROCEDURAL_AUTHORING_SPEC.md`; owning runtime specs describe only their typed consumer boundary and authority.

> Alluvium recipes, fields, outputs, runtime authority, private boundary, report fields, packages, gates, and risks agree without implementation promotion.

### 6.17 Project Meridian Plans (Prototype and Vertical Slice)

*Source: `specs/PROJECT_MERIDIAN_PROTOTYPE_PLAN.md`*

> `WP-PRC-002` sanitized Alluvium environmental-corpus evidence; private recipes, seeds, constraints, and overrides remain in the game repository.

Pipeline diagram step: "-> accepted Alluvium-built artifacts ->".

Completion requirement: "Alluvium recipe/provenance hashes without public private-source payload."

*Source: `specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md`*

Private-content list: "private Alluvium recipes, seeds, generated identities, hero locks/overrides, accepted artifact/provenance hashes, and cook/license decisions."

§ "VS-03 Meridian-modeled and Alluvium-authored Basalt forest world, assets, and streaming":

> Build and curate final-source Alluvium recipes and Basalt world cells, terrain, hero trees/undergrowth/grass, collision proxies, route blockers, visibility/streaming hints, variants, provenance, and lower-cost tiers. ... Alluvium remains authoring/cooking authority only. Accepted shipping content is fixed and versioned; Basalt, vegetation, Isobar, Cairn, Penumbra, audio, navigation, streaming, and saves retain live runtime authority. Regeneration must preserve hero locks and manual overrides or produce an explicit conflict/orphan review.

### 6.18 Implementation Planning Spec

*Source: `specs/IMPLEMENTATION_PLANNING_SPEC.md`*

Milestone table rows:
- MS-03: "...`WP-BLD-001` -> `WP-EDT-001` -> `WP-PRC-001` -> `WP-MDL-001` | import/browser, recovery, accessibility, **Alluvium text/headless/basic inspector** | Creator Editor Alpha plus native-modeler baseline."
- MS-04: "...renderer foundations, shadows/IBL, **Alluvium/Isobar/Basalt/vegetation** | production-shaped Penumbra scene."
- MS-05: "`WP-PRC-001` through `WP-PRC-004`, `WP-MDL-001`, terrain, vegetation, weather, streaming, quality tiers | measured representative forest and accepted native model sources."

### 6.19 Root Governance Documents (AGENTS.md, PLANNING.md, README.md)

*Source: `/AGENTS.md` (repo root)*

> Alluvium owns recipes, procedural evaluation, generated identity, overrides, provenance, and cooking. Runtime subsystems retain live authority, and baked-only games incur no Alluvium runtime cost.

> ...sparse/multirate simulation authority and transfer dynamic surface-water ownership through a typed handoff; **Alluvium owns authored combustion/fluid facets and derived cost predictions, not live state.**

*Source: `/PLANNING.md`*

Maturity table: "Alluvium | `ImplementedFoundation` | canonical text recipes, strict scalar evaluation, derived-cache recovery, CLI, and a basic inspector are qualified; production/domain work remains open."

> User-visible result: the v0.4 suite adopts The Alluvium Engine as Meridian's core procedural world-authoring and asset-generation architecture while keeping implementation status `Planned`, runtime ownership explicit, and private game content outside the engine repository. [governance closure record, historical]

WP-PRC-001 closure record (§13, "Closed package — WP-PRC-001 / MS-03 Alluvium foundation"): "a creator can keep a public canonical `.mproc` recipe... Files/crates/formats changed: `meridian-alluvium` owns `meridian.procedural-recipe/v1` pretty canonical JSON, strict scalar evaluation, stable generated IDs, derived cache integrity/recovery, dirty reports, retained overrides, and provenance/license policy. `meridian-ui-editor` provides a semantic text-first inspector; `meridian alluvium` exposes structured command parity."

*Source: `/README.md`*

> Meridian is an experimental general-purpose game and interactive-application engine... **The Alluvium Engine** its procedural world-authoring architecture...

> `meridian-rhi` currently uses wgpu behind Meridian-owned contracts. `meridian-renderer` is Penumbra's implementation crate. Alluvium has no implementation crate; `meridian-alluvium` is reserved for its first real package rather than a marker scaffold. *(Note: this line in README.md appears not yet updated to reflect `WP-PRC-001`'s `ImplementedFoundation` status recorded elsewhere — see §4.2/§4.5. Documentation currency, not a fact resolved by this merge.)*

---

## 7. Excluded (Not Merged)

The following were deliberately left out of this merge. Nothing here is "missing spec" — it is either not specification text, or (per your explicit choice) implementation/code status that this document does not cover:

- **`engine/meridian_alluvium/` crate source** (`Cargo.toml`, `src/lib.rs`) — Rust implementation, not specification.
- **`editor/meridian_editor/tests/alluvium_cli.rs`, `editor/meridian_spec_tools/`** — test/tooling source code.
- **`editor/meridian_ui_editor/src/workspace.rs`, `editor/meridian_ui_editor/Cargo.toml`, `editor/meridian_editor/Cargo.toml`, `editor/meridian_editor/src/lib.rs`, `engine/meridian_modeler/Cargo.toml`, `engine/meridian_modeler/src/lib.rs`** — implementation source referencing Alluvium as a dependency/workspace.
- **`docs/production/ui-mockups/alluvium.svg`, `mockups.json`** — visual design asset, not text specification.
- **`examples/creator-alpha/target/meridian-evidence/**`** — generated CI evidence output (build artifacts, not source spec).
- **`editor/meridian_spec_tools/tests/fixtures/private_workload_leak/specs/registry/workloads.json`** — a test fixture deliberately containing a policy-violation example; not real registry content.
- **`Cargo.lock`** — dependency lockfile; `meridian-alluvium` appears only as a workspace member entry.
- **No implementation-status narrative section** — per your choice, this merge omits a "what's actually built today" summary. The one exception is `EV-PRC-20260716-001` (§4.5), included because it's a first-class registry record, not a code-state description.
