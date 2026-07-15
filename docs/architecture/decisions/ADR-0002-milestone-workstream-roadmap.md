# ADR-0002: Milestone and Workstream Roadmap

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Implemented documentation roadmap; execution remains milestone-gated
- Owners: specs/DELIVERY_ROADMAP.md, specs/IMPLEMENTATION_PLANNING_SPEC.md, PLANNING.md
- Supersedes: none
- Superseded by: none

## Context

Meridian spans renderer, editor, data, physics, scripting, UI, security,
packaging, collaboration, and agent work. A subsystem marathon would delay the
opening forest and encourage broad unverified claims.

## Decision

The roadmap is an evidence-gated milestone graph, not a date schedule.
Workstreams can proceed in parallel only through bounded `WP-*` packages whose
dependencies and evidence are declared. MS-07, the opening playable slice,
remains the central product gate. Advanced renderer research, full Cairn
ownership, external integrations, collaboration, multiplayer, mods, agents,
native backends, and additional languages do not block it unless a narrow
dependency is explicitly adopted.

Each milestone records entry conditions, a critical path, parallel lanes, an
integration checkpoint, exit evidence, and stop conditions. Planning detail is
horizon-based: active work is task-exact, next work is package-exact, and
distant work stays milestone-exact until dependencies become stable. Definition
of Ready and Definition of Done govern package activation and closure.

## Current Evidence

- [Delivery roadmap](../../../specs/DELIVERY_ROADMAP.md)
- [Implementation-planning specification](../../../specs/IMPLEMENTATION_PLANNING_SPEC.md)
- [Planning ledger](../../../PLANNING.md)
- [Opening slice plan](../../../specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)

## Intended v0.3 Links

- `specs/DELIVERY_ROADMAP.md`
- `specs/IMPLEMENTATION_PLANNING_SPEC.md`
- `specs/registry/delivery-plan.json`
- `specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md`
- `PLANNING.md`

## Consequences

Each package needs a user-visible result, dependencies, affected files and
formats, tests, benchmark/capture needs, recovery behavior, security and
accessibility impact, and stop condition. Broad labels such as "implement
renderer" or "build physics engine" are invalid.

## Status Review

Review when a milestone gate is promoted, split, or removed.
