# Specification-Suite Agent Guidance

The root [AGENTS.md](../AGENTS.md) is the canonical operational policy for the repository. This file applies additional rules while editing specs/.

## Scope

- Preserve the authority order in [MERIDIAN_MASTER_SPEC.md](MERIDIAN_MASTER_SPEC.md) and [SPEC_MIGRATION_AND_CONTRADICTIONS.md](SPEC_MIGRATION_AND_CONTRADICTIONS.md).
- Do not copy subsystem text into many files. Put the normative contract in its owning spec and link to it.
- Do not upgrade Planned, Transitional, Research, Deferred, Constructed, or Structural Smoke to Implemented/Validated without fresh evidence.
- Preserve Project Meridian creative decisions and link to the private `bybrooklyn/project-meridian` source when engine integration depends on them; never copy the full closed-source documents into this repository.
- Keep phase numbers/names and the Phase 8 critical path synchronized with [IMPLEMENTATION_PHASES.md](IMPLEMENTATION_PHASES.md).

## Required content for a major subsystem

Include context, goals, non-goals, ownership/dependencies/invalid edges, public types/data, ordered pipeline/state machine, threading/memory, persistence/compatibility, editor and CLI/MCP workflows, accessibility, diagnostics/recovery, security, capability tiers/disabled cost, tests/benchmarks, phases/research gates, and end-to-end/failure/performance-debug examples.

If an item is not applicable, say why. A heading with vague future prose is not sufficient.

## Architecture changes

For a new or changed decision:

1. identify affected requirement and documents;
2. update the owning spec;
3. update the contradiction register if an older statement changes;
4. update research/ADR when algorithm or dependency evidence changes;
5. update format/API examples and validation;
6. update implementation phases and PLANNING only if sequencing/current work changes;
7. add a legacy banner or migration fixture where needed.

## Citations

Use current primary sources: official specifications/docs/repositories and original papers. Record verification date. State what a source supports and what remains Meridian judgment. Do not use a source link as a license or production-readiness claim.

## Audit

Before sign-off:

- all required spec filenames exist;
- relative links resolve;
- duplicate headings/statuses are coherent;
- Telepo, permanent egui/Rapier ownership, multiple initial languages, opaque one-file worlds/packages, mandatory cloud, and all-engine-first gating are not revived;
- provisional numbers are labeled and calibration is described;
- code/API examples are marked illustrative unless compiled;
- markdown/whitespace checks pass;
- root AGENTS and PLANNING agree with the suite.

Use the mandatory phase sign-off in root AGENTS.md.
