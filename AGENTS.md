# Meridian Repository Agent Policy

version 0.5 · 2026-07-16

This file governs coding and documentation work. [The v0.5 master specification](specs/MERIDIAN_MASTER_SPEC.md) is the architecture index; [the delivery roadmap](specs/DELIVERY_ROADMAP.md) owns milestone order; [implementation planning](specs/IMPLEMENTATION_PLANNING_SPEC.md) owns package readiness, completion, concurrency, and replanning; [PLANNING.md](PLANNING.md) owns current evidence and the active bounded work package.

## 1. Authority and conflicts

Use the authority order in the master specification. Do not resolve conflicts silently. Update the owning spec, adopted ADR, typed registry, contradiction/migration record, validation contract, and then PLANNING.

Documentation maturity, implementation maturity, and evidence status are independent. Never convert `Planned`, `Research`, `Deferred`, `Scaffold`, `StructuralSmoke`, `Occluded`, or `DefinitionOnly` into an implementation or quality claim.

## 2. Current scope

`MS-00` and `MS-01` have passed. `WP-REL-002` passed qualification on GitHub
Actions run `29452928922`: governance plus the Linux, Windows, and macOS
workspace/headless-smoke rows all passed for `010db80`. The repository has runtime, RHI,
render-graph, direct PBR, cascaded-shadow, diffuse-irradiance IBL,
extraction/upload, asynchronous capture, transactional source import, package,
streaming, save-recovery, physics-wrapper, and bounded Cargo metadata/JSON
build-service foundations with local durable worker-loss recovery and a
dependency-validated Cargo graph. The production
Penumbra Forward+ path, Creator Editor, Alluvium, native modeler,
Rust gameplay modules, optional Luau, animation, navigation, frameworks,
first-class 2D, Meridian Shader Language, Wavefront runtime, Collective,
Isobar, Basalt, Torsant, native backends, game prototype, Marquee, and other post-1.0 programs
remain incomplete.

`WP-UI-001` is `ImplementedFoundation` and MS-02 passed qualification on
GitHub Actions run `29457181283` for `fb8323f`: governance plus Linux, Windows,
and macOS workspace, UI-headless, UI-free runtime, and dependency-audit rows
passed. `WP-BLD-001` is `ImplementedFoundation` after GitHub Actions run
`29505405013` for `becef55486d434460c3afebfb96e734655dfcb09` passed governance
and the Linux, Windows, and macOS workspace/BLD rows. `WP-EDT-001` is
`ImplementedFoundation`: GitHub Actions run `29605881704` for `4463bad` passed
governance and the complete Linux, Windows, and macOS rows for the persistent
Creator hub, source-authoritative project persistence, native picker boundary,
Creator journey, and universal macOS bundle. That evidence does not qualify the
Meridian UI 1.0 framework, production shell, platform accessibility, or visual
quality. `WP-UI-002` through `WP-UI-004` are
`ImplementedFoundation` after GitHub Actions run `30733414227` for `6d27fd5`
passed governance and the complete Linux, Windows, and macOS rows. `WP-UI-005`
remains `Partial` because its human visual, real screen-reader, and calibrated
renderer evidence is still absent; `WP-UI-006` remains the active authored-
source package behind that dependency. Earlier runs `29611418454`,
`29621896632`, and `29622972884` concluded `failure` before executing
implementation steps because the account could not allocate hosted runners;
their skipped rows are historical `NotRun` evidence. `ADR-0029` decided `RG-UI-001` for a Penumbra-owned direct display-list
path while retaining the current CPU raster bridge as structural/recovery only.
The temporary `WVR-UI-001` and `WVR-EDT-001` waivers are closed after the
green hosted matrix. `WP-UI-005` reached its bounded local source stop point but
remains unqualified for human visual, accessibility, and calibrated renderer
evidence. `WP-UI-006` is the sole active source-only package: it makes the existing
`UiDocument` an ergonomic authored source and compiles it into the existing
renderer-neutral frame contract. `WP-EDT-002` is `Partial` and suspended at its
recorded local source state until `WP-UI-006` completes; `WP-EDT-003` then
composes the remaining current Creator workspaces with truthful domain states.
No evidence treats WP-UI-005, WP-UI-006, or MS-03 as fully qualified.
`WP-PRC-001` is `ImplementedFoundation` after GitHub Actions run
`29511174569` for `9c88cc152878b1eb22f18c236c00ad1abd984fa5` passed governance
and the Linux, Windows, and macOS workspace rows. `WP-MDL-001` is `Partial`:
its MS-03 bounded editable-model foundation remains available to later editor
composition, while the broader modeler and Alluvium programs remain incomplete.
MS-03 remains open until `WP-UI-005`, `WP-UI-006`, `WP-EDT-002`, native
accessibility evidence, and visible application review pass.

## 3. Repository and private-game boundary

- Engine crates never depend on game or editor products.
- The ignored `game/` directory is a separate proprietary repository. Never stage its files or nested Git metadata.
- Do not copy private route, narrative, art, assets, logos, documents, or content into engine code, docs, tests, or benchmarks.
- Engine records may use generated/redacted contracts and private source hashes. `PEN-B04` is a generated AMI interior surrogate only.
- Project Meridian creative authority remains `bybrooklyn/project-meridian`; engine architecture remains here.

## 4. Architecture rules

