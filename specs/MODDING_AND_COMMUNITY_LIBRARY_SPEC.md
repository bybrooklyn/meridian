# Modding and Community Library Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Gameplay](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)

version 0.5 · 2026-07-15 · Normative architecture · Optional and deferred to MS-09

Documentation maturity: `ArchitectureComplete`. Implementation maturity:
`Deferred`. Governing IDs: `REQ-MOD-001`, `WP-MOD-001`.

## 1. Principles

Games choose whether and how to support mods. Only explicitly published APIs are stable. A restricted editor and free/community-first library MAY be provided; no commercial marketplace or hosted Meridian account is required.

Mods are packages with identity, dependencies, content, declared capabilities, compatibility, provenance, signatures/trust, and deterministic load order. Installing a mod is not consent to arbitrary code, filesystem, network, process, native library, agent, or data access.

## 2. Mod types

- data/content: worlds, prefabs, materials, UI, audio, localization;
- Luau: sandboxed modules using published game API;
- server policy/config;
- editor extension using published command/panel APIs;
- native: disabled by default, platform-specific, explicit high-trust install, separate security/support policy.

A game may support only a subset. Unsupported kinds fail clearly.

## 3. Manifest

~~~text
ModManifest {
  id, version, game_id, game_version_range,
  api_version_range, authors, license,
  dependencies, conflicts, load_constraints,
  packages, entry_points, capabilities,
  signature_policy, provenance, content_hashes
}
~~~

IDs are stable and namespaced. Dependency resolution is deterministic and explains conflicts. Load order uses explicit constraints, never filesystem enumeration.

## 4. Capabilities

Examples: published gameplay module, selected world namespace, UI extension point, local settings storage, network message namespace, server command, editor document access, external URL. Capabilities have scope, lifetime, audit, user/game/server policy, and headless availability.

Mods cannot request secrets, signing keys, arbitrary project source, other mods’ private state, or agent authority. Native mods are outside normal sandbox guarantees and are labeled accordingly.

## 5. APIs and compatibility

Published API descriptors are generated from the gameplay/command schema and include stability/deprecation/migration. Internal types remain inaccessible. Compatibility check happens before mounting or executing.

Games define supported mod API windows. Breaking changes require migration guidance and can keep compatibility shims only within measured maintenance/security cost.

## 6. Packaging, load, and persistence

1. acquire from local file, direct peer, game-approved server, or optional library;
2. verify chunk hashes/signatures/trust/provenance;
3. parse manifest under limits;
4. resolve dependency/content set;
5. preview capabilities, conflicts, shipping/network impact;
6. mount read-only namespaces;
7. validate schemas/scripts/artifacts;
8. activate at safe lifecycle barrier;
9. store selected mod set in save/server/session metadata.

Disabling a stateful mod requires a migration, archival, or explicit data-loss preview. Saves keep unknown mod records where safe.

## 7. Restricted editor

Games publish templates, document types, properties, commands, validation, preview worlds, and export profiles. The restricted editor hides engine/private game source and prevents signing/publishing outside user authority.

Beginner path uses install/create template, guided capability selection, test, package, local share. Expert path exposes manifest/schema/API compatibility, dependency solver, performance, provenance, and network policy.

Install, trust, conflict, capability, and recovery flows must be keyboard and
screen-reader operable, support text scaling, and avoid color-only warnings.
Mods declare accessibility effects and cannot remove a game's required recovery
or settings surfaces.

## 8. Community library

The library protocol is provider-neutral. It supports metadata/search, immutable versions, hashes/signatures, dependency information, moderation/trust labels, report/takedown references, and resumable chunk delivery. Local folder/direct link/self-hosted indexes remain valid.

The engine does not imply endorsement or safety from discoverability. Offline install and export are first-class.

## 9. Multiplayer

Servers declare exact allowed/required mod set, signatures, API/content hashes, capabilities, and distribution sources. Clients do not auto-install native or excessive-capability mods. Mod network messages use allocated schema namespaces and normal rate/size limits.

## 10. Diagnostics, security, provenance, and recovery

Report resolution graph, mount order, capability grants/denials, API mismatch, source/provenance, validation, startup/runtime cost, package size, network compatibility, and save dependencies.

Safe mode disables mods without deleting data. A crashing script mod is stopped/isolated under game policy; last stable save and mod set remain recoverable.

Signatures identify publishers but do not prove safety. Every package retains
source, license, build, dependency, moderation, and trust provenance. Parsers,
scripts, native extensions, downloads, and community metadata remain untrusted
at their declared boundaries.

## 11. Tests and benchmarks

- dependency/conflict/load-order property tests;
- malformed/decompression/path/signature/sandbox corpus;
- capability denial and cross-mod isolation;
- save with missing/upgraded/downgraded mod;
- server/client mod-set negotiation;
- restricted editor information/command boundary;
- offline/local/self-hosted library;
- startup, memory, package, script, and network overhead attribution;
- disabled-modding no dependency/task/panel/chunk proof.

## 12. Delivery mapping and examples

MS-09 implements the SDK, restricted editor, and provider-neutral library after gameplay/UI/VCS/network seams stabilize. MS-10 certifies only the profiles a shipping game selects.

End-to-end: user installs a signed Luau content mod, previews world/UI capabilities, resolves one dependency, tests in a forked save, packages the selected set, and joins a server requiring identical hashes.

Failure/recovery: a mod update removes a saved component without migration. Compatibility validation blocks activation, preserves old package/save, and offers keep-old, disable-with-archive, or developer migration.

Performance debug: startup regression is grouped by mod import, script compile, world data, and package reads; disabling the identified mod removes its tasks/chunks and confirms attribution.
Non-goals are arbitrary native-code safety claims, guaranteed compatibility for
private engine internals, mandatory hosted services, silent capability grants,
or treating discoverability as endorsement.
