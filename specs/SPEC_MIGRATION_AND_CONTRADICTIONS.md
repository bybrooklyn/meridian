# Meridian Specification Migration and Contradiction Register

[Master index](MERIDIAN_MASTER_SPEC.md) · [Principles](PRINCIPLES_AND_SCOPE.md) · [Heading migration ledger](../docs/migrations/V0_1_DOCUMENT_MIGRATION.md) · [Implementation phases](IMPLEMENTATION_PHASES.md)

Status: normative migration record, version 0.2, 2026-07-14.

This document records how the binding amendment and the 2026-07-14 repository split replaced the seven original version 0.1 root documents. Engine/performance material is consolidated into this suite and benchmark definitions; creative material is rewritten in the separate private `bybrooklyn/project-meridian` repository. The deleted files remain traceable through the heading-level migration ledger. A requirement is never deleted silently: it is preserved, replaced, split, relocated, deferred, or rejected here.

## 1. Authority order

When two statements conflict, use this order:

1. The version 0.2 documents under `specs/`, with this register resolving migration questions.
2. `PLANNING.md` for current implementation status and the active work package only.
3. The private `bybrooklyn/project-meridian` creative specifications for game-specific narrative, art, pacing, and content decisions.
4. The [heading migration ledger](../docs/migrations/V0_1_DOCUMENT_MIGRATION.md) for historical design rationale and provisional benchmark hypotheses from the deleted version 0.1 documents.
5. Current code as evidence of what exists, never as an automatic long-term architecture decision.

The master amendment wins over older documents. A later measured ADR may amend version 0.2 only when it names the affected requirement IDs, records alternatives, supplies evidence, and updates this register.

## 2. Status vocabulary

| Status | Meaning |
|---|---|
| Preserved | Older decision remains normative. |
| Refined | Intent remains, but ownership, API, pipeline, or gate is made concrete. |
| Replaced | New decision supersedes the older one. |
| Transitional | Existing code remains temporarily while a named migration phase replaces it. |
| Deferred | Requirement remains in scope but cannot block the opening vertical slice. |
| Research gate | No production algorithm is selected until competing prototypes are measured. |
| Rejected | Requirement conflicts with product principles and is removed with rationale. |

## 3. Contradiction and duplication matrix

