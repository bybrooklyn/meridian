# Penumbra Rendering Architecture Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Shader language and IR](MERIDIAN_SHADER_LANGUAGE_SPEC.md) · [First-class 2D](TWO_DIMENSIONAL_ENGINE_SPEC.md) · [Native modeler](NATIVE_MODELING_AND_DCC_SPEC.md) · [Penumbra risk register](PENUMBRA_RISK_REGISTER.md) · [Competitive quality](COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Canonical Penumbra architecture

Documentation maturity: `ResearchReady`. Architecture status: `Adopted` by
[ADR-0004](../docs/architecture/decisions/ADR-0004-penumbra-clustered-forward-plus.md),
[ADR-0005](../docs/architecture/decisions/ADR-0005-shared-renderer-systems.md),
[ADR-0006](../docs/architecture/decisions/ADR-0006-meridian-rhi-wgpu-native-backend.md),
and [ADR-0007](../docs/architecture/decisions/ADR-0007-material-shader-ir.md).
[ADR-0026](../docs/architecture/decisions/ADR-0026-environmental-performance-contracts.md)
also adopts the shared environmental participating-media boundary.
Implementation maturity: `Partial` / `Transitional`.

This document owns Meridian rendering contracts, Penumbra production-renderer
direction, RHI boundaries, capability and native-backend gates, shared workload
references, and the uniform subsystem template used by renderer-adjacent named
systems. It preserves current implementation facts: RHI, render graph, PBR
materials, cascaded shadows, diffuse irradiance IBL, extraction/upload,
pass-level timing for current high-level RHI passes, asynchronous RGBA8 surface/
offscreen capture, and native smoke are implemented or transitional foundations.
Complete render-graph timing coverage, clustered Forward+, specular IBL, native backend
replacement, and advanced rendering remain planned or research work.

Rust, TOML, and text blocks in this document are planned schema/API contracts or
pseudocode unless a status row says the current crate implements that surface.
They are not compile-tested examples because the corresponding public APIs do
not fully exist yet.

## 1. Current status

| Named system | Status | Evidence and limit |
|---|---|---|
| Penumbra renderer architecture | Adopted architecture; Partial/Transitional implementation | The production direction is depth-prepass clustered Forward+ behind Meridian RHI/render-graph seams. Current code is a direct PBR/shadow/diffuse-IBL smoke path, not full Penumbra. |
| RHI boundary | `ImplementedFoundation`; `Transitional` backend | `meridian_rhi` owns backend-neutral adapter/device/surface config, feature reporting, buffers, textures, render pipeline creation, timestamp-query API, and private `wgpu` handles. Native backend replacement is a future gate. |
| Render graph | `ImplementedFoundation` | `meridian_render_graph` validates resources/passes, access hazards, producer requirements, dependency cycles, topological order, and resource lifetimes. Barriers, aliasing execution, async compute, and graph visualization are planned. |
| Pipeline warmup | `ImplementedFoundation` | `meridian_renderer` rejects missing required startup pipelines and records runtime creation attempts. Shipping release builds must not create new pipelines during active traversal. |
| PBR smoke path | Transitional foundation | Current code supports mesh/material metadata, base color, normal, metallic-roughness textures, material parameters, camera/object uniforms, direct sun, cascaded raster shadows, and diffuse irradiance IBL. |
| Diffuse IBL | `ImplementedFoundation` | `EnvironmentLight` validates diffuse intensity. `GpuEnvironmentMap` owns a cube texture and bounded face uploads. The material shader samples pre-convolved irradiance for diffuse ambient lighting. |
| Specular IBL | Planned | Prefiltered specular environment sampling and BRDF integration LUT are not implemented and must not be implied by diffuse IBL. |
| Pass timing | `ImplementedFoundation` | `WP-PEN-007` instruments clear, bootstrap, shadow-depth, and indexed-PBR encoding with typed CPU/GPU outcomes and bounded nonblocking readback. Future production render-graph execution must reuse this contract for every pass. |
| Visible captures | `ImplementedFoundation` | `WP-PEN-008` provides request-only three-slot asynchronous readback, typed unavailable/failure outcomes, BGRA/RGBA 8-bit sRGB normalization, PNG/metadata output outside the RHI, and source-derived native capture evidence. The MS-01 native surface was occluded/unavailable, so its visible PNG is explicitly offscreen and proves neither presentation nor visual quality. |
| Visibility buffer, virtual geometry, advanced GI, ray tracing | Research | Later RG-PEN-001 after MS-05 gates. They are not the MS-04/MS-07 production direction. |
| Isobar weather/atmosphere input | Planned consumer/producer contracts | Rendering will consume optical weather fields from Isobar. No weather solver, wind field, precipitation, wetness, or volumetric cloud path is implemented. |
| Basalt terrain/large-world geometry input | Partial precursors; terrain planned | 64-bit world positions, cells, rebasing, and deterministic streaming precursors exist. Production terrain/vegetation geometry remains planned/scaffold. |
| Torsant fire/fluid/thermal input | Planned/research | Renderer-facing heat, smoke, flame, and fluid visualization contracts are planned. No Torsant simulation or rendering integration exists. |
| Meridian Shader Language and canonical ShaderIr | Adopted architecture; Planned | Current WGSL/Naga foundations remain. No Meridian language frontend, shared text/graph IR, target module, or generated binding system exists. |
| First-class 2D Penumbra path | Adopted architecture; Planned | No sprite/tile/shape renderer, 2D batching, pixel-policy path, or 2D-specific evidence exists. |
| Project custom passes and renderer paths | Planned | Development/test extension contracts are specified below; no unrestricted backend escape or production custom-path SDK exists. |

