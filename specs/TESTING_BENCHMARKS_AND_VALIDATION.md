# Testing, Benchmarks, and Validation

[Master](MERIDIAN_MASTER_SPEC.md) · [Roadmap](DELIVERY_ROADMAP.md) · [Registries](registry/) · [Examples](API_AND_FILE_FORMAT_EXAMPLES.md)

version 0.5 · 2026-07-15 · Normative evidence authority

Documentation maturity: `ImplementationReady`. Implementation maturity:
`Partial`. Governing IDs: `REQ-REL-001`, `REQ-GOV-002`, `WP-GOV-001`,
`WP-GOV-003`, `WP-GOV-004`, `WP-REL-001`, `WP-REL-002`.

## 1. Purpose and truth boundary

This document defines how Meridian proves claims. A test name, constructor,
crate, schema, benchmark definition, screenshot, or unchecked box cannot prove
a subsystem complete. The pre-v0.3 workspace baseline contained 121 passing
tests. The v0.4 closure passed 160 workspace tests, including 20 governance
tests and 10 pass-timing/diagnostics tests. The v0.5 documentation closure
passed 165 workspace tests, including 26 governance-tool tests. The MS-01 local
implementation checkpoint passed 191 workspace tests with focused capture, lifecycle, diagnostics,
source import, package, streaming, save recovery, and application integration
coverage. Overall qualification remains `Inconclusive` until required Linux and
Windows headless CI rows run. This count is evidence for that source checkpoint,
not a frozen completion percentage. Any passing count proves only its covered runtime/code
boundaries and does not convert a scaffold, planned domain, validation-project
definition, or post-1.0 program into implementation.

Evidence identity and status are typed under `specs/registry/`. Raw artifacts
remain outside summary records but are content-hashed and retained according to
their security/private-content policy.

## 2. Evidence status and claim levels

Evidence status is exactly `Pass`, `Fail`, `NotRun`, `UnsupportedCapability`,
`UnsupportedPlatform`, `Occluded`, `Redacted`, `Waived`, `Stale`, or
`Inconclusive`.

| Claim | Minimum evidence |
|---|---|
| Compiles | named target/profile/toolchain and compiler output |
| Constructed | API object-creation assertion |
| Structural smoke | commands/resources submitted with outcome and explicit limits |
| Visually validated | known visible surface plus reference/expected comparison and human review where needed |
| Functionally validated | end-to-end user outcome plus state assertions |
| Recovered | induced failure plus retained authority, rollback/rebuild, user message, and escape path |
| Deterministic | declared platform/mode/thread/compiler envelope plus replay/diff |
| Calibrated | named hardware/software/corpus/profile/statistical method and raw samples |
| Milestone complete | all required functional, performance, recovery, security, accessibility, compatibility, documentation, and review rows |

Occluded or minimized GPU evidence may prove six-face upload, pipeline,
bind-group, command submission, and surface handling. It cannot satisfy
`REQ-PEN-003` or any visual-quality requirement.

MS-01 satisfies its bounded `REQ-PEN-003` capture obligation with an explicitly
labeled offscreen capture when presentation is occluded or unavailable. That
artifact proves source-derived nonuniform pixels, dimensions, format, and hash;
it does not prove presentation or production image quality.

## 3. Evidence record contract

Every evidence record contains:

```text
EvidenceRecord {
  evidence_id,
  requirement_ids,
  work_package_ids,
  milestone,
  status,
  source_checkpoint,
  BuildId,
  workload_or_test_id,
  implementation_maturity,
  hardware,
  operating_system,
  backend_and_driver,
  capability_profile,
  toolchain_and_dependencies,
  settings_and_resolution,
  corpus_hash,
  build_hash,
  warmup_and_cache_state,
  repetitions_and_statistics,
  raw_artifact_hashes,
  summary,
  thresholds_and_result,
  known_limits,
  reviewer_and_timestamp,
}
```

Secrets, personal paths, and licensed/private payloads are redacted. Redaction
preserves stable private source/corpus hashes, authority, and access location
without copying content into the engine repository.

## 4. Test portfolio

- compile/static: format, clippy, type/API/schema, forbidden edges, unsafe,
  dependency/license/provenance, docs and registry validation;
- unit/property: algorithms, IDs, parsers, migrations, schedulers, command
  inverses, snapshot epochs, bounded failures;
- integration: crate/process/RHI/audio/physics/build boundaries;
- fixture/golden: schemas, migrations, UI semantics/layout, images, audio,
  protocols, benchmark recipes;
- differential: backend/path, Cairn/upstream, algorithm candidates, format
  versions, CPU/reference;
