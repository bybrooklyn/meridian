# Meridian Implementation Phases

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Active plan](../PLANNING.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Opening slice](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)

Version 0.2 · 2026-07-14 · Normative delivery DAG

## 1. Rules

Phases are capability and evidence gates, not dates. Numbering communicates the dominant integration sequence; it does not prohibit parallel research or isolated work. No phase completes with crates, types, empty panels, or passing construction tests alone.

Every work package records:

- ID, user-visible result, dependencies, owner, and explicit non-goals;
- crates/documents/schema/API/process/thread/file changes;
- ordered implementation and migration;
- editor, CLI, accessibility, Ponder, and expert workflows;
- runtime data flow, memory ownership, error and recovery behavior;
- unit/integration/differential/fuzz/determinism/accessibility/recovery tests as applicable;
- benchmark corpus, hardware, metrics, provisional threshold, and regression policy;
- evidence paths, audit sign-offs, and source-control checkpoint.

Work packages SHOULD fit one implementation agent or one tightly reviewed change. Broad labels such as implement renderer or build physics engine are invalid.

## 2. Dependency DAG

~~~text
P0
 └─ P1
    ├─ P2 ─────────────┐
    ├─ P3 ────────┐    │
    └─ P4 ─┐      │    │
           ├─ P5 ─┼─ P7 ─┐
           └─ P6 ─┘      ├─ P8 opening forest
              │           │
              └───────────┘

P8 -> P9 -> P10 -> P11 -> P12
P3 + P8 -> P13
P5 + P11 -> P14 -> P15
P1 + P5 + P9 -> P16 -> P17 -> P18
P3 + P9 + P12 -> P19
P10 + P11 -> P20
P3 + P11 + P13 -> P21
P5 + P7 + P16 -> P22 -> P23
P7 + P9 + P17 + P22 -> P24
P6 + P16 + P17 -> P25
P14 + P21 -> P26
P11 + P20 + P21 -> P27
P7 + P16 + P24 -> P28
all shipping paths -> P29
~~~

Phase 8 is the early critical path. Phase 12 research, full Cairn ownership, external DCC links, VCS, collaboration, XR, multiplayer, mods, agents, additional languages, and advanced simulation MUST NOT delay it unless a Phase 8 acceptance criterion directly depends on a narrow seam.

## 3. Current evidence mapping

| Phase | Status on 2026-07-14 | Evidence boundary |
|---|---|---|
| P0 | In progress | amendment read and contradiction register written; full suite/audit/checkpoint not yet complete |
| P1 | Partial | fixed-step, diagnostics, tasks, winit/macOS smoke exist; Linux/Windows/headless and hardening open |
| P2 | Active partial | RHI/render graph/PBR/shadows/diffuse IBL structural paths exist; pass timings, visible captures, Forward+, forest viewport open |
| P3 | Transitional precursor | Rapier wrapper and grounded controller exist; Cairn fork/provenance/native ownership open |
| P4 | Scaffold/partial | editor/tool crates exist; no complete temporary editor product evidence; game code lives in a separate private repository |
| P5 | Partial precursors | asset/streaming/world/save foundations exist; source world, facets, final package/import isolation open |
| P6+ | Planned unless a subsystem spec states otherwise | marker/scaffold crates are not completion evidence |

## 4. Phase 0 — Specification reconciliation and research foundation

User-visible result: contributors can locate one winning decision, current status, next work, and evidence without reconciling contradictory documents themselves.

Dependencies: none. Non-goals: production subsystem implementation or freezing research choices without evidence.

Work packages:

- P0.1 inventory docs, code, crates, formats, tests, licenses, and current diffs.
- P0.2 write contradiction/duplication/authority register.
- P0.3 create coordinated v0.2 specs, source-of-truth index, root agent policy, and active plan.
- P0.4 establish research register with primary sources, prototypes, corpus, metrics, deadline, owner, archive rule, and stable seam.
- P0.5 define benchmark/capture artifact schemas and phase evidence manifest.
- P0.6 audit links, stale claims, status wording, format examples, and legacy banners.

