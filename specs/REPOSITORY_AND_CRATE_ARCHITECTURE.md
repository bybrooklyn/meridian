# Repository and Crate Architecture

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Runtime](CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md) · [Build](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md)

version 0.5 · 2026-07-15 · Normative architecture

Documentation maturity: `ImplementationReady`. Implementation maturity:
`Partial`. Governing IDs: `REQ-CORE-003`, `REQ-CORE-004`, `REQ-RHI-001`,
`REQ-DAT-001`, `WP-GOV-001`.

## 1. Goals

- keep platform, runtime, editor, game, tools, schemas, and content independently understandable;
- enforce dependency direction at Cargo, API, process, and data levels;
- hide transitional third-party types behind Meridian-owned contracts;
- permit minimal runtime/server/tool builds;
- allow optional feature packs to disappear completely;
- make provenance, tests, benchmarks, and schemas adjacent to ownership.

Non-goals are a single engine crate, a plugin-everything architecture, dynamic dispatch on hot paths by default, or moving source authority into generated artifacts.

## 2. Repository roots

| Root | Authority |
|---|---|
| engine/ | reusable runtime/editor libraries |
| editor/ | The single Meridian creator application, its bounded helper services/tools, and temporary bootstrap shell |
| external private game repository | Project Meridian code, content, and full creative documents; an ignored local checkout may be mounted at `game/` |
| shaders/ | shader sources, reflection expectations, validation |
| schemas/ | versioned shared source/file/network schema inputs |
| assets_source/ | editable or licensed source content |
| assets_built/ | derived local cache; never source authority |
| docs/ | ADRs, developer and architecture guidance |
| specs/ | normative version 0.5 product and engineering suite |
| third_party/ | pinned source/provenance manifests and patches |
| licenses/ | project and dependency licensing evidence |
| .github/ | CI policy and automation |

Generated output belongs in target/, a local cache root, or ignored build directories. It MUST NOT be hand-edited.

## 3. Layering

~~~text
game + editor + tools
        |
domain services: world assets Penumbra Cairn Wavefront UI gameplay animation navigation 2D Collective
        |
runtime contracts: core tasks platform diagnostics schema capability
        |
backend adapters: wgpu/winit/audio/os/accessibility/network/sdk
~~~

Allowed dependency direction is downward. Domain peers communicate through narrow contracts or orchestration crates, not circular Cargo edges. Tools MAY depend on runtime libraries; runtime libraries MUST NOT depend on editor or tools.

## 4. Crate families

Current names are retained where useful; new names become binding only when an
approved work package establishes their implementation boundary.

### 4.1 Foundation

- meridian-core: clocks, IDs, errors, capability values, deterministic primitives.
- meridian-diagnostics: events, spans, counters, crash context, redaction.
- meridian-tasks: executors, budgets, cancellation, task classes.
- meridian-platform: portable platform contracts.
- platform backend crates: macOS first, Linux second, Windows third.
- meridian-schema: schema registry, migrations, canonicalization.
- meridian-commands: typed command/query descriptors, transactions, audit.

Invalid edges: core to renderer; tasks to editor; platform to game; diagnostics to a network provider.

### 4.2 Runtime data

- meridian-ecs: Meridian entity/component/query/command contracts. Current bevy_ecs use is Transitional.
- meridian-world: source world model, cells, rooms, relevance, compiled views.
- meridian-assets: identity, import recipes, artifact/facet graph, residency.
- meridian-streaming: cross-system request scheduler and stage queues.
- meridian-save: journal, snapshots, recovery, migrations.
- meridian-package: provisional `.meridian` v1 superblock, canonical manifest,
  bounded uncompressed chunks, BLAKE3 verification, mounts, and random reads.

bevy_ecs entity IDs MUST NOT appear in saves, packages, scripts, network schemas, editor documents, or public game APIs.

### 4.3 Presentation and simulation

- meridian-renderer: Penumbra's current implementation crate for extraction,
  render graph integration, materials, lighting, and current GPU foundations.
- meridian-rhi: backend-neutral resource and command descriptors; wgpu is current backend.
- future Meridian Shader Language/ShaderIr modules: source/graph lowering, IR validation, reflection, target modules, variants, source maps, caches. Existing shader tools remain until a package establishes boundaries.
- cairn-core and Cairn domain crates: broadphase, narrowphase, dynamics, query, character, particles, fracture.
- meridian-audio: Wavefront graph compiler, mixer, devices, streaming, spatialization, acoustics, and optional voice audio path.
- meridian-ui-core, ui-render, ui-text, ui-semantics, ui-runtime, ui-editor.
- meridian-isobar: current empty Isobar scaffold.
- meridian-basalt: current empty Basalt scaffold.
- meridian-vegetation: current vegetation scaffold and future vegetation owner.
- meridian-torsant: reserved name; no crate exists until its first real package.

