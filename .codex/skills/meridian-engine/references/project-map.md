# Meridian Project Map

This reference is orientation, not status authority. Re-read live files before
making claims or edits.

## Authority map

| Need | Read first |
| --- | --- |
| Suite authority and invariants | `specs/MERIDIAN_MASTER_SPEC.md` |
| Current package and evidence | `PLANNING.md` |
| Milestone order | `specs/DELIVERY_ROADMAP.md` |
| Package readiness/completion/concurrency | `specs/IMPLEMENTATION_PLANNING_SPEC.md` |
| Current implementation maturity | `specs/registry/subsystem-maturity.json` plus code/evidence |
| Requirements and package mapping | `specs/registry/requirements.json`, `work-packages.json` |
| Evidence | `specs/registry/evidence.json` and production records |
| Validation policy | `specs/TESTING_BENCHMARKS_AND_VALIDATION.md` |
| Adopted decisions | `docs/architecture/decisions/README.md` |
| Conflicts/history | `specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md` |
| Open research | `specs/RESEARCH_AND_ALGORITHM_DECISIONS.md` |
| Private creative authority | `game/docs/README.md` and its owning game document |

## Named architecture

- **Meridian**: engine and single integrated creator application.
- **Penumbra**: Meridian-owned GPU-driven renderer. `meridian-rhi` abstracts the
  backend; wgpu is current. Clustered Forward+ is the adopted production path.
- **Alluvium**: procedural world authoring and asset generation. No placeholder
  crate; reserve `meridian-alluvium` for real implementation.
- **Cairn**: physics architecture; current Rapier wrapper is transitional.
- **Wavefront**: audio, music, acoustics, capture, DSP, and device behavior.
- **Collective**: optional provider-neutral online-services system; no hosted
  Meridian cloud is assumed.
- **Isobar**: weather and atmosphere.
- **Basalt**: terrain and large-world geometry.
- **Torsant**: fire, fluids, heat, smoke, and thermal coupling.

Other adopted directions include Rust-first gameplay, optional later Luau,
first-class 2D, ShaderIr plus Meridian Shader Language, navigation
infrastructure, animation, official gameplay frameworks, and a native
beginner-friendly modeler with optional DCC interchange.

## Milestone map

| ID | User-visible gate |
| --- | --- |
| `MS-00` | coherent governance/specification suite |
| `MS-01` | observable runtime and source foundations |
| `MS-02` | Meridian UI core proof |
| `MS-03` | Creator Editor Alpha |
| `MS-04` | Penumbra production Forward+ foundation |
| `MS-05` | representative forest renderer |
| `MS-06` | bounded Project Meridian prototype |
| `MS-07` | production-quality opening playable slice |
| `MS-08` | Engine Alpha and native Metal |
| `MS-09` | Engine Beta and native Vulkan/D3D12 where gated |
| `MS-10` | declared 1.0 profile qualification |

Milestones are evidence gates, not dates. `PRG-*` programs are post-1.0 and
cannot satisfy, block, or promote `MS-*` milestones.

## Stable IDs

- Requirement: `REQ-<DOMAIN>-NNN`
- Work package: `WP-<DOMAIN>-NNN`
- Research gate: `RG-<DOMAIN>-NNN`
- Evidence: `EV-<DOMAIN>-YYYYMMDD-NNN`
- Milestone: `MS-00` through `MS-10`
- Post-1.0 program: `PRG-<DOMAIN>-NNN`
- Validation project: `VAL-<DOMAIN>-NNN`
- Dependency strategy: `DEP-<DOMAIN>-NNN`
- Penumbra workload: `PEN-B01` through `PEN-B16`
- Waiver: `WVR-<DOMAIN>-NNN`
- Decision: `ADR-NNNN`

Use only domains registered by the current master specification.

## Maturity vocabulary

- Documentation: `Draft`, `ArchitectureComplete`, `ResearchReady`,
  `ImplementationReady`, `VerifiedCurrent`.
- Implementation: `Implemented`, `ImplementedFoundation`, `StructuralSmoke`,
  `Partial`, `Transitional`, `Scaffold`, `Planned`, `Research`, `Deferred`,
  `Unsupported`.
- Evidence: `Pass`, `Fail`, `NotRun`, `UnsupportedCapability`,
  `UnsupportedPlatform`, `Occluded`, `Redacted`, `Waived`, `Stale`,
  `Inconclusive`.

## Workspace map

`Cargo.toml` includes `engine/*` and `editor/*`; it excludes `game/*`.

Runtime/foundation crates currently include core, runtime, platform, input,
tasks, diagnostics, ECS, assets, package, world, streaming, save, physics, RHI,
render graph, renderer, UI, audio, Isobar, Basalt, and vegetation. Editor/tool
crates include the Meridian executable, asset/world/shader tools, benchmark
helpers, and `meridian-spec` governance tooling.

Do not infer implementation from a crate name. Query Cargo metadata and inspect
the maturity registry, code, and evidence. Torsant, Alluvium, Collective, and
many later systems intentionally may have no crate yet.

## Current foundation boundary

The 2026-07-15 baseline had meaningful runtime/RHI/render-graph, direct PBR,
cascaded-shadow, diffuse-irradiance IBL, typed pass timing, asynchronous capture,
transactional source import, provisional package/world streaming, save recovery,
physics-wrapper, and executable smoke foundations. It did not prove production
Forward+, Meridian UI/Editor, native modeler, Alluvium, Wavefront runtime,
Collective, production simulations, game prototype, native backends, or release
quality. This snapshot will age; verify every claim live.

## Known implementation traps

- Apple M4/macOS Metal timestamp data has produced zero and reversed values in
  the current wgpu path. Meridian must reject invalid samples, report typed
  `UnsupportedPlatform`/`Inconclusive` outcomes, disable unreliable GPU timing
  for that RHI lifetime, and retain CPU encode timing. Never report raw zero or
  reversed timestamps as measured GPU duration.
- Timing and capture readback use bounded asynchronous slot rings. Do not add
  `wait_indefinitely`, reuse a slot before callback completion, or turn ring
  saturation into a blocking path.
- Source identity must remain stable across LF and CRLF checkouts. Preserve the
  import boundary's line-ending normalization when touching fixture hashing.
- A successful offscreen capture proves the draw path and pixels, not surface
  presentation. Preserve `Presented`, `Occluded`, `Unsupported*`, and other
  typed outcomes separately.
- The `meridian-editor` Cargo package owns the executable named `meridian`; the
  product/window name remains **Meridian**.
- MS-01 JSON source, compiled-cell, `.meridian` package, and save encodings are
  provisional versioned foundations. Do not claim final format stability or add
  compression/signing/encryption without the owning package.
- Generated smoke/evidence files belong under `target/meridian-evidence/` (or a
  requested output path). They are not source authority and must not be tracked.

## Repository boundary

The engine is dual-licensed MIT OR Apache-2.0. The private game repository is
proprietary. The engine may hold sanitized proving requirements and surrogate
benchmarks, but never private game content. A local `game/` directory is both a
separate Git repository and ignored by the engine.
