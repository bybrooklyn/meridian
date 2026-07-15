# ADR-0004: Penumbra Clustered Forward+ Baseline

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
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

- [Rendering and graphics spec](../../../specs/RENDERING_AND_GRAPHICS_SPEC.md)
- [Delivery roadmap](../../../specs/DELIVERY_ROADMAP.md)
- [Planning ledger](../../../PLANNING.md)
- [Validation spec](../../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)

## Intended v0.3 Links

- `specs/RENDERING_AND_GRAPHICS_SPEC.md`
- `specs/DELIVERY_ROADMAP.md`
- `specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md`

## Consequences

Forward+ may be claimed only after depth prepass, clustering, visible captures,
and pass timing evidence exist. Current structural smoke proves construction
boundaries, not image quality. Visibility-buffer work must use a later research
gate with shared corpora and losing-prototype archive.

## Status Review

Review after MS-01, MS-04, and MS-05 Forward+ visible capture and
PEN-B01/PEN-B02 calibration evidence.
