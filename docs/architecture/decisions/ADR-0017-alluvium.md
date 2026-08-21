# ADR-0017: The Alluvium Engine

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: ImplementedFoundation; `WP-PRC-001` passed its CI evidence gate
- Owners: future `meridian-alluvium`, editor/build, data, and procedural workstreams
- Amends: ADR-0008, ADR-0009, ADR-0011, ADR-0014
- Supersedes: none
- Superseded by: none

## Context

Meridian needs first-party procedural authoring that can create coherent terrain, vegetation, materials, weathering, infrastructure, structures, and simulation-aware source data without requiring proprietary tools. The v0.3 procedural specification described useful graph, determinism, cache, and override foundations but treated the work too narrowly and left authoring ownership ambiguous with Basalt and optional capability packs.

## Decision

Adopt The Alluvium Engine as Meridian's core procedural world-authoring, asset-generation, environmental-composition, and simulation-aware cooking system.

Alluvium owns recipes, typed graph and field evaluation, cache/invalidation, generated identity, non-destructive overrides, provenance/license propagation, and cooking of generated outputs. It produces typed source or built artifacts for Basalt, vegetation, Isobar, Torsant, Cairn, Penumbra, audio/acoustics, navigation, world streaming, assets, packages, and saves. Those systems retain authority over live runtime state and behavior.

The editor/build capability is core Meridian functionality, not an optional proprietary plug-in. Runtime-safe evaluation remains content-triggered and capability-scoped. A baked-only project does not ship the editor, graph compiler, preview cache, or runtime evaluator and incurs no recurring Alluvium runtime cost.

`WP-PRC-001` created `meridian-alluvium` as the bounded textual scalar-reference foundation. Internal modules use descriptive names. Third-party foundations remain behind Meridian seams and are replaced only through measured research gates and an ADR.

Project Meridian supplies the first private proving requirements. Engine documents and evidence contain only sanitized functional contracts, generated surrogates, and controlled hashes; AMI content, proprietary recipes, seeds, hero overrides, and assets remain private.

## Amendments to Existing Decisions

- ADR-0008: Basalt retains terrain and large-world runtime authority; Alluvium owns procedural terrain authoring and derived source generation.
- ADR-0009: textual recipes, CLI/headless operation, and a basic inspector precede the full visual graph editor; every surface uses the same typed commands and schemas.
- ADR-0011: recipes, parameters, seeds, and overrides are source authority; generated artifacts and field caches remain derived unless explicitly promoted through a source transaction.
- ADR-0014: Alluvium editor/build support is core. Domain adapters and runtime evaluation still obey capability and zero-cost-disabled rules.

## Consequences

- The `PRC` domain remains stable while the owning specification is retitled in place.
- `MS-05` requires a minimum Alluvium foundation and environmental proving recipes.
- Alluvium cannot become a universal runtime solver or duplicate subsystem authority.
- First-party authoring cannot require proprietary software or an online account.
- AI output remains editable recipe/source data under normal command, provenance, license, and cooker policy.
- Competitive parity and performance claims require evidence; adoption of the architecture is not an implementation claim.

## Current Evidence

- [Alluvium specification](../../../MERIDIAN_SPECOMENT.md)
- [v0.4 migration ledger](../../migrations/V0_4_ALLUVIUM_AMENDMENT.md)
- [Source data authority](ADR-0011-data-authority.md)
- [Repository split](ADR-0003-repository-split.md)
- GitHub Actions run `29511174569` passed governance and Linux, Windows, and macOS workspace rows for `9c88cc152878b1eb22f18c236c00ad1abd984fa5`.

## Status Review

Review after `WP-PRC-001`, after the `MS-05` representative forest evidence, and before any runtime-safe evaluator or dependency replacement is promoted.
