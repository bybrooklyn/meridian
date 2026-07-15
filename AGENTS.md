# Meridian Repository Agent Policy

version 0.5 · 2026-07-15

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
build-service foundations with local durable worker-loss recovery. The production
Penumbra Forward+ path, Creator Editor, Alluvium, native modeler,
Rust gameplay modules, optional Luau, animation, navigation, frameworks,
first-class 2D, Meridian Shader Language, Wavefront runtime, Collective,
Isobar, Basalt, Torsant, native backends, game prototype, and post-1.0 programs
remain incomplete.

`WP-UI-001` is `ImplementedFoundation` and MS-02 passed qualification on
GitHub Actions run `29457181283` for `fb8323f`: governance plus Linux, Windows,
and macOS workspace, UI-headless, UI-free runtime, and dependency-audit rows
passed. `WP-BLD-001` is the sole active package once PLANNING records its
Definition of Ready. Alluvium architecture is adopted but implementation remains
`Planned`; do not activate a PRC package unless PLANNING records its Definition
of Ready.

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
