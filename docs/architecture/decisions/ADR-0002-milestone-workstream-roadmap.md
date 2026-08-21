# ADR-0002: Milestone and Workstream Roadmap

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Implemented documentation roadmap; execution remains milestone-gated
- Owners: MERIDIAN_SPECOMENT.md, MERIDIAN_SPECOMENT.md, PLANNING.md
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

- [Delivery roadmap](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)
- [Opening slice plan](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `governance/generated/`
- `MERIDIAN_SPECOMENT.md`
- `PLANNING.md`

## Consequences

Each package needs a user-visible result, dependencies, affected files and
formats, tests, benchmark/capture needs, recovery behavior, security and
accessibility impact, and stop condition. Broad labels such as "implement
renderer" or "build physics engine" are invalid.

## Status Review

Review when a milestone gate is promoted, split, or removed.
