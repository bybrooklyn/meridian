# Hand-listed on-disk formats — the cross-check input

Built by reading the crates that write files, **not** by grepping constant names. The grep is
a discovery tool; this list is what turns its output into a completeness claim.

A row is one *format*. A magic-plus-version pair is one row, not two.

| # | Format | Magic | Version constant | Owner crate |
|---|---|---|---|---|
| 1 | Package container | `MERIDN\0\0` | `FORMAT_VERSION` | `meridian-package` |
| 2 | Save file | `MSAV` | — | `meridian-save` |
| 3 | Save journal | `MJNL` | `JOURNAL_FORMAT_VERSION` | `meridian-save` |
| 4 | Compiled cell | `COMPILED_CELL_MAGIC` | `COMPILED_CELL_VERSION` | `meridian-streaming` |
| 5 | Visual facet | `VISUAL_FACET_MAGIC` | — | `meridian-assets` |
| 6 | Collision facet | `COLLISION_FACET_MAGIC` | — | `meridian-assets` |
| 7 | Recipe | — | `RECIPE_SCHEMA_VERSION` | `meridian-alluvium` |
| 8 | Model | — | `MODEL_VERSION` | `meridian-modeler` |
| 9 | UI document | — | `UI_DOCUMENT_SCHEMA_VERSION` | `meridian-ui-core` |
| 10 | UI source (`.mui`) | — | `UI_DOCUMENT_SOURCE_FORMAT_VERSION` | `meridian-ui-core` |
| 11 | Build protocol | — | `BUILD_PROTOCOL_VERSION` | `meridian-build` |
| 12 | Workspace state | — | `WORKSPACE_STATE_VERSION` | `meridian-editor-core` |
| 13 | Golden fixture | — | `GOLDEN_FIXTURE_VERSION` | benchmark harness |
| 14 | Fixture mesh import | — | `FIXTURE_MESH_IMPORTER_VERSION` | benchmark harness |
| 15 | Benchmark result | — | **none — JSON Schema only** | `schemas/benchmark-result.schema.json` |

## Why this file exists

The previous enumeration used
`grep -rhoE '[A-Z_]*(SCHEMA_VERSION|FORMAT_VERSION|_VERSION|_MAGIC)'` and reported 18. Both
`_MAGIC` and `_VERSION` require a leading underscore, so a bare `MAGIC` could not match — and
rows 1 and 2 above are exactly that. The package container and the save file are the two
serialized outputs whose corruption would be least recoverable, and `PH-AUTH-006`'s stop
condition is *"Stop if decomposition changes serialized output."* The measurement built to arm
that catch could not see the two formats it most exists for.

The same criterion also swept in three non-formats: `CARGO_PKG_VERSION` (a Cargo env var),
`ENGINE_VERSION` (literally `env!("CARGO_PKG_VERSION")`) and `GENERATOR_VERSION` (the specoment
projection generator's own version string).

Row 15 is the other structural gap: a format defined by a JSON Schema with no Rust constant is
invisible to any constant-name grep, however the pattern is written.

## The rule this establishes

A discovery command produces candidates. A completeness claim requires the candidates be
reconciled against a set built by a different method — here, reading the crates that write
files. `WP-V1-CENSUS-001` asserts the enumeration matches this table, so a format added later
without updating both fails rather than passing silently.
