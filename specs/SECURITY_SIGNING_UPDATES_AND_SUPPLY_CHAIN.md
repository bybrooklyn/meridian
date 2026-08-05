# Security, Signing, Updates, and Supply Chain

[Master](MERIDIAN_MASTER_SPEC.md) · [Build and IDE](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) · [Packages](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Marquee](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md) · [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) · [Distributed worlds](DISTRIBUTED_WORLDS_AND_MMO_SPEC.md) · [Integrity](INTEGRITY_ANTI_CHEAT_AND_MODERATION_SPEC.md) · [Agents](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)

version 0.5 · 2026-07-18 · Normative

Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-SEC-001`, `WP-SEC-001`, `RG-SEC-001`.

## 1. Security objectives

Protect project/source integrity, player/user systems, credentials/keys, build/update trust, private content, multiplayer/server authority, and recovery. Make trust visible and preserve offline/local workflows.

No security claim rests on obscurity, file extension, model judgment, TLS alone, or a signature without key/update/revocation policy.

## 2. Trust boundaries

Untrusted inputs include:

- project and world documents;
- imported assets and metadata;
- Meridian Shader Language/WGSL/target shader inputs, optional Luau, Rust game modules, mods, native plugins;
- .meridian packages, saves, patches, update metadata;
- VCS repositories/peers/live operations;
- network packets/providers/SDKs;
- Collective identity/session/social/voice-policy/analytics/moderation inputs and provider responses;
- future distributed-world authority/migration/storage messages and integrity signals/evidence;
- build workers/tool output/caches;
- Alluvium recipes, node libraries, generated metadata/artifacts, external
  authoring tools, and runtime-safe recipe inputs;
- MCP/agent/provider/model content;
- documentation links and external processes.

Every parser has size/depth/count/time/allocation limits and fuzz/property fixtures. Validation happens before allocation or authority where possible.

## 3. Threat model process

Each shipping milestone updates assets, actors, trust boundaries, abuse cases,
mitigations, residual risks, owner, and evidence. High-risk changes require
review before implementation API/format freeze.

Required scenarios: malicious package/mod/project, compromised dependency/tool/worker/provider/relay/update mirror, key theft, downgrade/freeze/replay, path/symlink escape, decompression bomb, credential leakage, insecure default listener, authorization bypass, unsafe agent command, save/package corruption, identity/session abuse, privacy/consent failure, moderation evidence leakage, distributed split brain, integrity false positive/evasion, and denial of service. Only scenarios for selected capabilities are required to ship, but omitted capabilities must prove zero-cost absence.

## 4. Signing and update roles

Meridian uses a TUF-inspired metadata model:

~~~text
RootRole: trust anchors, role keys/thresholds, root rotation
TargetsRole: versions, artifacts, hashes, lengths, custom compatibility
SnapshotRole: consistent metadata set/version
TimestampRole: freshness and current snapshot
DelegatedRoles: engine, game, packages, mods, providers/channels
~~~

Exact libraries, signature algorithms, key sizes, threshold counts, expiry durations, and storage are Research/Security gates. Current TUF principles inform roles; this document does not fabricate final cryptography.

Metadata is versioned, length bounded, canonicalized, signed, and rollback/freeze/mix-and-match checked. Offline root and release targets keys are separated from online freshness keys.

## 5. Artifact provenance

Every release artifact records:

- BuildId and source checkpoint;
- toolchain/compiler/build-service versions;
- dependency lock and source/checksum/license;
- feature/capability/profile/target;
- input artifact/package hashes;
- builder identity/environment policy;
- SBOM and license notices;
- test/benchmark/security/evidence references;
- signing role/metadata versions.
- Alluvium recipe/output hashes, evaluator/algorithm versions, determinism
  level, provenance-manifest hash, license disposition, and shipping eligibility
  when generated content participates.

Reproducibility claims state exact scope and variance. An unsigned local build is allowed but visibly labeled and never auto-promoted to trusted release.

## 6. Key management

Keys use OS/hardware/offline stores according to role. Editor UI never handles raw private keys. Signing helper receives narrow artifact hash/metadata and returns signature through authenticated IPC.

Policies cover generation, backup, quorum/threshold, access logging, expiry, rotation, revocation, compromise recovery, disaster exercise, and personnel/device loss. Secrets are referenced, never placed in Cargo/project files, environment dumps, logs, traces, crash reports, VCS, packages, or agent context.

## 7. Update pipeline

1. fetch bounded timestamp metadata over configured transport (default max
   parse size 16 KiB — a parsing/decompression-bomb bound, distinct from the
   key/threshold/expiry parameters `RG-SEC-001` still owns);
2. validate root trust/rotation and signatures/thresholds;
3. reject rollback/freeze/expiry/version/hash/length inconsistency;
4. fetch snapshot/targets/delegated metadata (default max parse size 1 MiB
   per role document, same parsing-bound rationale as step 1);
5. select compatible channel/platform/architecture/capabilities;
6. download independent chunks/patch into quarantine with resume quotas
   (default reference chunk size 4 MiB, default max 4 concurrent chunk
   downloads per update, each independently resumable and hash-verified —
   these are download-engineering limits, not cryptographic parameters);
7. verify hashes/signatures/package schema/provenance policy;
8. stage side-by-side;
9. preview changes/permissions/migrations;
10. atomically activate with prior version retained;
11. health check;
12. rollback on failure without downgrading trust metadata improperly.

Offline package install follows equivalent verification and trust prompts.
The parsing-bound numbers above are ordinary DoS-prevention engineering
limits and are safe to state now; the cryptographic parameters named in §4
(signature algorithms, key sizes, threshold counts, expiry durations) remain
deliberately unstated pending `RG-SEC-001` and must not be inferred from
these bounds.

The same pipeline governs externally managed development-toolchain components:
`rustc`, Cargo, rust-analyzer, platform SDKs, debuggers, and external shader
compilers. Their functional ownership, project pins, compatible-component
resolution, and editor/CLI workflow are defined by the [Build and IDE
specification](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md#31-managed-development-toolchains).
Security policy requires each component's exact artifact hash, declared length,
signature/provenance result, license/notice record, compatibility metadata, and
activation generation to be verified before atomic side-by-side activation.
Project pins remain immutable during global/default updates, repairs, rollback,
and cleanup; no recovery path may silently upgrade, downgrade, or substitute a
pinned component.

## 8. Package, mod, and plugin trust

Trust levels are explicit: local-unsigned, locally trusted, developer signed, provider/community signed, game-approved, release signed, revoked. A valid signature establishes key provenance, not safety.

Capabilities, sandbox, source, review, and game/server policy remain required. Native code is high trust and platform specific. Mods cannot inherit game/update signing authority.

## 9. Process and runtime hardening

- structured process args and allowlisted environment;
- least-privilege filesystem roots and network destinations;
- no ambient listener for optional features;
- authenticated/versioned/length-bounded IPC;
- memory-safe Rust baseline with unsafe denied or isolated/reviewed;
- watchdog/resource quotas for untrusted workers/scripts/providers;
- ASLR/OS hardening/signing/notarization where platforms require;
- safe mode excluding optional/untrusted content;
- local crash report and inspectable opt-in telemetry.

## 10. Network and privacy

Network features are off unless selected. Listeners, discovery, relay, cloud, provider, web search, telemetry, and updates are separate capabilities. UI shows destination, data class, purpose, retention/policy reference, and revocation.

Telemetry is local crash capture by default and opt-in for sending. Collective analytics, when selected, requires purpose-bound schemas, consent/policy, minimization, retention, export/deletion, redaction, and self-hostable/provider-neutral sinks. Games define product policy. Meridian does not promise to operate hosted telemetry or online infrastructure.

Wavefront microphone capture, Collective voice-room policy, and NET transport are separate permissions and trust boundaries. Joining a room cannot open a microphone. Social, messaging, analytics, moderation, and account modules do not activate merely because transport is enabled.

Marquee imports untrusted media, fonts, archives, templates, copy, and adapter output into isolated bounded workers. Campaign exports contain no service credentials or publishing actions. Optional cloud AI requires explicit per-request disclosure and is limited to text/analysis suggestions; audiovisual generation or modification is forbidden. Private campaign sources, spoilers, embargoes, and approvals remain outside public engine evidence.

## 11. Supply chain

Dependencies and borrowed source require source URL/revision/checksum, license, notices, local modifications, update process, security status, owner, and exit strategy. Cairn fork provenance is mandatory before copying/rewriting implementation. Alluvium donor libraries and generated outputs follow the same policy; generation cannot erase or loosen an input license.

CI checks lockfile/source policy, advisories under documented exception process, license/provenance, generated-code source, SBOM, artifact signatures, and secret scanning. An advisory exception has scope, impact, mitigation, owner, expiry, and removal plan.

## 12. Editor/CLI workflow

Beginner: Security & Updates shows channel, current/trusted version, update summary, source, signature/trust, permissions, restart/rollback, and plain errors.

Expert: role/metadata chain, key IDs/thresholds, SBOM/provenance, package chunks, reproducibility, advisories/exceptions, capabilities/listeners, audit/export.

Planned commands verify project/package/save/update/provenance/SBOM/signatures, stage/activate/rollback, rotate/revoke metadata, and run compromise drill. Destructive key/trust actions require explicit authority.

## 13. Diagnostics and recovery

Stable diagnostics distinguish parse, hash, signature, threshold, expiry, rollback, compatibility, capability, provenance, sandbox, and policy failures. They never suggest bypassing verification as the default fix.

Interrupted update preserves active version and quarantined resumable chunks. Failed health check returns to prior version. Corrupt cache is deleted/reacquired; trusted metadata and recovery logs remain.

## 14. Tests and audits

- parser fuzz/property/limits for every untrusted format;
- signature/threshold/root rotation/delegation;
- rollback/freeze/mix-and-match/expiry/replay;
- chunk resume/corruption/decompression/path traversal;
- key helper IPC/authorization/redaction;
- compromised worker/provider/relay simulations;
- Collective offline stripping, provider outage/exit, consent withdrawal, export/deletion, block/mute/report, moderation appeal, voice permission, and private-data redaction;
- post-1.0 WRL authority-epoch/split-brain and INT false-positive/evasion/privacy fixtures when those programs activate;
- post-1.0 Marquee path/archive/media-bomb, rights/consent/embargo, profile-staleness, approval, cloud-disclosure, AI-boundary, and private-leakage fixtures when `PRG-PRM-001` activates;
- safe-mode/no-network/minimal profile;
- package/mod/native capability and trust;
- reproducible build and SBOM/license/provenance;
- Alluvium provenance propagation, target-policy cooker rejection, private
  corpus redaction, runtime budget exhaustion, and hostile recipe limits;
- update activate/health/rollback and full key-compromise drill;
- managed-toolchain quarantine, hash/signature/license/compatibility rejection,
  atomic activation, interrupted update, exact repair, rollback, and pinned
  component retention tests;
- external review before 1.0 for selected shipping threat surfaces.

## 15. Delivery mapping

MS-00 establishes policy/research. Security gates are embedded in every milestone and work package. MS-01/MS-03/MS-04 define package/signing seams. MS-03/MS-08 produce build provenance. MS-08/MS-09 protect repositories/sync and add network/Collective/mod/agent threats only for selected profiles. MS-10 completes release/update/key-compromise and platform-signing certification. `PRG-WRL-001` and `PRG-INT-001` add separate post-1.0 threat models and cannot borrow MS-10 evidence as implementation proof.

## 15.1 Work package brief

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status change.

**`WP-SEC-001` — Security, signing, updates, and provenance**
Result: a release build produces SBOM/provenance and signed targets
metadata; a client validates the role chain/freshness/hash, stages chunks,
previews the update, activates, health-checks, and retains rollback (§16's
end-to-end example). Owning contracts: the TUF-inspired role model
(`RootRole`/`TargetsRole`/`SnapshotRole`/`TimestampRole`/`DelegatedRoles`,
§4). Entry conditions: `RG-SEC-001` decided (cryptography and key-management
selection, §4 — "exact libraries, signature algorithms, key sizes, threshold
counts... are Research/Security gates," so this package cannot fabricate
final cryptography ahead of its own gate). Deliverables: the update pipeline
in §7 (fetch bounded timestamp metadata → validate root trust/signatures →
reject rollback/freeze/expiry/hash inconsistency → fetch snapshot/targets →
select compatible channel → download into quarantine → verify → stage
side-by-side → preview → atomically activate with prior version retained →
health check → rollback on failure), artifact provenance recording (§5),
key management with editor UI never handling raw private keys (§6), and the
managed-toolchain pipeline sharing the same verification machinery for
rustc/Cargo/rust-analyzer/platform SDKs (§7). Non-goals: no security claim
resting on obscurity, file extension, model judgment, or TLS alone (§1);
this package does not itself open `PRG-WRL-001`/`PRG-INT-001`'s post-1.0
threat models (§15 — those "cannot borrow MS-10 evidence as implementation
proof"). Failure/recovery: an interrupted update preserves the active
version and quarantined resumable chunks; a failed health check returns to
the prior version without downgrading trust metadata improperly (§7, §13).
Tests: the full §14 list scoped to signing/update/key machinery (parser
fuzz/property/limits, signature/threshold/root-rotation, rollback/freeze/
replay, chunk resume/corruption/path-traversal, key-helper IPC/redaction,
compromised-provider simulation, reproducible build/SBOM, update activate/
health/rollback, full key-compromise drill, managed-toolchain quarantine/
hash/signature/atomic-activation). Stop condition: any trust-metadata path
that cannot prove rollback/freeze/replay rejection blocks that channel from
shipping, not just from the default profile (§13's diagnostics rule: never
suggest bypassing verification as the default fix). Next unblocked: MS-08/
MS-09's repository/sync/network/Collective/mod/agent threat-boundary rows
(§15), which assume this package's signing/update foundation exists.

## 16. Examples

End-to-end: release build produces SBOM/provenance and signed targets metadata; client validates role chain/freshness/hash, stages chunks, previews update, activates, health-checks, and retains rollback.

Failure/recovery: online timestamp key is compromised. Threshold/offline root policy publishes rotated/revoked metadata, clients reject rollback/freeze attempts, and staged affected artifacts quarantine without replacing active release.

Performance debug: update verification is slow. Trace attributes metadata parse, hashing, decompression, disk, and signature verification by chunk; optimization preserves the same trust checks and is compared on identical package/hardware.
