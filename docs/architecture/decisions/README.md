# Architecture Decision Records

Version: v0.5 canonical ADR set.

This directory is the canonical home for Meridian architecture decision records.
The former non-canonical ADR directory has been removed; its migration is
recorded in the v0.3 migration register.

## Status Vocabulary

Use the status vocabulary from the master specification:

- Implemented: verified in the current repository.
- ImplementedFoundation: verified for a narrow foundation, not the full feature.
- StructuralSmoke: construction or submission succeeded without proving product quality.
- Partial: useful code exists but named completion evidence is missing.
- Transitional: current code exists behind a planned Meridian-owned replacement.
- Scaffold: marker crate, panel, schema, or definition exists without product behavior.
- Planned: specified or adopted but not implemented.
- Research: a choice remains open until a named experiment.
- Deferred: valid scope outside the current active package.
- Unsupported: intentionally unavailable in the current profile or platform.

## Records

| ADR | Title | Decision status | Implementation status |
|---|---|---|---|
| [ADR-0001](ADR-0001-governance-status.md) | Governance and Status Authority | Adopted | Implemented documentation governance; enforcement remains partial |
| [ADR-0002](ADR-0002-milestone-workstream-roadmap.md) | Milestone and Workstream Roadmap | Adopted | Implemented documentation roadmap; execution remains milestone-gated |
| [ADR-0003](ADR-0003-repository-split.md) | Engine and Project Meridian Repository Split | Adopted | Implemented repository-boundary policy |
| [ADR-0004](ADR-0004-penumbra-clustered-forward-plus.md) | Penumbra Clustered Forward+ Baseline | Adopted | Planned baseline with implemented renderer foundations |
| [ADR-0005](ADR-0005-shared-renderer-systems.md) | Shared Renderer Systems | Adopted | Partial foundations |
| [ADR-0006](ADR-0006-meridian-rhi-wgpu-native-backend.md) | Meridian RHI, wgpu, and Native Backend Boundary | Adopted | Implemented RHI foundation; native backend planned/research |
| [ADR-0007](ADR-0007-material-shader-ir.md) | Material and Shader IR Authority | Adopted | Transitional WGSL foundation; IR planned |
| [ADR-0008](ADR-0008-isobar-basalt-torsant-boundaries.md) | Isobar, Basalt, and Torsant Boundaries | Adopted | Isobar/Basalt scaffold; Torsant planned |
| [ADR-0009](ADR-0009-editor-first.md) | Editor-first Product Architecture | Adopted | Transitional editor shell; Meridian UI planned |
| [ADR-0010](ADR-0010-cairn.md) | Cairn Physics Ownership | Adopted | Transitional Rapier wrapper; Cairn planned |
| [ADR-0011](ADR-0011-data-authority.md) | Source Data Authority | Adopted | Partial data foundations |
| [ADR-0012](ADR-0012-luau.md) | Luau as Initial High-level Runtime | Amended by ADR-0019 | Planned after the Rust-first gameplay foundation |
| [ADR-0013](ADR-0013-typed-commands-agents.md) | Typed Commands and Agent Access | Adopted | Command model planned/partial; agents deferred |
| [ADR-0014](ADR-0014-optional-capability-packs.md) | Optional Capability Packs | Adopted | Policy implemented in specs; verification partial |
| [ADR-0015](ADR-0015-security-update-trust.md) | Security and Update Trust | Adopted | Policy adopted; release implementation planned |
| [ADR-0016](ADR-0016-redacted-private-benchmark-policy.md) | Redacted Private Benchmark Policy | Adopted | Policy adopted; calibrated corpus planned |
| [ADR-0017](ADR-0017-alluvium.md) | The Alluvium Engine | Adopted | Planned architecture; no implementation crate |
| [ADR-0018](ADR-0018-general-purpose-single-application.md) | General-Purpose Engine and Single Meridian Application | Adopted | Planned architecture; existing foundations only |
| [ADR-0019](ADR-0019-rust-first-luau-after.md) | Rust-First Gameplay, Optional Luau Afterward | Adopted | Planned |
| [ADR-0020](ADR-0020-wavefront-and-collective.md) | Wavefront Audio and Collective Online Boundaries | Adopted | Wavefront scaffold; Collective deferred |
| [ADR-0021](ADR-0021-first-class-two-dimensional.md) | First-Class Two-Dimensional Architecture | Adopted | Planned |
| [ADR-0022](ADR-0022-native-modeler.md) | Meridian Native Modeler and Optional Blender Companion | Adopted | Planned |
| [ADR-0023](ADR-0023-meridian-shader-language.md) | Meridian Shader Language and Shared Shader IR | Adopted | Planned; WGSL remains transitional |
| [ADR-0024](ADR-0024-post-one-programs.md) | Post-1.0 Programs Do Not Extend the Core Milestone Gate | Adopted | Governance architecture adopted; programs deferred |

## Template

Use [ADR-0000](ADR-0000-template.md) for future records.
