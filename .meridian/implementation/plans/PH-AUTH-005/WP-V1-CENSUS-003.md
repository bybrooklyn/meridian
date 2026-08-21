# WP-V1-CENSUS-003 — Census dispositions, ownership and phase closure

## Ownership

- Owning phase: `PH-AUTH-005`
- Requirement ids: `SPEC-001`, `SPEC-002`, `IMPL-STATE-001`, `PH-AUTH-005` implementation scope
  and closure evidence (specoment lines 30170–30176), Appendix D, §0.4.
  `IMPL-WP-001` governs the protocol. **`IMPL-WP-003` is deliberately not cited**: it is a
  declared identifier whose heading carries no maturity label, so `requirements.json` drops it
  and this package's own rule would reject it as an owner. The previous draft cited it.
- Depends on: `WP-V1-CENSUS-001` (`6b1f569`, repaired `440bf3e`) and `WP-V1-CENSUS-002`
  (`55c473f`). Measurement is complete across all ten card axes and is not reopened.
- Declaration status: **not a specoment-declared id**. Recorded in `state.json`, not authority.
- Branch: `main`
- Primary semantic seam: **what each measured thing should become, and who owns it next.**

## Current source diagnosis

`.meridian/implementation/census.json` at `55c473f`: 37 crates, 901 public types, 18 direct
dependencies, 7 features, 14 examples, 6 evidence runners, 15 formats, 791 tests, 9 generated
files, 3 CI rows, **96** edges (84 mandatory, 12 optional) over 8 layers. **1,801
judgement-bearing rows.** Every judgement field is null. Ten limitations recorded.

## The mappable universe — corrected twice

The first draft used `requirements.json` (527 ids) and declared 522 mappable. That was the
wrong projection. `requirements.json` is the subset of declared identifiers whose heading ends
in `— *Label*`; `identifiers.json` holds **736**, so 209 declared identifiers are invisible to
it, of which 96 are product requirements before range inheritance and 85 after — including `ED-AOT-001..005` and `EDUX-VIS-001..010`,
which are exactly what `meridian-ui-editor`'s 59 tests assert
(`creator_workspace_uses_the_locked_shell_and_world_width_priorities`). Under that draft their
honest owners did not exist, so ~120 editor tests would have become escalations manufactured by
a projection defect.

The second draft fixed the universe but **kept the exclusion keyed to the id's own label**, in a
universe that no longer requires ids to have one. That leaked six post-1.0 ids straight back in:

```text
### Deferred first-party self-hosted game-server orchestrator `SRV-016..022`
    — *Post-1.0 normative direction; separate product*        (line 28108)
#### Game-server-specific control plane `SRV-017`              (line 28114, no label)
#### AGPL service boundary `SRV-022`                           (line 28173, no label)
```

`SRV-016` is in `requirements.json` and was excluded; `SRV-017..022` are not, and passed as
legal v1 owners. The draft's own evidence clause — "the 5 non-v1 ids are rejected" — would have
gone green while a deferred, separately-licensed orchestrator stood as a mappable requirement.
That is the excluded-hard-case pattern recurring **inside the correction written to eliminate
it**, one level deeper, which is this lineage's documented failure mode.

**One rule, applied in both directions: a heading that declares a range and carries a label
gives that label to every member.** It excludes `SRV-017..022` and simultaneously recovers
`ED-AOT-001..005` as `Normative`.

```text
cargo run -q -p meridian-spec -- project
# range-inheritance derivation over identifiers.json + the specoment's range headings:
#   736 declared − 114 process − 11 non-v1  =  611 mappable
#   of which 526 labelled, 85 declared-but-unlabelled
```