- fuzz/adversarial: untrusted projects/imports/shaders/packages/saves/scripts,
  network/VCS/build/provider/agent inputs;
- end-to-end: creator journeys, prototype/slice, build/package/install/launch,
  server/mod/agent profiles when activated;
- soak/recovery: cancellation, crash, device/surface/audio/provider loss,
  interrupted commits, long-run memory/resource stability;
- benchmark: calibrated distributions, memory, visual/reference, quality/cost,
  and maintenance evidence on controlled corpora.

Flaky quarantine requires owner, reproducible symptom, tracking ID, expiration,
blocked claim, and non-blocking rationale. It is not Pass.

## 5. Statistical and hardware policy

Reports declare warmup, cold/warm cache, deterministic inputs, sample duration,
repetitions, aggregation, outliers, confidence/variance, background load,
power/thermal state, display mode, and observer overhead. Frame reports use
distributions, percentiles/lows, and worst windows rather than average FPS
alone. CPU/GPU overlap is not double-counted as wall time.

Hardware records contain exact CPU/GPU/memory/storage/display/OS/driver/runtime
and capability profile. Priority is Apple Silicon; Linux/Steam Deck; Windows
NVIDIA; Windows AMD; Intel graphics; Windows on ARM. Unknown, emulated, and CI
hardware remain explicit uncalibrated/virtualized rows.

Missing timestamp, subgroup, mesh, ray, sparse, multiview, or other capability
is `UnsupportedCapability`, not zero cost or Pass. Numeric limits and regression
thresholds remain provisional until preregistered calibration.

## 6. Permanent Penumbra workload suite

All sixteen records are `DefinitionOnly` and `Uncalibrated` until an executable
recipe or explicitly non-Alluvium fixture, immutable corpus hash, camera/input recording, expected counts/state,
and evidence run exist.

| ID | Workload | Primary dimensions |
|---|---|---|
| PEN-B01 | Midnight forest | flashlight, night fog, foliage, streaming, shadows, pacing, temporal stability |
| PEN-B02 | Dense grass field | density, LOD, wind, horizon, overdraw, shadows, memory |
| PEN-B03 | Flashlight through alpha-tested foliage | alpha cost, local light/shadow, shimmer, ghosting |
| PEN-B04 | Redacted AMI interior with many local lights | generated connected rooms, mixed practical temperatures, shadowed lights, partial failures, materials, decals/transparency, indoor/outdoor streaming |
| PEN-B05 | Heavy Isobar storm | wind, rain, fog/volumetrics, wetness hooks, coupling, downgrade |
| PEN-B06 | Large Basalt terrain vista | precision, terrain LOD, geometry residency, atmosphere, memory |
| PEN-B07 | Torsant fire, fluids, heat, and smoke | optional fields/solvers, coupling, stability, rendering, disabled cost |
| PEN-B08 | Rapid camera rotation | visibility churn, temporal rejection, disocclusion, input-to-frame |
| PEN-B09 | High-speed traversal | prediction, streaming deadlines, uploads, LOD, pacing |
| PEN-B10 | Large world-streaming transition | IO/decode/upload/activation, room/cell transitions, residency, recovery |
| PEN-B11 | Low-memory stress | pressure, eviction, churn, staged fallback, required-data protection |
| PEN-B12 | Shader and pipeline compilation stress | permutations, compile/cache/warmup, runtime creation, diagnostics |
| PEN-B13 | Shadow-heavy scene | caster/update/cache policy, atlas/memory, local/directional cost |
| PEN-B14 | Transparency-heavy scene | blend/sort, alpha overdraw, particles/decals, lighting, tier policy |
| PEN-B15 | Temporal-disocclusion test | history validity, motion/exposure, ghosting, shimmer, debug views |
| PEN-B16 | VR-oriented stereo test | deferred until XR; stereo, multiview/foveation, predicted timing, late pose, memory, comfort |

PEN-B01/PEN-B02 preserve the original forest-workload intent. PEN-B04 is generated and may
record only a redacted private authority/hash. It contains no AMI logo,
document, narrative text, route data, or proprietary asset.

## 7. Penumbra report fields

Every workload report records hardware, operating system, backend, driver,
renderer path, capability profile, settings, resolution, upscaler, CPU/GPU/frame
distributions and lows, memory, shader/pipeline/upload/streaming stalls,
resource churn, overdraw, shadow/volumetric cost, temporal stability, device
bottlenecks, visual differences, artifacts, missing features, warmup/cache state,
source checkpoint, BuildId, corpus/build hashes, Alluvium recipe hashes/version,
determinism level, evaluation mode, provenance-manifest hash, and raw evidence.
Fields that do not apply use an explicit `NotApplicable` value; omission is not
allowed under `penumbra-benchmark-report-v0.5`.

