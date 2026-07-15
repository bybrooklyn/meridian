# Cairn Physics Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Migration register](SPEC_MIGRATION_AND_CONTRADICTIONS.md)

Status: normative specification, version 0.2, 2026-07-14.

Cairn is Meridian's in-tree physics family. It is a provenance-first hard-fork
path from pinned Rapier plus selected Box2D study and ports where licensing,
quality, and fit justify the work. Rapier API compatibility is not a goal.
Current `meridian_physics` Rapier integration is transitional bootstrap
evidence, not the final Cairn architecture.

Rust blocks in this document are schema/API contracts or pseudocode unless the
status table says the current crate already implements that surface. Planned
snippets are not compile-tested because the corresponding APIs do not exist
yet.

## 1. Current Status

| Area | Status | Evidence and limit |
|---|---|---|
| Grounded controller | Implemented foundation | `meridian_physics` owns an engine-side grounded controller with fixed-step movement, crouch, slope handling, reset, and no jump/sprint contract for the current game movement. |
| Rapier wrapper | Transitional | `RapierWorld`, `RapierCharacter`, and opaque collider IDs wrap static boxes, fixed stepping, kinematic character movement, and controller correction. This does not define permanent Cairn API compatibility. |
| Cairn provenance fork | Planned | Exact Rapier and Box2D commits, license archive, provenance manifest, differential tests, and baseline benchmarks still need Phase 3 execution. |
| Data-oriented storage | Planned | Body/shape/solver storage must move to Cairn-native handles, hot/cold fields, and SIMD-aware layouts. |
| Structural destruction | Planned flagship | Destructible connected structures are Cairn's first flagship, not a pile-of-boxes demo. |
| Determinism modes | Planned | Stable, deterministic, and strict replay modes require measured platform guarantees and costs. |

## 2. Context

Meridian needs grounded first-person movement for the opening slice, editor
physics previews, future VR interaction, structural destruction, deformables,
vehicles, multiplayer prediction, and headless server simulation. The current
Rapier wrapper is useful for service tests and early gameplay movement, but
Meridian needs owned persistent IDs, data-oriented storage, provenance control,
determinism modes, editor diagnostics, structural graph semantics, and
zero-cost feature tiers.

## 3. Goals

- Establish Cairn as an in-tree Meridian crate family.
- Preserve clear provenance for every forked, ported, rewritten, or studied
  subsystem.
- Build a Cairn-native public API immediately instead of preserving Rapier
  surface compatibility.
- Use stable generational handles and schema descriptors at public boundaries.
- Separate hot solver data from cold metadata.
- Support rigid bodies, queries, sensors, constraints, character/hand
  controllers, CCD, and deterministic scheduling.
- Make destructible connected structures the first flagship.
- Support optional deformables, vehicles, VR interaction, networking, and
  advanced simulation without recurring cost when disabled.

## 4. Non-goals

- Do not promise Rapier API compatibility.
- Do not import Box2D or Rapier code without exact commit, license, and
  provenance records.
- Do not expose Rapier handles, raw pointers, ECS entities, or game-specific
  IDs as persistent physics identity.
- Do not claim bit-identical cross-platform determinism without proof.
- Do not require Cairn in headless or app builds that do not use physics.
- Do not implement structural destruction by only spawning disconnected debris.
- Do not force all deformables, fluids, or structural behavior into one solver
  if specialized solvers are better after measurement.

## 5. Ownership and Crate Boundaries