| Subject | Existing document and statement | New or conflicting decision | Winner and reason | Migration action | Updated source of truth |
|---|---|---|---|---|---|
| Specification authority | The original plan listed seven version 0.1 root documents without a conflict hierarchy. | The amendment requires one coordinated engine suite and the repository split requires a separate private creative suite. | Version 0.2 wins; duplicate root documents permit drift and leak the game/engine boundary. | Merge engine/performance material into v0.2, relocate rewritten creative material to `bybrooklyn/project-meridian`, record heading dispositions, and delete all seven root files. | `MERIDIAN_MASTER_SPEC.md` and migration ledger |
| Engine identity | GDD requires a standalone custom engine; older plans primarily describe a game-specific engine. | Meridian is a general-purpose game and interactive-application engine; Project Meridian is its first proving game. | Both are compatible after scope separation. | Preserve custom-engine requirement; prohibit engine crates from depending on the game; classify game-only systems separately. | `PRINCIPLES_AND_SCOPE.md` |
| Ease of use | Older docs imply editor tooling but do not make progressive disclosure an architecture invariant. | Normal users need no Blender, shader code, Cargo, VCS knowledge, AI, or cloud account. | Amendment wins because usability affects formats, APIs, defaults, and process boundaries. | Add beginner/expert workflows and zero-required-external-tool gates to every subsystem. | `PRINCIPLES_AND_SCOPE.md` |
| Advanced systems | Version 0.1 lists weather, simulation, RT, and world systems near the main roadmap. | Expensive systems are optional feature packs and must be zero-cost when disabled. | Amendment wins to control scope and shipping cost. | Add capability registry, dependency pruning, no recurring work/resource tests, and explicit feature-pack ownership. | `REPOSITORY_AND_CRATE_ARCHITECTURE.md` |
| Editor framework | Version 0.1 technical design and `PLANNING.md` use `egui` for the editor foundation. | `egui` is disposable bootstrap scaffolding; Meridian UI is the permanent in-tree framework. | Amendment wins; `egui` cannot define persistence, accessibility, plugin, or runtime UI contracts. | Keep `egui` only in temporary editor targets; migrate panel-by-panel; prohibit `egui` types in project documents or public engine APIs. | `EDITOR_AND_MERIDIAN_UI_SPEC.md` |
| Runtime UI | Version 0.1 describes runtime UI separately from editor UI. | Editor, game UI, and native apps share Meridian UI core, renderer, text, layout, input, semantics, and styling; widget libraries remain separable. | Amendment wins to prevent duplicate UI engines while preserving minimal shipping builds. | Replace unrelated runtime UI architecture with shared core plus `ui-runtime` and `ui-editor` libraries. | `EDITOR_AND_MERIDIAN_UI_SPEC.md` |
| Physics product | Version 0.1 names Rapier as the physics implementation. Current `meridian-physics` wraps `rapier3d`. | Cairn is an in-tree engine family beginning from a provenance-controlled Rapier fork with selected Box2D study/ports; Rapier API compatibility is not a goal. | Amendment wins; current wrapper is transitional evidence, not final ownership. | Freeze wrapper expansion at vertical-slice needs; pin source revisions, archive licenses, establish differential tests, then fork behind Cairn-native handles and descriptors. | `CAIRN_PHYSICS_SPEC.md` |
| ECS | Current code wraps `bevy_ecs`; version 0.1 treats it as the runtime ECS. | Long-term baseline specifies Meridian-owned archetype chunks, stable persistent IDs, command buffers, and scheduler metadata. | Amendment wins for stable serialization, networking, tooling, and deterministic ownership. | Keep `bevy_ecs` as a replaceable Phase 1/2 implementation aid; prevent its entity/types from crossing public game/save/network APIs; migrate behind Meridian ECS contracts after benchmarks. | `ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md` and `CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md` |
| World source | Older planning uses cells/regions but leaves room for opaque built-world assumptions. | Authoritative source is a directory of schema-defined documents with stable IDs and binary sidecars; compiled chunks are caches. | Amendment wins for diff, merge, migration, and recovery. | Define world root layout, per-document versions, unknown-field handling, canonicalization, and compiled chunk headers. | `ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md` |
| World loading | Version 0.1 describes one streaming system but not cross-system authority. | One scheduler combines spatial cells, rooms, visibility, gameplay relevance, acoustics, network interest, and budgets. | Refined; old intent is preserved. | Keep current deterministic cell scheduler, add typed request reasons and staged IO/decode/upload/activation ownership. | `ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md` |
| Packaging | Older asset packs and export language can be read as independent pack files or one opaque archive. | `.meridian` is a chunked mountable package with manifest, indexes, independent compression, signatures, patches, and recovery. | Amendment wins; neither one giant compressed stream nor loose shipping output meets the contract. | Treat current pack index as an internal precursor; assign versioned superblock/chunk/index formats and CLI recovery operations. | `ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md` |
| Save format | Version 0.1 has versioned save/recovery foundations. | Saves are append-only transaction journals plus compacted snapshots and rotating recovery heads. | Refined; current versioning and recovery behavior are preserved. | Map current records to journal transactions, define commit markers/checksums, preserve unknown records, and add inspect/diff/repair tools. | `ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md` |
| Asset identity | Current code has deterministic `AssetId`, manifests, states, and residency. | Identity separates `AssetId`, `SourceId`, `ArtifactHash`, `FacetId`, `VariantKey`, and `PackageChunkId`. | Amendment refines the implemented foundation. | Keep `AssetId`; add the other identity domains without repurposing source paths or runtime handles. | `ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md` |
| Materials | Older rendering material means mostly visual PBR. | One conceptual material may expose visual, physical, structural, thermal, environmental, and acoustic facets. | Amendment wins, with facets opt-in. | Preserve current PBR resource as the visual facet; add independent schemas and loading so a server never loads textures for collision. | `RENDERING_AND_GRAPHICS_SPEC.md` and related subsystem specs |
| Rendering baseline | Version 0.1 discusses deferred opaque plus Forward+; current code has a direct PBR smoke path. | Opening-forest baseline must be chosen; visibility and virtual geometry remain measured alternatives. | Forward+ with depth prepass and clustered lighting is the Phase 2/8 baseline because it fits vegetation, transparency, MSAA/VR evolution, and the existing direct path. Visibility-buffer rendering is a Phase 12 research gate. | Evolve current direct PBR path into measured Forward+ passes; do not claim advanced renderer completion from smoke construction. | `RENDERING_AND_GRAPHICS_SPEC.md` |
| Image-based lighting | Current diffuse irradiance cube is implemented; older immediate plan made specular IBL next. | Diffuse IBL remains complete evidence; prefiltered specular IBL and BRDF LUT are future renderer work, subordinate to vertical-slice priorities. | Existing evidence preserved; priority refined. | Keep group-3 binding behavior; schedule specular IBL as a bounded Phase 2 work package after pass timing, without adding broader material variants. | `RENDERING_AND_GRAPHICS_SPEC.md` and `PLANNING.md` |
| Ray tracing and GI | Version 0.1 lists staged ray tracing and dynamic irradiance questions. | No vendor or algorithm is mandatory; use a capability and benchmark-selected portfolio with raster/baked fallback and path-traced reference. | Amendment wins to avoid unsupported claims. | Keep hardware RT out of opening critical path; prototype probe/radiance-cache and hardware-ray approaches in Phase 12. | `RENDERING_AND_GRAPHICS_SPEC.md` |
| Gameplay languages | Older roadmaps could be read as allowing several initial languages or generic scripting. | Rust plus exactly one initial high-level runtime: Luau with a broad Lua-compatible subset. C#, Anorak, Python, and mixed-language projects are later. | Amendment wins to prevent binding and tooling multiplication. | Generate Luau bindings from one API schema; reject additional runtime language work before Phase 28. | `GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md` |
| Logic tools | Older game docs describe hidden progression directly in game systems. | State Flow, Narrative Flow, Interaction, Action, and related documents share a typed logic IR but remain purpose-built documents. | Refined; creative behavior is preserved. | Encode stable state/event/command IDs and test path reachability without exposing a quest checklist. | `GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md` |
| Telepo | Any prior Telepo/cloud concept is separate and may imply accounts or hosted relays. | Telepo has not entered development and is absorbed into `meridian-sync`; direct encrypted P2P is first, with optional self-hosted relay and no required account. | Amendment explicitly replaces Telepo. | Remove `.telepo` and mandatory cloud assumptions; use `.meridian/sync/`; keep Meridian VCS authoritative. | `VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md` |
| VCS | Production docs use normal Git workflows. | Meridian adds Git-compatible storage and a Jujutsu-derived operation/change model with semantic operations. | Both coexist: Git remains interoperable storage/remotes; Meridian UX and operation log are canonical in-editor. | Keep repository compatibility, hide staging/HEAD concepts in beginner UI, and defer the fork until Phase 17 research/provenance gates. | `VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md` |
| Live collaboration | Broad collaboration plans risk replacing source control with session state. | Live collaboration augments VCS and checkpoints; it is never authoritative history. | Amendment wins. | Separate presence/locks/session transport from immutable operation history and package/artifact transfer. | `VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md` |
| Cargo/IDE | Version 0.1 uses Cargo workspace and CI but does not define lossless manifest editing or one build DAG. | Cargo files remain authoritative; editor edits use a lossless TOML tree; Cargo JSON, `cargo metadata`, assets, shaders, packaging, and signing feed one build service. | Amendment refines existing direction. | Preserve root workspace; add immutable build IDs, structured events, cancellation, and process restart boundaries. | `CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md` |
| Agent APIs | Existing `.mcp.json` is tooling configuration, not an engine API. | UI, CLI, Rust, MCP, and agents use one typed command/query registry with capabilities, transactions, audit, and checkpoints. | Amendment wins. | Do not create an AI-only backdoor; expose the same semantic commands with stricter permission profiles. | `AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md` |
| Ollama | Older roadmaps may place local AI late or treat it as generic OpenAI compatibility. | Deep Ollama support is in the initial agent phase; local and cloud hosts are distinct trust profiles; web search is separately permissioned. | Amendment wins. | Discover capabilities at runtime; never infer cloud/web permission from a local endpoint; version embedding indexes. | `AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md` |
| Security/signing | Version 0.1 has licensing/provenance but not a complete trust hierarchy. | TUF-inspired root/targets/snapshot/timestamp roles, delegated package roles, signed provenance, and explicit trust levels are foundational. | Amendment wins. | Add threat-model and cryptographic selection gates before shipping formats freeze; unsigned local builds remain clearly labeled. | `SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md` |
| Multiplayer | No early multiplayer in Project Meridian; long-term scope was vague. | Transport-neutral replication, dedicated/listen servers, Steam/EOS adapters, prediction/rollback, and impairment testing are later phases. | Preserved and refined; Project Meridian remains single-player. | Keep networking crates absent from early builds; define schemas now without implementing Phase 22 work. | `MULTIPLAYER_AND_SERVER_SPEC.md` |
| Modding | Version 0.1 defers a plugin marketplace. | Mod infrastructure is optional per game; only published APIs are stable; community library is free/community-first, not a required marketplace. | Amendment refines and preserves deferral. | Define capability manifests and restricted editor distribution in Phase 24; no opening-slice dependency. | `MODDING_AND_COMMUNITY_LIBRARY_SPEC.md` |
| Weather | Version 0.1 schedules a broad weather phase after the opening. | Opening uses basic weather/wind; multi-resolution regional atmosphere and coupled fields come later, with artist forcing and deterministic seeds. | Refined to vertical slices. | Implement only slice-required wind/fog/weather first; defer planetary, flooding, erosion, and high-quality local fluid tiles. | `WEATHER_ENVIRONMENT_AND_SIMULATION_SPEC.md` |
| Audio | Version 0.1 audio is largely a future system. | Audio callback, immutable compiled DSP graph, sample clock, streaming, spatialization, and hybrid acoustics have concrete ownership. | Amendment wins on technical detail; sequence remains slice-driven. | Build minimal output/mixer/spatial audio for Phase 8, preserving graph ABI for Phase 10/20 expansion. | `AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md` |
| Procedural authoring | Version 0.1 proposes a generalized authoring graph early. | Share graph/compiler infrastructure but keep domain documents separate; partial regeneration and non-destructive overrides are mandatory. | Amendment wins to avoid a giant universal graph. | Initial graph generates only opening-forest terrain/vegetation masks; broader buildings/ecosystems remain later. | `PROCEDURAL_AUTHORING_SPEC.md` |
| VR/XR | Version 0.1 does not put VR on opening critical path. | OpenXR lifecycle and physical interaction are specified now but implemented after desktop slice stabilization. | Preserved and refined. | Maintain renderer/input/physics timing seams; no Phase 19 implementation before Phase 8 evidence. | `VR_XR_AND_INTERACTION_SPEC.md` |
| Phase strategy | `PLANNING.md` has long engine and game milestone sequences and an Engine-Ready Gate before game content. | Every phase must produce a visible result; the opening forest is Phase 8 and may not wait for entire subsystems. | Amendment wins; an all-engine-before-game gate is rejected. | Replace the Engine-Ready Gate with capability gates feeding a vertical-slice DAG; preserve current completed code as partial evidence. | `IMPLEMENTATION_PHASES.md` |
| Completion evidence | Older plans use checkboxes and textual exit criteria. | No phase completes without demo, tests, benchmarks, recovery evidence, docs, and required audits. Skeleton types are insufficient. | Amendment wins. | Convert checkboxes to work-package status and evidence links; mark current partial implementations honestly. | `TESTING_BENCHMARKS_AND_VALIDATION.md` |
| Performance numbers | Version 0.1 has detailed numeric budgets before final scenes exist. | Unmeasured numbers are provisional and must state calibration method; no invented claims. | Amendment wins while retaining useful hypotheses. | Label legacy budgets provisional; keep measured current values separately; freeze gates only after B01/B02 corpus calibration. | `TESTING_BENCHMARKS_AND_VALIDATION.md` |
| Competitor claims | User ambition describes Meridian as more ambitious than UE5. | No superiority claim without reproducible, like-for-like benchmarks. | Amendment wins for truthfulness. | Describe intended differentiation and architecture, not unmeasured superiority. | `PRINCIPLES_AND_SCOPE.md` |