Renderer-path comparisons additionally record feature parity, artist review,
custom-shader compatibility, debugging/tooling, maintenance/staffing, native
backend rows, preregistered material thresholds, and lower-tier regressions.

## 8. Domain validation

Rendering: shader/IR/reflection/binding/cache tests; render-graph hazards/order/
lifetime; resource/handle/device/surface recovery; upload formats/faces/mips/
pitch; pipeline warmup; pass timing; visible references for materials, shadows,
diffuse/specular IBL, fog, transparency, vegetation, and temporal behavior.

Physics/simulation: canonical body/shape/query/contact/controller; extreme/
degenerate/CCD/stability/fuzz; declared determinism; Cairn/upstream differential;
Isobar field continuity; Basalt precision/rebase; vegetation coupling; optional
Torsant stability/reference and zero-disabled-cost.

Alluvium: recipe canonicalization/migration; graph types/units/cycles; strict,
stable, and opportunistic determinism; named random substreams; scalar/SIMD/GPU
structural differential; exact dirty regions/halos; cache corruption; bounded
cancellation and memory; generated identity; override reconciliation/orphan
recovery; provenance/license propagation and cooker rejection; typed subsystem
handoffs; headless/editor parity; private-content redaction; baked-only
zero-runtime-cost profiles.

Wavefront: callback no general allocation/block/lock/IO/log; graph compile; sample
clock; stream seek/starvation; device loss/change; spatial/acoustic fixtures;
silence/NaN/clipping/loudness/underrun; semantic captions/cues; optional voice permission/capture/jitter/mute and zero-cost-disabled behavior.

Gameplay/frameworks: Rust API schema and reflection; module lifecycle; isolated Play rebuild/restart and rollback; save/headless/replay; typed event/command ordering; framework removal/forking; local-player contexts; dedicated-server and multi-project evidence. Optional Luau adds generated binding parity, sandbox, budgets, migration, mixed-project, and stripped-build tests without blocking Rust evidence.

Animation/navigation: skeleton/clip/graph/event identity, import/migration, retargeting, streaming, root motion, rollback and CPU/GPU deformation differential; navigation artifact build, profiles, streamed seams, bounded queries, dynamic obstacles, partial results, replay, and game-authority separation.

First-class 2D: atlas determinism/bleeding, pixel/DPI policy, stable layer order, tile migration/streaming, dedicated Cairn 2D contacts/queries/joints, mixed-view composition, 2D-only stripped builds, batching, overdraw, memory, and accessibility.

Shader language/modeler: text and graph semantic equivalence into one ShaderIr, reflection/source maps/capability rejection/target differential/security; editable model topology invariants, stable element lineage, undo/recovery, modifier cancellation, Alluvium override conflicts, interchange loss reports, beginner journeys, and player-build stripping.

Collective: offline stripping, local/self-hosted/provider adapters, auth/session state machines, outage/quota/idempotency, privacy/consent/export/deletion, voice-room permissions, block/mute/report, moderation/appeal, secrets, accessibility, and adversarial clients. Mocks cannot prove a production service, and no Meridian-hosted service is assumed.

Distributed worlds and integrity remain post-1.0 definition/research domains. When their programs activate, validation adds authority-epoch migration/split-brain/restore/cost and adversarial/false-positive/privacy/accessibility/mod-compatibility evidence without changing MS-00 through MS-10.

Data/formats: current/previous/oldest fixtures; unknown optional/required;
canonical roundtrip/diff; malformed/oversized/recursive/path/decompression;
truncation/duplicates/order; migration/rollback; non-destructive repair;
hash/signature/provenance.

UI/editor/accessibility: layout/render over DPI/fonts/locales; keyboard/controller/
touch/IME/focus; semantic actions; scaling/contrast; reduced motion/flash;
virtualization; undo/play isolation/crash recovery; offline Ponder links/examples.

Security: threat-model-derived project/import/save/package/shader/script/build/
VCS/sync/network/provider/mod/agent/update tests, permission/listener audits,
secret/private-content redaction, safe mode, key compromise, and rollback.

## 9. Validation projects

[`registry/validation-projects.json`](registry/validation-projects.json) defines the permanent cross-domain proving set:

| ID | Authority | Purpose |
|---|---|---|
| `VAL-PRJ-001` | private consumer | Project Meridian integration with sanitized engine reports |
| `VAL-FWK-001` | public generic | first-person interaction and shooter foundations |
| `VAL-FWK-002` | public generic | third-person movement, parkour, and camera |
| `VAL-TWO-001` | public generic | first-class 2D platformer/runtime fixture |
| `VAL-UI-001` | public generic | UI-heavy accessible application and editor fixture |
| `VAL-RUN-001` | public generic | headless simulation and dedicated server |
| `VAL-COL-001` | public generic | offline and self-hosted Collective behavior |

