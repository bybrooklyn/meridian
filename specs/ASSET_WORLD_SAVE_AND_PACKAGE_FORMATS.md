# Asset, World, Save, and Package Formats

[Master index](MERIDIAN_MASTER_SPEC.md) · [Migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md)

Status: normative specification, version 0.2, 2026-07-14.

This document defines Meridian asset identity, source-world storage, compiled
world chunks, save persistence, streaming records, and `.meridian` packages. It
supersedes version 0.1 statements where the [migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md)
marks asset, world, save, or package decisions as refined or replaced.

Rust and TOML blocks in this document are schema/API contracts or pseudocode
unless their status table says the current crate already implements that
surface. Planned snippets are not compile-tested because the corresponding APIs
do not exist yet.

## 1. Current Status

| Area | Status | Evidence and limit |
|---|---|---|
| `AssetId`, runtime metadata, manifest, pack index, dependency validation | Implemented foundation | `meridian_assets` provides deterministic IDs, metadata, required/optional dependencies, canonical manifest text, cross-pack validation, and independent pack-entry lookup. It is not the final asset database. |
| Asset IO, cancellation, residency | Partial | File-backed range reads, cancellation-aware load requests, uncompressed decode, worker job boundaries, and deterministic eviction candidates exist. Zstandard and full importer isolation are planned. |
| World cells and spatial records | Implemented foundation | `meridian_world` owns 128 m default cells, 64-bit world positions, local-origin rebasing, spatial records, visibility category, residency state, and renderer/physics handles. Source-world documents are planned. |
| Streaming scheduler | Partial | `meridian_streaming` has deterministic cell residency states, request priority, cancellation, worker-backed cell loads, and bounded activation queues. Multi-reason scheduling is planned. |
| Saves | Partial | `meridian_save` has versioned envelopes, checksum validation, atomic replacement, backup recovery, one-step migrations, and append-only journal records with truncated-tail recovery. Schema-aware world deltas and rotating recovery heads are planned. |
| `.meridian` package | Planned | Current pack indexes are precursors only. The final package is a chunked, mountable, signed virtual filesystem. |
| ECS storage | Transitional | Current ECS wrapping is implementation evidence only. Persistent IDs and serialized world data must not expose `bevy_ecs` entities. |

## 2. Context

Meridian must serve beginners, expert engine users, headless servers, build
workers, editor plugins, MCP clients, and shipping games from one set of data
contracts. Source worlds must be readable, diffable, recoverable, and migratable.
Shipping packages must be streamable and patchable. Saves must survive crashes
and format migrations without pretending corrupted data is safe.

The authoritative world source is a schema-defined directory. Compiled chunks
are caches or shipping artifacts. A `.meridian` file is a chunked mountable
package, not a monolithic compressed blob and not the editable source of truth.

## 3. Goals

- Use stable IDs for assets, source documents, entities, facets, variants,
  artifacts, and package chunks.
- Keep source assets untouched; store Meridian metadata in sidecars or project
  documents.
- Make world source human-readable where practical, schema-defined everywhere,
  and backed by binary sidecars when needed.
- Compile world, asset, shader, script, material, collision, acoustic, and
  navigation data into independently addressable artifacts.
- Let headless and server builds load only the facets they need.
- Support transactional imports, saves, package creation, recovery, migration,
  inspection, and repair.
- Provide beginner workflows that hide implementation details without removing
  expert access to schemas, chunk tables, diagnostics, and build decisions.

## 4. Non-goals

- Do not use source paths as the only asset identity.
- Do not make `.meridian` the editable project format.
- Do not require Git, Cargo, Blender, shader authoring, AI, or cloud services
  for ordinary beginner workflows.
- Do not expose `bevy_ecs`, `wgpu`, Rapier, editor UI toolkit, or game crate
  types in persistent formats.
- Do not silently open newer or unavailable project schemas for editing.
- Do not claim compression ratio, patch size, load-time, or streaming
  superiority without measured benchmark records.

## 5. Ownership and Crate Boundaries

