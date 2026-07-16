# Research and Algorithm Decisions

[Master](MERIDIAN_MASTER_SPEC.md) · [Roadmap](DELIVERY_ROADMAP.md) · [Competitive quality](COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md) · [ADRs](../docs/architecture/decisions/README.md) · [Registry](registry/research-gates.json)

version 0.5 · 2026-07-15 · Normative research policy

Documentation maturity: `ResearchReady`. This document owns research method,
candidate framing, primary-source anchors, and decision output. Typed gate
identity/status lives in [`research-gates.json`](registry/research-gates.json).

## 1. Method and maturity

ResearchReady means a decision is intentionally open but has:

- stable `RG-*` identity, owner, opening dependency, and decision deadline;
- candidates broad enough to avoid a predetermined winner;
- stable public seam that prototypes cannot bypass;
- generated/public/redacted corpus and immutable hashes;
- named hardware/capability profiles, settings, cache/warmup state, and tooling;
- correctness, visual/artistic, performance-distribution, memory, recovery,
  security, accessibility, provenance, and maintenance metrics as applicable;
- preregistered material-improvement threshold before results are collected;
- decision rule, independent review, ADR output, migration, and losing-prototype
  archive/cleanup plan.

Research does not authorize production dependencies, private-content copies,
API leakage, a shipping claim, or a maturity promotion. An adopted architecture
remains in force until a gate passes and an ADR changes it.

## 2. Adopted baselines outside research

- Meridian UI is the permanent shared retained UI architecture; any egui shell
  is transitional (`ADR-0009`).
- Penumbra is one Meridian-owned renderer with clustered Forward+ as its
  production shading architecture (`ADR-0004`).
- Shared renderer systems and artist material authority are path-independent
  (`ADR-0005`, `ADR-0007`).
- `meridian-rhi` owns public graphics contracts; wgpu is the current production
  backend implementation (`ADR-0006`).
- Cairn owns long-term physics contracts; the current Rapier wrapper is
  transitional (`ADR-0010`).
- Luau is the first high-level gameplay runtime (`ADR-0012`).
- Isobar, Basalt, vegetation, and Torsant have separate authority (`ADR-0008`).
- Alluvium is the adopted procedural world-authoring and asset-generation
  architecture while implementation remains planned (`ADR-0017`).
- Optional capability packs have zero disabled cost (`ADR-0014`).
- Meridian is one user-facing application, Rust gameplay is implemented before optional Luau, Wavefront and Collective have separate authority, 2D is first-class, the native modeler is core, and text/material graphs share one ShaderIr (`ADR-0018` through `ADR-0023`).
- `MS-00` through `MS-10` remain the 1.0 delivery authority. Advanced ambitions use post-1.0 `PRG-*` records and cannot promote or block milestones (`ADR-0024`).
- Shared environmental media, sparse/multirate scheduling, surface-fluid
  handoff, material facets, and cook-time cost semantics use adopted stable
  boundaries while algorithms remain research (`ADR-0026`).
- Competitive performance and quality leadership is a post-1.0 evidence program,
  not a current claim or 1.0 gate (`ADR-0027`).

Research may improve these decisions but cannot describe them as undecided.

## 3. Registered gates

