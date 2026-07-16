# Specification-Suite Agent Guidance

version 0.5 · 2026-07-16

The root [AGENTS.md](../AGENTS.md) is canonical. These rules apply inside `specs/`.

## 1. Ownership and truth

- Preserve the authority order in [MERIDIAN_MASTER_SPEC.md](MERIDIAN_MASTER_SPEC.md).
- Use [IMPLEMENTATION_PLANNING_SPEC.md](IMPLEMENTATION_PLANNING_SPEC.md) for package readiness, completion, planning horizons, concurrency, and replanning.
- Put each normative contract in one owning spec and link to it.
- Keep documentation, implementation, and evidence maturity separate.
- Do not upgrade a status without current evidence registered under `specs/registry/`.
- Keep creative material in the private Project Meridian repository; link only sanitized engine requirements and source hashes.
- Keep Alluvium normative behavior in `PROCEDURAL_AUTHORING_SPEC.md`; owning
  runtime specs describe only their typed consumer boundary and authority.
- Keep Marquee normative behavior in `MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md`; linked specs retain capture, data, animation, audio, build, agent, security, release, and private-project authority.
- Keep competitive comparison and claim governance in `COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md`; Penumbra, Isobar, Torsant, and Alluvium retain renderer, simulation, and authoring authority.
- Historical roadmap identifiers are legal only in migration/history records. Active specs use `MS-*`, `WP-*`, `RG-*`, post-1.0 `PRG-*`, validation `VAL-*`, and strategic dependency `DEP-*` IDs.
- Domain codes are governance identifiers and do not require matching runtime crate/type names.

## 2. Required subsystem shape

Each coordinated subsystem document identifies:

1. authority, version, documentation maturity, implementation maturity, and governing IDs;
2. goals and non-goals;
3. ownership, dependencies, consumers, and forbidden edges;
4. public contracts and data authority;
5. ordered pipelines or state machines;
6. clocks, threading, memory, lifetime, persistence, and compatibility;
7. failure, recovery, diagnostics, security, and provenance;
8. accessibility and beginner/expert/editor/CLI/agent workflows;
9. capability tiers and zero-cost-disabled behavior;
10. tests, benchmarks, evidence classes, research gates, delivery mapping, and end-to-end/failure/performance-debug examples.

When an item is intentionally inapplicable, state why. Planned contract examples remain illustrative and MUST NOT imply runtime types exist.

## 3. Architecture changes

A changed decision updates requirements, the owning spec, ADR, registries, migration/contradiction register, research gate when applicable, API examples, validation, delivery mapping, and PLANNING if the active queue changes.

Use primary sources for algorithms/platform APIs and record verification date. A source supports context, not licensing, redistribution, production readiness, or a performance claim.

## 4. Audit

Before sign-off:

- `meridian-spec check` passes;
- every coordinated domain has exactly one maturity record;
- every requirement maps to a pre-1.0 work package or post-1.0 program and evidence class;
- links/fences/status vocabulary/ADR references validate;
- old active roadmap and deleted combined-weather references are absent;
- Telepo, permanent bootstrap-UI/Rapier ownership, multiple initial languages, opaque one-file worlds/packages, mandatory cloud, and all-engine-first gating are not revived;
- provisional numbers remain labeled and have a calibration plan;
- examples are marked illustrative unless compiled;
- root AGENTS, PLANNING, roadmap, registries, and subsystem specs agree.
- Alluvium recipes, fields, outputs, runtime authority, private boundary,
  report fields, packages, gates, and risks agree without implementation promotion.
- every milestone delivery-plan record has entry conditions, critical path, parallel lanes, integration checkpoint, exit evidence, and stop conditions;
- active and next packages meet the detail required for their planning horizon without inventing distant task precision.
- v0.5 one-app, Rust-first/Luau-after, native-modeler, Wavefront/Collective, first-class 2D, animation/navigation/framework, ShaderIr, post-1.0 program, validation-project, and dependency-strategy decisions agree across authorities.
- Marquee remains post-1.0, manual-capture, export-only, explicit-human-approved, and AI text/analysis-only; no `WP-PRM-*`, service publishing, website generation, or audiovisual AI appears before a future planning review.
- `PRG-REL-001` remains post-1.0 and cannot become a global superiority guarantee; every future claim is workload-, profile-, version-, evidence-, expiry-, and retraction-bound.
- environmental media uses one Penumbra consumption path, dynamic surface water has one owner per region/epoch, and sparse/multirate simulation does not move live authority into Alluvium or REL.
