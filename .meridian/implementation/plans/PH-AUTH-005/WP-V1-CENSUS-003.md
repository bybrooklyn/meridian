# WP-V1-CENSUS-003 — Census dispositions, ownership and phase closure

## Ownership

- Owning phase: `PH-AUTH-005`
- Requirement ids: `SPEC-001`, `IMPL-WP-003`, `IMPL-STATE-001`, `PH-AUTH-005` implementation
  scope and closure evidence (specoment lines 30170–30176), Appendix D, §0.4
- Depends on: `WP-V1-CENSUS-001` (`6b1f569`, repaired `440bf3e`) and `WP-V1-CENSUS-002`
  (`55c473f`). Measurement is complete across all ten card axes and is not reopened here.
- Declaration status: **not a specoment-declared id**, as with `-002`. The specoment declares
  `WP-V1-CENSUS-001` only (lines 28629, 30170). Recorded in `state.json`, not new authority.
- Branch: `main`
- Primary semantic seam: **what each measured thing should become, and who owns it next.**

## User-visible / operational result

`PH-AUTH-005` closes. Every code area carries one disposition and a next phase; every retained
test carries a requirement id; everything undecidable carries an escalation naming an owner
decision whose question actually covers it.

## Current source diagnosis

`.meridian/implementation/census.json` at `55c473f`: 37 crates, 901 public types, 18 direct
dependencies, 7 features, 14 examples, 6 evidence runners, 15 formats, 791 tests, 9 generated
files, 3 CI rows, over 96 edges and 8 layers. Every `disposition`, `next_phase`, `owner` and
`escalation` is null. Ten limitations are recorded in the artefact.

The card's closure evidence needs three things this state does not have: each **retained** test
has an owner (so tests need a disposition before an owner means anything), every code area has
one disposition **and next phase**, and format migrations plus forbidden-edge reasons exist.

## Where the assignments live

This is the question `-002`'s review said the earlier draft never answered, and it constrains
everything else. Roughly 1,800 judgements cannot live in Rust source at ~0.4 lines each, and
byte-identical regeneration means they must be an **input the generator reads**, not an edit to
its output.

They live in `.meridian/implementation/dispositions.toml`, hand-authored, checked in, read by
`census::measure`. The census remains generated: `census.json` stays reproducible from source
plus this input, and `check` still regenerates and compares byte-for-byte. A disposition naming
a row that no longer exists is a hard error, so deleting a crate without retiring its
disposition fails the gate rather than silently orphaning it.

## Approach

1. **Assign crate dispositions from evidence, not from absence of a reason to change.** A
   `retain` must name (i) at least one `requirements.json` id the crate serves, (ii) a
   dependent or an explicit entry-point justification, and (iii) a non-empty set of census test
   rows as its behavioural contract. Failing any of the three escalates.
   The workspace as measured: 33 crates have dependents or tests; **4 marker crates**
   (`meridian-audio`, `-basalt`, `-isobar`, `-vegetation`) have zero dependents, zero tests and
   a surface of 1 — `remove`, next phase `PH-AUTH-006`. Only 3 crates carry binaries
   (`meridian-build`, `meridian-editor`, `meridian-spec`), so the entry-point justification is
   available to those and no others. That leaves **`meridian-physics`** (0 dependents, no
   binary, 8 tests) and **`meridian-shader-tools`** (0 dependents, no binary, 4 tests) unable to
   satisfy (ii) — they escalate rather than defaulting to `retain`. `meridian-ui` satisfies (ii)
   with 3 dependents but declares 0 and re-exports 214 with no migration deadline recorded; it
   escalates as a façade decision.
2. **Map tests to requirement ids at module granularity, now that `module` exists.** The
   mappable set is **522 of 527**: `APP-003` (Rejected), `EXEC-010` and `MODELER-RES-001`
   (Research/prototype gate), `PRG-RECON-001` (Post-1.0 planning seed), `SRV-016` (Post-1.0,
   separate product) are excluded — a v1 test map must not point at a rejected or post-1.0
   heading.
3. **Give tests a `disposition` before an owner.** The card says each *retained* test gets an
   owner, which presupposes tests can be dropped. Tests of code slated for `remove` take the
   disposition of their crate.
4. **Add `format_migrations` and forbidden-edge reasons** — the two card-scope items
   `-002`'s limitations 5 record as absent. Every format not `retain` names its migration; the
   `meridian-rhi → meridian-render-graph` edge is classified or exempted **with a reason**.
5. **Add `next_phase` to `tests`, `generated_files` and `ci_rows`**, the three sections that
   lack it, so "one disposition and next phase" holds for every code area rather than seven
   tenths of them.
6. **Tighten the XOR from "never both" to "exactly one"** in `census.schema.json`, and validate
   the **on-disk** file as well as generator output, so a hand-edit reports as a schema
   violation naming the row rather than as generic staleness.

## Escalation budget, derived

A ceiling with no floor punishes only honesty: bulk-assigning every test to a plausible id
produces zero escalations and passes comfortably. The budget is therefore a derived prediction
plus three distribution constraints.

**Predicted escalations, by named bucket with its reason:**

| Bucket | Rows | Why |
|---|---|---|
| `meridian-physics`, `meridian-shader-tools` | 2 crates | no dependents, no binary; v1 scope unclear |
| `meridian-ui` façade | 1 crate | 214 re-exports, no migration deadline |
| 476 transitive dependencies | 1 | inventoried as a limitation, not as rows; `OD-006` |
| macro-generated public types | 3 | no rows exist; cannot be assigned individually |
| tests whose subject is a post-1.0 or unwritten programme | ≤ 40 | concentrated in `meridian-modeler` (10), `meridian-alluvium` (7) and scattered singles |

