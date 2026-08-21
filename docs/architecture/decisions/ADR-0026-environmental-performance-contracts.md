# ADR-0026: Shared Environmental Performance Contracts

- Status: Adopted
- Date: 2026-07-16
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned/Research; Isobar and vegetation remain Scaffold, Torsant has no crate
- Owners: Penumbra, Isobar, Torsant, Alluvium, and release/performance leads
- Supersedes: none
- Superseded by: none

## Context

Isobar fog/cloud/weather and Torsant smoke/fire/steam can otherwise grow separate
renderer allocations, lighting paths, raymarches, histories, and downgrade
policies. Isobar surface water and Torsant dynamic fluids also need an explicit
authority transfer to prevent double advancement. High-quality fire and fluid
behavior requires coherent authored material facets and cook-time cost insight
without moving runtime authority into Alluvium.

## Decision

- Penumbra owns one path-independent `ParticipatingMediaSourceSnapshot`
  consumption contract and the shared renderer residency, lighting, shadow,
  temporal, compositing, and downgrade resources behind it.
- Isobar and Torsant retain source/simulation meaning and publish immutable
  source fields. Penumbra never advances their simulation.
- Environmental systems use sparse/hierarchical, multirate, budgeted tiles with
  declared influence horizons, update debt, deterministic envelopes, fallbacks,
  and fixed work/memory limits. Presentation interpolation is separate from
  simulation frequency.
- Isobar and Torsant use a typed `SurfaceFluidHandoff`. Exactly one owner advances
  dynamic water in a region/epoch. Promotion, demotion, conservation/error,
  stale state, eviction, and disabled-Torsant behavior are explicit.
- Alluvium owns authored `CombustionMaterialFacet`, `FluidInteractionFacet`, and
  `RuntimeCostManifest` semantics. Torsant owns live solver state; Penumbra owns
  visual resources; observed runtime cost remains evidence rather than source.
- One-way immutable snapshot coupling is the default. Two-way feedback requires
  a separate stability, latency, persistence, downgrade, and workload decision.

## Consequences

The contracts are adopted now so pre-1.0 work can preserve stable seams, but
their advanced convergence and optimization are deferred to `PRG-REL-001` after
MS-10. This ADR does not implement a solver, allocate a volume, activate a work
package, require Torsant for 1.0, or create a universal environmental graph.

Subsystem specifications own the detailed contracts. Disabled optional packs
retain zero tasks, listeners, allocations, GPU resources, dependencies, or
package chunks.

## Validation

Future evidence includes shared-medium source and history tests, sparse/multirate
budget and replay tests, water-handoff ownership and conservation tests,
cost-manifest prediction-versus-observation reports, first-use/stutter evidence,
and complete lower-tier/disabled-path traces.
