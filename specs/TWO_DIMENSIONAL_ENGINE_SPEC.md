# First-Class Two-Dimensional Engine Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [ADR-0021](../docs/architecture/decisions/ADR-0021-first-class-two-dimensional.md) · [Penumbra](RENDERING_AND_GRAPHICS_SPEC.md) · [Cairn](CAIRN_PHYSICS_SPEC.md) · [Frameworks](OFFICIAL_GAMEPLAY_FRAMEWORKS_SPEC.md) · [Meridian UI](EDITOR_AND_MERIDIAN_UI_SPEC.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by ADR-0021. Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-TWO-001` through `REQ-TWO-003`; `WP-TWO-001`.

Current implementation status: Meridian has no dedicated 2D renderer, Cairn 2D solver, tile-map runtime, sprite pipeline, or 2D editor mode.

## 1. Authority and Scope

The `TWO` domain coordinates first-class 2D product behavior across existing owners. It owns 2D scene/document conventions, sprite/tile/shape presentation contracts, 2D camera and ordering semantics, 2D import/authoring workflows, and cross-subsystem acceptance criteria. Penumbra still owns rendering; Cairn owns 2D physics; DAT owns assets/worlds; UI owns runtime/editor widgets; ANI owns animation; FWK owns optional gameplay templates.

Before 1.0 the baseline includes sprites, atlases, tile maps, layers, cameras, pixel-aware scaling, text and vector/UI composition, particles, lighting hooks, Cairn 2D collision/query/joint foundations, 2D scene editing, and one public 2D proving project. Advanced skeletal 2D animation, destructible tile worlds, large-scale 2D streaming, and specialized lighting are later packages or research.

## 2. Goals and Non-Goals

Goals:

- make 2D projects native rather than flattened 3D workarounds;
- share engine services where semantics match while preserving dedicated 2D data and fast paths;
- guarantee deterministic layer/order rules and crisp output under declared pixel policies;
- support mixed 2D/3D projects through explicit composition, cameras, input, and physics boundaries;
- remove unused 3D systems completely from 2D-only builds.

Non-goals are a mandatory 3D scene behind every sprite, one physics solver pretending dimensions are interchangeable, a promise of feature parity with every specialist 2D engine, or forcing runtime UI into sprite scene semantics.

## 3. Data Authority and Contracts

```text
SpriteSource { asset_id, source_region, pivot, pixels_per_unit, sampling, color_space }
SpriteAtlas { id, pages, entries, padding, mip_policy, provenance }
Layer2d { id, order, parallax, visibility, blend_policy, depth_policy }
Camera2d { projection, pixel_policy, viewport, clear, layer_mask, comfort_policy }
TileSet { id, tiles, variants, collision_facets, navigation_facets, animation }
TileMap { id, layers, chunks, coordinate_system, bounds, streaming_policy }
RenderItem2d { transform, sprite_or_shape, layer, order_key, material, clip }
PhysicsFacet2d { shape, material, body_policy, filters, one_way_policy }
```

Stable IDs identify maps, layers, chunks, tiles, atlas entries, animation tracks, and physics facets. Derived atlases and cooked chunks never replace editable sources. Coordinate spaces, origins, units, axis directions, rounding, and pixel snapping are explicit.

## 4. Ownership and Forbidden Edges

| Concern | Authority |
|---|---|
| sprite/tile source, facets, scene documents | DAT/TWO contracts |
| culling, batching, draw order, materials, lights, presentation | Penumbra 2D path |
| bodies, shapes, contacts, queries, joints | Cairn 2D |
| game rules and genre templates | GAM/FWK |
| text, controls, focus, semantics | Meridian UI |
| clip/curve state | ANI where shared; bounded sprite animation contract otherwise |

Forbidden edges include deriving gameplay order from unstable GPU sort behavior, exposing backend textures publicly, mixing 2D and 3D collision spaces implicitly, UI focus from scene draw order, and hidden 3D allocations in 2D-only projects.

## 5. Ordered Pipelines

Asset build:

```text
import source with color/provenance metadata
-> validate pivots, scale, bounds, and transparent padding
-> pack deterministic atlas candidates
-> generate mips/compression by target policy
-> compile tile, collision, animation, and material facets
-> validate reconstruction and bleeding fixtures
-> publish source-to-artifact map
```

Frame:

```text
capture 2D camera and scene snapshot
-> resolve visible chunks/layers
-> cull and form stable order keys
-> batch compatible sprite/shape/text items
-> render lights/effects according to capability tier
-> compose Meridian UI and optional 3D views explicitly
-> publish timing, overdraw, batch, and residency diagnostics
```

Cairn 2D advances on the fixed simulation clock and publishes immutable contact/query snapshots. The 2D renderer interpolates presentation without modifying simulation.

## 6. Threads, Memory, Failure, and Diagnostics

Atlas packing, texture processing, tile baking, collision derivation, and large-map validation run on cancellable workers. Visible item extraction and batching use bounded arenas and immutable snapshots. Chunk and atlas residency are generation-checked.

Failures include atlas overflow, incompatible sampling, invalid tile reference, noninvertible transform, stale chunk, draw-order overflow, missing capability, collision facet error, and budget saturation. Accepted source remains editable and the prior artifact remains available when safe.

Diagnostics expose camera/pixel policy, logical and physical resolution, visible items, batches, texture pages, overdraw, transparent area, tile residency, collision/query cost, layer/order keys, and source/artifact hashes.

## 7. Accessibility, Tiers, and Zero-Cost Behavior

2D tools support keyboard tile painting, semantic layer/tree navigation, scalable previews, color-independent collision/navigation overlays, reduced-motion playback, and textual inspection of coordinates and ordering.

Tiers:

1. sprites, atlases, layers, cameras, tile maps, particles, and baseline Cairn 2D;
2. 2D lights/shadows, advanced batching, streamed maps, richer animation and effects;
3. specialized research such as GPU-generated worlds or advanced deformable 2D.

A 2D-only target includes no 3D pipelines, mesh assets, 3D physics world, 3D visibility state, or unused package chunks. A 3D-only target likewise omits 2D runtime modules.

## 8. Requirements, Evidence, and Delivery

- `REQ-TWO-001`: dedicated, stable 2D scene, rendering, asset, camera, ordering, and Cairn 2D contracts with mixed/2D-only evidence.
- `REQ-TWO-002`: bounded asset/runtime pipelines with pixel, batching, overdraw, physics, memory, and failure diagnostics.
- `REQ-TWO-003`: accessible authoring, deterministic source/artifact identity, and zero-cost-disabled proof.
- `WP-TWO-001`: first-class 2D baseline and one public validation project before 1.0.

Tests cover atlas determinism and bleeding, pixel scaling across DPI/resolutions, stable draw order, tile migration, chunk streaming, 2D collision/query correctness, replay, mixed-view composition, device loss, stripped builds, and accessibility. Reports separate CPU/GPU time, batch count, overdraw, texture memory, tile churn, and Cairn 2D time.

## 9. Examples

End to end: a creator imports a sprite sheet, slices it, paints a tile map, adds Cairn 2D collision, attaches a Rust movement module, and packages a build with no 3D systems.

Failure: atlas compression would corrupt pixel-art edges. Validation rejects the profile and offers a nearest-sampling, no-loss fallback.

Performance debug: a spike reveals excessive layer breaks and transparent overdraw rather than attributing all cost to one opaque 2D pass.
