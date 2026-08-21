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
   declared/re-exported distinction the predecessor promised); backend dependencies (the 18 declared
   direct third-party crates only — **not** the 494 packages `Cargo.lock` resolves, see
   Provenance below — each with licence left null for `OD-006`);
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

Provenance improves, but narrowly, and the limit is stated rather than overclaimed: the
dependency section lists the **18 declared direct** third-party dependencies, not the 494
packages `Cargo.lock` resolves. `OD-006`'s `LEGAL-005` question covers all 494, so this section
does not answer it and does not claim to; the 494 figure is recorded as a measurement with the
exclusion named, in `census.json` and in the schema, not only here. Licence stays null
throughout. Carried to `WP-V1-CENSUS-003`.

## Tests and evidence

- All ten card axes have a populated section. The test lists them explicitly with the card
  sentence quoted beside it; it does not parse the specoment, and says so rather than implying
  a derivation it does not perform.
- `reexported_public_items` is non-zero for `meridian-ui`. A glob forwards the **crate-root**
  namespace, so the figure is 202 from globs plus 12 named in `pub use path::{A, B}`, giving
  214 — and `declared` is 0, because a facade that only re-exports declares nothing. The
  whole-tree alternative (210) is recorded as a rejected reading, not left ambiguous as in
  `WP-V1-CENSUS-001`.
- Every test row's `module` is non-empty and consistent with its `file`.
- Schema validates the census across **every** judgement-bearing section: both-set,
  out-of-vocabulary and non-existent `OD-*` are rejected naming the row and the section.
  Neither-set is deliberately still legal — it is the measurement-phase state of every row —
  and the test asserts that too, so the gap is checked rather than merely described.
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

- Verdict: **accept**
- Reviewer: fresh isolated same-model context. Four rounds: `rethink`, `revise`, `revise`,
  `accept`.
- The reviewer re-derived the artefact independently each round rather than reading the diff —
  a separate whole-file Rust scanner for module attribution, a standalone `jsonschema` harness
  outside the repo for the schema, and a second implementation of the re-export rule with a
  stricter local-module set than this one uses.

### What the rounds found

**Round 1** judged the original plan, which assigned dispositions. It returned `rethink` on the
ground that `PH-AUTH-005` names **ten** inventory axes and `WP-V1-CENSUS-001` had delivered
five: closing the phase on judgement over half a census would have closed it on the half that
was easy to measure. That is why this package was rewritten from judgement to
measurement-completion and the judgement work moved to `WP-V1-CENSUS-003`.

**Round 2** found three blocking defects in the implementation. The schema left
`generated_files` and `ci_rows` unconstrained, so a both-set row in either was accepted — the
XOR covering eight of ten sections, which is `WP-V1-CENSUS-001`'s "every row was false for 104
rows" error recurring inside the artefact written to fix it. Module attribution counted braces
on raw text, so a `}}` inside a string popped `mod tests` early. And `evidence_runners`
inventoried CI call sites rather than runners, hiding three that exist and are wired into no
workflow. It also identified two assertions that could not fail.

**Round 3** found that the re-export fix, applied to the crate everyone was looking at, had
broken four crates nobody was looking at: 97 of 322 items wrong, `meridian-spec` reporting 7
re-exports that existed only in this file's own doc comments.

**Round 4** found the completion record's headline figure stale by three, because the guards
added in round 3 are themselves rows in a census that measures this workspace.

### The pattern, and what carries forward

Three of this package's blocking defects were found only under review, and in each case the
artefact passed every gate while wrong. The common shape is a check that cannot fail: a schema
that constrains eight sections and is tested against one; an assertion that iterates a table's
own members; a "not all zero" that a 90 %% collapse would satisfy. Each fix therefore ships with
a guard that fails on the unfixed code — `a_crate_without_pub_use_reexports_nothing` fires on
`meridian-spec`'s 7, and `tests_inside_a_test_module_are_not_attributed_to_root` fires on the
`meridian-diagnostics` row.

The second pattern is self-measurement: a census that measures the workspace containing its own
source will count its own prose and its own tests. It has now done both — a `#[test]` grep
reading this file's comments, and a re-export counter reading its own documentation.

Adopted for `WP-V1-CENSUS-003`: **make every code change, regenerate, run the gates, and only
then write a number into prose.** `WP-V1-CENSUS-001`'s rule — grep the document for the term
just retired — catches retired words but cannot catch a numeral that is spelled the same and
has quietly become wrong.

