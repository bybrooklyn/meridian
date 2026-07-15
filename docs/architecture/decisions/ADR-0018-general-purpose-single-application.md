# ADR-0018: General-Purpose Engine and Single Meridian Application

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.5
- Implementation status: Planned architecture; existing foundations only
- Owners: architecture, runtime, editor, build, release
- Amends: ADR-0002, ADR-0009, ADR-0014
- Supersedes: none

## Context

Project Meridian is the first proving game, but Meridian must support unrelated 2D, 3D, headless, networked, stylized, simulation, action, strategy, and creator-tool projects without importing Project Meridian assumptions into engine contracts. The v0.4 suite has strong subsystem boundaries but lacks explicit animation, navigation, framework, 2D, modeler, shader-language, online-ecosystem, distributed-world, and integrity authorities.

The product must remain understandable. Separate branded technologies do not imply a collection of unrelated user-facing applications.

## Decision

Meridian is one user-facing application with selectable workspaces, capabilities, and project profiles. Editor, modeler, animation, IDE, profiling, VCS, build, and service-management experiences share the same command, document, permission, undo, accessibility, and diagnostics infrastructure.

Meridian may launch supervised helper processes and expose CLI/headless tools for compilers, importers, cookers, language servers, crash handlers, plugins, builds, tests, servers, and automation. Process isolation does not create a separate product authority.

New governed domains are `ANI`, `NAV`, `FWK`, `TWO`, `SHD`, `MDL`, `COL`, `WRL`, and `INT`. Domain codes organize requirements and evidence; they are not required runtime API or crate names.

Engine-owned foundations cover cross-project identity, schemas, scheduling, diagnostics, build/package, rendering/physics/audio/input interfaces, navigation infrastructure, animation data/runtime, accessibility, and security. Optional official modules cover genre frameworks, Collective services, advanced simulations, distributed worlds, and integrity features. Games retain rules, balance, narrative, economy, specialized behavior, and unusual project-specific simulation.

## Consequences

- Low-level crates never depend on Project Meridian, genres, editor UI, Collective, Worlds, or integrity modules.
- Optional domains have zero-cost-disabled build and runtime evidence.
- Product branding may be reusable across projects without creating mandatory dependencies.
- The master suite remains federated; no monolithic duplicate specification becomes authority.
- Project Meridian sequencing remains editor-first and does not wait for every long-term capability.

## Review

Review when a new domain is proposed, when a subsystem becomes a separately shipped application, or when optional modules leak into a minimal/headless profile.
