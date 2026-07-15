# Meridian Active Implementation Plan

Status: Phase 0 specification reconciliation and repository split verified; Phase 2 renderer foundation active.

Updated: 2026-07-14.

This file is the current evidence ledger and next-work queue. The normative architecture and full Phase 0–29 DAG live in [specs/MERIDIAN_MASTER_SPEC.md](specs/MERIDIAN_MASTER_SPEC.md) and [specs/IMPLEMENTATION_PHASES.md](specs/IMPLEMENTATION_PHASES.md). Do not duplicate the whole roadmap here.

## 1. Authority and status

Decision authority:

1. [v0.2 specification suite](specs/MERIDIAN_MASTER_SPEC.md)
2. this file for current evidence and active scope
3. the private `bybrooklyn/project-meridian` creative suite for game design/art/narrative/content
4. [the v0.1 migration ledger](docs/migrations/V0_1_DOCUMENT_MIGRATION.md) for historical/provisional material consolidated from deleted root documents
5. code as evidence, not automatic permanent architecture

Status terms:

- Implemented foundation: code and tests exist for the stated boundary.
- Structural smoke: resources/commands were constructed/submitted; pixels or product quality may remain unproven.
- Partial: useful code exists but named completion evidence is missing.
- Transitional: current implementation is intentionally behind a future Meridian-owned replacement seam.
- Planned: specified but not implemented.
- Research: selection awaits a named prototype/corpus/decision.

## 2. Stop rule

Finish and verify the active bounded work package before starting another renderer feature or subsystem. Do not use this plan as permission to implement the entire v0.2 suite.

The current implementation stop point is pass-level renderer timing and visible capture evidence. Cairn, Meridian UI, gameplay content, advanced rendering, audio/weather expansion, VCS/sync, XR, networking, mods, agents, and other future phases remain out of scope unless activated explicitly.

## 3. Current phase summary

| Phase | Status | Honest evidence boundary |
|---|---|---|
| P0 specification reconciliation | Verified | amendment, contradiction register, coordinated suite, 953-heading zero-unmapped migration ledger, private game-document split, examples, research, agent policy, links, Markdown integrity, publishing audits, formatting, and workspace tests verified |
| P1 workspace/platform/tasks/diagnostics | Partial | fixed-step, task and diagnostic foundations, winit/macOS native smoke; Linux/Windows/headless and broader lifecycle/recovery evidence remain |
| P2 renderer/forest viewport | Active partial | RHI, render graph validation, shaders, PBR, cascaded shadows, diffuse IBL, extraction/upload, structural native smoke; pass timing, visible captures, Forward+ forest viewport and calibrated corpus remain |
| P3 Cairn fork | Transitional precursor | grounded controller and Rapier wrapper exist; provenance-controlled fork and Cairn-native internals are Planned |
| P4 temporary editor | Scaffold/partial | editor/tool targets exist but no complete creator workflow evidence; the game is a separate private repository |
| P5 assets/world/save/package | Partial precursors | identity/manifests/IO/streaming/world/save foundations; source-world documents, facets, isolated import, final .meridian package remain |
| P6–P29 | Planned/Research except stated foundations | scaffold crates and specifications are not phase completion |

The old Engine-Ready Gate is removed. Phase 8 opening-forest work begins when its narrow capability dependencies pass; it does not wait for entire engine subsystems.

## 4. Evidence already present

### P1 runtime foundation

Implemented foundations:

- fixed-step runtime timing and frame diagnostics;
- task execution/cancellation foundations;
- structured diagnostic samples including late GPU duration attachment;
- winit platform boot path and macOS native smoke;
- generation-safe and deterministic primitives where current APIs state them.

Open evidence:

- Linux and Windows native lifecycle;
- headless/minimal product profile;
- crash/restart and process-worker recovery;
- topology-aware scheduling and complete memory attribution;
- accessibility platform adapter lifecycle.

