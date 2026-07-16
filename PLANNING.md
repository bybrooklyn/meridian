# Meridian Active Work Plan

version 0.5 · 2026-07-16

Status: `MS-01` and `WP-REL-002` passed qualification on GitHub Actions run
`29452928922` for source checkpoint `010db80`: governance plus Linux, Windows,
and macOS workspace/headless-smoke rows passed. All named MS-01 implementation
packages are closed with fresh evidence. `WP-UI-001` is an
`ImplementedFoundation` and MS-02 is `Pass` from cross-platform CI evidence.
`WP-BLD-001` and `WP-EDT-001` are `ImplementedFoundation` after cross-platform
GitHub Actions evidence. `WP-PRC-001` is the sole active package; its
Definition of Ready and bounded Alluvium implementation slice are recorded below.

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
| Meridian UI | `ImplementedFoundation` | retained layout, text input, semantics, deterministic fixtures, an opt-in raster bridge, and local native/headless smokes are qualified for the MS-02 core proof; Creator Editor workflows remain planned |
| Creator Editor | `ImplementedFoundation` | Creator Alpha source/session/transaction/Play/recovery/UI/build foundation is qualified; milestone integration review remains |
| Audio, Isobar, Basalt, vegetation | `Scaffold` unless a registry entry says narrower | implementation remains outside the current Creator Editor package |
| Alluvium | `Partial` | active `WP-PRC-001` source delivery provides canonical text recipes, strict scalar evaluation, derived-cache recovery, CLI, and basic inspector; CI qualification and production/domain work remain open |
| Native modeler, Rust gameplay, Luau, animation, navigation, frameworks, first-class 2D, Meridian Shader Language, Collective | `Planned` or `Deferred` | specifications and registries exist; no product implementation claim |
| Torsant, networking, XR, modding, agents, VCS/sync | `Planned`, `Research`, or `Deferred` | no production implementation claim |
| Distributed worlds, advanced integrity, and other `PRG-*` programs | `Deferred` or `Research` | post-1.0 authority only; no milestone or implementation evidence |
| Marquee | `Deferred` | ResearchReady post-1.0 architecture; no crate, active package, service integration, or promotional-quality evidence |
| Competitive performance and quality leadership | `Deferred` | ResearchReady post-1.0 comparison, environmental-convergence, and claim architecture; no calibrated corpus, optimization, comparator integration, or superiority evidence |

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

`MS-01` and `MS-02` are closed. `WP-BLD-001` and `WP-EDT-001` are qualified
MS-03 prerequisites; `WP-PRC-001` is the sole active sequential package.
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
Next unblocked package: `WP-EDT-001`, now active

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
local-Cargo prerequisite; `WP-EDT-001` is active below.

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
change the execution model or trust boundary: multi-node result lineage,
general artifact/cache policy, service-process and remote-worker supervision,
team profiles, and broad reproducibility evidence. Those capabilities are not
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
rust-analyzer, remote execution, signing, and deployment are planned
`WP-BLD-002` work.

Reviewers/sign-offs: Definition of Ready was reviewed against the current
workspace and MS-02 evidence. `EV-BLD-20260715-001` records local
recovery/build evidence. `EV-BLD-20260716-002` records GitHub Actions run
`29505405013` for `becef55486d434460c3afebfb96e734655dfcb09`: governance plus
Linux, Windows, and macOS workspace rows passed, including the configured
Windows BLD helper/build/artifact proof. The package is closed at its bounded
local-Cargo foundation scope; `WP-BLD-002` remains planned.

Next unblocked package: `WP-EDT-001`, active below.

## 12. Closed package — WP-EDT-001 / MS-03 Creator Editor Alpha

Work package: `WP-EDT-001`

User-visible result: a creator can open the public generic Creator Alpha
project, transactionally import its source through the DAT-owned adapter, edit a world placement through typed
transactions, undo/redo, enter isolated Play and explicitly apply or discard
its diff, recover a durable session, inspect an accessible Meridian-native
workspace, and submit a request-bound one-worker local Cargo build artifact.

Status: `ImplementedFoundation`; implementation maturity: `ImplementedFoundation`

Requirements: `REQ-EDT-001`, `REQ-CORE-001`

Dependencies and Definition of Ready: `WP-UI-001`, `WP-DAT-004`, and
`WP-BLD-001` are `ImplementedFoundation` with registered evidence. The current
workspace, public `examples/creator-alpha/` source, existing Meridian UI
semantics, durable save boundary, and BLD command/event/artifact seam were
inspected. No private game input, new UI toolkit, external crate, remote
service, or platform credential is required. `WP-EDT-001` is the sole active
package and can stop without changing runtime/game authority.