### 5.1 Owners

| Owner crate or tool | Owns | Must not own |
|---|---|---|
| `meridian-assets` | Runtime asset IDs, metadata, manifest validation, pack lookup, load requests, decoder boundary, residency accounting | Source-world editing UI, source import tool execution, final package signing |
| `meridian-world` | World coordinates, cells, spatial records, source-world schema model, stable world IDs | Renderer resources, physics internals, ECS implementation entities |
| `meridian-streaming` | Runtime cell residency state machine, request scheduling, activation queues | File-format parsing policy, GPU upload handles, authored world edits |
| `meridian-save` | Save envelope, journal, snapshot, migration, recovery, inspect/repair model | Game-specific state interpretation, source-world authoring |
| Planned `meridian-package` | `.meridian` superblock, manifest, chunk index, mount API, patch API, signature coverage | Asset importing, source editing, runtime rendering |
| Editor asset/world tools | Beginner and expert workflows, schema editors, import preview, repair UI | Runtime-only hidden state that bypasses schemas |
| CLI/MCP command registry | Scriptable inspect, validate, diff, repair, export, mount commands | Private AI-only mutation path |

### 5.2 Invalid Dependencies

- Engine crates must not depend on consumer-game crates, including the separate Project Meridian repository.
- Runtime crates must not depend on editor UI crates.
- `meridian-save` must not depend on renderer, audio, Cairn, ECS internals, or
  consumer-game content types.
- `meridian-package` must not depend on source importers; it consumes immutable
  built artifacts and manifests.
- Headless builds must not pull renderer, client UI, audio output, visual
  texture facets, or editor panels unless explicitly enabled.
- Source-world schemas must not contain raw runtime handles, raw pointers,
  backend GPU handles, Rapier handles, `bevy_ecs` entity IDs, or OS paths as
  canonical identity.

## 6. Identity Domains

Meridian separates identity by authority and lifetime.

```rust
pub struct AssetId(pub u128);        // stable project-facing asset family
pub struct SourceId(pub u128);       // stable source document or sidecar
pub struct ArtifactHash(pub [u8; 32]); // content-addressed derived output
pub struct FacetId(pub u64);         // visual, collision, acoustic, etc.
pub struct VariantKey(pub u128);     // platform, quality, locale, feature set
pub struct PackageChunkId(pub u128); // addressable package chunk
pub struct StableEntityId(pub u128); // persistent world object identity
pub struct RuntimeEntityId { pub index: u32, pub generation: u32 }
```

Rules:

- `AssetId` is stable across moves and renames.
- `SourceId` follows the source document, not its absolute filesystem path.
- `ArtifactHash` changes when generated bytes or recorded build inputs change.
- `FacetId` identifies an independently loadable facet of one asset family.
- `VariantKey` records platform, quality tier, locale, compression preset,
  feature pack, shader variant, or server/client role.
- `PackageChunkId` names package storage chunks and may be regenerated on
  repack; it is not a source identity.
- `StableEntityId` is serialized in world and save data; `RuntimeEntityId` is
  never persisted as authority.

## 7. Public Data Structures and APIs

### 7.1 Asset Family

Illustrative schema:

```toml
[asset]
id = "asset:environment/concrete_wall"
display_name = "Concrete Wall"
source = "Sources/structures/concrete_wall.blend"
schema_version = 2

[facets.visual]
artifact = "artifact:visual_mesh"
material = "asset:materials/concrete#visual"
lod_policy = "screen_error"

[facets.collision]
artifact = "artifact:cairn_collision"
lod_policy = "authority_and_distance"

[facets.physical]
material = "asset:materials/concrete#physical"

[facets.acoustic]
material = "asset:materials/concrete#acoustic"

[facets.editor_thumbnail]
artifact = "artifact:thumbnail"
optional = true
```

Required API surface:

```rust
pub trait AssetCatalog {
    fn resolve(&self, id: AssetId, facet: FacetId, variant: VariantKey)
        -> Result<ArtifactRef, AssetDiagnostic>;
    fn dependencies(&self, id: AssetId, facet: FacetId, variant: VariantKey)
        -> Result<Vec<AssetDependency>, AssetDiagnostic>;
}

pub trait AssetRuntime {
    fn request(&self, artifact: ArtifactRef, priority: AssetPriority)
        -> AssetRequestHandle;
    fn poll(&self, handle: AssetRequestHandle) -> AssetRequestState;
    fn cancel(&self, handle: AssetRequestHandle);
}
```

### 7.2 Source World Root

World source layout:

```text
WorldName/
  world.meta
  regions/
  cells/
  entities/
  terrain/
  graphs/
  navigation/
  acoustics/
  weather/
  streaming/
  overrides/
  sidecars/
```

Every source document must declare:

- document ID;
- schema kind and schema version;
- authoring bounds where spatial;
- stable dependencies by ID;
- binary sidecars by `SourceId` or `ArtifactHash`;
- migration status;
- unknown-field preservation policy;
- editor module requirements.

Older editors that lack a required schema or module must refuse editing and
offer inspection/recovery-only tools.

### 7.3 World Cell Document

Illustrative schema:

```toml
[cell]
id = "cell:forest/opening/0004_-0001_0000"
schema = "meridian.world.cell"
schema_version = 1
region = "region:forest/opening"
bounds_meters = { min = [512.0, -64.0, 0.0], max = [640.0, 64.0, 128.0] }

dependencies.assets = [
  "asset:trees/birch_cluster",
  "asset:materials/wet_soil"
]
dependencies.cells = [
  "cell:forest/opening/0003_-0001_0000"
]

[[entities]]
id = "entity:opening_gate"
prefab = "asset:prefabs/rusty_gate"
transform = { position = [530.0, 0.0, 41.0], rotation = [0.0, 0.2, 0.0, 0.98], scale = [1.0, 1.0, 1.0] }
```

### 7.4 Compiled Chunk Header

All compiled chunks use little-endian integer fields unless a later signed
format decision explicitly changes the policy. Offsets are unsigned 64-bit byte
offsets from the beginning of the package data region or sidecar file.

```rust
#[repr(C)]
pub struct ChunkHeader {
    pub magic: [u8; 8],
    pub chunk_type: u128,
    pub schema_version: u32,
    pub flags: u32,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub content_hash: [u8; 32],
}
```

The header must be followed by a dependency table, compression metadata,
alignment padding if needed, and payload bytes. Corrupt chunks are isolated:
loaders reject only the affected chunk and dependent artifacts, then emit a
structured diagnostic.

### 7.5 Save Transaction

Schema-aware save records are planned on top of the implemented journal
foundation.

```rust
pub struct SaveTransaction {
    pub id: u128,
    pub parent: Option<u128>,
    pub schema_version: u32,
    pub clock: SaveClock,
    pub operations: Vec<WorldDelta>,
    pub checksum: [u8; 32],
}

pub enum WorldDelta {
    SetComponent { entity: StableEntityId, component: ComponentId, value: SchemaValue },
    RemoveComponent { entity: StableEntityId, component: ComponentId },
    SpawnEntity { entity: StableEntityId, prefab: Option<AssetId> },
    DespawnEntity { entity: StableEntityId },
    AssetOverride { target: StableEntityId, asset: AssetId, facet: FacetId },
}
```

## 8. Ordered Pipelines and State Machines

### 8.1 Import Transaction

1. Detect source change.
2. Snapshot importer settings, source hashes, tool versions, target platform,
   quality tier, seeds, and feature-pack state.
3. Run importer in an isolated, cancellable, resource-limited process.
4. Produce immutable artifacts, schema metadata, and diagnostics.
5. Validate all facets independently.
6. Compare artifact identities and semantic changes with previous outputs.
7. Preview destructive changes in the editor.
8. Atomically update asset database pointers.
9. Invalidate downstream artifacts.
10. Retain previous artifacts until rollback window or compaction policy expires.

