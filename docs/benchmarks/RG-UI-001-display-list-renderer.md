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
bridge accepts nine categories (rectangle, border, text, glyph run, focus
indicator, rounded rectangle, path, clip push, and clip pop) and explicitly
rejects the other six. It therefore remains a structural/recovery bridge, not
the production renderer.

## Candidate review

| Candidate | Correctness and integration | Platform and recovery | Maintenance, license, and result |
|---|---|---|---|
| Penumbra-owned direct display-list renderer | Preserves Meridian primitives, cache identities, stencil clips, opaque effect fallback, nested isolated full-viewport layers, bounded fixed 3x3 tent backdrops, and the existing cosmic-text raster boundary without translation into another scene model | Shares Meridian RHI `wgpu 30`, capability profiles, resource lifetime, surface/device-loss recovery, diagnostics, offscreen targets, and structural-smoke interfaces | No new dependency or license surface. Selected as the production implementation direction; the current bounded 15-category structural slice and local native `Presented` smoke are not pixel or visual qualification. |
| Vello 0.9.0 | Capable 2D scene model, but upstream still lists blur/filter work and introduces a second scene/cache abstraction | Its crate requires `wgpu 29.0.3` while Meridian owns `wgpu 30.0.0`; adopting it now would duplicate the GPU dependency and complicate common RHI recovery | Permissive `Apache-2.0 OR MIT`, Rust 1.88. Rejected for this package. A future reevaluation requires a new research gate and measured material advantage after version convergence. |
| Full-frame CPU raster plus GPU upload | Deterministic and simple; currently nine of 15 categories are implemented | Backend-portable and useful for recovery/structural diagnostics, but rebuilds a full RGBA image and does not share native vector/image/mesh caches | Retained only as a bounded fallback. The 16,777,216-pixel guard caps its RGBA storage at 67,108,864 bytes before allocator overhead; this is a safety bound, not an acceptable frame budget. |

## Decision evidence and limits

- Display-list validation rejects malformed geometry, unbalanced clips/layers,
  unbounded backdrop sampling, oversized text rasters, and excessive path or
  aggregate primitive counts before rendering.
- Existing 1x/2x frame replay tests establish deterministic retained output.
  Text shaping and rasterization stay in the qualified Meridian text adapter,
  so the renderer choice does not create a second text authority.
- Frame snapshots carry bounded diagnostics for layout, display, semantics,
  routed effects, requests, scale, contrast, motion, and rejected-frame
  recovery. RHI render identity and renderer cache-state tests prove surface
  and device cache invalidation preserves the immutable UI snapshot revision.
- The direct slice produces bounded vertex/index buffers and a final-dimension
  glyph/image atlas without a full-frame CPU raster. It tessellates rounded
  rectangles, flattened curves, concave simple fills, oriented strokes,
  shadows, images, meshes, and stencil clips; bounded full-viewport offscreen
  targets isolate and compose nested layers. Authored sRGB colors are converted
  to linear working values, non-sRGB surfaces receive a typed rejection,
  content and layers use premultiplied alpha, and axis-aligned geometry is
  physically snapped. Adaptive corner/curve tessellation, a one-physical-pixel
  rounded-rectangle fringe, join/cap wedges and sectors, and bounded four-step
  shadow falloff cover the implemented geometry-quality path. A fixed 3x3 tent
  backdrop filter reconstructs the parent-prefix GPU target and shares the
  64 MiB aggregate target guard with isolated layers. Resource, batch,
  geometry, clip-depth, scissor, binding, layer/effect-target memory, nested
  composition, and rejected-frame rollback tests are local structural evidence
  only.
- A local native direct-renderer smoke with two layers and one filter reached
  the RHI `Presented` outcome and captured bounded non-uniform RGBA8 sRGB
  surface pixels. It establishes submission, presentation, and readback
  plumbing on that machine; it is not a golden-image comparison,
  visual-quality review, device-loss replay evidence, cross-platform
  qualification, or a calibrated performance result.
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
cargo test -p meridian-ui-runtime
cargo test -p meridian-rhi --lib
cargo test -p meridian-renderer --features ui-raster-bridge
cargo test -p meridian-renderer --all-features
cargo clippy -p meridian-ui-render -p meridian-renderer --all-targets --all-features -- -D warnings
cargo run -p meridian-renderer --features ui-direct --example ui_direct_smoke
~~~