| Crate or tool | Owns | Must not own |
|---|---|---|
| Planned `meridian-cairn-core` | Handles, descriptors, body/shape/material schema, broadphase/narrowphase traits, deterministic ordering contracts | Renderer resources, world-source editing UI, game content |
| Planned `meridian-cairn-solver` | Rigid solver, islands, constraints, warm start, sleeping, CCD | Persistent save format interpretation |
| Planned `meridian-cairn-structure` | Structural graph, bonds, fracture operations, support/load propagation | Visual mesh rendering policy |
| Planned `meridian-cairn-deform` | Cloth/rope/soft-body/granular optional solvers | Required baseline rigid-body simulation |
| `meridian-physics` transitional | Current Rapier-backed wrapper and grounded-controller tests | Permanent Cairn ownership once fork exists |
| `meridian-world` | Spatial cells, origin rebasing, source IDs for physics entities | Solver internals |
| `meridian-save` | Persisted stable physics state and schema migrations | Direct solver memory dumps |
| Editor physics tools | Debug panels, previews, authoring controls, stress/failure visualization | Hidden runtime state outside schemas |

Invalid dependencies:

- Cairn crates must not depend on game crates.
- Core solver crates must not depend on renderer, audio, UI, or editor crates.
- Public Cairn APIs must not expose Rapier types.
- Structural destruction must not require the visual renderer to load high-res
  meshes on a server.
- Save data must serialize Cairn descriptors and state, not internal array
  indices without generation/version checks.

## 6. Provenance and Fork Procedure

Before modifying imported code:

1. Pin exact Rapier and Box2D repository commits.
2. Archive original licenses and notices.
3. Create a provenance manifest.
4. Record baseline correctness tests.
5. Record baseline performance tests.
6. Define which code is forked, ported, rewritten, or only studied.
7. Establish differential test harnesses against pinned upstream behavior.
8. Create an ADR for deviations that affect public API or solver behavior.

Provenance record:

```text
Original repository:
Original commit:
Original file or algorithm:
License:
SPDX identifier:
Use type: forked | ported | rewritten | studied
Intentional deviations:
Correctness tests:
Differential tests:
Benchmarks:
Review date:
```

Cairn must not become an untraceable mixture.

## 7. Public Types and APIs

### 7.1 Handles

```rust
pub struct CairnWorldId(pub u64);
pub struct CairnBodyId { pub index: u32, pub generation: u32 }
pub struct CairnColliderId { pub index: u32, pub generation: u32 }
pub struct CairnConstraintId { pub index: u32, pub generation: u32 }
pub struct CairnStructureId { pub index: u32, pub generation: u32 }
pub struct CairnMaterialId(pub u128);
```

Handles are stable only within their declared world lifetime. Persistent save
and source data use stable source IDs that map to runtime handles at load time.

### 7.2 Descriptors

```rust
pub struct RigidBodyDesc {
    pub body_type: BodyType,
    pub transform: Isometry,
    pub velocity: BodyVelocity,
    pub mass: MassProperties,
    pub damping: Damping,
    pub flags: BodyFlags,
    pub determinism_class: DeterminismClass,
}

pub struct ColliderDesc {
    pub shape: ShapeDesc,
    pub material: CairnMaterialId,
    pub layer: CollisionLayer,
    pub filter: CollisionFilter,
    pub sensor: bool,
}

pub enum ShapeDesc {
    Sphere { radius: f32 },
    Capsule { radius: f32, half_height: f32 },
    Box { half_extents: Vec3 },
    Cylinder { radius: f32, half_height: f32 },
    Cone { radius: f32, half_height: f32 },
    ConvexHull { points: ArtifactRef },
    Compound { children: Vec<ChildShapeDesc> },
    TriangleMesh { artifact: ArtifactRef },
    Heightfield { artifact: ArtifactRef },
    Sdf { artifact: ArtifactRef },
    VoxelField { artifact: ArtifactRef },
    CustomSupportMapped { type_id: u128, params: SchemaValue },
}
```

### 7.3 World API

```rust
pub trait CairnWorld {
    fn create_body(&mut self, desc: RigidBodyDesc) -> Result<CairnBodyId, CairnDiagnostic>;
    fn create_collider(
        &mut self,
        body: Option<CairnBodyId>,
        desc: ColliderDesc,
    ) -> Result<CairnColliderId, CairnDiagnostic>;
    fn create_constraint(&mut self, desc: ConstraintDesc)
        -> Result<CairnConstraintId, CairnDiagnostic>;
    fn step(&mut self, step: CairnStep) -> Result<CairnEvents, CairnDiagnostic>;
    fn query(&self) -> &dyn CairnQueryApi;
    fn snapshot(&self, filter: SnapshotFilter) -> CairnSnapshot;
}
```

