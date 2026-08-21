# ADR-0014: Optional Capability Packs

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
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

- [Principles and scope](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Every optional pack needs disabled-cost tests and package/dependency evidence.
Activation previews cost, permissions, platform support, and migrations.
Deactivation refuses data loss unless the user explicitly accepts migration or
export.

## Status Review

Review when package manifests and capability documents are implemented.
