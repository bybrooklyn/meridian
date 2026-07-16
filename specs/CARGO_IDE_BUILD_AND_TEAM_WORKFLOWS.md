# Cargo, IDE, Build, and Team Workflows

[Master](MERIDIAN_MASTER_SPEC.md) · [ADR-0018](../docs/architecture/decisions/ADR-0018-general-purpose-single-application.md) · [Architecture](REPOSITORY_AND_CRATE_ARCHITECTURE.md) · [Native modeler](NATIVE_MODELING_AND_DCC_SPEC.md) · [Shader language](MERIDIAN_SHADER_LANGUAGE_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [VCS](VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md)

version 0.5 · 2026-07-15 · Normative · Current Cargo workspace foundation, build service Partial

Documentation maturity: `ImplementationReady`. Implementation maturity:
`Partial` with current Cargo/CI foundations and a bounded `meridian-build`
service/CLI slice. Governing IDs: `REQ-BLD-001`,
`WP-BLD-001`.

## 1. Goals and non-goals

Cargo manifests and lockfiles remain authoritative for Rust. The one user-facing application is named **Meridian**; project management, editor, IDE, modeler, graph tools, debugger, profiler, build, VCS, and Play workflows are workspaces within it rather than separate Studio/IDE products. Meridian provides lossless project editing, rust-analyzer integration, and one observable/cancellable build DAG spanning Rust, shaders, assets, models, animation, logic, UI, tests, packages, signing, and deployment.

Goals: beginners build without terminal knowledge; experts retain Cargo/rustc detail; every artifact maps to immutable inputs/tool versions; stale events/results cannot corrupt a newer build; workers restart safely; teams share reproducible profiles/evidence.

Non-goals: replacing Cargo or rust-analyzer, creating separate Meridian Studio/IDE applications, rewriting TOML destructively, scraping human terminal text as the primary protocol, storing secrets in profiles, or treating remote workers as trusted.

## 2. Ownership and processes

- meridian-project: project/workspace/profile source documents and Cargo mapping.
- meridian-build: current bounded editor-only service foundation for deterministic
  local BuildIds, typed operation/event state, bounded Cargo metadata and
  redacted Cargo JSON mapping, bounded redacted Cargo process-failure stderr,
  structured Cargo checks/builds/test compilation, cancellation, a
  helper CLI, and a host-selected local durable state store. The store snapshots each
  accepted `DurableBuildService` mutation through a synced temporary sibling and
  same-directory rename, then persists `WorkerLost` recovery before returning it
  to the host. `BuildGraph` now rejects invalid/mismatched Cargo dependency
  graphs and deterministically schedules the local metadata -> check/build proof.
  Concurrent/resource-aware DAG scheduling, cross-checkout identity
  normalization, cache/provenance persistence, and worker supervision remain
  planned. A host-selected `ArtifactStore` can validate/copy one bounded regular
  file into a BLAKE3-addressed object and atomically create a BuildId/node
  reference carrying declared schema and tool identity. Cargo-reported
  executable references additionally retain Cargo's package ID and target name.
  A running service can expose that verified hash as a typed artifact event only
  when the reference matches its BuildId and root node. It does not prove that
  Cargo produced the source file. The helper can opt in to one Cargo-reported executable
  publication only after a successful build or test-compilation command, with
  paired `--artifact-store` and `--cargo-output-root` paths. It requires exactly
  one executable, requires that executable to be listed by Cargo, and rejects
  symlinked, non-regular, oversized, or output-root-escaping paths before an
  atomic publication/event. General artifact selection, cache policy, and
  remote provenance remain unimplemented. The current
  adapter concurrently drains stdout and bounded stderr; stderr becomes a typed
  process-failure diagnostic only for a non-success Cargo status, not a compiler
  artifact or persistent log.
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

## 4. Build identity and graph

~~~text
BuildId = hash(
  source_checkpoint,
  resolved_project_profile,
  cargo_metadata_and_lock,
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
- Windows Cargo/rustup execution receives only explicit `USERPROFILE` and
  `SYSTEMROOT` host roots when present; both are local BuildId inputs rather than
  ambient fallback;
- remote workers never receive unrelated project content or signing keys;
- logs/diagnostics redact secrets;
- downloaded tools/SDKs have pinned version/hash/license/provenance;
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
- cancellation at each node lifecycle point;
- worker crash, protocol mismatch, partial artifact, cache corruption;
- rust-analyzer restart/document-version races;
- clean versus incremental rebuild correctness;
- feature/minimal/default/all-profile dependency gates;
- build latency/critical path/cache hit/memory/IO on named corpus.

## 14. Delivery mapping

MS-01 establishes workspace/CI rules. MS-01/MS-03/MS-04 supply domain artifact
graph inputs. `WP-PRC-001` integrates Alluvium validation and baking with the
same observable build graph before MS-05. MS-03/MS-08 deliver build service,
Cargo/IDE integration, team profiles, and evidence output. MS-08/MS-09 integrate
source/sync checkpoints, the native modeler, animation, ShaderIr, Rust gameplay,
and optional Luau build adapters as their packages activate. MS-10 certifies
reproducibility and selected remote/signing profiles.

## 15. Examples

End-to-end: Export resolves Cargo/profile/assets/shaders/logic, reuses valid nodes, compiles changed nodes, packages/signs under profile, emits BuildId, installs, launches, and links every diagnostic/artifact to inputs.

Failure/recovery: shader worker crashes after writing a partial file. No artifact pointer commits; build service marks WorkerLost, restarts, retries from immutable inputs, and preserves prior valid shader artifact.

Performance debug: slow export opens critical-path view showing one asset import serializing unrelated shader work. The owner fixes dependency/resource declarations and compares BuildIds/traces on the same clean cache corpus.
