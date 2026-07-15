# ADR-0012: Luau as Initial High-level Runtime

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Planned with runtime/ECS precursors
- Owners: meridian-gameplay-schema, meridian-gameplay-runtime, meridian-luau
- Supersedes: none
- Superseded by: none

## Context

Meridian needs a high-level gameplay runtime for designers, interaction/state
flows, hot reload, debugging, save-safe state, and generated bindings. Multiple
initial runtimes would multiply API, sandbox, docs, and compatibility work.

## Decision

Luau is the single initial high-level runtime. Rust remains available for native
engine/game modules. C#, Anorak, Python, and mixed-language architecture are
later research. Luau binds the same schema as Rust and runs inside explicit
isolation domains with deny-by-default capabilities.

Scripts do not receive ambient filesystem, process, native library, network,
clipboard, debug, or agent access.

## Current Evidence

- [Gameplay, narrative, and scripting spec](../../../specs/GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md)
- [Delivery roadmap](../../../specs/DELIVERY_ROADMAP.md)
- [Opening slice plan](../../../specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)

## Intended v0.3 Links

- `specs/GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md`
- `specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md`
- `specs/SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md`
- `specs/DELIVERY_ROADMAP.md`

## Consequences

Runtime APIs need generated Rust and Luau bindings, stable IDs, hot-reload
migration rules, sandbox escape tests, budget diagnostics, save fixtures, and
opening completion tests. Additional language support must not destabilize the
initial runtime.

## Status Review

Review when MS-04/MS-06 runtime, sandbox, and binding evidence exists.
