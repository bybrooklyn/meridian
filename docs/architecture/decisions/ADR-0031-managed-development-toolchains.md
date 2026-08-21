# ADR-0031: Managed Development Toolchains

- Status: Adopted
- Date: 2026-07-18
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned
- Owners: BLD, SEC, EDT, platform, shader, and IDE workstreams
- Supersedes: none
- Superseded by: none

## Context

Rust compilation, semantic editing, platform builds, debugging, and external
shader compilation need versioned tools outside the Meridian application. An
ambient host installation makes a project result hard to reproduce and permits
an update to change behavior without a source-visible project decision.
Bundling compilers, SDKs, and debuggers into the main binary would make the
application unnecessarily large, obscure licensing/provenance, and prevent
independent platform-specific lifecycle management.

## Decision

Adopt managed external development-toolchain components. `rustc`, Cargo,
rust-analyzer, platform SDKs, debuggers or debugger adapters, and external
shader compilers are separately installed, versioned, hash-verified,
license-tracked components managed through Meridian's CLI and editor. Projects
pin exact compatible component records. The manager supports side-by-side
versions and explicit install, verification, update, repair, rollback, and
cleanup operations.

Updates stage and verify complete component generations before atomic
activation, retain a verified rollback generation, and never silently alter a
project pin. Repair restores the pinned bytes rather than selecting a newer
version. Cleanup retains components that remain project-pinned, active,
rollback-eligible, or required by evidence/provenance.

The main Meridian binary does not bundle those tools. `WP-BLD-001` remains a
host-selected local Cargo foundation only; `WP-BLD-002` owns implementation of
this managed-toolchain architecture.

## Current Evidence

The current bounded Cargo service records local toolchain identity as a build
input and does not install, verify, repair, roll back, pin, or clean up managed
components. This ADR and its linked specification are planned architecture and
do not promote BLD implementation maturity.

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `governance/generated/`
- `governance/generated/`

## Consequences

- Meridian owns component, lock, compatibility, recovery, and user-facing
  diagnostic contracts; upstream compiler, SDK, debugger, and language-server
  types remain behind adapters.
- Project reproduction records exact verified component identities and license
  provenance without introducing machine-specific absolute paths into shared
  profiles.
- Toolchain recovery cannot bypass trust, compatibility, license, or explicit
  project-pin transitions. Missing tools produce actionable blocked states.
- `WP-BLD-002` requires component-matrix, atomic-update, corruption/recovery,
  pinned-project non-mutation, side-by-side, license, and cleanup-retention
  evidence before this policy becomes an implementation claim.

## Status Review

Review when `WP-BLD-002` meets its Definition of Ready, when an adopted
platform/update decision changes component trust policy, or when implementation
evidence shows that a component class needs a separate delivery boundary.
