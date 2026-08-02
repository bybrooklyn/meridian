# Meridian Active Work Plan

version 0.5 · 2026-07-17

Status: `MS-01` and `WP-REL-002` passed qualification on GitHub Actions run
`29452928922` for source checkpoint `010db80`: governance plus Linux, Windows,
and macOS workspace/headless-smoke rows passed. All named MS-01 implementation
packages are closed with fresh evidence. `WP-UI-001` is an
`ImplementedFoundation` and MS-02 is `Pass` from cross-platform CI evidence.
`WP-BLD-001`, `WP-PRC-001`, and `WP-EDT-001` are
`ImplementedFoundation` after cross-platform GitHub Actions evidence. Run
`29605881704` for `4463bad` qualified the persistent Creator behavioral
foundation across Linux, Windows, and macOS. `WP-UI-002` through `WP-UI-005`
are `Partial` after locally validated source packages; GitHub Actions runs
`29611418454`, `29621896632`, and `29622972884` could not allocate runners
because of the account billing state. Each workflow concluded `failure` before
implementation steps ran; the skipped platform rows are `NotRun`, not failed
implementation evidence. `ADR-0029` decided `RG-UI-001` for the Penumbra-owned
direct display-list path without claiming renderer completion. `WVR-UI-001` and
`WVR-EDT-001` permit non-promoting local continuation. `WP-UI-005` reached its
bounded local source stop point but remains unqualified. `WP-UI-006` is the
sole active source-only package: it makes `UiDocument` the versioned,
ergonomic authored source and compiles it into the existing frame contract.
`WP-EDT-002` is suspended at its recorded `Partial` state and resumes after
that work; `WP-EDT-003` then composes the remaining current Creator workspaces.
The framework
has local profile-bound exact offscreen golden
capture, controlled device-destruction replay, and uncalibrated raw performance
output. That output remains `Inconclusive` qualification evidence for current
local/dirty source because provenance is caller-declared; this preserves
unavailable timing/memory states and does not establish visual review, real
screen-reader, accessibility, or cross-platform qualification.
Fresh Linux, Windows, and macOS evidence is still `NotRun` because hosted
runners cannot be allocated. The latest retry, GitHub Actions run
`29952062179` for `ae97f82`, again failed governance with zero executed steps
and skipped the Rust matrix; it provides no implementation evidence. `WP-UI-006`
source work does not establish UI
qualification or maturity promotion; no later Creator package may promote from
this local work.
`WP-MDL-001` remains `Partial`; MS-03 remains open.

## 1. Authority and stop rule

The normative architecture is [the v0.5 master suite](specs/MERIDIAN_MASTER_SPEC.md); delivery order is [the evidence roadmap](specs/DELIVERY_ROADMAP.md); package readiness, completion, and concurrency are governed by [the implementation-planning specification](specs/IMPLEMENTATION_PLANNING_SPEC.md); typed status is under [`specs/registry/`](specs/registry/). This file owns only current evidence, the active bounded package, and the immediate queue.

`WP-GOV-006` is closed with `EV-GOV-20260716-001`. Do not interpret that
documentation package as implementation of Forward+, Meridian UI, Creator
Editor, Project Meridian content, native graphics backends, or any newly
specified modeler, gameplay, 2D, animation, navigation, framework, shader,
online, distributed-world, integrity, simulation, Marquee, shared environmental
media, cost-prediction, or competitive-validation system.

## 2. Current implementation truth

| Domain | Implementation maturity | Honest boundary |
|---|---|---|
| Runtime/platform/tasks/diagnostics | `ImplementedFoundation` / `Partial` | MS-01 lifecycle epochs, surface outcomes, correlated tasks/events, fixed runtime, and native/headless smokes exist; full platform calibration remains |
| RHI/render graph | `ImplementedFoundation` | wgpu-backed lifetime, graph, and clear-frame foundations exist; stable native-ready RHI is not complete |
| Penumbra | `Partial` / `Transitional` | direct PBR, cascaded shadows, diffuse irradiance IBL, extraction/upload, typed pass timing, asynchronous visible capture, and structural/native GPU smokes exist |
| Assets/world/streaming/save | `ImplementedFoundation` / `Partial` | one bounded public source family imports transactionally, packages, worker-streams, activates, saves, recovers, and reconstructs; production schemas/compression/signing remain incomplete |
| Cairn | `Transitional` | current Rapier wrapper and grounded controller are evidence; Meridian-owned Cairn internals are not implemented |
| Meridian UI | `ImplementedFoundation` / `Partial` / `Active` | the MS-02 core proof is qualified; `WP-UI-002` through `WP-UI-005` are locally implemented but lack cross-platform qualification, and active `WP-UI-006` adds only local source-authoring ergonomics; profile-bound offscreen golden correctness, controlled replay, and raw uncalibrated timing remain `Inconclusive`; presented visual, real screen-reader, accessibility, and cross-platform evidence remain open |
| Creator Editor | `ImplementedFoundation` | persistent hub, source-authoritative project documents, typed actions, Creator journey, and universal app structure passed run `29605881704`; this is a behavioral foundation, not Meridian UI 1.0 visual completion |
| Audio, Isobar, Basalt, vegetation | `Scaffold` unless a registry entry says narrower | implementation remains outside the current Creator Editor package |
| Alluvium | `ImplementedFoundation` | canonical text recipes, strict scalar evaluation, derived-cache recovery, CLI, and a basic inspector are qualified; production/domain work remains open |
| Native modeler | `Partial` | bounded editable-model source, stable topology lineage, semantic recovery, and a derived preview exist; broad modeling remains later scope |
| Rust gameplay, Luau, Artus body motion, navigation, frameworks, first-class 2D, Meridian Shader Language, Collective | `Planned` or `Deferred` | specifications and registries exist; no product implementation claim |
| Torsant, networking, XR, modding, agents, VCS/sync | `Planned`, `Research`, or `Deferred` | no production implementation claim |
| Distributed worlds, advanced integrity, and other `PRG-*` programs | `Deferred` or `Research` | post-1.0 authority only; no milestone or implementation evidence |
| Marquee | `Deferred` | ResearchReady post-1.0 architecture; no crate, active package, service integration, or promotional-quality evidence |
| Competitive performance and quality leadership | `Deferred` | ResearchReady post-1.0 comparison, environmental-convergence, and claim architecture; no calibrated corpus, optimization, comparator integration, or superiority evidence |

The verified workspace and CI suites prove only their present test boundaries;
they do not prove Meridian UI 1.0 visual quality, platform screen-reader
qualification, the game, Forward+, weather, terrain, simulation, or the release
roadmap.

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

## 7.1 Closed documentation package — WP-GOV-005

User-visible result: Marquee is a coherent, machine-governed post-1.0 promotional-material authority without adding a Meridian 1.0 obligation or claiming implementation.

Deliverables:

- canonical Marquee specification and ADR-0025;
- `PRM` as the 37th current domain with `REQ-PRM-001` through `REQ-PRM-006`;
- deferred `PRG-PRM-001`, post-MS-10 `RG-PRM-001`, `VAL-PRM-001`, and ten risks;
- machine-enforced manual-capture, export-only, explicit-human-approval, source-classification, and text/analysis-only AI policy;
- zero-unmapped in-version amendment ledger and private Project Meridian production-boundary update.

Closure evidence: `EV-GOV-20260715-005`; review: `REV-GOV-20260715-003`.

Explicit limits: no Marquee crate, media/PDF adapter, campaign UI, service integration, promotional output, calibration, quality evidence, commit, or push. Marquee remains deferred; the current active package is recorded in section 12.

## 7.2 Closed documentation package — WP-GOV-006

User-visible result: Meridian now has an honest post-1.0 plan for pursuing and
proving workload-specific performance and quality leadership, plus stable seams
that prevent weather, smoke, fire, fluids, procedural authoring, and rendering
from duplicating authority or hidden cost.

Deliverables:

- canonical competitive performance and quality specification and ADR-0027;
- ADR-0026 for one Penumbra participating-media consumption path, sparse and
  multirate environmental work, typed surface-fluid ownership transfer, and
  one-way-snapshot default coupling;
- deferred `PRG-REL-001`, post-MS-10 `RG-REL-001`, `VAL-REL-001`, seven
  program requirements, and ten explicit risks;
- separate iso-quality performance, iso-cost quality, and matched-workflow
  claim classes with immutable exact-version baselines;
- first-use/stutter, tail latency, memory, lower-tier, recovery, accessibility,
  security, provenance, workflow, and maintenance evidence rules;
- Alluvium combustion/fluid material facets and a calibrated
  `RuntimeCostManifest` that predicts cost without becoming runtime authority;
- a zero-unmapped v0.5 amendment ledger and cross-spec/registry reconciliation.

Deferred execution after MS-10: freeze the public fixture and legal/evidence
policy; calibrate structural, reference, blinded perceptual, temporal,
first-use, and workflow methods; establish unoptimized baselines; measure
environmental convergence and cook/runtime prediction error; rank bottlenecks
by user-visible impact; prototype one bounded change behind an owning seam;
rerun every affected quality tier and non-performance gate; adopt winners via
owning ADRs, archive losers, and publish only scoped expiring claims. A future
planning review creates package IDs and dependencies only after those inputs
are stable.

