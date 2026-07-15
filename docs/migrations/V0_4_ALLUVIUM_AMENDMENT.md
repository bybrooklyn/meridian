# Meridian v0.4 Alluvium Amendment Migration Ledger

Version 0.4 · 2026-07-15 · Active migration record

This ledger maps every v0.3 procedural-authoring heading and every binding Alluvium amendment subject to a v0.4 authority or explicit disposition. Disposition vocabulary remains `Preserved`, `Split`, `Merged`, `Superseded`, or `Retired`.

## 1. v0.3 Procedural Specification Headings

| v0.3 heading | Disposition | v0.4 destination |
|---|---|---|
| Context | Merged | Alluvium sections 1-2, ADR-0017 |
| Goals and Non-Goals | Preserved | Alluvium section 2 |
| Ownership and Crate Boundaries | Superseded | Alluvium sections 1 and 3; subsystem ownership tables |
| Public Types and Data Structures | Superseded | Alluvium sections 4-5 |
| Compiler and Authoring Pipeline | Preserved | Alluvium sections 6-7 |
| Threading, Memory, and Lifetime | Split | Alluvium sections 6-7 and 14 |
| Persistence, Versioning, and Compatibility | Split | Alluvium sections 7, 9, 10, and 13 |
| Editor, CLI, MCP, and Workflows | Superseded | Alluvium section 12; agent/editor specifications |
| Diagnostics, Failure Recovery, and Security | Split | Alluvium sections 6, 10, 12, and 16 |
| Capability Tiers and Zero-Cost-Disabled Behavior | Superseded | Alluvium sections 6 and 14; ADR-0017 |
| Algorithm Alternatives and Research Gates | Superseded | Alluvium section 15; `RG-PRC-001`, `RG-PRC-002` |
| Tests, Benchmarks, and Acceptance Evidence | Preserved | Alluvium section 16; validation and workload registries |
| Delivery Mapping | Superseded | Alluvium section 17; roadmap and delivery-plan registry |
| Examples | Preserved | Alluvium section 18; API/file-format examples |

Mapped rows: 14. Unmapped rows: 0.

## 2. Binding Amendment Subjects

| Amendment subject | Disposition | v0.4 authority |
|---|---|---|
| Alluvium name and core-engine position | Preserved | Alluvium section 1; ADR-0017 |
| Competitive capability targets | Preserved | Alluvium section 2; principles and research specs |
| No proprietary first-party tool requirement | Preserved | Alluvium sections 1-2 and 12-13 |
| Permissive/public-domain dependency and replacement policy | Preserved | Alluvium sections 1, 10, 15; provenance policy |
| Authoring versus runtime authority | Preserved | Alluvium section 3; owning subsystem specs |
| Specialized domains on common evaluator | Preserved | Alluvium sections 4-6 |
| Typed recipes and outputs | Preserved | Alluvium section 4; API examples |
| Spatial field system | Preserved | Alluvium section 5 |
| Preview, bake, and runtime-safe modes | Preserved | Alluvium section 6 |
| Incremental evaluation and cache classes | Preserved | Alluvium section 7 |
| Strict, stable, and opportunistic determinism | Preserved | Alluvium section 8 |
| Explicit random substreams | Preserved | Alluvium section 8 |
| Stable generated identity and non-destructive overrides | Preserved | Alluvium section 9 |
| Provenance, licensing, redistribution, and cooker policy | Preserved | Alluvium section 10; data/security specs |
| Terrain and hybrid world-surface authoring | Split | Alluvium section 11.1; Basalt specification |
| Vegetation and tall-grass proving ground | Split | Alluvium section 11.2; vegetation specification; private game docs |
| Cross-facet materials and causal weathering | Split | Alluvium section 11.3; renderer/Cairn/audio/Isobar/Torsant specs |
| Semantic spline infrastructure and structures | Split | Alluvium section 11.4; Basalt/Cairn/data specs |
| Editable AI-generated recipes | Split | Alluvium sections 10 and 12; agent specification |
| Text/inspector first and visual graph later | Split | Alluvium section 12; editor specification; roadmap |
| CLI, headless, CI, and batch operation | Split | Alluvium section 12; build/agent specs |
| Testing and structural output comparison | Split | Alluvium section 16; validation specification |
| SIMD/GPU/tiled/sparse performance policy | Split | Alluvium section 14; `RG-PRC-001` |
| Open interchange and Meridian runtime formats | Split | Alluvium section 13; data/DCC specs |
| `.mproc` and `.mfield` | Preserved | Alluvium section 13; API examples |
| Avoid premature branded extensions | Preserved | Alluvium sections 1 and 13 |
| Capability progression | Superseded | `WP-PRC-001` through `WP-PRC-010`; delivery roadmap |
| Private Project Meridian target list | Split | private game production/opening docs; public sanitized `WP-PRC-002` |
| Initial non-goals | Preserved | Alluvium section 2 |

Mapped rows: 28. Unmapped rows: 0.

## 3. Contract and Identifier Migration

| v0.3 authority | Disposition | v0.4 destination |
|---|---|---|
| Procedural Authoring Specification title | Superseded | The Alluvium Engine title at the same canonical path |
| future `meridian-procedural` name | Retired | reserved future `meridian-alluvium`; no crate created |
| `meridian.procedural-graph/v1` definition-only example | Superseded | logical `meridian.procedural-recipe/v1`; no implemented compatibility promise |
| single `REQ-PRC-001` coverage | Split | `REQ-PRC-001` through `REQ-PRC-009` |
| single oversized `WP-PRC-001` | Split | `WP-PRC-001` through `WP-PRC-010` |
| no PRC research gate | Superseded | `RG-PRC-001` and `RG-PRC-002` |
| no PRC risk entries | Superseded | `RISK-PRC-001` through `RISK-PRC-010` |
| Alluvium work mostly in MS-08 | Superseded | minimum foundation and proving recipes required by MS-05; later packages remain MS-08/MS-09/MS-10 |
| Penumbra benchmark report v0.3 | Superseded | report v0.4 with recipe/evaluator/provenance fields |

Mapped rows: 9. Unmapped rows: 0.

## 4. Private Boundary

| Material | Disposition | Authority |
|---|---|---|
| General forest, grass, terrain, weathering, infrastructure, and structure capabilities | Preserved | public engine specifications and generated benchmark contracts |
| AMI facilities and proprietary environmental composition | Preserved | private Project Meridian documentation |
| Private recipes, seeds, hero overrides, route constraints, logos, documents, and assets | Preserved | private Project Meridian repository only |
| Public evidence | Preserved | generated surrogate, controlled private source identifier/hash, redacted differences |

Mapped rows: 4. Unmapped rows: 0.

## 5. Validation Contract

`meridian-spec list-unmapped` must report zero. Current normative documents and registries use v0.4. Historical v0.3 ADR and migration records remain legal in `docs/architecture/decisions/` and `docs/migrations/`; independent schema/report identifiers remain at their own version unless explicitly migrated above. Private-content validation remains unwaivable.

Total mapped rows: 55. Total unmapped rows: 0.
