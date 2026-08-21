# WP-V1-GOV-001 — v1 governance schemas and validator

## Ownership

- Owning phase: `PH-AUTH-003` — Build the v1 governance schemas and validator
- Depends on: `PH-AUTH-001`, `PH-AUTH-002` — both closed
- Branch: `v1-authority-reset` at `14d3feb`
- Primary semantic seam: **what the v1 validator considers authority, and what it refuses.**

## User-visible / operational result

`cargo run -p meridian-spec -- validate-v1` checks the root specoment and its generated
projections against typed schemas and machine-checkable rules, and fails with a named
identifier and line when any rule is violated — before v0.5 authority is removed.

## Current source diagnosis

Measured, not assumed. Full detail in `DIAGNOSIS-CARRIED-FORWARD.md` in this directory.

- `main.rs` is **2,212** lines and hard-codes a 37-entry `DOMAINS` list and a 46-entry
  `VALID_STATUSES` list, both v0.5 vocabulary. (An earlier draft of this plan said 2,188,
  which was the figure before `WP-V1-RESET-002` added its dispatch arm. Caught by
  re-measuring rather than restating — the same class of error this programme has hit in
  every prior round.)
- The v1 specoment uses **122 identifier family prefixes** counting every backticked
  identifier, or **117** by the generator's rule which filters to families with a declared or
  referenced member. Under either rule only **10** appear in `DOMAINS` and **27** v0.5 domains
  have no v1 counterpart. The vocabulary must be **derived**, not patched.
- §0.3 defines exactly **6** decision maturity labels; headings use **29** distinct suffix
  phrases, of which **24** are not in §0.3.
- §0.4 defines three independent axes with closed enums, separate from §0.3's labels.
- `governance/generated/` holds six projections at 736 declared / 0 undeclared /
  0 multiply-declared / 2 retired-v0.5 / 117 families, and `phases.json` carries 99 cards and
  20 recorded Appendix G divergences.
- `schemas/governance/` holds 23 v0.5 schemas; `specs/registry/` holds 22 v0.5 registries.
  Both are retired at `PH-AUTH-004`, not here.

## Approach

1. **Derive the vocabulary.** Families, maturity labels and phase identifiers come from the
   specoment via the existing `specoment` modules. No hard-coded v1 list.
2. **Schemas** at `governance/schemas/*.schema.json` for each generated projection, validated
   with the `jsonschema` crate the tool already depends on.
3. **Rules**, each with a test that fails before it exists:
   - zero unmapped identifiers;
   - phase DAG resolves and is acyclic;
   - one owning heading per identifier;
   - maturity labels map to a §0.3 label through a declared table, and an **unmapped label is
     an error, never a default**;
   - §0.4 axis values come from their closed enums;
   - root-to-projection equivalence: every projection regenerates byte-identically;
   - projection stamps carry the four Appendix H.5 fields and the current canonical digest;
   - no forbidden legacy authority reference in v1 content.
4. **Completion-record rule**, closing the control gap `DEV-007` recorded: a plan whose
   completion record exists must have every deliverable its Approach names. Nothing checked
   that, which is how `WP-V1-RESET-002` was recorded complete while two projections were
   missing.

## Explicit exclusions

- No edits to `specs/`, `schemas/governance/`, `AGENTS.md`, `PLANNING.md`, `DCO.md`, CI.
  All `PH-AUTH-004`.
- No specoment body edits. Divergences are recorded.
- No runtime feature work. No speculative packages for distant phases.
- No new dependency; `jsonschema` and `serde_json` are already present.

## Compatibility / migration / authority effects

The v0.5 `check` command keeps working unchanged until `PH-AUTH-004`. The v1 validator is a
separate subcommand over separate paths, so both suites stay separable by path through the
mixed window.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime, UI or dependency surface. Offline, reads and writes inside the repository.
Provenance improves: rules that were prose become executable.

## Tests and evidence

Every rule gets a fixture that violates it and a test asserting the violation is reported by
identifier and line. Failure classes from the phase card: malformed IDs, duplicate authority,
cycles, stale evidence, invalid status promotion, legacy-ID leakage, canonical-equivalence
gaps. Plus the `DEV-007` class: a completion record naming fewer deliverables than its plan.

No test asserts a count. Where a corpus figure matters the assertion is on the **set** or on
an invariant, following the four-round finding that every restated constant in this programme
has been wrong.

Gates: `cargo test --workspace`, `fmt`, `clippy --workspace -D warnings`, `spec check`,
`project --check`, `metadata --locked`, `git diff --check`.

## Failure injection and recovery

A malformed schema, a cyclic phase graph, an unmapped maturity label, a hand-edited
projection, a v1 file citing a retired v0.5 identifier as live authority, and a plan whose
completion record omits a deliverable — each must fail loudly and name the offender.

## Research candidates and selection metrics

None.

## LOC estimate

Production ~600, tests and fixtures ~500, schemas ~400 generated. Scope signal.
If hand-written rule code materially exceeds this, the plan is revised and re-reviewed.

## Stop / rollback rule

Stop if the validator needs prose duplication to understand authority; if the machine
registries become a second normative specification; if a rule cannot fail; or if any rule
requires editing the specoment to pass. Rollback is deleting the branch commits.

## Independent Review

_Pending._

## Completion record

_Pending._
