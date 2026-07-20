# Meridian Master Specification

version 0.5 · 2026-07-18 · Normative suite index

Meridian is a general-purpose engine for games and interactive applications. Penumbra is its Meridian-owned renderer. The Alluvium Engine is its adopted procedural world-authoring and asset-generation architecture. Marquee is its adopted post-1.0 promotional-material authoring and local-export architecture. `PRG-REL-001` is its deferred post-1.0 program for seeking and proving workload-specific performance and quality leadership without making a permanent superiority promise. Project Meridian is the first proving game, maintained in the separate private `bybrooklyn/project-meridian` repository and never a dependency of this engine repository.

## 1. Authority order

Use the following order when statements conflict:

1. the v0.5 owning subsystem specification and adopted ADR;
2. [the migration and contradiction register](SPEC_MIGRATION_AND_CONTRADICTIONS.md);
3. [the delivery roadmap](DELIVERY_ROADMAP.md);
4. [the implementation-planning specification](IMPLEMENTATION_PLANNING_SPEC.md);
5. machine-readable metadata under [`specs/registry/`](registry/);
6. root [PLANNING.md](../PLANNING.md) for the active bounded package and current evidence;
7. the private [Project Meridian creative suite](https://github.com/bybrooklyn/project-meridian/tree/main/docs) for creative decisions only;
8. migration ledgers for historical rationale;
9. code and evidence as proof of current behavior, not automatic permanent architecture.

Normative changes update the owning spec, ADR when required, registries, migration record, validation, and then PLANNING. Conflicts are never resolved silently.

## 2. How to use the suite

Read [principles](PRINCIPLES_AND_SCOPE.md), [repository architecture](REPOSITORY_AND_CRATE_ARCHITECTURE.md), [delivery roadmap](DELIVERY_ROADMAP.md), [implementation planning](IMPLEMENTATION_PLANNING_SPEC.md), the owning subsystem spec, and [validation](TESTING_BENCHMARKS_AND_VALIDATION.md). Read [API examples](API_AND_FILE_FORMAT_EXAMPLES.md) for illustrative contract shapes and [research](RESEARCH_AND_ALGORITHM_DECISIONS.md) for intentionally open choices. Use canonical [ADRs](../docs/architecture/decisions/README.md) for adopted decisions and [PLANNING.md](../PLANNING.md) for the next package.

## 3. Separate maturity axes

Documentation maturity:

- `Draft`: incomplete or unresolved ownership;
- `ArchitectureComplete`: coherent boundaries and contracts, but research or delivery detail may remain;
- `ResearchReady`: open algorithms have preregistered gates, corpus, metrics, owner, and decision rule;
- `ImplementationReady`: bounded packages, failures, tests, and acceptance evidence are defined;
- `VerifiedCurrent`: implementation claims were reconciled against fresh repository evidence.

Implementation maturity:

- `Implemented`, `ImplementedFoundation`, `StructuralSmoke`, `Partial`, `Transitional`, `Scaffold`, `Planned`, `Research`, `Deferred`, `Unsupported`.

Evidence status:

- `Pass`, `Fail`, `NotRun`, `UnsupportedCapability`, `UnsupportedPlatform`, `Occluded`, `Redacted`, `Waived`, `Stale`, `Inconclusive`.

`REQ-GOV-001`: these axes MUST NOT be collapsed. Documentation completeness never means runtime implementation. `REQ-GOV-002`: scaffolds, marker types, definitions, and structural smokes MUST NOT be promoted beyond their actual boundary.

## 4. Stable identifiers

The suite uses `REQ-<DOMAIN>-NNN`, `WP-<DOMAIN>-NNN`, `RG-<DOMAIN>-NNN`, `EV-<DOMAIN>-YYYYMMDD-NNN`, `MS-00` through `MS-10`, `PRG-<DOMAIN>-NNN`, `VAL-<DOMAIN>-NNN`, `DEP-<DOMAIN>-NNN`, `PEN-B01` through `PEN-B16`, `WVR-<DOMAIN>-NNN`, and `ADR-NNNN`.

The 37 current domains are `CORE`, `GOV`, `RUN`, `RHI`, `PEN`, `UI`, `EDT`, `DAT`, `PHY`, `GAM`, `PRJ`, `AUD`, `ISO`, `BAS`, `VEG`, `PRC`, `TOR`, `DCC`, `BLD`, `VCS`, `SYN`, `XR`, `NET`, `MOD`, `AGT`, `SEC`, `REL`, `ANI`, `NAV`, `FWK`, `TWO`, `SHD`, `MDL`, `COL`, `WRL`, `INT`, and `PRM`.

`WP-*` records deliver MS-00 through MS-10. `PRG-*` records are post-1.0 research or product programs and cannot satisfy, block, or promote an MS milestone. `VAL-*` records identify public generic and private-consumer proving projects; all begin `DefinitionOnly` and `Uncalibrated`. `DEP-*` records govern strategic external foundations and their exit evidence. Domain codes are governance identifiers, not mandatory runtime module, type, or source names.

Full prose remains in Markdown. Registries provide typed identity, ownership, status, and traceability metadata.

## 5. Product invariants

- `REQ-CORE-001`: Meridian MUST be usable without Blender, shader code, Cargo knowledge, VCS knowledge, AI, a cloud account, or a hosted Meridian service.
- `REQ-CORE-002`: expert workflows MUST remain available through typed Rust APIs, CLI commands, schemas, build events, and diagnostics.
- `REQ-CORE-003`: disabled optional packs MUST add no tasks, threads, listeners, GPU resources, allocations, panels, dependencies, or package chunks.
- `REQ-CORE-004`: engine crates MUST NOT depend on Project Meridian code or content.
- `REQ-BLD-002`: projects MUST pin exact compatible, separately managed
  development-toolchain components; component updates MUST be verified,
  recoverable, license-tracked, and unable to silently change a project pin.
- `REQ-DAT-001`: authoritative editable data MUST be schema-defined, versioned, inspectable, recoverable, mergeable, and distinct from derived caches.
- `REQ-AGT-001`: editor UI, CLI, Rust tools, MCP, and agents MUST share typed command/query semantics; AI has no privileged backdoor.
- `REQ-REL-001`: performance, quality, compatibility, and competitor claims MUST be reproducible, scoped, and calibrated.
- `REQ-REL-002`: competitive claims MUST bind exact versions, matched corpus and
  feature parity, raw evidence, scope, expiry, renewal, and retraction.
- `REQ-REL-003`: iso-quality performance, iso-cost quality, and
  matched-workflow throughput MUST remain separate claim classes with
  structural/reference/perceptual evidence as applicable.
- `REQ-REL-004`: first-use stalls, lower-tier regressions, accessibility,
  recovery, portability, debugging, and maintenance MUST remain visible in any
  competitive decision.
- `REQ-SEC-001`: security, provenance, accessibility, diagnostics, undo, and recovery are architecture concerns rather than release polish.
- `REQ-GOV-003`: every milestone MUST end in a user-visible result and evidence; placeholders cannot complete it.
- Meridian's user-facing creator environment is one application named **Meridian**. Editor, IDE, modeler, graph, debugger, profiler, and project workflows are modes and workspaces inside it; bounded helper processes and CLI tools remain allowed.
- Rust is the first gameplay implementation and extension language. Optional Luau follows stable Rust contracts and has zero cost when absent.
- Wavefront is the audio system. Collective is the optional all-in-one online-services system. Neither requires a Meridian-hosted account or cloud service for offline creation.
- First-class 2D, the native beginner-friendly modeler, general animation, navigation infrastructure, and selected reusable framework foundations are planned pre-1.0 capabilities at their registered milestones. Advanced family completion, facial production, MMO worlds, integrity systems, Marquee promotional exports, and competitive performance/quality leadership are post-1.0 programs.

## 6. Coordinated documents

| Subject | Authority |
|---|---|
| Principles and product boundaries | [PRINCIPLES_AND_SCOPE.md](PRINCIPLES_AND_SCOPE.md) |
| Repository, crates, optional packs | [REPOSITORY_AND_CRATE_ARCHITECTURE.md](REPOSITORY_AND_CRATE_ARCHITECTURE.md) |
| Runtime, tasks, platform | [CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md](CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md) |
| Penumbra renderer | [RENDERING_AND_GRAPHICS_SPEC.md](RENDERING_AND_GRAPHICS_SPEC.md) |
| Penumbra risks | [PENUMBRA_RISK_REGISTER.md](PENUMBRA_RISK_REGISTER.md) |
| Isobar weather and atmosphere | [ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md](ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md) |
| Basalt terrain and large-world geometry | [BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md](BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md) |
| Torsant fire, fluids, and thermal simulation | [TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md](TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md) |
| Vegetation ecosystem | [VEGETATION_ECOSYSTEM_SPEC.md](VEGETATION_ECOSYSTEM_SPEC.md) |
| Meridian UI and Creator Editor | [EDITOR_AND_MERIDIAN_UI_SPEC.md](EDITOR_AND_MERIDIAN_UI_SPEC.md) |
| Accessibility and Ponder | [ACCESSIBILITY_DOCUMENTATION_AND_PONDER_SPEC.md](ACCESSIBILITY_DOCUMENTATION_AND_PONDER_SPEC.md) |
| Assets, worlds, saves, packages | [ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) |
| Cairn physics | [CAIRN_PHYSICS_SPEC.md](CAIRN_PHYSICS_SPEC.md) |
| Wavefront audio, music, acoustics, and voice-device boundary | [AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md](AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) |
| Gameplay, Rust-first APIs, optional Luau | [GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) |
| Official gameplay frameworks | [OFFICIAL_GAMEPLAY_FRAMEWORKS_SPEC.md](OFFICIAL_GAMEPLAY_FRAMEWORKS_SPEC.md) |
| Artus body motion, cinematics, and facial systems | [ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md](ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md) |
| Navigation and game-AI infrastructure boundary | [NAVIGATION_AND_AI_INFRASTRUCTURE_SPEC.md](NAVIGATION_AND_AI_INFRASTRUCTURE_SPEC.md) |
| First-class 2D | [TWO_DIMENSIONAL_ENGINE_SPEC.md](TWO_DIMENSIONAL_ENGINE_SPEC.md) |
| Meridian Shader Language and ShaderIr | [MERIDIAN_SHADER_LANGUAGE_SPEC.md](MERIDIAN_SHADER_LANGUAGE_SPEC.md) |
| Native modeling and optional DCC interchange | [NATIVE_MODELING_AND_DCC_SPEC.md](NATIVE_MODELING_AND_DCC_SPEC.md) |
| The Alluvium Engine procedural authoring | [PROCEDURAL_AUTHORING_SPEC.md](PROCEDURAL_AUTHORING_SPEC.md) |
| Marquee promotional media and local export | [MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md) |
| Competitive performance and quality leadership | [COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md](COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md) |
| Build and IDE | [CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) |
| VCS and synchronization | [VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md](VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md) |
| Multiplayer and providers | [MULTIPLAYER_AND_SERVER_SPEC.md](MULTIPLAYER_AND_SERVER_SPEC.md) |
| Collective online services | [COLLECTIVE_ONLINE_SERVICES_SPEC.md](COLLECTIVE_ONLINE_SERVICES_SPEC.md) |
| Distributed worlds and MMO research | [DISTRIBUTED_WORLDS_AND_MMO_SPEC.md](DISTRIBUTED_WORLDS_AND_MMO_SPEC.md) |
| Integrity, anti-cheat, and moderation boundary | [INTEGRITY_ANTI_CHEAT_AND_MODERATION_SPEC.md](INTEGRITY_ANTI_CHEAT_AND_MODERATION_SPEC.md) |
| Modding | [MODDING_AND_COMMUNITY_LIBRARY_SPEC.md](MODDING_AND_COMMUNITY_LIBRARY_SPEC.md) |
| Agents and MCP | [AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md) |
| XR | [VR_XR_AND_INTERACTION_SPEC.md](VR_XR_AND_INTERACTION_SPEC.md) |
| Security and updates | [SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) |
| Delivery | [DELIVERY_ROADMAP.md](DELIVERY_ROADMAP.md) |
| Implementation planning | [IMPLEMENTATION_PLANNING_SPEC.md](IMPLEMENTATION_PLANNING_SPEC.md) |
| Project Meridian prototype | [PROJECT_MERIDIAN_PROTOTYPE_PLAN.md](PROJECT_MERIDIAN_PROTOTYPE_PLAN.md) |
| Project Meridian opening slice | [PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md) |
| Testing and benchmarks | [TESTING_BENCHMARKS_AND_VALIDATION.md](TESTING_BENCHMARKS_AND_VALIDATION.md) |
| Research gates | [RESEARCH_AND_ALGORITHM_DECISIONS.md](RESEARCH_AND_ALGORITHM_DECISIONS.md) |
| Migration | [SPEC_MIGRATION_AND_CONTRADICTIONS.md](SPEC_MIGRATION_AND_CONTRADICTIONS.md) |

## 7. Stable architecture seams

Long-lived contracts are Meridian-owned descriptors, generational handles, persistent IDs, schemas, capabilities, immutable snapshots, typed commands, and evidence records. Third-party types stop at adapters.

Runtime domains separate platform/event, fixed simulation, presentation, render submission, audio callback, asynchronous IO/build, and optional networking. Their clocks are distinct types. Cross-domain mutation uses commands and declared barriers; render/audio/worker consumers receive immutable snapshots.

## 8. Penumbra and platform direction

Penumbra is one Meridian-owned GPU-driven renderer. Clustered Forward+ is its adopted production shading path, while the current implementation remains Partial/Transitional. Shared render graph, GPU scene, material/shader IR, visibility, streaming, lighting, temporal, profiling, and resource systems are path-independent. MS-01 adds bounded asynchronous RGBA8 capture and typed surface outcomes; its passing offscreen artifact is not evidence of presentation or production image quality.

Isobar fog/cloud/atmosphere and Torsant smoke/steam/flame source fields use the
planned path-independent Penumbra `ParticipatingMediaSourceSnapshot` boundary.
Penumbra owns renderer residency, lighting, shadow, temporal, compositing, and
downgrade resources; source systems retain simulation authority. No such shared
volume implementation currently exists.

`meridian-rhi` remains the abstraction and wgpu the current production backend. Native Metal begins only after MS-07, stable RHI review, and `RG-RHI-001`; Vulkan and Direct3D 12 follow only after mature Metal/common-RHI evidence. wgpu remains available through the transition.

The Meridian Shader Language text frontend and material graphs lower into one canonical `ShaderIr`. WGSL remains a current target language, not permanent high-level source authority. Custom project paths and passes use capability-, trust-, lifetime-, and fallback-declared Penumbra contracts; they do not receive unrestricted backend access.

## 9. Data and private-game boundary

MS-01 implements one provisional public JSON fixture family, transactional
visual/collision import, deterministic uncompressed `.meridian` packaging,
worker-backed compiled-cell activation, and minimal schema-aware save recovery.
These are `ImplementedFoundation` evidence, not final source/package/save
formats or production streaming policy.

Project Meridian's creative documents, route, lore, art, assets, and game code remain private. The engine repository retains sanitized integration requirements, generated benchmark contracts, and private source hashes only. `PEN-B04` is a redacted generated interior surrogate; it contains no AMI logos, documents, narrative text, or proprietary assets.

Only the private creative repository defines what AMI means. Engine records use
the redacted benchmark identifier without expanding or importing its lore.

## 9.1 Alluvium direction

Alluvium is a core editor/build system for typed recipes, spatial fields,
incremental evaluation, generated identity, overrides, provenance, licensing,
and cooking. It authors source and derived artifacts; Basalt, vegetation,
Isobar, Torsant, Cairn, Penumbra, audio, navigation, streaming, and saves retain
live runtime authority. Projects using baked outputs only do not ship an
Alluvium runtime evaluator. Private game recipes and creative constraints remain
outside this repository.

Post-1.0 Alluvium work may author coherent combustion/fluid facets and a
calibrated `RuntimeCostManifest`; runtime systems retain live authority and
observed traces remain evidence rather than authored truth.

## 9.2 General-purpose creator platform

Meridian is one application with project, editor, IDE, modeler, shader/material, animation, profiler, build, version-control, and play/debug workspaces. The native modeler is core because first-party creation must not require Blender; Blender and other DCC applications remain optional expert companions. Model source uses stable mesh-element identity and explicit topology lineage so Alluvium generation, overrides, undo, materials, collision, and later animation can survive edits or report recoverable conflicts.

The normative Meridian UI 1.0 contract is
[EDITOR_AND_MERIDIAN_UI_SPEC.md](EDITOR_AND_MERIDIAN_UI_SPEC.md) and ADR-0028.
It fixes the website palette, two-row native shell, design metrics,
Meridian-owned retained interfaces, typed interaction, workspace persistence,
accessibility, and capability-truth rules. `WP-UI-002` through `WP-UI-005`
implement the framework sequentially; `WP-EDT-002` composes the MS-03
production shell and World workspace; `WP-EDT-003` composes later domain
workspaces without promoting their implementation maturity.

Gameplay begins with Rust modules, generated reflection, typed commands/events, save/headless hooks, and isolated Play-session rebuild/restart when safe native reload is unavailable. Luau is the first optional embedded high-level language only after these contracts stabilize.

First-class 2D shares runtime, data, build, UI, audio, and diagnostics where semantics match, while Penumbra and Cairn provide dedicated 2D paths. Navigation owns traversability and queries; gameplay/frameworks own decisions. Wavefront owns audio capture, DSP, mixing, acoustics, and device output; Collective owns optional voice-session policy and the broader modular online-service surface.

Post-1.0 `PRG-*` authorities preserve ambitious work without distorting MS-00 through MS-10. Advanced model creation, facial/performance capture, all six framework families, hosted-scale Collective work, distributed worlds, advanced integrity, native VCS storage, optional compiler internalization, Marquee promotional exports, and competitive performance/quality leadership require their own entry gates and evidence. `PRG-REL-001` may begin only after MS-10; it cannot guarantee global superiority and permits only scoped, reproducible, expiring claims. Marquee imports manually supplied approved captures and source media, produces local files only, and permits optional AI solely for non-authoritative text and analysis suggestions.

## 10. Change and completion control

A normative change identifies requirements, old/new behavior, compatibility, security, accessibility, performance, optionality, tests/research evidence, ADR impact, and migration disposition. `meridian-spec check` validates the suite before Rust CI.

The suite is complete at MS-00 when required documents, registries, schemas, ADRs, migration mappings, links, fixtures, and audits pass. The engine is not complete merely because its documentation is coherent. Product completion proceeds through the evidence gates in [DELIVERY_ROADMAP.md](DELIVERY_ROADMAP.md).