## 2. Requirements and adopted decisions

Stable requirement IDs:

| ID | Requirement | Status |
|---|---|---|
| REQ-PEN-001 | Penumbra is one Meridian-owned GPU-driven renderer. Clustered Forward+ is the adopted production shading path; current direct PBR code is foundation evidence, not completion. | Adopted architecture; Partial/Transitional implementation |
| REQ-RHI-001 | Public rendering APIs use Meridian descriptors and generational handles; backend types remain private. | `ImplementedFoundation` |
| REQ-RHI-002 | Feature selection uses declared capabilities plus measured profiles, not support bits alone. | Planned/partial |
| REQ-RHI-003 | Native backend work follows `RG-RHI-001`: Metal only after MS-07 and stable RHI evidence; Vulkan and Direct3D 12 only after mature Metal/common-RHI differential gates. wgpu remains available. | Planned |
| REQ-PEN-002 | Every claimed production renderer pass has CPU timing and optional GPU timing or explicit unsupported reason. | `ImplementedFoundation` for current high-level RHI passes; complete render-graph coverage planned |
| REQ-PEN-003 | Visual quality claims require visible captures; occluded structural smoke proves construction only. | `ImplementedFoundation` capture contract; no production visual-quality claim |
| REQ-PEN-004 | Diffuse irradiance IBL and specular IBL are separate work packages with separate evidence. | Diffuse implemented foundation; specular planned |
| REQ-PEN-005 | Renderer-adjacent named systems use the uniform subsystem template before adding runtime work, resources, or feature-pack behavior. | Adopted spec process |
| REQ-PEN-006 | Isobar and Torsant visual media use one path-independent Penumbra source, residency, lighting, temporal, compositing, and downgrade contract. | Adopted planned contract under `PRG-REL-001`; no implementation evidence |

Adopted decisions are `ADR-0004` through `ADR-0007`. A successor promotion,
Forward+ disposition after promotion, and each native-backend production entry
require later evidence-backed ADRs; no nonexistent future ADR path is treated as
an adopted decision.

## 3. Context

Meridian must render a dark but readable rural forest, dense vegetation,
weather/fog/atmosphere, editor viewports, native app UI surfaces, and later XR
and advanced simulation. Rendering decisions must remain backend-neutral at
public boundaries while the initial implementation may continue using `wgpu`
privately. The renderer must treat weather, terrain, fire, fluids, thermal
state, and procedural content as path-independent data contracts rather than
hard-coded route-specific effects.

## 4. Goals and non-goals

Goals:

- Hide backend graphics types behind Meridian descriptors and handles.
- Use one render graph for resources, passes, dependencies, barriers,
  lifetimes, timing, memory, and debug labels.
- Ship Penumbra as a clustered Forward+ opening-forest renderer with depth
  prepass, clustered lights, direct PBR materials, cascaded shadows, diffuse
  IBL, temporal-history foundations, and pass timing.
- Keep diffuse IBL intact while adding specular IBL later as a bounded package.
- Consume shared named-system snapshots from Isobar, Basalt, and Torsant without
  giving the renderer authority over their source data.
- Provide beginner presets and expert access to shader sources, graph passes,
  timings, memory, captures, resource residency, and fallbacks.
- Support dedicated first-class 2D extraction, batching, ordering, lighting hooks, and mixed-view composition without requiring hidden 3D scene cost.
- Allow trusted project-defined passes and experimental renderer paths through explicit resource, capability, lifetime, fallback, and evidence contracts.
- Support zero-cost-disabled advanced features.

Non-goals:

- Do not expose `wgpu`, Vulkan, Metal, Direct3D, or native backend handles in
  game-facing APIs.
- Do not claim a full renderer from smoke tests.
- Do not require handwritten shaders for normal material authoring.
- Do not make ray tracing, virtual geometry, visibility buffers, frame
  generation, vendor upscalers, Torsant simulation, or full Isobar weather
  required for the opening slice.
- Do not claim fixed frame budgets until PEN-B01/PEN-B02 and hardware records are
  measured.
- Do not replace the current diffuse irradiance IBL requirement with specular
  IBL work; they are separate.

## 5. Ownership and crate boundaries