### 8.2 Streaming State Machine

Current `meridian_streaming` state names are preserved and extended:

```text
Unknown
  -> MetadataOnly
  -> CpuCompressed
  -> CpuDecoded
  -> GpuQueued
  -> GpuResident
  -> Active
  -> EvictionCandidate
  -> Unknown
```

The v0.2 scheduler must score requests using:

- spatial cell and region distance;
- rooms/portals;
- camera and gameplay visibility;
- shadow, reflection, acoustic, weather, navigation, and network interest;
- always-loaded sets;
- memory budgets;
- package mount latency;
- user-selected capability tier.

Tie-breaking must be deterministic for equal priority.

### 8.3 Save Atomicity

1. Serialize schema-aware delta into a journal segment.
2. Write and flush segment bytes.
3. Write and flush commit marker.
4. Atomically update the recovery head pointer.
5. Keep previous rotating recovery heads.
6. Compact journal to a snapshot in the background after a validated checkpoint.
7. Preserve unknown records during migration where possible.
8. Mark repaired records explicitly.

The current `SaveJournal` truncated-tail recovery is preserved as a foundation,
but a complete record with bad magic, sequence, size, or checksum is an error,
not a silently skipped event.

### 8.4 Package Build

1. Resolve target platform, feature packs, capability tier, locale, mod policy,
   debug-symbol policy, and signing policy.
2. Freeze asset database snapshot and source-world build graph.
3. Build missing artifacts reproducibly.
4. Validate dependency closure by facet and variant.
5. Partition artifacts into chunks.
6. Compress chunks independently.
7. Build manifest, dependency index, mount table, patch table, license table,
   and recovery index.
8. Hash chunks and canonical manifest.
9. Sign manifest and chunk hash set when signing is enabled.
10. Write superblock, redundant indexes, and data regions.
11. Verify by mounting the just-written package through the runtime mount API.

## 9. Threading, Memory, and Lifetime

- Importers run out-of-process or in constrained worker processes once the
  importer host exists. A crash must not corrupt the asset database.
- Runtime loads use cancellation-aware worker jobs and bounded byte ranges.
- The package mount table is immutable after mount.
- Decoded CPU artifacts are immutable and reference-counted or epoch-owned
  until activation completes.
- GPU uploads are submitted through renderer-owned queues; asset and package
  crates never own backend handles.
- Streaming activation is budgeted by item count and estimated bytes.
- Save writes do not borrow live ECS or physics storage directly; they consume
  schema deltas or immutable snapshots at synchronization points.
- Compaction and package verification may run in background tasks but publish
  results through explicit state changes.

## 10. Persistence, Versioning, and Compatibility

All persistent documents and chunks carry schema versions. Migrations must be
one version step at a time unless an ADR records a safe multi-version shortcut.

Compatibility policy:

- Same major schema: readers may inspect older documents and migrate through
  registered steps.
- Newer required schema: editor refuses mutation and offers inspect/recover.
- Optional unknown field: preserve when round-tripping if the schema marks it
  as extension-preserving.
- Unknown required field: refuse mutation.
- Corrupt source document: keep original bytes, write repair output separately,
  and require user acceptance before replacing source.
- Corrupt package chunk: isolate affected chunk and dependents; keep other
  mount entries available when signature policy permits.

## 11. Editor, CLI, MCP, and Workflows

### 11.1 Beginner Workflow

1. User drags a source file into the Asset Browser.
2. Editor chooses an importer preset and shows plain-language intent.
3. User accepts suggested facets or opens advanced settings.
4. Editor imports in the background and shows diagnostics with fixes.
5. User places the asset in a world cell through the viewport.
6. Save stores source-world edits and a project checkpoint.
7. Export Game builds a `.meridian` package or platform bundle.

The beginner path must not require command-line use, source-control concepts,
shader code, compression settings, chunk tables, or schema editing.

### 11.2 Expert Workflow

Expert panels expose:

