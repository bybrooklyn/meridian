# ADR-0021: First-Class Two-Dimensional Architecture

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned
- Owners: 2D, rendering, Cairn, editor, assets
- Amends: ADR-0005, ADR-0010, ADR-0014
- Supersedes: none

## Context

Flat 3D geometry does not provide a sufficient 2D engine. It leaks irrelevant cost and fails to define sprite, tile, pixel, layer, 2D-light, 2D-physics, and editor semantics.

## Decision

Meridian adopts a first-class `TWO` domain sharing core assets, input, UI, scheduling, package, command, and diagnostics infrastructure while owning 2D scene/render data and editor workflows. Penumbra provides a dedicated lightweight 2D path through shared render-graph/resource systems. Cairn provides a distinct 2D solver family rather than forcing 2D projects through 3D collision.

Pure 2D profiles omit 3D renderer, terrain, vegetation, volumetric, 3D physics, and unrelated assets unless explicitly selected. Mixed 2D/3D projects remain supported through typed views and compositing.

## Consequences

- A usable 2D baseline is required before Meridian 1.0, but does not block Project Meridian.
- Advanced 2D lighting, skeletal animation, particles, and framework polish may mature later.
- 2D validation must prove pixel stability, deterministic sorting, no hidden 3D initialization, and mixed-mode behavior.
