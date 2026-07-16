# API and File-Format Examples

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15

All examples are illustrative specification syntax unless explicitly linked to current compiled code. They define intended shape and invariants, not a promise that the named API or CLI exists. Stable IDs are abbreviated for readability.

## 1. Cargo workspace and feature pack

~~~toml
[workspace]
resolver = "3"
members = ["engine/*", "editor/*"]
exclude = ["game/*"]

[workspace.metadata.meridian]
project = "project.meridian"
capabilities = ["render.raster", "ui.runtime", "audio.basic"]

[package.metadata.meridian.feature-pack]
id = "sim.advanced-weather"
schemas = ["weather.field/v1"]
package-chunks = ["weather-shaders", "weather-presets"]
permissions = []
fallback = "weather.basic"
~~~

The editor performs a lossless semantic TOML edit. Disabling the pack removes its dependency, tasks, resources, panels, and chunks.

## 2. Feature-pack manifest

~~~yaml
schema: meridian.feature-pack/v1
id: render.hardware-rays
version: 0.1.0
crates: [meridian-ray-hw]
capabilities: [render.ray-query]
platforms:
  macos: unsupported
  linux: probe
  windows: probe
fallback: render.raster
cost:
  disabled: { threads: 0, recurring_tasks: 0, package_bytes: 0 }
permissions: []
~~~

## 3. Meridian UI source

~~~mui
document SettingsPanel version 1 {
  column id=settings_root gap=12 padding=16 semantics.group="Settings" {
    heading text=@loc.settings.title level=1
    slider id=master_volume
      label=@loc.audio.master
      bind=settings.audio.master
      range=0..1
      step=0.01
      command=settings.set_audio_master
    button text=@loc.common.back command=ui.close_panel
  }
}
~~~

Persistent IDs and command IDs survive formatting and compiler cache changes.

## 4. Rust UI builder

~~~rust
ui.column(ui_id!("settings_root"), |ui| {
    ui.heading(loc!("settings.title"));
    ui.slider(
        ui_id!("master_volume"),
        bind!(Settings.audio.master),
        0.0..=1.0,
    )
    .label(loc!("audio.master"))
    .command(command_id!("settings.set_audio_master"));
});
~~~

The builder produces the same logical/semantic model as .mui.

## 5. Gameplay API schema and generated Luau view

~~~yaml
module: gameplay.interaction
version: 1
commands:
  - id: gameplay.open_gate
    input:
      gate: EntityRef<Gate>
    errors: [not_found, locked, denied]
    capabilities: [gameplay.mutate]
    thread: fixed_simulation
~~~

~~~lua
export type Gate = EntityRef<"Gate">

export type OpenGateError = "not_found" | "locked" | "denied"

function Interaction.openGate(gate: Gate): Result<(), OpenGateError>
    -- generated runtime binding
end
~~~

## 6. Entity/component creation

~~~rust
let entity = world.commands().spawn(EntityDescriptor {
    persistent_id: ids::GATE_A,
    name: "Forest gate".into(),
});
world.commands().insert(entity, Transform::from_translation([1.0, 0.0, 4.0]))?;
world.commands().insert(entity, Gate { state: GateState::Closed })?;
~~~

The returned runtime reference is generation checked. The persistent ID is serialized.

## 7. Prefab and overrides

~~~yaml
schema: meridian.prefab/v1
id: prefab.forest_gate
entities:
  root:
    persistent_seed: gate-root
    components:
      transform: { translation: [0, 0, 0] }
      gate: { state: closed, locked: false }
      renderable: { asset_family: asset.forest_gate }

instance:
  prefab: prefab.forest_gate
  id: entity.gate.a
  overrides:
    - set: root.gate.locked
      value: true
    - set: root.transform.translation
      value: [1, 0, 4]
~~~

Overrides are semantic operations over stable member IDs, not copied prefab blobs.

## 8. Asset family and facets

~~~yaml
schema: meridian.asset-family/v1
asset_id: asset.forest_gate
source_id: source.models.forest_gate
import:
  importer: gltf
  settings_hash: blake3:...
variants:
  - key: desktop.high
    artifact_hash: blake3:...
    facets:
      visual: { mesh: chunk:mesh_high, material: material.wood.gate }
      physical: { collider: chunk:collider_mid }
      acoustic: { absorption: material.wood.acoustic }
  - key: server
    artifact_hash: blake3:...
    facets:
      physical: { collider: chunk:collider_mid }
~~~

A server never loads the visual facet.

## 9. Unified material