Files/crates/formats changed: `meridian-editor-core` owns versioned
source-authoritative project sessions, typed transactions/inverses/checkpoints,
generation-checked selection, Play forks, and durable recovery.
`meridian-ui-editor` owns declarative accessible panel contracts for project,
hierarchy, viewport, inspector/history, asset/import/build, recipe, modeler,
diagnostics, and recovery. `meridian-editor` only composes those boundaries and
the existing native/runtime smoke. The public sample stores generic JSON source
only; it introduces no private format or game content.

Deliverables and public contracts: `meridian --creator-alpha-smoke --project
<path> --evidence <path>` requires caller-owned project and evidence paths and
executes open → import → edit → undo → redo → Play apply → Play discard →
recover → build. It rejects missing arguments and untrusted absolute, parent,
or non-regular project source references before use. Its local evidence records
the durable BLD state, BuildId, bound artifact hash, worker count, source
generation, and explicit limitations. All UI actions are semantic focusable
Meridian buttons; project mutation remains in typed editor-core commands, never
in UI callbacks.

Tests: editor-core transaction inverse, invalid metadata rollback, checkpoints,
stable-ID collisions, stale selection, Play isolation/apply/discard, and
save-backed recovery; UI panel/action semantic fixtures; parser coverage; and a
process-level Creator Alpha smoke that verifies the request-bound artifact. The
next validation step is the complete package gate and GitHub Actions matrix.

Accessibility and recovery: inspector/history actions are focusable semantic
buttons with named commands. The workspace declares recovery and diagnostics
panels. A stale recovered local selection is cleared rather than blocking source
recovery; the source document/history remain durable and generation checked.

Security/provenance: sample source paths must remain project-relative regular
files; the DAT-owned importer derives imported source identity/hash and the
editor preserves its resulting identity/path/hash as source authority.
Cargo remains behind the BLD-owned typed, bounded, explicit-environment seam.
No engine/runtime crate depends on editor-core, UI-editor, or BLD crates.

Explicit non-goals: Alluvium recipe evaluation and inspector parity,
editable-model operations/topology lineage, native presented-surface review,
platform accessibility adapters, docking, full asset import, and MS-03 closure.
Those boundaries remain for `WP-PRC-001`, partial `WP-MDL-001`, or the mandatory
post-package review.

Stop/rollback rule: stop and redesign if a UI type enters editor-core/source
commands, a derived preview becomes source authority, a Play change applies
without explicit diff, a stale selection mutates source, recovery loses a valid
source document, or a Cargo/build type crosses into runtime crates.

Reviewers/sign-offs: GitHub Actions run `29508496428` for
`ec2a6334dd19506b1d2b353e60557ef13d86b153` passed governance plus Linux,
Windows, and macOS format, workspace test, warning-denied clippy, editor,
runtime, BLD-helper, and dependency-audit rows. The workspace suite includes
the Creator Alpha process journey. Native review, screenshots, and the
keyboard/accessibility checklist remain milestone integration evidence.

Next unblocked package: `WP-PRC-001`, active below.

## 13. Active package — WP-PRC-001 / MS-03 Alluvium foundation

Work package: `WP-PRC-001`

User-visible result: a creator can keep a public canonical `.mproc` recipe as
source, validate, inspect, preview, bake, audit, explain, diff, and recover its
derived cache through the same typed Meridian contracts.

Status: `Active`; implementation maturity: `Partial`

Requirements: `REQ-PRC-001`, `REQ-PRC-002`, `REQ-PRC-003`, `REQ-PRC-004`,
`REQ-PRC-005`, `REQ-PRC-008`, `REQ-CORE-001`, `REQ-CORE-002`

Dependencies and Definition of Ready: `WP-DAT-002` and the now-qualified
`WP-EDT-001` provide source identity/import and the required basic inspector.
The source delivery adds no external dependency, private content, runtime
authority, worker, or renderer edge. `WP-PRC-001` is the sole active package.

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

Known limits and unsupported rows: local source/test evidence only until this
delivery has its own full CI run; no visual-quality or production-performance
claim is made.

Next unblocked package: `WP-PRC-001` remains active until its CI evidence is
recorded; only then may `WP-MDL-001` activate.

## 14. Evidence policy

Every run records source checkpoint, BuildId when available, corpus/build hashes, hardware, OS, backend, driver, capability profile, settings, cache/warmup state, distributions rather than averages alone, memory, artifacts, and missing features. Occluded structural evidence cannot satisfy visual quality. Unavailable hardware or capabilities are `NotRun`, `UnsupportedPlatform`, or `UnsupportedCapability`, never Pass.

No benchmark report may generalize beyond its measured workload/profile. No uncalibrated number becomes a release gate.

## 15. Mandatory package sign-off

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
