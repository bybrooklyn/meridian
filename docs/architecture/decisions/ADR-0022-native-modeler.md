# ADR-0022: Meridian Native Modeler and Optional Blender Companion

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.5
- Implementation status: Planned
- Owners: modeler, editor, assets, build, Alluvium
- Amends: ADR-0009, ADR-0011, ADR-0017
- Supersedes: none

## Context

Requiring Blender makes basic creation inaccessible to users who do not know a professional DCC package. Meridian needs exceptionally approachable native geometry creation without pretending to replace every specialist DCC workflow immediately.

## Decision

The single Meridian application includes a core native 3D modeler. Its first serious version owns editable mesh documents; stable element identity; primitive and vertex/edge/face editing; transforms, snapping, pivots, symmetry, extrude, inset, bevel, loop cut, bridge, fill, merge, normals, basic UVs, material slots, collision/LOD preparation, non-destructive modifiers, full undo, import/export, and Penumbra preview.

The modeler uses Meridian-native versioned source documents. glTF and OpenUSD are interchange; Blender source may remain external with sidecar identity and automatic reimport. Blender integration is optional expert tooling for modeling, sculpting, rigging, animation, and workflows beyond current native maturity.

Alluvium may generate editable model documents and stable generated elements. Manual edits use Alluvium override/reconciliation contracts when regeneration remains live. The modeler owns direct mesh editing; Alluvium owns recipes and generation provenance.

Advanced sculpting, automatic retopology, hair, cloth authoring, and professional character creation are post-foundation programs. Skeletons, skinning, rig controls, and animation editing are owned by `ANI` and shown in the same application.

## Consequences

- `MDL` becomes a core domain rather than an optional DCC adapter.
- A basic modeler must pass before the Project Meridian prototype gate.
- Blender remains supported but cannot be required for first-party beginner workflows.
- Stable topology-element identity and migration are architectural requirements, not editor implementation details.
