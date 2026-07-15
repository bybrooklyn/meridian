# Official Gameplay Frameworks Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Gameplay and scripting](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) · [Cairn](CAIRN_PHYSICS_SPEC.md) · [Navigation](NAVIGATION_AND_AI_INFRASTRUCTURE_SPEC.md) · [Animation](ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

Status: version 0.5 normative architecture, 2026-07-15.

Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-FWK-001` through `REQ-FWK-003`; `WP-FWK-001`; `PRG-FWK-001`.

Current implementation status: no official Meridian gameplay framework family is implemented. Project Meridian-specific gameplay is not evidence of a reusable framework.

## 1. Authority and Product Position

The `FWK` domain owns optional, reusable gameplay packages built only on public engine contracts. It does not own the runtime, ECS, input, physics, animation, navigation, networking, UI, audio, or game-specific rules.

Six long-term official families are planned:

1. first-person and immersive interaction;
2. third-person action/adventure;
3. character movement and parkour;
4. shooter/combat foundations;
5. vehicle and traversal foundations;
6. strategy/simulation and 2D genre foundations.

These are capability targets, not current features or universal 1.0 blockers. Before 1.0, Meridian delivers only shared genre-neutral primitives and selected proving packages required by validation projects. `PRG-FWK-001` governs completion and long-term support of all six families after MS-10.

## 2. Goals and Non-Goals

Goals:

- make a new project playable quickly without locking it to private game code;
- provide composable movement, camera, interaction, ability, equipment, damage, inventory, spawn, checkpoint, and session primitives;
- preserve source access, typed extension points, replacement, and zero-cost omission;
- provide templates, examples, tests, accessibility defaults, and diagnostic overlays;
- validate framework APIs against multiple public generic projects.

Non-goals include one giant base-game class, hidden engine privileges, compulsory combat, a proprietary marketplace dependency, forcing Luau, or claiming every family production-ready from one demo.

## 3. Ownership, Dependencies, and Forbidden Edges

Frameworks consume stable contracts from CORE/RUN, DAT, GAM, PHY, ANI, NAV, UI, AUD, NET, COL, and TWO. They publish project-selectable components, systems, commands, schemas, templates, and editor affordances. Games own configured rules, authored data, creative behavior, balance, and forks.

Forbidden edges:

- framework code importing Project Meridian or other private game types;
- direct wgpu, Rapier, OS, device, or ECS-native identifiers in public framework data;
- implicit global singletons that prevent split-screen, server, replay, or editor isolation;
- required modules that cannot be removed from cooked output;
- framework logic bypassing input contexts, accessibility settings, authority, save transactions, or capability policy.

## 4. Planned Contracts

```text
FrameworkDescriptor { id, version, maturity, capabilities, dependencies, modules }
FrameworkModule { id, public_schema, required_services, optional_services, defaults }
PlayerContext { player_id, device_set, viewport, input_context, authority, accessibility }
MovementIntent { frame_id, actor_id, local_axes, actions, camera_basis, constraints }
CameraRigRequest { target, mode, framing, collision_policy, comfort_policy }
AbilityRequest { actor, ability, target, sequence, authority, prediction_policy }
FrameworkExtensionPoint { id, input_schema, output_schema, ordering, budget, trust }
```

Rust is the first implementation and extension language. Optional Luau bindings follow the stable Rust API and use the same generated schema. Framework source and templates remain ordinary project-visible assets or crates, not opaque binaries.

## 5. Composition and Execution

```text
select framework modules in project manifest
-> validate dependency/capability graph
-> generate or instantiate project-visible defaults
-> compile shared schemas and bindings
-> register systems at declared runtime barriers
-> run framework tests and accessibility checks
-> strip unused modules and artifacts during cook
```

Per simulation tick, framework systems consume immutable input/physics/world snapshots, evaluate bounded rules in declared order, enqueue typed commands, and publish inspectable state. They never mutate renderer/audio/network backends directly.

Local multiplayer assigns devices, player contexts, cameras, UI focus, save ownership, and audio listeners explicitly. Dedicated servers compile without presentation modules.

## 6. Character, Camera, and Combat Boundaries

- Cairn owns collision, sweeps, constraints, and final physical state. Framework movement owns desired motion and policy.
- ANI owns pose and root-motion evaluation. Frameworks own state parameters and action selection.
- NAV owns paths and traversability. Framework AI modules own goals and tactical decisions.
- Penumbra owns rendering. Camera modules publish view intent and comfort constraints.
- Shooter/combat modules use typed queries, effects, damage, teams, inventory, and replication contracts; they do not become mandatory engine concepts.

## 7. Failure, Diagnostics, Security, and Accessibility

Invalid module graphs, missing capabilities, order cycles, stale schema, authority violations, budget excess, or unsupported platform tiers fail before activation or disable only the affected optional module. Diagnostics show module/version, system ordering, command/event traces, per-player context, authority decisions, timing, allocations, and strip/cook results.

Templates default to remappable actions, caption/haptic hooks, aim/movement assists where relevant, reduced-motion camera options, hold/toggle choices, timing adjustments, and color-independent cues. Framework editors are keyboard and semantic-accessible.

Untrusted project/framework modules receive explicit capabilities. Multiplayer commands validate authority. Framework packages carry provenance and license records.

## 8. Tiers, Evidence, and Delivery

Shared foundations include player contexts, movement intent, camera rigs, interaction, typed abilities/actions, spawn/checkpoint, and module composition. Specialized family packages are independently selectable. An unselected framework contributes no runtime systems, components, assets, workers, or package chunks.

- `REQ-FWK-001`: optional, composable, Rust-first frameworks over public engine contracts with removal and compatibility evidence.
- `REQ-FWK-002`: explicit local/server/network authority, player contexts, accessibility, and diagnostic behavior with multi-project evidence.
- `REQ-FWK-003`: each production claim requires independent framework workloads, replacement/fork evidence, and zero-cost-disabled proof.
- `WP-FWK-001`: shared player, movement, camera, interaction, and module-composition foundation before 1.0.
- `PRG-FWK-001`: measured completion and maintenance of all six official framework families after MS-10.

Validation projects cover at minimum a first-person interaction scene, third-person movement scene, parkour course, 2D platformer, local multiplayer fixture, dedicated server fixture, and one framework-free project. Evidence records selected modules, hashes, settings, platform, timing, memory, accessibility, and stripped output.

## 9. Examples

End to end: a creator selects first-person interaction; Meridian adds visible Rust modules and data; input creates movement intent; Cairn resolves motion; ANI and Wavefront consume typed outputs; the creator can replace any module.

Failure: a combat module requires networking in an offline build. Validation reports the dependency and offers removal or explicit NET enablement; it does not silently add online services.

Performance debug: a split-screen spike groups cost by player context, framework module, physics query, animation request, and UI viewport.
