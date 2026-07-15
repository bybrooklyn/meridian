# Principles and Scope

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Phases](DELIVERY_ROADMAP.md)

version 0.5 · 2026-07-15 · Normative

Documentation maturity: `ImplementationReady`. Governing IDs:
`REQ-CORE-001` through `REQ-CORE-004`, `REQ-GOV-001` through `REQ-GOV-003`.

## 1. Product definition

Meridian is a local-first, data-oriented engine and one user-facing creator application for games and interactive applications. It combines runtime, editor, IDE, native modeler, The Alluvium Engine procedural authoring, asset pipeline, build service, documentation, version control, collaboration, optional agents, packaging, and deployment behind coherent typed contracts. The application is named **Meridian**; helper processes and CLI tools do not create separate Studio or IDE products. Project Meridian is the first proving game and supplies concrete quality gates without becoming an engine dependency.

The target is not feature-count imitation. Meridian differentiates through inspectable data, recovery, progressive disclosure, one semantic command model, zero-cost optional packs, provenance-first ownership, and vertical slices that prove systems together.

## 2. Users

- Beginner creator: opens a project, imports assets, edits a world, presses Play, fixes plain-language diagnostics, and exports without a terminal.
- Technical designer: edits typed logic, Rust gameplay data/APIs, optional Luau, materials/shaders, models, animation, UI, Alluvium recipes/fields, and profiles with live validation.
- Engineer: uses Rust APIs, Cargo, schemas, traces, CLI, test harnesses, and backend diagnostics.
- Team lead: reviews semantic changes, dependencies, budgets, provenance, permissions, and package readiness.
- Player or operator: receives accessible, signed, recoverable builds with explicit telemetry/network choices.
- Tool or agent: uses the same typed commands as a human surface under stricter capabilities and auditable transactions.

## 3. Required experience

A new project MUST:

1. open without a cloud login;
2. show a runnable default world or selected template;
3. explain missing SDKs or optional features without breaking unrelated workflows;
4. keep source data readable and recoverable after a crash;
5. expose an Advanced view without forcing it on normal users;
6. preserve project behavior when AI features are absent;
7. make expensive systems opt-in and show their shipping/runtime cost before activation.

## 4. Scope

Foundation scope:

- platform/window/input, clocks, tasks, memory, diagnostics;
- data-oriented world/ECS contracts and extraction;
- wgpu-first rendering with explicit capability and fallback;
- assets, import/build graph, streaming, saves, packages;
- Alluvium textual recipes, typed fields, deterministic evaluation, generated
  identity, non-destructive overrides, provenance, and cooking;
- Meridian UI shared by editor, runtime UI, and native tools;
- editor shell, complete Rust-first IDE workflow, build service, Cargo/rust-analyzer integration;
- native editable-mesh modeler baseline, with Blender as an optional companion;
- Rust gameplay modules first; Luau as the first optional embedded high-level runtime afterward;
- first-class 2D path, general animation, navigation infrastructure, and selected reusable framework foundations through their registered packages;
- Project Meridian opening forest vertical slice.

Long-horizon scope, implemented only through later milestones and work packages:

- Cairn physics ownership and simulation portfolio;
- advanced weather, acoustics, fluids, fire, fracture, terrain, and vegetation;
- OpenXR;
- multiplayer and dedicated servers;
- optional Collective identity/session/social/voice-policy/analytics/moderation modules with self-hostable/provider-neutral contracts;
- semantic VCS, P2P sync, live collaboration;
- modding and community library;
- local/cloud agent integrations under capabilities;
- native backend experiments and advanced GI/geometry.

Post-1.0 programs, not 1.0 milestone requirements: advanced sculpting/retopology/hair/cloth/character modeling; facial/performance-capture production; completion of all six official framework families; funded hosted-scale Collective work; distributed worlds/MMO; advanced integrity/anti-cheat; native VCS storage/history; and optional shader-compiler internalization.

## 5. Non-goals

- shipping a thin wrapper whose public API is owned by Bevy, Rapier, egui, wgpu, Cargo, Jujutsu, Ollama, or a platform SDK;
- requiring a hosted Meridian control plane;
- implementing every optional system before a playable game;
- one universal graph, one giant world file, one opaque package stream, or one global mutable service locator;
- requiring proprietary software or an opaque AI binary for first-party
  procedural authoring;
- supporting multiple scripting runtimes in the first playable product;
- promising Meridian-operated cloud infrastructure, social services, voice service, analytics, moderation, or MMO hosting without funded operational evidence;
- preserving compatibility with a third-party implementation API during an ownership migration;
- claiming deterministic equivalence, security, accessibility, performance, or competitor superiority without evidence;
- treating AI output as trusted or allowing agents to bypass user-visible commands.

