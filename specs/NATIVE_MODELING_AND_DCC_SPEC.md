# Meridian Native Modeling and DCC Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [ADR-0022](../docs/architecture/decisions/ADR-0022-native-modeler.md) · [Editor and Meridian UI](EDITOR_AND_MERIDIAN_UI_SPEC.md) · [Assets](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Animation](ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by ADR-0022. Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-MDL-001` through `REQ-MDL-004`; `WP-MDL-001`; `WP-MDL-002`; `PRG-MDL-001`.

Current implementation status: Meridian has no native editable mesh document, modeling kernel, modeler viewport/tools, modifier stack, UV editor, rigging tools, or Blender companion. A basic native modeler must pass before the Project Meridian prototype gate.

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
ModelDocument { id, schema, coordinate_system, objects, materials, facets, history, provenance }
MeshObject { object_id, mesh, transform, modifier_stack, material_regions, derived_facets }
EditableMesh { vertices, edges, faces, corners, attributes, stable_element_ids }
ModelSelection { document_generation, object_ids, element_ids, selection_mode }
ModelOperation { operation_id, inputs, parameters, preconditions, output_mapping }
Modifier { id, kind, parameters, capability, determinism, evaluation_policy }
TopologyMap { prior_elements, resulting_elements, split_merge_lineage, orphaned_elements }
InterchangeReport { source, target, preserved, approximated, omitted, warnings, provenance }
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

## 9. Examples

End to end: a beginner creates a shed from primitives, uses extrude/bevel/symmetry, assigns semantic materials, generates collision and LODs, and previews it in the forest without opening another application.

Failure: a dissolve would orphan a protected Alluvium override. The operation previews the conflict and requires remap, discard, or cancel.

Performance debug: a modifier delay separates topology rebuild, UV propagation, collision derivation, render-mesh build, and GPU upload.