**Predicted total: ≤ 50.** Note this is far below the earlier draft's invented "≤ 120": the
governance toolchain is *not* an escalation bucket, because `SPEC-002`, `IMPL-STATE-001`,
`PONDER-IMPL-008`, `BUILD-002`, `PRJ-006`, `PRJ-007` and `AGENT-SEM-005` are real v1
requirements that `meridian-spec`'s 115 tests and `meridian-build`'s 49 genuinely serve.

**Constraints, all of which fail the build:**

- **Floor:** at least **60 distinct requirement ids** across the test map. 37 crates each
  touching two or three requirements is ~90; fewer than 60 means bulk assignment.
- **Concentration cap:** no single id owns more than **5 %** of mapped tests (≈ 40 of 791).
  This binds hardest on `meridian-ui-runtime`, whose 122 tests sit in one file and cannot all
  share an owner.
- **No crate with ≥ 5 tests maps entirely to one id.**
- **Sampled audit:** 30 randomly drawn mapped rows, each recorded in the completion record with
  the requirement heading text beside the test's assertion. The audit is the evidence; the
  count is not. This is the only mechanism that can catch a test mapped to a requirement that
  does not describe it, since `AAA-001` would pass every automated check.

## Owner decisions

The 11 open `OD-*` records concern ledger provenance, the `must` convention, the substantiality
threshold, CI push authority, dependency licences, the preimage tag, maturity labels, orphaned
v0.5 rules, merge authorisation and `ExistingUnqualified`. **Not one is about what a test proves
or what a crate should become.** An honest escalation has nothing to name today.

Creating new records is therefore the correct outcome, not a workaround. This package is
authorised to add **one record per escalation class**, budgeted at **≤ 6**, each stating the
class and the rows it covers.

`WP-V1-CENSUS-001`'s invariant "the escalation count equals the count of open `OD-*` entries"
is **retired**: it was never satisfiable — 11 records against dozens of escalations — and both
`-001` and the first `-002` draft restated it without noticing. It is replaced by: *every
escalation names an `OD-*` that exists and whose question covers that row.* Existence is
machine-checkable; coverage is checked by the sampled audit. Without the coverage half, an
`OD-*` id is `undecided` in a costume, which is the failure the XOR was built to prevent.

## Explicit exclusions

- **No re-measurement.** `-002`'s output is the frozen base.
- No decomposition, crate removal or format migration execution — `PH-AUTH-006` acts on these
  dispositions.
- No specoment edits.
- The 476 transitive dependencies are not promoted to rows; they stay a recorded limitation
  and an `OD-006` escalation.

## Compatibility / migration / authority effects

None to runtime. The census stays class (c), outside `emit::all()`, policed by regeneration.
`dispositions.toml` becomes a new checked-in input; `governance/manifest.json` regenerates.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime surface. Provenance improves materially: every crate, public API, format, test and
CI row stops being implicitly retained and becomes a stamped decision with a named next phase
or a named open question.

## Tests and evidence

- Every judgement-bearing row has **exactly** one of `disposition` / `escalation`; both-set,
  neither-set, out-of-vocabulary and non-existent `OD-*` are each rejected naming row and
  section, across all ten sections.
- The on-disk census is schema-validated, not only the generator's output.
- Every `disposition` is drawn from `retain|refactor|replace|merge|split|remove`.
- Every retained test's `owner` is in the 522-id mappable set; the 5 excluded ids are rejected.
- The four marker crates are `remove`; `meridian-physics`, `meridian-shader-tools` and
  `meridian-ui` carry escalations, not `retain`.
- Floor, concentration cap and per-crate-single-id rule all assert and fail the build.
- Escalation total is ≤ 50 and new `OD-*` records ≤ 6.
- A disposition naming a nonexistent row is a hard error.
- Byte-identity holds across roots with `dispositions.toml` as an input.
- `check`, `project --check`, `cargo test --workspace`, clippy, fmt green.

## Failure injection and recovery

Set both judgement fields; set neither; name a nonexistent `OD-*`; assign a disposition outside
the vocabulary; map a test to `APP-003` (Rejected) and to `SRV-016` (post-1.0); delete a crate
whose disposition remains; map every test in a crate to one id; drive distinct ids below 60;
push one id past 5 %. Each must fail naming the row.

## Research candidates and selection metrics

None.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| `dispositions.toml` (hand-authored input, ~1,800 rows) | ~2,000 | 0 |
| Input parsing, merge, and the six constraint checks | ~450 | ~20 |
| `format_migrations`, edge reasons, `next_phase` on three sections | ~180 | ~10 |
| Schema tightening and on-disk validation | ~120 | ~30 |
| Tests | ~320 | 0 |
| Regenerated `census.json` | ~+2,400 changed lines | 0 |

## Stop / rollback rule

Stop if a crate is retained without all three pieces of evidence; if a test is mapped to a
requirement that does not describe it; if escalations exceed 50 or new `OD-*` records exceed 6;
if distinct ids fall below 60 or any id exceeds 5 %; if any row carries both or neither
judgement field once assignment is complete; or if an escalation names an `OD-*` whose question
does not cover it. Rollback is one commit.

## Independent Review

_Pending._

## Completion record

_Pending._
