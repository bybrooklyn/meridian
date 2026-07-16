# Animation, Cinematics, and Facial Systems Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Assets and worlds](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Gameplay](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) · [Native modeler](NATIVE_MODELING_AND_DCC_SPEC.md) · [Penumbra](RENDERING_AND_GRAPHICS_SPEC.md) · [Marquee](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md) · [Delivery](DELIVERY_ROADMAP.md)

Status: version 0.5 normative architecture, 2026-07-15.

Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-ANI-001` through `REQ-ANI-003`; `WP-ANI-001`; `WP-ANI-002`; `PRG-ANI-001`.

Current implementation status: Meridian has no production animation graph, retargeter, cinematic sequencer, facial solver, or performance-capture pipeline. This document defines planned contracts and evidence gates only.

## 1. Authority, Goals, and Non-Goals

The `ANI` domain owns animation-source documents, skeleton and clip semantics, runtime pose evaluation, retargeting, animation graphs, cinematic sequencing, root-motion output, inverse-kinematics requests, and later facial-performance data. Gameplay owns decisions and commands; Cairn owns physical state; Penumbra owns skinning resources and presentation; the native modeler owns editable geometry and rig-authoring surfaces without becoming the runtime animation authority.

Goals:

- provide a general animation baseline before specialized cinematic and facial systems;
- keep imported, authored, generated, and captured motion editable and provenance-tracked;
- separate deterministic state selection from presentation-only interpolation;
- support streamed clips, layered graphs, root motion, IK, events, retargeting, and rollback-safe state;
- expose beginner workflows and expert graph, curve, event, and profiling tools in the single Meridian application.

Non-goals before 1.0 include a proprietary motion-capture service, neural facial synthesis, a film-compositing suite, mandatory cloud processing, or a promise to match dedicated commercial animation packages. `PRG-ANI-001` contains advanced facial, performance-capture, cinematic-character, and virtual-production work after 1.0 qualification.

## 2. Ownership and Forbidden Edges

| Producer or consumer | ANI authority | Other authority |
|---|---|---|
| Modeler/DCC | skeleton, skin, morph, clip, and marker import contracts | editable mesh, topology, rig tools, source interchange |
| Gameplay/frameworks | state parameters, commands, tags, event consumption | locomotion/combat decisions and game rules |
| Cairn | root-motion request and physical-animation target | collision, constraints, ragdoll bodies, final physical transform |
| Penumbra | immutable pose/palette/morph snapshot | GPU buffers, deformation execution, visibility, presentation |
| Audio/Wavefront | sample-accurate cue request and marker | mixing, device time, spatial output |
| Saves/network | stable graph state and clip/event cursor | transaction, replication, rollback transport |
| Marquee | imports approved rendered clips and audio after manual capture | in-engine sequencing, camera/game control, animation state, or source performance authority |

Forbidden edges include renderer handles in animation source documents, gameplay rules hidden inside clip events, animation jobs mutating physics directly, editor widgets as serialization authority, and network packets containing transient pointers or backend-specific pose layouts.

## 3. Planned Public Contracts

Logical contracts, not implemented Rust types:

```text
SkeletonAsset { id, joints, bind_pose, semantic_tags, compatibility_signature }
AnimationClip { id, duration_ticks, tracks, curves, events, root_motion, compression_profile }
AnimationGraph { id, parameters, states, transitions, layers, nodes, output_contract }
AnimationInstanceState { graph_id, state_ids, normalized_times, parameters, event_cursors }
PoseSnapshot { skeleton_id, generation, local_pose, morph_weights, root_motion_delta }
RetargetProfile { source_signature, target_signature, mappings, scale_policy, constraints }
CinematicSequence { id, tracks, shots, bindings, timebase, branch_policy, provenance }
```

Stable IDs identify joints, tracks, states, transitions, events, bindings, and source spans. Runtime instances use generation-checked handles. Source documents remain backend-neutral and migrate one version at a time.

## 4. Ordered Runtime and Authoring Pipelines

Runtime evaluation:

```text
capture immutable gameplay parameters
-> select graph transitions at declared simulation barrier
-> request/decompress required clip pages
-> sample clips and curves on integer animation ticks
-> blend layers and masks
-> solve bounded IK/constraints
-> emit ordered events and root-motion request
-> reconcile physical authority
-> publish immutable pose snapshot
-> Penumbra performs selected deformation path
```

Import and authoring:

```text
ingest source with provenance
-> validate skeleton/topology/units/timebase
-> preserve source IDs where possible
-> create explicit retarget/compression profile
-> preview in Meridian
-> build deterministic derived artifacts
-> compare visual, event, memory, and runtime evidence
-> atomically publish accepted artifacts
```

Cinematic sequences request gameplay, camera, animation, audio, UI, and world actions through typed bindings. They cannot seize ownership of those systems or hide required gameplay state in editor-only timelines.

ANI owns in-engine sequencing and shot execution. Marquee owns only deterministic post-capture promotion timelines over imported media. A Marquee campaign cannot invoke a `CinematicSequence`, move a camera, start gameplay, or automatically capture footage.

## 5. Time, Threads, Memory, and Lifetime

- Simulation-relevant graph transitions and events use integer ticks and recorded ordering.
- Presentation interpolation may run at display cadence and cannot feed nondeterministic values back into authoritative gameplay.
- Clip I/O, decompression, retarget preparation, graph compilation, and compression run on bounded workers.
- Pose evaluation uses preplanned scratch and bounded per-instance work; skinning path selection is capability-driven.
- Clip pages and pose buffers use generation-checked residency. Missing data produces a declared hold, fallback pose, or recoverable failure, never invalid memory.
- Rollback stores compact graph state and authoritative parameters, not opaque worker state or GPU buffers.

## 6. Failure, Diagnostics, Security, and Provenance

Required diagnostics include graph/clip/skeleton IDs and hashes, active states, transition reason, event order, missing pages, retarget error, compression error, pose cost, skinning cost, root-motion reconciliation, and source provenance. Invalid source, cyclic graph dependencies, incompatible skeletons, missing required joints, corrupt pages, or budget excess fail with stable codes and preserve the last accepted artifact.

Untrusted imports are decoded in bounded workers. Animation events invoke only registered typed commands. Captured biometric or performance data is private by default and cannot enter public evidence without explicit sanitization and consent.

## 7. Workflows, Accessibility, and Capability Tiers

Beginner workflow: import or create a skeleton, assign a clip, choose a graph template, preview, fix plain-language diagnostics, and attach typed events. Expert workflow: edit curves, masks, blend spaces, retarget maps, compression, source spans, event traces, pose memory, and per-node timing.

Animation tools require keyboard navigation, semantic labels, scalable timelines, reduced-motion preview, color-independent state distinctions, and textual alternatives for graph operations.

Tiers:

1. baseline clips, skeletons, events, CPU pose evaluation, and simple graphs;
2. layered graphs, root motion, retargeting, IK, streamed clips, and GPU skinning;
3. advanced physical animation, cinematic sequencing, facial rigs, capture, and research paths.

Disabled facial, cinematic, IK, or GPU paths allocate no runtime state and schedule no work.

## 8. Requirements, Evidence, and Delivery

- `REQ-ANI-001`: stable, versioned skeleton, clip, graph, event, and pose contracts with import/migration evidence.
- `REQ-ANI-002`: bounded runtime evaluation, streaming, root-motion, and rollback behavior with timing, memory, and determinism evidence.
- `REQ-ANI-003`: provenance-safe authoring, retargeting, diagnostics, and accessible workflows with representative corpus evidence.
- `WP-ANI-001` delivers skeleton, clip, import, compression, runtime pose, and event foundations.
- `WP-ANI-002` delivers graph authoring, root motion, IK, retargeting, debugging, and baseline sequencing.
- `PRG-ANI-001` covers advanced facial, capture, cinematic-character, and virtual-production work after MS-10.

Tests cover schema migration, stable IDs, malformed import fuzzing, clip-page starvation, graph determinism, event ordering, root-motion/physics reconciliation, retarget fixtures, rollback, memory budgets, CPU/GPU deformation differential output, and accessibility. Structural output cannot satisfy visual-quality requirements.

## 9. Examples

End to end: a modeler-authored rig and imported walk clip compile into a graph; a framework sets movement parameters; ANI emits root motion and footstep markers; Cairn accepts bounded motion; Wavefront schedules sound; Penumbra presents the pose.

Failure: a clip references a removed joint. Publication stops, the prior clip remains active, and Meridian identifies the source track and suggested remap.

Performance debug: a crowd spike groups pose time, decompression, skinning, and residency by graph and skeleton, revealing an oversized blend graph and cold clip pages.