| Crate or tool | Owns | Must not own |
|---|---|---|
| `meridian-rhi` | Backend adapter/device/surface, private backend handles, GPU resources, queue writes, pipeline creation, timestamps, device/surface recovery | Scene extraction, material authoring policy, world streaming decisions |
| `meridian-render-graph` | Renderer-independent graph declaration, validation, pass ordering, resource lifetimes | Backend resource allocation, shader compilation, gameplay scheduling |
| `meridian-renderer` | Cameras, snapshots, visual resources, lights, shadows, material contracts, pipeline warmup, upload planning, Penumbra passes | `wgpu` public leakage, source asset import, editor UI toolkit, Isobar/Basalt/Torsant authority |
| `meridian-shader-tools` | Shader manifests, validation, reflection, future IR/cache keys | Runtime gameplay policy |
| Meridian Shader Language / `SHD` | Text frontend, material-graph lowering, canonical ShaderIr, reflection, source maps, target requests | Renderer-path ownership, backend device access, gameplay policy |
| First-class 2D / `TWO` | 2D source/document and product acceptance contracts | GPU resources or backend submission; Penumbra owns those paths |
| `meridian-isobar` / Isobar | Atmosphere, wind, precipitation, visibility, optical-field snapshots | Renderer resource allocation |
| `meridian-basalt` / Basalt | Terrain/vegetation/large-world source contracts and geometry snapshots | Rendering backend internals |
| future `meridian-alluvium` / Alluvium | Visual geometry/material/volume/LOD/impostor source artifacts and provenance | GPU resources, visibility, lighting, temporal state, or render-pass ownership |
| future Torsant crates | Fire/fluid/thermal fields, source terms, solver diagnostics | Penumbra graph ownership |
| Editor renderer tools | Viewports, captures, graph inspection, material editor, debug overlays | Runtime-only hidden renderer state |
| Asset/build tools | Texture/mesh/material artifacts, shader variants, warmup manifests | Device-specific runtime state |

Invalid dependencies:

- Renderer-independent crates must not depend on `meridian-rhi`.
- Runtime game crates must not depend on private backend types.
- Headless server builds must not depend on renderer crates unless a
  compile-time feature explicitly asks for render validation tools.
- Alluvium, Isobar, Basalt, and Torsant source-data crates must not allocate renderer
  textures or issue render-graph passes directly.
- UI rendering surfaces may register render-graph nodes through public
  descriptors, not by borrowing backend internals.

## 6. Public contracts

### 6.1 RHI handles and capabilities

Required public object model:

```rust
pub struct DeviceHandle(pub u64);
pub struct QueueHandle(pub u64);
pub struct BufferHandle(pub u64);
pub struct TextureHandle(pub u64);
pub struct TextureViewHandle(pub u64);
pub struct SamplerHandle(pub u64);
pub struct ShaderModuleHandle(pub u64);
pub struct PipelineLayoutHandle(pub u64);
pub struct GraphicsPipelineHandle(pub u64);
pub struct ComputePipelineHandle(pub u64);
pub struct RayPipelineHandle(pub u64);
pub struct BindGroupHandle(pub u64);
pub struct CommandEncoderHandle(pub u64);
pub struct TimelineFenceHandle(pub u64);
pub struct SurfaceHandle(pub u64);
pub struct AccelerationStructureHandle(pub u64);
pub struct QueryPoolHandle(pub u64);
```

Capability table:

```rust
pub struct GpuCaps {
    pub api: GraphicsApi,
    pub shader_model: ShaderModel,
    pub bindless_tier: BindlessTier,
    pub mesh_shader: bool,
    pub ray_query: bool,
    pub ray_pipeline: bool,
    pub sparse_resources: bool,
    pub async_compute_queues: u8,
    pub timestamp_queries: bool,
    pub subgroup_width_min: u32,
    pub subgroup_width_max: u32,
    pub max_sampled_textures: u32,
}
```

Feature selection must use `GpuCaps` plus measured workload profiles and native
surface outcomes. Capability bits alone do not prove support for a quality tier.

### 6.2 Pass-timing foundation

The current RHI implements `TimingFrameId`, `PassTimingLabel`,
`PassTimingSample`, `GpuTimingOutcome`, `GpuTimingFailure`, and
`TimingDiagnostics`. Explicit `begin_timing_frame`/`end_timing_frame` scopes
correlate multiple submissions; unscoped single-pass calls receive an automatic
frame ID. `poll_pass_timings` and `take_pass_timing` never wait for GPU
completion. The former blocking `take_last_gpu_duration` surface is deprecated
and retained only as a nonblocking compatibility shim.

CPU duration covers render-pass encoding only. GPU queries use eight independent
generation-tracked slots, submit-time map callbacks, zeroed resolve buffers,
bounded result retention, and `PollType::Poll`. Samples carry frame ID,
submission ID, pass label, slot generation internally, CPU duration, and one of:

- `Measured(Duration)`;
- `NotRequested`;
- `UnsupportedCapability`;
- `UnsupportedPlatform(GpuTimingFailure)`;
- `Inconclusive(GpuTimingFailure)`.

Zero, equal, reversed, stale, overflowing, failed-map, saturated, or device-lost
data cannot become measured time. Invalid Metal legacy timestamp data changes
GPU timing to `UnsupportedPlatform(MetalTimestampDataInvalid)` for the remaining
RHI lifetime while CPU timing continues. This is a foundation contract, not
complete production graph coverage: every future render-graph executor pass must
reuse it before satisfying `REQ-PEN-002`.

### 6.3 Path-independent Penumbra contracts

The following names are planned public-contract direction, not implemented Rust
types in this pass:

```text
RendererPathId
RendererPathDescriptor { id, maturity, required_capabilities, supported_profiles }
GpuCapabilityProfile { portable_core, gpu_driven, advanced, vr }
RenderView
GpuSceneSnapshot
VisibilityOutput
IndirectCommandStream
LightSnapshot
ShadowSnapshot
EnvironmentalFieldSnapshot
TemporalHistorySnapshot
MaterialSource -> MaterialIr -> renderer-path lowering
ShaderIr -> WGSL during the wgpu era -> future native target lowering
CustomShaderCompatibilityManifest {
  renderer_paths,
  required_capabilities,
  fallback_policy,
  trust_level,
}
CustomRenderPassDescriptor {
  id, trust_level, renderer_paths, queue,
  declared_reads, declared_writes, lifetime_classes,
  capabilities, budget, ordering_constraints,
  fallback_policy, diagnostics, source_provenance,
}
CustomRendererPathDescriptor {
  id, maturity, development_only, shared_system_requirements,
  capability_profiles, material_lowering, fallback_path,
  workloads, promotion_gate,
}
```

One high-level material is authored once. Paths may lower `MaterialIr`
differently but cannot require duplicated artist materials. `ShaderIr` owns
reflection, binding generation, specialization, source maps, and pipeline-cache
identity. WGSL remains a valid current backend language, not Meridian's
permanent canonical source.

The [Meridian Shader Language](MERIDIAN_SHADER_LANGUAGE_SPEC.md) text frontend and material graphs lower into this same `ShaderIr`. The working language name is written in full; `MSL` is not a Meridian abbreviation. Naga and other mature compiler foundations may remain behind the target boundary indefinitely unless measured evidence justifies replacement.

Project-defined passes are denied ambient backend access. Their resources, queue, ordering, lifetime, capabilities, trust, budgets, diagnostics, and fallback are declared before graph compilation. Experimental renderer paths reuse the path-independent GPU scene, materials, lighting, streaming, histories, profiling, and capture systems; they remain development/test/benchmark paths until a full promotion ADR passes.

### 6.4 Render graph

```rust
pub struct PassNode {
    pub name: StringId,
    pub queue: QueueClass,
    pub reads: Vec<ResourceUse>,
    pub writes: Vec<ResourceUse>,
    pub callback: PassExecutor,
}
```

Compiler stages:

1. Validate names, resources, and queue compatibility.
2. Build dependency graph from explicit and resource edges.
3. Cull unused passes when safe.
4. Detect cycles.
5. Topologically order passes.
6. Infer transitions and barriers.
7. Compute transient lifetimes.
8. Alias compatible transient resources.
9. Schedule async compute only where legal and measured useful.
10. Emit command batches, timing scopes, memory labels, and debug metadata.

### 6.5 Scene and named-system snapshots

The existing immutable render snapshot contract is preserved:

```rust
pub struct RenderSnapshot {
    pub frame_id: u64,
    pub fixed_tick: u64,
    pub interpolation_alpha: f32,
    pub instances: Vec<RenderInstance>,
}

pub struct RenderInstance {
    pub id: RenderInstanceId,
    pub previous_transform: Transform,
    pub transform: Transform,
    pub bounds_radius: f32,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub flags: RenderFlags,
}
```

Named-system inputs use immutable snapshots with persistent source IDs and
generational runtime handles:

- Isobar publishes atmosphere, wind, visibility, precipitation, and optical
  scattering summaries.
- Basalt publishes terrain cells, vegetation instances, large-world transforms,
  and geometry residency requests.
- Torsant publishes heat, flame, smoke, fluid-surface, and thermal-material
  source fields when the optional pack is enabled.
- Alluvium publishes immutable visual artifacts, material facets, volumes,
  placement/LOD data, and provenance during build or bounded runtime-safe
  evaluation. Penumbra never invokes editor/compiler internals or treats GPU
  resources as generated-source authority.

Simulation publishes immutable snapshots. The render thread never mutates ECS,
physics, weather, terrain, thermal, world source, or gameplay state.

### 6.5.1 Shared environmental participating media

Planned contract; no volume allocator, renderer path, or producer integration is
implemented:

```text
ParticipatingMediaSourceSnapshot {
  source_id: PersistentId,
  owner: Isobar | Torsant | OtherRegisteredProducer,
  epoch: u64,
  bounds: WorldBounds,
  representation: Analytic | TiledField | SparseVolume | DenseLocal,
  extinction: Optional<VolumeFieldRef<Scalar>>,
  scattering: Optional<VolumeFieldRef<Vec3>>,
  emission: Optional<VolumeFieldRef<Vec3>>,
  phase: PhaseFunctionSummary,
  velocity: Optional<VolumeFieldRef<Vec3>>,
  temporal_validity: TemporalValidity,
  quality_priority: QualityPriority,
  budget_class: RuntimeBudgetClass,
}
```

Isobar owns the meaning and evolution of fog, cloud, and atmosphere source
fields. Torsant owns the meaning and evolution of smoke, steam, flame-emission,
and heat-haze source fields. Penumbra owns renderer-side representation,
residency, lighting injection, shadowing, empty-space skipping, temporal history,
reconstruction, compositing, and fallback. It may fuse compatible sources to
avoid duplicate volume hierarchies or raymarches, but fusion never merges or
advances simulation authority.

Every source declares bounds, epoch, validity, priority, and budget class.
Absent optional sources are supported. Incompatible representations remain
separate passes with an explicit diagnostic; they cannot silently allocate a
second persistent hierarchy or bypass the render graph. Temporal history is
invalidated from source epochs and transforms, not renderer guesses.

