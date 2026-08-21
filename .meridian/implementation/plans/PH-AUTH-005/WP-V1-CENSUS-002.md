# WP-V1-CENSUS-002 — Complete the census measurement

## Ownership

- Owning phase: `PH-AUTH-005`
- Requirement ids: `SPEC-001`, `IMPL-WP-003`, `PH-AUTH-005` implementation scope (specoment
  line 30174), Appendix D, §0.4
- Depends on: `WP-V1-CENSUS-001`, landed `6b1f569` + repair `440bf3e`
- Successor: `WP-V1-CENSUS-003` assigns dispositions and ownership
- Declaration status: **not a specoment-declared id.** The specoment declares
  `WP-V1-CENSUS-001` only (lines 28629, 30170). The 001/002/003 split is a plan-level
  decomposition recorded in `state.json`, not new authority. Flagged rather than left implicit.
- Branch: `main`
- Primary semantic seam: **the phase card's ten inventory axes, all of them.**

## User-visible / operational result

The census covers every axis `PH-AUTH-005` names, so the phase becomes closeable on evidence
rather than on five-tenths of its own scope. Still measurement only: every `disposition`,
`escalation`, `owner` and `next_phase` is null when this package lands.

## Current source diagnosis

`.meridian/implementation/census.json` at `440bf3e` measures crates (37), source formats (15),
tests (779), generated files (9), CI rows (3), plus 96 edges over 8 layers. Measurements are
real and byte-identity is proven across roots.

Five of the card's ten axes have no section at all: **public type, backend dependency, feature,
example, evidence runner.** The card's declared test — "every workspace member, **public API**,
source format and test has one owner and disposition" — is unsatisfiable against a census where
920 public items exist only as a per-crate scalar with no rows.

Two fields `WP-V1-CENSUS-001`'s accepted plan promised are absent: `reexported_public_items`
(argued for explicitly, then not shipped — `meridian-ui` reports 5 while re-exporting ~213) and
test `module` (rows carry `file`/`line`/`function`, so "module granularity" silently means file
granularity, and one file holds 122 tests).

`owner` means three different things across sections: owning crate on formats, and is intended
to mean next phase on crates and requirement id on tests. Populating it would overwrite
`meridian-package`.

## Approach

1. **Add the five missing sections.** Public types (rows, not a scalar, with the
   declared/re-exported distinction the predecessor promised); backend dependencies (18
   workspace third-party, 494 locked, each with licence field left null for `OD-006`);
   features (7 across 4 crates); examples (14 targets); evidence runners.
2. **Deliver the two promised fields.** `reexported_public_items` on crate rows; `module` on
   test rows, so `CENSUS-003` can honestly claim module granularity.
3. **Disambiguate `owner` before it is populated.** Rename the format field to `owning_crate`.
   Add `next_phase` as a field distinct from `owner` — the card requires "one disposition **and
   next phase**", which is two fields, not one.
4. **Add `disposition` to test rows.** The card says each *retained* test gets an owner, which
   presupposes tests can be dropped. Without it, tests of code slated for `remove` get owners.
5. **Write `governance/schemas/census.schema.json`** — `WP-V1-CENSUS-001` carry-in 1, unmet.
   It enforces `disposition` XOR `escalation` on the sections that carry them, and constrains
   `disposition` to the closed vocabulary. Sections without judgement fields (`edges`,
   `layers`) are declared exempt in the schema rather than silently omitted, because
   `CENSUS-001`'s "every row" claim was false for 104 rows.
6. **Assert non-uniformity.** No measured scalar may be uniformly zero across all rows of a
   section, and every `location` must match its manifest prefix. This is the assertion class
   whose absence let a fully zeroed census pass every test in the predecessor.

## Explicit exclusions

- **No dispositions, no owners, no escalations.** That is `WP-V1-CENSUS-003`. A non-null
  judgement field in this package's output means judgement leaked in, exactly as in `001`.
- No decomposition or crate removal — `PH-AUTH-006`.
- No specoment edits.

## Compatibility / migration / authority effects

None to runtime. Class (c) throughout, outside `emit::all()`, policed by byte-identical
regeneration. Renaming `owner` → `owning_crate` changes only an uncommitted-consumer field.

## Accessibility / security / privacy / provenance / disabled-cost effects

Provenance improves materially: the dependency section is the first machine-readable list of
the 494 locked packages whose licence provenance `OD-006` flags as unmet under `LEGAL-005`. It
records the inventory and leaves licence null; it does not resolve `OD-006`.

## Tests and evidence

- All ten card axes have a section; a test enumerates the axes from the card text and fails if
  one has no section.
- `reexported_public_items` is non-zero for `meridian-ui`, and `declared` + `reexported`
  reproduce the hand-checked 213.
- Every test row's `module` is non-empty and consistent with its `file`.
- Schema validates the census; both-set, neither-set, out-of-vocabulary and non-existent `OD-*`
  are each rejected naming the row.
- No section has a uniformly-zero measured scalar; every `location` matches its manifest.
- Byte-identity holds from a second root (re-proven, since new sections are new path surface).
- `check`, `project --check`, `cargo test --workspace`, clippy, fmt all green.

## Failure injection and recovery

Zero a section's measurements wholesale; set both judgement fields on a row; set neither; name
a non-existent `OD-*`; corrupt a `location`; regenerate from a different absolute root. Each
must fail naming the row and the section.

## Research candidates and selection metrics

None.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| Five new sections + two fields | ~420 | ~20 |
| `census.schema.json` + validation | ~180 | 0 |
| Tests | ~220 | 0 |
| Regenerated `census.json` | ~+1,900 lines | 0 |

## Stop / rollback rule

Stop if any judgement field is non-null; if an axis is declared covered by a section that
measures nothing; if byte-identity fails from a second root; if a uniformly-zero scalar ships;
or if the schema passes a census that violates XOR. Rollback is one commit.

## Independent Review

_Pending._

## Completion record

_Pending._
