# ADR-0005: Shared Renderer Systems

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Partial foundations
- Owners: meridian-renderer, meridian-render-graph, meridian-rhi, meridian-shader-tools
- Supersedes: none
- Superseded by: none

## Context

Rendering crosses runtime snapshots, asset residency, shader validation,
materials, graph scheduling, RHI resources, diagnostics, captures, and editor
debug views. These systems need shared contracts instead of hidden backend state.

## Decision

Renderer systems share these boundaries:

- simulation publishes immutable render snapshots;
- render graph owns pass/resource declarations and validation;
- RHI owns backend resources and queue/device behavior;
- renderer owns cameras, lights, shadows, materials, upload planning, and
  pipeline warmup;
- shader tools own manifests, validation, reflection, future IR, and cache keys;
- diagnostics tie pass timing, resource residency, surface outcome, and fallback
  decisions to one frame.

## Current Evidence

- [Rendering and graphics spec](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)
- [Validation spec](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Renderer features must land through these seams. Backend handles, ECS internals,
asset source paths, and editor widget state cannot become public renderer API.
Debug and expert workflows must expose the shared evidence without changing
runtime authority.

## Status Review

Review after pass-level timing, visible capture, and asset residency evidence.
