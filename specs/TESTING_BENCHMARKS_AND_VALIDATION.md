# Testing, Benchmarks, and Validation

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Phases](IMPLEMENTATION_PHASES.md) · [Examples](API_AND_FILE_FORMAT_EXAMPLES.md)

Version 0.2 · 2026-07-14 · Normative

## 1. Purpose and status

This document defines how Meridian proves claims. The repository has meaningful Rust unit/integration/workspace tests and native structural smoke coverage. The existing B01/B02 documents are benchmark definitions, not yet an executable calibrated corpus. Legacy numeric budgets are provisional until recorded through this process.

No test name, passing constructor, scaffold crate, screenshot, or unchecked box alone proves a subsystem complete.

## 2. Evidence vocabulary

| Claim | Required evidence |
|---|---|
| Compiles | named target/profile and compiler output |
| Constructed | API object creation test |
| Structural GPU/audio/physics smoke | commands/resources ran without validation error, with outcome limits |
| Visually validated | capture with known visible surface and expected/reference comparison |
| Functionally validated | end-to-end user outcome plus state assertions |
| Recovered | induced failure and demonstrated retained authority/recovery |
| Deterministic | declared mode/platform envelope and replay/diff evidence |
| Calibrated | named hardware/software/corpus/statistical method |
| Production-ready | phase-required functional, performance, recovery, security, accessibility, compatibility, documentation, and operational evidence |

An occluded native surface may prove six-face upload, pipeline, and bind-group construction. It does not prove visible irradiance quality.

## 3. Test pyramid

- Compile/static: formatting, lint, type/API/schema, forbidden dependencies, unsafe/dependency/license policy.
- Unit/property: algorithms, IDs, parsers, migrations, schedulers, command inverses.
- Integration: crate boundaries, process protocols, GPU/audio/physics adapters, build graph.
- Golden/fixture: schemas, migrations, UI layout/semantics, images, audio/impulse data, protocol.
- Differential: Cairn/Rapier baseline, algorithm prototypes, format old/new, CPU/reference.
- Fuzz/adversarial: untrusted formats/protocols, scripts, packages, repositories, tools.
- End-to-end: editor journeys, opening traversal, export/install, server/client, mod/agent.
- Soak/recovery: long run, cancellation, crash, power-loss simulation, device/provider loss.
- Benchmark: calibrated latency/throughput/memory/quality/cost under controlled corpus.

Tests are deterministic by default where practical. Flaky quarantine requires owner, symptom, issue, expiry, and non-blocking rationale.

## 4. Evidence artifact schema

~~~text
EvidenceRecord {
  evidence_id,
  requirement_ids,
  phase_and_work_package,
  source_checkpoint,
  BuildId,
  test_or_workload_id,
  implementation_status,
  hardware,
  os_driver_runtime,
  toolchain_and_dependencies,
  project_profile_and_capabilities,
  corpus_hash,
  warmup_and_cache_state,
  repetitions_and_statistics,
  raw_artifact_hashes,
  summary,
  thresholds_and_status,
  known_limits,
  reviewer_and_timestamp
}
~~~

Raw traces/captures/reports remain available. Human summaries link to them. Secrets and private licensed source are redacted or stored under controlled access without breaking reproducibility metadata.

## 5. Hardware and platform matrix

Tiers are named records, not vague low/medium/high:

- H-MAC-PRIMARY: exact M4 MacBook model, CPU/GPU cores, memory, display, macOS, driver/runtime.
- H-PC-PRIMARY: user’s exact main PC CPU/GPU/memory/storage/display/Windows or Linux version/driver.
- H-LINUX-CI: exact headless or GPU runner when available.
- H-WINDOWS-CI: exact runner when available.
- H-SERVER: exact headless server tier for later network phases.
- H-XR: headset/runtime/GPU for Phase 19.

Unknown hardware remains uncalibrated. Emulated/virtualized results are labeled.

## 6. Statistical policy

Each benchmark declares warmup, cold/warm cache, deterministic input, duration or sample count, repetitions, aggregation, outlier policy, confidence/variance, background-load policy, and thermal/power state.

Frame-time reports include distributions and worst-window behavior, not only average FPS. CPU/GPU overlap is not double-counted as wall time. Missing timestamp capability is an explicit unsupported outcome with CPU/queue fallback evidence.

Regression thresholds have both absolute and relative rules after calibration. A change can intentionally update a threshold only with evidence, rationale, affected tiers, and review.

## 7. Core workloads

### B00 Minimal runtime

Open minimal window/headless loop, fixed-step, task/diagnostic baseline, idle memory/tasks/threads/listeners, lifecycle/restart. Proves zero-cost and platform foundations.

### B01 Opening traversal

Deterministic recorded route through the five-minute opening with scripted camera/input, cold and warm runs, required checkpoints, capture markers, asset/cell expectations, and audio/weather events.

Metrics:

- frame/presentation/input latency and hitch windows;
- pass CPU/GPU, queue submit, surface outcome;
- memory categories, allocation and upload;
- IO/decode/stream/activate stages and deadlines;
- visible cells/objects/draws/triangles/materials/lights/vegetation;
- Cairn step/query/controller;
- audio callback, streams, voices, underruns;
- UI/script/save operation;
- pipeline/shader creation after warmup.

### B02 Forest stress

Repeatable dense forest/grass/fog/light/streaming camera paths designed to exceed normal opening complexity in controlled dimensions. Separate variants isolate draw/overdraw, shadows, visibility, uploads, world activation, and memory.

### B03 Build/export

Clean and incremental Cargo/shader/asset/logic/UI/package/sign builds, cancellation, worker restart, cache hit/miss, critical path, memory/IO, artifact reproducibility.

### B04 Save/package recovery

