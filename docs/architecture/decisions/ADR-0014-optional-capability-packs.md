# ADR-0014: Optional Capability Packs

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Policy implemented in specs; verification partial
- Owners: capability system, package system, all optional feature crates
- Supersedes: none
- Superseded by: none

Amendment notice: [ADR-0017](ADR-0017-alluvium.md) makes Alluvium editor/build support a core capability. Domain adapters and runtime-safe evaluation remain capability-scoped and zero-cost when absent.

## Context

Meridian includes long-horizon systems such as advanced weather, fracture,
OpenXR, multiplayer, community library, cloud agents, native backends, and
advanced rendering. These must not impose cost on projects that do not use them.

## Decision

Optional capability packs declare crates, schemas, package chunks, editor
panels, commands, permissions, platform support, fallbacks, startup cost,
recurring cost, and shipping cost. Disabled means no threads, recurring tasks,
GPU resources, listeners, panels, package chunks, save/network components, or
runtime cost unless authored content directly depends on the pack.

## Current Evidence

- [Principles and scope](../../../specs/PRINCIPLES_AND_SCOPE.md)
- [Repository and crate architecture](../../../specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md)
- [Testing, benchmarks, and validation](../../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)

## Intended v0.3 Links

- `specs/PRINCIPLES_AND_SCOPE.md`
- `specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md`
- `specs/MODDING_AND_COMMUNITY_LIBRARY_SPEC.md`

## Consequences

Every optional pack needs disabled-cost tests and package/dependency evidence.
Activation previews cost, permissions, platform support, and migrations.
Deactivation refuses data loss unless the user explicitly accepts migration or
export.

## Status Review

Review when package manifests and capability documents are implemented.
