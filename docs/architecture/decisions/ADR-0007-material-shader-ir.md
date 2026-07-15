# ADR-0007: Material and Shader IR Authority

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Transitional WGSL foundation; IR planned
- Owners: meridian-shader-tools, meridian-renderer, meridian-assets
- Supersedes: none
- Superseded by: none

## Context

Beginners should author materials without shader code, while experts need
inspectable sources, reflection, variants, cache keys, and backend diagnostics.
Current WGSL shader paths are useful implementation evidence but not the final
authoring model.

## Decision

Source materials and shader graphs are authoritative. A Meridian material/shader
IR will sit between authoring documents and backend shader outputs. WGSL remains
the current transitional implementation source until IR validation, reflection,
debugging, translation, and cache behavior are proven.

Visual material is one facet of a broader material asset. Physical, acoustic,
collision, and gameplay facets use separate schemas and IDs.

## Current Evidence

- [Rendering and graphics spec](../../../specs/RENDERING_AND_GRAPHICS_SPEC.md)
- [Asset, world, save, and package formats](../../../specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md)
- [Planning ledger](../../../PLANNING.md)

## Intended v0.3 Links

- `specs/RENDERING_AND_GRAPHICS_SPEC.md`
- `specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md`
- `specs/PROCEDURAL_AUTHORING_SPEC.md`

## Consequences

Shader cache keys are disposable derived data. Source material documents,
graphs, facets, and build inputs remain authoritative. No backend shader
language can become a persistent project format without migration policy.

## Status Review

Review when the shader IR work package is activated or a backend translation
prototype closes its research gate.
