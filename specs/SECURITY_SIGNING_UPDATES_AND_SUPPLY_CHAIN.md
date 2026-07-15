# Security, Signing, Updates, and Supply Chain

[Master](MERIDIAN_MASTER_SPEC.md) · [Packages](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) · [Distributed worlds](DISTRIBUTED_WORLDS_AND_MMO_SPEC.md) · [Integrity](INTEGRITY_ANTI_CHEAT_AND_MODERATION_SPEC.md) · [Agents](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)

version 0.5 · 2026-07-15 · Normative

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

Telemetry is local crash capture by default and opt-in for sending. Collective analytics, when selected, requires purpose-bound schemas, consent/policy, minimization, retention, export/deletion, redaction, and self-hostable/provider-neutral sinks. Games define product policy. Meridian does not promise to operate hosted telemetry or online infrastructure.

Wavefront microphone capture, Collective voice-room policy, and NET transport are separate permissions and trust boundaries. Joining a room cannot open a microphone. Social, messaging, analytics, moderation, and account modules do not activate merely because transport is enabled.

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
- safe-mode/no-network/minimal profile;
- package/mod/native capability and trust;
- reproducible build and SBOM/license/provenance;
- Alluvium provenance propagation, target-policy cooker rejection, private
  corpus redaction, runtime budget exhaustion, and hostile recipe limits;
- update activate/health/rollback and full key-compromise drill;
- external review before 1.0 for selected shipping threat surfaces.

## 15. Delivery mapping

MS-00 establishes policy/research. Security gates are embedded in every milestone and work package. MS-01/MS-03/MS-04 define package/signing seams. MS-03/MS-08 produce build provenance. MS-08/MS-09 protect repositories/sync and add network/Collective/mod/agent threats only for selected profiles. MS-10 completes release/update/key-compromise and platform-signing certification. `PRG-WRL-001` and `PRG-INT-001` add separate post-1.0 threat models and cannot borrow MS-10 evidence as implementation proof.

## 16. Examples

End-to-end: release build produces SBOM/provenance and signed targets metadata; client validates role chain/freshness/hash, stages chunks, previews update, activates, health-checks, and retains rollback.

Failure/recovery: online timestamp key is compromised. Threshold/offline root policy publishes rotated/revoked metadata, clients reject rollback/freeze attempts, and staged affected artifacts quarantine without replacing active release.

Performance debug: update verification is slow. Trace attributes metadata parse, hashing, decompression, disk, and signature verification by chunk; optimization preserves the same trust checks and is compared on identical package/hardware.