Closure evidence: `EV-GOV-20260716-001`; review:
`REV-GOV-20260716-001`.

Explicit limits: no renderer/simulation implementation, comparator adapter,
competitive fixture calibration, optimization, public claim, commit, push, or
remote CI run. `PRG-REL-001` cannot satisfy, block, or promote MS-00 through
MS-10. The current active package is recorded in section 12.

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

## 9. Closed package — WP-REL-002 / MS-01 qualification

| Wave | Packages | Convergence rule |
|---|---|---|
| Instrumentation | `WP-PEN-007` closed | trustworthy CPU/GPU timing outcomes for current high-level passes |
| Surface and runtime | `WP-PEN-008`, `WP-RUN-002`, `WP-RUN-003` closed `ImplementedFoundation` | asynchronous visible/offscreen capture, typed surface outcomes, lifecycle epochs, device rebuild action, and correlated diagnostics |
| Source data | `WP-DAT-002` -> `WP-DAT-003` -> `WP-DAT-004` closed `ImplementedFoundation` | provisional source import, independent facets, package mount, worker load, bounded activation, save and recovery preserve authority |
| Integration | `WP-RUN-004` closed `ImplementedFoundation` | one `meridian` executable runs native, native-smoke, and headless-smoke paths |
| Qualification | `WP-REL-002` closed `Implemented` | GitHub Actions run `29452928922` passed governance and Linux, Windows, and macOS workspace/headless-smoke rows for `010db80` |

User-visible result: Meridian imports the public generic MS-01 fixture, writes
and reopens a provisional `.meridian` package, streams and activates one cell,
advances fixed simulation from semantic input, renders package-derived geometry,
writes a hashed RGBA8 PNG plus metadata, exercises save replacement/backup/
journal/migration recovery, and emits one correlated JSON evidence timeline.

Passing implementation evidence: `EV-PEN-20260715-003`, `EV-RUN-20260715-002` through
`EV-RUN-20260715-004`, and `EV-DAT-20260715-002` through
`EV-DAT-20260715-004`. Qualification review `EV-REL-20260715-001` is `Pass`:
GitHub Actions run `29452928922` passed the governance and all three platform
rows, including the required Linux and Windows headless smokes.

Limits: the native surface was occluded or unavailable in the qualification run,
so the visible image is explicitly offscreen and makes no presentation or visual-
quality claim. JSON, compiled facets/cells, `.meridian` v1, and save transaction
encoding remain provisional. Compression, signing, patches, encryption,
Forward+, specular IBL, Meridian UI, Creator Editor workflows, and game content
remain outside MS-01.

`MS-01` and `MS-02` are closed. `WP-BLD-001`, `WP-PRC-001`, and
`WP-EDT-001` are qualified MS-03 foundations. `RG-UI-001` is `Decided` by
`ADR-0029`; `WP-UI-002` through `WP-UI-005` and `WP-MDL-001` are retained as
`Partial` under `WVR-UI-001` and cannot be promoted by local UI work.
`WP-UI-005` reached its bounded local source stop point and remains
unqualified. `WP-UI-006` is the sole active source-only package under
`WVR-UI-001`; `WP-EDT-002` is suspended as `Partial` under `WVR-EDT-001` and
remains unqualified.
Penumbra Stage 1 then proceeds through `WP-PEN-003` and
`WP-PEN-010`, with `WP-PEN-009` as bounded parallel image-quality work and
`WP-PEN-011` as the later executable/calibrated forest corpus. `RG-PEN-001`
cannot open before MS-05.

Alluvium begins only through a separately activated package. `WP-PRC-001`
through `WP-PRC-004` are required for MS-05; no Alluvium package is active.

Work package: `WP-REL-002`
User-visible result: the source-derived Meridian foundation is qualified on the
governance, Linux, Windows, and macOS CI matrix without converting its
offscreen capture or foundation formats into broader product claims.
Status: `Implemented`; `MS-01` `Pass`
Source checkpoint and BuildId: `010db80` / GitHub Actions run `29452928922`
Requirements: `REQ-REL-001`, `REQ-GOV-003`
Milestone contribution: MS-01 observable runtime and source foundations
Entry conditions and dependencies: MS-00 `Pass`; `WP-RUN-004`
`ImplementedFoundation`
Files/crates/formats changed: qualification registries and current-evidence
documents only; no runtime crate or durable format changed
Deliverables and public contracts: reproducible cross-platform CI qualification
for the existing `meridian --headless-smoke --frames 4` public executable path
Explicit non-goals: presented-surface evidence, visual-quality claims, stable
source/package/save formats, Meridian UI, and Creator Editor workflows
Tests: CI governance validation, format, workspace check/test/clippy, and
headless smoke all passed on Linux, Windows, and macOS
Benchmarks and hardware: no benchmark calibration; CI hardware remains a
virtualized qualification environment
Captures/traces/recovery evidence: `EV-PEN-20260715-003`,
`EV-RUN-20260715-002` through `004`, `EV-DAT-20260715-002` through `004`, and
`EV-REL-20260715-001`
Accessibility: headless qualification is non-visual; MS-02 owns native UI
keyboard, semantic, scaling, and motion evidence
Security/provenance: public generic fixture only; no private-game content or
new dependencies
Migration/compatibility: no format or API migration
Documentation: AGENTS, delivery, planning, testing, rendering, and typed
evidence/review registries reconciled to the completed CI run
Integration checkpoint: minimal observable source-derived application
Stop/rollback rule: no MS-01 stop condition was observed; a failed matrix row
would have retained `Partial`/`Active` status
Known limits and unsupported rows: native presentation was occluded or
unavailable, so the capture remains explicitly Offscreen; no production visual
quality or calibrated benchmark claim
Reviewers/sign-offs: GitHub Actions completed all required automated rows;
separate human release approval is not recorded
Next sequence: Meridian UI framework work continues in section 15.

## 10. Closed package — WP-UI-001 / MS-02 Meridian UI core proof

Execution status: `ImplementedFoundation`; MS-02 status: `Pass`. Implementation
maturity: `ImplementedFoundation`;
retained UI contracts, deterministic fixtures, system-font glyph rasterization,
and a bounded temporary native two-stage raster bridge are implemented. The
recovery panel first reported an unavailable local surface and submitted
structural fallback evidence, then presented on retry; the runtime overlay also
presented locally. `EV-UI-20260715-001` records that scoped local result and
GitHub Actions run `29457181283` for `fb8323f` passed governance and all three
platform rows, including UI-headless, UI-free runtime, and dependency-audit
checks. No production renderer selection has landed.

Disabled-cost evidence: the bridge is the opt-in
`meridian-renderer/ui-raster-bridge` feature selected only by
`meridian-editor`. The UI-free `meridian-rt` headless-profile smoke advances a
real runtime frame while `! cargo tree -p meridian-rt | grep -q meridian-ui`
proves its package graph selects no UI crate. CI runs both checks. This is a
profile-scoped no-UI-cost trace, not a claim that an editor build omits UI.

User-visible result: Meridian gains one native, keyboard-operable recovery
panel and one runtime overlay from an immutable Meridian-owned display list,
with a deterministic headless fixture and a semantic-tree snapshot. This is the
MS-02 core proof, not Creator Editor Alpha, docking, Ponder, a final renderer,
or a visual-quality claim.

Requirements and milestone contribution: `REQ-UI-001`; MS-02 accessible
native panel and runtime overlay integration checkpoint.

Entry evidence and dependencies: MS-01 is `Pass`; `WP-RUN-004` is
`ImplementedFoundation` with `EV-RUN-20260715-004` `Pass`. The existing
platform/input/renderer seams and the UI/accessibility specifications have been
inspected. `DEP-UI-001` records the text-shaping boundary; `RG-UI-001` remains
closed until after MS-02 and does not select a production display-list renderer.

Owned files/crates: `engine/meridian_ui`, the opt-in
`meridian-renderer/ui-raster-bridge` feature, `editor/meridian_editor`, and
only the Meridian-owned RHI bridge required to present the immutable overlay;
`Cargo.toml`, `Cargo.lock`, the UI/accessibility authority, registries, and
this plan change only when evidence requires them.

Public contracts: stable `UiNodeId`; retained node/document tree; bounded
`UiEvent`, text-input cursor/selection snapshots, capability-gated clipboard
requests, and semantic command requests; focus/event-route result; immutable
display-list primitives; owned semantic tree/delta and diagnostics; a text
layout result without exposed `cosmic_text`, Unicode-segmentation, backend, or
platform types. Runtime and editor widget surfaces remain separate.

Implementation slices and failure cases:

- replace the marker with retained tree, stack/overlay layout, focus, capture /
  target / bubble routing, deterministic display-list emission, semantic-node
  validation, and diagnostics; reject duplicate IDs, missing nodes, unnamed
  focusable nodes, invalid actions, and focus traps before rendering;
- add the `cosmic-text` adapter for shaping, fallback, line layout, bounded
  glyph rasterization, and IME composition state plus a bounded
  `unicode-segmentation` editing adapter for extended-grapheme cursoring,
  selection, insertion/deletion, password masking, and non-password
  capability-gated copy requests, without allowing either library's types past
  the Meridian boundary;