`RuntimeCostManifest` may predict source pages, pipelines, history, lighting,
and compositing demand. Penumbra reconciles that prediction with observed
runtime evidence and never treats a prediction as resource or quality authority.

### 6.6 Material facets

Visual material is one facet of a conceptual material asset. The renderer owns
only the visual facet and references other facets by ID.

Alluvium may author and cook the high-level material source and causal
weathering inputs. Penumbra lowers only the visual facet; Cairn, audio, Isobar,
and Torsant consume their own typed facets rather than inferring them from
rendered appearance.

```toml
[material.visual]
base_color = [0.62, 0.58, 0.52, 1.0]
metallic = 0.0
roughness = 0.83
normal = "asset:materials/wet_soil#normal"
metallic_roughness = "asset:materials/wet_soil#mr"

[material.physical]
density = 1800.0
friction = 0.8
restitution = 0.05

[material.environmental]
wetness_response = "soil_capillary"
thermal_profile = "cool_wet_soil"

[material.acoustic]
absorption = "porous_wet_soil"
```

A server that needs collision must not load visual textures because the same
conceptual material has physical data.

### 6.7 First-class 2D path

Penumbra owns a dedicated 2D path described by [the 2D specification](TWO_DIMENSIONAL_ENGINE_SPEC.md): 2D cameras, stable layer/order keys, sprite/atlas and tile-map extraction, shapes, particles, batching, clipping, 2D lighting hooks, pixel-aware scaling, diagnostics, and explicit mixed 2D/3D composition. A 2D-only profile does not instantiate 3D visibility, mesh, shadow, material, or temporal systems unless selected by project data. Meridian UI remains a separate semantic/display-list authority even when composited by Penumbra.

## 7. Penumbra production direction

Penumbra is the named production renderer architecture. Its adopted direction is
clustered Forward+ with depth prepass. Current code is only a partial,
transitional foundation on that path.

Ordered baseline:

```text
CPU publishes immutable render snapshot
-> named-system snapshots are latched by epoch
-> resource residency resolves required visual facets
-> upload planner emits changed instance/material/mesh writes
-> depth prepass for opaque geometry
-> clustered light assignment
-> cascaded shadow map passes
-> opaque Forward+ PBR shading
-> vegetation specialized shading path
-> terrain/decals/transparent passes
-> sky/atmosphere/fog and weather optical integration
-> optional Torsant visual fields when enabled
-> postprocessing and temporal resolve
-> UI/compositor
-> present
```

Reasons for Forward+ baseline:

- It handles many local lights without a heavy G-buffer baseline.
- It is compatible with transparency, vegetation, MSAA, and XR evolution better
  than a deferred-only baseline.
- It evolves naturally from the current direct material smoke path.
- It keeps visibility-buffer rendering available as a later measured research
  alternative.

## 8. Image-based lighting

`ImplementedFoundation`:

- one pre-convolved diffuse irradiance cube;
- validated diffuse intensity;
- bounded per-face/mip uploads;
- group-3 material binding in the current shader;
- diffuse ambient term in the direct PBR path.

Planned work:

- prefiltered specular environment cube;
- BRDF integration LUT;
- roughness-driven mip selection;
- fallback resources for missing specular data;
- validation that diffuse-only, specular-enabled, and missing-environment
  scenes produce stable diagnostics.

The specular IBL package remains separate from the closed pass-timing
foundation. Neither package implies the other.

## 9. Uniform subsystem template

Every renderer-adjacent named system must fill this template before adding
runtime tasks, render resources, package chunks, or public APIs.

| Field | Required content |
|---|---|
| Stable ID prefix | Short prefix used for requirements, diagnostics, fixtures, and workload rows. |
| Scope | What the named system owns and explicitly does not own. |
| Current implementation status | Implemented, Partial, Transitional, Planned, Research, Deferred, or Unsupported with evidence. |
| Source authority | Authoritative source documents, schemas, imported assets, generated artifacts, and cache rules. |
| Public data contracts | Persistent IDs, generational handles, immutable snapshots, descriptor types, unit systems, and compatibility rules. |
| Dependency direction | Allowed dependencies, invalid dependencies, and adapter boundaries. |
| Runtime pipeline | Ordered producer/consumer steps and declared latency classes. |
| Threading and memory | Task classes, allocation ownership, snapshot lifetime, synchronization, and device/backend implications. |
| Persistence and migration | Save/package/source compatibility, unknown-field behavior, cache regeneration, and version IDs. |
| Capability gates | Feature tiers, disabled behavior, unsupported diagnostics, and platform/backend requirements. |
| Native evidence gates | Which platforms/hardware/backends must run, what native smoke proves, and what it does not prove. |
| Security/provenance | Untrusted inputs, external solver/tool permissions, license/provenance records, and redaction. |
| Accessibility | Player/editor controls, diagnostic readability, photosensitivity/motion/audio implications, and recovery paths. |
| Diagnostics/recovery | Stable diagnostic codes, failure modes, stale data behavior, corruption handling, and fallback surfaces. |
| Tests and fixtures | Unit, integration, differential, fuzz, deterministic, recovery, and disabled-pack tests. |
| Workloads | References to the shared workload set below plus system-specific fixtures. |
| Research/ADR gates | Alternatives, corpus, metrics, owner, review milestone, stable seam, and losing-prototype archive plan. |