APIs/formats: SpecRequirementId, ResearchGateId, EvidenceManifest schema. UX: Ponder/source-of-truth navigation and a future spec status panel. CLI: planned meridian spec check and evidence verify.

Tests/benchmarks: link/heading/status validators; no invented performance threshold. Security: no secrets or private source URLs in public evidence. Completion: all required documents exist, audits pass, old conflicts route to the register, and a source-control checkpoint records the migration.

## 5. Phase 1 — Workspace, platform, tasks, diagnostics, and minimal window

User-visible result: a portable minimal app opens, handles lifecycle/input, runs bounded fixed ticks, emits structured diagnostics, and exits/restarts cleanly.

Dependencies: P0 architecture. Non-goals: production renderer/editor/game.

Work packages: P1.1 dependency/feature rules; P1.2 clocks and fixed-step; P1.3 task classes/cancellation/topology; P1.4 diagnostics/crash/recovery; P1.5 platform contracts; P1.6 macOS evidence; P1.7 Linux harness; P1.8 Windows harness; P1.9 headless/minimal profiles; P1.10 CI.

APIs: PlatformHost, TaskDescriptor, FrameTiming, RuntimeEvent, generational Handle. Processes: supervised workers only. Accessibility: minimal recovery/window controls keyboard and screen-reader reachable.

Evidence: timing/jitter tests, shutdown/cancellation torture, stale-handle tests, crash recovery, task/memory trace, native smoke on named platform. Security: bounded IPC/process arguments and redaction. Completion requires all claimed platform rows, not only compilation.

## 6. Phase 2 — Basic renderer and forest viewport

User-visible result: a forest test viewport renders measurable, readable PBR geometry with shadows, diffuse environment light, camera controls, imported assets, and captures.

Dependencies: P1. Non-goals: advanced GI, virtual geometry, hardware RT, broad renderer polish.

Work packages: P2.1 RHI lifetime/capability hardening; P2.2 executable render graph; P2.3 extraction/upload; P2.4 shader reflection/cache; P2.5 camera/mesh/material; P2.6 sun/cascaded shadows; P2.7 diffuse IBL closure; P2.8 pass-level CPU/GPU timings; P2.9 visible capture with unsupported/occluded outcomes; P2.10 depth prepass/clustered Forward+ baseline; P2.11 forest corpus and warmup.

Current status: P2.7 is Implemented foundation. P2.8 is immediate next. Prefiltered specular IBL/BRDF LUT is a bounded future P2 item after instrumentation, not diffuse IBL completion.

Evidence: shader tests, graph hazards, upload/bind-group tests, six-face irradiance smoke, visual captures, pass timings, pipeline-creation count, device-loss recovery, B01/B02 executable corpus. Security: shader/import limits. Completion requires visible forest evidence on first hardware tier.

## 7. Phase 3 — Cairn fork foundation

User-visible result: opening movement and collision run through Cairn-owned descriptors/handles with provenance and differential evidence.

Dependencies: P1; P2 debug rendering assists. Non-goals: full destruction/deformables/vehicles.

Work packages: P3.1 pin/archive Rapier and selected Box2D sources/licenses; P3.2 provenance/build reproducibility; P3.3 Cairn-native body/shape/query API; P3.4 broadphase/narrowphase seam; P3.5 fixed-step islands/solver; P3.6 character collision; P3.7 snapshot/debug draw; P3.8 Rapier differential corpus; P3.9 wrapper migration.

Evidence: provenance manifest, differential traces, query/controller scenes, determinism modes, benchmark and memory captures, save/handle migration fixture. No Rapier type may cross the public seam at completion.

## 8. Phase 4 — Temporary editor shell and project model

User-visible result: a creator opens a project, sees hierarchy/viewport/inspector/assets/logs, edits through commands, presses Play, and recovers a crashed session.

Dependencies: P1–P3 narrow runtime seams. Non-goals: permanent egui architecture or complete content tools.

