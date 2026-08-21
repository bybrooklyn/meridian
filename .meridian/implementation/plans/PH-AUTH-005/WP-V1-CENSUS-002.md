# WP-V1-CENSUS-002 — Census dispositions and test ownership

## Ownership

- Owning phase: `PH-AUTH-005`
- Depends on: `WP-V1-CENSUS-001` — complete at `6b1f569`, measurement frozen
- Branch: `main`
- Primary semantic seam: **what each measured thing should become.**

## User-visible / operational result

Every crate, format, generated file and CI row carries a disposition from the specoment's
closed vocabulary, or an escalation naming an `OD-*` record. Every test function carries an
owner or an escalation. `PH-AUTH-005` closes on this plus `CENSUS-001`.

## Current source diagnosis

`.meridian/implementation/census.json` at `6b1f569`: 37 crates, 778 tests, 15 formats,
96 edges, 8 layers, 9 generated files, 3 CI rows. Every `disposition`, `escalation` and `owner`
is null. Measurement is frozen and this package does not re-measure.

## Approach

1. **`disposition` XOR `escalation`, enforced in the schema.** A row is valid iff exactly one is
   non-null. This is the mechanism whose failure silently readmits `undecided` under another
   name, so it is a schema constraint that fails closed, not a prose rule.
2. **`escalation` values validate as real `OD-*` ids present in `state.json`.** Free strings
   would make "escalation count equals open owner decisions" decoration.
3. Assign the mechanically determined dispositions:
   - four marker crates, 205-210 bytes, one `pub const SCAFFOLD_STATUS`, **zero dependents** →
     `remove`, owner `PH-AUTH-006` (`WP-V1-ARCH-001` names marker-crate removal);
   - the seven files over 240 KB the `PH-AUTH-006` card names by size → `split`, owner
     `PH-AUTH-006`;
   - `meridian-ui`, a façade re-exporting 213 root items while declaring none → `retain`;
   - everything else with no evidence for change → `retain`, owner `PH-AUTH-006`.
4. Map test functions to requirement ids **validated against `requirements.json`**, at module
   granularity. A module whose subject has no v1 requirement escalates rather than defaulting.
5. Record `format_migrations`: every format not `retain` names its migration.

## Escalation budget

Reporting a count is not a control; a number with no threshold is decoration. Predicted:

| Section | Rows | Predicted escalations |
|---|---|---|
| crates | 37 | 0 — every crate is mechanically determined |
| formats | 15 | 0 — all at version 1, none migrating yet |
| generated files, CI rows | 12 | 0 |
| tests | 778 | ≤ 120, concentrated in modules whose subject is a post-1.0 program |

**Overshoot is a stop condition.** More than ~150 test escalations means the mapping work was
skipped rather than done, and the package stops rather than shipping a census that passes every
assertion while containing no information.

## Explicit exclusions

- No re-measurement. `CENSUS-001`'s output is the frozen base.
- No decomposition, no crate removal — those are `PH-AUTH-006` acting on these dispositions.
- No specoment edits.

## Compatibility / migration / authority effects

None to runtime. The census stays class (c), outside `emit::all()`, policed by regeneration.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime surface. Provenance improves: crate and test disposition stops being implicit and
becomes a stamped record with an escalation path to named owner decisions.

## Tests and evidence

- Every row has exactly one of `disposition` / `escalation` non-null; both-set and neither-set
  are rejected.
- Every `disposition` is drawn from `retain|refactor|replace|merge|split|remove` — the closed
  vocabulary verified verbatim in the specoment at lines 28722 and 31389.
- Every `escalation` names an `OD-*` id present in `state.json`.
- Every test row resolves to an id present in `requirements.json`, or escalates.
- Escalation counts are reported and compared against the budget above.
- The four marker crates are `remove`; the seven oversized files are `split`.
- Regeneration is byte-identical; `check` reports stale after a source mutation.

## Failure injection and recovery

Set both fields on a row; set neither; name an `OD-*` that does not exist; assign a disposition
outside the vocabulary; map a test to a requirement id absent from `requirements.json`. Each
must fail naming the row.

## Research candidates and selection metrics

None.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| Assignment logic and validation | ~350 | ~10 |
| Tests | ~180 | 0 |
| Regenerated `census.json` | ~0 net, ~2,000 changed lines | 0 |

## Stop / rollback rule

Stop if a crate is retained solely because it exists; if a test is mapped to a requirement that
does not describe it; if escalations exceed the budget; if any row carries both or neither of
`disposition` and `escalation`; or if an escalation names a non-existent `OD-*`. Rollback is one
commit.

## Independent Review

_Pending._

## Completion record

_Pending._
