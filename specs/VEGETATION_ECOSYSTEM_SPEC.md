# Vegetation Ecosystem Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Basalt](BASALT_TERRAIN_AND_LARGE_WORLD_GEOMETRY_SPEC.md) · [Isobar](ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md) · [Torsant](TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md) · [Penumbra](RENDERING_AND_GRAPHICS_SPEC.md)

version 0.5 · 2026-07-15 · Normative vegetation architecture

Documentation maturity: `ResearchReady`. Implementation maturity: `Scaffold`.
Governing IDs: `REQ-VEG-001`, `WP-VEG-001`.

## 1. Goals and non-goals

Vegetation owns plant instances, species/variant metadata, deterministic
placement results after generation, growth/damage state, LOD policy inputs,
interaction events, and vegetation-specific runtime snapshots. Alluvium owns
species/placement/ecosystem recipes, suitability fields, candidate generation,
generated identity, overrides, and cooking. Vegetation consumes those outputs,
Basalt geometry and surface authority, Isobar wind/moisture/weather fields, and
optional Torsant heat/fire/thermal events.

Goals are dense scalable vegetation, coherent environmental response,
deterministic source-to-artifact rebuilding, preserved manual overrides,
path-independent snapshots, accessible authoring/debugging, and zero work when
the vegetation pack is disabled.

Non-goals are owning terrain, weather, fire/fluid solvers, renderer resources,
Alluvium evaluator/source recipes, game-specific route logic, a universal
ecosystem solver, or claiming the
current `meridian-vegetation` marker crate is implementation evidence.

## 2. Ownership and forbidden edges

Vegetation may depend on core IDs/math, assets/world data, tasks, diagnostics,
Basalt queries, Isobar field contracts, optional Torsant event contracts, saves,
and typed Alluvium artifacts. Penumbra, Cairn, audio, and gameplay consume immutable
vegetation snapshots or typed events.

Forbidden edges:

- vegetation cannot allocate Penumbra resources or issue render passes;
- vegetation cannot maintain a private wind/weather RNG;
- Torsant cannot mutate species/source documents directly;
- vegetation runtime cannot invoke Alluvium editor/compiler internals or mutate
  recipe/override source;
- gameplay cannot depend on renderer LOD or private placement internals;
- disabled profiles cannot register tasks, listeners, queries, panels,
  resources, or package chunks.

## 3. Public contract direction

The following are planned names, not implemented runtime APIs:

```text
SpeciesId(u128 persistent)
VegetationInstanceId(u128 persistent)
VegetationRuntimeHandle { slot, generation }

VegetationSource {
  species,
  placement_provenance,
  persistent_transform,
  authored_overrides,
  growth_state,
}

VegetationSnapshot {
  epoch,
  origin_epoch,
  instances,
  geometry_batches,
  wind_response,
  thermal_events,
  interaction_proxies,
  lod_inputs,
}
```

Persistent source identity is independent from runtime handle and renderer
instance identity. Artists author species/material/placement once; renderer
paths may lower visual data differently without duplicating source vegetation.

## 4. Data authority and pipeline

Authoritative data is schema-defined species, Alluvium recipes and
placement/field inputs, manual instances, override stacks, growth/damage state
where persistent, and source provenance. Built placement sets, geometry batches, wind-response tables,
collision proxies, and renderer buffers are derived artifacts.

Ordered pipeline:

1. load species, accepted Alluvium placement outputs, seeds, Basalt surface data, and overrides;
2. validate units, bounds, density limits, provenance, and deterministic inputs;
3. generate or update only dirty placement regions;
4. preserve accepted/rejected candidates and manual override reasons;
5. resolve Basalt-relative transforms and geometry residency;
6. latch Isobar wind/moisture and optional Torsant events by epoch;
7. publish immutable vegetation snapshots for Penumbra, Cairn, audio, and gameplay;
8. record placement, LOD, interaction, coupling, memory, and fallback diagnostics.

## 5. Clocks, threading, memory, and lifetime

Placement/build work runs asynchronously with cancellation and content-addressed
cache identity. Persistent growth/damage advances on declared simulation clocks.
Presentation wind interpolation runs from immutable field snapshots; it does
not mutate source state. Snapshot epochs and origin epochs reject stale readers.

Hot runtime data uses bounded structure-of-arrays pages. Cold authoring metadata
and provenance stay out of render hot paths. Large geometry follows Basalt
residency; instance data follows declared budgets and generation-checked reuse.

## 6. Failure, recovery, diagnostics, security, and provenance

If generation fails, retain the last valid artifact and source/overrides. If a
cell or field snapshot is stale, retain the last allowed state, report age, or
downgrade according to profile. If a geometry artifact is corrupt, invalidate
only that artifact and rebuild. If Torsant is absent, fire/thermal events are a
supported empty input.

Diagnostics include source/seed/artifact hashes, accepted/rejected candidates,
manual overrides, active instances by tier, geometry residency, LOD
transitions, wind/thermal snapshot age, interaction proxies, CPU/GPU/memory,
overdraw contribution, and zero-cost-disabled assertions.

