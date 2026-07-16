# Competitive Performance and Quality Leadership Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Research](RESEARCH_AND_ALGORITHM_DECISIONS.md) · [Penumbra](RENDERING_AND_GRAPHICS_SPEC.md) · [Isobar](ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md) · [Torsant](TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md)

version 0.5 · 2026-07-16 · Normative post-1.0 competitive-evidence authority

Documentation maturity: `ResearchReady`. Implementation maturity: `Deferred`.
Governing IDs: `REQ-REL-002` through `REQ-REL-004`, `PRG-REL-001`,
`RG-REL-001`, and `VAL-REL-001`.

This specification owns Meridian's post-1.0 plan for seeking and proving
workload-specific performance and quality leadership. It does not claim that
Meridian currently outperforms another engine, and it cannot guarantee future
market leadership. It guarantees only the comparison, claim-expiry, review, and
regression rules that a future claim must pass.

## 1. Authority, entry gate, and scope

`PRG-REL-001` begins only after `MS-10` passes, declared 1.0 profiles are stable,
the relevant Meridian workloads are executable and calibrated, and a future
planning review activates bounded packages. Contract design before that point is
allowed so pre-1.0 systems do not close required seams; competitive optimization
work, public superiority claims, and program-completion credit are not.

The program owns:

- matched cross-engine comparison methodology and claim records;
- iso-quality performance, iso-cost quality, and matched-workflow studies;
- claim scope, expiry, renewal, retraction, and evidence retention;
- blind perceptual review and temporal-quality evaluation policy;
- convergence review for stutter, runtime cost prediction, and the shared
  environmental-performance contracts owned by Penumbra, Isobar, Torsant, and
  Alluvium;
- post-1.0 optimization sequencing, regression gates, and stop rules.

It does not own renderer algorithms, weather or simulation state, procedural
authoring, competitor source code, private game content, marketing approval, or
unqualified product-wide rankings.

## 2. Goals and non-goals

Goals:

- materially improve frame-time distributions, memory, power, storage,
  streaming, build/iteration latency, image quality, temporal stability, and
  environmental coherence on named workloads;
- prove each improvement on matched content, hardware, software, and feature
  profiles with raw evidence;
- make first-use stalls and lower-tier regressions visible rather than hiding
  them behind averages or high-end captures;
- preserve accessibility, recovery, security, portability, debuggability,
  authorability, and maintenance while optimizing;
- keep losing results and negative evidence so later decisions do not repeat
  failed experiments.

Non-goals:

- No permanent or universal claim that Meridian is faster or higher quality.
- No benchmark selected because Meridian already wins it.
- No private competitor code, disassembly-derived contract, license violation,
  trademark confusion, or circumvention of technical controls.
- No comparison that hides missing features, different content, different
  internal resolution, frame generation, dynamic resolution, cache state, or
  unsupported rows.
- No optimization that turns a research path into mandatory baseline cost.
- No post-1.0 program work may satisfy, block, or promote `MS-00` through
  `MS-10`.

## 3. Claim classes

Only these claim classes are permitted:

1. **Iso-quality performance**: blinded review and structural parity establish a
   preregistered quality-equivalence envelope; performance, memory, power,
   storage, or latency is then compared.
2. **Iso-cost quality**: the same calibrated frame, memory, power, storage, and
   hardware envelope is applied; quality and temporal behavior are then
   compared.
3. **Matched-workflow throughput**: the same source intent and accepted output
   are produced through import, authoring, build, shader warmup, package,
   recovery, or iteration workflows; elapsed distributions, intervention,
   failure, and output quality are compared.

A study may support more than one class only when each class has its own
preregistered threshold and analysis. A win in one class cannot be rewritten as
a win in another.

## 4. Planned competitive records

Illustrative planned contracts; no schema or runtime type is implemented:

