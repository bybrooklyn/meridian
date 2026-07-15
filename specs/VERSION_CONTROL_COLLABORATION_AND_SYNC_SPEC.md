# Version Control, Collaboration, and Sync Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Build/team](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)

Version 0.2 · 2026-07-14 · Normative architecture · Planned

## 1. Decision

Meridian VCS uses Git-compatible object/remotes interoperability plus a Jujutsu-derived change and operation model for user experience, recovery, concurrent work, and undo. The exact fork/implementation is a provenance and research gate.

Telepo is not a product or separate subsystem. It is replaced by meridian-sync: direct encrypted peer-to-peer exchange first, optional self-hosted relay, no mandatory account, cloud, hosted Meridian service, or inbound port. Local sync state lives under .meridian/sync/.

Live collaboration augments immutable VCS operations/checkpoints. It never becomes authoritative history.

## 2. Goals and non-goals

Goals: understandable changes rather than staging/HEAD, operation-log undo, semantic diffs/merges, large/binary assets, partial workspaces, offline work, Git remotes, encrypted direct sync, crash recovery, and beginner/expert parity.

Non-goals: hiding irreversible data loss, requiring Git knowledge, promising conflict-free editing for all data, storing working state only in cloud, silently force-pushing, or making live-session state the only copy.

## 3. Data model

~~~text
ChangeId: stable intent across rewritten commits
CommitId: immutable tree/parents/metadata object
OperationId: immutable repository-state transition
WorkspaceId: working view identity
Bookmark: movable named reference
SemanticChange: schema-aware operations over stable document IDs
~~~

Object content is hash-addressed. Operation log records parent operation(s), resulting view, command descriptor, actor/workspace, time, checkpoints, and audit. Working-copy materialization is derived from selected operation/view.

## 4. Repository layout

Project source remains normal files/directories plus VCS metadata. .meridian/sync contains peer identities/references, transfer indexes, resumable state, partial workspace declarations, and local session cache; secrets live in OS credential storage, not this directory.

Large artifacts generally rebuild or transfer through package/artifact stores. Source large files use content-addressed pointers/locks only under explicit project policy.

## 5. User operations

Core operations: describe change, snapshot, new change, split, combine, reorder, restore object, undo operation, compare, resolve, bookmark, fetch, push, clone, workspace create, partial materialize, checkpoint, and share.

Beginner UI says My Change, Shared Changes, History, Conflicts, Restore, Share, and Sync. It previews scope and protects published history.

Expert UI/CLI exposes commit graph, operation graph, refs/remotes, semantic patches, object hashes, Git mappings, concurrent-operation resolution, and recovery.

## 6. Semantic diff and merge

Text uses structured/text merge. Schema-defined documents use stable IDs and field operations. World changes can identify entity/component/property/add/remove/move. Graph changes identify node/edge/property. Packages/artifacts are never hand-merged.

Merge pipeline:

1. validate base/left/right schema versions;
2. migrate into a supported comparison view without overwriting originals;
3. compute semantic operations;
4. merge independent operations;
5. flag same-field, delete/edit, identity, order, invariant, and capability conflicts;
6. validate merged candidate;
7. present source and semantic views;
8. commit resolution as a new operation.

No CRDT/OT is assumed universally. Each live document type selects a method through research and invariant tests.

## 7. Binary and lock policy

Binary sources declare merge strategy: regenerate, choose-side, domain merge, or lease/lock. Locks include object scope, owner, lease, server/peer authority, expiry, and recovery. Offline edits are allowed but surface conflict risk.

No permanent lock can make data unrecoverable. Admin override is audited.

## 8. P2P sync

~~~text
Peer session:
Discover/Pair -> Authenticate -> Negotiate capabilities/repository
-> Exchange object summaries -> Request missing chunks
-> Verify/hash/decrypt -> Import immutable objects
-> Reconcile operations/views -> Complete
~~~

Pairing uses explicit user action and authenticated key fingerprints/short codes. NAT traversal and transport are research choices; outbound-friendly direct paths are first. Relay stores/forwards encrypted bounded payloads and can be self-hosted. Relay cannot read repository content.

Resumable transfer is chunked, deduplicated, quota-bound, and separately prioritizes source, metadata, package/artifact, and live-session traffic.

## 9. Partial workspaces

A partial workspace declares included paths, stable object/document IDs, world regions/cells, asset families, and dependency closure policy. Missing data is an explicit placeholder with fetch/materialize operation, not a fake empty object.

Edits that require absent dependencies are blocked or run against declared stubs only when schema permits. Build validation reports materialization requirements.

## 10. Live collaboration

Live sessions provide presence, cursors/selections, advisory locks, document operations, chat/notes if enabled, build/play status, and checkpoint prompts. Session operations are bounded, authenticated, capability checked, and periodically checkpointed into VCS changes.

On disconnect, local operations remain. Rejoin exchanges missing operations. Ending a session requires checkpoint/discard/export decisions for uncommitted session state.

## 11. Security and privacy

Threats: malicious repositories, path traversal, object/hash abuse, decompression bombs, peer impersonation, replay, relay abuse, history rewrite, secret leakage, oversized session operations, and dependency substitution.

Parsers are bounded/fuzzed. Peers have per-repository capabilities. Keys rotate/revoke. Remote URLs and external actions are visible. Secret files are classified/ignored/scanned by policy but scanning is not a guarantee.

## 12. Diagnostics and recovery

Expose current operation/workspace, uncheckpointed changes, conflict/invariant state, Git mapping, transfer queue/progress/rate, peer/relay trust, partial materialization, locks, session checkpoint, and storage quota.

Operation-log recovery supports undo after interrupted commands. Import receives immutable objects before reference movement. Corrupt objects quarantine and ref movement rolls back to prior valid operation.

## 13. Tests and benchmarks

- operation graph/concurrent workspace/property tests;
- undo/restore after every mutating operation;
- Git clone/fetch/push/import/export fixtures;
- semantic merge by document type and schema migration;
- binary lock/lease/offline conflict;
- P2P direct/relay/offline/resume/rekey/hostile peer;
- partial workspace build/materialization;
- live disconnect/rejoin/checkpoint/conflict;
- large repository/object/operation/transfer performance and storage.

## 14. Phases

Phase 17 delivers VCS model and Git interoperability after provenance research. Phase 18 adds P2P sync, partial workspaces, and live collaboration. Phase 24 uses VCS/package trust for mods. Phase 29 hardens recovery/interoperability/support.

## 15. Examples

End-to-end: two creators clone via Git-compatible remote, create independent ChangeIds, edit separate world entities, sync directly, semantic-merge, checkpoint a live review, and push without exposing staging jargon.

Failure/recovery: power loss occurs during reference update. Immutable objects are valid, the incomplete operation is ignored, prior operation view opens, and user can inspect/retry without repository reset.

Performance debug: sync is slow; trace separates discovery, hashing, encryption, relay, source chunks, and derived artifacts, revealing unnecessary artifact transfer. Partial policy changes and the identical corpus verifies improvement.
