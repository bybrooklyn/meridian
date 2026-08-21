# ADR-0023: Meridian Shader Language and Shared Shader IR

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
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