## 6. Progressive disclosure

Each major workflow has three views:

| Level | User sees | System guarantees |
|---|---|---|
| Guided | intent, preview, safe defaults, actionable errors | reversible commands and no hidden external requirement |
| Advanced | dependencies, budgets, variants, capabilities | complete validation and explicit tradeoffs |
| Expert | source schemas, CLI, Rust API, trace IDs | lossless round-trip and same semantic operation |

The guided UI MAY omit fields but MUST NOT invent a separate storage model. Exported source and CLI output remain the audit path.

## 7. Optionality contract

Every optional feature pack declares:

~~~text
FeaturePack {
  id, version, crates, package_chunks, capabilities,
  permissions, platform_support, fallbacks,
  startup_cost, recurring_cost, shipping_cost
}
~~~

Disabled means:

- no worker thread or callback registration;
- no recurring scheduler task;
- no GPU heap, pipeline, descriptor, or shader variant;
- no network socket or discovery;
- no package chunk unless content directly depends on it;
- no editor panel unless the pack is installed;
- no placeholder save/network component.

CI verifies a minimal build by examining dependencies, symbols, package chunks, startup traces, steady-state task counts, and network listeners.

## 8. Local-first and external tools

Blender, Git hosting, Steam, EOS, Ollama, cloud models, external profilers, and proprietary SDKs are integrations, never prerequisites for core authoring. Meridian's native modeler and Alluvium provide first-party modeling and procedural authoring through editor, CLI, and headless workflows. Cargo remains authoritative for Rust projects, but Meridian provides safe IDE, build, and manifest workflows. External interchange reports preserved, approximated, and omitted semantics; lossless round-trip is claimed only where tested.

## 9. Truthfulness and evidence

Status language:

- Constructed means an object or pipeline was created.
- Structural smoke means contracts were exercised without visual correctness proof.
- Visually validated means expected pixels were captured and compared.
- Calibrated means thresholds were measured on named hardware and corpus.
- Production-ready means recovery, accessibility, security, performance, compatibility, and operational gates passed.

An occluded native surface can prove upload and bind-group construction, but cannot prove image quality. A benchmark definition without an executable corpus is a placeholder, not a result.

## 10. Safety and trust

- Local project content is untrusted input.
- Imported assets, packages, mods, shaders, scripts, network messages, build output, agent output, and update metadata cross trust boundaries.
- Destructive commands require preview, transaction boundaries, checkpoints, and explicit authorization.
- Secret material never enters project documents, logs, traces, agent prompts, or packages.
- Offline use remains valid after first installation except for explicitly online features.

## 11. Accessibility and documentation

Semantics originate in Meridian UI and gameplay intent. Platform adapters such as AccessKit expose that semantic tree but do not own it. Accessibility is tested with keyboard, controller, screen reader, reduced motion, contrast, text scaling, captions, remapping, and failure recovery. Ponder supplies contextual documentation and learning without transmitting project content unless the user explicitly enables a network provider.

## 12. Performance philosophy

Budgets are hierarchical: frame, subsystem, pass/job, asset/cell, and operation. The engine measures CPU, GPU, memory, IO, audio, build, network, latency, and recovery. Quality tiers choose algorithms and content, not merely constants. A fallback is a supported behavior with tests and diagnostics.

Any numeric target copied from version 0.1 remains provisional until [validation](TESTING_BENCHMARKS_AND_VALIDATION.md) records hardware, OS, build, scene/corpus, sample count, statistical rule, and accepted variance.

## 13. Platform policy

Platform order is macOS Apple Silicon, Linux, then Windows. The order controls evidence and staffing, not portable API design. Platform contracts cover windows, events, file dialogs, paths, process launching, clocks, accessibility, graphics surfaces, audio devices, input, networking, and crash handling.

Unsupported capabilities return typed availability and fallback information. They do not panic, silently degrade, or contaminate portable project data.

## 14. Project Meridian creative boundary

The following remain authoritative in the separate private Project Meridian creative suite:

- ambient non-combat structure with no enemies, player death, or forced dialogue;
- no sprint or jump in the opening;
- optional documents with no objective checklist;
- midnight forest to field progression and restrained analog/VHS treatment;
- the opening slice remains final-game content.

Engine specs may define how these are represented and tested, not rewrite their creative intent.

## 15. Review checklist

A proposal is in scope only if it names the user value, owning crate, data
authority, runtime cost, disabled cost, failure recovery, diagnostics,
compatibility, security permissions, accessibility impact, test evidence,
milestone/work-package dependencies, and stop condition. Missing answers make
the proposal research, not implementation-ready.