The process filter `^(PH|WP|SD|OD|EV|IMPL-WP)-` removes **this programme's own phase,
work-package and decision identifiers** — things no code implements. Contracts code does
implement stay in, which is why `IMPL-STATE-001`, `IMPL-SCM-001` and `AGENT-SEM-*` are mappable
while `IMPL-WP-*` is not. `IMPL-WP-001` is cited in Ownership as protocol authority, never as an
owner id. Stated because the cut otherwise reads as arbitrary.

### The 85 unlabelled ids are a separate owner decision

The previous draft deferred these to `OD-009`. That was wrong on its own terms: `OD-009` asks
whether to *normalise 24 unlisted label suffixes onto the six defined in §0.3* — a question
about labels that exist. These 85 have **no label at all**, so `OD-009`'s question does not
reach them, and the draft's own stop rule forbids naming a non-covering record.

Measured, after range inheritance resolves the recoverable class to zero:

| Class | Count | Cause |
|---|---|---|
| range parent that is itself unlabelled | 35 | `AI-POLICY` 8, `NORM-MIG` 12, `EDUX-VIS` 10, `DIST` 5 |
| no range parent at all | 50 | `AI` 26, `SRV` 5, `TWO` 5, `MOD` 4, … |

A new record carries this, with those two classes as its closed options. They are legal owners
meanwhile, and that is recorded as a census limitation.

## Input: the `OD-010` ruling

The owner ruled **option A** — build v1 successors for the nine `no-v1-analogue` enforcement
units. That changes this package's inputs in three ways, stated here because a reviewer reading
the plan alone would otherwise not know.

- **8 test rows** take `retain`, not `remove`. Measured by scanning `#[test]` spans in
  `editor/meridian_spec_tools/tests/cli.rs` for the eight rule slugs. The ruling's first draft
  said 22 — an unstamped figure from a loose regex matching unrelated `waiver`/`adr`/`alluvium`
  tests, 2.75× the real count. Corrected in `state.json` as a noted correction rather than
  silently.
- **Their `next_phase` stays null**, pending `OD-013`. No open phase owns validator work:
  `PH-AUTH-003` is the conceptual home and is closed, `PH-AUTH-006` is structural decomposition,
  `007`–`010` are runtime. Writing a work-package id into a phase field would be `SD-012`'s shape
  a fourth time. These 8 rows are counted in the named residual this phase closes with.
- **The escalation bucket shrinks.** "Tests whose behaviour no v1 requirement describes" reserved
  a record for the v0.5-validator tests as its largest candidate population. Eight of them now
  have a disposition, so that record covers less than planned.

## Where the assignments live

1,801 judgements cannot live in Rust at ~0.4 lines each, and byte-identical regeneration means
they must be an **input the generator reads**, not an edit to its output. They live in
`.meridian/implementation/dispositions.toml`, checked in, read by `census::measure`.

**Structure: named rules plus an exception list — not 1,801 opaque rows.** ~2,000 inline-table
rows would be a machine-shaped file that no one hand-considers and no reviewer can distinguish
from a script. Instead the file declares perhaps 30 **named rules, each with its rationale**
(`R1: every public item of a crate dispositioned remove takes remove`; `R7: ui_runtime
accessibility tests map to UI-008`), plus per-row exceptions where a rule does not fit. A
reviewer reads ~30 rules and ~150 exceptions. Bulk assignment becomes a statement the file
makes about itself rather than a suspicion the reviewer must chase.

**Keys are `(file, function)` for tests and `(file, item)` for public types — never line
numbers.** Both are verified unique across all 791 and all 901 rows. Keying by line would turn
every unrelated source edit into a census failure under the "disposition naming a nonexistent
row is a hard error" rule. Failure injection covers this: insert a blank line, confirm no
disposition is orphaned.

## Approach

1. **Map two contrasting crates first, then set the budget.** `meridian-spec` (115 tests,
   governance toolchain) and `meridian-ui-editor` (59 tests, editor UX) are mapped and their
   escalation counts committed as **measured**. Only then is the workspace budget derived. The
   previous draft predicted ≤ 50 before any mapping existed; `-001` predicted ≤ 120 the same
   way. A budget invented before the work is the same failure twice.
