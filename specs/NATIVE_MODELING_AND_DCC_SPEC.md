# Meridian Native Modeling and DCC Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [ADR-0022](../docs/architecture/decisions/ADR-0022-native-modeler.md) · [Editor and Meridian UI](EDITOR_AND_MERIDIAN_UI_SPEC.md) · [Assets](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Animation](ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by ADR-0022. Documentation maturity: `ResearchReady`. Implementation maturity: `Partial`.
Governing IDs: `REQ-MDL-001` through `REQ-MDL-004`; `WP-MDL-001`; `WP-MDL-002`; `PRG-MDL-001`.

Current implementation status: `WP-MDL-001` is `Partial` after its bounded
MS-03 source delivery. Its MS-03 delivery is limited to a native editable-model
source, stable vertex/edge/face identity and lineage, immutable revisions,
generation-checked selection, primitive creation, transforms, one bounded
topology operation, semantic undo/recovery, an accessible inspector, and a
derived Penumbra preview contract. UVs, broad topology tools, modifiers,
collision/LOD, rigging, interchange, and Blender companion work remain later
scope. A basic native modeler must pass before the Project Meridian prototype
gate.

## 1. Authority and Product Goal

The `MDL` domain owns Meridian's native editable model document, stable mesh-element identity, modeling operations, modifiers, mesh validation, UV/normals authoring, collision/LOD source tools, and beginner-first modeling workflows inside the single Meridian application. DAT owns asset identity/import/cook; Penumbra owns preview rendering; Cairn owns runtime collision; ANI owns skeleton/animation semantics; Alluvium owns procedural recipes and generated identity; DCC integration owns optional external-tool bridges.

The product goal is a simple but powerful creator for people who do not know Blender, while keeping expert interchange and source access. Blender and other DCC tools are optional companions, never prerequisites for first-party workflows.

## 2. Initial Scope and Non-Goals

Required baseline:

- primitives and editable vertex/edge/face meshes;
- selection modes, transforms, snapping, pivots, symmetry, local/global spaces;
- extrude, inset, bevel, loop cut, merge, bridge, fill, delete/dissolve, duplicate, and basic topology repair;
- normals, smoothing, basic UV unwrap/edit, material slots and semantic regions;
- non-destructive modifier foundation, undo/redo, history inspection, collision and LOD source facets;
- glTF and OpenUSD interchange through explicit loss reports;
- Penumbra material/lighting preview and Alluvium-generated editable documents with override preservation.

Advanced sculpting, production retopology, hair/grooming, cloth authoring, full character creation, neural reconstruction, and high-end CAD are post-1.0 `PRG-MDL-001` work. Skeleton, skin, rig, and animation tools share the app but use ANI authority. Format support does not imply lossless round-trip for unsupported semantics.

## 3. Editable Data Authority

```text
ModelDocument {
  id: u128,
  schema: (u16 major, u16 minor),
  coordinate_system: CoordinateSystem,
  objects: Vec<MeshObject>,             // default max 4,096 objects per document
  materials: Vec<MaterialRegionRef>,
  facets: Vec<DerivedFacetRef>,
  history: Vec<ModelOperation>,          // default max 1,000 retained undo steps
  provenance: ProvenanceRef,
}
MeshObject {
  object_id: u128,
  mesh: EditableMesh,
  transform: Transform,
  modifier_stack: Vec<Modifier>,        // default max 32 stacked modifiers per object
  material_regions: Vec<MaterialRegionRef>,
  derived_facets: Vec<DerivedFacetRef>,
}
EditableMesh {
  vertices: Vec<Vertex>,                // default max 2,000,000 vertices per mesh (soft warning)
  edges: Vec<Edge>,
  faces: Vec<Face>,
  corners: Vec<Corner>,
  attributes: Vec<AttributeChannel>,    // default max 8 UV channels, 8 color channels
  stable_element_ids: ElementIdTable,
}
ModelSelection {
  document_generation: u32,
  object_ids: Vec<u128>,
  element_ids: Vec<ElementId>,          // default max 100,000 selected elements
  selection_mode: SelectionMode,        // Vertex | Edge | Face | Object
}
ModelOperation {
  operation_id: u64,
  inputs: ModelSelection,
  parameters: OperationParams,
  preconditions: Vec<Precondition>,
  output_mapping: TopologyMap,
}
Modifier {
  id: u128,
  kind: ModifierKind,
  parameters: ModifierParams,
  capability: CapabilityRequirement,
  determinism: DeterminismMode,         // Deterministic | BestEffort
  evaluation_policy: EvaluationPolicy,  // Live | OnDemand | Baked
}
TopologyMap {
  prior_elements: Vec<ElementId>,
  resulting_elements: Vec<ElementId>,
  split_merge_lineage: Vec<(ElementId, Vec<ElementId>)>,
  orphaned_elements: Vec<ElementId>,
}
InterchangeReport {
  source: FormatId,                     // Native | Gltf | OpenUsd | Blender
  target: FormatId,
  preserved: Vec<SemanticFeature>,
  approximated: Vec<(SemanticFeature, ApproximationNote)>,
  omitted: Vec<(SemanticFeature, OmissionReason)>,
  warnings: Vec<InterchangeWarning>,
  provenance: ProvenanceRef,
}
```

