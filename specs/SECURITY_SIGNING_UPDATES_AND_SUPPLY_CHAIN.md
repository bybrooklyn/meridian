# Security, Signing, Updates, and Supply Chain

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Packages](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Agents](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)

Version 0.2 · 2026-07-14 · Normative

## 1. Security objectives

Protect project/source integrity, player/user systems, credentials/keys, build/update trust, private content, multiplayer/server authority, and recovery. Make trust visible and preserve offline/local workflows.

No security claim rests on obscurity, file extension, model judgment, TLS alone, or a signature without key/update/revocation policy.

## 2. Trust boundaries

Untrusted inputs include:

- project and world documents;
- imported assets and metadata;
- shaders, Luau, mods, native plugins;
- .meridian packages, saves, patches, update metadata;
- VCS repositories/peers/live operations;
- network packets/providers/SDKs;
- build workers/tool output/caches;
- MCP/agent/provider/model content;
- documentation links and external processes.

Every parser has size/depth/count/time/allocation limits and fuzz/property fixtures. Validation happens before allocation or authority where possible.

## 3. Threat model process

Each shipping phase updates assets, actors, trust boundaries, abuse cases, mitigations, residual risks, owner, and evidence. High-risk changes require review before implementation API/format freeze.

Required scenarios: malicious package/mod/project, compromised dependency/tool/worker/provider/relay/update mirror, key theft, downgrade/freeze/replay, path/symlink escape, decompression bomb, credential leakage, insecure default listener, authorization bypass, unsafe agent command, save/package corruption, and denial of service.

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

Reproducibility claims state exact scope and variance. An unsigned local build is allowed but visibly labeled and never auto-promoted to trusted release.

## 6. Key management

Keys use OS/hardware/offline stores according to role. Editor UI never handles raw private keys. Signing helper receives narrow artifact hash/metadata and returns signature through authenticated IPC.

Policies cover generation, backup, quorum/threshold, access logging, expiry, rotation, revocation, compromise recovery, disaster exercise, and personnel/device loss. Secrets are referenced, never placed in Cargo/project files, environment dumps, logs, traces, crash reports, VCS, packages, or agent context.

## 7. Update pipeline

1. fetch bounded timestamp metadata over configured transport;
2. validate root trust/rotation and signatures/thresholds;
3. reject rollback/freeze/expiry/version/hash/length inconsistency;
4. fetch snapshot/targets/delegated metadata;
5. select compatible channel/platform/architecture/capabilities;
6. download independent chunks/patch into quarantine with resume quotas;
7. verify hashes/signatures/package schema/provenance policy;
8. stage side-by-side;
9. preview changes/permissions/migrations;
10. atomically activate with prior version retained;
11. health check;
12. rollback on failure without downgrading trust metadata improperly.

Offline package install follows equivalent verification and trust prompts.

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

Telemetry is local crash capture by default and opt-in for sending. Payload preview/redaction and self-hostable endpoint support are requirements for Meridian services; games define their own player telemetry policy.

## 11. Supply chain

Dependencies and borrowed source require source URL/revision/checksum, license, notices, local modifications, update process, security status, owner, and exit strategy. Cairn fork provenance is mandatory before copying/rewriting implementation.

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
- safe-mode/no-network/minimal profile;
- package/mod/native capability and trust;
- reproducible build and SBOM/license/provenance;
- update activate/health/rollback and full key-compromise drill;
- external review before 1.0 for selected shipping threat surfaces.

## 15. Phases

Phase 0 establishes policy/research. Security gates are embedded in every phase. Phase 5 defines package/signing seams. Phase 16 produces build provenance. Phases 17–18 protect repositories/sync. Phases 22–25 add network/mod/agent threats. Phase 29 completes release/update/key compromise and platform signing certification.

## 16. Examples

End-to-end: release build produces SBOM/provenance and signed targets metadata; client validates role chain/freshness/hash, stages chunks, previews update, activates, health-checks, and retains rollback.

Failure/recovery: online timestamp key is compromised. Threshold/offline root policy publishes rotated/revoked metadata, clients reject rollback/freeze attempts, and staged affected artifacts quarantine without replacing active release.

Performance debug: update verification is slow. Trace attributes metadata parse, hashing, decompression, disk, and signature verification by chunk; optimization preserves the same trust checks and is compared on identical package/hardware.
