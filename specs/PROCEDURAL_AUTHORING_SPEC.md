# The Alluvium Engine — Procedural World Authoring and Asset Generation Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Assets/world/save/package formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Basalt](BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md) · [Vegetation](VEGETATION_ECOSYSTEM_SPEC.md) · [Isobar](ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md) · [Torsant](TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md) · [Competitive quality](COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Delivery roadmap](DELIVERY_ROADMAP.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by [ADR-0017](../docs/architecture/decisions/ADR-0017-alluvium.md). Documentation maturity: `ResearchReady`. Implementation maturity: `ImplementedFoundation`.

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

## 1. Authority and Position

The Alluvium Engine, shortened to Alluvium, is Meridian's first-party procedural world-authoring, asset-generation, environmental-composition, and simulation-aware cooking system. It is a core editor/build capability rather than an optional external plug-in or a thin importer around proprietary authoring software.

Alluvium is general-purpose, but its first proving requirements come from Project Meridian: a midnight forest, dense tall grass, forest-to-field transitions, drainage and terrain shaping, weather-aware vegetation, overgrown infrastructure, material weathering, and curated environmental composition. Creative rules, AMI facilities, proprietary recipes, private seeds, hero overrides, and game-specific assets remain in the separate private game repository. Engine evidence uses generated surrogates and controlled hashes only.

The `PRC` domain remains the stable governance namespace. The canonical filename remains `PROCEDURAL_AUTHORING_SPEC.md`. A future first implementation package may introduce `meridian-alluvium`; this specification does not create a placeholder crate. Internal components use descriptive names such as `graph`, `fields`, `cache`, `evaluation`, `provenance`, and `overrides` rather than multiplying branded subsystem names.

Alluvium may use permissively licensed or public-domain foundations behind Meridian-owned contracts. Replacement requires measured product benefit, provenance review, maintenance capacity, and an evidence-backed ADR. Meridian ownership means control of public semantics, source authority, diagnostics, and exit strategy—not rewriting proven dependencies for branding.

## 2. Goals and Non-Goals

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

## 3. Ownership, Dependencies, and Forbidden Edges

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

## 4. Planned Public Contracts

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

## 5. Graph Domains and Spatial Fields

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

## 6. Evaluation Modes and Ordered Pipeline

### 6.1 Interactive Preview

- Optimized for low-latency iteration, bounded quality, cancellation, region-of-interest evaluation, and frequent cache reuse.
- May use approximate kernels only when marked `Opportunistic` or when a `Stable` recipe declares an accepted preview approximation.
- Never replaces an authoritative bake silently.

### 6.2 Authoritative Bake

- Produces reproducible artifacts, full provenance, license decisions, validation reports, and atomic artifact publication.
- Uses `Stable` determinism by default and `Strict` where cross-machine byte or structural identity is required.
- Fails closed for missing required dependencies, unresolved licensing, incompatible schema, corrupt cache input, or unsupported required capability.

### 6.3 Runtime-Safe Evaluation

- Must be explicitly authored, bounded, cancellable, streaming-aware, capability-scoped, persistence-aware, and equipped with a deterministic fallback.
- Declares maximum work, memory, output, update frequency, spatial scope, save/network behavior, and failure result.
- Cannot call editor-only nodes, unrestricted file/network APIs, external DCC tools, cloud services, or unbounded search.
- Is absent from shipping builds and schedules no work when no authored runtime recipe requires it.

### 6.4 Compile and Execute

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

## 7. Incremental Evaluation, Cache, and Scheduling

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

## 8. Determinism and Randomness

`Stable` is the default production level.

| Level | Contract |
|---|---|
| `Strict` | Structural or byte identity across supported machines/backends; uses reference kernels or proven deterministic alternatives. |
| `Stable` | Same semantic content, generated identities, topology/category decisions, and accepted error bounds across supported execution paths. |
| `Opportunistic` | Variation is permitted and recorded; never satisfies deterministic cooking, network, or migration requirements. |

Every random stream derives from project seed, recipe ID/version, node ID/version, region/cell, user seed, stream purpose, and algorithm version. Nodes request named substreams. Hidden global randomness, thread-order randomness, wall-clock seeding, and pointer/hash-map iteration as semantic input are prohibited.

