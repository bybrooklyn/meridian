# Version Control, Collaboration, and Sync Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Build/team](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)

version 0.5 · 2026-07-15 · Normative architecture · Planned

Documentation maturity: `ResearchReady`. Implementation maturity:
`Research` / `Deferred`. Governing IDs: `REQ-VCS-001`, `REQ-SYN-001`,
`WP-VCS-001`, `WP-SYN-001`, `PRG-VCS-001`.

## 1. Decision

Meridian VCS first provides Meridian-native project, asset, world, graph, model, and semantic workflows over existing Git and/or Jujutsu foundations. Git-compatible objects/remotes and a Jujutsu-shaped change/operation model supply interoperability, recovery, concurrent work, and undo while Meridian owns the user-facing contracts. The exact integration is a provenance and research gate.

`WP-VCS-001` does not require a new object store or history engine. A native Meridian storage/history replacement is post-1.0 `PRG-VCS-001` work and may begin only if production evidence shows material value that wrappers cannot provide, migration and Git escape remain safe, and maintenance capacity is sustainable. Wrapping Git/Jujutsu indefinitely is an acceptable outcome.

Telepo is not a product or separate subsystem. It is replaced by meridian-sync: direct encrypted peer-to-peer exchange first, optional self-hosted relay, no mandatory account, cloud, hosted Meridian service, or inbound port. Local sync state lives under .meridian/sync/.

Live collaboration augments immutable VCS operations/checkpoints. It never becomes authoritative history.

## 2. Goals and non-goals

Goals: understandable changes rather than staging/HEAD, operation-log undo, semantic diffs/merges, large/binary assets, partial workspaces, offline work, Git remotes, encrypted direct sync, crash recovery, and beginner/expert parity.

Non-goals: hiding irreversible data loss, requiring Git knowledge, promising conflict-free editing for all data, storing working state only in cloud, silently force-pushing, making live-session state the only copy, or rewriting mature storage/history solely to claim ownership.

## 3. Data model

~~~text
ChangeId: u128, stable intent across rewritten commits
CommitId: content hash (256-bit), immutable tree/parents/metadata object
OperationId: content hash (256-bit), immutable repository-state transition
WorkspaceId: u128, working view identity
Bookmark: String name (default max 255 bytes, UTF-8) -> CommitId, movable named reference
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

Resumable transfer is chunked (default reference chunk size 4 MiB, content-addressed for deduplication), quota-bound (default max 4 concurrent chunk transfers per peer session), and separately prioritizes source, metadata, package/artifact, and live-session traffic — live-session traffic preempts bulk chunk transfer on the same link.

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

Beginner history, conflict, lock, trust, and recovery flows must be keyboard and
screen-reader operable, scalable, and not color-only. Expert graph and semantic
diff views expose the same operations and provenance without requiring staging
jargon or raw object manipulation.

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

## 14. Delivery mapping

MS-08 delivers Meridian's UI/asset-aware VCS model and Git/Jujutsu interoperability after provenance research. MS-08/MS-09 adds P2P sync, partial workspaces, and live collaboration. MS-09 uses VCS/package trust for mods. MS-10 hardens recovery/interoperability/support. `PRG-VCS-001` is post-1.0 and cannot satisfy or block those milestones.

## 14.1 Work package briefs (medium — Deferred)

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status changes; lighter test/evidence detail since MS-08/MS-09 is further
out than the current work frontier.

**`WP-VCS-001` — Meridian VCS and Git interoperability**
Result: Meridian's UI/asset-aware VCS model over Git/Jujutsu foundations
(§14's MS-08 delivery), giving creators "My Change, Shared Changes, History,
Conflicts, Restore, Share, Sync" (§5) instead of staging/HEAD jargon. Entry
conditions: provenance research on the exact Git/Jujutsu integration shape
(§1 — "the exact integration is a provenance and research gate"); this
package explicitly does not require a new object store or history engine
(§1). Deliverables: the data model in §3 (`ChangeId`/`CommitId`/
`OperationId`/`WorkspaceId`/`Bookmark`/`SemanticChange`), core user
operations (§5), and the semantic diff/merge pipeline (§6: validate schema
versions → migrate into comparison view → compute semantic operations →
merge independent operations → flag conflicts → validate candidate →
present source/semantic views → commit resolution). Non-goals: no native
storage/history replacement — that stays `PRG-VCS-001`, gated on production
evidence of material value wrappers can't provide (§1); wrapping Git/Jujutsu
indefinitely is an explicitly acceptable permanent outcome, not a stopgap
this package must escape. Tests: operation graph/concurrent workspace,
undo/restore after every mutation, Git clone/fetch/push/import/export
fixtures, semantic merge by document type (§13, scoped to this package).
Stop condition: power loss during a reference update must leave immutable
objects valid and the incomplete operation ignorable, never a corrupted
repository (§15's failure example — this recovery guarantee is a hard
release bar, not best-effort). Next unblocked: `WP-SYN-001`.

**`WP-SYN-001` — Encrypted optional sync and collaboration**
Result: direct encrypted peer-to-peer exchange (meridian-sync, replacing
the retired Telepo concept), optional self-hosted relay, partial
workspaces, and live collaboration (§1, §14's MS-08/MS-09 delivery). Entry
conditions: `WP-VCS-001` closed — sync operates over VCS operations/
checkpoints and never becomes authoritative history on its own (§1).
Deliverables: the P2P session lifecycle in §8 (discover/pair → authenticate
→ negotiate → exchange summaries → request chunks → verify/decrypt →
import immutable objects → reconcile → complete), partial-workspace
materialization with explicit placeholders rather than fake empty objects
(§9), and live-session presence/cursors/advisory-locks/checkpoint prompts
(§10) that periodically checkpoint into real VCS changes. Non-goals: no
mandatory account, cloud, hosted Meridian service, or inbound port (§1);
live session state is never the only copy of anything (§2's non-goals).
Security: relay stores/forwards encrypted bounded payloads and cannot read
repository content (§8); no permanent lock can make data unrecoverable,
and admin override is audited (§7). Tests: P2P direct/relay/offline/resume/
rekey/hostile-peer, partial workspace build/materialization, live
disconnect/rejoin/checkpoint/conflict (§13, scoped to sync/collaboration).
Stop condition: on disconnect, local operations must remain intact and
rejoin must exchange only missing operations — a sync failure can never
silently discard local work (§10). Next unblocked: MS-09's mod-trust
integration, which uses VCS/package trust this package establishes (§14).

## 15. Examples

End-to-end: two creators clone via Git-compatible remote, create independent ChangeIds, edit separate world entities, sync directly, semantic-merge, checkpoint a live review, and push without exposing staging jargon.

Failure/recovery: power loss occurs during reference update. Immutable objects are valid, the incomplete operation is ignored, prior operation view opens, and user can inspect/retry without repository reset.

Performance debug: sync is slow; trace separates discovery, hashing, encryption, relay, source chunks, and derived artifacts, revealing unnecessary artifact transfer. Partial policy changes and the identical corpus verifies improvement.
