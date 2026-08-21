# ADR-0015: Security and Update Trust

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
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

- [Security, signing, updates, and supply chain](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Exact crypto libraries, algorithms, thresholds, expiry windows, and key storage
remain research/security gates. Release claims require provenance, SBOM,
license, update rollback, key-compromise drill, safe-mode, and external review
where required.

## Status Review

Review before package signing or update activation becomes implementation scope.
