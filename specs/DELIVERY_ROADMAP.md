# Meridian Delivery Roadmap

[Master](MERIDIAN_MASTER_SPEC.md) · [Implementation planning](IMPLEMENTATION_PLANNING_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Active plan](../PLANNING.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Normative evidence roadmap

## 1. Authority and delivery model

This document owns Meridian's delivery order. Milestones are evidence gates, not dates, sprints, or exclusive permission barriers. Workstreams may proceed in parallel when their declared dependencies pass, but no milestone is complete because a crate, type, panel, benchmark definition, or construction smoke exists.

The machine-readable work-package, milestone, and delivery-plan metadata under [`specs/registry/`](registry/) is authoritative for identifiers and cross-references. This document owns the human-readable outcomes, sequencing, and gate intent. [The implementation-planning specification](IMPLEMENTATION_PLANNING_SPEC.md) owns package activation, readiness, completion, concurrency, and replanning. Historical phase identifiers are confined to [the v0.3 roadmap migration ledger](../docs/migrations/V0_3_ROADMAP_MIGRATION.md).

## 2. Completion contract

Every milestone review records:

- user-visible result and supported profiles;
- source checkpoint, BuildId when available, dependency lock, and corpus hashes;
- satisfied `REQ-*` identifiers and completed `WP-*` packages;
- tests, benchmarks, captures, traces, recovery, accessibility, security, and provenance evidence;
- unsupported, occluded, redacted, stale, waived, or inconclusive rows without converting them to passes;
- remaining risks, expiring waivers, owners, and the next unblocked package.

`REQ-GOV-001` requires documentation maturity and implementation maturity to remain separate. `REQ-GOV-002` prohibits scaffold or definition-only work from being reported as implementation completion.

## 3. Workstream model

| Workstream | Domain | Long-lived result | Current implementation boundary |
|---|---|---|---|
| Governance and release | GOV, REL, SEC | truthful specifications, evidence, trust, compatibility, and release policy | v0.5 closure passed locally; recent remote workflows concluded `failure` before their skipped implementation rows could run |
| Runtime and platform | CORE, RUN | clocks, tasks, diagnostics, input, platform lifecycle, recovery | implemented foundations; platform matrix incomplete |
| RHI and Penumbra | RHI, PEN | capability-driven GPU abstraction and Meridian-owned renderer | direct PBR/shadow/diffuse-IBL foundation; production Forward+ incomplete |
| Creator experience | UI, EDT | Meridian UI and an accessible, recoverable editor | MS-02 UI core proof and `WP-EDT-001` Creator behavior are qualified; `WP-UI-005` remains `Partial` and unqualified, `WP-UI-006` is the sole active source-only authoring package under non-promoting waivers, and the production shell remains open |
| Data and production | DAT, BLD, MDL, DCC | source authority, imports, worlds, packages, saves, build graph, native modeling, optional DCC tools | build and Creator foundations are qualified; `WP-MDL-001` retains its bounded editable-model foundation as `Partial` |
| World authoring and simulation | PHY, ISO, BAS, VEG, PRC, TOR | Cairn, Isobar, Basalt, vegetation, Alluvium, and coupled simulation | physics wrapper transitional; named environmental crates scaffold; Alluvium `WP-PRC-001` is `ImplementedFoundation` |
| Game and media | GAM, FWK, ANI, NAV, PRJ, AUD | Rust-first gameplay, optional Luau, reusable frameworks, animation/navigation, Project Meridian, Wavefront | game external; Wavefront scaffold; other domains planned |
| Rendering languages and 2D | SHD, TWO | one ShaderIr plus dedicated first-class 2D paths | planned; current WGSL/3D foundations do not implement them |
| Ecosystem and online | VCS, SYN, NET, COL, MOD, AGT | version control, sync, networking, Collective, modding, typed agents | planned, research, or deferred |
| Extended and post-1.0 | XR, WRL, INT | OpenXR plus separately gated distributed-world and integrity programs | XR deferred; WRL/INT post-1.0 only |

Workstream ownership and forbidden dependency edges are defined in the owning subsystem specifications. Work starts only through bounded `WP-*` packages in the registry and [PLANNING.md](../PLANNING.md).

### 3.1 Execution control

Every milestone has machine-checked entry conditions, a critical package path,
parallel lanes, an integration checkpoint, required evidence, and explicit stop
conditions in [`registry/delivery-plan.json`](registry/delivery-plan.json).
Milestone tables deliberately stay coarser than active package briefs: the
active horizon is exact, the next horizon is dependency-ready, and distant
work remains milestone-ready until its inputs stabilize.

A package cannot become `Active` until it satisfies the Definition of Ready,
and it cannot close until it satisfies the Definition of Done in the
[implementation-planning specification](IMPLEMENTATION_PLANNING_SPEC.md).
Passing isolated packages is necessary but insufficient: the milestone's exit
integration and review must also pass.

## 4. Evidence milestone graph

~~~text
MS-00 governance
  └─ MS-01 observable runtime and source foundations
       ├─ MS-02 Meridian UI core proof
       │    └─ MS-03 Creator Editor Alpha ─────────────┐
       └─ MS-04 Penumbra Forward+ foundation           │
            └─ MS-05 representative forest renderer ───┤
                                                      └─ MS-06 Project Meridian prototype
                                                           └─ MS-07 opening playable slice
                                                                └─ MS-08 Engine Alpha + native Metal
                                                                     └─ MS-09 Engine Beta + native Vulkan/D3D12
                                                                          └─ MS-10 1.0 qualification

RG-PEN-001 may open after MS-05 and runs independently.
It does not block MS-06 through MS-10 unless a later adopted ADR explicitly changes that rule.
RG-PRC-001 may open after MS-01. RG-PRC-002 remains closed until MS-05.
~~~

## 5. MS-00 — v0.5 governance closure

User-visible result: a contributor can locate the winning architecture, current implementation truth, risks, next bounded package, and required evidence without reconciling contradictory documents.

Required result:

- coordinated v0.5 specifications and authority index;
- stable identifier registries and schemas;
- canonical ADRs and risk/provenance policies;
- zero-unmapped legacy roadmap, weather-spec, Alluvium, and general-purpose-platform migrations;
- one maturity record for all 37 current domains, post-1.0 `PRG-*` separation, validation-project registry, and dependency strategy;
- `meridian-spec` validation in CI before the Rust matrix;
- clean workspace metadata, format, tests, clippy, smoke, links, and publication-hygiene audits.

Non-goal: implementing any planned runtime subsystem. Primary packages:
`WP-GOV-001` for the v0.3 foundation, `WP-GOV-002` for execution planning,
`WP-GOV-003` for the v0.4 Alluvium amendment, and `WP-GOV-004` for the v0.5 general-purpose platform amendment.

## 6. MS-01 — observable runtime and source foundations

User-visible result: a minimal Meridian application opens, runs bounded clocks/tasks/input, renders through the current RHI/render graph, reports pass timing and surface outcome, captures a visible frame where supported, and loads source-derived data with recovery diagnostics.

Required capabilities:

- platform lifecycle, input, tasks, diagnostics, cancellation, and recovery;
- current RHI and render-graph lifetime/hazard contracts;
- `WP-PEN-007` pass-level CPU/GPU timing and asynchronous readback;
- `WP-PEN-008` visible capture with explicit visible, occluded, minimized, unsupported, and device-lost outcomes;
- source-data identity, import, world, streaming, and save foundations needed by later milestones.

Critical path: `WP-PEN-007` -> `WP-PEN-008` -> `WP-RUN-004` ->
`WP-REL-002`. Runtime lifecycle and diagnostic correlation proceed through
`WP-RUN-002` and `WP-RUN-003`; source import, world-cell activation, and
save/package recovery proceed through `WP-DAT-002`, `WP-DAT-003`, and
`WP-DAT-004`. `WP-RUN-004` is the convergence package, not another umbrella
for implementing those prerequisites.

The current renderer structural smoke is evidence only for construction/submission. It cannot satisfy the visible-capture gate.

Current result: `MS-01` and `WP-REL-002` are `Pass`/`Implemented` after GitHub
Actions run `29452928922` passed governance plus Linux, Windows, and macOS
workspace/headless-smoke rows for `010db80`. The `meridian` executable
imports and packages the public generic fixture, worker-streams and atomically
activates one compiled cell, advances semantic-input/fixed-runtime foundations,
renders package-derived geometry, writes a hashed offscreen-visible PNG when the
native surface is occluded/unavailable, proves save recovery and fresh-disk
reconstruction, and emits one correlated JSON timeline. This closes the named
implementation packages and MS-01 qualification; it does not claim presented
pixels, production image quality, stable final formats, or UI/editor workflows.

## 7. MS-02 — Meridian UI core proof

User-visible result: one accessible Meridian-native panel and one runtime overlay render without permanent dependence on the bootstrap UI.

Required capabilities: text shaping and IME, retained layout, input/events/focus, semantic tree, display list, Penumbra bridge, keyboard-only operation, scaling/contrast/reduced-motion behavior, diagnostics, deterministic UI fixtures, and disabled-profile zero-cost evidence.

Primary packages begin at `WP-UI-001`. A full editor is not required here.

## 8. MS-03 — Creator Editor Alpha

User-visible result: a creator opens a project in the permanent Meridian shell,
imports an asset, edits a world through hierarchy/viewport/inspector, undoes
changes, enters isolated Play mode, builds/runs a sample, changes workspace and
focus layouts, and recovers an interrupted session.

Required capabilities: the qualified `WP-EDT-001` project/session, persistence,
typed command, undo/checkpoint, recovery, Play, and build behavior; sequential
`WP-UI-002` through `WP-UI-005` retained framework, professional interaction,
docking/state, motion/effects, renderer decision, and platform accessibility;
then `WP-EDT-002` permanent shell, hub, production World workspace, native
captures, and visible review. Bootstrap UI remains isolated and deletable.

The critical path is `WP-UI-002 -> WP-UI-003 -> WP-UI-004 -> WP-UI-005 ->
WP-UI-006 -> WP-EDT-002 -> WP-EDT-003`. Normally, a source package advances
only after its Linux, Windows, and macOS evidence passes. Under the existing
non-promoting waivers, `WP-UI-006` is the sole active local source package;
`WP-EDT-002` resumes afterwards and `WP-EDT-003` composes the current Creator
workspaces in MS-03. Explicit unavailable states may expose a domain boundary,
but cannot substitute for that domain's foundations or qualify a package.

Alluvium contributes textual recipe, headless validation/evaluation, and basic
typed-inspector foundations through `WP-PRC-001`. The complete visual graph
editor is not required for Editor Alpha.

`WP-MDL-001` begins the native editable-mesh and beginner-modeler foundation in the same Meridian application. Editor Alpha must establish the command, source-document, undo, recovery, viewport, and inspection seams it consumes. The package may finish across MS-03/MS-05, but it must pass before `WP-PRJ-001` starts.

Project Meridian prototype work does not begin before this gate and MS-05 both pass.

## 9. MS-04 — Penumbra Stage 1: production Forward+ foundation

User-visible result: Penumbra renders an instrumented production-shaped scene through its adopted clustered Forward+ architecture.

Required capabilities: GPU scene and render views, depth/visibility foundation, clustered light assignment, indirect/table-driven submission where supported, unified material/shader IR direction, sun/local lights/shadows, diffuse and specular environment-light packages, Alluvium/Basalt/vegetation/Isobar seams, temporal reconstruction, profiling, capture, and recovery. Capability tiers select behavior; named GPU assumptions do not.

Current direct PBR, cascaded shadows, diffuse irradiance IBL, extraction, upload, RHI, and render graph are `ImplementedFoundation` or `Transitional`; they do not complete this milestone. Primary renderer packages are `WP-RHI-001` and `WP-PEN-001` through `WP-PEN-010`.

## 10. MS-05 — Penumbra Stage 2: representative forest renderer

User-visible result: a generated representative night forest demonstrates terrain, dense vegetation, flashlight/local lights, shadows, fog, basic Isobar weather, streaming, temporal stability, debugging, and measured performance.

Required evidence uses `PEN-B01`, `PEN-B02`, `PEN-B03`, `PEN-B08`, `PEN-B09`, `PEN-B10`, `PEN-B13`, and `PEN-B15` as applicable. Definition-only workloads must first become executable, calibrated corpora. Quality claims require visible captures; performance claims require preregistered hardware/profile/statistical methods.

The critical corpus path includes `WP-PRC-001` through `WP-PRC-004` and
`WP-PEN-011`. Alluvium must provide typed recipes/fields, deterministic
evaluation, generated identity, overrides, provenance, a sanitized forest/field
corpus, and Basalt/vegetation/Isobar handoffs. This requirement does not imply
the later visual editor, materials/weathering production, structures,
runtime-safe recipes, or ecosystem succession.

`WP-MDL-001` also closes its prototype gate here: editable source, stable element identity, baseline topology tools, undo/recovery, materials, simple collision/LOD source facets, and beginner accessibility require evidence. This is not the advanced modeling program.

Passing MS-05 opens `RG-PEN-001` successor research but does not require a successor.

## 11. MS-06 — Project Meridian prototype

User-visible result: the separate private game repository consumes published Meridian contracts to produce a bounded, non-production prototype with movement, interaction, representative forest presentation, minimum Cairn/Wavefront/Rust-gameplay/save integration, native-modeler-produced or accepted assets, and a reproducible package.

Dependencies: MS-03, MS-05, `WP-MDL-001`, and the other typed `WP-PRJ-001` dependencies. Rust is the prototype gameplay language; Luau is not a prototype prerequisite. The sanitized engine-facing contract is [PROJECT_MERIDIAN_PROTOTYPE_PLAN.md](PROJECT_MERIDIAN_PROTOTYPE_PLAN.md). Creative authority remains private. No private asset, logo, document, route, or narrative payload enters this repository.

The prototype is learning evidence, not the production opening slice.

## 12. MS-07 — complete opening playable slice

User-visible result: an uninvolved reviewer can install, launch, traverse, configure, recover, and inspect the production-quality opening slice.

The full engine-facing acceptance contract is [PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md). Completion includes performance, visual, audio, accessibility, save/recovery, package, provenance, and private creative sign-off evidence.

`WP-PRJ-002` owns production-slice integration. Its local `VS-*` slices are
activated as reviewable tasks only when MS-06 evidence and the private creative
checkpoint satisfy package readiness.

MS-07 is the earliest native-backend implementation unlock. It does not itself authorize Metal work until `RG-RHI-001` and stable RHI-contract review also pass.

## 13. MS-08 — Engine Alpha and native Metal

User-visible result: a documented Alpha profile supports real creator workflows and the opening slice while Penumbra gains a native Metal backend without losing wgpu.

Required capabilities include mature editor/data/build workflows; selected Wavefront, general animation, navigation, Rust gameplay framework, first-class 2D, native modeler, shader-language, and simulation foundations; stable RHI contracts; differential rendering; device-loss recovery; benchmark parity; and a maintenance plan. Each selected profile remains independently evidence-gated. `RG-RHI-001` preregisters native-backend entry thresholds and compares abstraction cost against measured needs.

Alpha work includes the selected later Alluvium packages for materials/weathering, infrastructure/structures, native visual authoring, and runtime-safe recipes when independently ready. `WP-GAM-001`, `WP-FWK-001`, `WP-ANI-001`, `WP-ANI-002`, `WP-NAV-001`, `WP-TWO-001`, `WP-SHD-001`, and `WP-MDL-002` remain separate packages; Artus supplies only its qualified rig/clip/pose and first usable humanoid foundations here. Listing them under Alpha does not make every optional capability a universal profile requirement. Advanced Cairn, DCC integration, VCS, typed agents, Torsant research, and broader Isobar/Basalt/vegetation work remain separately gated.

`WP-BLD-002` continues the MS-03 local-Cargo foundation with managed external
development toolchains, multi-node result lineage, general artifact/cache
policy, service-process/remote-worker supervision, and team profiles. It is an
MS-08 package and cannot delay the `WP-BLD-001` prerequisite for Creator Editor
Alpha.

Critical packages: `WP-RHI-002` native Metal and `WP-REL-003` Alpha
qualification. These are distinct from the current `WP-RHI-001` foundation.

## 14. MS-09 — Engine Beta and native Vulkan/Direct3D 12

User-visible result: Beta supports the selected desktop/platform capability matrix with mature common RHI behavior and native Vulkan and Direct3D 12 implementations where gates pass.

Entry requires mature native Metal plus common-RHI differential image, benchmark, recovery, backend-divergence, staffing, and maintenance evidence. wgpu remains available. Selected Beta work may include synchronization, networking/providers, optional Collective modules, modding, XR, `WP-GAM-002` Luau, Artus pose search, interaction, motion LOD, high-level intent replication, and narrow facial foundations, advanced navigation, shader target lowering, and platform ecosystem integration; none is implied by the milestone name unless its profile is declared. Collective is provider-neutral and self-hostable; Beta does not promise a Meridian-operated cloud.

Critical packages: `WP-RHI-003` native Vulkan/Direct3D 12 parity and
`WP-REL-004` Beta qualification.

## 15. MS-10 — 1.0 qualification

User-visible result: declared 1.0 profiles pass compatibility, certification, recovery, security, accessibility, provenance, reproducibility, migration, support, and LTS gates.

Unsupported capabilities and platforms remain explicit. A successor renderer path is not required for 1.0. No release claim may rely on an expired waiver, unredacted private corpus, definition-only benchmark, occluded visual result, or uncalibrated provisional threshold.

MS-10 does not require or authorize a competitive-superiority claim.
`PRG-REL-001` may begin only afterward and only through its independent entry
gates, matched evidence, and future bounded packages.

## 16. Penumbra successor and native-backend gates

`RG-PEN-001` compares the production Forward+ path with successor candidates only after MS-05. Promotion requires preregistered material-improvement thresholds, complete feature parity, equal-or-better artistic results, lower-tier stability, native-backend evidence, acceptable shader/pipeline behavior, no material frame-time/memory/debugging regression, and sustainable maintenance cost. A trivial gain cannot justify substantial complexity.

If a successor is promoted, Forward+ retention, fallback, or removal requires a separate ADR. v0.5 promises neither permanent retention nor removal.

`RG-RHI-001` controls native-backend entry. Metal is first after MS-07; Vulkan and Direct3D 12 follow only after Metal and common RHI gates pass. Backend support is capability-driven and recorded per profile.

## 17. Post-1.0 program boundary

MS-10 closes declared Meridian 1.0 profiles. It does not flatten every long-term ambition into the 1.0 critical path. `PRG-MDL-001`, `PRG-ANI-001`, `PRG-FWK-001`, `PRG-COL-001`, `PRG-WRL-001`, `PRG-INT-001`, `PRG-VCS-001`, `PRG-SHD-001`, `PRG-PRM-001`, and `PRG-REL-001` begin only after their own entry gates. Programs cannot satisfy, block, or promote MS-00 through MS-10, and their `Deferred` or `Research` status is not an implementation claim. Marquee remains entirely deferred: its future local exports, optional text/analysis AI, and adapter research create no 1.0 obligation. Competitive leadership is likewise deferred: adopted seams and benchmark definitions create no solver, optimization, comparator integration, corpus calibration, or superiority evidence.

## 18. Change control

Roadmap changes require affected `REQ-*` and `WP-*` identifiers, dependency and milestone impact, old/new disposition in the migration register, an ADR for architectural sequencing changes, and validator-clean registries. PLANNING may activate only one bounded primary package at a time, while explicitly listed sidecar research or maintenance may continue in parallel. Any changed critical path, parallel lane, integration checkpoint, or stop condition also updates `registry/delivery-plan.json`.
