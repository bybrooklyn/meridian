# ADR-0006: Meridian RHI, wgpu, and Native Backend Boundary

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
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

- [Rendering and graphics spec](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

The RHI can mature without freezing `wgpu` as product API. Native backend claims
require capability discovery, validation, device/surface recovery, pass timing,
visible captures, and platform evidence.

## Status Review

Review before any native backend prototype crosses from research into production
scope.