Work packages: P4.1 project/session model; P4.2 typed command/undo/checkpoint; P4.3 egui bootstrap shell; P4.4 hierarchy/selection; P4.5 inspector; P4.6 asset browser; P4.7 logs/diagnostics; P4.8 Play fork/apply-back; P4.9 recovery.

egui stays only in meridian-editor-egui-bootstrap. Evidence includes command parity with CLI, undo/recovery journeys, play isolation, and a migration inventory for every panel.

## 9. Phase 5 — Asset database, source worlds, packages, and saves

User-visible result: source assets/worlds import deterministically, stream into play, save/recover, and export a basic mountable .meridian package.

Dependencies: P1–P4. Non-goals: final store/update ecosystem or advanced procedural worlds.

Work packages: P5.1 identity/facets/provenance; P5.2 isolated import workers; P5.3 artifact DAG/cache; P5.4 schema-defined world directory; P5.5 compiled cell chunks; P5.6 multi-reason streaming; P5.7 save journal/snapshot/repair; P5.8 package superblock/chunks/index/mount; P5.9 signing seam; P5.10 inspect/diff/rebuild CLI.

Evidence: deterministic rebuild, malformed/fuzz corpus, truncated save/package recovery, mount/range/patch tests, import crash restart, unknown-field migration, and no source dependence in shipping package.

## 10. Phase 6 — Meridian UI minimum viable framework

User-visible result: a Meridian-native accessible panel and runtime overlay render without egui.

Dependencies: P1, P2, P4, P5 schemas. Non-goals: complete editor migration.

Work packages: P6.1 node/property/event model; P6.2 text/shaping/IME; P6.3 layout; P6.4 display list/render bridge; P6.5 focus/semantics/AccessKit adapter; P6.6 .mui parser/compiler; P6.7 Rust builder; P6.8 basic widgets/virtual lists; P6.9 first editor panel migration; P6.10 UI golden/profiler.

Evidence: DPI/locale/layout corpus, keyboard/controller/screen-reader journeys, virtualized stress test, device-loss recovery, no egui data/API dependency in migrated panel.

## 11. Phase 7 — Luau gameplay and core logic

User-visible result: designers author interactions/state flows and Luau behavior with validation, hot reload, debugging, and save-safe state.

Dependencies: P1, P4–P6. Non-goals: C#, Python, Anorak, or mixed runtimes.

Work packages: P7.1 one gameplay API schema; P7.2 generated Rust/Luau bindings; P7.3 sandbox/capabilities; P7.4 module/artifact model; P7.5 hot reload with state policy; P7.6 interaction/action/state-flow IR; P7.7 narrative integration; P7.8 debugger/profiler; P7.9 save migration.

Evidence: sandbox escape tests, API parity, deterministic fixtures where promised, hot-reload rollback, instruction/memory budgets, logic reachability, and save compatibility.

## 12. Phase 8 — Project Meridian opening-forest playable slice

User-visible result: the final-game midnight forest opening is playable for roughly five minutes from first launch through title/return transition.

Dependencies: selected capability gates from P1–P7. Non-goals: combat, enemies, full field/world, multiplayer, full weather/fluids, advanced GI, marketplace, more languages.

Work packages and acceptance are detailed in [the vertical-slice plan](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md): route/content, movement/interaction, darkness/fog, wind/vegetation, minimal audio/acoustics, save/recovery, accessible settings, one-click export, performance/capture, platform evidence, and narrative/art review.

Completion requires a demo build, traversal test, visual/audio captures, calibrated performance report on M4 MacBook and named main PC, save/crash recovery, clean export/install, accessibility journey, rights/provenance audit, and explicit creative sign-off.

## 13. Phase 9 — Meridian UI editor migration and app runtime

Result: permanent Meridian UI owns docking, multi-window, command palette, inspectors, text editor, runtime game UI, and a standalone sample app.

Dependencies: P6 and P8 lessons. Work packages: docking/workspaces, multi-window, rich inspectors, graphs/canvas accessibility, source editor/LSP views, runtime theme/widgets, panel migration, bootstrap deletion plan.