- render the recovery panel and runtime overlay through an immutable raster /
  RHI bridge, with device-loss rebuild from the logical document and explicit
  occluded/unavailable outcomes; both native paths now execute and present
  locally after the recovery panel's first unavailable-surface fallback;
- add keyboard-only and semantic golden fixtures, DPI/contrast/reduced-motion
  layout fixtures, a UI-free runtime profile trace plus package-graph audit,
  and a native smoke where the surface supports it.

Tests and commands: start with `cargo test -p meridian-ui` and
`cargo test -p meridian-editor`; add `meridian --ui-headless-smoke` for the
deterministic fixture and `meridian --ui-smoke` for native presentation where
available. Then run the proportional workspace ladder, `meridian-spec check`,
and privacy, dependency, and diff audits.

Security, accessibility, and recovery: no rich-text execution, external links,
network content, secrets, or agent context enter this package. Keyboard focus,
visible focus, semantic names/actions, text scaling, high contrast, and reduced
motion are verified from the same retained tree. A malformed document opens as
an accessible recovery/error panel; UI GPU resources rebuild from logical state.

Explicit non-goals: Creator Editor documents/undo/play, docking, source UI
document persistence, Ponder, platform AccessKit adapters, visual graph editing,
renderer-path selection, benchmark calibration, and any private-game input.

Stop/rollback rule: stop and redesign if any public UI contract exposes wgpu,
cosmic-text, or platform types; events mutate state during traversal; semantic
validation cannot report a remediation; a headless profile allocates UI work; or
the overlay cannot rebuild after device loss. In that case retain the logical
contracts and revert only the failed renderer adapter path.

Qualification and sign-off: source checkpoint `fb8323f`; `EV-UI-20260715-001`
is `Pass`. GitHub Actions run `29457181283` completed governance and Linux,
Windows, and macOS format/check/test/clippy, UI-headless, UI-free runtime, and
dependency-audit rows successfully. The qualification is headless and no local
capture establishes visual quality. `WP-BLD-001` subsequently qualified as the
local-Cargo prerequisite; `WP-EDT-001` subsequently qualified as the Creator
behavioral foundation; Meridian UI framework work continues in section 15.

## 11. Closed package — WP-BLD-001 / MS-03 observable build foundation

Work package: `WP-BLD-001`

User-visible result: a Meridian command or CLI caller can submit a bounded
Cargo operation to one observable build service and receive typed lifecycle,
diagnostic, artifact, cancellation, and stale-result outcomes instead of
scraped terminal text. The first slice proves the service contract with Cargo
metadata and JSON-message fixtures; it does not claim a complete Creator Editor.

Status: `ImplementedFoundation`; implementation maturity: `ImplementedFoundation`

Source checkpoint and BuildId: entry checkpoint `b1c87c3`; BuildId becomes a
Meridian-owned deterministic hash only after the first implementation slice
accepts its declared inputs.

Requirements: `REQ-BLD-001`, `REQ-CORE-002`, `REQ-CORE-004`

Milestone contribution: MS-03 Creator Editor Alpha critical path and reusable
MS-08 build/IDE seam.

Scope boundary: `WP-BLD-001` owns the bounded MS-03 local-Cargo prerequisite:
one observable operation, typed Cargo lifecycle/diagnostics, one optional
verified executable, durable local recovery, and the command/event seam for
`WP-EDT-001`. Planned `WP-BLD-002` owns MS-08 continuation work that would
change the execution model or trust boundary: managed external toolchains with
exact project pins, multi-node result lineage, general artifact/cache policy,
service-process and remote-worker supervision, team profiles, and broad
reproducibility evidence. Those capabilities are not
part of this package and cannot keep its MS-03 prerequisite open.

Entry conditions and dependencies: MS-02 is `Pass`; `WP-BLD-001` has no direct
package dependency and the current Cargo workspace, lockfile, source fixtures,
task/cancellation seams, and `meridian-editor` composition have been inspected.
No private-game input, remote worker, signing key, or named hardware is needed
for this first bounded slice.

Files/crates/formats changed: editor-only `meridian-build`, the shared fallible
`meridian-tasks` worker-pool primitive, workspace membership, optional
host-selected versioned local build-state JSON, and documentation/registry
records. No project manifest, persistent game format, or `game/` path may change.

Deliverables and public contracts: the editor-only `meridian-build` crate now
provides Meridian-owned `BuildId`, `BuildRequest`, `BuildNode`, lifecycle/event/
diagnostic types, cooperative cancellation, bounded file hashing, a structured
Cargo metadata/check/build/test-compilation JSON adapter, bounded redacted Cargo
process-failure diagnostics, and the `meridian-build --cargo-check`,
`--cargo-build`, or `--cargo-test-no-run` helper CLI. The helper runs every
operation through the durable service and one fallibly-created local supervisor
worker; it prints a unique default state path under `target/meridian-build/`,
retains it after interruption or failure, removes it only after successful
default-owned completion, and accepts caller-owned `--state PATH` recovery. Cargo and rustc types
remain internal; arguments remain arrays and Cargo
JSON remains the compiler-diagnostic protocol; ordered command arguments are
also declared BuildId inputs. Cargo metadata retains an exact bounded payload
hash for traces and derives a separate checkout-independent workspace
package/manifest/target hash that combines with the lockfile in the BuildId;
source checkpoint, toolchain, target, and allowlisted host environment remain
local inputs. `BuildId` also binds a canonical `BuildGraph` contract hash over
the requested roots and each node's ID, kind, tool, declared input hashes,
declared environment names, and dependency topology; changing that contract
cannot reuse the prior identity. New requests and request-bound artifact
references retain that canonical declared graph plus its contract hash, but not
per-node result lineage. A separate bounded stderr record is present for an unsuccessful
Cargo process; cancellation recovery can instead emit a typed descendant-warning.
A bounded versioned, host-selected local
state store publishes a synced temporary snapshot through a same-directory
rename; `DurableBuildService` persists each accepted mutation, restores
interrupted work as `WorkerLost`, and rejects malformed, oversized, symlinked,
or non-regular state files. External worker events are revalidated before they
can mutate or persist this service: protocol version, Cargo artifact bounds,
artifact/hash pairing, running-phase payload/lifecycle correspondence,
request-provenance correspondence, and redaction must match the Meridian-owned
event contract. This local boundary does not claim
worker authentication or sandboxing. `BuildGraph` validates declared dependencies,
the exact requested-root/BuildId graph-contract agreement, duplicate inputs/environment,
cycles, and unreachable nodes; its deterministic local scheduler starts a Cargo metadata
node before the dependent Cargo-check or Cargo-build node and blocks dependents after terminal
failure. Concurrent/resource-aware scheduling, durable cache/provenance, and
non-Cargo nodes remain unimplemented; the retained graph is input provenance,
not a complete cache or per-node result record. Those execution-model changes
are reserved for planned `WP-BLD-002`.
`CargoBuildSupervisor` is a long-lived, single-worker local coordinator for the
bounded Cargo adapter. It admits only one exact registered running request at a
time with a matching Cargo root node, retains cooperative cancellation, polls one outcome
in deterministic operation-ID order, persists normal completion/cancellation,
and records a task panic or disconnect as `WorkerLost`. It owns no project source
state and is not a remote-worker protocol or a separately supervised service process.
Cargo's `build-finished` result is retained as an operation constraint: it cannot
contradict the terminal `Succeeded` or `Failed` phase, and a contradictory local
worker outcome is durably failed without publishing its messages.
`ArtifactStore` accepts a host-selected bounded regular source file, copies and
hashes it into a BLAKE3-addressed object, verifies any pre-existing object, and
atomically creates a non-overwriting BuildId/node reference with declared schema
and tool identity. Cargo-reported executable references additionally retain the
bounded Cargo package ID and target name. A running `BuildService` or
`DurableBuildService` can emit a typed artifact event only when that verified
reference carries the request's BuildId and root-node ID, exposing the verified
content hash to the host. The helper CLI can opt in to publish exactly one
Cargo-reported executable after a
successful build or test-compilation command when paired host-selected
`--artifact-store` and `--cargo-output-root` paths are supplied. The executable
must be listed by Cargo and be a bounded regular non-symlink file beneath the
canonical output root; it is copied and event-bound before the terminal success
transition. General Cargo artifact selection, cache policy, and remote
provenance remain absent.

Explicit non-goals: a lossless TOML editor, rust-analyzer session, remote
worker, signing/deployment, Alluvium adapter, editor panels, package graph
visualization, or Creator Editor Alpha completion.

