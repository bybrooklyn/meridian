# ADR-0016: Redacted Private Benchmark Policy

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Policy adopted; calibrated corpus planned
- Owners: validation, benchmark corpus, Project Meridian integration
- Supersedes: none
- Superseded by: none

## Context

Project Meridian supplies proving-game quality gates, but its full creative
suite, assets, route details, licensed sources, and private captures may be
closed-source. Meridian still needs reproducible evidence without leaking
private material.

## Decision

Benchmark and capture summaries may redact private source, licensed assets,
secrets, and sensitive route/content details. Redaction must preserve the
metadata required to evaluate claims: source checkpoint, BuildId when
available, hardware, OS/driver/runtime, profile/capabilities, corpus hash or
private corpus identifier, warmup/cache policy, repetitions/statistics,
thresholds, known limits, and reviewer/timestamp.

Public documents may link to controlled-access private artifacts without copying
them. Claims that depend on private evidence must say so.

## Current Evidence

- [Testing, benchmarks, and validation](../../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)
- [Project Meridian vertical-slice plan](../../../specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)
- [Principles and scope](../../../specs/PRINCIPLES_AND_SCOPE.md)

## Intended v0.3 Links

- `specs/TESTING_BENCHMARKS_AND_VALIDATION.md`
- `specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md`
- `PLANNING.md`

## Consequences

Redaction cannot hide missing evidence. A private benchmark can support an
internal gate only when enough metadata remains to distinguish calibrated,
visible, structural, unsupported, occluded, and not-run outcomes. Public
performance or competitor claims require publishable evidence or explicit
private-evidence qualification.

## Status Review

Review when PEN-B01/PEN-B02 become executable calibrated corpora.