## 4. Duplicate requirement normalization

The following repeated creative decisions remain authoritative in the separate private Project Meridian documents and are referenced only where an engine contract needs them:

- no enemies, combat, player death, conventional jumpscare, or forced dialogue;
- no sprint or jump in the opening movement contract;
- documents and discoveries are optional and do not gate completion;
- the opening midnight forest, field transition, title interruption, compound, five sites, unfinished settlement, subtle road emergence, ending, and post-game state remain the game spine;
- realistic rural material language, darkness readability, rare silhouettes, and restrained analog degradation remain art/narrative constraints;
- the opening slice is production content, not a throwaway demo.

`PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md` owns the sanitized engine-facing acceptance mapping. The private `project-meridian` documents own exact narrative and art detail.

## 5. Current code classification

Current code is classified as follows:

| Area | Classification | Migration note |
|---|---|---|
| Fixed-step clocks, diagnostics, render graph, platform contracts, tasks, asset IDs, save recovery, immutable render extraction, upload planning | Preserved foundation | Extend behind version 0.2 ownership and diagnostic contracts. |
| `wgpu` RHI, direct PBR, cascaded shadows, diffuse IBL | Transitional production foundation | Keep backend private; evolve toward capability-selected render graph. Diffuse IBL is implemented; specular IBL remains planned. |
| `bevy_ecs` | Transitional implementation | Hide behind Meridian ECS contracts; benchmark before replacement. |
| `rapier3d` wrapper | Transitional research/bootstrap | Freeze public leakage; Phase 3 establishes Cairn provenance and fork. |
| Audio, UI, terrain, vegetation, weather marker crates | Scaffold only | Presence is not implementation evidence. Their version 0.2 specs define first real gates. |
| Editor/tool marker crates | Scaffold only | Do not report a usable editor. Game code and full creative content are not part of this repository. |
| Asset IO/decode/streaming queues | Partial | Uncompressed path exists; Zstandard, GPU residency activation, and corruption corpus remain open. |
| Renderer native smoke | Structural GPU evidence | Occluded surface validates construction/submission, not visible image quality. |