Tests: deterministic identity; invalid lifecycle transitions and progress
bounds; stale BuildId or
sequence rejection; cancellation before process spawn and induced Unix
build-script-child cancellation; malformed Cargo JSON and aggregate-output
rejection with reader backpressure;
compiler diagnostic/artifact mapping; typed full Cargo metadata parsing and a
checkout-relocation fixture for its workspace identity hash;
graph-contract identity mutation/rejection and request-provenance fixture;
canonical durable-graph-manifest mutation/rejection and artifact-reference reopen fixture;
single-worker Cargo-supervisor success, busy/duplicate-submit, and cancellation fixtures;
secret-like JSON and process-stderr diagnostic redaction; malformed,
mismatched, and unredacted external-worker-event rejection before durable
acceptance, including terminal-payload and request-provenance bypass attempts;
durable snapshot publication/reopen and
`WorkerLost` recovery; graph order, invalid-edge, cycle, unreachable-node, and
blocked-dependent fixtures; verified artifact object/reference publication,
corrupt-object rejection, conflicting-reference rejection, output-root escape
and unlisted-executable rejection, and request-bound artifact-event fixture;
and a structured local Cargo smoke, verified artifact-event smoke, and
helper-CLI check/build/test-compilation including one opt-in executable
publication that never invokes a shell.

Benchmarks and hardware: no performance claim in the first slice; local Cargo
toolchain only. Remote, sandbox, and named-hardware profiles are explicitly not
run.

Captures/traces/recovery evidence: local deterministic event timelines cover
queued through terminal states; cancellation-before-spawn, an induced Unix
Cargo build-script-child cancellation, and malformed-event tests pass. The
induced fixture writes a marker then starts `/bin/sleep 60`; the adapter places
Cargo in an isolated Unix process group and bounded cancellation returns before
that child can run to completion. Unix cancellation uses fixed `/bin/kill`
`TERM` then `KILL` group signals after a 250 ms grace; Windows uses the explicit
system `taskkill.exe /PID /T /F` tree path. Neither uses a shell or inherits the
host environment. If a platform tree terminator cannot run, the service kills
and reaps the direct Cargo child and records a typed warning rather than
claiming descendant recovery. GitHub Actions run `29505405013` for source
checkpoint `becef55486d434460c3afebfb96e734655dfcb09` exercised the configured
Windows Visual Studio developer environment and passed workspace tests,
warning-denied clippy, helper smokes, bounded Cargo build, and independent
Cargo test-artifact smoke. That row does not turn the Unix process-group
cancellation fixture into a Windows cancellation claim. The durable local store
reopens interrupted work as
`WorkerLost`, persists that recovery before exposing it, and rejects late
success. It also rejects unsupported event protocol versions, malformed Cargo
artifacts, mismatched event diagnostics, and unredacted external diagnostics
before they can advance or persist operation state; durable rejection leaves
the prior operation snapshot intact. Non-lifecycle worker payloads must retain
the running phase, and external artifact events must carry the same durable
input provenance as the registered request. The adapter drains stdout through a backpressured reader with aggregate byte and
line limits, and bounded stderr concurrently, so failed local Cargo runs return a typed redacted process diagnostic. The local single-worker
Cargo supervisor persists normal/cancelled outcomes and maps task loss to
`WorkerLost`. It also rejects a Cargo `build-finished` value that contradicts
the terminal lifecycle before publishing worker messages; external events and
durable snapshots reject the same contradiction. External service-process/remote-worker supervision, general
Cargo-artifact qualification, and durable build-wide provenance are planned
`WP-BLD-002` work and do not block this MS-03 package. The local
graph proof is only Cargo metadata -> check/build; it makes no parallelism, resource,
cache, or non-Cargo adapter claim. A host-selected `ArtifactStore` can copy one
bounded regular file into a BLAKE3-addressed object, verify a pre-existing
object, and atomically create a BuildId/node reference with declared schema and
tool identity. Cargo-reported executable references retain the package ID and
target name supplied by Cargo, but the store still verifies copied bytes rather
than proving that a Cargo invocation produced its source path. After a successful helper build or
test-compilation command, an opt-in paired artifact-store/output-root request
can select exactly one Cargo-reported executable: it must be listed by Cargo,
regular, non-symlinked, within the canonical output root, and within the
existing bounded-file limit. The host records its verified artifact event while
the matching request is still running; mismatched BuildId, node, or secret-safe
request-input provenance is rejected before the hash becomes observable. The
reference retains source checkpoint/profile, metadata-plus-lock identity,
toolchain/target, sorted roots, canonical graph-contract hash, and hashes of arguments and allowlisted
environment values without retaining the raw argument or environment values.
New references also retain the canonical declared graph (node IDs, kinds, tools,
declared input hashes, environment names, and dependency topology); they do not
yet record a result for every graph node.
Zero, multiple, arbitrary, or
oversized Cargo outputs are not published.

Accessibility: the service emits named stages, actionable typed diagnostics,
verified artifact outcomes, and cancellation/retry states for later Meridian UI
consumption; no visible
panel is claimed by this package.

Security/provenance: command arguments are structured arrays; environment is
allowlisted; Cargo children receive no ambient environment. Windows linker and
SDK context is limited to explicitly allowlisted Visual Studio variables and is
bounded local identity input rather than ambient fallback; Cargo output is untrusted input;
the graph-contract identity records only declared environment names and no raw
environment values; the retained graph manifest uses the same secret-safe fields;
external worker events are revalidated for protocol/payload/diagnostic
consistency, running-phase use, and request provenance before durable acceptance;
paths and diagnostic text are bounded and redacted; aggregate Cargo output and
every lifecycle progress value are bounded before durable acceptance; no secrets, credentials,
private-game paths, or shell concatenation are permitted. Artifact roots,
objects, references, and source files reject direct symlinks; opt-in Cargo
executable publication additionally requires an explicit non-symlink canonical
output root and rejects root escape. Existing objects must hash to their
content-addressed name and references never overwrite a different BuildId/node
result.

Migration/compatibility: Cargo-reported executable references add optional
package/target, graph-contract, and graph-manifest provenance. Existing local
references deserialize without any optional addition and retain no corresponding
provenance; the v1 protocol remains additive.

Documentation: the Cargo/IDE/build authority, repository architecture,
registries, and this package record are the owning sources; they are updated
only with implementation evidence.

Integration checkpoint: the bounded BLD service must remain editor-only and
provide the command/event seam consumed by `WP-EDT-001`; runtime crates must
not depend on Cargo, IDE, or build-service crates.

Stop/rollback rule: stop and redesign if a Cargo/IDE type leaks through a
Meridian public boundary, a command is shell-concatenated, a stale/cancelled
operation can publish an artifact, untrusted output is accepted without bounds,
or an engine/runtime crate gains a build-service dependency.

Known limits and unsupported rows: only local Cargo JSON metadata, check, build,
and test-compilation flows are in scope initially; test compilation uses Cargo
`--no-run`, so test-harness execution remains unsupported by this JSON adapter.
When this helper is itself launched through `cargo run` on Windows, its Cargo
build or test-compilation request must not relink the running
`meridian-build.exe`; the documented and CI proofs therefore target the
independent `meridian-core` library. Test compilation also passes an explicit
separate Cargo target directory, preventing linker contention with the
helper's active target tree. An installed or separately hosted service is not
that launch mode, but self-rebuild orchestration is not claimed by this bounded
slice.
The exact metadata payload hash remains trace-only. Its normalized workspace
identity excludes checkout roots but does not make the complete BuildId
cross-machine reproducible: source checkpoint, toolchain, target, and
allowlisted host environment remain deliberate local inputs. The local state and
artifact stores are host-selected foundations, not remote, signing, provider, or
cache-policy stores. The identity now detects a changed declared local graph
contract, and new durable references retain the complete declared graph manifest,
but build-wide provenance still lacks per-node result lineage. The current graph scheduler is single-host, dependency-only,
and restricted to the Cargo metadata -> check/build proof. Concurrent/resource-aware
build-DAG scheduling, external service-process restart, lossless manifest editing,
rust-analyzer, managed toolchain installation/repair/rollback, remote
execution, signing, and deployment are planned `WP-BLD-002` work.

Reviewers/sign-offs: Definition of Ready was reviewed against the current
workspace and MS-02 evidence. `EV-BLD-20260715-001` records local
recovery/build evidence. `EV-BLD-20260716-002` records GitHub Actions run
`29505405013` for `becef55486d434460c3afebfb96e734655dfcb09`: governance plus
Linux, Windows, and macOS workspace rows passed, including the configured
Windows BLD helper/build/artifact proof. The package is closed at its bounded
local-Cargo foundation scope; `WP-BLD-002` remains planned.

Next sequence: Meridian UI framework work continues in section 15 after the
qualified Creator foundation.

## 12. Qualified foundation — WP-EDT-001 / MS-03 Creator Editor Alpha

Work package: `WP-EDT-001`

User-visible result: a creator can open the public generic Creator Alpha
project, transactionally import its source through the DAT-owned adapter, edit a world placement through typed
transactions, undo/redo, enter isolated Play and explicitly apply or discard
its diff, recover a durable session, inspect an accessible Meridian-native
workspace, and submit a request-bound one-worker local Cargo build artifact.

Status: `ImplementedFoundation`; implementation maturity: `ImplementedFoundation`

Requirements: `REQ-EDT-001`, `REQ-CORE-001`

Dependencies and Definition of Ready: `WP-UI-001`, `WP-DAT-004`, and
`WP-BLD-001` are `ImplementedFoundation` with registered evidence.
`DEP-UI-003` records the editor-only `rfd` native-picker adapter and its
license/provenance boundary. The current workspace, public
`examples/creator-alpha/` source, existing Meridian UI semantics, durable save
boundary, and BLD command/event/artifact seam were inspected. No private game
input, new UI toolkit, remote service, or platform credential is required.
This package is closed as the behavioral baseline and can remain stable while
the sequential UI packages proceed without changing runtime/game authority.

