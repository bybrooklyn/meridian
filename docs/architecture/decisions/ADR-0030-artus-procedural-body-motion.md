# ADR-0030: Artus Procedural Body-Motion Architecture

- Status: Adopted
- Date: 2026-07-18
- Spec version: v0.5
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

## Intended v0.5 Links

- `specs/ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md`
- `specs/CAIRN_PHYSICS_SPEC.md`
- `specs/NAVIGATION_AND_AI_INFRASTRUCTURE_SPEC.md`
- `specs/DELIVERY_ROADMAP.md`
- `specs/RESEARCH_AND_ALGORITHM_DECISIONS.md`
- `specs/TESTING_BENCHMARKS_AND_VALIDATION.md`
- `specs/registry/research-gates.json`

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
