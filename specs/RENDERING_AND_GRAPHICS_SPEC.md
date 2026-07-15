# Rendering and Graphics Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md)

Status: normative specification, version 0.2, 2026-07-14.

This document defines Meridian rendering ownership, public graphics contracts,
the opening-forest renderer baseline, capability tiers, diagnostics, research
gates, and implementation evidence. It preserves the current diffuse irradiance
IBL foundation as implemented. Prefiltered specular IBL and the BRDF
integration LUT are future renderer work. Pass timing is the immediate next
measurement priority.

Rust and TOML blocks in this document are schema/API contracts or pseudocode
unless their status table says the current crate already implements that
surface. Planned snippets are not compile-tested because the corresponding APIs
do not exist yet.

## 1. Current Status

| Area | Status | Evidence and limit |
|---|---|---|
| RHI boundary | Implemented foundation | `meridian_rhi` owns backend-neutral adapter/device/surface config, feature reporting, buffers, textures, render pipeline creation, timestamp-query API, and private `wgpu` handles. Native backend replacement remains planned. |
| Render graph | Implemented foundation | `meridian_render_graph` validates resources/passes, access hazards, producer requirements, dependency cycles, topological order, and resource lifetimes. Barriers, aliasing execution, async compute, and graph visualization are planned. |
| Pipeline warmup | Implemented foundation | `meridian_renderer` rejects missing required startup pipelines and records runtime creation attempts. Shipping release builds must not create new pipelines during active traversal. |
| PBR smoke path | Transitional foundation | The current path supports mesh/material metadata, base color, normal, metallic-roughness textures, material parameters, camera/object uniforms, direct sun, cascaded raster shadows, and diffuse irradiance IBL. It is not yet the full Forward+ renderer. |
| Diffuse IBL | Implemented foundation | `EnvironmentLight` validates diffuse intensity. `GpuEnvironmentMap` owns a cube texture and bounded face uploads. The material shader samples pre-convolved irradiance for diffuse ambient lighting. |
| Specular IBL | Planned | Prefiltered specular environment sampling and BRDF integration LUT are not implemented and must not be implied by diffuse IBL. |
| Pass timings and visual captures | Immediate next | Frame-level optional GPU duration exists; pass-level CPU/GPU timings and visible capture evidence remain open. |
| Forward+ | Planned baseline | Clustered Forward+ with depth prepass is the opening-forest baseline. Current direct PBR smoke path is a stepping stone. |
| Visibility buffer | Research | Visibility-buffer rendering is a later research gate, not the Phase 2/8 baseline. |

## 2. Context

Meridian must render a dark but readable rural forest, dense vegetation,
dynamic weather and lighting, editor viewports, native app UI surfaces,
headless-free client builds, and later VR/XR and advanced simulation. The
renderer must remain backend-neutral at public boundaries while the initial
implementation may continue using `wgpu` privately.

Rendering decisions must be capability-selected and measured. A feature being
supported by a GPU is not evidence that it is fast enough or good enough for a
given target.

## 3. Goals

- Hide backend graphics types behind Meridian descriptors and handles.
- Use one render graph for resources, passes, dependencies, barriers,
  lifetimes, timing, memory, and debug labels.
- Ship a Forward+ opening-forest renderer with a depth prepass, clustered
  lights, direct PBR materials, cascaded shadows, diffuse IBL, temporal
  history foundations, and measured pass timing.
- Keep diffuse IBL intact while adding specular IBL later as a bounded work
  package.
- Provide a beginner material workflow through presets and visual nodes.
- Provide expert access to shader sources, graph passes, timings, memory,
  GPU captures, resource residency, and fallbacks.
- Support zero-cost-disabled advanced features.

## 4. Non-goals

- Do not expose `wgpu`, Vulkan, Metal, Direct3D, or native backend handles in
  game-facing APIs.
- Do not claim a full renderer from smoke tests.
- Do not require handwritten shaders for normal material authoring.
- Do not make ray tracing, virtual geometry, visibility buffers, frame
  generation, or vendor upscalers required for the opening slice.
- Do not claim fixed frame budgets until B01/B02 and hardware records are
  measured.