Files/crates/formats changed: `meridian-editor-core` owns canonical
`meridian.creator-project/v1` source documents, source-authoritative project
sessions, typed transactions/inverses/checkpoints, generation-checked
selection, Play forks, and durable recovery. `meridian-ui-editor` owns
declarative accessible hub/workspace panel contracts for project, hierarchy,
viewport, inspector/history, asset/import/build, recipe, modeler, diagnostics,
and recovery. `meridian-editor` composes those boundaries, a private typed
Creator action adapter, and the editor-only native-picker adapter. The public
sample stores generic JSON source only; it introduces no private format or game
content.

Deliverables and public contracts: default `meridian` launches a persistent
Creator hub with explicit Create, Open, and bounded Recent Projects actions.
It never opens a recent project implicitly; unavailable recents remain visible
with remediation/removal controls. A project stores canonical,
atomically-written `meridian.creator-project/v1` source and a separate local
recovery record. `--creator-alpha-ui-smoke` remains bounded test-only behavior.
`--creator-alpha-smoke --project <path> --evidence <path>` exercises the
public journey and records durable BLD state, BuildId, bound artifact hash,
worker count, source generation, and limitations. All UI actions pass through
a private typed whitelist; project mutation remains in typed editor-core
commands, never UI callbacks.

Tests: editor-core source transaction rollback, invalid metadata rollback,
checkpoints, stale selection, Play isolation/apply/discard, and ordered
recovery; hub creation/open/recents and typed-action rejection; input-to-command
routing and keyboard semantics; UI panel/action semantic fixtures; durable
asynchronous BLD status; a process-level Creator Alpha journey that verifies
the request-bound artifact; and lifecycle-policy/native review for normal-app
persistence. GitHub Actions run `29605881704` passed the complete Linux,
Windows, and macOS package matrix for `4463bad`.

Accessibility and recovery: hub and workspace controls are focusable semantic
Meridian controls with named commands. Tab/shift-tab, pointer, text/IME,
editing keys, and primary-command shortcuts route through the same typed
actions. The workspace declares recovery and diagnostics panels. A stale
recovered local selection is cleared rather than blocking source recovery; the
atomically written source document remains durable and generation checked. An
untrusted recovery sidecar may restore validated non-authoritative context, but
it never resumes undo/redo history after restart.

Security/provenance: sample source paths must remain project-relative regular
files; the DAT-owned importer derives imported source identity/hash and the
editor preserves its resulting identity/path/hash as source authority. The
native picker is invoked only by an explicit platform-thread user action and
returns a Meridian-owned path result. Cargo remains behind the BLD-owned typed,
bounded, explicit-environment seam. No engine/runtime crate depends on
editor-core, UI-editor, or BLD crates.

Explicit non-goals: trusted macOS distribution, Developer ID signing,
notarization, docking, broad asset import, full platform accessibility
adapters, and MS-03 closure. The bundled app is an unsigned developer preview;
users may explicitly authorize it through Gatekeeper, but it makes no trust or
notarization claim. Broad modeler work remains outside partial `WP-MDL-001`.

Stop/rollback rule: stop and redesign if a UI type enters editor-core/source
commands, a derived preview becomes source authority, a Play change applies
without explicit diff, a stale selection mutates source, recovery loses a valid
source document, or a Cargo/build type crosses into runtime crates.

Qualification evidence: `EV-EDT-20260717-002` records GitHub Actions run
`29605881704` for `4463bad5244697bf482e08ce755723928fccca31`; governance and
the complete Linux, Windows, and macOS rows passed. `EV-EDT-20260717-003`
records the local persistent native launch, first-frame redraw, bundle identity
and architectures, explicit exit, and exact capture hashes. These qualify only
the Creator behavioral foundation. Platform screen-reader integration,
Meridian UI 1.0 visual quality, and visible approval remain later evidence.

Next sequence: Meridian UI framework work continues in section 15.

## 13. Closed package — WP-PRC-001 / MS-03 Alluvium foundation

Work package: `WP-PRC-001`

User-visible result: a creator can keep a public canonical `.mproc` recipe as
source, validate, inspect, preview, bake, audit, explain, diff, and recover its
derived cache through the same typed Meridian contracts.

Status: `ImplementedFoundation`; implementation maturity: `ImplementedFoundation`

Requirements: `REQ-PRC-001`, `REQ-PRC-002`, `REQ-PRC-003`, `REQ-PRC-004`,
`REQ-PRC-005`, `REQ-PRC-008`, `REQ-CORE-001`, `REQ-CORE-002`

Dependencies and Definition of Ready: `WP-DAT-002` and the qualified
`WP-EDT-001` provided source identity/import and the required basic inspector.
The source delivery adds no external dependency, private content, runtime
authority, worker, or renderer edge.

Files/crates/formats changed: `meridian-alluvium` owns `meridian.procedural-
recipe/v1` pretty canonical JSON, strict scalar evaluation, stable generated
IDs, derived cache integrity/recovery, dirty reports, retained overrides, and
provenance/license policy. `meridian-ui-editor` provides a semantic text-first
inspector; `meridian alluvium` exposes structured command parity. The public
Creator Alpha sample is the generic fixture only.

Explicit non-goals: visual graph editing, terrain/vegetation/weather adapters,
runtime-safe evaluation, GPU/SIMD kernels, production corpus, or modeler
semantics. Derived outputs never become source authority.

Tests: canonical round-trip/migration, deterministic scalar evaluation,
cancellation/budget, precise dirty report, corrupt-cache recovery, override
applied/conflicted/orphaned outcomes, license audit, all structured commands,
semantic inspector actions, and Creator Alpha preview/bake parity.

Stop/rollback rule: stop if a cache or generated output becomes source
authority, IDs depend on array order, an override disappears silently, a
license failure bakes output, or a runtime/editor dependency enters the crate.

Reviewers/sign-offs: GitHub Actions run `29511174569` for
`9c88cc152878b1eb22f18c236c00ad1abd984fa5` passed governance plus Linux,
Windows, and macOS format, workspace check/test, warning-denied clippy, editor
headless/UI-headless smokes, runtime profile smoke, BLD helper/artifact smokes,
and minimal-runtime dependency audit. The workspace suite includes recipe
canonicalization/migration, deterministic and dirty evaluation, cache recovery,
override outcomes, structured CLI/UI parity, and the public Creator Alpha
recipe journey.

Known limits and unsupported rows: CI is headless; no native presented-surface,
visual-quality, production-performance, graph-authoring, domain-adapter, or
runtime-safe-evaluation claim is made.

Next sequence: Meridian UI framework work continues in section 15. The
delivered MS-03 subset of `WP-MDL-001` is recorded as `Partial` below and
cannot become a second active package in this context.

## 14. Partial package — WP-MDL-001 / MS-03 native modeler foundation

Work package: `WP-MDL-001`

User-visible result: a creator can create one editable public primitive, select
its stable source elements, transform it, perform one bounded topology change,
undo or redo semantic edits, recover an accepted revision, inspect every action
by keyboard, and view a derived Penumbra preview without making that preview
source authority.

Status: `Partial`; implementation maturity: `Partial`

Requirements: `REQ-MDL-001`, the MS-03 subset of `REQ-MDL-002` and
`REQ-MDL-003`, and `REQ-CORE-001`

Dependencies and Definition of Ready: `WP-DAT-002`, `WP-UI-001`, the
qualified `WP-EDT-001` foundation, and `WP-PRC-001` are available. The package
owns only a Meridian-native source-document/modeling kernel and editor composition; it adds
no runtime, physics, animation, game, private-content, external-DCC, or new
third-party dependency edge. It remains `Partial` while the sequential
UI/editor composition packages run.

Files/crates/formats changed: this source delivery adds `meridian-modeler` for
a versioned editable-model document, immutable revisions, stable element IDs,
topology mapping, semantic transactions, recovery records, and a derived
Penumbra-preview descriptor. `meridian-ui-editor` exposes a text-first,
keyboard-accessible model inspector. `meridian-editor` and the public Creator
Alpha fixture exercise the same typed source boundary.

Explicit non-goals: UVs, normals tooling beyond the primitive invariant,
material authoring, broad topology tools, modifiers, collision/LOD, sculpting,
rigging, external interchange, Blender integration, GPU mesh ownership, and
production Penumbra rendering. Those remain `WP-MDL-002`, MS-05, or later
work. `WP-MDL-001` remains `Partial` after this MS-03 delivery.

Tests: source schema and topology invariants; stable vertex/edge/face lineage;
stale selection rejection; primitive creation and transforms; bounded topology
map; semantic undo/redo and recovery; source-versus-preview authority;
Alluvium override-migration seam; accessible inspector actions; and the public
Creator Alpha process journey.

Stop/rollback rule: stop and redesign if a renderer mesh becomes editable
source, a topology edit silently changes or loses a stable ID, stale selection
mutates a revision, undo replays UI events rather than semantic source changes,
or a preview/cache replaces the accepted source revision.