The native document—not a render mesh, imported binary, UI state, or modifier cache—is source authority. Vertices, edges, faces, corners, objects, modifiers, regions, UV islands, and facets have stable IDs or explicit lineage. Every topology-changing operation publishes a `TopologyMap` so selections, overrides, materials, collision facets, Alluvium identity, and agent edits can migrate or become explicit orphans.

## 4. Ownership and Forbidden Edges

| Boundary | MDL owns | Neighbor owns |
|---|---|---|
| DAT/build | editable document and derivation intent | persistent asset IDs, import/cook, artifact manifests |
| Penumbra | preview scene/material requests | render meshes, GPU resources, shading, capture |
| Cairn | collision source facets and preview requests | runtime shapes, queries, physical state |
| ANI | mesh-side skin/morph source and joint placement UI seam | skeleton, skin semantics, clips, graphs, pose |
| Alluvium | editable generated result acceptance and overrides | recipes, evaluation, generated identity, regeneration conflicts |
| External DCC | interchange and companion command contracts | external application's native authority |

Forbidden edges include GPU vertex buffers as editable source, topology operations silently invalidating IDs, destructive import overwriting the only source, modeler code owning runtime physics/animation, editor-only validation, and hidden proprietary formats required to recover a document.

## 5. Ordered Authoring and Evaluation

```text
open immutable document revision
-> validate schema, topology, units, provenance, and references
-> begin typed edit transaction
-> evaluate operation preconditions and bounded preview
-> commit operation plus topology mapping and diagnostics
-> incrementally evaluate affected modifiers/facets
-> validate render/collision/UV/material outputs
-> atomically publish new source revision and derived previews
```

Undo/redo stores semantic transactions and required snapshots, not replay of arbitrary UI events. Long modifiers are cancellable and retain the prior accepted result. Alluvium regeneration enters the same transaction model: unchanged generated IDs update, manual overrides migrate, conflicts are shown, and orphan recovery remains available.

Interchange:

```text
decode in bounded worker
-> normalize coordinate/unit conventions explicitly
-> map supported semantics with stable source provenance
-> record approximations and omissions
-> create candidate native document
-> validate topology/material/facets
-> require user or policy acceptance for lossy conversion
```

## 6. Threads, Memory, Lifetime, and Failure

Interactive selection and small operations provide immediate bounded feedback. Heavy modifiers, UV operations, LOD generation, validation, import/export, and collision derivation use cancellable workers or isolated processes. Documents use immutable revisions, generation-checked selections, copy-on-write/shared topology where justified, bounded preview caches, and explicit memory diagnostics.

Failures include invalid/non-manifold topology, stale selection generation, numerical degeneracy, modifier cycle, budget excess, unsupported interchange feature, corrupt source, lost topology mapping, license conflict, and worker crash. Failed edits do not replace accepted source. Recovery opens the document with quarantined operations, previous revisions, and orphan mappings.

## 7. Diagnostics, Security, Accessibility, and Workflows

Diagnostics identify document/object/element/operation/modifier/source IDs, topology counts, invalid regions, numerical tolerances, evaluation time, memory, cache/invalidation, UV/material/collision impact, and provenance. Untrusted imports are bounded; embedded scripts/macros never execute ambiently; external tool invocation is explicit, capability-scoped, logged, and optional.

Beginner workflow: choose a primitive, manipulate it with visible handles and snapping, use plain-language tools, assign materials, generate simple collision/LOD, preview, and save. Expert workflow: element filters, numeric transforms, topology lineage, modifier graph, custom attributes, UV statistics, validation rules, batch/headless commands, and interchange loss reports.

Every operation is reachable through keyboard/search/typed command as well as pointer tools. Selection is never color-only. Viewport overlays scale, support high contrast, and have textual inspectors. Reduced-motion and precision-input settings apply to gizmos and previews.

## 8. Requirements, Tiers, and Delivery

- `REQ-MDL-001`: versioned editable model documents with stable element identity, topology lineage, semantic undo, migration, and recovery evidence.
- `REQ-MDL-002`: beginner-accessible baseline modeling, UV/normals/material/collision/LOD tools with correctness and usability evidence.
- `REQ-MDL-003`: bounded deterministic modifiers, Alluvium override migration, and Penumbra/Cairn/ANI handoffs without authority duplication.
- `REQ-MDL-004`: optional glTF/OpenUSD/Blender interchange with provenance and explicit loss reports; no proprietary tool requirement.
- `WP-MDL-001`: native editable mesh/modeler foundation required before the Project Meridian prototype.
- `WP-MDL-002`: modifier, UV, collision/LOD, interchange, and optional Blender companion expansion.
- `PRG-MDL-001`: advanced sculpting, retopology, hair, cloth, and character modeling after MS-10.

