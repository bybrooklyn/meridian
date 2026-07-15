# ADR-0003: Engine and Project Meridian Repository Split

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Implemented repository-boundary policy
- Owners: repository policy, specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md
- Supersedes: none
- Superseded by: none

## Context

Meridian is a reusable engine. Project Meridian is the proving game and owns
private creative documents, content, route, pacing, art, and narrative intent.
Combining those repositories would leak closed-source content into the engine
and blur dependency direction.

## Decision

The public engine repository and the private `bybrooklyn/project-meridian`
repository remain separate. Engine crates must not depend on game crates or
content. A local ignored `game/` checkout may exist for integration work, but it
is not staged, copied, or treated as an engine source directory.

Engine specs may preserve engine-facing constraints from the private creative
suite, but they must not import full closed-source documents.

## Current Evidence

- [Repository and crate architecture](../../../specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md)
- [Master specification](../../../specs/MERIDIAN_MASTER_SPEC.md)
- [Opening slice plan](../../../specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)
- [Planning ledger](../../../PLANNING.md)

## Intended v0.3 Links

- `specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md`
- `specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md`
- `PLANNING.md`

## Consequences

Public engine APIs must be reusable without Project Meridian. Game integration
evidence is produced in the private repository or through published engine APIs.
Creative source IDs may be referenced only when needed and without copying
private content.

## Status Review

Review before any cross-repository build, fixture, package, or benchmark flow is
introduced.
