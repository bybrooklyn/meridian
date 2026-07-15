# Gameplay, Narrative, and Scripting Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [ADR-0019](../docs/architecture/decisions/ADR-0019-rust-first-luau-after.md) · [Frameworks](OFFICIAL_GAMEPLAY_FRAMEWORKS_SPEC.md) · [Prototype](PROJECT_MERIDIAN_PROTOTYPE_PLAN.md) · [Data](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)

Version 0.5 · 2026-07-15 · Normative architecture

Documentation maturity: `ImplementationReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-GAM-001`; `REQ-GAM-002`; `WP-GAM-001`; `WP-GAM-002`.

Current implementation status: Meridian has runtime/ECS/data precursors but no stable public gameplay API, Rust game-module lifecycle, gameplay reflection, Play-session reload, typed logic compiler, Luau VM, generated bindings, or gameplay debugger. Project Meridian's prototype will use Rust through `WP-GAM-001`; Luau is not its prerequisite.

## 1. Authority, Goals, and Non-Goals

The `GAM` domain owns stable gameplay-facing schemas, modules, components/resources/queries, typed events and commands, gameplay lifecycle barriers, reflection metadata, logic documents, save/network/headless hooks, debugging, and optional language bindings. Engine subsystems own their runtime state. Frameworks provide replaceable packages over these APIs. Projects own game-specific rules and creative data.

Goals:

- implement one safe, observable Rust gameplay path first;
- generate editor reflection, documentation, compatibility metadata, and later Luau bindings from one API schema;
- preserve stable identity, typed failure, save/network readiness, deterministic modes where promised, and isolated Play sessions;
- make Rust-only, Luau-only, data/logic-only, and explicitly mixed projects possible after the relevant packages exist;
- keep beginner templates and visual/text logic tools without hiding authoritative state.

Non-goals are exposing Bevy/Rapier/wgpu/backend types, requiring an embedded VM, one giant universal graph, arbitrary process/filesystem/network access, pretending native Rust always hot reloads safely, or changing a project's creative design. Additional languages beyond Luau are later independent research.

## 2. Ownership and Forbidden Edges

Planned components are descriptive boundaries, not crate commitments:

- gameplay schema: stable modules, types, members, events, commands, capabilities, versions;
- Rust gameplay runtime: module lifecycle, registration, reflection, command/query adapters, isolated Play loading;
- optional Luau runtime: sandbox, module loading, budgets, generated bindings, mapped debugging;
- logic IR: shared values, expressions, events, actions, state and narrative primitives;
- project modules: game-specific Rust, optional scripts, documents, and data.

Forbidden edges:

- public game code storing ECS-native entity IDs, pointers, renderer handles, physics donor types, or OS resources;
- Rust or Luau mutating render/audio/network/physics internals directly;
- editor widgets becoming serialization authority;
- native module reload continuing after ABI/state safety is unknown;
- script capabilities self-authorizing or bypassing project security/accessibility policy;
- private Project Meridian types entering public framework or engine APIs.

## 3. Planned API Schema

```text
ApiModule {
  id, version, stability, capabilities,
  types, components, resources, queries,
  events, commands, functions, documentation
}

ApiFunction {
  id, name, parameters, result, errors,
  purity, thread_domain, capabilities,
  determinism, allocation_class, since, deprecated
}

GameplayModuleDescriptor {
  id, version, api_hash, build_id, dependencies,
  capabilities, lifecycle, state_schema, reload_policy
}
```

Generation produces Rust descriptors/adapters, editor property metadata, documentation, CLI/MCP schemas, compatibility fixtures, API hashes, and later Luau definitions/runtime glue. Handwritten binding exceptions require a registered review and parity test.

Cross-boundary values are schema-defined scalars, IDs, handles, vectors/transforms, enums, immutable arrays/maps, records, or opaque capability handles. Scripts and modules use `PersistentEntityId` or generation-checked `EntityRef`, never ECS-native IDs. Every invalid/stale reference returns a typed result.

