# ADR-0030: Artus Procedural Body-Motion Architecture

- Status: Adopted
- Date: 2026-07-18
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned
- Owners: ANI, Cairn, gameplay, navigation, data, editor, networking, agents
- Supersedes: none
- Superseded by: none

## Context

Meridian previously defined a general animation authority but did not name the
character-performance coordination layer or fully fix its boundary with Cairn,
gameplay, navigation, sequencing, authoring, and agents. Direct bone mutation,
physics-authoritative animation, or a motion system tied to one consumer game
would violate existing stable-boundary, source-authority, and general-purpose
product rules.

## Decision

Adopt Artus as the named `ANI` procedural body-motion subsystem. Artus owns
semantic rig and performance data, pose and motion-source composition,
retargeting, contacts, IK requests, root-motion proposals, body-performance
sequencing, and the shared semantic motion-intent boundary. It uses negotiated
movement: consumers supply intent and desired trajectory; Artus proposes a
body-performance result; Cairn resolves physical reality; Artus adapts to the
achieved result.

Gameplay and future Bearings decision layers remain intent producers; NAV
remains a traversability/query system; Cairn retains collision, rigid bodies,
controllers, joints, contacts, and final physical state; Penumbra retains
deformation execution and presentation. Bearings is a consumer-layer alias,
not a new domain, package, or implementation claim.

MS-09 may establish a narrow facial profile/import/pose-layer foundation.
Advanced facial performance, capture, cinematic characters, virtual production,
and active physical control remain `PRG-ANI-001` after MS-10. Exact motion
intent, retarget/IK, pose-search, compression, deformation, and network
selection choices remain research-gated.

## Current Evidence

The existing repository has no Artus runtime, animation graph, retargeter,
motion matching, facial solver, or performance-capture pipeline. This ADR and
the Artus specification are architecture only; they do not promote ANI
implementation maturity or activate a package.

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `governance/generated/`

## Consequences

- Artus contracts use Meridian-owned descriptors, stable source IDs, immutable
  snapshots, and typed failures; external physics, renderer, ECS, or agent
  types do not become public Artus APIs.
- Source profiles and accepted edits are authoritative; search indexes, caches,
  and bakes are derived and provenance-bearing.
- Motion matching must retain a graph or current-motion fallback. Disabled
  Artus capability packs have zero recurring cost.
- Future package work must demonstrate source migration, intent/Cairn
  reconciliation, representative rig/contact recovery, accessibility,
  provenance, and bounded time/memory before any product claim.

## Status Review

Review when `RG-ANI-001` or `RG-ANI-002` produces its decision ADR, when
`WP-ANI-001` becomes ready, or if evidence requires a different owner or
delivery boundary.
