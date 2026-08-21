# ADR-0019: Rust-First Gameplay, Optional Luau Afterward

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned
- Owners: gameplay, editor, runtime, build
- Amends: ADR-0012
- Supersedes: none

## Context

ADR-0012 selected Luau as Meridian's first embedded high-level gameplay language. It did not determine whether native Rust or the embedded language must be implemented first. The Project Meridian prototype needs one honest, testable gameplay path without making a partially designed VM a prerequisite.

## Decision

Meridian implements native Rust gameplay modules first. Rust receives the stable gameplay API, module lifecycle, reflection/editor properties, events and commands, save/network/headless hooks, tests, diagnostics, and Play-session reload fallback.

Luau follows after those contracts are stable. Luau remains Meridian's first embedded high-level language and binds the same schema-generated APIs. Projects may use Rust only, Luau only, data/logic documents only, or a declared combination. Luau is optional and has zero cost when absent.

Native Rust hot reload is best-effort. When ABI, platform, or state safety prevents reliable reload, Meridian rebuilds and restarts an isolated Play session while preserving an explicit checkpoint where compatible. Luau may later provide faster state-migrating reload behind the same lifecycle contract.

## Consequences

- `WP-GAM-001` becomes the Rust gameplay foundation used by the prototype.
- Luau moves to `WP-GAM-002` and cannot block `MS-06` or `MS-07`.
- Gameplay semantics cannot diverge between Rust, Luau, and typed logic documents.
- Compilation success alone does not prove reload, migration, or editor safety.
