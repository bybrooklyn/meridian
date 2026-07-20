# Cargo, IDE, Build, and Team Workflows

[Master](MERIDIAN_MASTER_SPEC.md) · [ADR-0018](../docs/architecture/decisions/ADR-0018-general-purpose-single-application.md) · [ADR-0031](../docs/architecture/decisions/ADR-0031-managed-development-toolchains.md) · [Architecture](REPOSITORY_AND_CRATE_ARCHITECTURE.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) · [Native modeler](NATIVE_MODELING_AND_DCC_SPEC.md) · [Shader language](MERIDIAN_SHADER_LANGUAGE_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Marquee](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md) · [VCS](VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md)

version 0.5 · 2026-07-18 · Normative · Current Cargo workspace foundation, build service Partial

Documentation maturity: `ImplementationReady`. Implementation maturity:
`ImplementedFoundation` for the bounded `WP-BLD-001` local Cargo service and
CLI slice; broader build graph, managed development-toolchain, and team-service
work remains planned. Governing IDs: `REQ-BLD-001`, `REQ-BLD-002`,
`WP-BLD-001`, `WP-BLD-002`.

## 1. Goals and non-goals

Cargo manifests and lockfiles remain authoritative for Rust. The one user-facing application is named **Meridian**; project management, editor, IDE, modeler, graph tools, debugger, profiler, build, VCS, and Play workflows are workspaces within it rather than separate Studio/IDE products. Meridian provides lossless project editing, rust-analyzer integration, and one observable/cancellable build DAG spanning Rust, shaders, assets, models, animation, logic, UI, tests, packages, signing, and deployment.

Goals: beginners build without terminal knowledge; experts retain Cargo/rustc detail; every artifact maps to immutable inputs/tool versions; stale events/results cannot corrupt a newer build; workers restart safely; teams share reproducible profiles/evidence.

Non-goals: replacing Cargo or rust-analyzer, creating separate Meridian Studio/IDE applications, rewriting TOML destructively, scraping human terminal text as the primary protocol, storing secrets in profiles, or treating remote workers as trusted.

After `PRG-PRM-001` activates, Marquee reuses BLD-owned bounded jobs, cancellation, worker isolation, artifact storage, and reproducibility records. BLD does not own campaign sources, claims, approval policy, export profiles, or service publishing, and current `WP-BLD-001` includes no Marquee implementation.

## 2. Ownership and processes