### P2 RHI and render graph

Implemented foundations:

- backend-neutral RHI surface/device/capability descriptors with wgpu private behind the crate boundary;
- optional timestamp-query capability and frame-level duration readback;
- render graph validation for resources, hazards, producers, cycles, topological order, and lifetimes;
- clear-frame/native examples and structured surface outcomes;
- shader parsing/validation through Naga;
- startup pipeline construction and warmup safeguards.

Evidence:

- [RHI implementation](engine/meridian_rhi/src/lib.rs)
- [render graph](engine/meridian_render_graph/src/lib.rs)
- [clear-frame native example](engine/meridian_rhi/examples/clear_frame.rs)

Open:

- pass-scoped timestamp allocation/resolve/readback;
- executable graph barrier/aliasing sophistication beyond current boundary;
- per-pass CPU encoding/submission timing;
- device-loss/resource rebuild product workflow;
- Linux/Windows backend evidence.

### P2 PBR, shadows, and diffuse IBL

Implemented foundations:

- mesh and instance upload planning;
- camera/object/material uniforms;
- base-color, normal, and metallic-roughness channels;
- direct sun lighting and cascaded raster shadows;
- EnvironmentLight validation;
- six-face diffuse irradiance cube allocation/upload;
- PbrLightingResources descriptor grouping sun, shadow, irradiance, and environment resources;
- group-3 lighting/shadow/environment bind group construction;
- shader sampling of pre-convolved irradiance for diffuse ambient light;
- native smoke output reporting irradiance cube, bindings, pipeline, surface outcome, GPU duration, and diagnostics.

Evidence:

- [renderer lighting](engine/meridian_renderer/src/lighting.rs)
- [RHI lighting resources](engine/meridian_rhi/src/lib.rs)
- [native renderer smoke](engine/meridian_renderer/examples/instance_upload_smoke.rs)
- [PBR shader](shaders/textured_material_triangle.wgsl)
- [shader scope note](shaders/README.md)

Diffuse IBL status: complete for the current structural foundation.

Explicit future work: prefiltered specular environment IBL and BRDF integration LUT are not implemented. They are a bounded future P2 work package after instrumentation and visible evidence. Do not describe diffuse IBL as full IBL.

### P2 extraction and upload

Implemented foundations:

- immutable render snapshots and extraction boundaries;
- instance/material/mesh metadata and upload plans;
- bounded instance buffer growth/update;
- RHI resource construction exercised by native smoke.

Open:

- production visibility lists and clustered lighting;
- streaming-to-GPU activation and eviction integration;
- complete memory/bandwidth attribution;
- visible forest corpus.

### P5 asset, world, streaming, and save precursors

Implemented/partial foundations:

- deterministic AssetId and canonical metadata/manifests;
- required/optional dependency validation and pack entry lookup;
- file-backed range reads, cancellation, uncompressed decode, deterministic residency candidates;
- 64-bit world positions, default cells, origin rebasing, spatial records/residency;
- deterministic cell request/priority/cancellation and bounded activation;
- versioned save envelopes, checksums, atomic replacement, backup recovery, migrations, append-only records, and truncated-tail recovery.

Open:

- SourceId, ArtifactHash, FacetId, VariantKey, PackageChunkId model;
- process-isolated importers and deterministic artifact DAG;
- Zstandard decode path;
- schema-defined authoritative source-world directory;
- multi-reason streaming and GPU/audio/physics activation;
- final append-only transaction/snapshot/recovery-head model;
- chunked, mountable, signed .meridian format.

### P3/P4/P6+ precursors

- meridian_physics has a grounded controller and Rapier-backed service wrapper. It is Transitional and must not define Cairn API compatibility.
- bevy_ecs use is Transitional behind Meridian persistent IDs/commands/schema boundaries.
- audio, UI, terrain, vegetation, and weather crates are marker/scaffold foundations only.
- editor/tool scaffolds do not prove product workflows; game code is not part of this workspace.