2. **Assign crate dispositions from evidence.** A `retain` names (i) a mappable id the crate
   serves, (ii) a dependent or an explicit entry-point justification, and (iii) **at least one
   of its own retained tests mapped to the same id named in (i)**. The binding of (iii) to (i)
   is the correction: as previously written, (iii) required only "a non-empty set of test rows",
   which eliminated exactly the four crates that already failed (ii) and therefore discriminated
   nobody.
   Measured: 4 marker crates (`meridian-audio`, `-basalt`, `-isobar`, `-vegetation`) have 0
   dependents, 0 tests, surface 1 → `remove`, next phase `PH-AUTH-006`. Only 3 crates carry
   binaries (`meridian-build`, `meridian-editor`, `meridian-spec`), so the entry-point
   justification is available to those and no others. `meridian-physics` (0 dependents, no
   binary, 8 tests) and `meridian-shader-tools` (0 dependents, no binary, 4 tests) cannot
   satisfy (ii) and escalate. `meridian-ui` satisfies (ii) but declares 0 and re-exports 214
   with no migration deadline; it escalates as a façade decision.
3. **The test map is per-test, with file-level defaults and explicit per-test overrides.**
   "Module granularity" is withdrawn: across 791 rows there are **four** distinct module strings
   (`tests` 736, `<root>` 52, `enabled::tests` 2, `winit_adapter::tests` 1) and 66 `(file,
   module)` pairs against 65 files. Module granularity *is* file granularity — the degradation
   `-001` flagged and `-002` was meant to fix by adding the field. The override list is the
   evidence: it shows where a file genuinely serves more than one requirement.
4. **Public types get their own written rule.** 901 rows are the largest section and the
   previous draft gave them no evidence standard at all, making them the cheapest place in the
   artefact to bulk-default. Rule: an item is `retain` only if its crate is retained **and** it
   is named by a retained test or an example; otherwise `refactor` with a next phase, or it
   escalates. Sampled by the audit like any other section.
5. **Add `format_migrations` and edge reasons.** Every format not `retain` names its migration.
   `meridian-rhi → meridian-render-graph` is already *classified* (forward under the declared
   ordering); what it lacks is a **reason**, which is what `-001` promised.
6. **Add `next_phase` to `tests`, `generated_files` and `ci_rows`.**
7. **Tighten the XOR to exactly-one**, and validate the **on-disk** file, so a hand-edit reports
   as a schema violation naming the row rather than as generic staleness.

## Constraint regime — rebuilt

The previous draft's global floor of 60 distinct ids and flat 5 % concentration cap are both
withdrawn. They were not derived, and worse, they are satisfiable by a script: deal 791 rows
round-robin into 66 buckets and assign each bucket an arbitrary id, and the result shows 66
distinct ids, a 1.5 % maximum, and every multi-test crate touching ≥ 2 ids — passing all three
constraints with zero escalations and no honest content. The flat cap also **punishes real
clustering**: `meridian-ui-runtime`'s 119 accessibility and layout tests genuinely do serve a
handful of `UI-*` ids, and at file granularity seven groups already exceed 5 % of 791, so the
cap was unsatisfiable as well as gameable.

Replaced by:

- **Per-family cap, derived in step 1 — not pre-set.** A family cap is the right shape: spreading
  within a family does not evade it, and genuine clustering is not punished. But any number
  chosen before the mapping repeats the failure this plan diagnoses two sections earlier. A
  worked check shows why any pre-set number would have been wrong. The `UI` **requirement
  family** load is 197 — six `meridian-ui-*` crates, excluding `meridian-ui-editor`, whose 59
  tests this plan argues are asserted by `ED-AOT` and `EDUX-VIS` — plus 51 in
  `meridian_renderer/src/ui_direct.rs` (45) and `ui_direct_qualification.rs` (6), giving **248**
  against a 25 % ceiling of 197.75. Breached before a single judgement is made. The seven
  `meridian-ui-*` crates together hold 256 tests, but that is a crate-name prefix and not the
  family load; conflating the two would contradict this plan's own universe argument. The cap is therefore set from the measured `meridian-spec` and
  `meridian-ui-editor` mappings and recorded as measured.