## 6. Legacy document disposition

1. The two engine/performance v0.1 root documents are deleted after every level 1–3 heading maps to a current engine spec, benchmark definition, explicit replacement, or rejection.
2. The five creative v0.1 root documents are rewritten without obsolete engine architecture and versioned in the private `bybrooklyn/project-meridian` repository.
3. The [migration ledger](../docs/migrations/V0_1_DOCUMENT_MIGRATION.md) is the retained historical map and must report zero unmapped headings.
4. Legacy numeric budgets are hypotheses until present in the validation spec with calibration status.
5. Creative changes require an explicit game-design amendment in the private game repository; the engine suite does not casually rewrite them.
6. The ignored local `game/` checkout and its nested Git metadata must never be staged by this repository.

## 7. Unresolved research gates

These are intentionally not fabricated as settled production decisions:

- final native-backend threshold beyond `wgpu`;
- visibility buffer versus clustered Forward+ after the opening baseline;
- virtual geometry hierarchy, page size, raster path, and deformable support;
- production dynamic-GI portfolio and ray-tracing API maturity;
- final shader source/IR and backend translation stack;
- Meridian ECS replacement timing and measured advantage over the current wrapper;
- Cairn solver portfolio constants, SIMD layout, and strict cross-platform determinism envelope;
- update/package signature algorithms and key-management library after threat-model review;
- live-collaboration CRDT/OT use by data type;
- QUIC/native UDP/Steam/EOS transport assignments by workload;
- volumetric fire, local fluid, shallow-water, snow/granular, and acoustic propagation production algorithms;
- WebAssembly component-model plugin runtime and sandbox guarantees;
- specific proprietary SDK integrations and redistribution terms.

Each gate is assigned a phase, corpus, metrics, stable API seam, and decision owner in `RESEARCH_AND_ALGORITHM_DECISIONS.md` and `IMPLEMENTATION_PHASES.md`.

## 8. Migration acceptance

This register is complete when:

- every old assumption named by the amendment appears in the matrix;
- every version 0.2 document links here and does not silently revive a replaced decision;
- all seven legacy root files are absent and every heading has a recorded disposition;
- the private creative repository contains the rewritten five-document suite and authority index;
- `PLANNING.md` distinguishes current evidence from future scope;
- a clean engine clone contains no `game/` directory and still passes Cargo metadata/tests;
- searches for `Telepo`, permanent `egui`, permanent Rapier ownership, one-file world/package assumptions, and multiple initial scripting languages either point here or use compliant language;
- a future contributor can identify the winning decision and migration phase without reading the amendment prompt.