- meridian-project: project/workspace/profile source documents and Cargo mapping.
- meridian-build: current bounded editor-only service foundation for deterministic
  local BuildIds, typed operation/event state, bounded Cargo metadata with a
  checkout-independent workspace package/manifest/target identity component and
  redacted Cargo JSON mapping, bounded redacted Cargo process-failure stderr,
  structured Cargo checks/builds/test compilation, cancellation, a
  helper CLI, and a local durable state store. The CLI creates a unique default
  state file under workspace `target/meridian-build/`, prints its path before
  execution, removes only a successful default-owned file, and accepts `--state`
  for caller-owned recovery. The store snapshots each
  accepted `DurableBuildService` mutation through a synced temporary sibling and
  same-directory rename, then persists `WorkerLost` recovery before returning it
  to the host. `BuildGraph` now rejects invalid/mismatched Cargo dependency
  graphs, binds its canonical declared contract into `BuildId`, and deterministically
  schedules the local metadata -> check/build proof.
  External worker events are structurally revalidated before they can mutate or
  persist service state: the protocol version, artifact bounds, artifact-hash
  payload, diagnostic-to-payload correspondence, running-phase lifecycle use,
  request provenance, and secret redaction must all match the Meridian-owned
  event contract. This is a local validation boundary,
  not an authentication or sandbox claim for a worker.
  `CargoBuildSupervisor` owns one fallibly-created local worker for the bounded Cargo
  adapter. It admits only one exact registered running request at a time with a matching
  Cargo root node, retains cancellation, persists completion/cancellation, and
  maps a task panic or disconnect to `WorkerLost`. It is not a remote-worker
  protocol or separately supervised service process. Cargo's `build-finished`
  success value must agree with the persisted `Succeeded` or `Failed` terminal
  phase; contradictory local worker outcomes, external events, and snapshots
  are rejected before they can create a durable success claim. Concurrent/resource-aware
  DAG scheduling, cache policy, complete per-node result lineage, and external
  service-process/remote-worker supervision are assigned to planned `WP-BLD-002`.
  They do not enlarge the MS-03-local `WP-BLD-001` completion boundary. A host-selected `ArtifactStore` can validate/copy one bounded regular
  file into a BLAKE3-addressed object and atomically create a BuildId/node
  reference carrying declared schema and tool identity. Cargo-reported
  executable references additionally retain Cargo's package ID and target name.
  A running service can expose that verified hash as a typed artifact event only
  when the reference matches its BuildId, root node, and secret-safe request-input
  manifest. It does not prove that Cargo produced the source file. The helper can opt in to one Cargo-reported executable
  publication only after a successful build or test-compilation command, with
  paired `--artifact-store` and `--cargo-output-root` paths. It requires exactly
  one executable, requires that executable to be listed by Cargo, and rejects
  symlinked, non-regular, oversized, or output-root-escaping paths before an
  atomic publication/event. The reference retains the source checkpoint,
  resolved profile, metadata-plus-lock identity, toolchain/target, sorted roots,
  canonical graph-contract hash, the complete declared graph manifest, and hashes
  of ordered command arguments and each allowlisted environment value;
  it never retains those raw argument or environment values. General artifact
  selection, cache policy, and build-wide/remote provenance remain unimplemented. The current
  adapter concurrently drains stdout through a backpressured reader with aggregate
  byte and line limits, and drains bounded stderr; stderr becomes a typed
  process-failure diagnostic for a non-success Cargo status, not a compiler
  artifact or persistent log. A cancellation fallback can instead emit a typed
  descendant-recovery warning. On Unix each Cargo child starts in its own process
  group; cancellation invokes the fixed `/bin/kill` program with structured
  `TERM` and, after a 250 ms bounded grace, `KILL` signals for that child group.
  On Windows cancellation invokes the explicit
  `SystemRoot\System32\taskkill.exe /PID <cargo-pid> /T /F` tree terminator.
  Neither path invokes a command shell or inherits the host environment. If the
  platform tree terminator cannot run, the service still reaps the direct Cargo
  child and emits a typed warning rather than claiming that descendants were
  terminated. A Unix induced test
  proves cancellation of a Cargo build script that inherits a `/bin/sleep 60`
  child. GitHub Actions run `29505405013` for
  `becef55486d434460c3afebfb96e734655dfcb09` passed the configured Windows
  workspace, helper-smoke, bounded Cargo-build, and independent test-artifact
  rows after entering the Visual Studio developer environment. That evidence
  does not convert Unix process-group cancellation into a Windows-specific
  cancellation claim.
  The prepared Windows CI smoke discovers the installed C++ toolchain through
  `vswhere`, invokes that installation's `VsDevCmd.bat` for amd64 host/target,
  and then lets the adapter snapshot only its explicit environment allowlist.
  The configuration is exercised by the passing Windows source checkpoint above;
  it remains an explicit local-process setup rather than ambient inheritance.
  A helper launched through `cargo run` on Windows cannot relink its own active
  `meridian-build.exe`; the documented structural build and test-compilation
  proofs therefore target the independent `meridian-core` library. The
  test-compilation proof also uses an explicit separate Cargo target directory,
  so its linker does not contend with the helper's active target tree.
  Self-rebuild orchestration remains outside this bounded slice.
- meridian-build-protocol: versioned editor/service/worker messages.
- meridian-cargo: cargo metadata, JSON messages, rustc artifacts, tests.
- meridian-ide: language-server/session, code intelligence, debugger, test, profiler, and source-navigation contracts used inside Meridian.
- domain compiler adapters: shader, asset, UI, logic, package/signing.

The editor invokes a long-lived build service process. Import/compiler/signing/remote work runs in narrower supervised workers. Runtime crates do not depend on Cargo/IDE crates.

## 3. Authoritative manifests

Cargo.toml and Cargo.lock are source authority. Editing uses a lossless syntax tree preserving comments, ordering, formatting, workspace inheritance, target tables, unknown keys, and user expressions. A semantic edit generates a preview and minimal patch.

Meridian project metadata lives under namespaced Cargo package/workspace metadata only where Cargo semantics permit; engine-specific project documents remain separate and reference package IDs from cargo metadata.