Template text block:

```text
NamedSystemSpec {
  id_prefix:
  status:
  owns:
  does_not_own:
  current_evidence:
  planned_contracts:
  invalid_edges:
  runtime_pipeline:
  snapshot_lifetime:
  capability_tiers:
  native_gates:
  security_and_provenance:
  accessibility:
  diagnostics_and_recovery:
  validation:
  workloads:
  intended_adrs:
}
```

## 10. Permanent Penumbra workload references

These stable workloads are definition-only and uncalibrated until executable
corpora and evidence records exist. Their strict report contract lives in
[`specs/registry/workloads.json`](registry/workloads.json).

| ID | Workload |
|---|---|
| PEN-B01 | Midnight forest |
| PEN-B02 | Dense grass field |
| PEN-B03 | Flashlight through alpha-tested foliage |
| PEN-B04 | Redacted generated AMI interior with many local lights |
| PEN-B05 | Heavy Isobar storm |
| PEN-B06 | Large Basalt terrain vista |
| PEN-B07 | Torsant fire, fluids, heat, and smoke |
| PEN-B08 | Rapid camera rotation |
| PEN-B09 | High-speed traversal |
| PEN-B10 | Large world-streaming transition |
| PEN-B11 | Low-memory stress |
| PEN-B12 | Shader and pipeline compilation stress |
| PEN-B13 | Shadow-heavy scene |
| PEN-B14 | Transparency-heavy scene |
| PEN-B15 | Temporal-disocclusion test |
| PEN-B16 | VR-oriented stereo test, deferred until XR work begins |

PEN-B04 is a reproducible generated surrogate with connected interiors, mixed
practical light temperatures, many local shadowed lights, partial failures,
varied materials, decals/transparency, and indoor/outdoor streaming. It contains
no AMI logos, documents, narrative text, route data, or proprietary assets; only
a redacted private authority/corpus hash may be recorded.

## 11. Threading, memory, and lifetime

- Simulation owns ECS and physics writes during fixed update.
- Render extraction copies stable data into immutable snapshots.
- Named-system snapshots are latched by epoch and released before reclamation.
- Upload planning diffs snapshots and commits CPU slot state only after GPU
  writes succeed.
- RHI resources are device-owned and have resurrection metadata sufficient for
  device loss recovery.
- Render-graph transient resources may be aliased only after compiler lifetime
  proof.
- Persistent world/source data is never borrowed directly by GPU work.
- Renderer worker tasks may prepare CPU data but cannot mutate gameplay state.
- Temporal histories are explicitly invalidated for camera cuts, teleport,
  origin rebase, FOV change, dynamic-resolution change, surface resize, and
  incompatible material/reprojection changes.

Double or triple buffering is selected per platform by latency policy and
measured presentation behavior. The policy must record the frame-latency target
and observed present mode.

## 12. Persistence, versioning, and compatibility

Persistent render data appears in:

- visual material schemas;
- mesh/texture artifacts;
- shader manifests and shader cache keys;
- package chunk manifests;
- benchmark captures;
- manually requested captures later imported by [Marquee](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md); Marquee never drives the game or renderer to discover shots;
- editor viewport layouts and debug presets.

Shader cache keys must include source or graph hash, compiler version, target
backend and shader model, feature flags, specialization constants, material
layout, pipeline layout, and relevant driver compatibility data.

Old shader caches are disposable. Source materials, shader graphs, and named
system source documents are authoritative.

## 13. Editor, CLI, MCP, and workflows

Beginner workflow:

1. User opens a world viewport.
2. Editor selects a render preset from target hardware and accessibility
   preferences.
3. Material presets and visual nodes cover ordinary surfaces.
4. Play uses prewarmed pipelines and shows plain-language diagnostics for
   missing textures, unsupported features, or low-memory fallbacks.
5. Export includes only required renderer assets and variants.

Expert panels expose graph passes, timing, memory, resource residency, warmup
state, shader reflection, light clusters, shadow cascades, IBL state, named
system field overlays, captures, and backend capability rows.

Penumbra owns rendering and capture completion. Marquee is a post-1.0 consumer of manually requested, approved captures and cannot launch, navigate, stage, or control the running game through renderer APIs.

Planned command names are semantic surfaces, not current runnable command
evidence:

| Domain | Commands |
|---|---|
| Render graph | `inspect`, `validate`, `dump`, `diff` |
| Shader | `validate`, `reflect`, `compile`, `cache-key`, `warmup-plan` |
| Capture | `start`, `stop`, `export`, `compare` |
| Benchmarks | `run`, `summarize`, `compare`, `gate` |

MCP tools must use the same command registry and permission checks as the CLI
and editor. GPU capture, package extraction, network upload, and benchmark
publication require explicit capabilities.

## 14. Diagnostics, recovery, and security

Required diagnostics include unsupported GPU feature, unsupported render tier,
missing required pipeline, runtime pipeline creation attempt, shader validation
failure, missing material texture, invalid color space, invalid normal map,
diffuse IBL missing/invalid, specular IBL requested but unavailable, pass timing
unavailable, device lost, surface lost/outdated/occluded, resource budget
exceeded, transient aliasing conflict, stale render snapshot, stale named-system
snapshot, and invalid draw bounds or buffer usage.

