# ADR-0008: Isobar, Basalt, and Torsant Boundaries

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Isobar/Basalt scaffold; Torsant planned
- Owners: future Isobar, Basalt, and Torsant workstreams
- Supersedes: none
- Superseded by: none

Amendment notice: [ADR-0017](ADR-0017-alluvium.md) assigns procedural authoring and generated-source ownership to Alluvium. [ADR-0026](ADR-0026-environmental-performance-contracts.md) adds the shared Penumbra participating-media boundary, sparse/multirate policy, typed surface-fluid authority transfer, and authored material/cost facets. Basalt retains terrain and large-world runtime authority; Isobar and Torsant retain their simulation authority.

## Context

The v0.3 roadmap uses Isobar, Basalt, and Torsant as named simulation and
world-system boundaries. Concurrent workspace scaffolds exist for Isobar and
Basalt, but marker crates do not prove product behavior. Torsant is a named
future workstream, not a current crate.

## Decision

Reserve these v0.3 workstream boundaries:

- Isobar owns weather, atmosphere, wind, fog, visibility fields, surface
  wetness, and environment simulation data that crosses rendering, audio,
  gameplay, and vegetation.
- Basalt owns terrain, large-world geometry, ground/rock substrate,
  material-source facets, and runtime world-surface authority. Alluvium owns
  procedural authoring and feeds Basalt, renderer, Cairn, audio, and world
  streaming through schemas.
- Torsant owns fire, fluids, and thermal simulation. It may interact with Cairn,
  Isobar, Basalt, renderer, audio, and gameplay through schemas and snapshots,
  but it does not own their core systems.

These are ownership labels, not implementation claims. They must not create
runtime cost, dependency edges, or package chunks until a phase activates them.

## Current Evidence

- [Delivery roadmap](../../../MERIDIAN_SPECOMENT.md)
- [Isobar scaffold](../../../engine/meridian_isobar/src/lib.rs)
- [Basalt scaffold](../../../engine/meridian_basalt/src/lib.rs)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Future v0.3 subsystem specs must either adopt these names consistently or
supersede this ADR before using them differently. Existing Isobar and Basalt
marker crates are Scaffold evidence only.

## Status Review

Review when dedicated Isobar, Basalt, or Torsant subsystem specs are added or
when any of these workstreams move beyond Scaffold/Planned status.