```text
CompetitiveBaselineRecord {
  record_id,
  study_revision,
  claim_class,
  engine_and_exact_version,
  license_and_access_basis,
  source_checkpoint_or_public_build,
  hardware_os_driver_backend,
  display_and_power_state,
  capability_profile,
  source_corpus_and_asset_hashes,
  camera_input_and_workflow_recording_hashes,
  feature_parity_matrix,
  settings_internal_and_output_resolution,
  upscaler_frame_generation_dynamic_resolution,
  warmup_cold_warm_and_first_use_policy,
  sample_and_statistical_method,
  raw_sample_and_capture_hashes,
  structural_quality_result,
  perceptual_and_temporal_review_result,
  performance_memory_power_storage_results,
  workflow_and_maintenance_results,
  missing_features_and_known_limits,
  measurement_date,
  review_owner,
}

CompetitiveClaim {
  claim_id,
  baseline_record_ids,
  exact_scope,
  supported_profiles,
  wording,
  confidence_and_materiality,
  exclusions,
  approved_at,
  expires_at,
  renewal_trigger,
  retraction_state,
}
```

Every competitor version, Meridian BuildId, driver, corpus, setting, claim, and
raw artifact is immutable within a record. A new version creates a new record;
it never silently edits the old comparison.

## 5. Matched-comparison method

Before data collection, the study preregisters:

- user-visible question and claim class;
- competitors and exact versions;
- public or appropriately licensed source corpus and transformation rules;
- expected functional outcome and feature-parity matrix;
- hardware/capability profiles and unsupported behavior;
- internal/output resolution, upscaling, frame generation, dynamic resolution,
  quality settings, and display mode;
- camera/input/workflow recordings, warmup, cache, startup, and first-use state;
- metrics, repetitions, outlier handling, confidence, material threshold, and
  stop rule;
- blind review procedure, reviewer conflicts, accessibility review, security,
  provenance, and maintenance assessment;
- claim expiry and the competitor/Meridian changes that force renewal.

The same source intent need not produce byte-identical engine artifacts, but
accepted differences must be visible in the parity matrix. Missing or weaker
features are not normalized away. When a fair transformation is impossible,
the comparison is `Inconclusive` rather than adjusted until Meridian wins.

## 6. Quality evidence

Quality is evaluated in three independent layers:

- **structural**: expected geometry, material, light, weather, simulation,
  visibility, interaction, and state counts or invariants;
- **reference**: deterministic golden output, CPU/reference solver, offline
  render, measured real-world reference, or other preregistered authority where
  the domain admits one;
- **perceptual**: randomized/blinded pairwise or rating review with individual
  results, confidence, reviewer expertise, accessibility needs, and conflict
  disclosure.

Moving-image review additionally records shimmer, ghosting, disocclusion,
boiling, popping, exposure instability, latency, motion readability, and quality
changes between tiers. Environmental studies record wind coherence, fog/cloud/
smoke lighting, foliage silhouette response, wetness and water continuity,
fire/material response, audio agreement, and gameplay-state agreement.

An offline path trace, reference solver, metric, or reviewer is evidence, not a
universal aesthetic authority. Material differences require human review and a
documented product decision.

## 7. Performance, stutter, and workflow evidence

Every applicable study records distributions and worst windows for:

- CPU, GPU, end-to-end frame time, input-to-present, simulation, and audio;
- peak, transient, resident, committed, and transferred memory;
- IO, decompression, upload, streaming, activation, and eviction;
- shader and pipeline compile, warmup, cache, runtime creation, and first use;
- startup, level/world transition, save/load, build, cook, package, and recovery;
- power, thermal state, battery/energy where supported, and observer overhead;
- storage, artifact, patch, cache, and shipped-package size;
- author intervention, failed attempts, cancellation, and time to first usable
  output for matched workflows.

`PEN-B12` is the mandatory first-use and pipeline-stutter workload. Averages
cannot hide a hitch. Cooked production traversals must declare all required
pipelines and environmental resources; undeclared synchronous creation is a
failure unless the profile explicitly permits and calibrates it.