## 3.1 Managed development toolchains

`REQ-BLD-002`: Meridian manages development tools as externally installed,
versioned components. `rustc`, Cargo, rust-analyzer, each selected platform SDK,
each debugger or debugger adapter, and each external shader compiler are
separately installed, addressable components; none is bundled into the Meridian
main binary. An upstream distribution may be acquired as one archive only when
its component records, installed roots, versions, hashes, licenses, and
activation state remain separately inspectable and replaceable.

A project owns an exact compatible toolchain lock. It names every required
component by component kind, provider, version, platform/architecture,
artifact hash, license/provenance record, and compatibility profile. Platform
SDK, debugger, and external shader-compiler entries are mandatory whenever the
selected project profile requires them; an unavailable, incompatible, or
unverified entry is a typed blocked-toolchain outcome, never a fallback to an
ambient host installation. BuildId and evidence records retain the resolved
component manifest hash and exact component versions. A project pin changes
only through an explicit, previewable project transaction after compatibility
verification; a Meridian, channel, or default-toolchain update MUST NOT
silently rewrite it.

The Meridian CLI and editor expose the same manager operations: discover,
install, verify, update, repair, select, pin, compare, rollback, list, and
clean up. They invoke a bounded toolchain-management service rather than
embedding compilers or SDKs in the application binary. Discovery reports the
component source, trust state, compatibility, license/notice record, disk use,
projects that pin it, and whether it is active, staged, corrupt, or removable.
The editor supplies the same information through accessible status, recovery,
and confirmation workflows; it never requires a terminal for the normal path.

Install, update, and repair download into a quarantined staging generation,
verify declared length, cryptographic hash, signature/provenance policy,
component identity/version, compatibility, and required license/notice record
before use, then perform any bounded health check. Only a complete verified
generation may become selectable, using an atomic same-store activation. The
previous verified generation remains available until health and recovery policy
allow its removal. Interrupted, corrupt, or incompatible staging is quarantined
and cannot change the active generation. A repair reinstalls or reconstructs
the exact pinned component bytes; it does not substitute a newer compatible
version.

Multiple verified versions coexist in separate managed roots. Rollback selects
a retained verified generation and records the reason; it never weakens trust
metadata or changes a project's lock without the explicit pin transaction.
Cleanup is quota-aware and previewable, and MUST retain every version pinned by
a local project, active operation, retained rollback generation, or required
provenance/evidence record. License tracking retains component identity,
version, artifact hash, SPDX or vendor license expression, notice location,
source/provenance, acceptance or restriction state where applicable, and the
projects/builds that used it. A missing or changed license record blocks
activation rather than being silently carried forward.

`WP-BLD-001` does not implement this manager: its local host toolchain remains
an explicitly recorded input only. `WP-BLD-002` owns the planned managed
component store, project lock resolution, compatibility matrix, user-facing
recovery flows, and evidence. This section is normative architecture, not a
claim that the current CLI or editor can install or update toolchains.

## 4. Build identity and graph

~~~text
BuildId = hash(
  source_checkpoint,
  resolved_project_profile,
  cargo_metadata_and_lock,
  build_graph_contract,
  command_arguments,
  toolchain_versions,
  target_and_capabilities,
  environment_allowlist,
  root_node_ids
)

BuildNode {
  id, kind, input_hashes, tool_id_version,
  declared_environment, outputs,
  resources, sandbox, cache_policy, dependencies
}
~~~

For the current Cargo adapter, the metadata component preserves an exact bounded
payload hash for traceability and separately hashes the sorted workspace package
name/version, workspace-relative manifest path, and target contract. The latter
hash combines with `Cargo.lock` in `BuildId`, so relocating one checkout does
not alone alter this component. Source checkpoint, toolchain, target, and
allowlisted host environment remain deliberate local identity inputs; this is
not yet a full cross-machine reproducibility claim. Before `WP-BLD-002`, this
field is the current local host toolchain identity, not a managed project lock.
After that package, it is the exact verified component manifest described in
section 3.1. The current
`build_graph_contract` is a canonical hash of requested roots and each declared
node ID, kind, tool, input hashes, environment names, and dependency topology.
It prevents a changed declared graph from reusing a `BuildId`. New requests and
request-bound artifact references also retain the canonical declared graph
manifest. This is not yet a per-node result lineage or complete build-wide
provenance record.

