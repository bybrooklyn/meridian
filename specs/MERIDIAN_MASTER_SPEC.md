# Meridian Master Specification

Version 0.2 · 2026-07-14 · Normative index

This suite defines Meridian as a general-purpose engine for games and interactive applications. Project Meridian is the first proving game, maintained in the separate private `bybrooklyn/project-meridian` repository and not a dependency of the engine. The suite incorporates the binding v2 amendment, preserves the engine-facing creative constraints from the private game suite, and replaces conflicting legacy architecture and phase assumptions.

## 1. How to use this suite

Read in this order:

1. [Principles and scope](PRINCIPLES_AND_SCOPE.md)
2. [Migration and contradictions](SPEC_MIGRATION_AND_CONTRADICTIONS.md)
3. [Repository and crate architecture](REPOSITORY_AND_CRATE_ARCHITECTURE.md)
4. [Implementation phases](IMPLEMENTATION_PHASES.md)
5. The subsystem specification relevant to the work
6. [Testing, benchmarks, and validation](TESTING_BENCHMARKS_AND_VALIDATION.md)
7. Root [PLANNING.md](../PLANNING.md) for current evidence and the next bounded work package
8. The private [Project Meridian creative suite](https://github.com/bybrooklyn/project-meridian/tree/main/docs) only when a proving-game decision is required

If statements conflict, the authority order in the migration register applies. A planned type or algorithm is not evidence that it exists. Current implementation evidence is listed in PLANNING and must link to tests, traces, captures, or code.

## 2. Normative language and status

MUST and MUST NOT are release-blocking requirements. SHOULD records the expected design with an allowed, documented exception. MAY is optional.

Every capability is labeled:

- Implemented: verified in the current repository.
- Transitional: usable current code behind a Meridian-owned seam, scheduled for replacement.
- Planned: specified but not implemented.
- Research: choice intentionally open until a named experiment.
- Deferred: valid scope outside the current vertical slice.

No document may convert Planned or Research into Implemented without evidence.

## 3. Product invariants

- M-001: Meridian MUST be usable without Blender, shader code, Cargo knowledge, VCS knowledge, AI, a cloud account, or a hosted Meridian service.
- M-002: Expert workflows MUST remain available through typed Rust APIs, CLI commands, schemas, build events, and diagnostics.
- M-003: Optional feature packs MUST add no threads, recurring tasks, allocations, GPU resources, network listeners, or package payload when disabled.
- M-004: macOS on Apple Silicon is the first evidence platform; Linux is second; Windows is third. Platform-specific code MUST stay behind platform contracts.
- M-005: engine crates MUST NOT depend on Project Meridian game crates or content.
- M-006: authoritative editable data MUST be schema-defined, versioned, inspectable, recoverable, and mergeable. Derived artifacts are caches.
- M-007: one typed command/query model MUST serve editor UI, CLI, Rust tools, MCP, and agents. AI receives no privileged backdoor.
- M-008: performance and competitor claims MUST be reproducible and calibrated. Unmeasured targets are provisional.
- M-009: security, provenance, accessibility semantics, diagnostics, undo, and recovery are architecture concerns, not release polish.
- M-010: every phase MUST end in a visible result and evidence; scaffolds and placeholder types do not complete a phase.

## 4. Coordinated documents

| Subject | Normative document |
|---|---|
| Principles, users, boundaries | [PRINCIPLES_AND_SCOPE.md](PRINCIPLES_AND_SCOPE.md) |
| Crates, dependency rules, feature packs | [REPOSITORY_AND_CRATE_ARCHITECTURE.md](REPOSITORY_AND_CRATE_ARCHITECTURE.md) |
| Clocks, tasks, memory, handles, platform | [CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md](CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md) |
| Editor and shared UI framework | [EDITOR_AND_MERIDIAN_UI_SPEC.md](EDITOR_AND_MERIDIAN_UI_SPEC.md) |
| Accessibility and Ponder | [ACCESSIBILITY_DOCUMENTATION_AND_PONDER_SPEC.md](ACCESSIBILITY_DOCUMENTATION_AND_PONDER_SPEC.md) |
| Asset, world, save, package formats | [ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) |
| Rendering and graphics | [RENDERING_AND_GRAPHICS_SPEC.md](RENDERING_AND_GRAPHICS_SPEC.md) |
| Cairn physics | [CAIRN_PHYSICS_SPEC.md](CAIRN_PHYSICS_SPEC.md) |
| Audio, music, acoustics | [AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md](AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) |
| Weather and coupled simulation | [WEATHER_ENVIRONMENT_AND_SIMULATION_SPEC.md](WEATHER_ENVIRONMENT_AND_SIMULATION_SPEC.md) |
| Procedural authoring | [PROCEDURAL_AUTHORING_SPEC.md](PROCEDURAL_AUTHORING_SPEC.md) |
| Gameplay, narrative, Luau | [GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) |
| OpenXR and interaction | [VR_XR_AND_INTERACTION_SPEC.md](VR_XR_AND_INTERACTION_SPEC.md) |
| Networking and servers | [MULTIPLAYER_AND_SERVER_SPEC.md](MULTIPLAYER_AND_SERVER_SPEC.md) |
| Modding and community library | [MODDING_AND_COMMUNITY_LIBRARY_SPEC.md](MODDING_AND_COMMUNITY_LIBRARY_SPEC.md) |
| Cargo, IDE, builds, teams | [CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) |
| VCS, collaboration, sync | [VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md](VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md) |
| Agent API, MCP, Ollama | [AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md) |
| Signing, updates, supply chain | [SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) |
| Proving-game slice | [PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md) |
| Phase DAG and gates | [IMPLEMENTATION_PHASES.md](IMPLEMENTATION_PHASES.md) |
| Test and benchmark contracts | [TESTING_BENCHMARKS_AND_VALIDATION.md](TESTING_BENCHMARKS_AND_VALIDATION.md) |
| Concrete schemas and flows | [API_AND_FILE_FORMAT_EXAMPLES.md](API_AND_FILE_FORMAT_EXAMPLES.md) |
| Decisions and primary research | [RESEARCH_AND_ALGORITHM_DECISIONS.md](RESEARCH_AND_ALGORITHM_DECISIONS.md) |
| Migration record | [SPEC_MIGRATION_AND_CONTRADICTIONS.md](SPEC_MIGRATION_AND_CONTRADICTIONS.md) |
| Agent working policy | [AGENTS.md](AGENTS.md) |

## 5. Stable architectural seams

The long-lived contracts are descriptors, handles, schemas, capability queries, command/query registries, and immutable snapshots. Third-party implementation types MUST NOT leak through those seams.

Core stable concepts:

- persistent IDs identify project data across saves, packages, networks, and VCS;
- generational runtime handles reject stale process-local references;
- immutable render, audio, physics, and network snapshots isolate real-time domains;
- command buffers are the only cross-domain mutation path;
- budgeted schedulers publish reasons, priorities, deadlines, and cancellation;
- capability manifests describe optional code, data, permissions, platform support, and fallback;
- all long-running operations emit structured progress, diagnostics, cancellation, and recovery information.

## 6. Runtime domains and clocks

Meridian separates:

- platform/event domain;
- fixed simulation domain;
- variable presentation domain;
- render submission domain;
- audio callback domain;
- asynchronous IO/build domain;
- optional network domain.

Simulation time, presentation time, audio sample time, network time, and wall time are distinct types. Cross-domain messages include sequence, epoch, and intended clock. No callback or real-time thread waits on editor, filesystem, build, or network work.

## 7. Data authority

| Data | Authority | Derived forms |
|---|---|---|
| Project settings and Cargo files | human-readable source documents | resolved build graph |
| World | schema-defined directory and sidecars | compiled cells/chunks |
| Assets | source plus import settings and provenance | artifacts, facets, variants |
| Logic | typed graph/text documents | validated IR and runtime bytecode |
| UI | Meridian UI documents/styles/semantics | layout and render caches |
| Saves | committed journal transactions and snapshots | indexes and recovery views |
| Packages | signed manifest and independent chunks | mounted lookup tables |
| Version control | immutable objects plus operation log | working views and live sessions |

Unknown optional fields MUST round-trip when safe. Unknown required fields MUST fail with a diagnostic and leave the source untouched.

## 8. Phase strategy

Phases 0 through 29 form a dependency DAG, not a subsystem marathon. Phase 8 delivers the playable opening forest and is fed by capability gates from runtime, rendering, assets, input, physics, audio, UI, saves, and authoring. Later systems MAY be specified early but MUST NOT block Phase 8.

The current repository is partway through Phase 2. Fixed-step runtime, structural renderer construction, PBR/shadows, diffuse irradiance IBL, deterministic asset/streaming foundations, save foundations, and a Rapier wrapper exist. Visible-pixel validation, pass-level timing, platform evidence, production audio/UI, Cairn ownership, and most advanced systems remain open.

## 9. Change control

A normative change requires:

1. requirement IDs and affected documents;
2. old and new behavior;
3. compatibility and migration impact;
4. security, accessibility, performance, and optionality impact;
5. tests or research evidence;
6. update to the contradiction register when it changes an older decision.

Large changes are recorded as ADRs under docs/architecture/decisions and linked from this suite. PLANNING is updated only after the normative documents agree.

## 10. Completion standard

The suite is internally complete when every required document exists, links resolve, the legacy heading ledger reports zero unmapped content, deleted root documents have current destinations, the phase DAG maps all requirements, all research gates have owners and evidence criteria, and the root plan distinguishes current evidence from future scope. Product completion is governed phase-by-phase; this specification does not claim the engine is finished.
