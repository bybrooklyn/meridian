# ADR-0004: Penumbra Clustered Forward+ Baseline

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned baseline with implemented renderer foundations
- Owners: meridian-renderer, meridian-render-graph, meridian-rhi
- Supersedes: none
- Superseded by: none

## Context

The opening forest needs readable dark scenes, many local lights, vegetation,
transparency, fog, cascaded shadows, diffuse IBL, and measured fallback behavior.
The current renderer has PBR, shadows, diffuse IBL, RHI, render graph, and
structural native smoke foundations, but not the full opening renderer.

## Decision

Penumbra is the MS-01, MS-04, MS-05, and MS-08 renderer baseline: clustered Forward+ with a depth
prepass, direct PBR materials, cascaded shadows, diffuse IBL, fog/atmosphere,
vegetation paths, UI/compositor integration, pass timing, and visible capture
evidence.

Deferred alternatives include deferred-only rendering, visibility-buffer
rendering, virtual geometry, dynamic GI, hardware ray tracing, and mandatory
vendor upscalers. These remain research or later-milestone options.

## Current Evidence

- [Rendering and graphics spec](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)
- [Validation spec](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Forward+ may be claimed only after depth prepass, clustering, visible captures,
and pass timing evidence exist. Current structural smoke proves construction
boundaries, not image quality. Visibility-buffer work must use a later research
gate with shared corpora and losing-prototype archive.

## Status Review

Review after MS-01, MS-04, and MS-05 Forward+ visible capture and
PEN-B01/PEN-B02 calibration evidence.