Evidence: panel-by-panel parity, accessibility/performance/recovery, minimal runtime dependency proof, sample app, and no new egui-backed source data. Non-goal: effects at the cost of semantics.

## 14. Phase 10 — Audio mixer, spatialization, and adaptive music foundation

Result: real-time-safe device output, compiled DSP graph, streaming, spatial voices, buses, automation, and authored adaptive music.

Dependencies: P1, P5, P7, P8 audio lessons. Work packages: device backends, sample clock, graph compiler, callback, decoders/rings, voice management, spatial tiers, music state, editor/diagnostics.

Evidence: callback allocation/lock assertions, underrun stress, device switch/recovery, stream seek, sample-accurate transitions, loudness/caption integration. Advanced geometric acoustics remains P20.

## 15. Phase 11 — Weather fields, vegetation, and procedural forest authoring

Result: artists author reproducible regional weather/wind/fog and scalable vegetation with partial forest regeneration.

Dependencies: P5, P8, P10. Work packages: weather state graph, tiled wind fields, surface wetness seam, vegetation tiers, deterministic placement, overrides, partial rebuild, cross-system snapshots.

Evidence: seed/rebuild determinism, transition capture, vegetation wind/LOD benchmark, streaming behavior, artist override preservation. Non-goals: planetary weather, full fluids/fire/snow/erosion.

## 16. Phase 12 — Advanced rendering foundation

Result: measured prototypes choose scalable geometry, temporal, GI, ray, and quality architecture without destabilizing the opening baseline.

Dependencies: P2, P8, P11 corpus. Research work packages: visibility buffer versus Forward+, virtual geometry hierarchy/raster/page sizes, temporal AA/upscaling, dynamic GI portfolio, ray abstraction/hardware capabilities, path-traced reference.

Stable seams: scene extraction, material facets, render graph, capability/quality descriptors. Evidence: shared scenes/hardware, image metrics plus human review, frame/memory/build cost, losing prototype archive. No prototype becomes production by enthusiasm alone.

## 17. Phase 13 — Cairn structural destruction flagship

Result: authored connected structures fracture plausibly with bounded simulation, debris, save, replication-ready events, and editor inspection.

Dependencies: P3, P5, P8. Work packages: structural graph/bonds, stress/damage, fracture assets, solver islands, fragment promotion/demotion, persistence, debug/authoring.

Evidence: differential invariants, deterministic modes, large-structure benchmark, memory/debris budgets, save/reload, failure fallback to authored break states. Non-goal: every material fracture model.

## 18. Phase 14 — Procedural graph platform and candidate workflow

Result: reusable typed graph compiler supports domain-specific terrain/vegetation/material/building candidate documents with explainable placement and overrides.

Dependencies: P5, P11. Work packages: graph schema/type system, compiler/IR, deterministic RNG, content-addressed node cache, dirty-region propagation, candidate review, override stack, provenance/cost tooling.

Evidence: partial regeneration, manual override survival, graph migration, cycle/type diagnostics, stress benchmark. A universal gameplay/UI/audio graph is prohibited.

## 19. Phase 15 — Blender live link and native content tools foundation

Result: optional Blender integration and native tools exchange assets/materials/scenes with provenance, incremental updates, and reversible mappings.

Dependencies: P5, P9, P14. Work packages: interchange schema, Blender add-on/process bridge, change IDs, coordinate/material mapping, conflict UI, native mesh/material/animation tools.

Evidence: round-trip corpus, disconnect/reconnect, version mismatch, malicious file limits, no-Blender workflow. Blender remains optional.

## 20. Phase 16 — Cargo IDE, build service, and team workflows

Result: one editor-visible cancellable build DAG covers Cargo, shaders, assets, tests, packages, and signing while Cargo files remain authoritative.

Dependencies: P1, P5, P9. Work packages: lossless TOML model, cargo metadata/JSON ingestion, long-lived build service, immutable build IDs, artifact graph, rust-analyzer bridge, test/debug/run profiles, remote worker seam.