| Gate | Opens | Decision boundary |
|---|---|---|
| RG-PEN-001 | after MS-05 | Penumbra successor renderer-path candidates versus production Forward+ |
| RG-RHI-001 | after MS-07 | native Metal entry, then later Vulkan/Direct3D 12 common-RHI parity and maintenance |
| RG-UI-001 | after MS-02 | Meridian UI display-list renderer implementation |
| RG-PHY-001 | after MS-06 | Cairn solver/layout/determinism portfolio behind owned contracts |
| RG-ISO-001 | after MS-05 | Isobar weather/atmosphere algorithms by capability tier |
| RG-BAS-001 | after MS-05 | Basalt conventional/meshlet/hierarchy/sparse geometry portfolio |
| RG-TOR-001 | after MS-07 | specialized optional fire/fluid/thermal/smoke solver portfolios |
| RG-PRC-001 | after MS-01 | Alluvium evaluator representation and scalar/SIMD/GPU kernel portfolio |
| RG-PRC-002 | after MS-05 | measured dependency replacement and deep kernel ownership |
| RG-SEC-001 | after MS-07 | release cryptography and key-management implementation |
| RG-REL-001 | after MS-10 | matched competitive baseline, perceptual method, claim expiry, and automation policy |
| RG-PRM-001 | after MS-10 | Marquee local media, audio, and PDF adapter selection |

Unregistered experiments cannot be cited as roadmap evidence.

## 4. RG-PEN-001 — Penumbra successor research

Forward+ is the production baseline and a full implementation target, not a
throwaway prototype. `RG-PEN-001` starts only after the representative forest
renderer passes MS-05.

Candidates remain open: visibility-buffer, deferred, hybrid, compute/material
binning, or another Meridian-owned path may be evaluated. Every path consumes
the same `RenderView`, `GpuSceneSnapshot`, material/shader IR, light/shadow/
environment snapshots, visibility/indirect streams, render graph, histories,
streaming/residency, profiling, capture, and resource/pipeline systems.

Required workload coverage is PEN-B01 through PEN-B15 as applicable. A
promotion requires complete feature parity, equal-or-better artistic results,
meaningful new capabilities or material measured advantage, stability across
portable/GPU-driven/advanced tiers and native backends, acceptable shader and
pipeline behavior, no material frame-time/memory/debugging regression, and a
sustainable maintenance burden. A roughly two-percent isolated win is not
enough for substantial complexity; high-end-only wins cannot regress lower
supported tiers.

Experimental paths are development/benchmark/test/debug only. Promotion
requires an ADR. Forward+ retention, fallback, or removal after promotion is a
separate later ADR. A successor is not required for 1.0.

Primary anchors:

