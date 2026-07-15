# ADR-0023: Meridian Shader Language and Shared Shader IR

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.5
- Implementation status: Planned; WGSL foundation remains transitional
- Owners: shader language, Penumbra, RHI, assets, IDE
- Amends: ADR-0007
- Supersedes: none

## Context

Meridian intentionally wants a first-party textual shader language with owned semantics, diagnostics, reflection, and engine integration. Current WGSL is useful backend-era source evidence, while material graphs and planned `ShaderIr` already establish a backend-neutral direction.

## Decision

The full working name is "Meridian Shader Language" until a distinct reusable subsystem name is selected. Documentation and code must not abbreviate it as `MSL`, which conventionally means Apple's Metal Shading Language.

Textual shaders and material graphs are separate frontends lowering to one Meridian-owned typed `ShaderIr`. The language owns parsing, type checking, resource/address-space semantics, interfaces, functions, abstraction/generics policy, capability checking, specialization, diagnostics, source maps, reflection, static analysis, and render-graph integration.

Initial lowering may use Naga, WGSL, SPIR-V tools, platform compilers, and other mature infrastructure. Backend replacement is not an entry gate. Native escape hatches are capability-scoped, explicit, non-portable, and require fallback policy.

## Consequences

- Artists do not duplicate materials per renderer path or backend.
- Backend binaries and caches remain derived data.
- CPU reference execution is required only for deterministic/testable subsets where it has product value.
- Shader language usability precedes backend compiler internalization.