Device loss recovery:

1. Stop submitting new frames.
2. Preserve editor/source/game state.
3. Record recent render graph, resource budget, device capabilities, backend,
   named-system epochs, and pending uploads.
4. Recreate adapter/device/surface when safe.
5. Recreate pipelines from warmup manifest.
6. Recreate resources from CPU/cache/package metadata.
7. Resume preview if validation succeeds; otherwise abort the preview and keep
   source state editable.

Security:

- Shaders from packages, mods, or plugins are untrusted input until validated.
- Native backend escapes require elevated capability.
- GPU capture files may contain asset data and must follow export permissions. Import into Marquee requires DAT provenance, project approval, spoiler/embargo classification, and a content hash; a staged render remains labeled as staged.
- Runtime package data must not compile arbitrary shaders unless the package is
  trusted and policy permits.
- Named-system imports are untrusted and must validate length, count, depth,
  path, hash, version, and unit fields before allocation or rendering.

## 15. Capability tiers and native gates

| Tier | Renderer behavior |
|---|---|
| No-render/headless | Renderer crates, shaders, textures, GPU tasks, and visual facets are omitted. |
| Minimal client | Forward+ baseline with conservative textures, shadows, fog, and no optional RT/upscaler/vendor features. |
| Standard client | Forward+ with measured clustered lights, cascaded shadows, diffuse IBL, temporal resolve, and target-specific presets. |
| High client | Higher shadow/vegetation/volumetric quality and future specular IBL when measured. |
| Research | Visibility buffer, virtual geometry, dynamic GI portfolio, RT shadows/GI, native backends, frame generation, and advanced temporal experiments. |

Native gates:

- platform priority is Apple Silicon; Linux/Steam Deck; Windows NVIDIA;
  Windows AMD; Intel graphics; then Windows on ARM;
- support is capability-profile driven, not inferred from a GPU name;
- timestamp queries may be unsupported and must report an explicit outcome;
- current Apple Metal legacy timestamp data may be invalid despite advertised
  capability; invalid results disable GPU timing for that RHI lifetime while
  preserving CPU pass timing;
- occluded/minimized/lost surfaces are valid structural outcomes only;
- native Metal begins only after MS-07, stable RHI contracts, complete
  opening-slice evidence, and `RG-RHI-001`;
- native Vulkan and Direct3D 12 begin only after mature Metal and common-RHI
  differential rendering, benchmark, recovery, divergence, and maintenance
  gates; wgpu remains available throughout;
- disabled features contribute no pipeline variants, resident resources, graph
  passes, package chunks, background workers, or recurring frame cost.

Priorities are stable frame pacing, image quality, memory, scalability, then
compatibility. Metrics include distributions and lows, CPU/GPU time,
shader/pipeline/upload/streaming stalls, memory, churn, overdraw,
shadow/volumetric cost, temporal stability, and device bottlenecks.

## 16. Research gates

| Decision | Alternatives | Gate |
|---|---|---|
| Production shading | clustered Forward+, deferred, hybrid, visibility buffer, other measured candidates | Forward+ is adopted for MS-04 through MS-10 unless a successor passes `RG-PEN-001`. |
| Virtual geometry | conventional LODs, meshlets, custom hierarchy, sparse pages | Separate research after MS-05; static/rigid and animated/deformed evidence remain distinct. |
| GI portfolio | probes, radiance cache, screen-space GI, baked probes/lightmaps, RTGI, path-traced reference | Separate research after MS-05; no mandatory vendor path. |
| Shadows | cascades, clipmaps, virtual shadow maps, cached local shadows, RT shadows | Cascaded directional shadows are current foundation; broader portfolio is measured later. |
| Shader lowering | `ShaderIr` to WGSL now; future native target lowering | WGSL remains supported current output, not permanent canonical authoring source. |
| Upscaling | native TAA, Meridian upscaler, DLSS, FSR, XeSS, supersampling, dynamic resolution | Capability and platform policy selected; no single-vendor dependency. |
| Native backend | wgpu, native Metal, native Vulkan, native Direct3D 12 | `RG-RHI-001`: Metal after MS-07 and stable RHI; Vulkan/D3D12 after mature Metal/common-RHI parity. |

`RG-PEN-001` opens only after MS-05. Every candidate uses shared workloads and
preregisters workload/platform-specific material-improvement thresholds before
collecting results. Promotion requires complete feature parity, equal-or-better
artistic results, improved capabilities, stability across supported tiers and
native backends, acceptable shader/pipeline behavior, no frame-time, memory,
debugging, or lower-tier regression, and reasonable maintenance cost. A small
gain such as roughly two percent cannot justify substantial complexity.

Experimental paths remain development, benchmark, test, or renderer-debug only
until promotion. If a successor is promoted, Forward+ retention, fallback, or
removal requires another ADR; v0.5 promises neither permanent retention nor
removal. A successor is not required for 1.0.

## 17. Tests, benchmarks, and acceptance evidence

Required tests:

- render-graph invalid access, missing producer, cycle, and lifetime tests;
- shader manifest/reflection tests;
- Meridian Shader Language text/graph-to-IR, source-map, capability, security, and target differential tests when `WP-SHD-*` activates;
- custom-pass undeclared access, lifetime, ordering, capability, budget, trust, fallback, and device-loss tests;
- 2D layer/order, atlas/pixel policy, batching/overdraw, mixed composition, and stripped 2D-only profile tests when `WP-TWO-001` activates;
- pipeline warmup duplicate/missing/runtime-creation tests;
- camera projection and frustum tests;
- material parameter and texture color-space validation;
- diffuse IBL intensity and cube upload validation;
- snapshot duplicate-ID and stale-frame rejection;
- named-system snapshot epoch/stale-data rejection;
- shared participating-media source validation, history invalidation, compatible
  fusion, incompatible-source diagnostics, budget downgrade, and producer
  absence tests;
- proof that enabled Isobar/Torsant sources do not create duplicate undeclared
  persistent volume hierarchies or runtime pipeline creation during a recorded
  prewarmed traversal;
- upload planner capacity and rollback tests;
- device/surface lost recovery tests where automation permits;
- headless no-render feature exclusion tests.

Required benchmarks and captures use PEN-B01 through PEN-B15 as relevant to the
claimed feature and profile. PEN-B16 activates only with XR work. Each report
records hardware, OS, backend, driver, renderer path, settings, resolution,
upscaler, CPU/GPU distributions and lows, memory, stalls, churn, overdraw,
shadow/volumetric cost, temporal stability, bottlenecks, visual differences,
artifacts, missing features, warmup/cache state, checkpoint/BuildId, corpus and
build hashes, and raw evidence.

Acceptance evidence for Penumbra baseline:

- textured PBR mesh and terrain from built assets;
- depth prepass and Forward+ light clustering;
- cascaded shadows with visible capture;
- diffuse IBL visible capture;
- pass timings for major graph nodes;
- structured fallback when pass timings are unsupported;
- no backend types in public game-facing APIs.

## 18. Delivery mapping

| Gate | Penumbra result |
|---|---|
| MS-01 | `MS-01` passed qualification on GitHub Actions run `29452928922`: observable current RHI/render graph, correlated `WP-PEN-007` pass timing, explicit offscreen-visible `WP-PEN-008` capture with an occluded/unavailable surface result, and passing Linux/Windows/macOS headless smoke. The capture is not presented-surface or production-quality evidence. |
| MS-04 | GPU-driven clustered Forward+ foundation, shared scene/material/light/environment/temporal systems, debugging, and profiling. |
| MS-05 | Representative measured forest renderer; opens `RG-PEN-001`. |
| MS-06 | Private Project Meridian prototype consumes published contracts after Editor Alpha and forest gates. |
| MS-07 | Complete production opening-slice renderer evidence; earliest native-backend research unlock. |
| MS-08 | Native Metal only after stable RHI and `RG-RHI-001`; wgpu retained. |
| MS-08 | Selected `WP-SHD-001` and `WP-TWO-001` foundations integrate only through Penumbra/RHI contracts. |
| MS-09 | Native Vulkan/Direct3D 12 only after mature Metal/common-RHI gates; `WP-SHD-002` and selected XR work may activate; XR may activate PEN-B16. |
| MS-10 | Declared renderer profiles pass release qualification and compatibility. |
| Post-1.0 | `PRG-REL-001` may optimize and competitively validate shared environmental media only after MS-10 and its independent entry gates; it cannot retroactively satisfy a milestone. |

## 19. Examples

End-to-end example:

1. A source tree asset imports into visual mesh, material, collision, and
   thumbnail facets.
2. The editor viewport requests the visual facet for the active world cell.
3. Asset runtime resolves platform texture variants and mesh artifacts.
4. Basalt publishes terrain/vegetation geometry snapshots for the visible cells.
5. Isobar publishes wind/visibility snapshots for the same epoch.
6. Simulation publishes a render snapshot for fixed tick `N` and render frame
   `F`.
7. Upload tracker emits changed instance writes.
8. Render graph executes depth prepass, cluster assignment, shadow pass,
   Forward+ opaque shading, fog, UI, and present.
9. Diagnostics attach pass timings, named-system epochs, and resource residency
   to the frame record.

Failure/recovery example:

1. RHI reports device loss and stops new submissions.
2. The editor preserves source-world edits and unsaved material graph changes.
3. Renderer crash report captures device caps, backend, pass list, recent frame
   IDs, resource budgets, named-system epochs, and pending uploads.
4. RHI recreates device and surface.
5. Pipeline warmup rebuilds declared required pipelines.
6. Resources are resurrected from package/cache CPU metadata.
7. If any required resource cannot be restored, the viewport is disabled with
   diagnostics while source editing remains available.

Performance-debug example:

- per-pass CPU/GPU time;
- depth prepass and shadow cascade costs;
- clustered light counts per tile;
- vegetation draw and overdraw heatmaps;
- texture residency misses;
- diffuse IBL binding state;
- pipeline warmup status;
- surface outcome and timestamp availability;
- Isobar/Basalt/Torsant snapshot age and disabled-pack state.

Acceptable fixes include earlier prefetch, changed shadow update policy, lower
vegetation tier, altered pass order, or reduced light influence ranges. Each
fix must include before/after captures and benchmark records; it must not claim
general superiority beyond the measured scene and hardware.