Evidence: manifest round-trip, incremental invalidation, cancellation/process restart, stale result rejection, reproducible build record, keyboard-accessible diagnostics. Non-goal: replacing Cargo/rust-analyzer.

## 21. Phase 17 — Meridian VCS and Jujutsu-derived operation model

Result: creators use changes/operations, semantic diffs, undo/restore, branches/bookmarks, and Git-compatible remotes without staging/HEAD jargon.

Dependencies: P5, P9, P16. Work packages: provenance/legal study, object/change/operation model, working view, semantic diff/merge, binary locks, Git import/export, editor/CLI, recovery.

Evidence: operation-log recovery, concurrent edit/conflict fixtures, Git interoperability, large assets, malicious repository limits. Jujutsu is research lineage, not copied branding/API.

## 22. Phase 18 — P2P sync, live collaboration, and partial workspaces

Result: meridian-sync provides encrypted direct peer exchange, optional self-hosted relay, partial workspaces, presence, and live sessions without mandatory account/cloud/inbound ports.

Dependencies: P17; P5 package/chunks. Work packages: identities/pairing, capability negotiation, NAT traversal, chunk exchange, optional relay, sparse materialization, presence/locks, live document sessions, VCS checkpoints.

Evidence: offline/direct/relay journeys, hostile peer tests, interrupted resume, bandwidth/storage benchmarks, session-to-operation checkpoint. Telepo does not exist separately.

## 23. Phase 19 — VR/OpenXR and interaction foundation

Result: one reference application runs stereo OpenXR with actions, comfort UI, spatial audio, and physical interactions.

Dependencies: P3, P9, P12. Work packages: lifecycle, swapchains, predicted timing, action/space mapping, multiview, late pose, grab/constraint/haptics, world UI/accessibility.

Evidence: runtime matrix, session/device-loss recovery, frame deadline captures, comfort journeys, disabled-pack zero cost.

## 24. Phase 20 — Hybrid acoustics and advanced audio authoring

Result: artists author scalable room/portal, probe, diffraction/occlusion, and optional wave/geometric acoustic tiers with inspectable fallbacks.

Dependencies: P10, P11, P14. Work packages: acoustic scene facets, bake/probe pipeline, runtime propagation, convolution budgets, authoring/visualization, platform fallback.

Evidence: reference impulse-response corpus, transition continuity, callback safety, bake determinism, CPU/memory tiers. Non-goal: universal physically exact acoustics.

## 25. Phase 21 — Deformables, advanced vegetation, fire, and thermal research

Result: bounded research demos determine which Cairn/environment extensions merit production.

Dependencies: P3, P11, P13. Gates compare XPBD/related deformables, vegetation coupling, fire propagation/visual models, and thermal fields on shared scenes.

Evidence: stability, visual usefulness, authoring cost, CPU/GPU/memory, determinism, save/network implications, fallback. Losing prototypes are archived; none blocks shipping.

## 26. Phase 22 — Multiplayer transport, server, and replication

Result: a reference networked sample supports dedicated/listen server, transport-neutral sessions, replication, prediction/reconciliation, interest, and impairment testing.

Dependencies: P5, P7, P16. Work packages: protocol/schema, transport interface, connection/auth seams, snapshots/deltas, ownership, prediction/rollback, interest/streaming, server packaging/ops.

Evidence: loss/latency/jitter/reorder matrix, soak, compatibility, malformed/flood tests, headless resource report, migration fixtures. Project Meridian remains single-player.

## 27. Phase 23 — Steamworks, EOS, and modded multiplayer

Result: optional provider adapters and mod-set negotiation integrate without changing core replication authority.

Dependencies: P22 and licensing/access. Work packages: Steam/EOS auth/lobbies/relay adapters, provider capability mapping, mod/package hash negotiation, server policy, trust/safety hooks.

Evidence: provider sandbox tests, outage/fallback, credential redaction, mod mismatch, license/redistribution review. No provider is mandatory.

## 28. Phase 24 — Modding SDK, restricted editor, and community library

Result: a game can publish stable mod APIs, capability-scoped packages, a restricted editor, and optional community distribution.

