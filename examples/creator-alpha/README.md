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

The recipe and editable-model source are intentionally public source inputs.
`WP-PRC-001` evaluates the recipe through the strict scalar reference path and
the smoke verifies its source/provenance/license/inspector journey. The partial
`WP-MDL-001` package still owns editable-model semantics.