Node kinds include Cargo check/build/test/doc, Meridian Shader Language parse/IR/target/reflect, asset import/facet/variant, native model validate/modifier/derive/interchange, animation import/compress/build, Alluvium recipe validate/migrate/evaluate/bake/provenance/license-audit, world/UI/logic compile, package, sign, install, launch, benchmark, and evidence assemble.

The graph is content-addressed. Timestamps may optimize discovery but never establish correctness.

## 5. Operation lifecycle

~~~text
Queued -> Resolving -> Ready -> Running -> Succeeded
                           \-> CancelRequested -> Cancelled
any -> Failed | WorkerLost | Superseded
~~~

Every event carries BuildId, OperationId, NodeId, sequence, phase, progress, diagnostic, artifact hash, and trace ID. The editor ignores stale BuildId/sequence. Cancellation is cooperative then process-enforced; partial outputs are unreferenced until atomic validation/commit.

## 6. Cargo integration

Use cargo metadata for workspace/package/target/features/dependencies and Cargo JSON messages for compiler artifacts/diagnostics/build scripts/tests. Preserve full rendered diagnostics while mapping spans, codes, suggestions, related information, and child messages into typed events.

Profiles declare target, features, platform, engine feature packs, environment policy, package/signing/deploy, and test/benchmark sets. Feature resolution previews duplicate versions and unintended optional packs.

Build-script/proc-macro output is untrusted and attributed. Sandboxing is platform/tier dependent and reported honestly.

## 7. IDE integration

rust-analyzer remains the Rust semantic engine. Meridian manages one session per compatible workspace/toolchain profile, forwards document versions, maps diagnostics/code actions/symbols/references/formatting, and recovers after process loss.

Editor code actions become previews and normal file/VCS transactions. Generated sources are read-only with provenance and source mapping. Different build/profile diagnostics are labeled; they do not overwrite each other.

Meridian's IDE baseline includes project/workspace navigation, code editing, semantic completion/navigation, diagnostics, formatting, safe refactors/code actions, build/test/run/debug controls, breakpoints, stack/locals with redaction, structured terminal/process views, generated-source/source-map navigation, profiler links, and documentation. Rust is first. Optional Luau support follows `WP-GAM-002` and cannot fork command, breakpoint, source-map, or capability semantics.

The complete IDE is not one monolithic process. rust-analyzer, compilers, debugger adapters, importers, Alluvium workers, model operations, and remote workers may be bounded helper processes. The user still experiences one Meridian application and one project/session/permission model.

## 8. Team workflows

Project-defined tasks include build, run, play, test, benchmark, package, and validate profiles. Personal UI/layout/tool paths remain local overlays. Shared profiles prohibit machine-specific absolute paths and secret values.

Build/evidence manifests let a teammate reproduce or explain differences. Remote workers negotiate platform/toolchain/capabilities, receive content-addressed bounded inputs, and return signed/verified outputs under trust policy.

Remote development separates source authority, file synchronization, semantic service, build execution, runtime/debug target, artifact transport, and secrets. Disconnects preserve local source and durable checkpoints. Remote work is optional, provider-neutral, capability-declared, and never a Meridian-hosted-service requirement.

## 9. Beginner and expert UX

Beginner:

1. choose Run or Export;
2. see named stages and actionable diagnostic;
3. use Fix/Learn where command is safe;
4. cancel/retry without corrupting source;
5. receive an installable output and known limitations.

Expert:

- inspect Cargo feature/dependency graph;
- edit lossless manifest diff;
- view full rustc/Cargo output;
- inspect build node inputs/cache/provenance/resources;
- pin toolchain/target/environment;
- run one node or compare BuildIds;
- export trace/evidence.
- move from model element, shader source span, animation node, gameplay system, or world object to the responsible build node and generated artifact.

CLI and MCP submit the same BuildRequest and consume the same event stream.

## 10. Memory, scheduling, and cache

Build nodes declare CPU, memory, IO, GPU, process, and license-token resources. Scheduler respects interactive runtime priority and user deadlines. Cache has quotas by project/tool/domain, LRU/rebuild cost, integrity verification, and inspect/prune controls.