wgpu, Rapier, egui, and AccessKit types MUST stop at their adapter crates. Existing leaks are migration defects.

### 4.4 Authoring and product

- meridian-editor-core: document sessions, selection, commands, undo, play mode.
- meridian-editor: the native application composition currently named by crate
  history; its executable is `meridian` and user-facing title is **Meridian**.
  MS-01 native/headless integration exists; Creator Editor Alpha remains planned.
- meridian-editor-egui-bootstrap: temporary shell only; deletable when Meridian UI migration gates pass.
- meridian-build: editor-only build-service foundation with Meridian-owned
  BuildId/request/event contracts, bounded Cargo JSON mapping, structured Cargo
  checks/builds, helper CLI, host-selected local durable worker-loss recovery, and a
  deterministic Cargo metadata -> check/build dependency graph; long-lived scheduling,
  durable cache/provenance, and broad DAG adapters remain planned.
- meridian-vcs and meridian-sync.
- meridian-agent-api and provider adapters.
- meridian-ponder.
- future Rust gameplay module/reflection/reload components under `WP-GAM-001`.
- future optional meridian-luau and generated gameplay bindings under `WP-GAM-002`.
- future native model-document/kernel/tool modules under `WP-MDL-001`; no placeholder crate is created by v0.5.
- future animation, navigation, framework, dedicated 2D, and Collective modules only when their first real work packages start.
- meridian-spec: documentation-governance CLI under editor/; no runtime or game
  crate may depend on it.
- future meridian-alluvium: typed procedural recipe/field evaluation,
  incremental cache, generated identity, overrides, provenance, and cooking.
  The name is reserved; no crate exists until `WP-PRC-001` begins real work.
- external Project Meridian game crates and content, consumed through published Meridian APIs after that integration is activated.

Alluvium is core editor/build architecture, not an optional proprietary plug-in.
Its domain adapters and runtime-safe evaluator remain capability-scoped. A
baked-only shipping profile depends on generated asset/world/package facets,
not on editor, graph compiler, preview cache, or runtime Alluvium code.

The `ANI`, `NAV`, `FWK`, `TWO`, `SHD`, `MDL`, `COL`, `WRL`, and `INT` identifiers are governance domains, not mandatory crate names. Wavefront and Collective are subsystem/product names; current crate names remain until an implementation package has a concrete migration reason. WRL and INT are post-1.0 authorities and create no placeholder crates.

## 5. Contract shape

Public domain operations use owned descriptors and generational handles:

~~~rust
pub struct Handle<T> { slot: u32, generation: u32, marker: PhantomData<T> }
pub struct CapabilityId(pub StableId);
pub struct OperationId(pub StableId);
pub struct BuildId(pub Hash256);

pub trait Service {
    type Request;
    type Response;
    fn submit(&self, request: Self::Request, ctx: OperationContext)
        -> Result<OperationTicket<Self::Response>, ServiceError>;
}
~~~

Descriptors are versioned and validated before allocation. Handles are process-local. Stable IDs cross persistence boundaries. Snapshots are immutable and epoch-tagged. Commands hold explicit authority and undo metadata.

## 6. Crate boundary rules

- no global mutable singleton in a public API;
- no blocking filesystem/network/process call on simulation, render, or audio real-time paths;
- no panic for user/project data errors;
- no async runtime type in portable domain APIs;
- no backend resource in source documents;
- no editor dependency in a shipping runtime unless the game explicitly includes editor features;
- no optional pack registry entry unless the pack is compiled or dynamically installed;
- no third-party error as the only public error; preserve source detail behind Meridian categories.

CI uses cargo metadata to assert forbidden edges and feature unification. A minimal feature profile is compiled separately.

## 7. Process boundaries

The editor process owns user interaction and document sessions. Crash-prone or untrusted work is isolated:

- asset import workers;
- shader compiler/validator workers where practical;
- build service;
- package/signing helper with narrow key access;
- optional agent provider host;
- model import/heavy-operation and animation import/compression workers;
- optional Collective provider/service adapter hosts;
- dedicated game/server processes for play and network tests.

IPC messages use versioned schemas, length limits, cancellation, deadlines, and trace IDs. Process restart restores from source plus committed transaction/checkpoint, never undocumented in-memory state.