~~~yaml
schema: meridian.material/v1
id: material.wood.gate
facets:
  visual:
    model: pbr.metallic_roughness
    base_color: texture.wood.albedo
    normal: texture.wood.normal
    roughness: 0.72
    metallic: 0.0
  physical:
    friction: 0.62
    restitution: 0.08
  structural:
    density: 620
    tensile_class: wood.soft
  acoustic:
    absorption_bands: [0.12, 0.18, 0.31, 0.48]
  environmental:
    wetness_response: porous
~~~

Facets load independently and use separately versioned schemas.

## 10. Planned Penumbra path, capability, scene, material, and shader contracts

These types are specification-only in v0.5. They are not current Rust APIs.

~~~rust
pub struct RendererPathId(pub StableId);

pub struct RendererPathDescriptor {
    pub id: RendererPathId,
    pub maturity: RendererPathMaturity,
    pub required_capabilities: Vec<CapabilityId>,
    pub supported_material_features: MaterialFeatureSet,
    pub fallback: RendererFallbackPolicy,
}

pub struct GpuCapabilityProfile {
    pub portable_core: PortableGpuCapabilities,
    pub optional: BTreeMap<CapabilityId, CapabilityRecord>,
    pub limits: GpuLimits,
    pub backend: BackendIdentity,
}

pub struct RenderView {
    pub id: RenderViewId,
    pub epoch: Epoch,
    pub camera: CameraSnapshot,
    pub output: OutputDescriptor,
    pub history: TemporalHistoryId,
}

pub struct GpuSceneSnapshot {
    pub epoch: Epoch,
    pub geometry: GeometryTable,
    pub instances: InstanceTable,
    pub materials: MaterialTable,
    pub lights: LightSnapshot,
    pub shadows: ShadowSnapshot,
    pub environment: EnvironmentalFieldSnapshot,
}
~~~

Visibility output and indirect-command streams reference immutable scene/view
epochs. Backends reject stale epochs instead of guessing ownership.

~~~text
MaterialSource
  -> validated MaterialIr
  -> renderer-path lowering
  -> ShaderIr
  -> WGSL lowering during the wgpu era
  -> reflection + binding generation + specialization
  -> pipeline-cache manifest + source map

Future native backend lowerings consume ShaderIr; WGSL remains a valid current
backend language but is not the permanent canonical source authority.
~~~

~~~yaml
schema: meridian.custom-shader-compatibility/v1
shader: shader.water.ripples
renderer_paths: [penumbra.forward-plus]
required_capabilities: [gpu.indirect.table-driven]
fallback: material.standard-water
trust: project-authored
reflection_manifest: blake3:...
source_map: blake3:...
~~~

Artists author one high-level material. Renderer paths may lower it differently
but cannot require duplicated artist-authored materials. Public engine, game,
simulation, asset, and editor contracts expose Meridian descriptors rather than
wgpu or future native-backend types.

## 11. Cairn body, shape, constraint, and destruction

~~~rust
let body = cairn.create_body(BodyDescriptor {
    persistent_id: ids::GATE_BODY,
    motion: Motion::Dynamic,
    transform,
    mass: Mass::FromDensity(620.0),
})?;
let shape = cairn.create_shape(ShapeDescriptor::ConvexHull {
    artifact: artifacts::GATE_COLLIDER,
    material: materials::WOOD_PHYSICAL,
})?;
cairn.attach(body, shape, LocalTransform::IDENTITY)?;
cairn.create_constraint(ConstraintDescriptor::Hinge {
    a: body,
    b: world_anchor,
    axis: Vec3::Y,
    limits: Some((-0.05, 1.7)),
})?;
cairn.attach_structure(body, StructureDescriptor {
    graph: artifacts::GATE_BOND_GRAPH,
    quality: StructureQuality::AuthoredBreakStates,
})?;
~~~

Handles are process-local; persistent body/structure identity is stored separately.

## 12. DSP graph and adaptive music

~~~yaml
schema: meridian.audio-graph/v1
sample_rate: device
nodes:
  forest_bed: { type: stream, asset: audio.forest.bed, loop: true }
  forest_gain: { type: gain, value: 0.75 }
  master: { type: limiter, ceiling_db: -1.0 }
edges:
  - [forest_bed.out, forest_gain.in]
  - [forest_gain.out, master.in]
outputs: [master.out]

music:
  clock: samples
  states:
    silence: { stems: [] }
    title: { stems: [audio.music.title], transition: next_bar }
~~~

The graph compiler rejects cycles without an explicit delay.

## 13. Isobar weather and atmosphere state

