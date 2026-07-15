# Meridian Active Work Plan

version 0.5 · 2026-07-15

Status: MS-01 implementation passes locally, but milestone status remains
`Active`. `WP-REL-002` is `Partial` because required Linux and Windows headless
smokes are `NotRun`; this task forbids the commit/push needed to run remote CI.
All other named MS-01 implementation packages are closed with fresh evidence.
`WP-UI-001` follows only after qualification completes.

## 1. Authority and stop rule

The normative architecture is [the v0.5 master suite](specs/MERIDIAN_MASTER_SPEC.md); delivery order is [the evidence roadmap](specs/DELIVERY_ROADMAP.md); package readiness, completion, and concurrency are governed by [the implementation-planning specification](specs/IMPLEMENTATION_PLANNING_SPEC.md); typed status is under [`specs/registry/`](specs/registry/). This file owns only current evidence, the active bounded package, and the immediate queue.

`WP-GOV-004` is closed with `EV-GOV-20260715-004`. Do not interpret that
documentation package as implementation of Forward+, Meridian UI, Creator
Editor, Project Meridian content, native graphics backends, or any newly
specified modeler, gameplay, 2D, animation, navigation, framework, shader,
online, distributed-world, integrity, or simulation system.

## 2. Current implementation truth

| Domain | Implementation maturity | Honest boundary |
|---|---|---|
| Runtime/platform/tasks/diagnostics | `ImplementedFoundation` / `Partial` | MS-01 lifecycle epochs, surface outcomes, correlated tasks/events, fixed runtime, and native/headless smokes exist; full platform calibration remains |
| RHI/render graph | `ImplementedFoundation` | wgpu-backed lifetime, graph, and clear-frame foundations exist; stable native-ready RHI is not complete |
| Penumbra | `Partial` / `Transitional` | direct PBR, cascaded shadows, diffuse irradiance IBL, extraction/upload, typed pass timing, asynchronous visible capture, and structural/native GPU smokes exist |
| Assets/world/streaming/save | `ImplementedFoundation` / `Partial` | one bounded public source family imports transactionally, packages, worker-streams, activates, saves, recovers, and reconstructs; production schemas/compression/signing remain incomplete |
| Cairn | `Transitional` | current Rapier wrapper and grounded controller are evidence; Meridian-owned Cairn internals are not implemented |
| Meridian UI, Creator Editor, audio, Isobar, Basalt, vegetation | `Scaffold` unless a registry entry says narrower | the `meridian-editor` package now owns the MS-01 **Meridian** executable, not Creator Editor Alpha or Meridian UI |
| Alluvium | `Planned` | current v0.5 authority preserves the v0.4 adoption, requirements, packages, gates, and risks; no crate, evaluator, recipe parser, inspector, cook path, or corpus exists |
| Native modeler, Rust gameplay, Luau, animation, navigation, frameworks, first-class 2D, Meridian Shader Language, Collective | `Planned` or `Deferred` | specifications and registries exist; no product implementation claim |
| Torsant, networking, XR, modding, agents, VCS/sync | `Planned`, `Research`, or `Deferred` | no production implementation claim |
| Distributed worlds, advanced integrity, and other `PRG-*` programs | `Deferred` or `Research` | post-1.0 authority only; no milestone or implementation evidence |

The verified local workspace suite proves only its present test boundaries; it does not prove the Creator Editor,
game, Forward+, weather, terrain, simulation, or release roadmap.

## 3. Closed renderer foundation package

`WP-PEN-006` diffuse irradiance IBL is `ImplementedFoundation`:

- six-face irradiance cube upload exists;
- group-3 sun/shadow/environment binding is preserved;
- renderer construction smoke creates the pipeline and bind group;
- workspace tests and clippy passed when the package closed.

This is diffuse IBL only. `WP-PEN-009` prefiltered specular environment IBL and BRDF integration LUT remain planned future work.

## 4. Closed package — WP-GOV-001

User-visible result: Meridian's v0.3 documentation suite, Penumbra amendment,
evidence roadmap, governance registries, ADRs, schemas, validator, CI gate,
named subsystem split, and scaffold names agree and validate locally.

In scope:

- rewrite the suite authority and delivery model;
- migrate every historical roadmap and combined weather heading with zero unmapped entries;
- establish separate documentation, implementation, and evidence maturity;
- register stable requirements, packages, gates, evidence, workloads, risks, provenance, waivers, and ADRs;
- define Penumbra's adopted architecture without overstating implementation;
- rename only the empty weather/terrain scaffolds to Isobar/Basalt;
- update the private game documentation with the AMI/redacted-benchmark authority boundary;
- add `meridian-spec` and CI governance validation.

Explicit non-goals: renderer feature implementation, new runtime APIs, production UI/editor/game code, native backends, Torsant crate, benchmark execution/calibration, commits, and pushes.

Acceptance:

