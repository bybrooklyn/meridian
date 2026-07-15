# Meridian Repository Agent Policy

This file governs coding and documentation work in this repository. [The v0.2 master specification](specs/MERIDIAN_MASTER_SPEC.md) is the architecture index; [PLANNING.md](PLANNING.md) is the current evidence and work-package tracker.

## 1. Authority and conflicts

Use this order:

1. specs/ version 0.2, with [the migration register](specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md) resolving old decisions.
2. PLANNING.md for current implementation evidence and the active bounded work package.
3. The private `bybrooklyn/project-meridian` creative suite for narrative, art, pacing, route, content, and game behavior.
4. The [v0.1 migration ledger](docs/migrations/V0_1_DOCUMENT_MIGRATION.md) for historical rationale and provisional hypotheses consolidated from deleted legacy documents.
5. code as evidence of what exists, not automatic permanent architecture.

Do not resolve a conflict silently. Record old/new/winner/reason/migration/source of truth in the migration register, update affected specs, then update PLANNING.

## 2. Current phase and scope

The repository is in partial Phase 2 after Phase 0 specification migration and engine/game repository separation. Current renderer foundations include PBR, cascaded shadows, and diffuse irradiance IBL. The next implementation package is pass-level CPU/GPU renderer timing and visible capture evidence, with unsupported or occluded surface outcomes recorded honestly.

Prefiltered specular IBL and BRDF LUT are future bounded Phase 2 work. Do not start advanced GI, visibility-buffer rendering, virtual geometry, game content expansion, Meridian UI implementation, Cairn fork work, multiplayer, XR, agents, or other later phases unless PLANNING explicitly activates that package.

When the user gives a narrower task, that scope wins.

## 3. Delivery discipline

- Work in small requirement/work-package slices.
- Preserve unrelated dirty changes.
- Do not commit, push, tag, publish, deploy, message externally, or change credentials without explicit user authorization.
- The ignored `game/` directory is a separate private repository. Never stage it, traverse its `.git`, or copy its closed-source creative content into engine commits.
- A phase cannot complete from scaffolds, marker types, constructors, or documentation alone.
- Before claiming completion, collect the evidence required by [validation](specs/TESTING_BENCHMARKS_AND_VALIDATION.md).
- Distinguish Implemented, Transitional, Planned, Research, Deferred, and Unsupported.
- Do not claim visible quality from an occluded/structural GPU smoke.
- Do not invent performance targets or competitor superiority.

## 4. Architecture rules

- Engine crates never depend on game or editor products.
- Third-party types stop at adapters. Do not expand egui, Rapier, bevy_ecs, wgpu, AccessKit, Cargo, Jujutsu, Ollama, or SDK types into stable Meridian APIs.
- Use stable persistent IDs for source/save/package/network data and generational handles in-process.
- Cross-domain mutation uses commands and declared barriers; render/audio/worker domains consume immutable snapshots.
- Optional packs add no tasks, threads, GPU resources, listeners, panels, dependencies, or package chunks when disabled.
- Source documents are authoritative; artifacts and compiled chunks are rebuildable caches.

## 5. Cargo, Rust, and generated code

- Respect the workspace rust-toolchain and Cargo.lock.
- Prefer workspace dependencies and existing utilities; add dependencies only for an activated requirement with license/provenance review.
- Run cargo fmt. Do not suppress warnings to pass clippy without documenting a justified exception.
- Unsafe Rust is denied by default. A required unsafe module needs the narrowest isolation, invariants in Safety comments, tests/fuzzing as applicable, and architecture/security review.
- Generated files identify generator, source schema, version, and regeneration command. Never hand-edit generated output.
- Public APIs use Meridian-owned descriptors/errors/handles, document stability, and include migration/deprecation policy before publication.

## 6. Required validation

Run the smallest targeted tests first, then proportional workspace gates. For the active renderer slice:

~~~text
environment-light targeted tests
cargo test --workspace
native renderer smoke with six-face upload and pipeline/bind-group evidence
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check plus untracked-file/link audit
~~~

For other work, use the matrix in the validation spec. Missing tool/platform/hardware is Not Run, not Pass. Save raw traces/captures/reports with source checkpoint, BuildId when available, corpus, hardware, software, profile, statistics, and known limits.

## 7. Formats, security, and provenance

- Treat project/import/save/package/shader/script/mod/network/VCS/build/provider/agent input as untrusted.
- Validate length/count/depth/path/hash/version before allocation or authority.
- No shell-concatenated process command, ambient filesystem/network access, plaintext secret, hidden listener, or unredacted secret in logs/traces/prompts.
- Destructive/trust-changing/external-production actions require preview, explicit authority, and checkpoint/rollback.
- Borrowed source and dependencies record exact source/revision/hash, SPDX identifier, notices, modifications, owner, tests, and exit/update strategy under third_party/ and licenses/.
- Cairn source migration cannot begin before its provenance gate.
- Agent or AI output is untrusted and uses the same typed commands and permissions as human tools.

## 8. Accessibility and documentation

Every user-visible workflow includes keyboard behavior, focus, semantic names/actions/errors, scaling/contrast/motion implications, controller behavior where relevant, and an accessible recovery path. AccessKit/platform APIs are adapters; Meridian semantics are authoritative.

Update API docs, examples, diagnostics, Ponder content, migration fixtures, and relevant specs with behavior changes. A code change that contradicts a spec is incomplete until the normative decision and migration register are updated.

## 9. Research

Research decisions use primary sources and the template in [research decisions](specs/RESEARCH_AND_ALGORITHM_DECISIONS.md): competing prototypes, shared corpus, hardware, metrics, deadline, owner, stable seam, security/accessibility/licensing, and losing-prototype archive. Record verification date and links. Do not turn a candidate into a production commitment without an ADR and evidence.

## 10. Checkpoints and version control

Meridian VCS is planned, not implemented. Until it exists, use the current repository and Git status/diffs as inspection surfaces. Do not destroy or overwrite unrelated work. Only create commits when the user authorizes them; a commit message should name the phase/work-package and evidence. Project Meridian uses its own nested private repository under ignored `game/`.

When Meridian VCS exists, use ChangeId/OperationId/checkpoints and semantic diffs. Live collaboration never replaces a durable checkpoint.

## 11. Mandatory phase sign-off

Use this format:

~~~text
Phase/work package:
User-visible result:
Status:
Source checkpoint and BuildId:
Requirements:
Files/crates/formats changed:
Tests:
Benchmarks and hardware:
Captures/traces/recovery evidence:
Accessibility:
Security/provenance:
Migration/compatibility:
Documentation:
Known limits and unsupported rows:
Reviewers/sign-offs:
Next unblocked package:
~~~

Do not mark complete when a required row lacks evidence or an explicit, scoped, expiring waiver.
