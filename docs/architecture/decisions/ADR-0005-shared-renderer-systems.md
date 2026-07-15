# ADR-0005: Shared Renderer Systems

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
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

- [Rendering and graphics spec](../../../specs/RENDERING_AND_GRAPHICS_SPEC.md)
- [Planning ledger](../../../PLANNING.md)
- [Validation spec](../../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)

## Intended v0.3 Links

- `specs/RENDERING_AND_GRAPHICS_SPEC.md`
- `specs/CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md`
- `specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md`

## Consequences

Renderer features must land through these seams. Backend handles, ECS internals,
asset source paths, and editor widget state cannot become public renderer API.
Debug and expert workflows must expose the shared evidence without changing
runtime authority.

## Status Review

Review after pass-level timing, visible capture, and asset residency evidence.