## 8. Feature packs

A feature pack is a manifest plus a closed set of crates, schemas, package chunks, editor panels, commands, permissions, and tests. Core knows capabilities, not concrete packs.

Examples: advanced-weather, fluid-simulation, fracture, hardware-ray-tracing, openxr, provider-specific NET/Collective adapters, optional Luau, advanced animation, community-library, and cloud-agent. First-class baseline 2D and the editor's native modeler are product capabilities, but player/runtime inclusion remains profile- and content-driven.

Activation:

1. resolve manifest and licenses;
2. validate platform and dependencies;
3. preview package/runtime costs and permissions;
4. update project capability document transactionally;
5. rebuild affected artifacts;
6. load only after all required schemas and runtime capabilities agree.

Deactivation refuses when authored content depends on the pack unless the user accepts a migration/export of that data.

## 9. Threading and memory ownership

Foundation crates define task classes and arenas; domain crates request budgets rather than spawning private unmanaged pools. The audio callback owns no general allocator. Render resources are created/destroyed on the render submission owner. World and asset data move through immutable blobs or generation-checked handles.

Large blobs use content-addressed shared storage and mapped/read-only views. Cross-process transfers use hashes and bounded shared-memory/file handles rather than arbitrary object serialization.

## 10. Persistence and compatibility

Every persisted document has:

- schema ID and version;
- stable object IDs;
- required/optional field classification;
- canonical encoding rules;
- migration chain;
- unknown-field policy;
- content hash and optional signature;
- maximum sizes and recursion limits.

Crate version is not a file-format version. A format change requires fixtures for oldest-supported, previous, current, malformed, truncated, and unknown-field cases.

## 11. Diagnostics

Each service exposes:

- availability and selected backend;
- dependency and feature graph;
- current budgets and queue depth;
- last operation IDs and structured failures;
- recovery action;
- source-to-artifact provenance;
- redacted trace export.

Beginner surfaces translate diagnostics but preserve a stable code and an Inspect detail path.

## 12. Security

Crates declare trust boundaries in module-level docs. Parsers enforce limits before allocation. External processes receive least-privilege paths/tokens. Signing keys are never loaded by editor UI code. Provider adapters cannot synthesize command authority.

Supply-chain requirements are defined in [security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md).

## 13. Testing and acceptance

- cargo metadata forbidden-edge test;
- minimal/default/all-feature build matrix;
- duplicate dependency and feature-unification report;
- no-work-disabled runtime trace;
- adapter boundary compile-fail tests;
- process crash/restart and stale-handle tests;
- schema fixture/migration corpus;
- package composition proves unused optional chunks absent.

Architecture acceptance requires one minimal headless runtime, one editor build, one external consumer-game integration build when available, and one dedicated-server-shaped build to share schemas without accidental presentation dependencies. Project Meridian integration evidence is produced in its separate private repository.

## 14. Delivery migration

- MS-00: establish this dependency policy and automated inventory.
- MS-01: stabilize core/task/platform/diagnostic contracts.
- MS-01/MS-04/MS-05: keep wgpu current; prevent new backend leakage.
- MS-01, MS-04, MS-06, and MS-07: formalize asset/world/save/UI/Wavefront/Rust-gameplay seams.
- MS-03/MS-08: begin Cairn source/provenance and native contract migration.
- MS-03, MS-08, and MS-09: build service, VCS, sync, and agents use common commands.
- MS-08/MS-09: re-run modularity audit against shipping profiles.
- MS-03/MS-05/MS-08/MS-09: add modeler, animation, navigation, framework, 2D, ShaderIr, optional Luau, and Collective modules only through their typed packages and forbidden-edge tests.
- Post-1.0: WRL/INT and advanced MDL/ANI/FWK/COL/VCS/SHD programs remain outside the MS graph and create code only after their entry gates.
- MS-10: freeze supported format/API compatibility for the release line.

## 15. End-to-end and failure examples

End-to-end: importing a texture starts in editor command registry, runs in an importer process, writes a content-addressed artifact, registers visual material facets, invalidates build nodes, and streams a generation-tagged runtime resource. No renderer crate reads the source file.

Failure/recovery: importer crash leaves no manifest pointer to a partial artifact. The operation reports a crash diagnostic, preserves source/import settings, restarts the worker, and reuses validated prior hashes.

Performance debug: a frame spike links render pass, asset upload, world cell activation, and originating build/artifact IDs through one trace ID. The user can disable the responsible optional pack and verify zero recurring tasks on the next trace.