CPU scalar, CPU SIMD, and GPU paths use differential structural comparison appropriate to the output: field error bounds, topology and winding, generated identities, material facets, object counts/categories, spatial tolerances, and semantic masks. Visual similarity alone is insufficient.

## 9. Generated Identity and Non-Destructive Overrides

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

## 10. Provenance, Licensing, Security, and Trust

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

## 11. Domain Authoring Requirements

### 11.1 Terrain and Basalt

Alluvium terrain authoring combines heightfields, meshes, local SDF/voxel patches where justified, semantic splines, and spatial fields. It covers geology, slope, drainage, watersheds, channels, erosion/weathering bakes, soil, moisture, roads, paths, ditches, culverts, embankments, and placement constraints. Basalt receives typed geometry/source artifacts and retains runtime authority.

### 11.2 Vegetation and Ecosystems

The first major proving target is dense grass and forest vegetation. Authoring includes species/archetype parameters, clumping, ecological suitability, canopy shelter, moisture, slope, soil, disturbance, paths, water, competition, wind source data, LOD/impostor source, collision, and basic damage facets.

The tall-grass proving ground must cover density, species mix, clumping, paths, trampling source data, wind/rain response inputs, optional fire/flood coupling, LOD/streaming, shadows, temporal stability, and acoustic interaction. Later research may add growth, competition, succession, seasonality, fire response, and flood response.

Placement is ecological and explainable, not undifferentiated random scatter. Every accepted or rejected candidate can identify contributing fields, constraints, random stream, score components, and overrides.

### 11.3 Materials and Weathering

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

### 11.4 Infrastructure and Structures

Semantic splines may generate connected roads, paths, rivers, drainage, fences, poles, cables, and pipes with terrain shaping, support placement, collision, navigation, streaming, material, and weathering outputs. Procedural structures use authored constraints, modules, egress/accessibility rules, structural facets, and explicit hero-space locks. Story-critical layouts remain manually accepted or authored.

## 12. Editor, CLI, Headless, and Accessibility

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

## 13. Persistence, Formats, and Interchange

`meridian.procedural-recipe/v1` is the planned logical source schema. `.mproc` is reserved for recipe source. Encoding is not frozen by this documentation pass; `WP-PRC-001` must select and fixture a canonical, human-readable, deterministic encoding before implementation. YAML examples are illustrative, not an encoding commitment.

`.mfield` is reserved for derived field artifacts or caches. It is never the only source authority. The specification does not reserve `.mspecies`, `.mmat`, or `.mterrain`; independent extensions require an owning schema, migration policy, fixtures, and an ADR if existing asset/facet documents are insufficient.

Every persistent recipe stores stable IDs, schema and recipe versions, node versions, explicit edges, parameters, editor metadata, dependencies, outputs, seed/determinism policy, evaluation policy, provenance, license policy, and migration history. Unknown optional data round-trips when extension policy permits it. Unknown required semantics block mutation and baking while preserving source for inspection and recovery.

[glTF](https://registry.khronos.org/glTF/), [OpenUSD](https://openusd.org/release/intro.html), [MaterialX](https://materialx.org/Specification.html), [OpenEXR](https://openexr.com/en/latest/TechnicalIntroduction.html), [PNG](https://www.w3.org/TR/png-3/), [KTX2](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html), [WAVE](https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/ns-mmeapi-waveformatex), [FLAC](https://xiph.org/flac/format.html), and [OpenVDB](https://www.openvdb.org/documentation/doxygen/overview.html) may be used at import/export boundaries when capability, fidelity, licensing, and maintenance evidence justify them. These primary format authorities were reviewed for this amendment only to validate the interchange boundary; each selected implementation, library, extension, codec, asset, and trademark policy still requires its own versioned provenance and license review. The formats are not automatically canonical source or shipping formats. Meridian owns optimized runtime artifacts and package manifests.

## 14. Performance and Capability Policy

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

## 15. Research Gates and Risk Register

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

## 16. Tests, Benchmarks, and Evidence

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

## 17. Delivery Mapping

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

## 18. Examples

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