All imported meshes, masks, species documents, sidecars, and generated caches
are untrusted. Validate counts, paths, nesting, transforms, compression, and
hashes before allocation. Every borrowed asset or algorithm records provenance
and license; private Project Meridian assets never enter generated public
fixtures.

## 7. Accessibility and workflows

Vegetation cannot make required traversal, interaction, recovery UI, or critical
cues unreadable. Motion/reduced-effects profiles scale branch/leaf/grass
animation and temporal noise. Editor heatmaps have text/table alternatives,
semantic labels, keyboard navigation, and non-color-only status.

Beginner workflow: choose a species set, paint or generate a bounded region,
preview, accept, and undo. Expert workflow exposes seeds, masks, candidate
reasons, field epochs, density/LOD/overdraw/residency budgets, diff/regenerate,
and provenance. CLI/agent operations use typed previewable commands and preserve
manual overrides.

## 8. Capability tiers and disabled behavior

| Tier | Contract |
|---|---|
| Disabled | No vegetation crate work, tasks, listeners, queries, resources, panels, or chunks. |
| Prototype | Bounded manual/generated instances, conservative geometry LOD, simple Isobar wind response. |
| Representative forest | Dense grass/trees, multi-tier LOD, flashlight/shadow/alpha-tested foliage, streaming and temporal diagnostics. |
| Advanced | Growth/damage, richer interaction, procedural ecosystems, selected Torsant coupling after evidence. |
| Research | Specialized deformation, ecosystem simulation, and GPU-driven generation candidates behind stable snapshots. |

## 9. Tests, benchmarks, evidence, and delivery

Tests cover deterministic placement, dirty-region rebuild, override preservation,
source/artifact migration, snapshot epoch rejection, origin rebasing, Isobar
field coherence, optional Torsant absence/events, corrupt-cache recovery, and
disabled-profile zero work.

Permanent workloads: PEN-B01, PEN-B02, PEN-B03, PEN-B05, PEN-B06, PEN-B07,
PEN-B08, PEN-B09, PEN-B10, PEN-B11, PEN-B13, PEN-B14, and PEN-B15.

MS-04 establishes path-independent vegetation snapshots and Forward+ support.
`WP-PRC-004` supplies ecological placement and production authoring. MS-05
proves the representative forest. MS-06 consumes the foundation in the
private prototype after editor/forest gates. MS-07 validates production opening
content. MS-08 may activate richer Alluvium/Torsant integrations;
`WP-PRC-009` keeps ecosystem growth and succession in research until MS-09.

## 9.1 Work package brief (medium — Scaffold)

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md)
for the next real step beyond the current `meridian-vegetation` marker
crate. No status change.

**`WP-VEG-001` — Vegetation geometry and environmental coupling**
Result: plant instances, deterministic placement results after generation,
growth/damage state, LOD policy inputs, and vegetation-specific runtime
snapshots become real (§1) — MS-04's "path-independent vegetation snapshots
and Forward+ support" (§9). Owning crate: `meridian-vegetation`, explicitly
not evidenced by the current marker crate alone (§1's non-goals). Entry
conditions: `WP-PRC-004` supplies ecological placement and production
authoring (§9) as the Alluvium-side input this package consumes, not
authors; Basalt geometry/surface authority and Isobar wind/moisture fields
(§1) through their own published contracts. Deliverables: the public
contract direction in §3, the data authority/pipeline in §4, and
disabled-pack zero-work behavior (§8). Non-goals: no terrain, weather, or
fire/fluid solver ownership; no Alluvium evaluator/source recipes; no
universal ecosystem solver (§1) — growth/competition/succession/season/
disturbance stays `WP-PRC-009`, held in research until MS-09 (§9); this
package does not preempt that gate. Forbidden edges: cannot allocate
Penumbra resources or issue render passes; cannot maintain a private wind/
weather RNG (§2) — wind/weather state comes from Isobar's published
snapshots only. Tests: the §9 list (deterministic placement, dirty-region
rebuild, override preservation, source/artifact migration, snapshot-epoch
rejection, origin rebasing, Isobar field coherence, optional Torsant
absence/events, corrupt-cache recovery, disabled-profile zero work). Stop
condition: an override made by a creator must survive dirty-region
regeneration rather than being silently discarded (§9's override-
preservation test) — this mirrors Alluvium's own generated-identity/
override rules (`PROCEDURAL_AUTHORING_SPEC.md` §9). Next unblocked: MS-05's
representative forest proof (§9), which this package is a direct
prerequisite for alongside `WP-BAS-001` and `WP-ISO-001`.

## 10. Examples

End-to-end: an artist selects species and a Basalt region, generates candidates
from a deterministic recipe, rejects protected-route overlaps, preserves manual
trees, builds artifacts, and previews the same Isobar wind snapshot in Creator
Editor and Penumbra.

Failure/recovery: generation crashes after writing temporary output. The prior
artifact remains active, source and overrides are untouched, temporary output is
discarded by hash/transaction rules, and retry resumes the dirty region.

Performance debug: PEN-B02 reports frame/pass distributions, vegetation counts,
geometry residency, LOD transitions, alpha overdraw, shadow casters, Isobar
field cost, temporal rejection, pipeline state, and memory. A density/LOD fix is
compared with identical seed, camera, hardware, profile, and cache state.