Dependencies: P7, P9, P17, P22. Work packages: manifest/capabilities, API compatibility, script/content/native policy, sandbox, dependency resolution, signing/trust, restricted editor, local/imported/community sources.

Evidence: denied-capability, malicious package, migration, dependency conflict, offline install, server policy, accessibility. No mandatory commercial marketplace.

## 29. Phase 25 — MCP, Codex, Ollama, and local agents

Result: tools and agents inspect and modify projects through typed transactions, preview, checkpoints, capabilities, and audit; Ollama local/cloud profiles are explicit.

Dependencies: P6, P16, P17. Work packages: command/query registry exposure, MCP server, resource views, approval/capability engine, checkpoint/rollback, provider adapters, local embedding index, evaluation suite, guarded YOLO project profile.

Evidence: parity with UI/CLI, prompt-injection and path/secret tests, denied destructive command, crash/rollback, offline Ollama, cloud/web permission separation. Agent absence changes no project behavior.

## 30. Phase 26 — Procedural buildings, interiors, materials, and ecosystems

Result: optional domain packs generate reviewable buildings/interiors/material variants/ecosystem candidates with stable overrides and provenance.

Dependencies: P14 and relevant P21 decisions. Evidence: domain corpora, constraint validity, traversal/accessibility, partial rebuild, performance/streaming, manual override preservation. Non-goal: one opaque generate-world button.

## 31. Phase 27 — Advanced fluids, flooding, erosion, snow, and coupling

Result: selected optional simulation tiers support bounded local fluids, shallow-water flooding, erosion, snow/granular behavior, and explicit field coupling.

Dependencies: P11, P20, P21 research. Work packages choose algorithms only after research gates; define grids/particles, coupling graph, clocks, conservation/stability diagnostics, authoring, save/network tiers, graceful fallbacks.

Evidence: canonical fixtures, stability ranges, deterministic modes, CPU/GPU/memory, activation/streaming, disabled-zero-cost, recovery. Not an engineering CFD claim.

## 32. Phase 28 — Additional languages and multi-language architecture

Result: after Luau/core stabilization, measured demand may add C#, Anorak, Python, or mixed-language modules through the same API schema and capability model.

Dependencies: P7, P16, P24 and stable compatibility policy. Each language is a separate research gate for runtime size, startup, debugging, hot reload, binding parity, sandbox, packaging, maintenance, and user demand.

No second runtime ships merely because a binding prototype works. Python is last by default. Cross-language object identity and exceptions never bypass Meridian handles/errors.

## 33. Phase 29 — Hardening, LTS, platform certification, and 1.0

Result: a supported 1.0 line has documented compatibility, recovery, security/update, accessibility, performance, platform, packaging, and operational evidence.

Dependencies: all capabilities selected for 1.0; unselected optional phases may remain future.

Work packages: API/format freeze, migration horizon, LTS/update policy, platform certification, supply-chain/signing drills, crash/data recovery, accessibility audit, performance corpus, installer/update/rollback, server ops, docs/examples, license/provenance, deprecation and support.

Completion requires independent audit sign-offs, release candidate soak, reproducible artifacts, key compromise rehearsal, update rollback, supported-old-project migration, and published known limitations. Ambition is not a gate; evidence is.

## 34. Research gate template

~~~text
ResearchGate:
  id:
  decision:
  owner:
  deadline_phase:
  stable_api:
  prototypes:
  corpus:
  hardware_platforms:
  metrics:
  acceptance_rule:
  security_accessibility_review:
  losing_prototype_archive:
  resulting_ADR:
~~~

## 35. Evidence manifest

~~~text
PhaseEvidence:
  phase:
  commit_or_checkpoint:
  demo_builds:
  test_reports:
  benchmark_reports:
  traces_and_captures:
  recovery_fixtures:
  accessibility_report:
  security_and_provenance_report:
  migrations:
  docs:
  known_limits:
  sign_offs:
~~~

A phase status is computed from this evidence and explicit waivers. PLANNING may summarize it but cannot override missing evidence.