Large save journal/snapshot, transaction interruption, truncation/corruption/migration; package mount/range/patch/verify/rollback under cold/warm storage.

### B05 UI/editor

Large hierarchy/asset browser/inspector/graph, DPI/text/locale, virtualization, semantics, undo, play fork, rebuild events.

Later B10+ corpora cover audio/acoustics, Cairn destruction, VCS/sync, XR, server/network, mods/agents, and coupled simulation.

## 8. Rendering validation

- shader parse/validate/reflection/variant tests;
- render graph hazard/order/lifetime tests;
- upload size/format/face/mip/row-pitch tests;
- resource/handle/device-loss/surface outcome;
- pipeline warmup and no runtime creation where prohibited;
- pass-level CPU/GPU timings;
- visible reference/capture scenes for material, normals, shadows, diffuse IBL, fog, transparency, vegetation, temporal effects;
- image comparison uses masks, exposure/color-space metadata, metric tolerance, and human review for perceptual cases.

Diffuse IBL acceptance includes six validated faces, correct group-3 bindings, structural native smoke, and visible quality capture before visual claims. Specular IBL has separate prefilter/LUT/reference tests.

## 9. Physics and simulation validation

- canonical body/shape/query/contact/joint/controller fixtures;
- scale/extreme/degenerate/CCD/stability/fuzz;
- fixed-step and declared determinism modes;
- Rapier differential baseline during Cairn migration;
- structural graph/fracture/debris/save/debug;
- weather/wind field continuity/forcing/deterministic regeneration;
- optional fluid/fire/snow/acoustic algorithms against reference fixtures and stability envelopes;
- disabled pack has no tasks/resources/chunks.

## 10. Audio validation

- callback has no general allocation, blocking, lock wait, log, IO, or worker wait;
- graph compile/cycle/channel/sample-rate tests;
- sample-clock automation/music transition;
- decoder/stream seek/starvation/recovery;
- device add/remove/default change;
- spatial attenuation/pan/HRTF tier and acoustic transition fixtures;
- output capture, silence/NaN/clipping/loudness, underrun/overrun;
- accessibility captions/cues map to semantic events.

## 11. Data and format validation

Every persisted format supplies:

- current, previous, oldest-supported fixtures;
- unknown optional and unknown required fields;
- canonical round-trip and semantic diff;
- malformed, oversized, recursive, path traversal, decompression bomb;
- truncated/partial/duplicate/out-of-order;
- migration success/failure/rollback;
- recovery/repair that never overwrites sole original;
- hash/signature/provenance where applicable.

Fuzzers preserve minimized regression cases.

## 12. UI, accessibility, and documentation validation

- layout/render golden corpus over DPI/viewport/fonts/locales;
- keyboard/controller/touch/IME/focus/drag/drop;
- semantic tree/actions, screen-reader adapter, text scaling, contrast;
- reduced motion/flash/analog effects and captions;
- virtualized collection bounds and performance;
- Ponder offline search, version matching, broken links/examples/commands;
- every user-facing diagnostic has action or explanation and stable code.

## 13. Security validation

Threat-model-derived tests cover package/project/save/import, build workers, VCS/sync, network/server, providers/SDKs, mods/plugins, MCP/agents, update/signing/key recovery, secret redaction, permissions/listeners, and safe mode.

Security tests do not imply absence of vulnerabilities. Shipping high-risk features require review and documented residual risk.

## 14. Recovery matrix

Induce failure during every state-changing stage:

- source transaction, import/build artifact commit;
- shader/GPU device/surface;
- world load/activate;
- script/UI hot reload;
- audio device/stream;
- save journal/snapshot;
- package download/mount/update;
- VCS operation/ref update;
- sync/live session;
- server/provider;
- agent proposal/command;
- signing/update activation.

Evidence states what remained authoritative, what was discarded/rebuilt, user message, automated recovery, and manual escape hatch.

## 15. CI lanes

- Fast: formatting, lint, unit, schemas, docs links/examples.
- Workspace: all-target tests and feature profiles.
- Platform: native lifecycle/backend/audio/accessibility where available.
- GPU: shader/render graph/headless/native visible captures.
- Fuzz/security: scheduled and before format freeze.
- Corpus: B01/B02 and calibrated regression tiers.
- Recovery/soak: scheduled long-running.
- Release: clean/reproducible build, SBOM/license/provenance, package/sign/update/rollback, evidence manifest.

Missing runner capability is reported as not run, never pass.

## 16. Completion and waiver

A work package completes when required evidence is fresh for the checkpoint and known limits are compatible with phase scope. A waiver names requirement, reason, risk, mitigation, owner, expiry, and downstream blocked claims. Waivers cannot silently turn untested into supported.

## 17. Current immediate validation sequence

For the active diffuse-IBL renderer slice:

1. environment-light validation tests;
2. cargo test --workspace;
3. native renderer smoke requiring six-face irradiance upload plus pipeline/bind-group creation, with surface outcome recorded;
4. cargo clippy --workspace --all-targets -- -D warnings;
5. cargo fmt --all -- --check;
6. git diff --check plus untracked-file whitespace/link audit.

After closure, pass-level CPU/GPU timing and visible capture are the next work package. No broader renderer feature should hide that evidence gap.

## 18. Example report

~~~text
Evidence B01-H-MAC-PRIMARY-0042
checkpoint: <source id>
BuildId: <hash>
hardware: M4 model/config recorded
profile: opening-forest/quality-balanced
corpus: B01 hash
surface: visible
samples: declared warmup and runs
result: pass timing calibrated; one activation hitch over provisional gate
artifacts: trace, render captures, memory, audio, save log
known limit: specular IBL not implemented
decision: Phase 8 rendering gate remains open
~~~
