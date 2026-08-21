# ADR-0012: Luau as Initial High-level Runtime

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned with runtime/ECS precursors
- Owners: meridian-gameplay-schema, meridian-gameplay-runtime, meridian-luau
- Supersedes: none
- Superseded by: none

## Context

Meridian needs a high-level gameplay runtime for designers, interaction/state
flows, hot reload, debugging, save-safe state, and generated bindings. Multiple
initial runtimes would multiply API, sandbox, docs, and compatibility work.

## Decision

Luau is the single initial high-level runtime. Rust remains available for native
engine/game modules. C#, Anorak, Python, and mixed-language architecture are
later research. Luau binds the same schema as Rust and runs inside explicit
isolation domains with deny-by-default capabilities.

Scripts do not receive ambient filesystem, process, native library, network,
clipboard, debug, or agent access.

## Current Evidence

- [Gameplay, narrative, and scripting spec](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Runtime APIs need generated Rust and Luau bindings, stable IDs, hot-reload
migration rules, sandbox escape tests, budget diagnostics, save fixtures, and
opening completion tests. Additional language support must not destabilize the
initial runtime.

## Status Review

Review when MS-04/MS-06 runtime, sandbox, and binding evidence exists.