- `meridian-spec check` and all validator fixtures pass;
- old roadmap tokens appear only in migration/history records;
- deleted filenames have no active links;
- all coordinated domains have exactly one maturity record;
- all requirements trace to packages and evidence classes;
- `PEN-B01` through `PEN-B16` are valid and honestly `DefinitionOnly/Uncalibrated`;
- `cargo metadata --locked`, format, workspace tests, clippy, RHI clear-frame smoke, Penumbra structural smoke, diff check, and repository audits pass.

## 5. Closed documentation package — WP-GOV-002

User-visible result: a contributor can move from any milestone to an executable
package chain with readiness, dependency, integration, evidence, and stop rules
without treating a ten-year roadmap as a flat checklist.

Deliverables:

- normative Definition of Ready and Definition of Done;
- active, next, milestone-ready, and research/deferred planning horizons;
- a machine-validated `MS-00` through `MS-10` delivery-plan registry;
- acyclic `WP-*` dependency metadata and ordered critical paths;
- a complete near-term MS-01 decomposition;
- separate native-Metal, native-Vulkan/D3D12, Alpha-review, and Beta-review packages;
- a distinct `WP-PRJ-002` production opening-slice package after the prototype.

Non-goals: runtime implementation, dates, staffing estimates, speculative
file-level plans for distant work, commits, or pushes.

Acceptance passed: governance validation rejects missing milestone plans, broken
critical paths, cyclic package dependencies, and unknown package references;
all suite links and registries agree; proportional Rust gates pass. Evidence:
`EV-GOV-20260715-002`.

## 6. Closed documentation package — WP-GOV-003

User-visible result: the v0.4 suite adopts The Alluvium Engine as Meridian's
core procedural world-authoring and asset-generation architecture while keeping
implementation status `Planned`, runtime ownership explicit, and private game
content outside the engine repository.

Deliverables:

- one canonical Alluvium specification at the stable PRC path;
- `ADR-0017`, nine requirements, ten work packages, two research gates, and ten risks;
- typed recipe, field, evaluation, cache, identity, override, provenance, license,
  headless/editor, runtime-safe, and zero-cost-disabled contracts;
- MS-05 dependency on the minimum Alluvium foundation and sanitized environmental corpus;
- v0.4 registries, schemas, benchmark report contract, migration ledger, and private-game links;
- zero unmapped v0.3 procedural headings or amendment subjects.

Non-goals: Alluvium runtime/editor code, a marker crate, dependency adoption,
benchmark calibration, proprietary content transfer, commit, or push.

Closure evidence: `EV-GOV-20260715-003`. No `WP-PRC-*` package is active.
At that closure checkpoint, `WP-PEN-008` was the immediate runtime candidate;
it is now closed under MS-01 evidence below.

## 7. Closed documentation package — WP-GOV-004

User-visible result: Meridian v0.5 is a coherent general-purpose engine and one
integrated application specification without turning every ambition into a 1.0
blocker or claiming planned systems already exist.

Deliverables:

- adopted authorities for animation, navigation, official gameplay frameworks,
  first-class 2D, Meridian Shader Language, native modeling, Collective,
  distributed worlds, and integrity;
- Rust-first gameplay with optional Luau afterward, Wavefront/Collective voice
  boundaries, and a native beginner-friendly modeler required before the
  Project Meridian prototype;
- stable `PRG-*`, `VAL-*`, and `DEP-*` governance records for post-1.0 work,
  proving projects, and evidence-gated dependency strategy;
- `ADR-0018` through `ADR-0024`, 36 maturity records, updated roadmap and
  private-suite boundaries, and a 66-row zero-unmapped v0.5 migration ledger;
- validator failures for stale v0.4 headers, program-to-milestone leakage,
  missing validation projects, missing dependency strategies, and a missing
  v0.5 ledger.

Acceptance passed: all governance commands and 26 governance-tool tests,
private/public Markdown validation, metadata and format checks, 165 workspace
tests, warning-denied clippy, RHI and Penumbra native smokes, diff validation,
and repository boundary audits. Evidence: `EV-GOV-20260715-004`.

Explicit non-goals: runtime/editor feature implementation, a Meridian UI
redesign, benchmark calibration, hosted cloud infrastructure, new placeholder
crates, commits, or pushes. `WP-PEN-008` was next at this historical closure
checkpoint and is now closed under MS-01.

## 8. Closed package — WP-PEN-007

User-visible result: current high-level RHI passes expose pass labels, frame and
submission correlation, CPU encoding duration, and either trustworthy GPU
duration or a typed unavailable outcome without blocking the frame loop.

Implemented foundation:

- explicit timing-frame begin/end plus automatic single-pass frame IDs;
- frame ID, submission ID, pass label, slot generation, and CPU encode duration;
- bounded eight-slot asynchronous timestamp readback using submit callbacks and
  nonblocking polling;
- zeroed resolve buffers and rejection of zero, equal, reversed, stale,
  overflowing, failed-map, saturated, and device-lost results;
- explicit `UnsupportedCapability`, `UnsupportedPlatform`, or `Inconclusive`
  results where a trustworthy duration is unavailable;
- typed `FrameSample` GPU status while preserving `gpu_time: Option<Duration>`;
- clear, bootstrap, shadow-depth, and indexed-PBR pass instrumentation.