~~~yaml
schema: meridian.isobar-state/v1
id: isobar.opening
seed: 318472
states:
  midnight_still:
    fog_density: 0.62
    wind_profile: wind.forest.low
    precipitation: none
  field_edge:
    fog_density: 0.38
    wind_profile: wind.field.gusts
transitions:
  - from: midnight_still
    to: field_edge
    trigger: state.reached_field_edge
    duration_seconds: 18
    curve: ease_in_out
~~~

Penumbra, vegetation, and audio consume one immutable Isobar snapshot. Basalt
may contribute terrain shelter and surface data through an explicit versioned
input; Isobar does not read renderer state.

### 13.1 Planned environmental convergence contracts

Illustrative only; these types and schemas are not implemented.

~~~text
ParticipatingMediaSourceSnapshot {
  source_id, owner, epoch, bounds, representation,
  extinction, scattering, emission, phase, velocity,
  temporal_validity, quality_priority, budget_class
}

SurfaceFluidHandoff {
  region_id, source_epoch, target_epoch,
  owner: IsobarCoarse | TorsantDynamic,
  initial_or_settled_state,
  mass_or_depth_error, stale_after, fallback
}
~~~

Penumbra owns participating-media GPU residency, lighting, shadow, temporal,
and compositing behavior. Isobar and Torsant retain source/simulation meaning.
Exactly one system advances dynamic surface water for a region and epoch.

## 14. Alluvium terrain and vegetation recipe

~~~yaml
schema: meridian.procedural-recipe/v1
id: recipe.public.representative_forest
version: 1
seed: 9917
determinism: stable
evaluation:
  allowed: [interactive_preview, authoritative_bake]
  runtime_safe: false
outputs: [geometry, field, vegetation, collision, navigation, acoustic, scene_fragment]
graph:
  - id: slope
    op: basalt.terrain_slope
  - id: path_distance
    op: field.distance_to_spline
    input: route.public_test
  - id: suitable
    op: field.ecological_suitability
    inputs: [slope, path_distance, field.moisture, field.canopy_shelter]
  - id: trees
    op: vegetation.place
    input: suitable
    random_stream: ordinary_trees
overrides: overrides/public_representative_forest.moverride
license_policy: public_engine_fixture
~~~

`.mproc` is reserved for recipe source and `.mfield` for derived field
artifacts. Encoding is not frozen by this example. Dirty propagation
regenerates only affected cells plus declared halos, preserves stable generated
identity and overrides, and emits provenance/license disposition.

Torsant has no crate or required opening-slice schema in v0.5. A future first
implementation package must define versioned fire/fluid/thermal source events,
solver tiers, coupling latency, recovery, and zero-cost-disabled behavior before
adding `meridian-torsant`.

### 14.1 Planned material facets and runtime-cost manifest

Illustrative only; no schema or predictor is implemented.

~~~yaml
schema: meridian.runtime-cost-manifest/v1
build_id: blake3:qualified-build
profile: public-standard-client
calibration:
  version: uncalibrated-example
  corpus: public-representative-forest
  uncertainty: required
regions:
  - id: region.public.forest-edge
    predicted:
      geometry_residency: required-metric
      texture_residency: required-metric
      participating_media_pages: required-metric
      pipelines: required-metric
      weather_tiles: required-metric
      torsant_tiles: not-enabled
    causes:
      - recipe.public.representative_forest
      - material.public.wet-soil
    downgrade:
      - reduce-participating-media-tier
      - reduce-vegetation-tier
~~~

Runtime evidence records observed values and prediction error. It does not
silently rewrite this derived manifest or treat an uncalibrated prediction as a
budget guarantee.

## 15. Gameplay State Flow

~~~yaml
schema: meridian.state-flow/v1
id: flow.opening
initial: forest_walk
states:
  forest_walk:
    on:
      reached_field_edge:
        target: title_transition
        actions: [save.checkpoint_opening, audio.enter_title]
  title_transition:
    on:
      transition_complete: { target: complete }
  complete:
    terminal: true
assertions:
  - path_exists: [forest_walk, complete]
  - completion_requires_no: discovery.optional_document_count
~~~

## 16. Narrative text and graph integration

~~~yaml
schema: meridian.narrative-flow/v1
id: narrative.opening
beats:
  ambient_forest:
    kind: environmental
    required: true
    state: forest_walk
  optional_note_a:
    kind: document
    required: false
    interaction: interaction.note_a
    text: text.note_a
  field_reveal:
    kind: environmental
    required: true
    trigger: reached_field_edge
validation:
  no_objective_checklist: true
  optional_beats_gate_completion: false
~~~

## 17. Network replication schema