## 8. Environmental convergence contracts

The owning subsystem specifications, not REL, define runtime behavior:

- Penumbra owns the shared `ParticipatingMediaSourceSnapshot` consumption,
  residency, lighting, shadow, temporal, and compositing contract;
- Isobar owns sparse/multirate weather fields and the coarse-surface side of the
  `SurfaceFluidHandoff`;
- Torsant owns bounded local fire/fluid/thermal solvers, the dynamic-fluid side
  of that handoff, and solver stability;
- Alluvium owns `CombustionMaterialFacet`, `FluidInteractionFacet`, and the
  authored/cooked `RuntimeCostManifest` source semantics.

`PRG-REL-001` validates their convergence but cannot move authority into REL or
create a universal environment solver. One-way snapshot coupling is the
default. Two-way feedback requires a separately preregistered stability,
latency, persistence, downgrade, and workload gate.

## 9. Cook-time prediction and runtime reconciliation

The planned `RuntimeCostManifest` reports predicted geometry, texture, volume,
pipeline, shadow, lighting, vegetation, weather, simulation, upload, streaming,
and residency demand by profile and region. Predictions include uncertainty,
calibration provenance, model/version identity, and unsupported dimensions.

Runtime traces reconcile predicted and observed costs. Prediction error is a
diagnostic and calibration input, never silently rewritten source authority.
The editor explains which authored causes contribute cost and which downgrade
or removal changes would affect quality, gameplay, accessibility, or recovery.

## 10. Workflow, accessibility, and recovery

The future Meridian workspace exposes study definition, parity review, raw
sample inspection, capture comparison, blind-review administration, claim
approval/expiry, and regression triage. CLI/headless surfaces use the same typed
operations. Planned semantic commands include `baseline`, `run`, `compare`,
`review`, `gate`, `renew`, `retract`, and `explain`; exact executable spelling is
not implemented by this specification.

Blind review supports keyboard-only operation, scalable text/images, captions,
reduced motion, contrast controls, color-independent difference marking, and
reviewer opt-out. A quality win that depends on inaccessible presentation or
removes required accessible cues is not a win.

Interrupted studies retain immutable raw samples and completed review rows.
Partial output cannot authorize a claim. Revoked access, corrupted evidence,
expired claims, or missing competitor builds cause an explicit stale,
inconclusive, or retracted state.

## 11. Security, provenance, legal, and publication boundary

Competitor binaries, projects, captures, telemetry, licenses, and benchmark
imports are untrusted. Studies use documented public interfaces and authorized
access. No credentials, personal paths, private source, confidential preview
builds, or private Project Meridian content enter public corpora or reports.

Engine names and trademarks identify measured products only. A source link or
successful import does not grant redistribution rights. Public claim wording
requires release/legal review and the exact evidence-bound BuildId; Marquee may
later consume an approved claim but cannot create or broaden it.

## 12. Risks and stop rules

- `RISK-REL-001`: asymmetric settings or cherry-picked corpora create a false
  win; mitigation is preregistration, parity review, and retained losing rows.
- `RISK-REL-002`: averages hide first-use stalls, tail latency, memory, or
  lower-tier regressions; mitigation is distribution, worst-window, cold/warm,
  and complete profile evidence.
- `RISK-REL-003`: duplicate environmental fields, ownership, or render paths
  erase performance and coherence gains; mitigation is shared-media, single-
  owner handoff, and zero-duplication validation.
- `RISK-REL-004`: cook-time predictions drift from runtime reality; mitigation
  is versioned uncertainty plus prediction-versus-observation reconciliation.
- `RISK-REL-005`: a scoped result is generalized into an unsupported product-
  wide guarantee; mitigation is exact claim scope and release review.
- `RISK-REL-006`: versions or drivers make a claim stale; mitigation is expiry
  and mandatory renewal triggers.
