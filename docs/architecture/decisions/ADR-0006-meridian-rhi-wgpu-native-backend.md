# ADR-0006: Meridian RHI, wgpu, and Native Backend Boundary

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Implemented RHI foundation; native backend planned/research
- Owners: meridian-rhi, platform backend crates
- Supersedes: none
- Superseded by: none

## Context

Meridian needs a backend-neutral public graphics boundary while the current
implementation uses `wgpu`. Future native backend work must not require game or
editor code to change public types.

## Decision

Meridian public graphics APIs use Meridian-owned descriptors, handles,
capabilities, diagnostics, and recovery states. `wgpu` remains the private first
backend. Native Metal/Vulkan/Direct3D backend experiments are allowed only
behind the RHI seam and research gates.

No game-facing, source-data, package, scripting, or editor command API exposes
`wgpu` or native backend handles.

## Current Evidence

- [Rendering and graphics spec](../../../specs/RENDERING_AND_GRAPHICS_SPEC.md)
- [Repository and crate architecture](../../../specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md)
- [Planning ledger](../../../PLANNING.md)

## Intended v0.3 Links

- `specs/RENDERING_AND_GRAPHICS_SPEC.md`
- `specs/REPOSITORY_AND_CRATE_ARCHITECTURE.md`
- `specs/TESTING_BENCHMARKS_AND_VALIDATION.md`

## Consequences

The RHI can mature without freezing `wgpu` as product API. Native backend claims
require capability discovery, validation, device/surface recovery, pass timing,
visible captures, and platform evidence.

## Status Review

Review before any native backend prototype crosses from research into production
scope.