All seven begin `DefinitionOnly` / `Uncalibrated`. A project becomes executable only through a versioned corpus, source/build hash, exact profile, expected functional outcomes, calibrated metrics, failure/recovery cases, and evidence record. Project Meridian private content never becomes public corpus; only sanitized contracts, IDs, hashes, outcomes, and limits cross repositories.

These projects prevent one private game from being mistaken for general-purpose engine proof. No single project can prove all domains, and a framework-free/minimal build remains required to prove optionality.

## 10. Recovery matrix

Induce failure during source transactions, imports, artifact commits, shader/GPU
device/surface work, world activation, script/UI reload, audio device/stream,
save journal/snapshot, package mount/update, VCS/sync, network/provider, agent
proposal/command, and signing/update activation. Evidence states retained
authority, discarded/rebuilt state, user message, automatic recovery, and
manual escape path.

## 11. Governance validator

`meridian-spec` validates:

- duplicate/missing stable IDs, links, fences, retired authorities, and status vocabulary;
- every coordinated domain has exactly one maturity record;
- requirements map to pre-1.0 work packages or post-1.0 programs and evidence classes;
- schemas and typed registries;
- ADR existence/index coverage;
- expired/incomplete waivers;
- implemented promotions without passing evidence;
- occluded structural evidence used for visual claims;
- orphan requirements/references and zero-unmapped migration state;
- all 16 workloads, honest calibration, PEN-B04 redaction, reports, risks, and provenance.
- all Alluvium requirements/packages/gates/risks and historical v0.4 migration rows;
- v0.5 domains, ADRs, programs, validation projects, strategic dependencies, current-version headers, and zero-unmapped migration rows;
- post-1.0 programs cannot satisfy or block MS milestones or promote implementation maturity;
- all benchmark report recipe/provenance fields use the v0.5 report contract without promoting planned implementation.

False implementation claims, private leakage, missing IDs, and invalid schemas
are unwaivable.

## 12. Dependency and budget policy

[`registry/dependency-strategy.json`](registry/dependency-strategy.json) records purpose, license, current use, boundary, replacement difficulty, strategic/performance importance, maintenance risk, compatibility, tests, category, alternative, and status for significant foundations. `InternalizeEventually` is not a promise or schedule; wrapping indefinitely is allowed when it remains the best product and maintenance decision.

Performance, memory, latency, build, storage, bandwidth, service-cost, accessibility, and quality budgets are workload/profile contracts. Before calibration they are provisional ranges or required metrics, not fabricated fixed gates. Every calibrated threshold records corpus, target hardware/profile, statistical method, user impact, owner, review date, and downgrade/failure policy. A high-end win cannot hide a lower-tier regression.

## 13. CI and local gates

Governance validation runs before the Rust matrix. Tool tests and fixtures run
on macOS, Linux, and Windows through the workspace matrix.

Local MS-00 gates:

```text
cargo run -p meridian-spec -- check
cargo run -p meridian-spec -- list-unmapped
cargo metadata --locked
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p meridian-rhi --example clear_frame
cargo run -p meridian-renderer --example instance_upload_smoke
git diff --check
link, secret, personal-path, private-content, tracked-game, oversized-file audits
```

Remote Actions verification is deferred until a later explicitly authorized
commit/push. Missing remote or hardware evidence is `NotRun`.

## 14. Waiver and completion

A waiver requires `WVR-*` identity, subsystem owner, validation/release
approver, expiration, blocked milestone, remediation package, reason, and risk.
It cannot promote maturity or hide an unsupported row. Expired waivers fail.
False implementation claims, private-content leakage, missing stable IDs, and
invalid governance schemas are unwaivable.

A package completes only when required evidence is fresh for its checkpoint and
known limits are compatible with scope. Milestone review includes every domain,
profile, recovery, accessibility, security, provenance, migration, and
documentation row or an allowed expiring waiver.

## 15. Example uncalibrated report

```text
evidence: EV-PEN-YYYYMMDD-NNN
workload: PEN-B01
status: NotRun
reason: executable generated corpus not yet implemented
surface: no visual claim
current structural evidence: six-face irradiance upload and pipeline/bind-group construction
known limits: complete render-graph-executor timing coverage, presented-surface capture, Forward+, specular IBL, and calibrated corpus remain open; MS-01 offscreen capture exists separately
decision: MS-05 and MS-07 remain open
```