No method returns mutable aliases into hot solver arrays.

## 8. Data-oriented Storage

Baseline hot body storage:

```rust
pub struct BodySet {
    pub generations: Vec<u32>,
    pub flags: Vec<BodyFlags>,
    pub position: Vec<Isometry>,
    pub linear_velocity: Vec<Vec3>,
    pub angular_velocity: Vec<Vec3>,
    pub inverse_mass: Vec<f32>,
    pub inverse_inertia: Vec<Mat3>,
    pub collider_ranges: Vec<Range<u32>>,
    pub sleep_state: Vec<SleepState>,
}
```

Requirements:

- Hot fields used by integration and solver loops are contiguous.
- Cold metadata, names, source IDs, diagnostics, editor notes, and provenance
  live outside solver-hot arrays.
- Stable handle maps survive compaction by generation checks.
- AoSoA blocks may be used for SIMD after benchmark evidence.
- Broadphase proxy storage is separate from collider authoring metadata.
- Structural graph state is separate from rigid-body solver rows.

## 9. Ordered Pipelines and State Machines

### 9.1 Fixed-step Rigid Pipeline

```text
collect commands
-> validate descriptors and handle generations
-> integrate external forces
-> predict unconstrained velocities
-> update broadphase proxies
-> generate candidate pairs
-> narrowphase and manifold refresh
-> build islands
-> build solver rows
-> color or partition constraints when useful
-> warm start
-> velocity iterations
-> integrate poses
-> position stabilization or substeps
-> CCD resolution
-> sleeping analysis
-> publish events and immutable snapshot
```

### 9.2 Body State Machine

```text
Created
-> Active
-> Sleeping
-> Active
-> PendingRemoval
-> Removed
```

Kinematic bodies bypass force integration but participate in broadphase,
narrowphase, queries, contacts, and event generation according to descriptor
policy.

### 9.3 Streaming Physics State

World cells can load collision independently from visual facets:

```text
Source descriptor
-> compiled collision artifact
-> broadphase proxy metadata
-> inactive loaded colliders
-> active simulation bodies
-> sleeping or metadata-only eviction state
```

Activation and eviction must preserve stable source IDs and wake semantics.

## 10. Broadphase, Narrowphase, Solver, and CCD

### 10.1 Broadphase Portfolio

- Dynamic BVH baseline for general dynamic bodies.
- Sweep-and-prune for coherent dense regions if measured useful.
- Uniform or multilevel grids for debris and particles.
- Static tiled BVHs for source-world geometry.
- Expert region control for unusual workloads.

Required details:

- fat AABB expansion policy;
- deterministic pair ordering;
- region migration;
- pair cache invalidation;
- large-world coordinate strategy;
- streaming-cell proxy ownership.

### 10.2 Narrowphase Portfolio

Required shape-pair coverage research:

- analytic primitive tests;
- SAT for boxes and selected polyhedra;
- GJK for convex support shapes;
- EPA or alternative penetration depth;
- MPR alternative;
- convex-mesh traversal;
- heightfield contacts;
- SDF contacts;
- voxel-field contacts;
- persistent manifold refresh and reduction.

The selected baseline must define tolerances, feature IDs, warm-start keys,
degenerate input handling, and fuzz tests.

### 10.3 Solver Portfolio

Compare and assign by constraint class:

- sequential impulse or projected methods;
- Baumgarte stabilization;
- split impulse;
- nonlinear position correction;
- temporal Gauss-Seidel/substepping;
- XPBD for compliant constraints and deformables;
- specialized structural and vehicle solvers.

No solver constant is fixed by this document. Constants are versioned and must
come from measured test scenes.

### 10.4 CCD Portfolio

