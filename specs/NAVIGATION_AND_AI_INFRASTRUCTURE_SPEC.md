# Navigation and AI Infrastructure Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Assets and worlds](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Cairn](CAIRN_PHYSICS_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Gameplay frameworks](OFFICIAL_GAMEPLAY_FRAMEWORKS_SPEC.md) · [Delivery](DELIVERY_ROADMAP.md)

Status: version 0.5 normative architecture, 2026-07-15.

Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-NAV-001` through `REQ-NAV-003`; `WP-NAV-001`; `WP-NAV-002`.

Current implementation status: no Meridian navigation representation, query service, crowd solver, or navigation editor is implemented.

## 1. Authority, Goals, and Non-Goals

The `NAV` domain owns derived traversability data, query contracts, path and flow results, dynamic-obstacle integration, crowd-motion infrastructure, streaming seams, debug traces, and navigation artifact validation. Gameplay and framework code own goals, tactics, behavior, perception interpretation, and whether an agent follows a result.

Goals are deterministic or explicitly nondeterministic query modes, large-world streaming, multiple agent profiles, semantic costs and exclusions, asynchronous bounded requests, dynamic changes, 2D/3D support, and explainable debugging. Non-goals are a universal game-AI brain, hidden NPC behavior, direct ownership of animation or physics, or one representation forced on every project.

## 2. Ownership and Data Authority

| Boundary | NAV owns | Neighbor owns |
|---|---|---|
| Basalt/Alluvium | compiled navigation surfaces, tiles, links, fields | source terrain, geometry, semantic regions, generation inputs |
| Cairn | collision/query snapshots consumed for updates | physical bodies, contacts, sweeps, final movement |
| Gameplay/FWK | query and crowd results | intent, goals, path acceptance, behavior, tactics |
| Artus (`ANI`) | locomotion target and link-transition metadata | intent production, animation selection, root proposal, contacts, and IK |
| Streaming | navigation artifact dependencies and readiness | cell residency, activation, eviction |
| NET/saves | stable navigation state needed for replay | transport, authority, persistence transactions |

Source geometry is not duplicated as navigation authority. Navigation artifacts are derived, versioned, replaceable, and traceable to exact world/facet hashes. Forbidden edges include AI decisions in the nav service, renderer triangles as mandatory source authority, unbounded synchronous queries on latency-critical threads, and transient tile handles in saves.

## 3. Planned Contracts

```text
AgentProfile { id, shape, movement_modes, slope, step, clearance, capabilities }
NavigationSourceSnapshot { world_revision, geometry_refs, semantic_fields, dynamic_obstacles }
NavigationArtifact { id, schema, representation, bounds, tiles, links, costs, provenance }
PathRequest { request_id, frame_id, agent, start, goals, filters, budget, determinism }
PathResult { request_id, status, corridor, points, costs, partial_reason, evidence }
CrowdRequest { agent_id, preferred_velocity, neighbors, constraints, budget }
NavigationTrace { inputs, visited_regions, rejected_edges, costs, timings, artifact_hashes }
```

Representations may include meshes, grids, voxel/volume fields, waypoints, flow fields, or hybrids. Selection is capability- and workload-driven; public semantics do not expose a donor library's types.

## 4. Build and Runtime Pipelines

```text
capture immutable world/facet snapshot
-> validate units, bounds, agent profiles, and source provenance
-> partition by streaming cell and affected halo
-> build candidate representation
-> validate connectivity, clearance, costs, links, and seams
-> compare against accepted artifact and replay corpus
-> atomically publish generation-checked artifacts
```

```text
accept bounded query at simulation barrier
-> resolve resident generations and semantic filters
-> perform hierarchical/coarse search
-> refine corridor/path/flow under budget
-> validate dynamic obstacles and links
-> return complete, partial, unavailable, cancelled, or failed result
-> gameplay decides follow-up
```

Dynamic updates are region-bounded. Streaming transitions preserve query identity, report stale generations, and never silently reinterpret coordinates.

## 5. Time, Threads, Memory, and Failure

- Authoritative query ordering records frame/tick, request sequence, artifact generation, and determinism mode.
- Builds and long searches run on cancellable workers; short local queries may use bounded task slices.
- Tiles and query scratch use explicit budgets, residency priorities, and generation checks.
- Saturation produces backpressure or typed partial/unavailable results, not unbounded queues.
- Device loss is irrelevant to baseline CPU navigation; optional GPU acceleration must preserve a reference path and differential evidence.

Failures include invalid source, disconnected required route, stale tile, missing cell, exceeded node/time budget, corrupt artifact, unsupported movement mode, and cancellation. The previous accepted artifact remains available when safe. Diagnostics expose visited regions, cost composition, rejected links, dynamic blockers, queue wait, execution time, memory, and source hashes.

## 6. Security, Accessibility, and Workflows

Untrusted navigation artifacts and project scripts cannot invoke arbitrary code or bypass capability checks. Networked clients cannot authoritatively inject paths or crowd state. Debug exports redact private scene labels and content where required.

Beginner workflow: mark walkable/excluded regions, choose an agent profile, build, visualize reachability, and run a path test. Expert workflow: inspect tiles, hierarchy, cost fields, off-mesh links, streaming seams, query trace, memory, replay divergence, and rebuild invalidation.

Visual overlays have keyboard/textual query alternatives, color-independent legends, scalable labels, and screen-reader descriptions of selected paths and failures.

## 7. Tiers, Requirements, and Delivery

Tiers are baseline local path queries; streamed multi-profile navigation and dynamic links; then crowd/flow/GPU research. Disabled navigation or crowd modules create no world components, workers, or package data.

- `REQ-NAV-001`: versioned, representation-neutral navigation artifacts and queries with build, migration, and differential evidence.
- `REQ-NAV-002`: bounded asynchronous queries, streaming seams, dynamic updates, and deterministic replay modes with failure evidence.
- `REQ-NAV-003`: game-neutral ownership, accessible editing, provenance, and explainable trace diagnostics.
- `WP-NAV-001`: source facets, artifact build, profiles, query API, debug view, and streaming baseline.
- `WP-NAV-002`: dynamic updates, links, crowds/flows, replay, and advanced profiling.

Tests include malformed geometry, tile seam continuity, multiple profiles, partial paths, stale generations, dynamic obstacles, cancellation, saturation, determinism, replay, 2D/3D fixtures, crowd stress, and private-content redaction. Benchmarks report latency distributions, queue wait, visited nodes, rebuild area, memory, and artifact churn.

## 8. Examples

End to end: Alluvium emits walkability fields for a Basalt cell; NAV builds a tile; gameplay requests a path; Artus consumes locomotion targets and proposes movement; Cairn resolves it.

Failure: a destination cell is not resident. The result is `Partial` with a stable frontier and dependency, allowing gameplay to wait or choose another action.

Performance debug: a traversal hitch trace separates streaming wait, tile generation, coarse search, local refinement, and dynamic-obstacle revalidation.