- asset family/facet graph;
- artifact hashes and build inputs;
- importer stdout/stderr and structured diagnostics;
- dependency and reverse-dependency graphs;
- world-cell bounds and residency scores;
- save journal and snapshot records;
- package superblock, chunk index, compression, signatures, and patch table;
- schema migration preview;
- mount simulation and corruption repair tools.

### 11.3 CLI Surface

These are planned semantic command names, not evidence of current executable
commands:

| Domain | Commands |
|---|---|
| Assets | `inspect`, `import`, `validate`, `diff`, `rebuild`, `rollback`, `gc` |
| World | `inspect`, `validate`, `migrate`, `diff`, `compile`, `repair` |
| Save | `inspect`, `unpack`, `diff`, `validate`, `repair`, `repack` |
| Package | `inspect`, `list`, `verify`, `extract`, `diff`, `patch`, `mount`, `repack` |

CLI commands must emit machine-readable diagnostics and stable exit codes.

### 11.4 MCP and Agent Surface

MCP tools call the same typed commands as the editor and CLI. They must declare
capabilities for project read, project write, package mount, package extract,
save repair, network access, and signing. No MCP or AI path may bypass schema
validation, save recovery rules, signing policy, or source-control checkpoints.

## 12. Diagnostics, Failure Recovery, and Security

Diagnostics use stable codes, severity, source spans or IDs, affected assets,
suggested fixes, and documentation links.

Required failures:

- missing required dependency;
- duplicate asset ID;
- duplicate package chunk ID;
- corrupt chunk hash;
- unsupported schema version;
- newer editor module required;
- package signature missing, invalid, revoked, or untrusted;
- save journal truncated tail;
- save checksum mismatch;
- world cell bounds invalid;
- streaming activation budget exceeded;
- importer crashed or exceeded resource limit.

Security rules:

- Signing covers the canonical manifest and chunk hashes, not mutable runtime
  caches.
- Optional encryption uses authenticated encryption with independent nonces per
  encrypted chunk.
- Unsigned local development packages are clearly labeled and cannot masquerade
  as release packages.
- Package mount must prevent path traversal, symlink escape, executable launch,
  and native plugin load unless explicitly permitted.
- Repair tools write new artifacts or require explicit replacement approval.

## 13. Capability Tiers and Zero-cost-disabled Behavior

| Tier | Asset/world/package behavior |
|---|---|
| Minimal runtime | Loads only required runtime facets and uncompressed or fast-load chunks. No editor metadata, thumbnails, source, high-quality visual variants, or mod SDK. |
| Headless server | Loads collision, navigation, gameplay, network, and save facets. Does not load textures, shaders, renderer resources, client audio, or editor UI. |
| Editor default | Loads source metadata, thumbnails, diagnostics, selected preview facets, and active viewport world cells. Does not build shipping chunks until requested. |
| Expert/full tools | Enables schema inspectors, package mount, repair, diff, migration, provenance, and artifact graph views. |
| Research | May enable experimental chunking, compression, or streaming strategies only behind explicit flags and benchmark records. |

Disabled feature packs must contribute no persistent background tasks, no
runtime resource registration, no package chunks, and no required dependencies.

## 14. Algorithm Alternatives and Research Gates

| Decision | Alternatives | Gate |
|---|---|---|
| Package chunk partitioning | fixed target size, asset-family grouping, streaming-route grouping, compression-window grouping | Phase 5 package corpus; measure mount latency, patch delta, corruption isolation, and streaming stalls. |
| Compression | none, Zstandard presets, per-category codecs, platform-native texture/audio containers | Phase 5/8; measure load time, size, CPU cost, and memory pressure. |
| World cell sizing | fixed 128 m baseline, per-world override, portal/room partition, hybrid region/cell | Phase 5/8; use B01/B02 and opening-forest traversal. |
| Save compaction | snapshot after N transactions, idle-time compaction, checkpoint-triggered compaction | Phase 5/8; measure load time, crash recovery, and write amplification. |
| Schema encoding | TOML, JSON, purpose-built text, binary sidecars | Phase 5; judge diffability, comments, validation tooling, and migration reliability. |