~~~yaml
schema: meridian.replication/v1
component: Transform
authority: server
fields:
  translation:
    quantize: { range: [-8192, 8192], bits: 20 }
    reliability: unreliable_sequenced
    relevance: spatial
  rotation:
    quantize: smallest_three_16
    reliability: unreliable_sequenced
frequency_hz: 20
prediction: local_owner
~~~

Project Meridian does not enable this schema.

## 18. Mod manifest

~~~yaml
schema: meridian.mod/v1
id: community.example.lanterns
version: 1.2.0
game: { id: example.game, range: ">=1.0 <2.0" }
api: { range: ">=1.3 <2.0" }
license: CC-BY-4.0
packages: [lanterns.meridian]
entry_points:
  luau: [mods/lanterns/init.luau]
capabilities:
  - gameplay.api: example.decorations
  - world.namespace: mod.community.example.lanterns
dependencies: []
~~~

## 19. Agent command

~~~json
{
  "command": "world.set_property",
  "version": 1,
  "input": {
    "object_id": "isobar.opening",
    "property": "states.midnight_still.fog_density",
    "value": 0.58
  },
  "expected_version": "blake3:...",
  "transaction": "preview",
  "checkpoint": true
}
~~~

Preview returns semantic diff, affected artifacts, performance/security impact, and required approval.

## 20. Save journal

~~~text
SaveHeader { magic, format_version, game_id, schema_set_hash }
TransactionBegin { tx_id, previous_commit_hash }
Record { schema_id, schema_version, owner_id, payload_length, payload_hash, payload }
TransactionCommit { tx_id, record_count, transaction_hash }
~~~

A transaction without a valid commit is ignored during recovery.

## 21. .meridian package

~~~text
Superblock {
  magic, format_version, package_id, package_version,
  manifest_chunk, primary_index_chunk, flags
}
ChunkHeader {
  chunk_id, kind, codec, uncompressed_length,
  stored_length, content_hash, alignment
}
IndexEntry {
  logical_path_or_object_id, chunk_id, offset, length,
  capabilities, platform_variant
}
~~~

Chunks compress and verify independently. Mounting reads bounded superblock/index data; it does not decompress one giant stream.

## 22. Planned Marquee campaign and export manifest

Illustrative only; no Marquee schema or runtime type is implemented.

~~~yaml
schema: meridian.promotion-campaign/v1
campaign_id: public-generic-launch
build_id: blake3:qualified-build
state: Draft
sources:
  - id: capture-001
    class: GameplayCapture
    hash: blake3:source
    rights: project-owned
    consent: not-applicable
    spoiler: none
    embargo: none
    transform_policy: crop-grade-overlay-transcode
claims:
  - text: "Keyboard and controller supported"
    source: qualified-feature-record
    build_id: blake3:qualified-build
    locale: en-US
    expires: 2027-07-15
profiles:
  - id: generic-16x9
    revision: 1
    official_source_url: https://example.invalid/profile-authority
    verified: 2026-07-15
ai:
  mode: TextAnalysisOnly
  generated_media: false
  modified_media: false
  human_review_required: true
publishing: ExportOnly
capture: ManualImportOnly
~~~

`ReleaseReady` additionally requires an approval record over the exact BuildId, source/claim/profile/template/tool hashes, locale set, and AI suggestions. Export writes local files and a `PromotionManifest`; it does not contact a service.

## 23. Planned competitive baseline and claim

Illustrative only; `PRG-REL-001` is deferred until after MS-10 and no comparison
or superiority evidence exists.

~~~yaml
schema: meridian.competitive-baseline/v1
study: public-forest-iso-quality
claim_class: IsoQualityPerformance
meridian_build_id: blake3:qualified-build
comparator:
  product: exact-public-product-name
  version: exact-version
  access_basis: documented-and-authorized
hardware_profile: exact-record-reference
corpus_hash: blake3:public-licensed-corpus
camera_input_hash: blake3:recording
feature_parity:
  status: review-required
settings:
  internal_resolution: exact
  output_resolution: exact
  upscaler: exact
  frame_generation: exact
  dynamic_resolution: exact
warmup:
  cold: required
  warm: required
  first_use: required
quality:
  structural: required
  reference: as-applicable
  blinded_perceptual: required
  temporal: required
evidence:
  raw_samples: required
  captures: required
  lower_tiers: required
claim:
  wording: pending-independent-review
  expires_at: required
  retraction_state: Draft
~~~

An impossible parity row produces `Inconclusive`. A new comparator version,
Meridian BuildId, driver, corpus, material setting, or quality method produces a
new immutable record rather than modifying the result.
