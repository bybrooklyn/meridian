# WP-V1-CENSUS-001 — Crate and API disposition inventory

## Ownership

- Owning phase: `PH-AUTH-005` — Requalify and classify the existing implementation
- Declared at `MERIDIAN_SPECOMENT.md` under `# Near-term work-package decomposition`
- Requirements: `SPEC-001`, `IMPL-WP-003`, Appendix D projection rules, §0.4 maturity axes
- Depends on: `PH-AUTH-004` — closed, merged at `e9a7ff6`
- Branch: `main`
- Primary semantic seam: **what every existing crate, API and test is, under v1 authority.**

## User-visible / operational result

Every workspace member, public API surface, test, example, source format and CI row carries
one honest v1 disposition and a next phase, replacing maturity inherited from retired v0.5
milestones.

## Current source diagnosis

Measured at `49bdd74`, each figure with its producing command.

- **37 workspace members** (`cargo metadata --no-deps`), across `engine/` and `editor/`.
- **716 test functions** (`grep -rc '#\[test\]' engine editor`).
- **96 internal dependency edges**: 84 mandatory, 12 optional, 0 dev-only.
- **18 workspace third-party dependencies**; 494 locked packages.
- **Four marker crates** at ~205 bytes and one public item each: `meridian-audio`,
  `meridian-basalt`, `meridian-isobar`, `meridian-vegetation`. `meridian-ui` is 1,174 bytes
  with zero public items — a façade re-export, not a marker.
- **Seven files over 240 KB**, matching the `PH-AUTH-006` card's stated 248-540 KB range
  exactly: `ui_runtime/lib.rs` 540 KB down to `rhi/lib.rs` 247 KB.
- **Two genuine mandatory reverse edges**: `meridian-ecs → meridian-renderer` and
  `meridian-rt → meridian-renderer`. A first pass wrongly flagged twelve more; those are all
  optional and feature-gated, and the CI guard `cargo tree -p meridian-rt | grep meridian-ui`
  genuinely holds.

**Root cause of the two real edges, stated because the census must not just list them:** the
snapshot *contract* — `RenderSnapshot`, `RenderInstanceId`, `Transform`, `MeshHandle`,
`MaterialHandle`, `SnapshotError` — is defined in `meridian-renderer`. Any crate that
*produces* a snapshot must therefore depend on the renderer that *consumes* it. The inversion
is the contract's location, not the edge. Moving the edge without moving the contract would
relocate the inversion rather than remove it.

## Approach

1. Generate `governance/generated/census.json` from `cargo metadata` plus source measurement:
   per crate — location, targets, features, mandatory/optional edges, public-item count,
   source size, test count, disposition, next phase.
2. Assign one disposition per crate from the specoment's own vocabulary:
   `retain` / `refactor` / `replace` / `merge` / `split` / `remove`.
3. **All implementation maturity is `ExistingUnqualified`.** The phase card is explicit:
   *"Treat all current code as ExistingUnqualified after reset … do not call foundations
   stable merely because old milestones passed."* No crate inherits a v0.5 status.
4. Generate a dependency graph as checked-in data, with mandatory and optional edges
   distinguished — the distinction a first pass got wrong.
5. Map retained tests to a next phase, so no test is retained merely because it exists.
6. Record forbidden edges and their root cause, not just their existence.

## Explicit exclusions

- No feature work. No mass renames. No decomposition — `PH-AUTH-006` owns that.
- No promotion of any crate's maturity. The census records state; it does not improve it.
- No removal of the marker crates; that is `PH-AUTH-006`'s `WP-V1-ARCH-001`.

## Compatibility / migration / authority effects

None to runtime. The census is a generated projection under `governance/`, policed by
`project --check` like its siblings, and carries the four Appendix H.5 stamp fields.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime, UI or dependency surface. Provenance improves: crate disposition stops being
implicit in retired milestone records and becomes a stamped, regenerable artefact.

## Tests and evidence

- Every workspace member appears exactly once — count asserted against `cargo metadata`, so
  a crate added later fails the check rather than being silently absent.
- Every crate has a disposition drawn from the closed vocabulary; an unmapped crate is an error.
- Every crate has a next phase.
- Mandatory and optional edges are distinguished; a test asserts the twelve optional edges are
  not reported as violations, guarding the mistake the first pass made.
- The two genuine reverse edges are reported with their root cause.
- No crate carries an implementation maturity other than `ExistingUnqualified`.

Gates: `cargo test --workspace`, `fmt`, `clippy --workspace -D warnings`, `check`,
`project --check`, `metadata --locked`, `git diff --check`.

## Failure injection and recovery

Add a synthetic crate with no disposition and confirm the check fails naming it. Mark a crate
`Implemented` and confirm the maturity rule rejects it. Present an optional edge as mandatory
and confirm the distinction is asserted rather than assumed.

## Research candidates and selection metrics

None.

## LOC estimate

| Area | Added |
|---|---|
| Census generation in `emit.rs` plus a `census` module | ~300 |
| Tests | ~150 |
| Generated `census.json` | ~900 |

## Stop / rollback rule

Stop if a crate is retained solely because it exists; if a test has no behavioural contract or
next phase; if any implementation maturity is promoted without new v1 evidence; or if the
census asserts a disposition the specoment's vocabulary does not contain. Rollback is one
commit; nothing outside `governance/` changes.

## Independent Review

_Pending._

## Completion record

_Pending._