- **No crate with ≥ 5 tests maps entirely to one id.** Retained — it does honest work.
- **The distinct-id count is recorded, not gated.** With ~30 named rules covering ~1,650 rows
  the map may carry as few as ~30–60 distinct ids, below the floor this plan withdrew.
  Withdrawing it was right and id-coverage is the better control, but the measured count goes
  in the completion record so "the map got coarser" is visible without re-deriving it.
- **Every distinct id used appears in the audit sample at least once**, with its heading text
  quoted beside one test's assertion. This is the control that kills round-robin: a script
  producing *n* ids must then defend *n* id-to-assertion pairs in prose.

## The audit

Two instruments, stated separately because merging them made the previous draft's "~60 rows"
arithmetically impossible — 37 crate rows + 23 groups of ≥ 10 tests + 8 non-test sections is
**68 before** the per-id stratum, and the per-id stratum is the largest.

**(a) Id coverage — a completeness obligation, not a sample.** Every distinct requirement id
used appears with its heading text quoted beside one test's assertion. Its size is unknown
until step 1 and is reported as measured. This is the control that kills round-robin: a script
producing *n* ids must then defend *n* id-to-assertion pairs in prose.

**(b) Sampled audit — 37 crate rows + 23 group rows + 8 non-test section rows + a seeded random
remainder, ≈ 75–80.** Seeded from `source_tree_checkpoint` with the seed recorded, so the draw
is reproducible and a sample chosen after the mapping is not passed off as a sample.

Arithmetic, stated rather than assumed: at *n* = 30 of *N* = 791 with 100 wrong, P(catch ≥ 1) =
98.4 %. So 30 detects a gross error — but a **clean** 30-row audit certifies only that fewer
than ~74 rows (9.4 %) are wrong at 95 %, a blind spot larger than any plausible escalation
budget. Instrument (b) below is sized accordingly.

**Routing, with a mechanism rather than an intention.** `IMPL-WP-001` mandates review *before*
implementation and defines no post-implementation step, so "goes to the reviewer" would in
practice mean self-certification. The contract's own re-entry clause is the hook: *"If
implementation materially exceeds it because the diagnosis or architecture changed, revise the
plan and repeat independent review before continuing."*

**Trigger, named explicitly:** the step-1 measured budget being exceeded, or **any audit row
failing**, re-opens independent review under `IMPL-WP-001`, and the reviewer — not the author —
accepts the written reason. The round is recorded in `state.json` `review_rounds`. Without this
the budget is advisory and the audit is self-graded. Four rounds on `-002` established that
self-review does not catch category errors: `meridian-spec`'s seven phantom re-exports came
from the doc comment on the function doing the counting.

## Owner decisions

**9 unresolved of 12 entries.** `OD-007`, `OD-010` and `OD-011` carry `resolved`; `OD-013`
was added by the same commit that resolved `OD-010`. The existence check must be
scoped to unresolved entries in `open_owner_decisions` — `collect_od_ids` currently harvests any
6-character `OD-` string anywhere in `state.json` (census limitation 7), so an escalation naming
the resolved `OD-011` would pass a check this package calls machine-checkable.

None of the 9 concerns what a test proves or what a crate should become, so new records are the
correct outcome. **Budgeted by decision, not by class**, each carrying `question`, a **closed
set of options**, `blocks`, and the default if unanswered — the shape `OD-007` and `OD-010`
already use. Without options a record is "what should we do about X", which is `undecided`
wearing an id.

