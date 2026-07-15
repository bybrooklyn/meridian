# Gameplay, Narrative, and Scripting Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Opening slice](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md) · [Data formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md)

Version 0.2 · 2026-07-14 · Normative · Planned with runtime/ECS precursors

## 1. Goals and non-goals

Meridian gameplay is authored through stable engine APIs, typed domain documents, and exactly one initial high-level runtime: Luau with a broad Lua-compatible subset. Rust remains available for native engine/game modules.

Goals: one API schema, generated bindings, safe hot reload, stable identity, typed events/commands, inspectable state, save/network readiness, deterministic modes where promised, and purpose-built logic tools.

Non-goals: exposing Bevy/Rapier/wgpu objects; multiple initial script runtimes; one giant universal graph; hidden objective systems; allowing script to perform arbitrary filesystem/process/network/agent operations; or changing Project Meridian’s creative design.

C#, Anorak, Python, and mixed-language architecture are Phase 28 research. Python is last by default.

## 2. Creative boundary

Project Meridian retains:

- no enemies, combat, player death, or forced dialogue;
- no sprint or jump in the opening;
- optional documents and discoveries never gate completion;
- no objective checklist;
- subtle environmental progression across forest, field, compound, settlement, roads, and ending;
- opening route remains final content.

Gameplay tooling represents these decisions and validates them. It does not normalize them into conventional quests.

## 3. Ownership and invalid edges

- meridian-gameplay-schema: API declarations, stable type/member/event/command IDs.
- meridian-gameplay-runtime: entity/component/query/event/command facade.
- meridian-luau: VM lifecycle, sandbox, modules, budgets, generated bindings.
- meridian-logic-ir: shared typed value/expression/event/action primitives.
- domain documents: State Flow, Narrative Flow, Interaction, Action, behavior, and data tables.
- project game crates: Project Meridian-specific components/systems/documents.

logic-ir does not depend on editor widgets or Luau. Luau and Rust bind the same schema. Domain documents share primitives/compiler infrastructure but retain separate schemas and editors.

## 4. API schema

~~~text
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
~~~

Generation produces Rust descriptors/adapters, Luau definitions/runtime glue, documentation, editor completion, MCP schemas, compatibility fixtures, and API hashes. Handwritten bindings are prohibited unless registered as a reviewed escape hatch.

## 5. Runtime identity and values

Scripts use PersistentEntityId or generation-checked EntityRef, never ECS-native IDs. Values crossing the runtime boundary are schema-defined scalars, IDs, handles, vectors/transforms, enums, immutable arrays/maps, or opaque capability handles.

An EntityRef can become invalid between callbacks. Every operation returns a typed result. Long-lived references are reacquired by persistent ID or subscription.

## 6. Luau lifecycle and sandbox

One VM is owned per configured isolation domain, normally a game world or trusted module group. Modules are content-addressed artifacts with source map, API hash, capabilities, and budget profile.

Sandbox rules:

- no ambient filesystem, process, native library, network, clipboard, agent, or debug access;
- no dynamic native module loading;
- bounded memory and instruction/time budgets;
- deterministic random/time APIs in deterministic modes;
- explicit capability objects for permitted operations;
- stack traces and errors redact secrets;
- untrusted content uses stricter domain/capabilities.

Luau compatibility deviations from standard Lua are documented and tested. Project code cannot depend on undocumented VM internals.

## 7. Scheduling

Script callbacks run at declared barriers:

- OnLoad and OnUnload around world/module lifecycle;
- OnFixedUpdate within bounded gameplay job partitions;
- event handlers consume immutable event batches;
- OnPresent is optional presentation-only and cannot mutate simulation;
- async operations return operation tickets resumed at a safe barrier.

Script code never runs on render submission or audio callback threads. Mutations enqueue typed commands. Event order is stable by event sequence and subscriber key in deterministic modes.

## 8. Hot reload

1. compile/validate new source in worker;
2. verify API/capability/budget;
3. run module migration function against a bounded serialized state view if required;
4. pause affected callbacks at barrier;
5. instantiate candidate and run validation hook;
6. atomically switch module generation;
7. retain prior artifact/state checkpoint until success window;
8. roll back on error and report mapped source diagnostic.

Hot reload never guesses state layout. Incompatible state requires explicit reset or migration.

