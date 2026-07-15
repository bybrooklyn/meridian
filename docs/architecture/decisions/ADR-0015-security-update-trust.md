# ADR-0015: Security and Update Trust

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Policy adopted; release implementation planned
- Owners: security policy, signing/update pipeline, package system
- Supersedes: none
- Superseded by: none

## Context

Projects, packages, saves, shaders, scripts, mods, providers, build workers,
agents, network messages, and update metadata all cross trust boundaries.
Signatures alone do not prove safety, and local-first workflows must remain
valid.

## Decision

Meridian uses explicit trust boundaries, least privilege, deny-by-default
capabilities, secret redaction, safe mode, and TUF-inspired signing/update roles.
A valid signature establishes key provenance, not safety. Updates validate role
metadata, hashes, lengths, compatibility, rollback/freeze protection,
quarantine, preview, activation, health check, and rollback.

## Current Evidence

- [Security, signing, updates, and supply chain](../../../specs/SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)
- [Agent API, MCP, Ollama, and AI spec](../../../specs/AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)
- [Testing, benchmarks, and validation](../../../specs/TESTING_BENCHMARKS_AND_VALIDATION.md)

## Intended v0.3 Links

- `specs/SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md`
- `specs/ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md`
- `specs/MODDING_AND_COMMUNITY_LIBRARY_SPEC.md`

## Consequences

Exact crypto libraries, algorithms, thresholds, expiry windows, and key storage
remain research/security gates. Release claims require provenance, SBOM,
license, update rollback, key-compromise drill, safe-mode, and external review
where required.

## Status Review

Review before package signing or update activation becomes implementation scope.
