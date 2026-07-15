# Meridian Shader Language and Shader IR Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [ADR-0023](../docs/architecture/decisions/ADR-0023-meridian-shader-language.md) · [Penumbra](RENDERING_AND_GRAPHICS_SPEC.md) · [Assets](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by ADR-0023. Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-SHD-001` through `REQ-SHD-003`; `WP-SHD-001`; `WP-SHD-002`; `PRG-SHD-001`.

Current implementation status: Meridian has WGSL shaders and current Naga/wgpu foundations, but no Meridian Shader Language frontend, canonical ShaderIr, material-graph lowering, source mapper, compatibility declaration compiler, or native-backend lowering.

The full working name is used in normative prose. The abbreviation `MSL` is prohibited because it conflicts with established shader-language terminology.

## 1. Authority, Goals, and Non-Goals

The `SHD` domain owns high-level shader source semantics, the canonical typed `ShaderIr`, material-graph lowering into that IR, validation, reflection, specialization declarations, compatibility metadata, source maps, and backend-neutral diagnostics. Penumbra owns renderer paths, material runtime semantics, binding policy, pipeline construction, caches, and presentation. RHI/backend implementations own target-language and binary compilation boundaries.

Goals:

- allow text and graph authors to target one semantic IR without duplicated materials;
- provide stable types, units/spaces, stages, resource declarations, feature/capability checks, and deterministic diagnostics;
- lower to WGSL during the wgpu era and later to native backend inputs through explicit target modules;
- generate reflection/bindings/source maps from one authority;
- sandbox custom shaders and make compatibility/fallback behavior inspectable.

Non-goals are replacing mature compiler infrastructure for branding, embedding renderer internals in source schemas, promising source or binary compatibility with another shading language, unrestricted code execution, or claiming a future native backend before its research gate.

## 2. Planned Contracts

```text
ShaderModuleSource { id, language_version, imports, declarations, entry_points, provenance }
MaterialGraphSource { id, graph_version, nodes, ports, parameters, outputs, provenance }
ShaderIrModule { id, ir_version, types, constants, resources, functions, stages, source_map }
ShaderCapabilitySet { portable, indirect, subgroup, mesh, ray, sparse, vr, limits }
ShaderCompatibility { renderer_paths, required_capabilities, fallback, trust, budgets }
ShaderReflection { entries, bindings, layouts, specialization, vertex_io, diagnostics }
ShaderTargetRequest { backend, capability_profile, renderer_path, optimization, debug }
PipelineCacheManifest { source_hash, ir_hash, target, capabilities, compiler, artifacts }
```

Stable IDs identify declarations, graph nodes/ports, source spans, resources, entry points, and specializations. Text and graph sources may round-trip only where semantics permit; both always lower to the same `ShaderIr`, not to each other as the canonical representation.

## 3. Language and IR Semantics

The first language subset favors explicitness: scalar/vector/matrix/value types, structures, functions, bounded control flow, stage entry points, typed resources, immutable-by-default values, explicit conversions, semantic spaces/units where required, and portable capability checks. Undefined behavior, ambient globals, unbounded recursion, unrestricted pointers, host filesystem/network access, and backend-specific handles are forbidden.

IR validation proves type correctness, control-flow validity, resource bounds, stage legality, interface compatibility, derivative/uniformity constraints, capability requirements, and declared budget class. Optimization cannot erase source mapping or change observable numeric policy without recorded mode and differential evidence.

## 4. Ordered Toolchain

```text
load source/graph and provenance
-> resolve versioned imports without ambient filesystem search
-> parse or graph-validate with stable source spans
-> type, stage, space, unit, capability, and safety checking
-> lower to canonical ShaderIr
-> validate and normalize IR
-> specialize for material, renderer path, and capability profile
-> lower to WGSL or approved native target input
-> invoke approved compiler boundary
-> reflect bindings/layouts and compare generated contracts
-> cache by complete manifest
-> atomically publish artifact plus diagnostics/source maps
```