Large artifacts are immutable and streamed/range-addressed. A worker cannot publish output until hash/schema/tool result validates.

## 11. Security

- command arguments are structured arrays, never shell-concatenated strings;
- environment variables use allowlists and secret references;
- Cargo children start with no inherited environment. The explicit common
  allowlist contains Cargo/rustup roots, temporary-directory roots, and `PATH`.
  On Windows it also admits the Visual Studio developer-environment linker and
  SDK search roots (`INCLUDE`, `LIB`, `LIBPATH`, `VCINSTALLDIR`,
  `VCToolsInstallDir`, `VSINSTALLDIR`, `WindowsSdkDir`,
  `WindowsSDKVersion`, `UniversalCRTSdkDir`, `UCRTVersion`,
  `VSCMD_ARG_HOST_ARCH`, `VSCMD_ARG_TGT_ARCH`, and `VSCMD_VER`) when present.
  Those values are bounded,
  explicit local BuildId inputs, not ambient fallback or portable provenance;
- remote workers never receive unrelated project content or signing keys;
- logs/diagnostics redact secrets;
- untrusted worker events are revalidated before lifecycle state can advance or
  enter the durable local snapshot; malformed, mismatched, or unredacted values
  are rejected rather than rewritten silently;
- downloaded tools/SDKs use the managed component policy in section 3.1;
- signing is a separate least-privilege operation;
- compiler/build output is treated as untrusted until validated.
- Alluvium external tools and kernels receive only declared content-addressed
  inputs and cannot publish until output schema, provenance, license, budget,
  and determinism policy pass.

## 12. Diagnostics and recovery

Report graph critical path, node state/duration/cache, queue/resource waits, process/worker, diagnostics, artifact provenance, invalidation reason, memory/IO, cancellation, superseded events, and reproducibility deltas.

Build service crash recovery reopens the operation database, validates committed artifacts, marks running nodes interrupted, and permits resume/retry. Source and prior valid artifacts remain intact.

## 13. Tests and benchmarks

- lossless TOML round-trip/property corpus and minimal semantic patch;
- Cargo metadata/JSON fixture versions and malformed events;
- stale/superseded event rejection;
- malformed, mismatched, and unredacted external-worker-event rejection before
  durable acceptance;
- cancellation at each node lifecycle point;
- worker crash, protocol mismatch, partial artifact, cache corruption;
- rust-analyzer restart/document-version races;
- managed-component install/verify/update/repair/rollback/cleanup, including
  interrupted staging, hash/signature/license mismatch, side-by-side selection,
  pinned-project non-mutation, and referenced-version retention;
- clean versus incremental rebuild correctness;
- feature/minimal/default/all-profile dependency gates;
- build latency/critical path/cache hit/memory/IO on named corpus.

## 14. Delivery mapping

MS-01 establishes workspace/CI rules. MS-01/MS-03/MS-04 supply domain artifact
graph inputs. `WP-BLD-001` is the MS-03 local-Cargo prerequisite: one bounded
observable operation, a verified optional executable, durable local recovery,
and the editor command/event seam. `WP-PRC-001` integrates Alluvium validation
and baking with the same observable build graph before MS-05. Planned
`WP-BLD-002` is the MS-08 continuation for managed development toolchains,
multi-node result lineage, general artifact/cache policy, service-process and
remote-worker supervision, team profiles, and broader evidence output; it
cannot block `WP-BLD-001` or MS-03.
MS-08/MS-09 integrate source/sync checkpoints, the native modeler, animation,
ShaderIr, Rust gameplay, and optional Luau build adapters as their packages
activate. MS-10 certifies reproducibility and selected remote/signing profiles.

## 15. Examples

End-to-end: Export resolves Cargo/profile/assets/shaders/logic, reuses valid nodes, compiles changed nodes, packages/signs under profile, emits BuildId, installs, launches, and links every diagnostic/artifact to inputs.

Failure/recovery: shader worker crashes after writing a partial file. No artifact pointer commits; build service marks WorkerLost, restarts, retries from immutable inputs, and preserves prior valid shader artifact.

Performance debug: slow export opens critical-path view showing one asset import serializing unrelated shader work. The owner fixes dependency/resource declarations and compares BuildIds/traces on the same clean cache corpus.