- Do not replace the current diffuse irradiance IBL requirement with specular
  IBL work; they are separate.

## 5. Ownership and Crate Boundaries

| Crate or tool | Owns | Must not own |
|---|---|---|
| `meridian-rhi` | Backend adapter/device/surface, private backend handles, GPU resources, queue writes, pipeline creation, timestamps, device/surface recovery | Scene extraction, material authoring policy, world streaming decisions |
| `meridian-render-graph` | Renderer-independent graph declaration, validation, pass ordering, resource lifetimes | Backend resource allocation, shader compilation, gameplay scheduling |
| `meridian-renderer` | Cameras, snapshots, resources, lights, shadows, material contracts, pipeline warmup, upload planning | `wgpu` public leakage, source asset import, editor UI toolkit |
| `meridian-shader-tools` | Shader manifests, validation, reflection, future IR/cache keys | Runtime gameplay policy |
| Editor renderer tools | Viewports, captures, graph inspection, material editor, debug overlays | Runtime-only hidden renderer state |
| Asset/build tools | Texture/mesh/material artifact generation, shader variants, warmup manifests | Device-specific runtime state |

Invalid dependencies:

- Renderer-independent crates must not depend on `meridian-rhi`.
- Runtime game crates must not depend on private backend types.
- Headless server builds must not depend on renderer crates unless a
  compile-time feature explicitly asks for render validation tools.
- UI rendering surfaces may register render-graph nodes through public
  descriptors, not by borrowing backend internals.

## 6. Public Types and API Contracts

### 6.1 RHI Handles and Capabilities

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

Feature selection must use `GpuCaps` plus measured profiles, not capability
bits alone.

### 6.2 Render Graph

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

### 6.3 Scene Snapshot

The existing immutable snapshot contract is preserved:

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

Simulation publishes immutable snapshots. The render thread never mutates ECS,
physics, world source, or gameplay state.

### 6.4 Material Facets

Visual material is one facet of a conceptual material asset. The renderer owns
only the visual facet and references other facets by ID.

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