- `RISK-REL-007`: perceptual review becomes biased, statistically weak, or
  inaccessible; mitigation is blinded ordering, reviewer disclosure,
  individual results, accessibility support, and confidence reporting.
- `RISK-REL-008`: high-end optimization regresses portability, recovery,
  accessibility, or maintainability; mitigation is a complete profile matrix
  and a no-hidden-regression gate.
- `RISK-REL-009`: optimization effort exceeds sustainable product value;
  mitigation is a preregistered material threshold, bounded prototypes, and
  losing-prototype archive/cleanup.
- `RISK-REL-010`: licensing, provenance, trademark, or retention constraints
  invalidate a study; mitigation is preflight review and no claim when required
  evidence cannot be retained.

Stop and report `Inconclusive` or `Fail` when parity cannot be established,
measurement is unreliable, required raw evidence is unavailable, a reviewer or
tool conflict is undisclosed, licensing forbids the study, or a win depends on
hiding a material regression.

## 13. Research gate and primary-source anchors

`RG-REL-001` opens only after MS-10 and the program entry gates. It selects the
initial comparator set, study templates, perceptual method, claim-expiry policy,
and automation boundary behind the stable records above. Each renderer,
upscaler, geometry, visibility, GI, simulation, or pipeline technique remains
owned by its subsystem gate.

Primary-source capability anchors reviewed 2026-07-16:

- Epic's [Nanite virtualized geometry documentation](https://dev.epicgames.com/documentation/en-US/unreal-engine/nanite-virtualized-geometry-in-unreal-engine);
- Epic's [Lumen performance guide](https://dev.epicgames.com/documentation/en-US/unreal-engine/lumen-performance-guide-for-unreal-engine);
- Unity's [Spatial-Temporal Post-processing documentation](https://docs.unity3d.com/6000.0/Documentation/Manual/urp/stp/stp-upscaler.html) and [GPU occlusion culling documentation](https://docs.unity3d.com/6000.0/Documentation/Manual/urp/gpu-culling.html);
- Godot's [pipeline compilation documentation](https://docs.godotengine.org/en/stable/tutorials/performance/pipeline_compilations.html).

These sources establish that competitor capability and optimization surfaces
change. They do not prove comparative quality, performance, support, licensing,
or a Meridian implementation decision.

## 14. Deferred execution plan

After the entry gates pass, a future planning review decomposes this program
into bounded packages in this order:

1. freeze `VAL-REL-001`, record schemas, legal/provenance policy, and comparator
   access;
2. calibrate structural, perceptual, temporal, first-use, and workflow methods;
3. establish unmodified Meridian and competitor baselines with no optimization;
4. execute environmental convergence and cook/runtime prediction baselines;
5. profile and rank bottlenecks by user-visible material impact;
6. prototype one bounded optimization at a time behind existing stable seams;
7. rerun all affected profiles, quality layers, recovery, accessibility,
   security, and maintenance rows;
8. adopt winners through owning ADRs, archive losers, and repeat the baseline;
9. approve only scoped, expiring claims whose raw evidence remains available;
10. schedule periodic renewal or retract stale claims.

No item receives file-level estimates, implementation dependencies, or active
package status before that future review.

## 15. Examples

Iso-quality example: a public forest corpus, camera path, internal/output
resolution policy, foliage motion, shadows, fog, and temporal acceptance envelope
are frozen. Blinded review establishes equivalence before frame-time and memory
distributions are compared. A Meridian win applies only to those versions,
hardware profiles, settings, and expiry.

Failure example: Meridian appears faster because the comparator renders dynamic
cloud shadows while Meridian does not. The parity row fails; the result is not
adjusted or published as a win.

Workflow example: the same licensed source asset is imported, processed,
prewarmed, packaged, corrupted at a declared point, and recovered. The study
records elapsed distributions, manual interventions, output fidelity, retained
authority, and first-use behavior instead of timing only the successful build.