## 5. Closed work package — diffuse IBL engine slice

Result:

- PbrLightingResources groups sun buffer, shadow map/parameters, irradiance cube, and environment parameters without suppressing clippy.
- group-3 bindings and diffuse irradiance behavior are preserved.
- specular IBL, BRDF LUT, game content, and unrelated renderer features were not added.
- environment validation, workspace tests, native structural smoke, clippy, formatting, and diff checks were the required closure sequence.

Known limit: the structural native surface may be occluded; visible-pixel quality is not claimed.

## 6. Active work package — P2.8 pass-level renderer timing

### User-visible result

The renderer profiler and capture output identify CPU encoding and GPU duration by named render pass, correlate them to one frame, and report unsupported or unavailable timing without fabricating zeroes.

### Why now

Frame-level GPU duration exists, but further renderer polish cannot be evaluated or regression-gated without pass attribution. This is the next separate slice after diffuse IBL closure.

### Dependencies

- current RHI timestamp capability/readback;
- render graph pass identity/order;
- diagnostics frame samples and trace IDs;
- native renderer smoke.

### Explicit non-goals

- specular IBL or BRDF LUT;
- Forward+ clustered lighting;
- visibility buffer, virtual geometry, dynamic GI, ray tracing;
- game content or forest visual polish;
- broad profiler/editor UI;
- replacing wgpu.

### Work packages

P2.8.1 Pass identity and timing record

- define stable per-build PassId and display name;
- define PassTiming with frame, CPU encode interval, optional GPU interval, queue/readback status, and unsupported reason;
- keep diagnostics independent of wgpu types.

P2.8.2 Timestamp planning

- assign bounded query pairs to timed passes;
- handle device query limits and pass selection;
- write begin/end timestamps at correct render/compute boundaries;
- resolve/copy once per frame into a bounded rotating readback pool.

P2.8.3 Asynchronous readback

- never block frame submission waiting for timestamps;
- tag readback by frame/epoch and reject stale/device-lost data;
- convert ticks with validated timestamp period;
- report disjoint/invalid/unsupported outcomes.

P2.8.4 CPU timing

- measure graph planning and per-pass command encoding with monotonic clock;
- distinguish CPU encoding from queue wait/submit and presentation;
- minimize observer overhead and record whether timing is enabled.

P2.8.5 Diagnostics and native evidence

- attach pass timings to frame diagnostics;
- print/serialize named pass results in native smoke;
- add tests for supported/unsupported/limit/readback/ordering/device-loss paths;
- preserve existing frame-level duration compatibility until migration is complete.

### Acceptance

- targeted timestamp/pass tests pass;
- cargo test --workspace passes;
- native renderer smoke reports named pass timing or a precise unsupported reason and still proves current resource/pipeline/bind-group construction;
- cargo clippy --workspace --all-targets -- -D warnings passes;
- cargo fmt --all -- --check passes;
- git diff --check and untracked-file audit pass;
- one trace/report states hardware, backend, capability, surface outcome, frame ID, pass names, CPU time, optional GPU time, and observer mode.

### Failure behavior

Timestamp unsupported: CPU timings remain and GPU field is Unsupported(capability). Query exhaustion: declared passes are selected/dropped deterministically with diagnostic. Readback not ready: frame remains pending/dropped by bounded policy, never blocks. Device loss: epoch invalidates readbacks and timings identify the loss.

## 7. Next queue after P2.8

Only activate one bounded package at a time:

1. P2.9 visible capture/reference path with explicit visible, occluded, minimized, unsupported, and device-lost outcomes.
2. P2.10 executable B01/B02 renderer corpus and initial calibration.
3. P2.11 bounded specular IBL: environment prefilter plus BRDF integration LUT and visible/reference evidence.
4. P2.12 depth prepass and clustered Forward+ forest baseline.
5. P5 narrow Zstandard decode and streaming-to-GPU activation slice.
6. Phase 8 capability work selected from the vertical-slice plan.

