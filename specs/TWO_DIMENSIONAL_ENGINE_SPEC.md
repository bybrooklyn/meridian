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
SpriteSource {
  asset_id: u128,
  source_region: (u32 x, u32 y, u32 w, u32 h), // pixel-space rect within the source image
  pivot: Vec2,                                  // normalized 0.0-1.0
  pixels_per_unit: f32,                         // default reference 100.0 (project-configurable)
  sampling: SamplingMode,                       // Nearest | Bilinear
  color_space: ColorSpace,                      // Srgb | Linear
}
SpriteAtlas {
  id: u128,
  pages: Vec<AtlasPage>,                        // default max 16 pages, 4,096x4,096 px each
  entries: Vec<AtlasEntry>,                     // default max 8,192 packed entries per atlas
  padding: u16,                                 // px, default reference 2
  mip_policy: MipPolicy,                        // None | Generate { max_levels: u8 }
  provenance: ProvenanceRef,
}
Layer2d {
  id: u32,
  order: i32,                                   // stable sort key, lower draws first
  parallax: Vec2,                                // 1.0 = locked to camera, 0.0 = static
  visibility: bool,
  blend_policy: BlendPolicy,                    // Alpha | Additive | Multiply
  depth_policy: DepthPolicy,                    // OrderOnly | PseudoDepth { z: f32 }
}
Camera2d {
  projection: OrthographicProjection,
  pixel_policy: PixelPolicy,                    // FreeScale | IntegerScale | PixelPerfect
  viewport: (u32 x, u32 y, u32 w, u32 h),
  clear: ClearPolicy,
  layer_mask: u32,                              // bitflags over Layer2d.id groups
  comfort_policy: ComfortPolicy,
}
TileSet {
  id: u128,
  tiles: Vec<TileDef>,                          // default max 8,192 tile definitions per set
  variants: Vec<TileVariant>,
  collision_facets: Vec<PhysicsFacet2d>,
  navigation_facets: Vec<NavFacet2d>,
  animation: Vec<TileAnimation>,
}
TileMap {
  id: u128,
  layers: Vec<TileMapLayer>,                    // default max 32 layers
  chunks: Vec<ChunkRef>,                        // default chunk size 32x32 tiles
  coordinate_system: CoordinateSystem,          // XyUp | XzUp, origin, axis direction
  bounds: (i32 min_x, i32 min_y, i32 max_x, i32 max_y),
  streaming_policy: StreamingPolicy2d,
}
RenderItem2d {
  transform: Transform2d,
  sprite_or_shape: SpriteOrShapeRef,
  layer: u32,
  order_key: u64,                               // (layer << 32 | stable within-layer key)
  material: MaterialRef,
  clip: Option<ClipRect>,
}
PhysicsFacet2d {
  shape: Shape2d,                               // Circle | Box | Capsule | Polygon { max 16 verts }
  material: PhysicsMaterialRef,
  body_policy: BodyPolicy2d,                    // Static | Kinematic | Dynamic
  filters: CollisionFilter,                     // category/mask bitflags, u32 each
  one_way_policy: OneWayPolicy,                 // None | OneWayPlatform { normal: Vec2 }
}
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

## 8.1 Work package brief

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status change.

**`WP-TWO-001` — First-class 2D rendering, assets, scene editing, Cairn 2D, and proving project**
Result: a creator imports a sprite sheet, slices it, paints a tile map, adds
Cairn 2D collision, attaches a Rust movement module, and packages a build
with no 3D systems present (§9's example) — the tier-1 baseline in §7
(sprites, atlases, layers, cameras, tile maps, particles, baseline Cairn 2D).
Owning contracts: `SpriteSource`, `SpriteAtlas`, `Layer2d`, `Camera2d`,
`TileSet`, `TileMap`, `RenderItem2d`, `PhysicsFacet2d` (§3). Entry
conditions: none of TWO exists yet (current-status line); consumes Penumbra's
render-graph/material contracts and Cairn's 2D physics contracts through
their public seams (§4's ownership table), not their full 3D-domain
maturity. Deliverables: the asset build pipeline (§5: import with color/
provenance metadata → validate pivots/scale/bounds/padding → pack
deterministic atlas candidates → generate mips/compression → compile tile/
collision/animation/material facets → validate reconstruction/bleeding →
publish source-to-artifact map), the frame pipeline (capture 2D camera/scene
snapshot → resolve visible chunks/layers → cull/order → batch → render by
capability tier → compose Meridian UI and optional 3D views explicitly →
publish diagnostics), Cairn 2D collision/query/joint foundations advancing on
the fixed simulation clock, one 2D scene editing surface, and one public 2D
proving project (§8, `REQ-TWO-001`). Non-goals: no advanced skeletal 2D
animation, no destructible tile worlds, no large-scale 2D streaming, no
specialized lighting — those are tier 2/3 (§7) or later research, not this
package. Forbidden edges: no deriving gameplay order from unstable GPU sort
behavior, no exposed backend textures, no implicit mixing of 2D/3D collision
spaces, no UI focus derived from scene draw order (§4). Failure/recovery:
atlas overflow, incompatible sampling, invalid tile reference, or stale chunk
keeps the accepted source editable and the prior artifact available when
safe (§6) — e.g. atlas compression that would corrupt pixel-art edges is
rejected with a nearest-sampling, no-loss fallback offered, not silently
applied (§9). Tests: the full §8 list (atlas determinism/bleeding, pixel
scaling across DPI/resolutions, stable draw order, tile migration, chunk
streaming, 2D collision/query correctness, replay, mixed-view composition,
device loss, stripped builds, accessibility). Stop condition: a 2D-only
target that cannot prove zero 3D pipeline/mesh/physics-world/package-chunk
cost (§7) blocks release of that build profile, not the whole package. Next
unblocked: tier-2 2D lights/shadows/streaming work, and any `WP-FWK-001`
family (e.g. the strategy/simulation and 2D genre foundation) that depends on
a real 2D baseline to build on.

## 9. Examples

End to end: a creator imports a sprite sheet, slices it, paints a tile map, adds Cairn 2D collision, attaches a Rust movement module, and packages a build with no 3D systems.

Failure: atlas compression would corrupt pixel-art edges. Validation rejects the profile and offers a nearest-sampling, no-loss fallback.

Performance debug: a spike reveals excessive layer breaks and transparent overdraw rather than attributing all cost to one opaque 2D pass.
