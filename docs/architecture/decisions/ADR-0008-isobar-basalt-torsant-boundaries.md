# ADR-0008: Isobar, Basalt, and Torsant Boundaries

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Isobar/Basalt scaffold; Torsant planned
- Owners: future Isobar, Basalt, and Torsant workstreams
- Supersedes: none
- Superseded by: none

Amendment notice: [ADR-0017](ADR-0017-alluvium.md) assigns procedural authoring and generated-source ownership to Alluvium. Basalt retains terrain and large-world runtime authority; the remaining Isobar and Torsant boundaries are unchanged.

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

- [Delivery roadmap](../../../specs/DELIVERY_ROADMAP.md)
- [Isobar weather and atmosphere spec](../../../specs/ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md)
- [Basalt terrain and large-world geometry spec](../../../specs/BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md)
- [Torsant fire, fluids, and thermal simulation spec](../../../specs/TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md)
- [Procedural authoring spec](../../../specs/PROCEDURAL_AUTHORING_SPEC.md)
- [Repository and crate architecture](../../../specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md)
- [Isobar scaffold](../../../engine/meridian_isobar/src/lib.rs)
- [Basalt scaffold](../../../engine/meridian_basalt/src/lib.rs)

## Intended v0.3 Links

- `specs/ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md`
- `specs/BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md`
- `specs/TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md`
- `specs/DELIVERY_ROADMAP.md`

## Consequences

Future v0.3 subsystem specs must either adopt these names consistently or
supersede this ADR before using them differently. Existing Isobar and Basalt
marker crates are Scaffold evidence only.

## Status Review

Review when dedicated Isobar, Basalt, or Torsant subsystem specs are added or
when any of these workstreams move beyond Scaffold/Planned status.