Development compilation may be asynchronous. Runtime pipeline requests never block indefinitely; missing pipelines use declared fallback, bounded wait, or typed failure. Shipping cooks reject unresolved imports, unsafe trust, missing fallbacks, incompatible paths, or unreviewed compiler provenance.

## 5. Ownership and Forbidden Edges

- Material source owns artistic parameters and semantic outputs; renderer-path lowering owns path-specific implementation.
- RHI owns capability records and resource abstractions; source cannot inspect named GPU models.
- Build owns target/profile/cook transactions; editor is not the only compiler entry point.
- Security owns trust/capability policy; custom source cannot self-authorize.

Forbidden edges include handwritten binding layouts diverging from reflection without a waiver, backend-language source as the only high-level material authority, graph-only hidden semantics, renderer-private structs in public source, and silently compiling unsupported features to incorrect output.

## 6. Threads, Memory, Failure, and Diagnostics

Parsing, validation, optimization, target lowering, and driver compilation use bounded workers or isolated processes according to trust. Cache entries are content-addressed by source, IR, compiler, capability, renderer-path, target, settings, and dependency hashes. Failed candidates never replace the last accepted artifact.

Diagnostics include stable code, source/graph span, IR operation, target/backend, renderer path, capability/limit, expansion/import stack, generated target span, compiler identity, cache state, compile duration, peak memory, and suggested repair. Pipeline-stall reports distinguish source compile, IR lowering, target compile, driver compile, cache miss, and pipeline creation.

Security requires bounded compilation, import allowlists, no arbitrary host access, validation before backend submission, crash containment for untrusted compilers, artifact signing/provenance in release profiles, and private-source redaction.

## 7. Accessibility, Tiers, and Dependency Strategy

Text editing provides semantic diagnostics, completion, navigation, keyboard-first workflows, high-contrast source spans, and screen-reader-readable errors. Material graphs provide keyboard and textual alternatives, deterministic layout-independent identity, explicit types, and parity diagnostics.

Tiers follow Penumbra/RHI capabilities: portable core; GPU-driven advanced; subgroup/mesh/ray/sparse; VR. Unsupported required capability produces a typed error or authored fallback. Unused language, graph, compiler, and renderer-path modules are absent from cooked runtime output.

Naga and other mature permissive foundations may remain behind Meridian contracts indefinitely. Replacement or internalization requires evidence of product value, compatibility, security, performance, and sustainable maintenance under the dependency-strategy registry and `PRG-SHD-001`.

## 8. Requirements, Work Packages, and Evidence

- `REQ-SHD-001`: one versioned typed ShaderIr for text and graph material/shader sources with reflection, source-map, and migration evidence.
- `REQ-SHD-002`: capability-safe, sandboxed, reproducible target lowering and pipeline manifests with differential backend evidence.
- `REQ-SHD-003`: renderer-path compatibility, fallback, specialization, diagnostics, and zero-cost omission with representative material evidence.
- `WP-SHD-001`: textual frontend, graph lowering seam, semantic analysis, canonical IR, validation, and reflection.
- `WP-SHD-002`: WGSL/native target modules, source mapping, pipeline/cache integration, security, and debugging.
- `PRG-SHD-001`: optional measured compiler-infrastructure internalization after 1.0.

Tests cover parser/type/IR fixtures, malformed/untrusted fuzzing, source-map accuracy, graph/text semantic equivalence, reflection/binding generation, capability rejection/fallback, target differential rendering, cache identity, compiler crashes, pipeline-stall reporting, reproducible cooks, and private-source redaction.

## 9. Examples

End to end: an artist graph and an expert text module both lower into one material IR; Penumbra specializes it for Forward+; the wgpu-era target emits WGSL and generated bindings.

Failure: a custom shader requires subgroup operations on a portable profile without fallback. Cooking rejects it with the exact declaration and affected renderer paths.

Performance debug: a hitch report traces a material edit through IR cache miss, WGSL generation, driver compilation, and pipeline creation instead of reporting an undifferentiated shader stall.