### Residual items the reviewer accepted without blocking

Ten `limitations` entries in `census.json`, listed in the completion record's carry-ins. The
reviewer confirmed `PH-AUTH-005` does **not** close on this package: every axis is measured,
but owners, dispositions and next phases are null by design, and format migrations and
forbidden-edge reasons are absent. That is `WP-V1-CENSUS-003`.

## Completion record

### Sequence, stated plainly

Implementation preceded any review of this plan text. The commit hook blocked it twice —
first because `active_work_package` had been advanced to `WP-V1-CENSUS-003` before that package
had a plan, then because this plan carried no accepted review of itself. Both were sequencing
errors on my part, not hook noise. An earlier draft of this record was written in the past
tense before any review existed and claimed "landed at the commit carrying this record" while
nothing was committed; `state.json` simultaneously listed this package as complete and active.
All three are corrected here, and the second review round is what found them.

### Result

All ten card axes have a populated section: crates 37, public types 901, dependencies 18,
features 7, examples 14, evidence runners 6, formats 15, tests 791, generated files 9, CI rows
3, over 96 edges and 8 layers. Every judgement field is null, verified by sweep.

### What the second review round changed

Three blocking defects, all confirmed independently before being acted on.

**The schema left two of ten judgement-bearing sections unconstrained.** `generated_files` and
`ci_rows` carry `disposition` and `escalation` in the artefact but had no `properties` entry,
so `$defs/judged` never applied and a both-set row in either was accepted. This reproduced
inside the artefact the exact error it was written to fix — `CENSUS-001` claimed "every row"
and was wrong for 104. It went unnoticed because the failure-injection test injected only into
`crates`; it now loops over all ten sections.

**Module attribution was wrong, and the guard could not see it.** Brace counting ran on raw
text, so `assert!(json.ends_with("}}"))` popped `mod tests` off the stack early and one
`meridian-diagnostics` test was recorded at `<root>` beside eleven correctly-attributed
siblings in the same file. The first fix — a per-line literal stripper — was worse: it could
not see a multi-line `r#"{{ ... }}"#` JSON fixture and broke `meridian-asset-tools` too. The
working fix is a whole-file state machine handling nested block comments, raw strings with any
hash count, and lifetimes-versus-char-literals. An intermediate version silently lost four
newlines out of 1,784 by stepping over one at an escape, shifting every line number and
attributing `mod tests {`'s brace to the `#[cfg(test)]` above it; line-for-line correspondence
is now structural, since the caller indexes this output by source line. All 52 remaining
`<root>` rows are integration tests in `tests/`, which genuinely have no module.
`every_test_row_has_a_module` asserted only non-emptiness and `<root>` is non-empty, so the
guard is now "no test below a `mod tests {` is attributed to root."

**`evidence_runners` inventoried CI call sites, not runners.** Three runners that exist in the
repository are wired into no workflow and had no row, no disposition and no next phase — which
is precisely what a requalification census must surface. Runners are now enumerated from
runnable targets (examples and binaries, comments excluded) and unioned with CI invocations,
with `wired_in_ci` explicit. `promoting` now derives from `continue-on-error`, which is what
actually gates the build; the previous version keyed off the literal string "non-promoting" in
a step name and was correct only because label and flag happen to coincide today.

Two assertions that could not fail were replaced. `every_workspace_crate_has_a_layer` iterated
the layer table's own members, which is true by construction; it now asserts which four crates
no layer covers and that no edge touches one, so an unlayered crate with edges fails instead of
silently reporting every edge forward. And every axis now has a floor, because "not all zero"
would have passed a regression dropping 800 of 901 public types.

`reexported_public_items` took three rounds and was wrong in a different way each time.

Round 1 shipped no such field at all. Round 2 added one whose doc comment said a glob forwards
the crate-root namespace while the code summed whole-tree counts, and which counted named
re-exports nowhere. Round 3 fixed the crate everyone was looking at — `meridian-ui`, declared 0
and re-exported 214, being 202 crate-root items from four globs plus 12 named from
`meridian_ui_text` over four wrapped lines a single-line parser could not see — and in doing so
broke four crates nobody was looking at.

