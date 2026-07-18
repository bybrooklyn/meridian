# ADR-0029: Meridian UI Display-List Renderer Direction

- Status: Adopted
- Date: 2026-07-17
- Spec version: v0.5
- Implementation status: Partial
- Owners: UI, renderer, RHI, accessibility
- Supersedes: none
- Superseded by: none

## Context

`RG-UI-001` must select a renderer direction against the real Meridian display
contract without confusing structural coverage with visible or performance
qualification. The retained UI owns 15 primitive categories, independent
semantics, private resource handles, opaque effect fallbacks, and recovery
requirements. The transitional CPU raster bridge implements nine categories.
Vello 0.9.0 currently targets a different wgpu major and retains upstream
blur/filter work.

## Decision

Adopt a Penumbra-owned direct GPU consumer of Meridian `DisplayList` snapshots
as the production renderer direction. It will use Meridian RHI resources,
capability profiles, diagnostics, caches, and device/surface recovery. Text
remains shaped and rasterized through the private Meridian text adapter;
accessibility remains the independent `SemanticTree` and AccessKit projection.

Retain the bounded full-frame CPU raster bridge for recovery and structural
diagnostics only. It may never establish visual quality or production
performance. Unsupported or high-contrast backdrop effects resolve to the
descriptor's opaque fallback before submission. A supported direct profile
uses the bounded fixed 3x3 tent backdrop pass; capability selection remains
explicit and is never silently downgraded.

Do not adopt Vello 0.9.0. Reconsidering a general 2D renderer requires a new
research gate with version convergence, the same complete corpus, recovery and
cache integration, representative latency/memory distributions, text-quality
review, platform coverage, provenance, and a material maintenance or product
advantage.

## Current Evidence

The [RG-UI-001 decision record](../../benchmarks/RG-UI-001-display-list-renderer.md)
documents the 15-category corpus, nine-category fallback coverage, candidate
review, bounded memory calculation, commands, and limits. Unit tests validate
the complete renderer-neutral corpus, deterministic display-list replay,
effect fallback, explicit rejection rather than silent approximation, bounded
frame diagnostics, RHI render identity, and direct-renderer cache recovery
state that preserves immutable snapshot revisions across surface or device
cache invalidation. The current direct slice builds bounded vertex/index data,
a final-dimension glyph/image atlas, rounded and curve-flattened geometry,
stencil clip batches, per-pipeline atlas bindings, typed RHI submission, and
bounded isolated full-viewport targets for nested layer composition across the
15-category contract. It converts authored sRGB colors to linear working
values, rejects non-sRGB surfaces with a typed error, uses premultiplied alpha
for content and layer composition, snaps axis-aligned geometry to physical
pixels, adaptively tessellates corners and curves, adds a one-physical-pixel
rounded-rectangle fringe, emits join/cap wedges and sectors, and uses a bounded
four-step soft-shadow falloff. A fixed 3x3 tent backdrop pass samples
reconstructed parent-prefix GPU targets within the shared 64 MiB aggregate
target guard. Fully clipped draws are omitted rather than becoming unscissored
work. A local native direct-renderer smoke with two layers and one filter
reached the RHI `Presented` outcome and captured bounded non-uniform RGBA8 sRGB
surface pixels; that proves submission, presentation, and readback plumbing,
not golden pixel correctness or review quality.

This is an architecture decision, not renderer completion. The direct GPU path
still needs golden-image qualification, calibrated
latency/memory/cache evidence, device-loss replay evidence, real screen-reader
evidence, cross-platform CI, and presented native visual review.

## Intended v0.5 Links

- `specs/EDITOR_AND_MERIDIAN_UI_SPEC.md`
- `specs/RESEARCH_AND_ALGORITHM_DECISIONS.md`
- `specs/TESTING_BENCHMARKS_AND_VALIDATION.md`
- `specs/registry/research-gates.json`
- `PLANNING.md`

## Consequences

- Meridian does not add a second wgpu major or third-party scene authority.
- UI rendering and runtime rendering share RHI lifetime and recovery policy.
- The compatibility bridge stays deliberately small and honest about gaps.
- Semantics survive rendering failure and third-party types remain private.
- `WP-UI-005` remains `Partial` until its missing implementation and evidence
  rows exist; this ADR alone cannot promote a package or milestone.

## Status Review

Review after the direct renderer implements the full corpus and has fresh
Linux, Windows, macOS, device-loss, visual, and calibrated performance evidence,
or when a new registered gate demonstrates a materially better candidate.
