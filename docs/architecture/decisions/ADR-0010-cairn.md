# ADR-0010: Cairn Physics Ownership

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Transitional Rapier wrapper; Cairn planned
- Owners: meridian-physics, future Cairn crates
- Supersedes: none
- Superseded by: none

## Context

The current physics layer has useful grounded-controller and Rapier-wrapper
evidence. Meridian still needs owned physics APIs, provenance, data-oriented
storage, determinism modes, structural semantics, and future advanced
simulation without preserving Rapier's public API.

## Decision

Cairn is Meridian's in-tree physics family. It follows a provenance-first hard
fork or rewrite path from pinned Rapier plus selected Box2D study/ports where
licensing and evidence justify the work. Rapier API compatibility is not a
goal. Public Cairn APIs use Meridian descriptors, stable IDs, generation-checked
handles, snapshots, commands, and diagnostics.

## Current Evidence

- [Cairn physics spec](../../../specs/CAIRN_PHYSICS_SPEC.md)
- [Delivery roadmap](../../../specs/DELIVERY_ROADMAP.md)
- [Planning ledger](../../../PLANNING.md)

## Intended v0.3 Links

- `specs/CAIRN_PHYSICS_SPEC.md`
- `specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md`
- `specs/TESTING_BENCHMARKS_AND_VALIDATION.md`
- `specs/DELIVERY_ROADMAP.md`

## Consequences

The Rapier wrapper is transitional evidence only. Cairn completion requires
source provenance, licenses, differential tests, benchmark records, public API
seams, save/handle migration fixtures, and no Rapier public type leakage.

## Status Review

Review when MS-01/MS-04/MS-06 provenance, API, and differential evidence are produced.
