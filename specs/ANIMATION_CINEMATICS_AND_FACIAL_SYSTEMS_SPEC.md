# Artus — Meridian Procedural Body-Motion Subsystem

[Master index](MERIDIAN_MASTER_SPEC.md) · [Assets and worlds](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Cairn](CAIRN_PHYSICS_SPEC.md) · [Gameplay](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) · [Navigation](NAVIGATION_AND_AI_INFRASTRUCTURE_SPEC.md) · [Multiplayer](MULTIPLAYER_AND_SERVER_SPEC.md) · [Agents](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md) · [Delivery](DELIVERY_ROADMAP.md)

Status: version 0.5 normative architecture, 2026-07-18.

Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-ANI-001` through `REQ-ANI-003`; `WP-ANI-001`;
`WP-ANI-002`; `RG-ANI-001`; `RG-ANI-002`; `PRG-ANI-001`.

Current implementation status: Meridian has no Artus runtime, animation graph,
retargeter, motion-matching database, cinematic sequencer, facial solver, or
performance-capture pipeline. This document defines intended contracts and
evidence gates only; illustrative names below do not declare implemented Rust
types, crate boundaries, asset extensions, algorithms, or quality.

## 1. Authority, Goals, and Non-Goals

Artus is Meridian's unified character-performance subsystem. It accepts
semantic intent, authored motion, environmental context, body proportions, and
physical constraints, then produces an adaptive body and later facial
performance. Its initial public scope is humanoids; the architecture must not
permanently exclude other articulated bodies.

Artus owns animation-source documents, skeleton and rig semantics, pose
evaluation, graph and future pose-search motion sources, retargeting,
root-motion proposals, contact and IK requests, performance sequencing,
motion diagnostics, and motion-intent execution. It is a named `ANI`
subsystem, not a new governance domain or user-facing application.

Goals:

- make a beginner humanoid workflow possible through a profile-first path;
- preserve authored timing and artistic control while adapting to contacts,
  terrain, proportions, and physical outcomes;
- give gameplay, future Bearings decision layers, player controllers, and
  sequences one semantic intent boundary rather than direct bone mutation;
- keep advanced graph, contact, curve, timeline, diagnostic, and scripting
  workflows available inside the single Meridian application;
- support bounded, explainable, provenance-safe, accessible, and testable
  performance across hero characters and reduced-cost crowds.

Pre-1.0 non-goals include self-balancing physical locomotion, general creature
locomotion, unrestricted agent mutation, a proprietary motion-capture service,
neural facial synthesis, a film-compositing suite, mandatory cloud processing,
or claims to match dedicated commercial animation products. Advanced facial
performance, capture, cinematic characters, and virtual production remain
`PRG-ANI-001` after MS-10. MS-09 may establish only a versioned facial
profile/import and pose-layer foundation.

## 2. Ownership and Forbidden Edges

| Boundary | Artus owns | Neighbor owns |
| --- | --- | --- |
| Modeler/DCC | rig semantics, imported clip interpretation, retarget profiles | editable mesh, topology lineage, rig-authoring surfaces, source interchange |
| Gameplay/frameworks | semantic intent execution and completion reports | decisions, abilities, permissions, damage, inventory, and game-state consequences |
| Bearings | intent production as a future gameplay/AI decision-layer consumer | goals, tactics, awareness, behavior selection, and planning |
| NAV | locomotion targets and transition metadata consumed by Artus | traversability artifacts, path/flow queries, dynamic-obstacle results |
| Cairn | proposed root displacement, physical-animation targets, and pose adaptation | collision, rigid-body dynamics, controllers, joints, contacts, impulses, and final physical transform |
| Penumbra | immutable pose, palette, morph, and LOD snapshots | deformation execution, GPU resources, motion vectors, visibility, and presentation |
| Wavefront | semantic contact/event timing | surface choice, cue selection, mixing, device time, and output |
| NET/saves | stable intent/graph/profile state and replay inputs | transport authority, replication policy, persistence transactions, and rollback transport |
| Sequencing | body-performance tracks and execution | timeline structure, camera timing, scene timing, and event scheduling |

Forbidden edges include renderer handles in source documents; direct mutation of
Cairn state by animation jobs; gameplay rules hidden in events; transient
handles in saves or network packets; editor widgets as serialization authority;
and raw file mutation or privileged agent paths. Artus does not make Bearings
an implemented subsystem or move gameplay decisions into NAV.

## 3. Public Contract Direction and Data Authority

Source documents are authoritative, schema-defined, versioned, inspectable,
recoverable, and provenance-bearing. Compiled clips, motion-search indexes,
pose caches, retarget tables, and baked results are derived artifacts. Stable
source IDs cross import, save, package, and network boundaries; generation-
checked handles remain process-local.

The following are illustrative Meridian-owned contract shapes:

```text
ArtusRigProfile {
  skeleton_id: u128,
  semantic_bones: BoundedMap<SemanticBoneId, BoneRef>, // default max 256 semantic bones
  canonical_rest_pose: Pose,
  retarget_chains: Vec<RetargetChain>,     // default max 32 chains
  joint_limits: Vec<JointLimit>,
  body_measurements: BodyMeasurements,
  contact_sites: Vec<ContactSite>,         // default max 16 (feet, hands, hips, head, ...)
  facial_profile: Option<FacialProfile>,
}
ArtusMotionProfile {
  rig_profile: u128,
  motion_sources: Vec<MotionSourceRef>,    // default max 128 clips/sources per profile
  graph: Option<GraphRef>,
  motion_database: Option<MotionDatabaseRef>,
  procedural_layers: Vec<ProceduralLayerRef>, // default max 16 concurrent layers
  interaction_profile: InteractionProfileRef,
  physics_profile: PhysicsProfileRef,
  facial_profile: Option<FacialProfileRef>,
  lod_profile: LodProfile,                 // tiered by distance/screen coverage
  fallback_policy: FallbackPolicy,         // Hold | GraphFallback | RelaxLowPriority
}
ArtusMotionIntent {
  id: u64,
  source: IntentSource,                    // Gameplay | Bearings | Cutscene | Debug
  kind: IntentKind,                        // Locomotion | Facing | Gaze | Reach | Grasp |
                                            // Carry | Interact | Sit | Climb | Brace |
                                            // Avoid | Reaction | Gesture | Performance
  priority: u8,                            // 0 (lowest) - 255 (safety-critical), higher wins
  timing: IntentTiming,
  body_mask: u32,                          // bitflags over body regions
  spatial_target: Option<Vec3>,
  constraints: Vec<IntentConstraint>,      // default max 8 constraints per intent
  interrupt_policy: InterruptPolicy,       // CanInterrupt | HardLock { max_duration_ms: u32 }
}
ArtusContact {
  effector: EffectorId,
  target: Vec3,
  desired_transform: Transform,
  priority: u8,
  timing: ContactTiming,
  ownership: ContactOwnership,             // Local | ServerAuthoritative
  tolerances: ContactTolerances,           // position/rotation epsilon before relaxation
  failure_policy: ContactFailurePolicy,    // Relax | Hold | ReportOnly
}
ArtusMotionResolution {
  intent_id: u64,
  proposed_root_delta: Transform,
  contacts: Vec<ArtusContact>,             // default max 8 concurrent contacts resolved per tick
  status: ResolutionStatus,                // Active | Completed | Interrupted | Failed
  completion_reason: Option<CompletionReason>,
  diagnostics: ResolutionDiagnostics,
}
ArtusPoseSnapshot {
  skeleton_id: u128,
  generation: u32,                          // generation-checked handle epoch
  local_pose: Pose,
  morph_weights: Vec<f32>,                  // default max 256 morph targets
  achieved_root_delta: Transform,
  events: Vec<MotionEvent>,                 // default max 16 events per published snapshot
  lod_state: LodState,
}
```

`ArtusMotionIntent` is the shared semantic boundary. Candidate families include
locomotion, facing, gaze, reach, grasp, carry, interact, sit, climb, brace,
avoid, reaction, gesture, and performance. Its precise schema, extension
policy, arbitration, hard-lock permissions, cancellation, and authority rules
remain `RG-ANI-001`; consumers never manipulate bones as their normal API.

## 4. Runtime and Authoring Pipelines

Artus uses negotiated movement. A controller or higher-level system supplies
desired intent and trajectory; Artus selects or synthesizes a pose and
proposes root displacement; Cairn resolves physical movement and contacts;
Artus adapts the pose to the achieved result. Safety and collision validity,
then high-priority contacts, body-region priorities, interaction requirements,
and source-pose fidelity resolve conflict in that order unless a later
profiled policy explicitly changes it.

```text
capture immutable intents, navigation results, and Cairn snapshot
-> arbitrate intents and build desired trajectory
-> select graph, authored performance, recorded performance, or future pose search
-> sample/decompress motion and retarget canonical motion to the target rig
-> apply bounded procedural layers and predict contacts
-> propose root displacement to Cairn
-> consume achieved root/contact result
-> solve prioritized contacts and IK, relaxing lower-priority constraints first
-> apply physical reaction and optional facial layers
-> emit ordered semantic events and publish immutable pose snapshot
-> Penumbra selects deformation execution
```

Simulation-relevant intent arbitration, root negotiation, contact ownership,
and completion reporting use declared simulation ordering. Pose sampling and
render interpolation may run at display cadence but cannot feed
nondeterministic state back into gameplay. Clip I/O, decompression, retarget
preparation, motion-search preparation, validation, and cache building use
bounded cancellable workers. Missing data, budget saturation, or a failed
solve produces a declared hold, graph fallback, relaxed low-priority contact,
or recoverable failure; it never creates unbounded work, invalid memory, or
explosive joint motion.

### 4.1 Rig, graph, and procedural composition

Humanoid import detects candidate topology, proposes semantic assignments and
retarget chains, estimates body measurements/contact sites, identifies rest
pose, builds canonical representation, runs validation motions, and requires
visual confirmation before saving an `ArtusRigProfile`. The canonical form
normalizes axes, body-space orientation, rest reference, chain measures, joint
limits, and contact sites while preserving the source skeleton for rendering,
export, and recovery. Required validation motions include stance, crouch,
reach, walk/run, turn, stairs, foot plant, sit alignment, ragdoll transition,
and head look; unsupported or incomplete semantics are explicit capability
limits, not silently repaired guesses.

The first graph surface supports clip players, one- and two-dimensional blend
spaces, state machines, transitions, layered/additive blends, masks, sync
groups, events, root-motion extraction, parameter inputs, pose caches, and
procedural insertion points. Profile-first setup may create or configure a
graph, but advanced graph edits and profile synchronization remain an open
contract decision rather than an implied hidden graph format.

Initial procedural layers are foot placement, pelvis adjustment, stride/turn
warping, terrain lean, head/gaze, basic hand placement, ragdoll blending, and
basic impact reactions. Layers declare their inputs, body mask, phase,
priority, canonical/target space, root-motion permission, contact authority,
determinism, and LOD behavior. Later layers may cover breathing, balance,
stairs, carry, injury, fatigue, water, ladders, and crowd posture; self-
balancing locomotion and torque-level control remain research-only.

### 4.2 Interactions, face, baking, and networking

Artus interaction definitions combine authored smart-object metadata, contact
events, runtime inference for simple cases, full-body alignment, and typed
failure/exit paths. Reusable definitions can describe approach regions, facing
tolerance, limb availability, contact transforms/timing, root expectations,
object bindings, variants, and recovery. Initial hands use hand IK, authored
grip profiles, lightweight finger correction, and contact events; collision-
aware finger synthesis is deferred.

Face and body are one long-term performance system. The MS-09 foundation may
carry a versioned facial profile and import/pose layer for blendshape,
bone-driven, hybrid, stylized, or eye-only rigs. It does not promise speech,
emotion, production gaze, capture, or full facial visual quality; those remain
post-1.0 work with privacy and consent evidence.

Artus may later bake full or selected poses, root motion, canonical or target
poses, procedural layers, contacts, physics results, and facial results. Every
bake retains source profile/clip/intent/procedural settings, rig version,
generator version, and dependency hashes so it can be inspected and rebuilt.
For multiplayer profiles, the default direction is authoritative high-level
intent, root state, and important contact/interaction state from the server
while clients synthesize noncritical body detail locally. Gameplay-critical
contacts and outcomes retain their declared server authority.

## 5. Capability Tiers and Delivery

1. `WP-ANI-001` establishes pose/skeleton time and identity, clip import and
   sampling, simple blending, root-motion extraction, semantic rig/profile
   source, validation, and bounded event foundations for MS-08.
2. `WP-ANI-002` establishes graph and profile authoring, retargeting, IK,
   contacts, negotiated Cairn coupling, terrain/stride/turn adaptation,
   interactions, diagnostics, and the beginner usable-humanoid workflow.
   Its MS-09 scope adds evidence-gated pose search, motion LOD, replicated
   high-level intent, and a narrow facial profile/import/pose-layer foundation.
3. `PRG-ANI-001` covers advanced facial performance, capture, cinematic
   characters, virtual production, active physical control, and other
   research-only performance capabilities after MS-10.

Motion sources remain interchangeable: graphs, blend spaces/state machines,
authored or recorded performances, future motion matching, and future physical
controllers all feed the same composition, contact, IK, and Cairn-coupling
pipeline. Motion matching must retain a declared graph or current-motion
fallback. Motion LOD can reduce update rate, secondary motion, fingers,
facial detail, contact solve, IK iterations, search cost, or skeleton detail;
it must not silently weaken authoritative physical or gameplay contacts.
Disabled Artus, facial, pose-search, or GPU paths allocate no components,
workers, listeners, GPU resources, editor panels, dependencies, or package
chunks.

## 5.1 Work package briefs

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status changes.

**`WP-ANI-001` — Artus semantic rig, clip, pose, root-motion, and event foundation**
Result: an imported humanoid gets pose/skeleton time and identity, clip
import/sampling, simple blending, root-motion extraction, and a semantic
rig/profile source a gameplay consumer can request a pose from (§5.1 of the
delivery list, MS-08 scope). Owning contracts: `ArtusRigProfile`,
`ArtusPoseSnapshot` (§3). Entry conditions: none of Artus exists yet (current-
status line); depends on Modeler/DCC for imported clip interpretation (§2),
not on the modeler's full maturity. Deliverables: the humanoid-import
pipeline in §4.1 (detect topology → propose semantic assignments/retarget
chains → estimate body measurements/contact sites → canonical representation
→ required validation motions: stance, crouch, reach, walk/run, turn, stairs,
foot plant, sit alignment, ragdoll transition, head look → visual
confirmation before saving), bounded event foundations, and validation that
unsupported semantics are explicit capability limits, not silently repaired
guesses (§4.1). Non-goals: no graph/state-machine authoring, no retargeting
beyond import, no IK/contacts, no Cairn coupling — all `WP-ANI-002` (§5's
ordering). Security: captured biometric data private by default, no public
evidence without consent (§7). Tests: the §8 static-validation list scoped to
import (semantic mapping, hierarchy, rest pose, joint limits, root motion,
source versions) against the representative humanoid corpus. Stop condition:
a rig that fails required validation motions blocks save with stable
diagnostics rather than saving a guessed mapping (§4.1). Next unblocked:
`WP-ANI-002`.

**`WP-ANI-002` — Artus profiles, graphs, retargeting, contacts, and negotiated movement**
Result: the beginner usable-humanoid workflow (§7: import, create profile,
confirm mapping, select starter motion, enable safe procedural defaults,
preview, save) works end to end with negotiated Cairn-coupled movement.
Entry conditions: `WP-ANI-001` closed (rig/pose foundation to build the graph
and retargeting on); `RG-ANI-001` decided (the motion-intent/arbitration/
negotiated-control contract, §6) before packages depend on it — this package
cannot finalize its intent boundary ahead of that gate. Deliverables: the
first graph surface (§4.1: clip players, 1D/2D blend spaces, state machines,
transitions, layered/additive blends, masks, sync groups, root-motion
extraction, procedural insertion points), initial procedural layers (foot
placement, pelvis adjustment, stride/turn warping, terrain lean, head/gaze,
basic hand placement, ragdoll blending, basic impact reactions), the full
negotiated-movement pipeline (§4: arbitrate intents → select motion source →
sample/retarget → propose root displacement to Cairn → consume achieved
result → solve prioritized contacts/IK → publish immutable pose), and
interaction definitions (§4.2: approach regions, facing tolerance, contact
transforms, recovery). MS-09 scope within this package (per §5) adds
evidence-gated pose search, motion LOD, replicated high-level intent, and a
narrow versioned facial profile/import/pose-layer foundation — explicitly
not speech, emotion, production gaze, or capture (§4.2), which stay
`PRG-ANI-001`. Non-goals: self-balancing locomotion, torque-level control
remain research-only (§4.1); this package does not become a separate
application — one Artus workspace inside Meridian (§7). Failure/recovery:
missing data, budget saturation, or a failed solve produces a declared hold,
graph fallback, or relaxed low-priority contact, never unbounded work or
explosive joint motion (§4). Tests: the §8 dynamic-fixture list (foot
sliding/penetration, joint inversion, hand drift, root teleportation, solver
instability, ragdoll continuity, motion-search thrashing, LOD transitions,
correction spikes) against the representative corpus; `RG-ANI-002`-gated
retargeting/IK/pose-search/deformation evidence before MS-09's advanced
subset ships. Stop condition: an incompatible rig or unreachable interaction
blocks publication with stable diagnostics and keeps the last accepted
artifact active where safe (§8) — it does not ship a believable-looking but
unvalidated result; structural graph construction alone is never proof of
believable motion (§8). Next unblocked: `PRG-ANI-001` (post-MS-10, its own
entry gate, cannot be started by this package).

## 6. Research Gates and Open Decisions

`RG-ANI-001` selects the motion-intent, arbitration, negotiated-control, and
completion-report contract before Artus packages depend on it. `RG-ANI-002`
selects bounded retargeting, IK/constraint, pose-search, compression, and
deformation portfolios before the MS-09 advanced scope. Each must preregister
candidate revisions, representative humanoid corpus, determinism and recovery
expectations, accessibility/security/licensing review, metrics, stop rule, and
losing-prototype archive; a decision requires an ADR.

Intentionally deferred choices are the exact intent schema and custom-extension
policy, canonical humanoid coordinate convention, solver algorithms,
motion-feature schema, compression, facial standard, morph limits, network
correction protocol, asset extensions, crate layout, and GPU pose evaluation.
No illustrative example resolves them.

## 7. Workflows, Accessibility, Security, and Agents

Beginner workflow: import a character, create an Artus profile, confirm the
proposed semantic mapping, select starter motion, enable safe procedural
defaults, preview generated test environments, resolve plain-language errors,
and save. Expert workflow: inspect and edit graphs, masks, curves, contacts,
retarget chains, joint limits, solver passes, motion databases, LOD policy,
and timelines. The Artus workspace provides a viewport, profile/library tree,
context inspector, timeline/contact history, and optional graph/search/Cairn
diagnostics without becoming a separate application.

All Artus workflows require keyboard operation, visible focus, semantic labels,
scalable timelines, color-independent diagnostics, textual graph/overlay
alternatives, reduced-motion preview, and accessible recovery from invalid
assets or failed solves. Visual overlays include semantic bones, trajectories,
contacts, priorities, joint limits, root disagreement, solver error, active
motion source, LOD state, and network corrections.

Imports, graph nodes, procedural code, agent input, and scripts are untrusted.
Validate bounds before allocation and run decoding or expensive validation in
bounded workers. Agents use the same typed command/query, permission, preview,
transaction, audit, undo, and rollback paths as people. They may propose rig
mappings, profiles, graphs, databases, contacts, test scenes, diagnostics, and
bakes, but never commit a mutation without the configured reviewable
transaction. Captured biometric data is private by default and cannot enter
public evidence without explicit consent and sanitization.

## 8. Validation, Evidence, and Failure Policy

Static validation covers semantic mapping, hierarchy, rest pose, joint limits,
root motion, contact events, interaction reachability, masks, and source
versions. Dynamic fixtures cover foot sliding/penetration, joint inversion,
hand drift, root teleportation, solver instability, ragdoll continuity,
motion-search thrashing, LOD transitions, and correction spikes. The
representative humanoid corpus varies topology, naming, rest poses,
proportions, optional bones, terrain, frame/simulation rates, latency, forces,
and malformed input. Golden references compare declared root trajectories,
contact timing, bounded error, selection identity when reproducible, and
measured cost; they do not by themselves prove visual quality.

Required evidence includes schema migration, stable identity, malformed-import
fuzzing, graph/event determinism, clip-page starvation, root-motion/Cairn
reconciliation, retarget corpus, contact/interaction recovery, rollback,
thread/memory bounds, CPU/GPU deformation differential where a GPU path exists,
accessibility, provenance, and stripped disabled-pack builds. Structural output
or a successfully constructed graph is never proof of believable motion.

End to end: an imported humanoid receives a validated rig profile and motion
profile; a gameplay or Bearings consumer submits a locomotion intent; Artus
proposes movement, Cairn resolves it, Artus keeps the foot contact or reports a
bounded fallback, Wavefront receives a semantic event, and Penumbra consumes
the final immutable pose.

Failure: an incompatible rig or unreachable interaction blocks publication or
execution with stable diagnostics, leaves the last accepted artifact active
where safe, and offers a remap, graph fallback, contact relaxation, or explicit
cancel path. Performance debugging attributes pose, decompression, retarget,
search, IK, skinning, and residency costs to the profile and motion source.
