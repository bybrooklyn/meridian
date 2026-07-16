# Meridian Active Work Plan

version 0.5 · 2026-07-15

Status: `MS-01` and `WP-REL-002` passed qualification on GitHub Actions run
`29452928922` for source checkpoint `010db80`: governance plus Linux, Windows,
and macOS workspace/headless-smoke rows passed. All named MS-01 implementation
packages are closed with fresh evidence. `WP-UI-001` is an
`ImplementedFoundation` and MS-02 is `Pass` from cross-platform CI evidence.
`WP-BLD-001` is the sole active package; its Definition of Ready and bounded
first implementation slice are recorded below.

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
| Meridian UI | `ImplementedFoundation` | retained layout, text input, semantics, deterministic fixtures, an opt-in raster bridge, and local native/headless smokes are qualified for the MS-02 core proof; Creator Editor workflows remain planned |
| Creator Editor, audio, Isobar, Basalt, vegetation | `Scaffold` unless a registry entry says narrower | the `meridian-editor` package now owns the MS-01 **Meridian** executable, not Creator Editor Alpha |
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

`MS-01` and `MS-02` are closed. `WP-BLD-001` is the sole active MS-03 critical
path package. Penumbra Stage 1 then proceeds through `WP-PEN-003` and
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
Next unblocked package: `WP-BLD-001`, now active

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
capture establishes visual quality. `WP-BLD-001` is the next unblocked package
and is active below.

## 11. Active package — WP-BLD-001 / MS-03 observable build foundation

Work package: `WP-BLD-001`

User-visible result: a Meridian command or CLI caller can submit a bounded
Cargo operation to one observable build service and receive typed lifecycle,
diagnostic, artifact, cancellation, and stale-result outcomes instead of
scraped terminal text. The first slice proves the service contract with Cargo
metadata and JSON-message fixtures; it does not claim a complete Creator Editor.

Status: `Active`; implementation maturity: `Partial`

Source checkpoint and BuildId: entry checkpoint `b1c87c3`; BuildId becomes a
Meridian-owned deterministic hash only after the first implementation slice
accepts its declared inputs.

Requirements: `REQ-BLD-001`, `REQ-CORE-002`, `REQ-CORE-004`

Milestone contribution: MS-03 Creator Editor Alpha critical path and reusable
MS-08 build/IDE seam.

Entry conditions and dependencies: MS-02 is `Pass`; `WP-BLD-001` has no direct
package dependency and the current Cargo workspace, lockfile, source fixtures,
task/cancellation seams, and `meridian-editor` composition have been inspected.
No private-game input, remote worker, signing key, or named hardware is needed
for this first bounded slice.

Files/crates/formats changed: new editor-only `meridian-build` crate; workspace
membership; optional host-selected, versioned local build-state JSON; and
documentation/registry records. No runtime crate, project manifest, persistent
game format, or `game/` path may change.

Deliverables and public contracts: the editor-only `meridian-build` crate now
provides Meridian-owned `BuildId`, `BuildRequest`, `BuildNode`, lifecycle/event/
diagnostic types, cooperative cancellation, bounded file hashing, a structured
Cargo metadata/check/build/test-compilation JSON adapter, bounded redacted Cargo
process-failure diagnostics, and the `meridian-build --cargo-check`,
`--cargo-build`, or `--cargo-test-no-run` helper CLI. Cargo and rustc types
remain internal; arguments remain arrays and Cargo
JSON remains the compiler-diagnostic protocol; a separate bounded stderr record
is present only for an unsuccessful Cargo process. A bounded versioned, host-selected local
state store publishes a synced temporary snapshot through a same-directory
rename; `DurableBuildService` persists each accepted mutation, restores
interrupted work as `WorkerLost`, and rejects malformed, oversized, symlinked,
or non-regular state files. `BuildGraph` validates declared dependencies,
requested-root/BuildId agreement, duplicate inputs/environment, cycles, and
unreachable nodes; its deterministic local scheduler starts a Cargo metadata
node before the dependent Cargo-check or Cargo-build node and blocks dependents after terminal
failure. Concurrent/resource-aware scheduling, durable cache/provenance,
non-Cargo nodes, and cross-checkout metadata normalization remain unimplemented.
`ArtifactStore` accepts a host-selected bounded regular source file, copies and
hashes it into a BLAKE3-addressed object, verifies any pre-existing object, and
atomically creates a non-overwriting BuildId/node reference with declared schema
and tool identity. The build adapter reports Cargo artifact messages but does not yet
automatically select, validate, or publish them; cache policy remains absent.