## 4. Rust-First Gameplay Foundation

Rust is the first implementation and extension language under `WP-GAM-001`. The foundation includes:

- project-visible gameplay crates/modules and stable engine API facades;
- schema-derived component/resource/property reflection for hierarchy and inspector;
- typed event, command, query, operation-ticket, and error contracts;
- fixed/presentation/headless lifecycle registration at declared barriers;
- deterministic time/random/query modes where selected;
- save migrations, replay/network seams, diagnostics, and tests;
- isolated Play-session build, load, stop, checkpoint, rebuild, and restart.

Rust game code remains ordinary auditable source. The engine may support dynamic libraries, process isolation, relinked modules, or whole Play-session restart by platform/profile. No one mechanism is promised universally.

## 5. Native Reload and Play Isolation

```text
snapshot source-authoritative editor state
-> build candidate game module in bounded worker
-> verify API hash capabilities dependencies and state schema
-> stop affected Play world at safe barrier
-> checkpoint declared save-safe state where compatible
-> load candidate in isolated session or restart process/session
-> migrate and validate state
-> publish new generation after health window
-> retain prior build and checkpoint for rollback
```

If ABI, platform loader, leaked thread, native resource, or state safety is uncertain, Meridian restarts the isolated Play session. It does not pretend unsafe native code was hot-reloaded. Editor source state remains separate from Play state. Failed candidates preserve the previous accepted build and provide mapped compiler/runtime diagnostics.

## 6. Optional Luau Runtime

Luau is Meridian's first optional embedded high-level language, delivered only by `WP-GAM-002` after Rust APIs stabilize. It binds the same schema and cannot redefine gameplay semantics.

One VM is owned per configured isolation domain, normally a world or trusted module group. Modules are content-addressed with source maps, API hash, capabilities, budget, and state schema.

Sandbox rules:

- no ambient filesystem, process, native library, network, clipboard, debug, agent, or host pointer access;
- bounded memory, instruction/time, recursion, module, and event budgets;
- deterministic random/time APIs in deterministic modes;
- explicit capability objects for permitted operations;
- stack traces and diagnostics redact secrets/private data;
- untrusted mods receive stricter isolation and published APIs only.

Luau reload may migrate declared serialized module state at a safe barrier. VM stacks, closures, unknown raw tables, pointers, and operation tickets are never persisted. If Luau is absent, no VM, artifact, task, binding table, panel, dependency, or package chunk exists.

## 7. Scheduling and State Authority

Gameplay callbacks run only at declared barriers:

- module load/unload around world lifecycle;
- fixed simulation systems in explicit ordered sets or dependency graph;
- event handlers consume immutable ordered batches;
- presentation systems may request effects but cannot mutate authoritative simulation;
- asynchronous operations return generation-checked tickets resumed at safe barriers;
- dedicated/headless profiles omit presentation hooks.

Mutations enqueue typed commands. Event order is stable by sequence and subscriber key in deterministic modes. Game code never runs on render submission or Wavefront audio callback threads.

## 8. Typed Logic Documents

Shared logic IR includes stable state/event/command IDs, typed ports, expressions, variables, references, source spans, capabilities, budgets, and debug metadata.

- State Flow: hierarchical and parallel state transitions and guards.
- Narrative Flow: beats, optional discoveries, presentation requests, prerequisites.
- Interaction: focus, eligibility, action mapping, prompts, responses.
- Action: ordered/parallel typed commands with compensation.
- Behavior assets: optional project/framework decisions over game-visible state, never NAV or subsystem internals.

Text and graph views operate on one model and have keyboard/semantic parity. Compilers validate types, missing references, cycles/livelock, unreachable required states, capability absence, save compatibility, and authority violations.

## 9. Project Meridian Boundary

The prototype uses Rust modules plus typed data/logic documents. It proves engine APIs, isolated Play restart, interaction, movement, save, Wavefront, Cairn, Penumbra, and packaging. It does not block on Luau.