- discrete;
- speculative contact;
- swept shape or conservative advancement;
- substepping;
- per-body and per-pair overrides.

Thresholds, rotational limitations, complex mesh fallbacks, and tunneling tests
must be explicit.

## 11. Threading, Memory, and Lifetime

- Physics writes occur on the fixed-step schedule.
- Queries during simulation use snapshots or read-only query structures, not
  unsynchronized hot arrays.
- Solver islands may run in parallel when body write sets are disjoint.
- Graph coloring may be used for large islands; small islands may use scalar
  fallback when coloring overhead dominates.
- Strict replay mode may reduce parallelism.
- Events are published after the step as immutable records.
- Render interpolation consumes previous/current physics transforms; render
  threads do not access mutable solver state.
- Save snapshots occur at synchronization points and serialize stable IDs plus
  schema state, not raw pointers or borrowed solver memory.

## 12. Persistence, Versioning, and Compatibility

Persistent physics records include:

- stable source entity ID;
- body descriptor schema version;
- collider descriptor schema version;
- material facet IDs;
- current transform and velocity when save-authoritative;
- sleep state when relevant;
- structural graph state;
- deterministic solver mode and constants version;
- platform/build identity for strict replay records.

Compatibility policy:

- Cairn may break internal storage freely behind stable schema migrations.
- Rapier compatibility is not promised.
- A saved game created from transitional Rapier-backed builds must migrate
  through explicit descriptor records, not by replaying Rapier internals.
- Strict replay files name exact engine build, platform, solver constants,
  floating-point mode, and feature set.

## 13. Editor, CLI, MCP, and Workflows

### 13.1 Beginner Workflow

1. User chooses a collision preset such as "walkable floor", "thin trigger",
   or "heavy movable object".
2. Editor creates body/collider descriptors with safe defaults.
3. Viewport previews collision and grounded movement.
4. Diagnostics explain invalid mass, missing collision facet, steep slopes, or
   expensive destruction settings in plain language.
5. Play uses fixed-step simulation and records recoverable save state.

### 13.2 Expert Workflow

Expert panels expose:

- body, collider, constraint, island, and broadphase views;
- contact manifold data and warm-start IDs;
- sleeping/wake reasons;
- solver iteration and residual views;
- CCD paths;
- deterministic ordering and replay mode;
- structural graph, stress, support, and bond damage;
- fracture seeds and generated fragments;
- physics cost by world cell and feature tier;
- differential-test comparison with pinned upstream when applicable.

### 13.3 CLI and MCP Surface

Planned semantic command names:

| Domain | Commands |
|---|---|
| Physics scene | `inspect`, `validate`, `simulate`, `snapshot`, `replay` |
| Collision assets | `build`, `validate`, `diff`, `repair` |
| Determinism | `record`, `replay`, `compare` |
| Structure | `preview`, `stress`, `fracture`, `audit` |
| Provenance | `manifest`, `verify`, `diff-upstream` |

MCP tools must use the same command registry and must request capabilities for
project write, simulation execution, file export, and external process launch.

## 14. Diagnostics, Failure Recovery, and Security

Diagnostics require stable code, severity, source ID, affected handle, step
number, suggested fix, and documentation link.

Required diagnostics:

- invalid handle generation;
- non-finite transform, velocity, mass, inertia, or timestep;
- invalid collider dimensions;
- missing collision facet;
- unsupported shape pair;
- broadphase proxy corruption;
- CCD fallback activated;
- solver divergence or iteration cap;
- determinism mismatch;
- strict replay platform mismatch;
- fracture operation exceeded budget;
- structural graph disconnected unexpectedly;
- imported-code provenance missing;
- license or notice mismatch.

Failure recovery:

- Invalid commands are rejected before mutating hot storage.
- Step failure leaves the previous published snapshot available.
- Editor preview can reset to last checkpoint.
- Save replay can skip cosmetic physics events only when schema marks them
  non-authoritative; authoritative body state must fail or repair explicitly.
- Structural fracture generation stores seeds and operation parameters so it
  can be replayed or regenerated.