Planned: one each for `meridian-physics`, `meridian-shader-tools` and `meridian-ui` (three
different rulings with different evidence, not one); one for the 3 macro-generated public types;
one per affected requirement family for tests whose behaviour no v1 requirement describes;
**one for the 85 unlabelled declared identifiers**, with the two measured classes as its options;
and **one for the layer ordering itself**.

That last is owed and the previous draft omitted it. Every `reverse` verdict is judged against
`census.json`'s `layers` array, which `-001` emitted as the census's own judgement while stating
that no ordering was declared anywhere. It has never been ratified, four crates sit in no layer,
and `PH-AUTH-006`'s "dependency graph obeys new layer rules" inherits it as if it were authority.

The `-001` invariant "escalation count equals open `OD-*` count" is **retired** as never
satisfiable, and replaced by: *every escalation names an unresolved `OD-*` whose question covers
that row.* Existence is machine-checked; coverage is checked by the audit.

## Closure: escalation versus "one disposition"

The card requires *"every code area has one disposition and next phase."* An escalation is not a
disposition — that is the point of the XOR. The previous draft asserted both halves in
consecutive sentences and they cannot both be true.

**Resolved explicitly.** Three row shapes exist, and the third was found by the `OD-010`
ruling rather than by this plan — an external input locating a seam the internal work could not
see, which is what an external input is for.

1. **Dispositioned** — a disposition and a next phase. The ordinary case.
2. **Escalated** — a next phase and no disposition, naming an unresolved `OD-*`.
3. **Dispositioned, phase pending** — a disposition and **no** next phase, naming an unresolved
   `OD-*` whose question covers the missing phase. The eight `OD-010` rows: the ruling
   established that the disposition is `retain`; only the owning phase is open.

Shape 3 is stated as a third interpretation of the card, not assumed — on the same footing as
the other two. Giving those rows `escalation: OD-013` instead would keep the tidier two-shape
model and discard what the ruling actually settled, so it is rejected.

`PH-AUTH-005` therefore closes with a **residual of two named counts** — escalated rows, and
dispositioned rows whose next phase is pending — recorded separately in `state.json`
and in the closure record. This is stated as an interpretation of the card, not assumed. The
alternative reading — that "one disposition" means "one judgement, disposition or escalation" —
is rejected here because `-001` argued at length that the two are distinct, and re-merging them
at closure would undo that.

## Explicit exclusions

- No re-measurement. `-002`'s output is the frozen base.
- No decomposition, crate removal or migration execution — `PH-AUTH-006`.
- No specoment edits. In particular this package does **not** fix the 85 unlabelled ids; that is
  the new owner-decision record above, not `OD-009`, whose question cannot reach them.
- The 476 transitive dependencies stay a recorded limitation and an `OD-006` escalation.

## Compatibility / migration / authority effects

None to runtime. Census stays class (c), outside `emit::all()`. `dispositions.toml` becomes a
new checked-in input; `governance/manifest.json` regenerates.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime surface. Provenance improves: every crate, public API, format, test and CI row stops
being implicitly retained and becomes a stamped decision with a named next phase or a named open
question.

## Tests and evidence

- Every judgement-bearing row has **exactly** one of `disposition` / `escalation`, across all
  ten sections.
- **`next_phase` may be null only when the row names an unresolved `OD-*` whose question covers
  the missing phase.** Without this, a null in a required field is `undecided` returning through
  the one field the XOR never covered — a row declining to name a next phase with no record of
  why. Enforced in the schema and machine-checked, not asserted. both-set, neither-set, out-of-vocabulary and unresolved-`OD-*` failures each
  rejected naming row and section.
- The **on-disk** census is schema-validated, not only generator output.
- Every retained test's owner is in the **611**-id mappable universe. `APP-003` (Rejected),
  `SRV-016` **and `SRV-022`** are rejected — the second proves range inheritance excludes
  unlabelled members of a labelled non-v1 range, which the previous draft admitted as owners.
  `ED-AOT-003` is accepted, proving inheritance recovers labelled range members.
  `EDUX-VIS-001` is accepted as one of the 85, proving unlabelled ids stay reachable.
