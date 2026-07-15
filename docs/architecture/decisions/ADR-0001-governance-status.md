# ADR-0001: Governance and Status Authority

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Implemented documentation governance; enforcement remains partial
- Owners: specs, PLANNING.md, docs/architecture/decisions
- Supersedes: none
- Superseded by: none

## Context

Meridian has engine architecture, game-facing constraints, active work-package
state, research gates, and legacy migration history. Without one status model,
planned systems can be mistaken for shipped behavior.

## Decision

ADRs live under `docs/architecture/decisions/` and record large architecture
choices. The legacy `docs/architecture/decisions/` path is not canonical. Status terms come from
the master specification and must separate `Implemented`, `ImplementedFoundation`,
Partial, Transitional, Planned, Research, Deferred, and Unsupported claims.

The authority order is:

1. the v0.3 owning subsystem specification and adopted ADR;
2. the migration and contradiction register;
3. the delivery roadmap;
4. machine-readable metadata under `specs/registry/`;
5. PLANNING.md for current evidence and active scope;
6. the private Project Meridian creative suite for creative decisions only;
7. migration ledgers for historical rationale;
8. code and evidence as proof of current behavior, not automatic architecture.

## Current Evidence

- [Master specification](../../../specs/MERIDIAN_MASTER_SPEC.md)
- [Delivery roadmap](../../../specs/DELIVERY_ROADMAP.md)
- [Planning ledger](../../../PLANNING.md)
- [Migration register](../../../specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md)
- [v0.1 migration ledger](../../migrations/V0_1_DOCUMENT_MIGRATION.md)

## Intended v0.3 Links

- `specs/MERIDIAN_MASTER_SPEC.md`
- `specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md`
- `specs/DELIVERY_ROADMAP.md`
- `PLANNING.md`

## Consequences

New ADRs must include decision status and implementation status. A code change
that contradicts specs needs a normative update, migration-register entry when
it changes an older decision, and planning update only after the decision is
settled.

## Status Review

Review after v0.3 spec publication and whenever status vocabulary changes.