This ordering may change only with a documented reason and affected evidence. Do not bundle these into one renderer epic.

## 8. Phase 0 documentation closure

Written:

- full required v0.2 file set under specs/;
- authority and contradiction register;
- subsystem boundaries, APIs, pipelines, data, failure, tests, phases, and examples;
- Phase 0–29 DAG and opening vertical slice;
- current primary-source research and algorithm gates;
- root/spec agent policies;
- legacy document migration policy;
- engine/game repository boundary and private creative-document relocation.

Verified on 2026-07-14:

- all 27 required specification filenames exist;
- all 953 level-1 through level-3 headings from the seven v0.1 root documents have explicit mapped dispositions and the superseded files are deleted;
- all repository Markdown relative links resolve;
- Markdown fences and untracked-file whitespace checks pass;
- stale contradictory claims are either removed, bannered as historical, or intentionally recorded in the migration register;
- README points to v0.2 and states current renderer evidence honestly;
- a clean engine copy with no `game/` directory passes locked Cargo metadata, while the outer repository ignores the independent private game repository completely;
- credential-signature, personal-path, machine-local-file, `.DS_Store`, generated-artifact, and oversized-file publication audits pass;
- cargo fmt --all -- --check, cargo test --workspace, and clippy with warnings denied pass;
- native renderer smoke proves six-face irradiance upload and pipeline/bind-group construction with SkippedOccluded surface outcome.

Documentation and repository-boundary closure is complete. The initial source-control checkpoint records this state; future changes use the normal amendment and validation process.

## 9. Cross-cutting gates

Every package:

- names owning crates and invalid dependencies;
- keeps third-party types behind Meridian seams;
- records thread/task/memory and disabled-feature cost;
- includes failure/recovery, not only happy path;
- updates formats/migrations and fixtures;
- includes keyboard/semantic/accessibility behavior for user-visible work;
- treats inputs as untrusted and capabilities as deny-by-default;
- records provenance/licensing for borrowed source/dependencies;
- updates specs, examples, validation, Ponder/docs, and this evidence ledger;
- emits the sign-off format in AGENTS.md.

## 10. Benchmark policy

Use [the validation spec](specs/TESTING_BENCHMARKS_AND_VALIDATION.md). Initial required hardware records include an exact M4 MacBook Air 16 GB configuration and the exact main Windows/Linux-class PC available to the project. Representative GPU/XR/server tiers are added only when hardware exists.

Legacy v0.1 numbers remain provisional. A threshold becomes a gate only after hardware, OS/driver, build/profile, corpus hash, cache/warmup, repetitions/statistics, variance, and regression rule are recorded.

## 11. Deferred and research

Deferred does not mean abandoned:

- Meridian UI permanent framework and egui deletion;
- Cairn fork, structures, deformables;
- advanced renderer portfolio;
- full audio/acoustics, weather/environment, procedural authoring;
- Blender/live link/native tools;
- build service/Cargo IDE;
- VCS/P2P/live collaboration;
- OpenXR;
- multiplayer/providers;
- modding/community library;
- typed MCP/Ollama/agent features;
- buildings/ecosystems and coupled simulations;
- additional languages;
- 1.0/LTS/certification.

Selections remain in [research decisions](specs/RESEARCH_AND_ALGORITHM_DECISIONS.md). No deferred feature may leak cost into the current minimal runtime.

## 12. Definition of done

A work package is done only when:

- its user-visible or technical result works at the stated boundary;
- targeted and proportional workspace gates pass;
- raw evidence and limitations are recorded;
- failure/recovery and unsupported paths are tested;
- security, accessibility, provenance, and compatibility obligations are met;
- specs and PLANNING reflect reality;
- no unrelated phase was silently started.

A phase is done only with its complete evidence manifest. A scaffold is never done.