Known limits and unsupported rows: local evidence only until this delivery has
its own full CI run. Native presented-surface screenshots and a keyboard/
accessibility review remain required MS-03 integration evidence after the
modeler source gate.

Next sequence: Meridian UI framework work continues in section 15; the
milestone cannot close before the framework, production shell, accessibility,
and visible review.

## 15. Partial package — WP-UI-002 / modular retained framework

Work package: `WP-UI-002`

User-visible result: Meridian gains one locked, testable design system and a
modular retained UI foundation capable of representing the permanent shell and
basic controls without backend or editor types entering public contracts.

Status: `Partial`; implementation maturity: `Partial`. Source checkpoint
`050b41ff5ab9dce309f94d7ea53f82de2acd27c6` is locally validated and pushed;
cross-platform qualification is unavailable.

Requirements: `REQ-UI-001`, `REQ-UI-002`

Dependencies and Definition of Ready: `WP-UI-001` is
`ImplementedFoundation`. ADR-0028, the normative UI specification, exact token,
component, and workspace registries, the reviewed external design brief, and
the generated 17-state mockup corpus define the accepted boundary. No renderer
choice, AccessKit adapter, docking, or application rewrite is in this package.

Delivered source: `meridian-ui-core`, `meridian-ui-text`,
`meridian-ui-semantics`, `meridian-ui-render`, and `meridian-ui-runtime` crate
boundaries; a compatibility `meridian-ui` facade; retained documents and stable
node identities; incremental reconciliation; immutable frame snapshots;
Flex/Grid/Overlay/Absolute/Scroll layout; renderer-neutral display primitives,
clips/layers/shadows/bounded backdrop descriptors; locked token/theme/density/
contrast/motion descriptors; typography and audited-icon asset boundaries; and
basic controls.

Tests: schema and registry rejection; stable identity and reconciliation;
layout constraints/cycles/rollback; display-list ordering/clips/effect bounds;
1×/2× deterministic output; theme/high-contrast/reduced-motion descriptors;
runtime/editor dependency isolation; and compatibility-facade migration.

Explicit non-goals: text editing/IME, complete pointer/scroll/drag behavior,
professional controls, virtualization, docking, companion windows, platform
accessibility adapters, renderer promotion, or final application panels. Those
belong to `WP-UI-003` through `WP-EDT-003`.

Stop/rollback rule: stop if stable identity is derived from row position, a
backend/font/icon/editor type enters a public contract, mutable traversal can
partially commit a frame, runtime UI gains an editor dependency, the locked
palette/geometry drifts, or mockups are treated as capability evidence.

Local validation passed targeted UI tests, full workspace tests,
warning-denied Clippy, `meridian-spec check`, locked metadata, format, native
and headless UI smokes, Creator journey, RHI/renderer structural smokes,
dependency-boundary checks, and `git diff --check`. GitHub Actions run
`29611418454` and its manual rerun executed zero steps because GitHub refused
to allocate a hosted runner for the account billing state. The workflows
concluded `failure`; their skipped Linux, Windows, and macOS implementation rows
are `NotRun`. No package or milestone promotion follows.

Current sequence: `WP-UI-002` through `WP-UI-005` remain locally delivered as
`Partial`; active `WP-UI-006` owns the authored-document and source-to-frame
foundation. `WP-EDT-002` is suspended at its recorded source state.

## 15.1 Partial package — WP-UI-003 / professional interaction and controls

Work package: `WP-UI-003`

User-visible result: Meridian UI gains precise pointer and scrolling phases,
complete bounded text and clipboard requests, typed drag/drop with cancellation
and keyboard alternatives, stable collection navigation, professional retained
control contracts, and deterministic virtualization without introducing editor
or platform-library types into the framework.

Status: `Partial`; implementation maturity: `Partial`. The public interaction
contracts, failure paths, professional control families, editor bridge, and
local validation exist. Hosted cross-platform qualification remains unavailable
and cannot be waived into a completion claim.

Requirements: `REQ-UI-001`, `REQ-UI-002`

Dependencies and Definition of Ready: `WP-UI-002` supplies its locally proven
crate, identity, frame, layout, display-list, and token boundaries but remains
`Partial`. `WVR-UI-001` is a validation-role, non-promoting waiver expiring
2026-08-17; it permits local continuation only and records the risk of
unobserved Linux or Windows regressions. The package adds no third-party
dependency and changes no source-authoritative editor or game format.

Files/crates: `meridian-ui-core` owns typed device, scroll, drag/drop,
validation, collection, and virtualization contracts; `meridian-ui-text` owns
bounded editing behavior; `meridian-ui-runtime` owns gesture capture, nested
scroll handoff, focus-stable interaction state, typed drop proposals, and frame
snapshots; the compatibility facade and editor adapter migrate without exposing
platform types.

Tests: press/move/release/cancel and capture; line, pixel, momentum, gesture
locking, and nested scroll handoff; IME, selection, cut/copy/paste and invalid
text; pointer and keyboard drag completion/cancellation; stable Home/End/Page
navigation and filtering; virtual-range bounds and stable identities; malformed
event and aggregate-limit rollback; runtime/editor/UI-free dependency checks.

Explicit non-goals: docking, saved workspaces, companion windows, animation,
AccessKit/platform screen readers, renderer selection, the production
application shell, or claims that incomplete domain panels function.

Stop/rollback rule: stop if pointer activation occurs before a valid release,
momentum is smoothed twice, a nested scroll loses residual delta, drag/drop
mutates source directly, filtering replaces stable selection identity, virtual
controls allocate the full collection, password text reaches semantics or a
clipboard request, or a platform/editor type enters public framework contracts.

Local validation passed `meridian-spec check`, locked metadata, format, full
workspace tests, warning-denied workspace Clippy, native/headless UI and Creator
journey smokes, RHI/renderer structural smokes, runtime/UI dependency checks,
private-content and secret scans, and `git diff --check`. IME validation uses
UTF-8 byte boundaries matching the private winit adapter while retained editing
uses grapheme boundaries. The current workflow passes local `actionlint`; the
historical package run did not include that check.
Linux and Windows remain unobserved under `WVR-UI-001`; no package or milestone
promotion follows.

Next package: active `WP-UI-006` is source-only under non-promoting
`WVR-UI-001`; it remains unqualified.

## 15.2 Partial package — WP-UI-004 / docking, workspaces, and persistence

Work package: `WP-UI-004`

User-visible result: Meridian UI gains transactional split/tab/floating dock
trees, remembered workspace layouts, responsive region priorities, recoverable
versioned persistence, and typed companion-window transfer without binding the
framework to an editor domain or native windowing library.

Status: `Partial`; implementation maturity: `Partial`. The public dock and
workspace contracts, corruption/recovery paths, and local validation exist.
Hosted cross-platform qualification remains unavailable and cannot be waived
into a completion claim.

Requirements: `REQ-UI-001`, `REQ-UI-002`

Dependencies and Definition of Ready: the locally proven `WP-UI-003` slice
supplies stable input, focus, professional controls, typed drag/drop, and
bounded virtualization while remaining `Partial`. `WVR-UI-001` permits only
non-promoting local continuation. No third-party dependency, source-authoritative
project format, game content, or native window type enters this package.

Files/crates: `meridian-ui-editor` owns editor-only dock trees, panel IDs,
workspace layouts, responsive priorities, companion-window descriptors, and
versioned state persistence. Shared core/runtime crates gain only genuinely
domain-neutral layout or focus contracts required by the public boundary.

Deliverables: split/tab/floating trees; preview tabs, pinning, reorder,
tear-off, collapse, maximize, minimum sizing, reset, and transactional rollback;
named layouts with migration and corruption, missing-panel, and monitor-loss
recovery; primary-frame and session-sharing companion-window descriptors;
per-workspace document, selection, camera, query, expansion, scroll, pin, and
focus state; responsive priorities that preserve the working canvas and
accessible control sizes.

Tests: invalid and cyclic dock trees; minimum-size rejection; split, reorder,
tear-off, redock, maximize, collapse, reset, and rollback; schema migration,
corrupt state, unknown panel, and missing monitor recovery; stable context
through workspace switches; responsive collapse order; companion-window
session isolation; aggregate and persistence bounds; editor/runtime dependency
checks.

Explicit non-goals: animation and shared-element effects, AccessKit or native
screen-reader adapters, renderer qualification, production application shell,
domain workspace behavior, or claims that a descriptor alone creates a native
window.

Stop/rollback rule: stop if an invalid dock mutation partially commits, a saved
layout can allocate without bounds, monitor loss makes content unreachable,
responsive adaptation shrinks controls below accessible sizes, companion
windows gain separate source authority, unknown panels prevent recovery, or
editor/native-window types enter runtime UI crates.

Delivered source: `meridian-ui-editor` now owns fixed-width stable panel, dock,
workspace, companion-window, monitor, and remembered-context identities;
validated split/tab/collapsed/floating dock forests; preview and pinned tabs;
transactional activation, reorder, move, split, tear-off, whole-subtree redock,
collapse, maximize, and reset; and accessible integer split resolution.
Moving the last panel from a branch prunes the empty split without losing the
panel, and a floating tree cannot be re-docked into itself.

