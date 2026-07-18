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
requirements. The transitional CPU raster bridge implements only five
categories. Vello 0.9.0 currently targets a different wgpu major and retains
upstream blur/filter work.

## Decision

Adopt a Penumbra-owned direct GPU consumer of Meridian `DisplayList` snapshots
as the production renderer direction. It will use Meridian RHI resources,
capability profiles, diagnostics, caches, and device/surface recovery. Text
remains shaped and rasterized through the private Meridian text adapter;
accessibility remains the independent `SemanticTree` and AccessKit projection.

Retain the bounded full-frame CPU raster bridge for recovery and structural
diagnostics only. It may never establish visual quality or production
performance. Unsupported or high-contrast backdrop effects resolve to the
descriptor's opaque fallback before submission.

Do not adopt Vello 0.9.0. Reconsidering a general 2D renderer requires a new
research gate with version convergence, the same complete corpus, recovery and
cache integration, representative latency/memory distributions, text-quality
review, platform coverage, provenance, and a material maintenance or product
advantage.

## Current Evidence

The [RG-UI-001 decision record](../../benchmarks/RG-UI-001-display-list-renderer.md)
documents the 15-category corpus, five-category fallback coverage, candidate
review, bounded memory calculation, commands, and limits. Unit tests validate
the complete renderer-neutral corpus, deterministic display-list replay,
effect fallback, and explicit rejection rather than silent approximation.

This is an architecture decision, not renderer completion. The direct GPU path
still needs the ten primitive categories absent from the fallback, calibrated
latency/memory/cache evidence, device-loss replay, cross-platform CI, and
presented native visual review.

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