- Harada, McKee, and Yang, [Forward+: Bringing Deferred Lighting to the Next Level](https://diglib.eg.org/items/1db2c4c6-dcab-42ea-8c0a-6805d781759e/full), for terminology and the clustered Forward+ family;
- Burns and Hunt, [The Visibility Buffer](https://jcgt.org/published/0002/02/04/), as one successor candidate family;
- Majercik et al., [Dynamic Diffuse Global Illumination](https://jcgt.org/published/0008/02/01/), for later GI research;
- Bitterli et al., [ReSTIR](https://cwyman.org/papers/sig20_ReSTIR.pdf), for later sampling research, not current production commitment.

## 5. RG-RHI-001 — native backend entry

The current production backend remains wgpu. Capabilities are queried and
recorded; feature bits do not constitute performance or quality evidence.

Native sequence:

1. MS-07 and stable RHI contracts pass.
2. Metal entry is preregistered and implemented while wgpu remains available.
3. Common RHI and Metal pass differential images, permanent workloads,
   synchronization/lifetime tests, device/surface recovery, tooling, backend
   divergence, provenance, staffing, and maintenance review.
4. Vulkan and Direct3D 12 may begin independently only after step 3.

Platform priority is Apple Silicon; Linux/Steam Deck; Windows NVIDIA; Windows
AMD; Intel graphics; Windows on ARM. Capability profiles, not named devices,
determine supported behavior.

Primary anchors:

- current [wgpu feature documentation](https://docs.rs/wgpu/30.0.0/wgpu/struct.Features.html);
- official [Apple Metal capability tables](https://developer.apple.com/metal/capabilities/);
- official [Direct3D feature levels](https://learn.microsoft.com/en-us/windows/win32/direct3d11/overviews-direct3d-11-devices-downlevel-intro);
- official [Vulkan specification registry](https://registry.khronos.org/vulkan/).

These sources define capabilities, not Meridian support claims or redistribution
rights.

## 6. UI, physics, and data research

`RG-UI-001` compares Penumbra display-list rendering, selected 2D GPU paths, and
conservative CPU-tessellation/native-GPU paths on accessible editor/runtime UI.
Stable seams are `DisplayList`, glyph/image handles, semantic tree, focus/event
model, and recovery. Metrics include correctness, text quality, latency,
CPU/GPU/memory, cache behavior, device recovery, platform support, maintenance,
and license. Sources include [AccessKit](https://accesskit.dev/),
[Vello](https://github.com/linebender/vello), and official candidate docs.

`RG-PHY-001` begins only after exact Rapier/Box2D provenance, an unmodified
baseline, differential fixtures, and Cairn-owned descriptors/handles/snapshots.
It compares solver/layout changes and measured determinism envelopes without a
universal bit-identical promise. Sources include official
[Rapier determinism guidance](https://rapier.rs/docs/user_guides/templates/determinism),
[Rapier simulation structures](https://rapier.rs/docs/user_guides/rust/simulation_structures/),
and [Box2D releases](https://box2d.org/documentation/md_release__notes__v310.html).

Meridian ECS replacement remains a future gate to register after MS-07. Current
bevy_ecs use stays behind persistent IDs, schemas, queries, commands, barriers,
extraction, save, and network seams. Replacement requires measured total product
value, not ownership preference alone.

Content hash, compression, package partition, world-cell sizing, and save
compaction remain format/profile decisions. Candidates use deterministic build,
range/streaming, decompression-limit, corruption, migration, patch-size, and
platform evidence. No algorithm name becomes an unversioned format assumption.

## 7. Alluvium, Isobar, Basalt, Torsant, and Wavefront research

`RG-PRC-001` keeps `ProceduralRecipe`, `FieldValue`, `EvaluationRequest`,
`EvaluationResult`, `GeneratedObjectId`, and `ProvenanceManifest` stable while
comparing strict reference, optimized scalar, architecture SIMD, GPU,
tiled/sparse, and isolated-worker execution. Required evidence covers
structural correctness, determinism level, preview/clean/dirty latency,
throughput, CPU/GPU transfer, memory, cancellation, diagnostics, platform
coverage, and maintenance on PEN-B01/B02/B05/B06/B10/B11. A strict reference
path is mandatory; acceleration cannot redefine recipe semantics.

`RG-PRC-002` defaults to retaining permissively licensed foundations behind
Meridian seams. Replacement or deep custom ownership requires a preregistered
material product benefit, representative workload evidence, license/provenance
review, migration and compatibility cost, debugging/tooling impact, sustained
owner capacity, and an ADR. Branding or a trivial isolated gain is rejected.

`RG-ISO-001` selects authored/simple/regional/advanced atmosphere and weather
algorithms by visual stability, deterministic envelope, authorability, field
coupling, CPU/GPU/memory cost, fallback, and PEN-B05 evidence.

`RG-BAS-001` compares conventional LOD, meshlet, hierarchy, and sparse-page
approaches by precision, streaming, memory, deformation, authoring, recovery,
and PEN-B02/PEN-B06/PEN-B09/PEN-B10/PEN-B11 evidence. It is distinct from
renderer-path selection.

`RG-TOR-001` chooses specialized per-effect portfolios; it never seeks one
universal solver. Candidate references include Stam/Bridson fluid methods,
shallow-water methods, APIC-like particle-grid methods, XPBD, and physically
based or authored fire/thermal models. Metrics include stability, conservation
where meaningful, visual/reference error, controls, CPU/GPU/memory, streaming,
determinism, persistence/network fit, accessibility, fallback, and disabled
cost. PEN-B07 remains definition-only until this work activates.

Isobar and Torsant candidates additionally preserve the adopted
`EnvironmentalTilePolicy`, `SurfaceFluidHandoff`, and
`ParticipatingMediaSourceSnapshot` seams. Every candidate reports simulation
and presentation clocks, hierarchy/residency, influence horizon, work and memory
quotas, update debt, first-use activation, downgrade, and CPU/simple fallback.
Two-way coupling is a separate candidate with explicit stability, latency,
persistence, and recovery evidence; it is never assumed from a one-way snapshot
prototype.

Hybrid acoustics, deformables, snow/granular, and specialized cross-system
solver coupling require separately registered gates before implementation.
Alluvium buildings, ecosystems, terrain, material, and weathering work follows
its registered package chain and the owning runtime subsystem gates. Sharing
evaluation or field infrastructure does not imply a universal graph or solver.

Primary candidate references include:

- Jos Stam's [fluid publications](https://www.josstam.com/publications);
- Bridson's [fluid simulation resources](https://www.cs.ubc.ca/~rbridson/fluidsimulation/);
- Jiang et al., [APIC](https://www.cs.ucr.edu/~craigs/papers/2015-apic/paper.pdf);
- Macklin et al., [XPBD](https://matthias-research.github.io/pages/publications/XPBD.pdf);
- Nguyen et al., [physically based fire](https://graphics.stanford.edu/papers/fire-sg02/).

## 8. Gameplay, modeler, animation, navigation, 2D, and shaders

Rust gameplay implementation under `WP-GAM-001` is not a language-selection experiment. Research focuses on stable API/schema generation, module isolation, platform-safe Play rebuild/restart, state migration, debugging, and performance. Luau research begins only under `WP-GAM-002` after these contracts stabilize; it covers VM isolation, allocator/instruction limits, module/bytecode policy, debugger/profiler, state migration, deterministic APIs, and exact compatibility. Primary sources are the [Luau sandbox](https://luau.org/sandbox/), [embedding API](https://luau.org/api/), [performance guidance](https://luau.org/performance/), and [compatibility documentation](https://luau.org/compatibility/).

Native modeling research keeps editable model documents, stable element IDs, topology lineage, semantic operations, modifiers, undo/recovery, and explicit interchange loss stable. Candidate mesh kernels, robust predicates, booleans, UV/LOD methods, and optional DCC bridges compare correctness, degeneracy behavior, source recovery, accessibility, performance, licensing, and maintenance. Advanced sculpting/retopology/hair/cloth remains `PRG-MDL-001`, not a hidden 1.0 gate.

Animation research keeps skeleton/clip/graph/event/pose contracts stable while comparing compression, retargeting, blend/IK execution, CPU/GPU deformation, streaming, and rollback behavior. Facial/performance-capture work is `PRG-ANI-001` and adds biometric privacy/provenance gates.

Navigation research keeps source facets, profiles, artifacts, bounded queries, partial outcomes, streaming epochs, and trace semantics stable while comparing mesh/grid/voxel/flow/hybrid representations and optional accelerators. Gameplay AI decisions remain outside NAV.

First-class 2D research compares dedicated sprite/tile/shape batching, atlas policy, 2D lighting, pixel scaling, and Cairn 2D algorithms on `VAL-TWO-001`. It may share infrastructure but cannot rely on hidden 3D execution or memory.

Shader research keeps the Meridian Shader Language semantics, material-graph lowering, canonical `ShaderIr`, reflection, source maps, capability declarations, and compatibility manifests stable. WGSL/Naga is the current target path. Native target lowering follows RHI gates. Compiler internalization is `PRG-SHD-001` and requires differential correctness, security, backend, performance, and maintenance evidence.

## 9. Ecosystem and trust research

Collective provider research keeps identity/session/social/voice-policy/analytics/moderation contracts provider-neutral, modular, privacy-bound, self-hostable, and offline-safe. No research result authorizes a Meridian-operated service without separate operational, legal, security, moderation, reliability, and funding evidence.

VCS lineage, synchronization models, network transports/providers, mod sandbox,
agent providers/retrieval, XR extensions, and DCC integrations need registered
gates before implementation. Stable seams are already specified; proprietary
SDK availability or popularity is not an adoption decision.

The strategic dependency authority is [`registry/dependency-strategy.json`](registry/dependency-strategy.json). Categories are `InternalizeEarly`, `InternalizeEventually`, and `WrapIndefinitelyUnlessNecessary`. None is a promise to rewrite. Every replacement preregisters product value, representative workloads, compatibility/migration, license/security, debugging, staffing, and maintenance. Existing permissive foundations remain when they are the best evidence-backed choice.

Distributed-world and advanced-integrity research is confined to `PRG-WRL-001` and `PRG-INT-001` after 1.0 and their entry gates. Distributed scale and anti-cheat claims require adversarial/failure/cost/false-positive evidence; neither is inferred from NET, Collective, SEC, or MS-10 completion.

`RG-PRM-001` opens after MS-10 and compares mature local image, video, audio, and PDF adapters behind stable Marquee contracts. Selection requires cross-platform, deterministic, licensing/patent, sandbox, performance/memory, accessibility, maintenance, and escape-path evidence. It authorizes neither custom Meridian codecs nor service publishing and cannot begin implementation without a separate post-1.0 planning review.

`RG-REL-001` opens after MS-10 and the entry gates in
[the competitive specification](COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md).
It selects the initial comparator set, study templates, structural/reference/
blinded perceptual method, claim-expiry policy, and safe automation boundary
behind `CompetitiveBaselineRecord` and `CompetitiveClaim`. It cannot select a
Penumbra, Isobar, Torsant, Alluvium, RHI, or platform algorithm; owning research
gates retain those decisions.

Each study preregisters exact versions, access/license basis, matched public or
appropriately licensed corpus, feature parity, hardware, internal/output
resolution, upscaling/frame-generation/dynamic-resolution state, warmup/cache/
first use, raw distributions, temporal quality, lower tiers, accessibility,
recovery, security, maintenance, material threshold, expiry, and stop rule.
Unfair or impossible parity is `Inconclusive`, not adjusted until Meridian wins.

Official capability anchors reviewed 2026-07-16 include Epic's
[Nanite](https://dev.epicgames.com/documentation/en-US/unreal-engine/nanite-virtualized-geometry-in-unreal-engine)
and [Lumen performance](https://dev.epicgames.com/documentation/en-US/unreal-engine/lumen-performance-guide-for-unreal-engine)
documentation, Unity's [Spatial-Temporal Post-processing](https://docs.unity3d.com/6000.0/Documentation/Manual/urp/stp/stp-upscaler.html)
and [GPU occlusion culling](https://docs.unity3d.com/6000.0/Documentation/Manual/urp/gpu-culling.html)
documentation, and Godot's [pipeline compilation](https://docs.godotengine.org/en/stable/tutorials/performance/pipeline_compilations.html)
documentation. They establish moving capability surfaces only; they do not prove
comparative quality, performance, support, licensing, or a Meridian decision.

`RG-SEC-001` begins from a threat model and compares implementation libraries,
algorithms, thresholds, expiry, key storage, canonicalization, compromise,
rotation, reproducibility, audit maturity, platform support, license, and
operations. Meridian adopts the role/freshness/delegation threat model from the
[TUF specification](https://theupdateframework.github.io/specification/), not an
unreviewed custom cryptosystem.

## 10. Decision output

Every completed gate produces an ADR containing gate ID, candidate revisions,
raw evidence links, corpus/build hashes, hardware/software/capabilities,
preregistered thresholds, metrics/statistics, visual/artistic review,
security/accessibility/provenance/maintenance review, winner and limits,
migration/rollback, and losing-prototype archive. Registries and owning specs
update before PLANNING activates the selected implementation package.
