# Cargo, IDE, Build, and Team Workflows

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Architecture](REPOSITORY_AND_CRATE_ARCHITECTURE.md) · [VCS](VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md)

Version 0.2 · 2026-07-14 · Normative · Current Cargo workspace foundation, build service Planned

## 1. Goals and non-goals

Cargo manifests and lockfiles remain authoritative for Rust. Meridian provides lossless project editing, rust-analyzer integration, and one observable/cancellable build DAG spanning Rust, shaders, assets, logic, UI, tests, packages, signing, and deployment.

Goals: beginners build without terminal knowledge; experts retain Cargo/rustc detail; every artifact maps to immutable inputs/tool versions; stale events/results cannot corrupt a newer build; workers restart safely; teams share reproducible profiles/evidence.

Non-goals: replacing Cargo or rust-analyzer, rewriting TOML destructively, scraping human terminal text as the primary protocol, storing secrets in profiles, or treating remote workers as trusted.

## 2. Ownership and processes

- meridian-project: project/workspace/profile source documents and Cargo mapping.
- meridian-build: build graph, scheduler, operation state, cache/provenance.
- meridian-build-protocol: versioned editor/service/worker messages.
- meridian-cargo: cargo metadata, JSON messages, rustc artifacts, tests.
- meridian-ide: language-server/session and source navigation contracts.
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

Node kinds include Cargo check/build/test/doc, shader validate/compile/reflect, asset import/facet/variant, world/UI/logic compile, package, sign, install, launch, benchmark, and evidence assemble.

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

## 8. Team workflows

Project-defined tasks include build, run, play, test, benchmark, package, and validate profiles. Personal UI/layout/tool paths remain local overlays. Shared profiles prohibit machine-specific absolute paths and secret values.

Build/evidence manifests let a teammate reproduce or explain differences. Remote workers negotiate platform/toolchain/capabilities, receive content-addressed bounded inputs, and return signed/verified outputs under trust policy.

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

CLI and MCP submit the same BuildRequest and consume the same event stream.

## 10. Memory, scheduling, and cache

Build nodes declare CPU, memory, IO, GPU, process, and license-token resources. Scheduler respects interactive runtime priority and user deadlines. Cache has quotas by project/tool/domain, LRU/rebuild cost, integrity verification, and inspect/prune controls.

Large artifacts are immutable and streamed/range-addressed. A worker cannot publish output until hash/schema/tool result validates.

## 11. Security

- command arguments are structured arrays, never shell-concatenated strings;
- environment variables use allowlists and secret references;
- remote workers never receive unrelated project content or signing keys;
- logs/diagnostics redact secrets;
- downloaded tools/SDKs have pinned version/hash/license/provenance;
- signing is a separate least-privilege operation;
- compiler/build output is treated as untrusted until validated.

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

## 14. Phases

Phase 1 establishes workspace/CI rules. Phase 5 supplies domain artifact graph inputs. Phase 16 delivers build service, Cargo/IDE integration, team profiles, and evidence output. Phases 17–18 integrate source/sync checkpoints. Phase 29 certifies reproducibility and selected remote/signing profiles.

## 15. Examples

End-to-end: Export resolves Cargo/profile/assets/shaders/logic, reuses valid nodes, compiles changed nodes, packages/signs under profile, emits BuildId, installs, launches, and links every diagnostic/artifact to inputs.

Failure/recovery: shader worker crashes after writing a partial file. No artifact pointer commits; build service marks WorkerLost, restarts, retries from immutable inputs, and preserves prior valid shader artifact.

Performance debug: slow export opens critical-path view showing one asset import serializing unrelated shader work. The owner fixes dependency/resource declarations and compares BuildIds/traces on the same clean cache corpus.