Baseline tools are core editor capability; advanced modules remain optional. When modeling is absent from a player/runtime target, no modeler code, history, caches, workers, or source-only data ship.

Tests cover topology invariants and fuzzing, stable-ID lineage, undo/redo and crash recovery, stale selections, modifiers, UV/collision/LOD fixtures, Alluvium regeneration conflicts, import/export loss reports, accessibility, memory, cancellation, and stripped player builds. Competitive comparisons are capability targets until repeatable evidence exists.

## 8.1 Work package brief

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status change.

**`WP-MDL-002` — Native modifiers, UV, collision, LOD, and interchange expansion**
Result: the beginner shed-from-primitives workflow (§9's example) gains
full UV editing, a non-destructive modifier stack, collision/LOD source
tools, and glTF/OpenUSD/optional-Blender interchange beyond `WP-MDL-001`'s
bounded MS-03 delivery (current-status line: "UVs, broad topology tools,
modifiers, collision/LOD, rigging, interchange, and Blender companion work
remain later scope"). Owning contracts: `Modifier`, `InterchangeReport`
(§3), extending the existing `ModelDocument`/`EditableMesh`/`TopologyMap`
foundation `WP-MDL-001` already shipped. Entry conditions: `WP-MDL-001`
closed beyond `Partial` — this package is the explicit continuation named in
its own current-status line, not new scope. Deliverables: the non-destructive
modifier foundation (bounded deterministic evaluation, incremental
re-evaluation on affected facets, §5), broad UV unwrap/edit tools, collision
and LOD source facets, and the interchange pipeline (§5: decode in bounded
worker → normalize coordinates/units → map supported semantics with
provenance → record approximations/omissions → candidate document →
validate → require explicit acceptance for lossy conversion) for glTF,
OpenUSD, and optional Blender companion workflows. Non-goals: advanced
sculpting, production retopology, hair/grooming, cloth authoring, full
character creation, neural reconstruction, and high-end CAD stay
`PRG-MDL-001` post-1.0 (§2) — this package does not open any of that scope.
Failure/recovery: a dissolve or modifier change that would orphan a
protected Alluvium override previews the conflict and requires remap,
discard, or cancel, never a silent orphan (§9's failure example, §5's
transaction model). Tests: modifiers, UV/collision/LOD fixtures, Alluvium
regeneration conflicts, import/export loss reports (§8, scoped to this
package's additions). Stop condition: an interchange format that cannot
produce an accurate loss report for its unsupported semantics ships gated
behind explicit user/policy acceptance, never a silent lossy default (§5).
Next unblocked: `PRG-MDL-001`'s post-MS-10 advanced-modeling program, which
depends on this package's modifier/topology foundation but requires its own
separate entry gate.

## 8.2 Work package brief: WP-DCC-001 (medium — Deferred)

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status change; lighter test/evidence detail than `WP-MDL-001`/`WP-MDL-002`
since this package sits further from the current work frontier.

Result: an optional live-link bridge to an external DCC tool (starting with
Blender, per §1's "Blender and other DCC tools are optional companions")
that goes beyond `WP-MDL-002`'s static glTF/OpenUSD/interchange work —
`WP-DCC-001` owns the "External DCC" boundary row in §4 (interchange and
companion command contracts, while the external application retains its own
native authority) as a live round-trip, not a one-shot import/export.
Entry conditions: `WP-MDL-002` closed — a live-link bridge needs the
modifier/UV/collision/LOD foundation and the batch interchange loss-report
machinery (§5's interchange pipeline) already in place; this package adds a
live channel on top, not a replacement path. Deliverables: a companion-tool
command contract (§4) that stays capability-scoped and explicit rather than
ambient (§7: "external tool invocation is explicit, capability-scoped,
logged, and optional"), and live synchronization of edits through the same
`TopologyMap`/provenance machinery `WP-MDL-001`/`WP-MDL-002` already
established (§3, §5), so a live-linked change is not a second, divergent
source of truth. Non-goals: no proprietary format becomes required to
recover a document (§4's forbidden edges — "hidden proprietary formats
required to recover a document" stays forbidden even for a live-linked
companion); this package does not relax `REQ-MDL-004`'s "no proprietary
tool requirement" (§8). Security: external tool invocation remains logged
and capability-scoped like any other untrusted import (§7). Stop condition:
if live synchronization cannot preserve stable element identity/lineage
across a round trip, the bridge falls back to `WP-MDL-002`'s batch
interchange with an explicit loss report rather than silently corrupting
topology mapping. Next unblocked: any future companion-tool integration
beyond Blender, gated the same way.

## 9. Examples

End to end: a beginner creates a shed from primitives, uses extrude/bevel/symmetry, assigns semantic materials, generates collision and LODs, and previews it in the forest without opening another application.

Failure: a dissolve would orphan a protected Alluvium override. The operation previews the conflict and requires remap, discard, or cancel.

Performance debug: a modifier delay separates topology rebuild, UV propagation, collision derivation, render-mesh build, and GPU upload.
