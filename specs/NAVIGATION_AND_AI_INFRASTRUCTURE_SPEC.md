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
AgentProfile {
  id: u32,                              // stable per-project profile identifier
  shape: AgentShape,                    // Capsule { radius, height } | Box { extents }
  movement_modes: Vec<MovementMode>,    // default max 8 modes per profile (Walk, Swim, Fly, ...)
  slope: f32,                           // max traversable slope, degrees, 0.0-90.0
  step: f32,                            // max step height, meters
  clearance: f32,                       // min vertical clearance, meters
  capabilities: u32,                    // bitflags: jump, climb, crouch, ...
}
NavigationSourceSnapshot {
  world_revision: u64,
  geometry_refs: Vec<GeometryRef>,
  semantic_fields: Vec<SemanticFieldRef>,
  dynamic_obstacles: Vec<ObstacleRef>,  // default max 4,096 live obstacles per streaming cell
}
NavigationArtifact {
  id: u128,
  schema: (u16 major, u16 minor),
  representation: RepresentationKind,   // NavMesh | Grid | VoxelField | Waypoints | Hybrid
  bounds: Aabb,
  tiles: Vec<TileRef>,
  links: Vec<OffMeshLink>,
  costs: Vec<CostField>,
  provenance: ProvenanceRef,
}
PathRequest {
  request_id: u64,
  frame_id: u64,
  agent: AgentProfile,
  start: Vec3,
  goals: Vec<Vec3>,                     // default max 8 candidate goals per request
  filters: NavigationFilterMask,        // u32 bitflags over semantic cost classes
  budget: Duration,                     // default reference bound 4 ms per request at Standard tier
  determinism: DeterminismMode,         // Deterministic | BestEffort
}
PathResult {
  request_id: u64,
  status: PathStatus,                   // Complete | Partial | Unavailable | Cancelled | Failed
  corridor: Vec<TileRef>,
  points: Vec<Vec3>,
  costs: PathCostBreakdown,
  partial_reason: Option<PartialReason>,
  evidence: PathEvidence,
}
CrowdRequest {
  agent_id: u32,
  preferred_velocity: Vec2,
  neighbors: Vec<NeighborRef>,          // default max 32 neighbors considered per agent per tick
  constraints: Vec<CrowdConstraint>,
  budget: Duration,                     // default reference bound 1 ms per agent per tick
}
NavigationTrace {
  inputs: TraceInputSummary,
  visited_regions: Vec<RegionId>,
  rejected_edges: Vec<(EdgeRef, RejectReason)>,
  costs: PathCostBreakdown,
  timings: TraceTimings,
  artifact_hashes: Vec<u64>,
}
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

## 7.1 Work package briefs

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status changes.

**`WP-NAV-001` — Navigation sources, artifacts, profiles, queries, and streaming baseline**
Result: a creator marks walkable/excluded regions, chooses an agent profile,
builds, visualizes reachability, and runs a path test (§6's beginner
workflow) end to end. Owning contracts: `AgentProfile`,
`NavigationSourceSnapshot`, `NavigationArtifact`, `PathRequest`/`PathResult`
(§3). Entry conditions: none implemented yet per this doc's current-status
line — this package is the first navigation work; it depends on Basalt/
Alluvium emitting the source geometry/semantic fields it consumes (§2), not
on their full implementation maturity. Deliverables: the build pipeline in
§4 (capture snapshot → validate → partition by streaming cell → build
representation → validate connectivity/clearance/costs/links → compare
against accepted artifact → atomically publish), the runtime query pipeline
(accept bounded query → resolve resident generations → hierarchical search →
refine under budget → validate dynamic obstacles → typed result), and the
streaming baseline (query identity preserved across transitions, stale
generations reported explicitly). Non-goals: no crowd/flow solver, no
dynamic links, no GPU acceleration — those are `WP-NAV-002` (§7's tier
ordering: "baseline local path queries" before "streamed multi-profile...
dynamic links" before "crowd/flow/GPU research"). Forbidden edges: no AI
decisions inside the nav service, no renderer triangles as mandatory source
authority, no unbounded synchronous queries on latency-critical threads, no
transient tile handles in saves (§2). Failure/recovery: saturation produces
backpressure or typed partial/unavailable results, never unbounded queues
(§5); the previous accepted artifact remains available when a rebuild fails
(§5). Tests: malformed geometry, tile seam continuity, multiple profiles,
partial paths, stale generations, determinism, 2D/3D fixtures (§7, scoped to
this package's baseline subset). Stop condition: a representation that
cannot pass connectivity/clearance/seam validation does not publish — the
prior accepted artifact stays authoritative. Next unblocked: `WP-NAV-002`.

**`WP-NAV-002` — Dynamic navigation links, crowds, flow, replay, and advanced profiling**
Result: dynamic obstacles/links update region-bounded without full rebuilds,
crowd/flow queries return usable results under budget, and replay/advanced
profiling (§6's expert workflow: hierarchy, cost fields, off-mesh links,
query trace, replay divergence) are available. Entry conditions: `WP-NAV-001`
closed — this package extends its artifact/query contracts, it does not
redefine them. Deliverables: `CrowdRequest` handling (§3), region-bounded
dynamic updates (§4), deterministic replay mode, and any GPU-accelerated path
kept behind a required reference implementation with differential evidence
(§5 — GPU acceleration "must preserve a reference path," never replace it
unconditionally). Non-goals: still no universal game-AI brain or hidden NPC
behavior — gameplay/framework code retains intent, goals, and tactics
ownership (§1, §2). Tests: dynamic obstacles, cancellation, crowd stress,
replay divergence, private-content redaction in debug exports (§7, §6).
Stop condition: if GPU-path differential evidence against the CPU reference
fails, ship CPU-only and keep the GPU path research/debug-only. Next
unblocked: streamed multi-profile and crowd/flow tiers close per §7; NAV
becomes a real dependency for framework movement/AI work (`WP-FWK-001`) that
wants navigation-driven agents.

## 8. Examples

End to end: Alluvium emits walkability fields for a Basalt cell; NAV builds a tile; gameplay requests a path; Artus consumes locomotion targets and proposes movement; Cairn resolves it.

Failure: a destination cell is not resident. The result is `Partial` with a stable frontier and dependency, allowing gameplay to wait or choose another action.

Performance debug: a traversal hitch trace separates streaming wait, tile generation, coarse search, local refinement, and dynamic-obstacle revalidation.