No losing prototype is deleted without archiving test corpus, results, and the
API seam it exercised.

## 15. Tests, Benchmarks, and Acceptance Evidence

Required tests:

- asset ID stability;
- manifest canonicalization;
- duplicate ID and duplicate name rejection;
- required dependency closure across packs;
- optional dependency absence;
- cancellation during read and decode;
- corrupt chunk checksum;
- streaming state transition validity;
- deterministic request ordering;
- activation queue capacity;
- source-world schema validation;
- unknown-field preservation;
- save atomic replacement;
- save migration chains;
- save journal truncated-tail recovery;
- package mount path traversal rejection;
- package signature verification;
- package repair on corrupt redundant index;
- headless facet exclusion.

Required benchmarks:

- import transaction throughput and memory on representative source assets;
- package build time and output size by preset;
- random-access package mount latency;
- sequential streaming under B01/B02 camera paths;
- save journal append and load replay;
- repair time for representative corruption fixtures.

Acceptance evidence for Phase 5:

- one source-world directory compiles to chunks;
- one asset family exposes visual and collision facets;
- one basic `.meridian` package mounts and verifies;
- one save delta survives simulated crash recovery;
- CLI or editor inspect output identifies package chunks and save records;
- tests and benchmark records are linked from `docs/benchmarks/`.

## 16. Phased Implementation

| Phase | Scope |
|---|---|
| Phase 1-2 | Preserve current runtime asset ID, RHI, render, diagnostics, task, world-cell, and save foundations. |
| Phase 5 | Implement source-world schemas, asset database, importer transactions, basic `.meridian`, save delta tools, and package inspect/verify. |
| Phase 8 | Use these formats in the Project Meridian opening-forest playable slice with save/export evidence. |
| Phase 12+ | Revisit chunking and streaming algorithms after renderer and world complexity grow. |
| Phase 24+ | Add mod package capability manifests and restricted editor distribution. |
| Phase 29 | Freeze stable 1.0 compatibility and long-term migration guarantees. |

## 17. End-to-end Example

1. Artist imports `Sources/trees/birch_cluster.blend`.
2. Importer creates one asset family with visual, collision, acoustic, and
   editor-thumbnail facets.
3. User places the asset in `WorldName/cells/forest_0004_-0001_0000.toml`.
4. Build service compiles the cell into a world chunk and records dependency
   edges to the asset facets.
5. Runtime streams the cell from package chunk `pkgchunk:...` through
   `MetadataOnly -> CpuCompressed -> CpuDecoded -> GpuResident -> Active`.
6. Save journal records that the gate in the cell was opened.
7. Export Game writes a `.meridian` package containing runtime, world chunks,
   visual/collision facets, licenses, and a signed manifest.
8. Package verification remounts the output and resolves the cell plus all
   required dependencies before the export is marked complete.

## 18. Failure and Recovery Example

Scenario: the game crashes while writing a save.

1. The next launch replays the committed save journal.
2. The journal scanner finds a partial final record.
3. Complete records are returned and `truncated_tail = true`.
4. The recovery UI reports the interrupted transaction and offers to continue
   from the last committed state.
5. A later append truncates the incomplete tail before writing a new record.
6. If a complete record checksum fails, the save is not silently repaired; the
   repair tool writes a marked repaired copy or falls back to a rotating head.

## 19. Performance-debug Example

Scenario: entering the field causes a hitch.

The expert streaming panel shows:

- package chunk IDs requested during the hitch;
- compression codec and compressed/uncompressed bytes;
- decode queue wait;
- CPU decode duration;
- GPU upload bytes;
- activation queue bytes;
- cell request reasons, including visibility and gameplay relevance;
- assets missing required lower-quality variants;
- eviction candidates and pinned assets.

The fix must be evidence-driven: for example, repartition one chunk, add a
lower-cost variant, prefetch a required cell earlier, or reduce activation
bytes. The report must include before/after benchmark records rather than a
claim that the new partitioning is generally superior.
