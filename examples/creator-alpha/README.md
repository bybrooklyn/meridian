# Creator Alpha public sample

This is a generic public project for Meridian Creator Alpha evidence. It contains
one imported public mesh source, one editable world placement, one editable-model
source document, and one deterministic procedural-placement recipe. It contains no
Project Meridian content.

Run the end-to-end Editor Alpha smoke with an explicit evidence destination:

```text
cargo run -p meridian-editor -- --creator-alpha-smoke \
  --project examples/creator-alpha \
  --evidence target/meridian-evidence/creator-alpha/manual
```

On a native desktop with a presentable surface, render the same public Creator
Alpha workspace through Meridian's native UI raster bridge:

```text
cargo run -p meridian-editor -- --creator-alpha-ui-smoke \
  --project examples/creator-alpha
```

This review smoke verifies native composition of the semantic workspace. An
occluded surface is reported as structural-only evidence and is not a visual
quality result.

The recipe and editable-model source are intentionally public source inputs.
`WP-PRC-001` evaluates the recipe through the strict scalar reference path and
the smoke verifies its source/provenance/license/inspector journey.

The MS-03 `WP-MDL-001` foundation reads the versioned
`meridian.editable-model/v1` source as canonical pretty JSON. Its stable IDs
are fixed-width 32-character hexadecimal strings. The smoke exercises one
typed primitive create, source-object translation, a single bounded edge split
with `TopologyMap` lineage, semantic undo/redo, durable recovery, a
keyboard-accessible source inspector, and a derived Penumbra preview contract.
The model document remains source authority throughout; no renderer resource is
saved as editable source.

This is deliberately partial modeler scope. UVs, broad topology tools,
modifiers, collision/LOD, interchange, and an interactive visual-quality claim
remain out of this sample and continue under `WP-MDL-001`/`WP-MDL-002`.