**The blast radius, stated rather than left to be discovered.** That fix gave
`meridian-renderer` 84, `meridian-spec` 7, `meridian-platform` 3 and `meridian-ui-render` 3,
where all four had been 0. Ninety-seven of the resulting 322 items were wrong, in two distinct
ways:

- `meridian-spec` declares **no** `pub use` at module root anywhere. Its 7 came from three doc
  comments in `census.rs` — including the doc comment on the very function doing the counting,
  which documents itself with an example of a named re-export. The census measured its own
  prose as public API. This is the same self-measurement class as the `#[test]` grep that read
  798 because this file mentions `#[test]` in its own comments, against a true 791; that one
  was caught and fixed by requiring a whole-line match, and this one was not caught at all
  until review.
- The other 90 were intra-crate: `meridian-renderer`'s `pub use camera::{Camera, ...}`
  re-surfaces items already inside its declared 93, so `declared + reexported` double-counted
  them. The field exists for exactly one purpose, argued at length in `WP-V1-CENSUS-001`: a
  facade that declares nothing and exposes hundreds. Only cross-crate re-exports serve it.

Now: literals and comments are stripped before counting, and a re-export whose first path
segment is one of the crate's own modules is excluded. Two crates report non-zero —
`meridian-ui` 214 and `meridian-ecs` 11 from `bevy_ecs::prelude` — and both are genuine
cross-boundary facades. The guard `a_crate_without_pub_use_reexports_nothing` fails on the
unfixed code; without it the next regression would have been exactly as invisible as this one.

### Factual errors corrected

The stamped `test_functions_total` command returned 7 when run literally, because `[test]` is a
character class; it now reproduces 791 exactly. `census.json`'s `assignment` string asserted an
exactly-one invariant that all ~2,000 of its own rows violated. The plan still claimed 213 in
one section while another explained the correction, claimed the dependency list covered 494
packages when it lists 18, claimed neither-set was rejected when it is deliberately legal, and
claimed a test enumerated axes from the card text when it hardcodes them. A duplicate
`## Completion record` heading sat at EOF.

Seven limitations are now recorded **in `census.json` and the schema**, not only here — the
recurring failure in this lineage being a limitation documented where the next reader will not
look.

### Evidence

`check` 0, `project --check` 0, clippy `-D warnings` 0, `fmt --check` 0, `cargo test -p
meridian-spec` 115 passed, `cargo test --workspace` 79 suites ok. Byte-identity re-proven across
roots after every structural change.

### Why the enumeration went stale, twice

The Result sentence said 788 tests against an artefact holding 791, and the stamped-command
paragraph said the same. Cause: the three guards added in the final round —
`evidence_runner_shape_is_pinned`, `a_crate_without_pub_use_reexports_nothing` and
`intra_crate_reexports_are_not_counted` — are themselves `#[test]` rows in the census that
measures this workspace. 788 + 3 = 791, and 112 + 3 = 115. Adding a test to a self-measuring
census changes the census.

Worse, 791 briefly meant two opposite things in one record: the rejected over-count in one
paragraph and the current correct count three paragraphs later. The naive grep now returns 798.

This is the third time in this lineage a figure went stale because a later change was not
propagated back to the prose, so the ordering is now a rule rather than an intention: **make
every code change, regenerate, run the gates, and only then write a number into prose.** The
predecessor's "grep the document for the term just retired" catches retired *words*; it does
not catch a numeral that is still spelled the same and has quietly become wrong.

### Carried to `WP-V1-CENSUS-003`

1. Escalation budget needs a floor, a concentration cap, and a sampled audit recording
   requirement text beside test assertion. A ceiling alone punishes only honesty.
2. `retain` must carry positive evidence, not absence of a reason to change — forcing real
   decisions on `meridian-physics` and `meridian-shader-tools` (zero dependents, not markers)
   and on `meridian-ui`.
3. New `OD-*` records authorised and budgeted; none of the 11 open records is about what a test
   proves. "Escalation count equals open `OD-*` count" is retired as never-satisfiable.
4. Post-1.0, deferred and rejected requirement ids are not mappable targets for a v1 test map.
5. The XOR tightens from "never both" to "exactly one".
6. `format_migrations`, forbidden-edge reasons, `next_phase` for tests/generated files/CI rows,
   rows for macro-generated public types and named re-exports, and the 476 transitive
   dependencies.
7. `schema_problems` should also validate the on-disk file so a hand-edit reports as a schema
   violation naming the row rather than as generic staleness.