Workspace state uses strict `meridian.ui-workspace-state/v1` JSON inside the
existing atomic `meridian-save` envelope. It has explicit v0 migration,
revision-checked mutations, named layouts, second-activation focus layouts,
session-sharing companion descriptors, previous-state recovery, missing-panel
reporting, monitor-loss clamping, and typed corrupt/session-mismatch fallback.
Per-workspace document, selection, camera, query, expansion, scroll, focus, and
companion state remain local user state rather than project source authority.
Responsive adaptation includes the locked 8px dock gutters, preserves pinned
and working-canvas regions, and uses controlled overflow before violating the
44px accessible minimum.

Local validation passed 23 `meridian-ui-editor` tests, `meridian-spec check`,
locked metadata, formatting, full workspace tests, warning-denied workspace
Clippy, native/headless UI and Creator journeys, RHI/renderer/runtime structural
smokes, the UI-free runtime dependency check, and `git diff --check`.
The current workflow passes local `actionlint`; the historical package run did
not include that check. Linux and Windows remain unobserved under
`WVR-UI-001`; no package or milestone promotion follows.

Next package: active `WP-UI-006` is source-only under non-promoting
`WVR-UI-001`; it remains unqualified.

## 15.3 Partial package — WP-UI-005 / motion, accessibility, and renderer qualification

Work package: `WP-UI-005`

User-visible result: Meridian UI gains interruptible physical motion,
reduced-motion substitutions, bounded overlay effects, complete Meridian-owned
semantics with private platform adapters, and an evidence-backed display-list
renderer decision without allowing adapter types into public APIs.

Status: `Partial`; implementation maturity: `Partial`. `WP-UI-005` reached its
bounded local source stop point under non-promoting `WVR-UI-001`. Interruptible bounded
motion, Reduced Motion, effect fallback, complete Meridian semantic snapshots,
private AccessKit projection/action routing/recovery, pane-focus cycling,
100–400% scale fixtures, and the complete 15-category display-list corpus have
real local evidence. `RG-UI-001` is `Decided` by `ADR-0029`. The selected direct
GPU path now has bounded isolated/nested layer targets and a local native
`Presented` structural smoke with bounded surface readback. It also has the
bounded direct backdrop and geometry/color-quality treatments described below,
plus local profile-bound offscreen raw-RGBA golden comparison, controlled
device-destruction replay, raw uncalibrated performance samples, and a canonical
2x capture copied from a mapped `Presented` surface. These are `Inconclusive`
qualification evidence for current local/dirty source and are not human-approved
visual, accessibility, or cross-platform qualification. Real screen-reader and
human-review evidence is absent, and hosted cross-platform qualification remains
unavailable; none can be waived into a completion claim.

Requirements: `REQ-UI-001`, `REQ-UI-002`; gate: `RG-UI-001`

Dependencies and Definition of Ready: the locally proven `WP-UI-004` slice
supplies transactional docking, persisted workspaces, responsive priorities,
and companion-window descriptors while remaining `Partial`. MS-02 has passed,
so `RG-UI-001` could open. `WVR-UI-001` permits only non-promoting local
continuation. `DEP-UI-004` records exact AccessKit versions, checksums, licenses,
features, private adapter boundaries, recovery, and platform fixtures.

Files/crates: shared UI crates own interruptible motion, shared-element,
contrast, scale, semantics, live-region, reading-order, and recovery contracts;
private platform adapters may map accepted semantic snapshots. The editor
crate consumes these contracts but does not move docking or workspace state
into runtime crates.

Deliverables: springs only for physical panel/shared-element movement;
100–160ms state transitions; immediate or brief-opacity Reduced Motion;
bounded floating blur with opaque fallbacks; keyboard traversal, pane cycling,
focus restoration, names, relationships, values, live regions, 100–400% text
scaling, and high contrast; private assistive-action adapters; device-loss and
adapter recovery; and a recorded `RG-UI-001` renderer decision against the real
display-list corpus in an ADR.

Tests: animation interruption and retargeting; Reduced Motion substitution;
effect bounds and opaque fallbacks; semantic tree reading order and identity;
keyboard/pane traversal and focus restoration; accessible names, actions,
values, and live regions; 100–400% scaling and high contrast; adapter rejection
and recovery; deterministic display-list replay; renderer corpus correctness,
latency, memory, text quality, device loss, and platform support.

Explicit non-goals: production application shell and World composition,
invented screen-reader qualification, unbounded blur, decorative focus rings,
application capability simulation, or a renderer decision without the adopted
ADR and corpus evidence. Those remain `WP-EDT-002` or later.

Stop/rollback rule: stop if motion changes source or layout authority, Reduced
Motion retains spatial animation, inaccessible content is hidden behind an
effect, semantic identity follows row position, adapter objects cross public
boundaries, device loss strands focus/recovery, or occluded structural output
is described as visual quality.

Local validation passed motion interruption, Reduced Motion, semantic-tree and
untrusted-action tests, AccessKit projection/reactivation, companion focus
restoration, high contrast and 100–400% scale, effect fallback, all 15 display
primitive categories, explicit fallback gaps, targeted Clippy, and
`meridian-spec check`. The corrective source adds locked fonts/icons with
machine provenance, bounded frame diagnostics, RHI render identity and typed
indexed batches, plus a real direct GPU preparation/upload slice across the
15-category contract. Bounded full-viewport offscreen targets isolate and
compose nested layers, and the local native direct smoke reached `Presented`.
The direct path converts authored sRGB values to linear, rejects non-sRGB
surfaces with a typed error, uses premultiplied alpha for content and layers,
physically snaps axis-aligned geometry, adaptively tessellates curves/corners,
adds a one-physical-pixel rounded-rectangle fringe, emits real join/cap
wedges/sectors, and applies bounded four-step soft-shadow falloff. Its fixed 3x3
tent backdrop filter reconstructs parent-prefix GPU targets within the shared
64 MiB aggregate target guard and scales declared padding to one physical texel.
Clear-only layer/backdrop targets submit without fake draws, and fully clipped
layers allocate no GPU target. The native smoke reached `Presented` with two
layers and one filter after clear-only target regressions. Atlas dimensions,
geometry treatment, fully clipped draws, resource and effect-target bounds,
stencil depth, binding requirements, nested composition, and rejected-frame
state are regression-tested. Native presented-surface readback also validates
bounded RGBA8 sRGB metadata and non-uniform output; this is local structural
evidence, not a visual-quality claim. Local profile-bound raw-RGBA comparison,
controlled device-destruction replay, raw uncalibrated performance samples, and
presented review input are recorded as `EV-UI-20260718-001` through
`EV-UI-20260719-004`: the settled
local working-tree runner outputs are profile-bound 3/3 exact raw-RGBA goldens
at `target/meridian-evidence/ui-direct-qualification/20260718-settled/`, a
controlled `Destroyed` -> `DeviceLost` recovery with a zero-difference
57,600-pixel replay at
`target/meridian-evidence/ui-direct-device-loss-replay/20260718-settled/`, and
three warmups plus ten samples per mode at
`target/meridian-evidence/ui-direct-performance/20260718-settled/`, plus the
640x360 canonical 2x presented-surface capture at
`target/meridian-evidence/ui-direct-presented-review/20260719-local-2/`. Each is
`Inconclusive`, caller-declared working-tree evidence and is non-promoting.
Human visual approval, calibrated renderer measurements, real screen-reader and
accessibility qualification, and cross-platform CI
remain open. This is not renderer or package completion. The complete proportional
gates are rerun before source delivery. Cross-platform CI remains required
before promotion.

The bounded `ui_accessibility_review` runner now keeps a real AccessKit-backed
fixture alive long enough for an assistive client, records only native adapter
actions with payload contents redacted, and leaves five spoken-output/focus
checks to a human reviewer. Local `EV-UI-20260719-005` exercised its timeout
path: adapter projection passed, no assistive action was observed in five
seconds, and screen-reader evidence is correctly `NotRun`.

The retained runtime now records saturated monotonic durations for accepted
reconciliation, layout, text shaping/rasterization, display validation, and
semantic-delta work. A bounded Meridian-owned source-timestamp side table
reports exact minimum/maximum/mean source-to-reconciliation-boundary latency
only when a caller supplies comparable monotonic frame-boundary timing.
Malformed timing batches are typed pre-mutation rejections with accepted
revision, focus, and private text rollback covered by local tests; untimestamped
callers remain truthfully `Unavailable`. Actual input-to-presented-surface
latency remains platform/renderer evidence. This closes the locally actionable
diagnostics-contract gap but does not supply calibrated performance, human
review, cross-platform CI, or a package-promotion claim.

The corrective runtime scopes keyboard, assistive, programmatic focus, and
host-delivered target interaction to the visible top
combo/menu/context-menu/command-palette subtree. The first pointer press outside
that surface dismisses it without activating the background target; opening the
surface cancels prior pointer/scroll/drag/timeline/canvas preview capture; and
document replacement restores invalidated focus inside the visible transient
before considering the background. The semantic projection keeps only the root
and active transient subtree, correctly reparenting nested transient semantics
without exposing background controls to assistive traversal. Local regressions
cover keyboard cycling; hidden/background assistive and host-value rejection;
programmatic-focus denial; scoped scroll and property-cancel rollback; focus
restoration; topmost nested-surface dismissal; stale pointer-release
cancellation; and outside-click dismissal. This remains local source evidence
only.

