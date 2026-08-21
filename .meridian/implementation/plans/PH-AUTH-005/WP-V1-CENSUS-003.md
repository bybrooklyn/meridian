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

## The mappable universe — corrected, and the reason it matters

The previous draft used `requirements.json` (527 ids) and excluded 5 for non-v1 maturity,
declaring 522 mappable. That was the wrong projection, and the error was not neutral.

`requirements.json` is the subset of declared identifiers whose heading ends in `— *Label*`.
`identifiers.json` holds **736**. **209 declared identifiers are invisible to
`requirements.json`**, of which ~98 are product requirements — including every one of
`ED-AOT-001..005` (*Moderate professional density*, *Persistent top workspace strip*,
*Three-column World workspace with viewport priority*) and `EDUX-VIS-001..010`. They are
dropped because their labels sit on a parent heading (`ED-AOT-001..005 — *Normative*`, line
19468) rather than on each child.

Those are precisely the ids that describe `meridian-ui-editor`'s 59 tests —
`creator_workspace_uses_the_locked_shell_and_world_width_priorities`,
`world_tool_panel_headers_use_one_quiet_hairline_divider`. Under the previous draft their
honest owners did not exist, so ~120 editor-UX tests would have become escalations produced by
a projection defect rather than by a real open question.

**The universe is therefore derived from `identifiers.json`:**

```text
cargo run -q -p meridian-spec -- project   # regenerates both projections
python3 - <<'EOF'
import json,re
ids={x['id'] for x in json.load(open('governance/generated/identifiers.json'))['identifiers']}
req={r['id']:r for r in json.load(open('governance/generated/requirements.json'))['requirements']}
proc={i for i in ids if re.match(r'^(PH|WP|SD|OD|EV|IMPL-WP)-',i)}
nonv1={i for i,r in req.items() if r['maturity_label'] in (
  'Rejected','Post-1.0 planning seed; not a 1.0 requirement',
  'Post-1.0 normative direction; separate product','Research/prototype gate')}
print(len(ids-proc-nonv1))   # 617
EOF
```

**736 declared − 114 process − 5 non-v1 = 617 mappable**, of which 521 are labelled and **96
are declared but unlabelled**. The 96 are recorded as a census limitation and are legal owners;
the label gap is `OD-009`'s subject, not this package's to fix.

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

- **Per-family cap.** No requirement *family* (`UI`, `ED-AOT`, `SPEC`, …) owns more than 25 % of
  mapped tests. Spreading within a family does not evade it; genuine clustering is not punished.
- **No crate with ≥ 5 tests maps entirely to one id.** Retained — it does honest work.
- **Every distinct id used appears in the audit sample at least once**, with its heading text
  quoted beside one test's assertion. This is the control that kills round-robin: a script
  producing 66 ids must then defend 66 id-to-assertion pairs in prose.

## The audit

Seeded from `source_tree_checkpoint`, with the seed recorded, so the draw is reproducible and a
sample chosen after the fact is not passed off as a sample. **Stratified, ~60 rows**: all 37
crate rows (the highest-leverage section, and previously audited by nothing), one row per
`(file, module)` group of ≥ 10 tests, one row per distinct requirement id used, one row per
non-test section, plus a random remainder.

Arithmetic, stated rather than assumed: at *n* = 30 of *N* = 791 with 100 wrong, P(catch ≥ 1) =
98.4 %. So 30 detects a gross error — but a **clean** 30-row audit certifies only that fewer
than ~74 rows (9.4 %) are wrong at 95 %, a blind spot larger than any plausible escalation
budget. 60 stratified rows across all sections is the smallest honest instrument.

**The sample goes to the independent reviewer, not into a self-certified record.** Four rounds
on `-002` established that self-review does not catch category errors: `meridian-spec`'s seven
phantom re-exports came from the doc comment on the function doing the counting.

## Owner decisions

**9 open records, not 11.** `OD-007` and `OD-011` carry `resolved`. The existence check must be
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
one per affected requirement family for tests whose behaviour no v1 requirement describes; and
**one for the layer ordering itself**.

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

**Resolved explicitly:** an escalated row carries a `next_phase` and no disposition, and
`PH-AUTH-005` closes with a **named residual** — the escalation count recorded in `state.json`
and in the closure record. This is stated as an interpretation of the card, not assumed. The
alternative reading — that "one disposition" means "one judgement, disposition or escalation" —
is rejected here because `-001` argued at length that the two are distinct, and re-merging them
at closure would undo that.

## Explicit exclusions

- No re-measurement. `-002`'s output is the frozen base.
- No decomposition, crate removal or migration execution — `PH-AUTH-006`.
- No specoment edits. In particular this package does **not** fix the 96 unlabelled ids; that is
  `OD-009`.
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
  ten sections; both-set, neither-set, out-of-vocabulary and unresolved-`OD-*` failures each
  rejected naming row and section.
- The **on-disk** census is schema-validated, not only generator output.
- Every retained test's owner is in the 617-id mappable universe; the 5 non-v1 ids are rejected;
  `ED-AOT-003` is accepted, proving the 96 unlabelled ids are reachable.
- The four marker crates are `remove`; `meridian-physics`, `meridian-shader-tools`,
  `meridian-ui` and the layer ordering carry escalations.
- Per-family cap, per-crate single-id rule, and every-id-in-audit all assert and fail the build.
- A disposition naming a nonexistent row is a hard error; inserting a blank line orphans none.
- Escalation existence resolves only against **unresolved** records.
- Byte-identity holds across roots with `dispositions.toml` as an input.
- `check`, `project --check`, `cargo test --workspace`, clippy, fmt green.

## Failure injection and recovery

Set both judgement fields; set neither; name `OD-011` (resolved); name `OD-999`; assign outside
the vocabulary; map a test to `APP-003` (Rejected) and `SRV-016` (post-1.0); delete a crate whose
disposition remains; insert a blank line and confirm no orphan; map every test in a crate to one
id; push a family past 25 %; use an id that appears in no audit row. Each must fail naming the row.

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

## Stop / rollback rule

Stop if a crate is retained without all three pieces of evidence, or without one of its own
tests mapped to the id it names; if a test is mapped to a requirement that does not describe it;
if the measured budget from step 1 is exceeded without a written reason; if any family exceeds
25 %; if an id used appears in no audit row; if any row carries both or neither judgement field
once assignment is complete; if an escalation names a resolved or non-covering `OD-*`; or if a
new `OD-*` lacks options, `blocks` and a default.

The card's third stop condition — *"implementation maturity is promoted without new v1
evidence"* — cannot fire here: `implementation_maturity` is null in all 37 crate rows per
`SD-012`, and this package does not set it. Stated rather than omitted.

## Independent Review

_Pending._

## Completion record

_Pending._