## 9. Typed logic documents

Shared IR includes StableStateId, EventId, CommandId, typed ports, conditions, expressions, variables, references, debug spans, and capability requirements.

- State Flow: hierarchical/parallel state transitions and guards.
- Narrative Flow: beats, optional discoveries, presentation requests, prerequisites.
- Interaction: focus, prompt, eligibility, action mapping, response.
- Action: reusable ordered/parallel commands with compensation.

Compiled IR validates types, missing references, unreachable required states, cycles/livelock, capability absence, save compatibility, and domain restrictions.

## 10. Project Meridian model

The opening uses explicit route/start/title states, flashlight and simple interaction actions, optional document discoveries, checkpoint events, and environmental transition state. Completion is reachable with zero optional documents. Tests prove no objective checklist or prohibited movement/combat action is introduced.

Later compound/settlement/road/world-boundary decisions reference their creative source IDs and remain outside Phase 8 except for schema compatibility.

## 11. Persistence and compatibility

Save-safe script/document state is schema-declared:

~~~text
GameplayStateRecord {
  owner_id, schema_id, schema_version,
  fields, module_generation, migration_id
}
~~~

VM stacks, closures, raw tables with unknown shape, ECS IDs, pointers, and operation tickets are not persisted. Unknown optional fields round-trip. Missing required migrations stop load into a recoverable inspection mode.

API compatibility uses stable IDs and semantic versions. Removing a published member requires deprecation, migration, fixture, and release-policy approval.

## 12. Editor, CLI, and Ponder

Beginner path: create interaction from template, select trigger and action, preview prompt, play from location, inspect plain validation.

Expert path: view IR, stable IDs, API schema, capabilities, event trace, allocations/instructions, save representation, generated Luau definitions, and hot-reload transaction.

Editors provide graph and text views that round-trip one source model. Graphs have keyboard/semantic alternate views. Planned commands include gameplay validate, api generate/check, luau test/profile, logic trace, and path reachability.

## 13. Diagnostics and debugging

Required: module/source/API hash, callback duration/instructions/allocations, event/command sequence, entity IDs, capability decisions, hot-reload generation, state transition history, save migration, and deterministic divergence.

Debugger supports mapped breakpoints, stack/locals with redaction, pause policy, step, event/state-flow trace, and time-bounded evaluation. Pausing one world cannot block audio or corrupt network/server timing.

## 14. Security and zero-cost behavior

Capabilities are deny-by-default and project/profile scoped. Scripts cannot grant capabilities. Agent-generated source is treated as untrusted until normal validation/review.

If Luau is not selected by a project, its VM, artifacts, editor panels, tasks, and package chunks are absent. Logic documents that compile directly to native/IR MAY remain without a VM.

## 15. Tests and benchmarks

- schema generation parity and stable-ID compatibility;
- Luau compatibility corpus and sandbox escape suite;
- budget/infinite-loop/memory exhaustion;
- callback/event ordering and deterministic replay modes;
- hot reload success, failure, state migration, rollback;
- save fixtures across versions and missing modules;
- graph type/cycle/reachability/livelock fuzz tests;
- opening completion with zero optional discoveries;
- API call, event dispatch, VM startup/module load, memory, and hot-reload benchmarks.

Provisional thresholds are calibrated on opening and stress corpora.

## 16. Phases

- Phase 7: API schema, Luau, sandbox, hot reload, core documents, opening logic.
- Phase 8: opening traversal, interaction, optional discoveries, save/title integration.
- Later game phases extend documents without changing shared IR ownership.
- Phase 22 uses schema metadata for replication.
- Phase 24 publishes selected mod API.
- Phase 28 evaluates additional languages independently.

## 17. Examples

End-to-end: an Interaction document maps focus on a gate to Use, validates state, invokes a typed OpenGate command, updates the State Flow, emits an audio event, and journals a save-safe field. Rust, Luau, UI, and trace use the same IDs.

Failure/recovery: a Luau hot reload changes state shape without migration. Candidate validation fails, old artifact/state remain active, the editor highlights the schema difference, and play continues.

Performance debug: a frame spike opens a script trace grouped by module/callback/entity. It reveals an unbounded world query; the user replaces it with a schema-indexed query and verifies instruction/allocation reductions on the same replay.
