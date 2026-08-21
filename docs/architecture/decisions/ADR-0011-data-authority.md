# ADR-0011: Source Data Authority

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Partial data foundations
- Owners: meridian-assets, meridian-world, meridian-save, future meridian-package
- Supersedes: none
- Superseded by: none

Amendment notice: [ADR-0017](ADR-0017-alluvium.md) defines Alluvium recipes, parameters, seeds, and override layers as source authority. Generated fields and artifacts remain derived unless promoted through an explicit source transaction.

## Context

Meridian source projects must be inspectable, recoverable, diffable, migratable,
and usable by editor, CLI, build workers, runtime, packages, servers, and agents.
Derived artifacts are important but cannot become hidden authority.

## Decision

Schema-defined source documents are authoritative. Generated artifacts,
compiled chunks, shader caches, built assets, indexes, and package lookup tables
are rebuildable caches or shipping artifacts. Stable IDs cross source, save,
package, network, and VCS boundaries; runtime handles remain process-local and
generation checked.

Unknown optional fields round-trip when safe. Unknown required fields fail with
diagnostics and leave source untouched.

## Current Evidence

- [Asset, world, save, and package formats](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

No source format may persist backend GPU handles, ECS implementation IDs, raw
pointers, OS paths as identity, or third-party public types. A format change
requires fixtures, migration policy, malformed cases, recovery behavior, and
compatibility statement.

## Status Review

Review when source-world documents, final package format, or save transaction
model become active work packages.
