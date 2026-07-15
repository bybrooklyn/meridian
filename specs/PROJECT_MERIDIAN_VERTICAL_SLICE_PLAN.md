# Project Meridian Opening-Forest Vertical Slice Plan

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Delivery](DELIVERY_ROADMAP.md) · [Implementation planning](IMPLEMENTATION_PLANNING_SPEC.md) · [Private creative slice](https://github.com/bybrooklyn/project-meridian/blob/main/docs/OPENING_FOREST_VERTICAL_SLICE.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Normative integration plan

Documentation maturity: `ImplementationReady`. Implementation maturity:
`Planned`. Delivery: `MS-07`. Governing IDs: `REQ-PRJ-002`, `WP-PRJ-002`.

## 1. Result

MS-07 delivers roughly five minutes of production-quality final-game traversal
beginning in the midnight forest and reaching the intended title/return
transition. It is extremely dark but deliberately readable, ambient and
non-combat, with dense trees/grass, flashlight navigation, restrained
fog/analog treatment, environmental sound, subtle interaction, reliable
save/recovery, and a one-click export. The bounded MS-06 prototype is governed
separately by [PROJECT_MERIDIAN_PROTOTYPE_PLAN.md](PROJECT_MERIDIAN_PROTOTYPE_PLAN.md)
and cannot satisfy this slice.

This engine-facing plan does not replace the private creative suite. The separate Project Meridian repository remains authoritative for route, pacing, art, audio intent, narrative, asset list, and no-enemy/no-death/no-sprint/no-jump requirements. This plan turns those constraints into engine integration gates without importing the full closed-source game documentation.

## 2. Explicit non-goals

- enemies, combat, death, conventional jumpscare, forced dialogue, sprint, or jump;
- completing the field, compound, settlement, roads, ending, or whole game;
- multiplayer, mods, marketplace/community library, XR, additional languages, generated cities, planetary weather, full fluids/fire/snow/erosion;
- advanced GI/virtual geometry/visibility buffer as mandatory;
- full Cairn destruction/deformables;
- complete permanent editor migration;
- claiming visual quality from an occluded GPU smoke.

## 3. Capability dependency map

~~~text
Runtime/platform ─┬─ Input/movement ─┐
Penumbra/assets ──┼─ Forest/fog/light├─ Playable route
Basalt/streaming ─┤                  │
Cairn collision ──┤                  │
Wavefront/Isobar ─┤                  │
Rust/logic ───────┤                  │
UI/accessibility ─┤                  │
Save/package ─────┴─ Export/recovery ┘
~~~

Each incoming capability is a narrow gate. MS-07 may use the simplest tested
implementation that preserves the long-lived seam and creative result, but it
cannot inherit prototype-only acceptance.

## 4. Slice documents and data

Authoritative source uses:

- project settings and capability manifest;
- world directory with stable route/zone/entity IDs;
- imported assets with provenance and visual/physical/acoustic facets as needed;
- material documents;
- Isobar weather/wind/fog state document;
- Basalt terrain/cell documents and vegetation placement sources;
- private Alluvium recipes, seeds, generated identities, hero locks/overrides,
  accepted artifact/provenance hashes, and cook/license decisions;
- Rust gameplay modules plus typed interaction/state-flow documents; optional Luau only if independently ready and useful;
- Meridian UI settings/pause/accessibility documents;
- audio graph/events/regions;
- save schema and MS-07 migration fixtures;
- build/export profile.

Compiled cells, shaders, pipelines, textures, meshes, audio streams, logic bytecode, and package chunks are derived artifacts.

## 5. Work packages

The `VS-*` labels below are local implementation slices within `WP-PRJ-002`,
not global registry identifiers. Before MS-07 activation, the owning package
must turn each ready slice into bounded tasks with exact dependencies, changed
authority, tests, evidence, and stop conditions. Their order may overlap where
write ownership is disjoint, but all converge at the `WP-PRJ-002` production
review.

### VS-01 Route and creative lock

Purpose: convert the authoritative private route specification into stable zones, traversal beats, camera/readability shots, audio regions, interactions, and evidence markers.

Steps: inventory route; assign stable IDs; define start/end and checkpoint; mark darkness/readability targets; list optional documents; mark no-event quiet beats; create reference captures; conduct narrative/art review.

Gate: route is walkable with proxy geometry and no prohibited mechanic. Changing a locked beat requires a creative amendment.

### VS-02 Movement, camera, and interaction

Use fixed-step semantic input, grounded Cairn-owned movement seam, walk/crouch as specified, camera comfort settings, flashlight, focus/use interactions, and controller/keyboard remapping.

Failure behavior: lost focus releases movement; invalid ground/contact falls back safely; stuck detection offers checkpoint recovery. Gate: recorded traversal replays without divergence beyond declared mode and no sprint/jump action exists.

### VS-03 Meridian-modeled and Alluvium-authored Basalt forest world, assets, and streaming

Build and curate final-source Alluvium recipes and Basalt world cells, terrain,
hero trees/undergrowth/grass,
collision proxies, route blockers, visibility/streaming hints, variants,
provenance, and lower-cost tiers. Vegetation remains its own subsystem and
consumes Basalt geometry. Streaming requests include visibility, player path,
audio, gameplay, and preload reasons.

Alluvium remains authoring/cooking authority only. Accepted shipping content is
fixed and versioned; Basalt, vegetation, Isobar, Cairn, Penumbra, audio,
navigation, streaming, and saves retain live runtime authority. Regeneration
must preserve hero locks and manual overrides or produce an explicit conflict/
orphan review.

Hero and route-critical model sources may be created or repaired in Meridian's native modeler. Editable mesh documents, stable element lineage, materials/semantic regions, collision/LOD facets, and explicit interchange reports remain source authority. Blender is optional and cannot be required to reproduce the accepted slice.

Gate: cold/warm traversal runs without missing required assets, cell activation hitch beyond calibrated threshold, route holes, or provenance gaps.

### VS-04 Rendering, darkness, fog, and atmosphere

Use Penumbra's adopted Forward+ production path and shared PBR, shadow,
environment-light, flashlight, Isobar atmosphere, vegetation, temporal,
tonemapping, and restrained optional analog-effect systems. Current direct-PBR
foundations remain useful but do not satisfy this gate by themselves.

Immediate prerequisite: pass-level CPU/GPU timing and visible capture. Specular IBL/BRDF LUT may be added as bounded quality work after instrumentation; it does not redefine diffuse IBL completion.

Gate: named hero/traversal captures are readable on calibrated displays, reduced-effects mode works, pipeline warmup is clean, and unsupported/occluded outcomes are reported honestly.

### VS-05 Basic Isobar wind, weather, and vegetation response

Author deterministic Isobar weather transition/state and a simple shared wind
field consumed by vegetation, audio, and Penumbra. No advanced planetary solver
or Torsant fire/fluid/thermal package is required.

Gate: wind is coherent across consumers, deterministic from source/seed where promised, scalable by tier, and absent work is proven when weather pack portions are disabled.

### VS-06 Basic Wavefront audio and acoustics

Deliver device output, streamed ambience, footsteps/material events, authored one-shots, simple spatial attenuation/occlusion/reverb regions, title/static transition, captions/non-speech cue policy, and diagnostics.

Gate: no callback allocation/blocking, no underruns in traversal corpus, device loss recovers, forest/field contrast supports creative intent, and no score appears where the creative spec forbids it.

### VS-07 Logic and narrative

Represent start, route triggers, optional discoveries, title transition, and recovery state with Rust gameplay modules plus typed State Flow/Interaction/Action documents. Optional Luau may be used only after `WP-GAM-002` and is never required for this slice. There is no objective checklist and optional documents never gate completion.

Gate: reachability tests cover completion with zero optional documents, repeat/load behavior, invalid module/build rollback, and isolated Play rebuild/restart boundaries.

### VS-08 UI and accessibility

Provide first-launch flow, pause/settings, graphics/audio/input/accessibility, save status/error, captions/cues, remapping, text scaling, contrast/readability, reduced motion/camera shake/analog effects, and controller/keyboard operation.

The shell may be transitional, but source semantics and commands follow Meridian UI contracts. Gate: keyboard/controller/screen-reader applicable journeys and no inaccessible recovery dialog.

### VS-09 Save, checkpoint, and recovery

Save authoritative gameplay/world deltas at declared checkpoints using journal transactions and compact snapshot. Do not save runtime handles, render state, or derived caches.

Gate: normal load, crash during transaction, truncated tail, prior-version fixture, missing optional content, corrupt primary/recovery head, and reset-to-checkpoint journeys all produce safe outcomes.

### VS-10 Build, package, install, and launch

One editor action invokes the build DAG for Rust, logic, shaders, assets, package, and signing profile; emits immutable BuildId and .meridian package/application; installs/launches in a clean environment.

Gate: no Cargo knowledge required; expert build graph remains inspectable; cancellation/retry works; source-only dependencies are absent; licenses/provenance are included; unsigned local build is labeled.

### VS-11 Performance, diagnostics, and quality scaling

Create executable PEN-B01 opening traversal and PEN-B02 forest stress workloads.
Capture frame/pass CPU/GPU, memory, IO/streaming, task queues, audio
callback/streaming, pipeline creation, world activation, and input latency.

Quality profiles change meaningful algorithms/content tiers and preserve route/readability. Thresholds remain provisional until corpus and named hardware calibration.

### VS-12 Platform and final review

Primary profiling: M4 MacBook model/config recorded. Secondary: user’s named main PC hardware/OS recorded. Linux and Windows claims match actual available evidence; unsupported rows remain open.

Final reviews: creative/narrative, art/readability, audio, accessibility, performance, recovery, security, package/provenance, and documentation.

## 6. Runtime pipeline

1. launch verifies project/package and selects safe capability profile;
2. source/build artifacts produce a mounted read-only runtime view;
3. start cell and required assets preload under budgets;
4. fixed simulation consumes semantic input and commits gameplay/physics commands;
5. world scheduler updates relevance and staged cell activation;
6. Isobar publishes an immutable environment snapshot;
7. renderer/audio extract immutable domain snapshots;
8. UI presents settings/interactions/save state through commands;
9. checkpoint writes journal transaction and rotating recovery head;
10. title/return transition commits final opening state and releases route resources.

## 7. Memory and threading

The slice cannot add private unmanaged threads. General tasks, render owner, audio callback, and worker processes follow runtime contracts. Budgets attribute memory to world cells/assets, renderer resources, audio streams, UI, scripts, physics, and save buffers.

Activation/upload/decode are staged and bounded. Under pressure, distant variants downgrade before required route assets; required gameplay/audio/save data is never evicted without a safe pause/error.

## 8. Editor workflow

Beginner path:

1. Open Project Meridian.
2. Select Opening Forest workspace.
3. edit route/world properties through inspector;
4. press Play From Start or Play From Here;
5. see plain diagnostics with Fix/Learn;
6. press Export Opening Slice.

Expert path adds streaming cells/reasons, material facets, wind fields, audio regions, logic graph, render graph/pass timings, artifact provenance, save journal, capability/quality profiles, and BuildId.

Every change is command-based and undoable or checkpointed. Play-mode changes do not silently overwrite source.

## 9. CLI and automation workflow

Planned semantic commands:

~~~text
meridian project validate --profile opening-forest
meridian build --profile opening-forest --events json
meridian run --workload PEN-B01 --capture all
meridian save verify <fixture>
meridian package inspect <build.meridian>
meridian evidence verify MS-07
~~~

CLI and MCP use the same registry as the editor. Agents may run validation and propose edits but cannot waive creative, security, or evidence gates.

## 10. Diagnostics

The integrated HUD/trace must correlate:

- tick/frame/input IDs;
- render pass CPU/GPU and surface outcome;
- world cell request reason/stage;
- asset/artifact/variant and upload;
- Cairn query/controller/contact;
- weather/wind snapshot;
- audio voice/stream/callback;
- UI/semantic events;
- script/state-flow event;
- save transaction/build/package operation.

A capture is useful only if a reviewer can move from symptom to owning source object and recovery/fix.

## 11. Failure matrix

| Failure | Required behavior |
|---|---|
| GPU/device/surface unavailable | typed fallback or actionable stop; no false visual pass |
| missing/corrupt asset | preserve project, identify dependency/provenance, use declared optional fallback only |
| cell load misses deadline | preserve route collision/game state, degrade visuals, trace reason |
| audio device loss | continue muted with notice, retry/reselect safely |
| Rust module rebuild/restart or optional script reload fails | keep last valid artifact and declared checkpoint/state policy |
| save write interrupted | load last committed transaction/recovery head |
| build worker crashes | keep source and prior artifacts, restart/retry by BuildId |
| optional SDK/cloud absent | opening remains buildable/playable |

## 12. Security and provenance

All imported assets, scripts, shaders, packages, and build outputs are untrusted at boundaries. The slice uses no required network service. Asset rights and provenance are complete for every shipping file. Local unsigned builds are visibly distinct from release-signed artifacts. Traces and crash reports are local/redacted by default.

## 13. Evidence bundle

MS-07 evidence contains:

- exact source-control checkpoint and dependency/toolchain lock;
- clean demo builds and package manifests;
- PEN-B01/PEN-B02 reports for named hardware and quality profiles;
- CPU/GPU/memory/IO/audio captures;
- hero and traversal image/video captures;
- save/crash/device/worker recovery demonstrations;
- accessibility report;
- creative/art/audio sign-offs;
- asset rights/provenance and license report;
- known limitations and deferred work;
- updated specs, PLANNING, Ponder articles, and player-facing settings docs.

## 14. Completion rule

The slice completes only when an uninvolved reviewer can install/launch, traverse from start to transition, use accessibility/settings, recover from a tested interrupted save, inspect evidence, and confirm that the experience matches the creative documents. A graybox, scaffold, occluded smoke, or scripted video is insufficient.

## 15. Examples

End-to-end: first launch enters the forest, the player walks with flashlight, streaming preloads the field-edge cells, wind drives grass/audio from one snapshot, an optional document is ignored, a checkpoint commits, and the title transition runs from a clean package.

Failure/recovery: the process is terminated during checkpoint. Relaunch detects the incomplete tail, loads the prior committed head, explains the recovery, and places the player at the declared safe checkpoint without source or package mutation.

Performance debug: PEN-B01 hitches at a route bend. The trace correlates a cell
activation, texture upload, vegetation draw burst, and audio stream refill. The
owner changes preload/variant partitioning, reruns identical workload/hardware,
and attaches before/after evidence without generalizing beyond the corpus.
