# RG-UI-001 Display-List Renderer Decision Record

Date: 2026-07-17
Package: `WP-UI-005`
Evidence class: local structural corpus, non-promoting offscreen runner output,
presented review input awaiting a human verdict, and dependency/maintenance
review
Qualification limit: dirty/local output is `Inconclusive`; no visual-quality,
calibrated latency, screen-reader, cross-platform, or package-promotion claim

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
- The feature-gated hidden qualification runner now captures the final direct
  target through an opt-in RHI copy-source target and writes raw RGBA8,
  PNG/metadata, and a portable JSON report. Each checked fixture is versioned
  and binds generator/input schema, corpus hashes, unique case IDs, dimensions,
  raw hashes, and backend/adapter/driver/vendor/device/surface/OS/architecture
  identity. Aggregate runner success requires every exact comparison to pass; a
  missing or different profile is `NotRun`, while a matched-fixture pixel
  difference is `Fail` with a durable failure artifact. Local or dirty source
  output remains `Inconclusive` as qualification evidence even when all case
  comparisons pass. This is profile-bound offscreen renderer correctness
  evidence, not presented visual review.
- The non-default controlled device-loss runner calls the actual backend device
  destruction seam, observes `Destroyed`, proves `DeviceLost` old submission
  and `StaleRhiIdentity` rejection after RHI rebuild, then compares baseline
  and replayed pixels exactly. It records both RHI profiles/identities. It does
  not simulate or qualify hardware, driver, power, or spontaneous loss.
- The local performance runner writes raw JSONL for resource-setup and
  steady-reuse samples. It reports stage wall time, typed timing availability,
  capture diagnostics, and payload accounting; backend allocation, VRAM, and
  driver residency remain explicitly unavailable. Its data are uncalibrated and
  do not set a latency, FPS, memory, or visual-quality claim.
- The visible presented-review runner requests platform focus, presents the
  canonical 2x corpus, and writes PNG/raw RGBA/profile metadata only after a
  presented-surface readback succeeds. Occlusion is an explicit inconclusive
  failure. Successful capture remains `AwaitingHumanReview`; the runner cannot
  approve its own visual output.
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
MERIDIAN_SOURCE_STATE=working-tree MERIDIAN_SOURCE_CHECKPOINT=<path-free-source-id> cargo run -p meridian-benchmark --example ui_direct_qualification --features ui-direct-qualification -- --evidence target/meridian-evidence/ui-direct-qualification/<unique>
MERIDIAN_SOURCE_STATE=working-tree MERIDIAN_SOURCE_CHECKPOINT=<path-free-source-id> cargo run -p meridian-benchmark --example ui_direct_device_loss_replay --features ui-direct-device-loss -- --evidence target/meridian-evidence/ui-direct-device-loss-replay/<unique>
MERIDIAN_SOURCE_STATE=working-tree MERIDIAN_SOURCE_CHECKPOINT=<path-free-source-id> cargo run -p meridian-benchmark --example ui_direct_performance --features ui-direct-qualification -- --evidence target/meridian-evidence/ui-direct-performance/<unique>
MERIDIAN_SOURCE_STATE=working-tree MERIDIAN_SOURCE_CHECKPOINT=<path-free-source-id> cargo run -p meridian-benchmark --example ui_direct_presented_review --features ui-direct-qualification -- --evidence target/meridian-evidence/ui-direct-presented-review/<unique>
~~~

The source variables are caller-declared local provenance labels, not trusted
source or build attestation. A local or dirty run must use
`MERIDIAN_SOURCE_STATE=working-tree`, reports `Inconclusive`, and cannot promote
a package. `clean-commit` is reserved for a separately verified build wrapper
with its exact 40-character commit identity; it still does not turn offscreen
output into cross-platform, screen-reader, visible-review, or accessibility
qualification evidence.