The direct renderer now preflights a collision-safe, exact-content-deduplicated
atlas, keeps per-image, aggregate-atlas, and vertex/index failures typed
separately, and treats valid zero-area whitespace masks as non-drawing glyphs.
The real retained 720x450 logical / 1440x900 physical framework component
gallery prepares successfully without platform-specific state. Its local native
presented-review attempt remained `SkippedOccluded`, so no screenshot, visual
approval, or evidence promotion is claimed. The runner also preserves its first
failure report instead of allowing a later platform callback to overwrite the
root cause.

## 15.4 Active source-only package — WP-UI-006 / authored documents and source-to-frame compilation

Work package: `WP-UI-006`

User-visible result: Meridian UI uses the existing `UiDocument` as its
versioned canonical editable source while making the locked design system easy
to author through typed spacing/geometry, named styles, reusable components,
stable IDs, source diagnostics, bounded packaged assets, and deterministic
source-to-frame compilation.

Status: `Active` only under non-promoting `WVR-UI-001`; implementation maturity
is not promoted by this local source work. `WP-UI-005` supplies the retained
layout, semantics, renderer-neutral display list, direct-path recovery, and
locked tokens while remaining `Partial`. `WP-EDT-002` is deliberately suspended
at its recorded source state until this shared authored vocabulary exists. The
requirement registry now maps both `REQ-UI-001` and `REQ-UI-002` to this source
package, matching the package registry and the owning UI specification.

Deliverables: one `UiDocument` source model with schema/version, root, stable
nodes, styles, component definitions and explicit typed instance projections,
token/style references,
semantic bindings, and bounded raster asset references; ergonomic builders and
diagnostics; stable source-location categories for typed validation failures
and document-level canonical-source envelope failures;
immutable compiled snapshots consumed identically by runtime and Creator;
bounded canonical source bytes with an explicit envelope, legacy
snapshot migration, and full revalidation before recovery; source-to-frame
determinism and recovery tests; and direct text corpus checks at 1x and 2x.
Vectors remain native bounded paths or audited `IconId` values. Raw
constructors remain only for migration and fixture compatibility.

Explicit non-goals: a separate UI authoring document, loose file paths or SVG
parsing in the source model, backend handles in source, executable content,
direct-manipulation canvas editing, renderer/accessibility qualification, or
milestone promotion.

Latest local source closure pass: `UiDocumentCompiler` now exposes the
canonical source-to-frame boundary as one immutable `UiCompiledFrame`, and the
native Creator application owns that compiler façade rather than a parallel
editor-only runtime. Authored image nodes require a packaged `UiAssetRef`;
private resolution binds the source node to a process-local image handle and
successful compilation emits a renderer-neutral image primitive, while missing
or wrong-kind assets produce typed unavailable diagnostics without advancing
retained state. Authored document deltas report changed style definitions, component
definitions, and node source provenance by stable identity in addition to
retained/inserted/removed/updated nodes. `UiDocument::component_instances`
exposes each component-attached stable node as a typed explicit instance
projection, so Creator inspection does not infer instances by scanning raw
source maps. The fresh continuation pass also covers stable typed component
instance projection and component-attachment deltas in `EV-UI-20260801-005`.
Regression coverage proves that an
authored style, component default, or packaged-asset reference cannot change
without an addressable delta, and that authored/recovered source compiles to
the same layout, renderer-neutral display list, and semantic tree as the
resolved contract. The Creator UI-authoring preview now consumes this compiler
façade for its real World-source inspection frame instead of constructing a
parallel preview path. Canonical persisted source bytes now enter through that
same façade: the compiler decodes and fully validates bounded source before
atomically replacing retained state, reports legacy storage-shape migration,
uses typed envelope/legacy parsing without first materializing an untyped JSON
tree, and preserves the last accepted source on decode failure. Source
diagnostics distinguish transient-surface failures from generic semantics.
Packaged-asset
lowering is now part of the same façade: resolution happens before frame
advancement, repeated references share one derived package resolution, and
missing assets leave retained runtime state unchanged. The UI-authoring preview
also carries the same
source-derived World grid and placement decoration as the normal World path,
so its compiled viewport is an actual inspection surface rather than an empty
placeholder. The current local
non-promoting evidence set includes the UI-authoring review capture at
`target/meridian-evidence/ui-authoring-header-fix.png`, direct display-list
corpus output at
`target/meridian-evidence/ui-direct-qualification/wpui-final/`, controlled
device-loss replay at
`target/meridian-evidence/ui-direct-device-loss-replay/wpui-final/`, and the
native accessibility review at
`target/meridian-evidence/ui-accessibility-review/wpui-final/`. The latest
authoring capture is
`target/meridian-evidence/ui-authoring-header-fix.png`; its retained browser
and inspector headers remain inside their panel rows, and the same preview
path now has a 1×/2× projected-glyph scaling regression. Sequential package
suites also pass for core (40), text (14), runtime (122), and editor (59),
plus the Creator application suite (42), covering the locally implemented
UI-002 through UI-004 contracts and the UI-006 source path. The clean workspace
suite, warning-denied Clippy, spec/metadata/diff checks, RHI and renderer
smokes, UI headless smoke, and Creator UI smoke are now recorded in
`EV-UI-20260801-001`; the earlier July closure record remains historical. A
feature-gated `meridian-platform --features accessibility` run also passes all
20 private AccessKit projection/action tests as `EV-UI-20260801-002`, without
claiming real assistive-client observation; the same feature compiles for
Windows GNU and macOS targets as `EV-UI-20260801-003`, without claiming native
execution. A
fresh World-shell review capture is registered as
`EV-UI-20260722-009`; it remains local offscreen review material and does not
self-approve visual quality. The feature-gated and full-workspace cross-target
checks are compile-only dirty-source evidence and do not execute native Windows
or macOS, platform accessibility adapters, renderer presentation, or packaging;
the full-workspace result is recorded separately as
`EV-UI-20260801-004`. The evidence set is therefore
registered as `EV-UI-20260722-001` through `EV-UI-20260724-001`, with the
current source pass recorded separately in `EV-UI-20260801-001`; canonical
source encoding now rejects payloads larger than the same bounded 8 MiB source
envelope accepted by recovery.
These artifacts remain `Inconclusive` or `NotRun` where the evidence contract
requires human, screen-reader, or cross-platform observation; they do not
promote any UI package.

Stop/rollback rule: stop if an authored asset becomes a loose path or renderer
handle, a component instance loses explicit stable identity, a compile failure
cannot name its source node/property, raw constructors regain design-system
authority, or a cache becomes source authority.

## 15.5 Suspended partial package — WP-EDT-002 / permanent shell and World workspace

Work package: `WP-EDT-002`

User-visible result: normal `meridian` launches the exact permanent two-row
application shell, a production-quality hub, and the World workspace with the
locked palette, hierarchy/browser, real viewport, wider inspector, shelf,
status, persistent context, and native accessibility.

Status: `Partial` and suspended under non-promoting `WVR-EDT-001`; it resumes
only after `WP-UI-006`. The qualified `WP-EDT-001` Creator behavior remains
available, but current local work cannot establish editor completion, visual
quality, accessibility, framework qualification, or milestone promotion.
Current local source adds a typed Settings surface backed by a versioned local
hub-preference document: high contrast, Reduced Motion, and density persist
atomically, v1 hub state migrates to v2, settings restore an open workspace,
preferences never mutate project source, and the permanent Play/Build actions
remain bound to a temporarily covered open project rather than falsely becoming
unavailable. The activity rail's pane command now cycles the active dock pane
through the versioned workspace state and rolls back if its local save fails.

Deliverables: exact separate 44px application and 36px workspace rows;
macOS-correct title chrome; persistent hub; World composition over real
Creator source/import/edit/Play/build/recovery authority; typed unavailable
states for incomplete domains; remembered focus/context; keyboard and assistive
journeys; native captures; and visible review.

Explicit non-goals: completing later domain workspaces, simulating unavailable
capabilities, a web shell, a generic assistant sidebar, decorative rings,
private game content, signing/notarization claims, or milestone promotion
without fresh Linux, Windows, macOS, accessibility, and visible evidence.

On resumption, rebuild the shell and World workspace from their existing
authoritative Creator behavior using `WP-UI-006` components. Then
`WP-EDT-003` may compose current workspaces while preserving explicit
unavailable states for incomplete domains. Neither package, `WP-UI-006`, nor
`MS-03` can close while cross-platform, accessibility, and visible-review
evidence remains absent.

## 16. Evidence policy

Every run records source checkpoint, BuildId when available, corpus/build hashes, hardware, OS, backend, driver, capability profile, settings, cache/warmup state, distributions rather than averages alone, memory, artifacts, and missing features. Occluded structural evidence cannot satisfy visual quality. Unavailable hardware or capabilities are `NotRun`, `UnsupportedPlatform`, or `UnsupportedCapability`, never Pass.

No benchmark report may generalize beyond its measured workload/profile. No uncalibrated number becomes a release gate.

## 17. Mandatory package sign-off

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
