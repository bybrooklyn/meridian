# ADR-0010: Cairn Physics Ownership

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
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

- [Cairn physics spec](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

The Rapier wrapper is transitional evidence only. Cairn completion requires
source provenance, licenses, differential tests, benchmark records, public API
seams, save/handle migration fixtures, and no Rapier public type leakage.

## Status Review

Review when MS-01/MS-04/MS-06 provenance, API, and differential evidence are produced.