[material.acoustic]
absorption = "porous_wet_soil"
```

A server that needs collision must not load visual textures because the same
conceptual material has physical data.

## 7. Opening Renderer Baseline

The Phase 2/8 baseline is clustered Forward+ with a depth prepass. It evolves
from the current direct PBR smoke path.

Ordered baseline:

```text
CPU publishes immutable render snapshot
-> resource residency resolves required visual facets
-> upload planner emits changed instance/material/mesh writes
-> depth prepass for opaque geometry
-> clustered light assignment
-> cascaded shadow map passes
-> opaque Forward+ PBR shading
-> vegetation specialized shading path
-> transparent and decal passes
-> sky/atmosphere/fog
-> postprocessing and temporal resolve
-> UI/compositor
-> present
```

Reasons for Forward+ baseline:

- It handles many local lights without a heavy G-buffer baseline.
- It is compatible with transparency, hair-like vegetation, MSAA, and VR
  evolution better than a deferred-only baseline.
- It matches the current direct material smoke path better than replacing it
  with visibility-buffer architecture before vertical-slice evidence exists.
- It keeps visibility-buffer rendering available as a later measured research
  alternative.

## 8. Image-based Lighting

Implemented foundation:

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

The specular IBL package must be separate from pass timing. Pass timing is the
immediate next measurement priority.

## 9. Threading, Memory, and Lifetime

- Simulation owns ECS and physics writes during fixed update.
- Render extraction copies stable data into immutable snapshots.
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

## 10. Persistence, Versioning, and Compatibility

Persistent render data appears in:

- visual material schemas;
- mesh/texture artifacts;
- shader manifests and shader cache keys;
- package chunk manifests;
- benchmark captures;
- editor viewport layouts and debug presets.

Shader cache keys must include:

- source or graph hash;
- compiler version;
- target backend and shader model;
- feature flags;
- specialization constants;
- material layout;
- pipeline layout;
- relevant driver compatibility data.

Old shader caches are disposable. Source materials and shader graphs are
authoritative.

## 11. Editor, CLI, MCP, and Workflows

### 11.1 Beginner Workflow

1. User opens a world viewport.
2. Editor selects a render preset from target hardware and accessibility
   preferences.
3. Material presets and visual nodes cover ordinary surfaces.
4. Play uses prewarmed pipelines and shows plain-language diagnostics for
   missing textures, unsupported features, or low-memory fallbacks.
5. Export includes only required renderer assets and variants.

No beginner workflow requires shader source, graph compiler details, GPU
capture tools, or backend selection.

### 11.2 Expert Workflow

Expert rendering panels expose:

- render graph pass list and dependency edges;
- pass CPU/GPU timings;
- transient resource lifetimes and aliasing;
- GPU memory residency by resource and asset;
- pipeline warmup status and runtime creation attempts;
- shader reflection and variant keys;
- clustered light heatmap;
- overdraw, depth, normals, roughness, material ID, motion vectors, temporal
  confidence, shadow cascades, diffuse IBL, future specular IBL, and streaming
  debug views;
- capture export for external GPU debuggers when platform policy permits.

### 11.3 CLI and MCP Surface

These planned command names are semantic surfaces, not current runnable command
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

## 12. Diagnostics, Failure Recovery, and Security

Required diagnostics:

- unsupported GPU feature;
- unsupported but requested render tier;
- missing required pipeline at runtime entry;
- runtime pipeline creation attempt in release traversal;
- shader validation failure with source span and reflected binding;
- missing material texture or invalid color space;
- normal map not declared linear;
- diffuse IBL missing or invalid;
- specular IBL requested but unavailable;
- pass timing unavailable;
- device lost;
- surface lost/outdated/occluded;
- resource budget exceeded;
- transient aliasing conflict;
- stale render snapshot;
- invalid draw bounds or buffer usage.

Device loss recovery:

1. Stop submitting new frames.
2. Preserve editor/source/game state.
3. Record recent render graph, resource budget, device capabilities, backend,
   and pending uploads.
4. Recreate adapter/device/surface when safe.
5. Recreate pipelines from warmup manifest.
6. Recreate resources from CPU/cache/package metadata.
7. Resume preview if validation succeeds; otherwise abort the preview and keep
   source state editable.

Security:

- Shaders from packages, mods, or plugins are untrusted input until validated.
- Native backend escapes require elevated capability.
- GPU capture files may contain asset data and must follow export permissions.
- Runtime package data must not compile arbitrary shaders unless the package is
  trusted and policy permits.

## 13. Capability Tiers and Zero-cost-disabled Behavior

| Tier | Renderer behavior |
|---|---|
| No-render/headless | Renderer crates, shaders, textures, GPU tasks, and visual facets are omitted. |
| Minimal client | Forward+ baseline with conservative textures, shadows, fog, and no optional RT/upscaler/vendor features. |
| Standard client | Forward+ with measured clustered lights, cascaded shadows, diffuse IBL, temporal resolve, and target-specific presets. |
| High client | Higher shadow/vegetation/volumetric quality and future specular IBL when measured. |
| Research | Visibility buffer, virtual geometry, dynamic GI portfolio, RT shadows/GI, native backends, frame generation, and advanced temporal experiments. |

Disabled features must contribute no pipeline variants, no resident resources,
no graph passes, no package chunks, no background workers, and no recurring
frame cost.

## 14. Algorithm Alternatives and Research Gates

| Decision | Alternatives | Gate |
|---|---|---|
| Opening opaque baseline | clustered Forward+, deferred, hybrid, visibility buffer | Forward+ is selected for Phase 2/8. Re-evaluate visibility buffer in Phase 12 with B01/B02 and vegetation stress scenes. |
| Virtual geometry | conventional LODs, meshlets, custom hierarchy, sparse pages | Phase 12; static/rigid first, animated/deformed separate. |
| GI portfolio | probes, radiance cache, screen-space GI, baked probes/lightmaps, RTGI, path-traced reference | Phase 12; no mandatory vendor path. |
| Shadows | cascades, clipmaps, virtual shadow maps, cached local shadows, RT shadows | Cascaded directional shadows are current foundation; broader portfolio is measured later. |
| Shader source/IR | WGSL, HLSL, custom high-level source, visual graph to IR | Decide after validation, reflection, debugging, translation, and backend evidence. Current WGSL is transitional implementation. |
| Upscaling | native TAA, Meridian upscaler, DLSS, FSR, XeSS, supersampling, dynamic resolution | Capability and platform policy selected; no single-vendor dependency. |

Research gates must name corpus, metrics, owner, deadline phase, API seam, and
archive plan for losing prototypes.

## 15. Tests, Benchmarks, and Acceptance Evidence

Required tests:

- render-graph invalid access, missing producer, cycle, and lifetime tests;
- shader manifest/reflection tests;
- pipeline warmup duplicate/missing/runtime-creation tests;
- camera projection and frustum tests;
- material parameter and texture color-space validation;
- diffuse IBL intensity and cube upload validation;
- snapshot duplicate-ID and stale-frame rejection;
- upload planner capacity and rollback tests;
- device/surface lost recovery tests where automation permits;
- headless no-render feature exclusion tests.

Required benchmarks and captures:

- B01 midnight forest flashlight;
- B02 field horizon sunset;
- synthetic clustered light stress;
- shadow cascade stability and cost;
- diffuse IBL cost and fallback;
- future specular IBL cost and visual reference;
- package/streaming interaction with texture residency;
- pass-level CPU/GPU timing;
- visible captures on supported target hardware.

Acceptance evidence for renderer baseline:

- textured PBR mesh and terrain from built assets;
- depth prepass and Forward+ light clustering;
- cascaded shadows with visible capture;
- diffuse IBL visible capture;
- pass timings for major graph nodes;
- structured fallback when pass timings are unsupported;
- no backend types in public game-facing APIs.

## 16. Phased Implementation

| Phase | Rendering scope |
|---|---|
| Phase 1 | Window, platform, diagnostics, basic RHI setup. |
| Phase 2 | Render graph, camera, mesh/material, basic lighting, first forest viewport, pass timing. |
| Phase 5 | Asset/package integration for renderer artifacts and source-world visual facets. |
| Phase 8 | Opening-forest playable slice with measured render gates. |
| Phase 12 | Virtual geometry, temporal pipeline, dynamic GI portfolio, ray abstraction, visibility-buffer research. |
| Phase 19 | OpenXR timing, stereo, late-latching seams, VR frame pacing. |
| Phase 29 | 1.0 hardening, backend support matrix, long-term shader/material compatibility. |

## 17. End-to-end Example

1. A source tree asset imports into visual mesh, material, collision, and
   thumbnail facets.
2. The editor viewport requests the visual facet for the active world cell.
3. Asset runtime resolves platform texture variants and mesh artifacts.
4. Renderer resource registry validates mesh counts, bounds, material ranges,
   and texture references.
5. Simulation publishes a render snapshot for fixed tick `N` and render frame
   `F`.
6. Upload tracker emits changed instance writes.
7. Render graph executes depth prepass, cluster assignment, shadow pass,
   Forward+ opaque shading, fog, UI, and present.
8. Diagnostics attach pass timings and resource residency to the frame record.

## 18. Failure and Recovery Example

Scenario: device loss during editor preview.

1. RHI reports device loss and stops new submissions.
2. The editor preserves source-world edits and unsaved material graph changes.
3. Renderer crash report captures device caps, backend, pass list, recent frame
   IDs, resource budgets, and pending uploads.
4. RHI recreates device and surface.
5. Pipeline warmup rebuilds declared required pipelines.
6. Resources are resurrected from package/cache CPU metadata.
7. If any required resource cannot be restored, the viewport is disabled with
   diagnostics while source editing remains available.

## 19. Performance-debug Example

Scenario: B01 flashlight scene hitches.

Expert view shows:

- per-pass CPU/GPU time;
- depth prepass and shadow cascade costs;
- clustered light counts per tile;
- vegetation draw and overdraw heatmaps;
- texture residency misses;
- diffuse IBL binding state;
- pipeline warmup status;
- surface outcome and timestamp availability.

Acceptable fixes include earlier prefetch, changed shadow update policy, lower
vegetation tier, altered pass order, or reduced light influence ranges. Each
fix must include before/after captures and benchmark records; it must not claim
general superiority beyond the measured scene and hardware.
