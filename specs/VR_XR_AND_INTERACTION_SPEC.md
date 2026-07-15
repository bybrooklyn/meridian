# VR, XR, and Interaction Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Rendering](RENDERING_AND_GRAPHICS_SPEC.md) · [Cairn](CAIRN_PHYSICS_SPEC.md)

Version 0.2 · 2026-07-14 · Normative architecture · Deferred implementation

Research anchors: [OpenXR 1.1 specification](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html) and [Khronos OpenXR frame submission guide](https://github.com/KhronosGroup/OpenXR-Guide/blob/main/chapters/frame_submission.md). These sources support lifecycle, spaces, actions, predicted display time, and frame-submission requirements; Meridian owns engine boundaries and capability policy.

## 1. Scope

Meridian XR is OpenXR-first and capability-driven. It reuses renderer, input, Cairn, UI semantics, assets, world streaming, audio, and gameplay commands while respecting predicted display timing and physical interaction.

OpenXR is not on the opening desktop slice critical path. Vendor extensions, passthrough, anchors, eye tracking, hand tracking, body tracking, and platform stores are optional adapters.

## 2. Boundaries

- meridian-xr-core: session/view/action/space contracts and capability model.
- meridian-xr-openxr: loader, instance/system/session/swapchain, events, extensions.
- meridian-xr-interaction: hands/controllers, grabs, sockets, tools, haptics.
- render owns multiview/swapchain rendering and timing inputs.
- input maps OpenXR actions into semantic actions.
- Cairn owns physical queries/constraints; gameplay owns meaning.
- UI owns world/screen panels and accessible alternate actions.

OpenXR handles and extension structs do not enter project source or game APIs.

Invalid dependencies: game crates must not depend on OpenXR loader crates; renderer public APIs must not require XR swapchains; UI documents must not serialize OpenXR paths; Cairn must not import XR runtime handles; MCP tools must not read pose, eye, face, room, camera, or passthrough data without explicit capability.

## 3. Lifecycle

~~~text
Unavailable -> Instance -> System -> SessionReady
SessionReady <-> Running <-> Visible <-> Focused
any -> LossPending -> Recreate
any -> Exiting
~~~

The adapter consumes runtime events and performs only legal transitions. Session loss preserves project/game state and releases XR resources; it never corrupts world or save authority.

## 4. Frame pipeline

1. poll XR lifecycle/events;
2. wait/begin frame and receive predicted display time;
3. locate views and action spaces for predicted time;
4. sample actions/poses and publish XR input snapshot;
5. fixed simulation consumes semantic actions at its clock boundary;
6. presentation predicts late poses without mutating simulation;
7. cull once where valid and build per-view/multiview commands;
8. acquire/wait swapchain images;
9. render, resolve, and release images;
10. end frame with declared composition layers;
11. submit bounded haptics and diagnostics.

Late-latching affects presentation transforms only. Reprojection is runtime-owned.

## 5. Interaction model

~~~rust
pub struct InteractionPose { aim: Transform, grip: Transform, confidence: Confidence }
pub enum InteractionIntent { Point, Select, Grab, Use, Teleport, Manipulate }
pub struct GrabConstraintDesc {
    actor: PersistentEntityId,
    anchor: InteractionAnchor,
    mode: GrabMode,
    limits: ConstraintLimits,
}
~~~

Direct touch, ray, gaze, controller, hand, keyboard, and accessible alternate input map to semantic intents. Gameplay chooses allowed operations. Physics supports kinematic targets, constraints, collision filtering, two-hand manipulation, throw velocity filtering, and deterministic-enough recording modes.

## 6. Comfort and accessibility

Required options: seated/standing origin, height calibration, snap/smooth turn, vignette, teleport/continuous locomotion where game permits, handedness, reach assistance, one-handed modes, haptic intensity, subtitle placement, UI distance/scale, reduced motion, and non-VR fallback when the product supports it.

World-space UI exposes the same semantic tree as desktop UI. Critical actions have non-gesture alternatives.

## 7. Performance tiers

- Baseline stereo with conservative resolution and raster fallbacks.
- Multiview/single-pass where backend supports it.
- Optional foveation/eye-tracked foveation with privacy permission.
- Optional hardware RT effects only after XR frame budget evidence.

Dynamic resolution and quality changes are bounded and reported. No XR resource or polling task exists when the OpenXR pack is disabled.

Algorithm gates:

| Problem | Baseline | Alternative | Gate |
|---|---|---|---|
| Stereo rendering | multiview/single-pass when supported | per-eye fallback | Per-eye remains mandatory until all target runtimes support a better path. |
| Grabbing | compliant Cairn constraints | kinematic attach | Kinematic attach may be a low tier, but benchmarked physical scenes decide default. |
| Throwing | timestamped pose history | proxy velocity only | Compare reproducible throw scenes before committing constants. |
| UI | Meridian world panels plus compositor layers where useful | head-locked raster overlay only | Use compositor layers only when runtime support and latency/clarity evidence justify it. |

## 8. Persistence and data

Project data stores semantic actions, interaction profiles, comfort defaults, and authored anchors, never runtime device paths as sole identity. Runtime bindings are user/platform overlays. Spatial anchors require explicit provider, lifetime, privacy, export, and failure policy.

## 9. Diagnostics and recovery

Report runtime/system, selected extensions, session state, predicted time, view configuration, swapchain formats, acquire/wait time, CPU/GPU frame timing, missed frames, pose age/confidence, action profile, haptic queue, and recreation count.

On headset removal or session loss, pause/persist according to game policy, keep the desktop recovery UI usable, release swapchains, and recreate only after the runtime reports readiness.

Planned editor, CLI, and MCP workflows:

~~~text
meridian xr doctor
meridian xr list-runtimes
meridian xr check-project
meridian xr benchmark --scene many-grabbables
mcp.xr.capabilities
mcp.xr.validate_project
~~~

These operations are planned contracts. Live pose, room, camera, microphone, eye, face, body, and passthrough queries are separate sensitive capabilities.

## 10. Security and privacy

Pose, room, hand, eye, camera/passthrough, microphone, and spatial-anchor data are sensitive. Capabilities declare collection, retention, network transmission, recording, and agent access. Raw biometric/room data never enters default traces or VCS.

## 11. Tests and benchmarks

- OpenXR conformance-informed lifecycle fixtures and mocked event sequences;
- device/session loss and swapchain recreation;
- action binding/profile and handedness tests;
- pose age/late-latch invariants;
- stereo/multiview image tests;
- interaction constraint/throw/filter tests;
- comfort/accessibility journeys;
- frame timing under resolution/quality transitions;
- disabled-pack no-loader/no-task/no-resource proof.

## 12. Phases and research

- Phases 1–8 preserve timing, input, render, physics, UI, and save seams.
- Phase 19 implements OpenXR lifecycle, stereo rendering, actions, basic interactions, and one reference application.
- Later gates evaluate hand/eye tracking, passthrough, anchors, foveation, and store SDKs.

The current official OpenXR registry is tracked in [research decisions](RESEARCH_AND_ALGORITHM_DECISIONS.md); extensions are selected only against tested runtimes.

## 13. Examples

End-to-end: pressing controller grab maps to Grab intent, gameplay validates object permissions, Cairn creates a bounded constraint, presentation late-latches the hand pose, audio receives contact events, and save data records only authored/game state.

Failure/recovery: the runtime enters LOSS_PENDING during play. Meridian suspends XR submission, checkpoints game state, shows desktop recovery, destroys session resources, then recreates and restores interaction bindings without stale handles.

Performance debug: missed frames correlate view acquisition, visibility, shadow, UI, and submit spans against predicted display deadline; the trace identifies whether dynamic resolution or a feature pack caused the miss.
