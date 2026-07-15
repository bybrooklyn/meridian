# ADR-0011: Source Data Authority

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
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

- [Asset, world, save, and package formats](../../../specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md)
- [Master specification](../../../specs/MERIDIAN_MASTER_SPEC.md)
- [Planning ledger](../../../PLANNING.md)

## Intended v0.3 Links

- `specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md`
- `specs/VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md`
- `specs/API_AND_FILE_FORMAT_EXAMPLES.md`

## Consequences

No source format may persist backend GPU handles, ECS implementation IDs, raw
pointers, OS paths as identity, or third-party public types. A format change
requires fixtures, migration policy, malformed cases, recovery behavior, and
compatibility statement.

## Status Review

Review when source-world documents, final package format, or save transaction
model become active work packages.