- The four marker crates are `remove`; `meridian-physics`, `meridian-shader-tools`,
  `meridian-ui` and the layer ordering carry escalations.
- Per-family cap, per-crate single-id rule, and every-id-in-audit all assert and fail the build.
- A disposition naming a nonexistent row is a hard error; inserting a blank line orphans none.
- Escalation existence resolves only against **unresolved** records.
- Byte-identity holds across roots with `dispositions.toml` as an input.
- `check`, `project --check`, `cargo test --workspace`, clippy, fmt green.

## Failure injection and recovery

Set both judgement fields; set neither; name `OD-011` (resolved); name `OD-999`; assign outside
the vocabulary; map a test to `APP-003` (Rejected), `SRV-016` and `SRV-022` (post-1.0,
the second unlabelled); set `next_phase` null with no covering unresolved `OD-*`; delete a crate whose
disposition remains; insert a blank line and confirm no orphan; map every test in a crate to one
id; push a family past the step-1 derived cap; use an id that appears in no audit row. Each must fail naming the row.

## Research candidates and selection metrics

None.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| `dispositions.toml` — ~30 named rules plus ~150 exceptions | ~450 | 0 |
| Input parsing, rule expansion, merge, constraint checks | ~520 | ~20 |
| `format_migrations`, edge reasons, `next_phase` on three sections | ~180 | ~10 |
| Schema tightening, on-disk validation, `OD-*` status scoping | ~150 | ~30 |
| Tests | ~360 | 0 |
| Regenerated `census.json` | ~+2,400 changed lines | 0 |

## The sweep rule, generalised

This lineage has hit one failure class five times in five disguises: the fix lands, an earlier
sentence stays. Two passes exist and both have now been shown insufficient on their own.

- `-001` adopted **grep for the term just retired**. It catches retired words. It cannot catch a
  numeral that is spelled the same and has quietly become wrong.
- `-002` adopted **re-derive every figure from source before writing prose**. It catches stale
  measurements. It passed while four sites were stale, because those numbers were correct
  against source and contradicted a rule the same document had withdrawn.
- `-003` adopted **grep for the number a withdrawn rule contained**. It caught those four. It
  could not catch "9 open records", because nothing in the document was retracted — a fact in
  the accompanying `state.json` edit changed underneath a sentence that was true when written.

**The rule is keyed to the commit, and covers every artefact the commit touches: for each fact a
commit changes, grep both the plan and the changed artefact for the ids and counts that change
touches.**

Round 5 proved both halves necessary. It ran the plan grep for `OD-013` and `open records`,
found the string, and judged it correct — the sweep looked directly at the defect and passed it,
because `OD-013` had been added by the same commit and the sentence counting open records was
stale by one. And `OD-010.resolved.rationale` kept "All ten" through a commit whose entire
subject was that the number is nine, because only the plan was swept and the stale value sat in
a sibling field of the record being corrected.

## Stop / rollback rule

Stop if a crate is retained without all three pieces of evidence, or without one of its own
tests mapped to the id it names; if a test is mapped to a requirement that does not describe it;
if the measured budget from step 1 is exceeded without a written reason; if any family exceeds the
step-1 derived cap; if an id used appears in no audit row; if any row carries both or neither judgement field
once assignment is complete; if an escalation names a resolved or non-covering `OD-*`; or if a
new `OD-*` lacks options, `blocks` and a default.

The card's third stop condition — *"implementation maturity is promoted without new v1
evidence"* — cannot fire here: `implementation_maturity` is null in all 37 crate rows per
`SD-012`, and this package does not set it. Stated rather than omitted.

## Independent Review

_Pending._

## Completion record

_Pending._