Explicit non-goals: a lossless TOML editor, rust-analyzer session, remote
worker, signing/deployment, Alluvium adapter, editor panels, package graph
visualization, or Creator Editor Alpha completion.

Tests: deterministic identity; invalid lifecycle transitions; stale BuildId or
sequence rejection; cancellation before process spawn; malformed Cargo JSON;
compiler diagnostic/artifact mapping; typed full Cargo metadata parsing;
secret-like JSON and process-stderr diagnostic redaction; durable snapshot publication/reopen and
`WorkerLost` recovery; graph order, invalid-edge, cycle, unreachable-node, and
blocked-dependent fixtures; verified artifact object/reference publication,
corrupt-object rejection, and conflicting-reference rejection; and a structured
local Cargo smoke and helper-CLI check/build/test-compilation that never invoke a shell.

Benchmarks and hardware: no performance claim in the first slice; local Cargo
toolchain only. Remote, sandbox, and named-hardware profiles are explicitly not
run.

Captures/traces/recovery evidence: local deterministic event timelines cover
queued through terminal states; cancellation-before-spawn and malformed-event
tests pass. The durable local store reopens interrupted work as `WorkerLost`,
persists that recovery before exposing it, and rejects late success. The adapter
drains stdout and bounded stderr concurrently so failed local Cargo runs return
a typed redacted process diagnostic. Process supervision, child-process-group
termination, automatic Cargo-artifact qualification, and durable build-wide
provenance remain required before package completion. The local
graph proof is only Cargo metadata -> check/build; it makes no parallelism, resource,
cache, or non-Cargo adapter claim. A host-selected `ArtifactStore` can copy one
bounded regular file into a BLAKE3-addressed object, verify a pre-existing
object, and atomically create a BuildId/node reference with declared schema and
tool identity. The store verifies copied bytes but does not prove that a Cargo
invocation produced its source path. Cargo build output is observable but is not yet automatically
selected, validated, or published into that store.

Accessibility: the service emits named stages, actionable typed diagnostics,
and cancellation/retry states for later Meridian UI consumption; no visible
panel is claimed by this package.

Security/provenance: command arguments are structured arrays; environment is
allowlisted; Windows-required `USERPROFILE` and `SYSTEMROOT` are explicit local
identity inputs rather than ambient fallback; Cargo output is untrusted input;
paths and diagnostic text are bounded and redacted; no secrets, credentials,
private-game paths, or shell concatenation are permitted. Artifact roots,
objects, references, and source files reject direct symlinks; existing objects
must hash to their content-addressed name and references never overwrite a
different BuildId/node result.

Migration/compatibility: no existing public or persistent format changes. The
new versioned protocol begins at v1 and is additive.

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
The metadata hash is a local Cargo payload hash rather
than a cross-checkout-normalized reproducibility identity. The local state and
artifact stores are host-selected foundations, not remote, signing, provider, or
cache-policy stores. The current graph scheduler is single-host, dependency-only,
and restricted to the Cargo metadata -> check/build proof. Concurrent/resource-aware
build-DAG scheduling, long-lived worker restart, lossless manifest editing,
rust-analyzer, remote execution, signing, and deployment remain planned.

Reviewers/sign-offs: Definition of Ready was reviewed against the current
workspace and MS-02 evidence. The current local implementation slice passes its
focused tests, durable-recovery tests, warning-denied lint, structured Cargo
smoke, and helper-CLI Cargo check/build/test-compilation. Completion requires fresh registered evidence
and the mandatory package sign-off before this package may close.

Next unblocked package: `WP-BLD-001` remains active until it closes; only then
may `WP-EDT-001` activate.

## 12. Evidence policy

Every run records source checkpoint, BuildId when available, corpus/build hashes, hardware, OS, backend, driver, capability profile, settings, cache/warmup state, distributions rather than averages alone, memory, artifacts, and missing features. Occluded structural evidence cannot satisfy visual quality. Unavailable hardware or capabilities are `NotRun`, `UnsupportedPlatform`, or `UnsupportedCapability`, never Pass.

No benchmark report may generalize beyond its measured workload/profile. No uncalibrated number becomes a release gate.

## 13. Mandatory package sign-off

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