Security:

- Mod/plugin collision shapes are untrusted until validated for size, topology,
  non-finite values, and resource cost.
- Native physics plugins require elevated capability.
- Imported source code from Rapier/Box2D must preserve licenses and notices.
- MCP simulation commands cannot sign, publish, or mutate source without the
  appropriate capability and checkpoint.

## 15. Determinism Modes

| Mode | Contract |
|---|---|
| Fast | Prioritizes local performance; no replay guarantee beyond normal stability. |
| Stable | Deterministic ordering where cheap; suitable for editor previews and most single-player saves. |
| Deterministic | Stable handles, pair ordering, island construction, graph coloring, reductions, random seeds, and solver constants on a supported platform class. |
| Strict replay/network | Records platform/build identity and may reduce parallelism. Cross-platform bit identity is not claimed without proof. |

## 16. Structural Destruction

Structural graph:

```rust
pub struct StructuralBond {
    pub a: ChunkId,
    pub b: ChunkId,
    pub area: f32,
    pub normal: Vec3,
    pub tensile_limit: f32,
    pub shear_limit: f32,
    pub compressive_limit: f32,
    pub fatigue: f32,
    pub thermal_damage: f32,
}
```

Quality tiers:

- simple breakable joints;
- event-driven support graph;
- incremental approximate stress;
- high-quality structural solve;
- offline reference solve.

Runtime fracture pipeline:

1. Identify damaged volume and material.
2. Select authored or procedural fracture rule.
3. Generate cut cells, planes, or crack path.
4. Intersect source volume or mesh robustly.
5. Generate interior faces, UVs, and materials.
6. Generate mass properties.
7. Generate collision approximations.
8. Update structural graph.
9. Promote fragments under budget.
10. Cache/replay from seed and operation parameters.

Tiny debris may become particles or decals. Settled debris may merge into
static collision. Server authority may use cheaper representations than visual
debris.

## 17. Capability Tiers and Zero-cost-disabled Behavior

| Tier | Cairn behavior |
|---|---|
| No physics | Cairn crates, collision facets, solver tasks, and physics package chunks are omitted. |
| Queries only | Static collision/query acceleration only; no dynamic solver step. |
| Opening slice | Grounded controller, static environment collision, triggers/sensors, simple dynamic interactions. |
| Standard rigid | Dynamic bodies, joints, motors, CCD, character controller, deterministic snapshots. |
| Structural | Structural graph, breakable bonds, runtime fracture budget, debris policy. |
| Advanced | deformables, vehicles, VR hand solve, fluids/granular coupling, GPU secondary sims. |
| Research | Experimental solvers or determinism modes behind explicit flags and benchmark gates. |

Disabled tiers must register no fixed-step systems, allocate no solver arrays,
load no collision artifacts, and write no save records.

## 18. Algorithm Alternatives and Research Gates

| Decision | Alternatives | Gate |
|---|---|---|
| Fork baseline | Rapier stable, Rapier development revision, selected rewrite | Phase 3; correctness, maturity, license, performance, and API fit. |
| 2D algorithm reuse | Box2D study only, direct ports, rewrites | Phase 3/13; provenance, license, dimensional fit, and test value. |
| Broadphase | dynamic BVH, SAP, grids, tiled static BVH | Phase 3/8/13 scene corpus. |
| Penetration depth | EPA, MPR, SAT-specific paths, alternative contact generation | Phase 3 fuzz and differential tests. |
| Solver | sequential impulse, TGS, XPBD, split impulse, specialized structural solver | Phase 3 and Phase 13 benchmark scenes. |
| SIMD layout | SoA, AoSoA, scalar fallback, runtime dispatch | Phase 3/13; Apple Silicon first, then x86-64/Windows/Linux evidence. |
| Fracture geometry | authored chunks, Voronoi, robust booleans, voxel/level-set, tetrahedral | Phase 13; nonmanifold robustness and artist control. |

## 19. Tests, Benchmarks, and Acceptance Evidence