Project Meridian's private rules and creative content remain private. Engine-side tests may prove generic route/state/optional-discovery invariants using sanitized fixtures, but no private narrative, route payload, document, or asset enters this repository.

## 10. Persistence, Network, and Compatibility

```text
GameplayStateRecord {
  owner_id, schema_id, schema_version,
  fields, module_generation, migration_id
}
```

Only declared values persist. Unknown optional fields round-trip. Missing required migrations open recoverable inspection rather than corrupting source. Network/replay schemas use stable IDs and explicit authority; language implementation details never enter the protocol.

Published API removal requires deprecation, migration, fixtures, compatibility review, and release approval. Rust, Luau, logic documents, editor, CLI, and MCP use the same member IDs and errors.

## 11. Editor, CLI, Accessibility, and Diagnostics

Beginner workflow: create a Rust gameplay project/module from a visible template, add reflected properties and typed interactions, build, run in isolated Play, inspect plain diagnostics, and recover after failure. Once available, Luau is an optional project setting, not the default prerequisite.

Expert workflow: inspect API schemas, system ordering, capabilities, event/command traces, allocations, state migrations, build IDs, replay divergence, generated bindings, and reload/restart transactions.

Gameplay schemas express input alternatives, timing adjustments, captions/cues, text/contrast needs, reduced-motion behavior, and progression requirements. Graph and debugger tools are keyboard and screen-reader accessible and never encode required state by color alone.

Diagnostics include module/source/API/build hashes, lifecycle state, system/callback duration and allocation, event/command sequence, persistent entity IDs, capability decisions, build/reload generation, state migration, and deterministic divergence. Pausing a Play world cannot block Wavefront or corrupt a server session.

## 12. Security, Failure, and Recovery

Capabilities are deny-by-default and project/profile scoped. Native project code is trusted at the level its build profile declares but remains isolated from editor source state where possible. Script/mod code is untrusted. Agent-generated code passes the same build, review, capability, and test gates as human code.

Failures include compile error, API mismatch, missing dependency, capability denial, stale entity, budget excess, migration failure, leaked native resources, VM fault, deterministic divergence, and module crash. Each has a stable error, affected IDs, source span where possible, rollback/restart action, and preserved prior source/build/checkpoint.

## 13. Tests, Evidence, and Delivery

- `REQ-GAM-001`: Rust is first and must prove stable APIs, reflection, lifecycle, isolated Play rebuild/restart, saves, diagnostics, and headless operation.
- `REQ-GAM-002`: optional Luau must prove generated binding parity, sandboxing, migration, mixed-project semantics, and zero-cost omission.
- `WP-GAM-001` contributes to MS-06/MS-08 and is required by the Project Meridian prototype.
- `WP-GAM-002` contributes to selected MS-08/MS-09 profiles and cannot block MS-06/MS-07.

Tests cover API generation and compatibility, Rust module lifecycle, reflected properties, event/command ordering, fixed/presentation/headless barriers, build failures, isolated restart and checkpoint rollback, save migrations, stale entities, deterministic replay, graph type/cycle/reachability fuzzing, and accessibility. Luau adds compatibility, sandbox escape, budget exhaustion, binding parity, reload, and stripped-build tests.

Benchmarks record module build/restart, system/callback distributions, event dispatch, queries, save migration, memory, allocations, optional VM startup/load, and reload. Compilation success alone is not runtime or migration evidence.

## 14. Examples

End to end: Rust game code registers a reflected door component and typed `OpenDoor` command. The editor exposes properties, an Interaction document invokes the command, Wavefront receives an event, and the save stores schema-defined state.

Failure: a Rust change alters state without a migration. Candidate Play startup fails, the editor keeps source edits and prior build/checkpoint, and offers reset or an explicit migration rather than unsafe continuation.

Performance debug: a spike groups fixed systems by module/query/command and reveals an unbounded world scan; the creator replaces it with a schema-indexed query and verifies the same replay.
