# ADR-0003: Engine and Project Meridian Repository Split

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Implemented repository-boundary policy
- Owners: repository policy, MERIDIAN_SPECOMENT.md
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

- [Repository and crate architecture](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `PLANNING.md`

## Consequences

Public engine APIs must be reusable without Project Meridian. Game integration
evidence is produced in the private repository or through published engine APIs.
Creative source IDs may be referenced only when needed and without copying
private content.

## Status Review

Review before any cross-repository build, fixture, package, or benchmark flow is
introduced.
