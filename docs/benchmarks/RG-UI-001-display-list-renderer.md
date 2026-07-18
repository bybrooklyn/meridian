# RG-UI-001 Display-List Renderer Decision Record

Date: 2026-07-17
Package: `WP-UI-005`
Evidence class: local structural corpus and dependency/maintenance review
Qualification limit: no visual-quality, calibrated latency, screen-reader, or
cross-platform performance claim

## Stable boundary and corpus

The evaluated boundary is Meridian's immutable `DisplayList`, its private
glyph/image/mesh cache handles, the independent `SemanticTree`, and RHI
device/surface recovery. The checked corpus contains every current primitive
category: rectangle, border, text, glyph run, rectangular focus indicator,
rounded rectangle, path, image, mesh, clip push/pop, layer begin/end, shadow,
and bounded backdrop with an opaque fallback.

The corpus is constructed and validated by
`qualification_corpus_covers_every_display_primitive_without_overclaiming_bridge_support`.
It covers 15 of 15 primitive categories. The current full-frame CPU raster
bridge accepts five categories (rectangle, border, text, glyph run, focus
indicator) and explicitly rejects the other ten. It therefore remains a
structural/recovery bridge, not the production renderer.

## Candidate review

| Candidate | Correctness and integration | Platform and recovery | Maintenance, license, and result |
|---|---|---|---|
| Penumbra-owned direct display-list renderer | Preserves Meridian primitives, cache identities, clips/layers, opaque effect fallback, and the existing cosmic-text raster boundary without translation into another scene model | Shares Meridian RHI `wgpu 30`, capability profiles, resource lifetime, surface/device-loss recovery, diagnostics, and structural smokes | No new dependency or license surface. Selected as the production implementation direction; the missing ten categories remain implementation work, not claimed support. |
| Vello 0.9.0 | Capable 2D scene model, but upstream still lists blur/filter work and introduces a second scene/cache abstraction | Its crate requires `wgpu 29.0.3` while Meridian owns `wgpu 30.0.0`; adopting it now would duplicate the GPU dependency and complicate common RHI recovery | Permissive `Apache-2.0 OR MIT`, Rust 1.88. Rejected for this package. A future reevaluation requires a new research gate and measured material advantage after version convergence. |
| Full-frame CPU raster plus GPU upload | Deterministic and simple; currently only five of 15 categories are implemented | Backend-portable and useful for recovery/structural diagnostics, but rebuilds a full RGBA image and does not share native vector/image/mesh caches | Retained only as a bounded fallback. The 16,777,216-pixel guard caps its RGBA storage at 67,108,864 bytes before allocator overhead; this is a safety bound, not an acceptable frame budget. |

## Decision evidence and limits

- Display-list validation rejects malformed geometry, unbalanced clips/layers,
  unbounded backdrop sampling, oversized text rasters, and excessive path or
  aggregate primitive counts before rendering.
- Existing 1x/2x frame replay tests establish deterministic retained output.
  Text shaping and rasterization stay in the qualified Meridian text adapter,
  so the renderer choice does not create a second text authority.
- High contrast and unsupported effect profiles resolve bounded backdrops to
  their required opaque color. Focus geometry is rectangular; no decorative
  ring is part of the contract.
- Accessibility remains the independent Meridian semantic tree projected by a
  private platform adapter. Renderer loss cannot remove semantic authority.
- Current local structural tests do not calibrate interactive latency, GPU
  time, cache residency, visual text quality, or driver behavior. Those remain
  required package and native review evidence. The architecture selection does
  not promote `WP-UI-005` or MS-03.

## Reproduction

~~~text
cargo test -p meridian-ui-render
cargo test -p meridian-renderer --features ui-raster-bridge
cargo clippy -p meridian-ui-render -p meridian-renderer --all-targets --features meridian-renderer/ui-raster-bridge -- -D warnings
~~~