Closure evidence:

- conversion, invalid ordering, overflow, stale generation, saturation, map
  failure, device loss, result overflow, frame correlation, disabled-query, and
  unsupported-capability tests pass;
- ten Apple M4/macOS 27/Metal clear-frame runs returned explicit
  `UnsupportedPlatform(MetalTimestampDataInvalid)` with CPU timing, never `0ns`
  or a hard timestamp error;
- renderer smoke preserved six-face irradiance upload and bind groups 0-3, then
  returned correlated shadow-depth and indexed-PBR timing outcomes;
- governance, metadata, format, workspace tests, clippy, native smokes, no-wait
  search, and diff checks pass. Evidence: `EV-PEN-20260715-002`.

Non-goals: visible capture, clustered Forward+, specular IBL, benchmark
calibration, renderer quality claims, or native-backend implementation.

Known limit: current Apple Metal legacy timestamp data is not trusted after the
first invalid result, so GPU timing becomes `UnsupportedPlatform` for that RHI
lifetime while CPU timing continues. Future render-graph execution must reuse
this contract for every claimed production pass; this package does not claim
complete production-pass coverage.

## 9. MS-01 local implementation complete — qualification open

| Wave | Packages | Convergence rule |
|---|---|---|
| Instrumentation | `WP-PEN-007` closed | trustworthy CPU/GPU timing outcomes for current high-level passes |
| Surface and runtime | `WP-PEN-008`, `WP-RUN-002`, `WP-RUN-003` closed `ImplementedFoundation` | asynchronous visible/offscreen capture, typed surface outcomes, lifecycle epochs, device rebuild action, and correlated diagnostics |
| Source data | `WP-DAT-002` -> `WP-DAT-003` -> `WP-DAT-004` closed `ImplementedFoundation` | provisional source import, independent facets, package mount, worker load, bounded activation, save and recovery preserve authority |
| Integration | `WP-RUN-004` closed `ImplementedFoundation` | one `meridian` executable runs native, native-smoke, and headless-smoke paths |
| Qualification | `WP-REL-002` remains `Partial` | local evidence reviewed; required Linux/Windows headless rows remain `NotRun` until CI is authorized |

User-visible result: Meridian imports the public generic MS-01 fixture, writes
and reopens a provisional `.meridian` package, streams and activates one cell,
advances fixed simulation from semantic input, renders package-derived geometry,
writes a hashed RGBA8 PNG plus metadata, exercises save replacement/backup/
journal/migration recovery, and emits one correlated JSON evidence timeline.

Passing implementation evidence: `EV-PEN-20260715-003`, `EV-RUN-20260715-002` through
`EV-RUN-20260715-004`, `EV-DAT-20260715-002` through
`EV-DAT-20260715-004`. Qualification review `EV-REL-20260715-001` is
`Inconclusive` until Linux/Windows headless evidence runs.

Limits: the native surface was occluded or unavailable in the qualification run,
so the visible image is explicitly offscreen and makes no presentation or visual-
quality claim. JSON, compiled facets/cells, `.meridian` v1, and save transaction
encoding remain provisional. Compression, signing, patches, encryption,
Forward+, specular IBL, Meridian UI, Creator Editor workflows, and game content
remain outside MS-01.

`WP-REL-002` is the immediate closure candidate. `WP-UI-001` follows after
MS-01 qualification. Penumbra Stage 1 then proceeds through `WP-PEN-003` and
`WP-PEN-010`, with `WP-PEN-009` as bounded parallel image-quality work and
`WP-PEN-011` as the later executable/calibrated forest corpus. `RG-PEN-001`
cannot open before MS-05.

Alluvium begins only through a separately activated package. `WP-PRC-001`
through `WP-PRC-004` are required for MS-05, but their presence in the roadmap
does not reorder or activate the current `WP-REL-002` closure candidate.

## 10. Evidence policy

Every run records source checkpoint, BuildId when available, corpus/build hashes, hardware, OS, backend, driver, capability profile, settings, cache/warmup state, distributions rather than averages alone, memory, artifacts, and missing features. Occluded structural evidence cannot satisfy visual quality. Unavailable hardware or capabilities are `NotRun`, `UnsupportedPlatform`, or `UnsupportedCapability`, never Pass.

No benchmark report may generalize beyond its measured workload/profile. No uncalibrated number becomes a release gate.

## 11. Mandatory package sign-off

~~~text
Work package:
User-visible result:
Status:
Source checkpoint and BuildId:
Requirements:
Milestone contribution:
Entry conditions and dependencies:
Files/crates/formats changed:
Deliverables and public contracts:
Explicit non-goals:
Tests:
Benchmarks and hardware:
Captures/traces/recovery evidence:
Accessibility:
Security/provenance:
Migration/compatibility:
Documentation:
Integration checkpoint:
Stop/rollback rule:
Known limits and unsupported rows:
Reviewers/sign-offs:
Next unblocked package:
~~~

Do not close a package when a required row lacks evidence or a valid scoped, expiring waiver.
