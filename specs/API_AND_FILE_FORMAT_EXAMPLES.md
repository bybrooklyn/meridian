# API and File-Format Examples

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

Version 0.2 · 2026-07-14

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

## 10. Cairn body, shape, constraint, and destruction

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

## 11. DSP graph and adaptive music

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

## 12. Weather state and field

~~~yaml
schema: meridian.weather/v1
id: weather.opening
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

Rendering, vegetation, and audio consume one immutable WindSnapshot.

## 13. Procedural forest graph

~~~yaml
schema: meridian.procedural-graph/v1
domain: vegetation
id: graph.opening_forest
seed: 9917
nodes:
  slope: { op: terrain.slope }
  path_distance: { op: mask.distance, input: route.main }
  candidates: { op: scatter.poisson, radius: 1.8 }
  suitable:
    op: filter.all
    inputs: [candidates, { slope_lt: 0.55 }, { path_distance_gt: 1.2 }]
  output:
    op: vegetation.emit
    asset_family: asset.tree.pine
    input: suitable
overrides: overrides/opening_forest.yaml
~~~

Dirty propagation regenerates only affected cells plus declared halo.

## 14. Gameplay State Flow

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

## 15. Narrative text and graph integration

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

## 16. Network replication schema

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

## 17. Mod manifest

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

## 18. Agent command

~~~json
{
  "command": "world.set_property",
  "version": 1,
  "input": {
    "object_id": "weather.opening",
    "property": "states.midnight_still.fog_density",
    "value": 0.58
  },
  "expected_version": "blake3:...",
  "transaction": "preview",
  "checkpoint": true
}
~~~

Preview returns semantic diff, affected artifacts, performance/security impact, and required approval.

## 19. Save journal

~~~text
SaveHeader { magic, format_version, game_id, schema_set_hash }
TransactionBegin { tx_id, previous_commit_hash }
Record { schema_id, schema_version, owner_id, payload_length, payload_hash, payload }
TransactionCommit { tx_id, record_count, transaction_hash }
~~~

A transaction without a valid commit is ignored during recovery.

## 20. .meridian package

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