- Third-party types stop at adapters; wgpu, Rapier, bevy_ecs, egui, AccessKit, Cargo, Jujutsu, provider SDKs, and agent-host types do not become stable Meridian APIs.
- Stable persistent IDs cross source/save/package/network boundaries; generational handles stay process-local.
- Cross-domain mutation uses typed commands and barriers; render/audio/workers consume immutable snapshots.
- Disabled optional packs add no tasks, threads, listeners, allocations, GPU resources, panels, dependencies, or package chunks.
- Source documents are authoritative; artifacts and compiled chunks are rebuildable caches.
- Penumbra shared systems remain path-independent. Experimental renderer paths are development/benchmark/test/debug only until promotion gates pass.
- Alluvium owns recipes, procedural evaluation, generated identity, overrides, provenance, and cooking. Runtime subsystems retain live authority, and baked-only games incur no Alluvium runtime cost.
- The user-facing creator product is one application named Meridian. Do not introduce separate Meridian Studio/IDE products; bounded helper processes and CLI tools are allowed.
- Rust gameplay is implemented before optional Luau. `WP-GAM-002` cannot block the Project Meridian prototype or opening slice.
- The native modeler is core, Blender remains optional, and topology-changing edits preserve or explicitly orphan stable element identity.
- Wavefront owns audio/device/DSP behavior; Collective owns modular online-service policy/provider seams; NET owns transport/replication. No hosted Meridian service is assumed.
- First-class 2D uses dedicated Penumbra and Cairn paths. NAV owns traversability/queries, not game decisions. Text and graph shaders share one ShaderIr.
- Marquee is a deferred post-1.0 local promotional export system. It imports manual approved captures, never publishes or manages accounts, and permits optional AI only for non-authoritative text/analysis suggestions; no audiovisual AI generation or modification.
- `PRG-REL-001` is deferred until after MS-10. It may seek and prove only
  workload/version/profile-specific performance or quality leadership through
  matched, raw, expiring evidence; it cannot guarantee global superiority or
  block/promote a milestone.
- Penumbra owns shared environmental media rendering; Isobar and Torsant retain
  sparse/multirate simulation authority and transfer dynamic surface-water
  ownership through a typed handoff; Alluvium owns authored combustion/fluid
  facets and derived cost predictions, not live state.
- `PRG-*` records are post-1.0 only and cannot satisfy, block, or promote `MS-*` milestones. Domain codes are governance IDs, not mandatory code names.

## 5. Delivery discipline

- Work in bounded pre-1.0 `WP-*` slices tied to `REQ-*`, `RG-*`, `MS-*`, and evidence classes. Post-1.0 work uses separately gated `PRG-*` records.
- Apply the Definition of Ready and Definition of Done in `specs/IMPLEMENTATION_PLANNING_SPEC.md`; an isolated package pass does not replace its milestone integration checkpoint.
- Keep one primary active package per working context. Parallel lanes require disjoint write authority and a named convergence package or review.
- Preserve unrelated dirty changes.
- Do not commit, push, tag, publish, deploy, message externally, or change credentials unless explicitly authorized.
- A milestone cannot complete from scaffolds, marker types, constructors, definitions, or documentation alone.
- Unsupported hardware/capability is an explicit evidence status, never a silent skip or Pass.
- Do not claim visible quality from occluded or structural GPU smoke.
- Do not invent numeric budgets, hardware support, competitor superiority, or benchmark conclusions.

## 6. Cargo, Rust, generated code, and provenance

Respect the workspace toolchain and lockfile. Prefer workspace dependencies and existing utilities. The `meridian-spec` tool is the only package in this pass authorized to add its bounded tool-only dependency set.

Unsafe Rust is denied by default. Generated files identify generator, input schema, version, and regeneration command. Public APIs use Meridian-owned descriptors/errors/handles. MS-01 additionally permits the registered `blake3` data-integrity boundary and `png` benchmark-capture encoder; other dependency additions require an owning `DEP-*` decision. Donor or borrowed code requires exact source, revision, hash, SPDX identity, notices, modifications, owner, tests, and exit/update strategy under `third_party/provenance/` before use.

During long Rust implementation sessions, run `cargo clean` often enough to keep
incremental artifacts from becoming stale or exhausting local disk. Never launch,
package, smoke, capture, or otherwise summon the native Meridian app unless the
user explicitly asks for that action; prefer source and non-windowed validation
while the user is working in the application.

## 7. Required validation

Run targeted tests first, then proportional gates:

~~~text
cargo run -p meridian-spec -- check
cargo metadata --locked
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p meridian-rhi --example clear_frame
cargo run -p meridian-renderer --example instance_upload_smoke
git diff --check
~~~

Also audit links, stale identifiers/filenames, secrets, personal paths, private content, tracked `game/` paths, and oversized files. Record unavailable native surfaces honestly.

## 8. Security, accessibility, and recovery

Treat project/import/save/package/shader/script/mod/network/VCS/build/provider/agent input as untrusted. Validate limits before allocation. No shell-concatenated processes, ambient authority, plaintext secrets, hidden listeners, or unredacted sensitive data.

Every visible workflow defines keyboard/focus/semantic/error behavior, scaling/contrast/motion implications, and an accessible recovery path. Agent output uses the same typed commands, permissions, preview, transaction, audit, undo, and rollback as human tools.

## 9. Research and waivers

Research gates preregister candidates, shared corpus, hardware/profile, metrics, material threshold, deadline, owner, stable seam, security/accessibility/licensing review, and losing-prototype archive. A production decision requires evidence and an ADR.

Waivers require the subsystem owner, validation/release approval, expiration, blocked milestone, and remediation package. Waivers cannot promote maturity. False implementation claims, private leakage, missing identifiers, or invalid schemas are unwaivable.

## 10. Mandatory package sign-off

Use the sign-off template in [PLANNING.md](PLANNING.md). Do not mark complete when a required row lacks evidence or a valid scoped, expiring waiver.