Required tests:

- handle generation and stale-handle rejection;
- descriptor validation;
- shape-pair contacts;
- manifold persistence;
- broadphase pair ordering;
- island construction ordering;
- solver convergence fixtures;
- CCD tunneling fixtures;
- deterministic replay fixtures;
- fuzz tests for degenerate geometry;
- differential tests against pinned Rapier for selected baseline behaviors;
- provenance manifest validation;
- grounded-controller movement and reset tests;
- structural bond break/reconnect tests;
- fracture seed replay;
- save migration for transitional Rapier-backed records.

Required benchmarks:

- static floor and grounded-controller service scene;
- stack stability;
- many sleeping bodies;
- dense dynamic debris;
- mixed static/dynamic forest collision;
- CCD projectile corridor;
- large-world origin rebase;
- structural collapse scene;
- fracture generation budget;
- strict replay overhead;
- Apple Silicon/NEON first, then Linux and Windows target evidence.

Acceptance evidence for Phase 3:

- pinned provenance manifest;
- differential harness;
- Cairn-native body/shape API;
- fixed timestep scene;
- grounded character collision;
- benchmark report;
- migration note freezing further Rapier-wrapper expansion.

Acceptance evidence for Phase 13:

- connected structure with authored/generated chunks;
- support graph and bond damage;
- collapse island promotion;
- fracture seed replay;
- visual/collision/server representation split;
- tests, benchmarks, and performance captures.

## 20. Phased Implementation

| Phase | Scope |
|---|---|
| Phase 3 | Cairn fork foundation, provenance, body/shape API, broadphase/narrowphase start, fixed timestep, character collision, differential suite. |
| Phase 5 | Collision facets integrate with asset/world/package formats. |
| Phase 8 | Opening-forest grounded movement, collision, triggers, save/export evidence. |
| Phase 13 | Structural destruction flagship. |
| Phase 19 | VR hands/controllers, grab constraints, haptic event extraction. |
| Phase 21 | Deformables, vegetation coupling, fire/thermal research prototypes. |
| Phase 22 | Multiplayer prediction, replication, rollback scope. |
| Phase 29 | 1.0 hardening and long-term compatibility. |

## 21. End-to-end Example

1. Asset import builds a wall asset with visual, collision, physical material,
   and structural facets.
2. World source places the wall in a cell with stable source entity ID.
3. Runtime loads only collision and physical/structural facets for server
   authority; client also loads visual facets.
4. Cairn maps source IDs to runtime body, collider, and structure handles.
5. Fixed-step simulation detects an impact, updates bond damage, and breaks one
   connection.
6. Fracture operation uses the recorded seed to generate fragments under the
   active budget.
7. Save records stable structure state and damage, not internal array indices.
8. Renderer consumes immutable transform/fragment snapshots, never mutable
   solver storage.

## 22. Failure and Recovery Example

Scenario: a source collision mesh contains non-finite vertices.

1. Import validation rejects the collision facet.
2. Diagnostic identifies source asset, facet, vertex range, and suggested fix.
3. Runtime package build excludes the invalid artifact and fails dependency
   closure if a required gameplay collision facet is missing.
4. Beginner editor shows "collision could not be built" and keeps the visual
   asset editable.
5. Expert panel offers mesh repair preview, writes repaired output separately,
   and requires explicit acceptance before replacing source metadata.

## 23. Performance-debug Example

Scenario: a structural collapse frame exceeds budget.

Expert profiler shows:

- active bodies and sleeping bodies;
- island sizes;
- broadphase pair count;
- contact manifold count;
- solver row count;
- graph-color count;
- fracture operation time;
- promoted fragment count;
- downgraded debris count;
- strict/deterministic mode;
- world cell and structure IDs.

Acceptable fixes include lowering structural quality tier, delaying fragment
promotion, merging tiny debris, changing bond thresholds, or moving high-quality
solve offline. Each change needs before/after benchmark evidence for the same
scene and hardware.
