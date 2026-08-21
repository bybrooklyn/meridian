# WP-V1-GOV-001 — v1 governance schemas and validator

Revision 4. Revision 1 received `rethink`; revisions 2 and 3 received `revise`. See Independent Review.

**Every figure below is stamped with the command that produced it, at `14d3feb`.** Two figures
in revision 2 were wrong because they originated in the reviewer's own findings and were
adopted verbatim on the strength of their source. A reviewer's number is not evidence either.

## Ownership

- Owning phase: `PH-AUTH-003` — Build the v1 governance schemas and validator
- Work package declared at `MERIDIAN_SPECOMENT.md:30156`
- Requirements and governing authorities: `SPEC-ROOT-001`, `SPEC-001`, `SPEC-002`,
  `GOV-COVERAGE-002`, `IMPL-BOOTSTRAP-001`, `IMPL-STATE-001`, Appendix D (projection rules
  1-8), Appendix H.4 (evidence index shape), Appendix H.5 (projection stamp)
- Depends on: `PH-AUTH-001`, `PH-AUTH-002` — both closed
- Branch: `v1-authority-reset` at `14d3feb`
- Primary semantic seam, **compound and deliberately named**: *the v1 governance authority
  boundary, and the evidence that licenses retiring v0.5.*

  Revision 3 declared a single seam — what the validator considers authority — then made the
  coverage matrix the primary artefact. The matrix asserts nothing about what counts as
  authority; it licenses a deletion. That is a migration seam, and disguising two seams as one
  makes the package less reviewable, not more. Named rather than split: the matrix is only
  derivable once the v1 rules exist, so splitting would put it in a package depending on the
  first, with `PH-AUTH-004` blocking on the second — a serial dependency in front of the
  cutover for no reviewability gain.

## User-visible / operational result

`cargo run -p meridian-spec -- check` runs the **v1** validator. The v0.5 entry point becomes
`check-v05` for the mixed window. A recorded coverage matrix maps every v0.5 check to its v1
successor or to an explicitly recorded intentional drop, so `PH-AUTH-004` can delete the v0.5
validator as a pure deletion, without losing enforcement and without renaming the surviving
command.

## Current source diagnosis

Measured at `14d3feb`. **`DIAGNOSIS-CARRIED-FORWARD.md` in this directory is superseded by this section.** It carries
four figures that are now wrong: 2,188 `main.rs` lines (2,212), 39 tests in a 417-line suite
(50 in 728), "~40-entry `VALID_STATUSES`" (46), and "122 identifier families", which is not the
output of any defensible rule and is withdrawn. Rather than leave a partially-corrected input
in the plan directory, that file is marked superseded in full.

- `main.rs` is **2,212** lines, hard-coding a 37-entry `DOMAINS` list and a 46-entry
  `VALID_STATUSES` list, both v0.5 vocabulary.
- `governance/generated/` holds **five** files — `index.md`, `identifiers.json`,
  `requirements.json`, `phases.json`, `research-gates.json` — with `governance/manifest.json`
  one level above. Revision 1 said six, restating the tool's `wrote 6 projections` stdout,
  which counts the manifest, as a directory fact. Only four are JSON; `index.md` cannot carry
  a JSON Schema.
- **Family vocabulary, under one stated rule.** The projection reports 117 families. That is
  **115 families of declared identifiers plus 2 families of retired-v0.5 identifiers** — `RG`
  from `RG-TOR-001` and `WP-UI` from `WP-UI-006`. `undeclared` is empty, so no
  "referenced-member" filter does any work. Revision 1's "122" is not the output of any
  defensible rule and is withdrawn. **The derived v1 vocabulary is therefore
  `families − retired_v05 families` = 115.** Deriving from 117 would admit `RG` and `WP-UI`
  as valid v1 families, which `index.rs` explicitly forbids.
- §0.3 defines exactly 6 maturity labels; headings use 29 distinct suffixes, 24 unlisted.
  `Deferred program` is defined in §0.3 and used in **no** heading.
- `.meridian/implementation/evidence/index.json` is `{"schema_version": 1, "records": []}`.
  Appendix H.4 mandates `{"schema": 1, "specoment_sha256": "...", "records": [...]}`. The key
  is misnamed, the mandatory digest is absent, and `records` is empty while **14 artefacts sit
  in that directory** (15 files, one of which is `index.json`, the registry itself) and four work packages are recorded complete. This violates
  `IMPL-WP-003` item 6 and is the phase card's own "stale evidence" failure class, live in the
  repository now.
- **The root's Appendix A is the defective output this programme already fixed.** It carries
  **700** entries against the generated index's **736**; the 36 absent are exactly the
  `SD-001` range members, `SD-002`'s `NETPROJ-006A..D` and `SD-005`'s `SCM-010A`. Its own
  preamble asserts a MUST to "preserve zero-unmapped traceability" that it violates. Nobody
  made this argument for Appendix A; it was made only for Appendix G.
- `jsonschema` is pinned `default-features = false` (`Cargo.toml:26`), which disables
  `resolve-file`. Cross-file `$ref` will **not** resolve, silently.
- `phases.json`'s `gate` field carries 9 distinct free-text values including one-offs such as
  *"Conditional mobile; required architecture/handheld floor"*.
- `schemas/governance/` holds 23 v0.5 schemas; `specs/registry/` holds 22 v0.5 registries.
  Both retire at `PH-AUTH-004`.

## Approach

### 1. Suite equivalence is the primary artefact, at rule level

The root's Tests line for this package names "suite equivalence", and the phase card's closure
evidence ends "The new validator runs against the staged suite." That is the precondition
letting `PH-AUTH-004` delete the v0.5 validator without losing enforcement.

**Rule level, not command level.** `Command::Check` calls **15** sub-validators, and **8 of
them have no subcommand at all** — they are reachable only through `check`:
`validate_delivery_plan`, `validate_work_package_graph`, `validate_program_boundaries`,
`validate_marquee_policy`, `validate_ui_contracts`, `validate_validation_projects`,
`validate_dependency_strategy`, `validate_cross_references`. A seven-row command-level matrix
collapses all fifteen into one row and renders those eight invisible, so deleting the v0.5
validator on that evidence would silently drop eight enforcement units — precisely the failure
the matrix exists to prevent.

The real enforcement surface, measured:

```text
sed -n '/Command::Check => {/,/^        }/p' src/main.rs | grep -oE '[a-z_]+\(' | sort -u
  -> 15 sub-validators

regex over push/push_with_severity call sites in src/main.rs
  -> 65 distinct issue ids, 67 distinct (check, id) pairs
```

Revision 2's reviewer measured 64 and 66. This plan uses **65 and 67** and states the command,
because two figures adopted from that reviewer in revision 2 proved wrong. The exact count is
not load-bearing; the matrix is generated from the extracted set, so whatever the set contains
is what must be mapped.

**Partitioned by owning command.** The 65-id set is *not* a subset of what `check` runs:
`stale-projection` is emitted from `Command::Project` (`main.rs:259`), which `PH-AUTH-004`
does not delete. Demanding a successor for every id would conflate "enforcement being deleted,
needs a successor" with "enforcement being retained, needs nothing". The matrix therefore
partitions the extracted set by owning command and requires a successor or drop reason **only
for ids emitted by code the cutover removes**, recording the remainder as retained.

Deliver `governance/coverage-matrix.md`, **generated** by extracting the issue-id set from the
v0.5 validator and requiring each deleted-scope id to carry either a `v1_rule` or a
`dropped_because`.

**Every `v1_rule` row must name the fixture test that demonstrates the successor catches it,
and an unbacked row is a validation failure.** This is what keeps the mapping file from
becoming a second normative specification. The discriminator is falsifiability, not "tool
rules versus product requirements": *"v0.5 rule X is superseded by v1 rule Y"* is falsifiable
by fixture — take the input that tripped X, assert Y reports it. Unbacked, each row is an
unverified equivalence claim asserting coverage nothing checks, which is exactly what the
v0.5 deletion would be resting on.

**`dropped_because` is the genuine hazard and is constrained to a closed set.** A drop reason
claims enforcement is no longer required — a normative judgement, unfalsifiable by
construction, and the place where scope shrinks silently. Permitted values:

- `v05-authority-retired` — the rule policed `specs/`, which `PH-AUTH-004` deletes;
- `subsumed-by-root-structure` — the root makes the condition unrepresentable;
- `no-v1-analogue` — **escalates to the owner** as an open decision alongside `OD-009`, and is
  not resolvable inside a data file. Being
generated from the code it is a derived artefact, not restated prose, so the stop condition on
prose duplication is not tripped — hand-writing 65 rows of English would trip it.

This is also the **only rule in the package that is red on day one**: all 65 ids currently map
to nothing. Every other rule needs a synthetic fixture to fail.

Root-to-projection equivalence — a different thing, which revision 1 wrongly substituted for
this — is **already shipped** as `project --check` and is cited, not rebuilt.

### 2. `check` becomes the v1 validator; `check-v05` is the mixed-window name

Inverting revision 1's `validate-v1`. At `PH-AUTH-004` the v0.5 command is deleted and the
surviving command needs no rename, so no v1 name ever carries a migration-era suffix.

### 3. Ten chartered registries, split by class

The ten do not share a source, and treating them as one class would destroy data.

**Class (a) — root projections.** Derived from `MERIDIAN_SPECOMENT.md`, emitted by
`emit::all()`, policed by `project --check` byte-identity.

| Registry | Disposition |
|---|---|
| requirements | shipped at `PH-AUTH-002` |
| phases | shipped at `PH-AUTH-002` |
| research gates | shipped at `PH-AUTH-002` |
| near-term work packages | **new** — 12 `WP-V1-*` declarations under `# Near-term work-package decomposition` (line 30138), spanning 30142-30219. `grep -c '^## WP-V1-' MERIDIAN_SPECOMENT.md` → 12 |
| dependencies | **new** — 98 of 99 phase cards carry `depends_on` |
| maturity | **new** — verbatim labels plus the §0.3/§0.4/§0.7 axis content |

**Class (b) — accumulated state.** Sourced from test runs, approvals and shipped artefacts —
**not** from the root, and not derivable from it. Shipped as a **schema plus a conformance and
completeness rule**, and deliberately kept **outside `emit::all()`**.

| Registry | Disposition |
|---|---|
| evidence | schema (Appendix H.4) + conformance rule + registration of the 14 existing artefacts |
| waivers | **schema only.** No instance until a waiver exists |
| releases | **schema only.** No instance until a release exists |
| compatibility | **schema only.** No instance until a window is declared |

**Why the split is not a technicality: emitting class (b) would destroy data.**
`specoment::run` writes every projection unconditionally — `fs::write(&target, &projection.contents)`
at `mod.rs:91`, with no merge and no read-back. Putting the evidence index into `emit::all()`,
as revision 3 directed, means the next `cargo run -p meridian-spec -- project` **overwrites
`.meridian/implementation/evidence/index.json` with an empty regeneration, destroying every
registered record** — the exact inverse of this package's goal. `project --check` would then
report correctly-registered evidence as "stale or hand-edited".

Revision 3's further claim that these would be "self-populating the moment the root declares a
waiver" is also false for class (b): a waivers query over the root returns zero rows *forever*,
because the root is a design document and cannot contain an approver, an expiry or a CI run id.

**Evidence from the frozen baseline.** `specs/registry/` holds `evidence.json` (**52** records,
with fields like `source` naming gate invocations and `limits` naming NotRun rows) and
`waivers.json` (records carrying `approver`, `approval_role`, `expires`, `blocked_milestone`).
None of that content exists anywhere in the specoment and none of it could. And after a
complete milestone programme through MS-02, v0.5 created **no releases and no compatibility
registry at all** — which is evidence those two are not registry-shaped, and that the honest
deliverable for them is the schema alone.

### 4. Rules

Each gets a fixture that violates it. Rules over corpus properties already computed —
zero-unmapped, multiply-declared, retired-leakage — are tested against a **synthetic
violating fixture specoment**, never the live root, because the live root satisfies them today
and such a test has never been red.

- phase DAG resolves and is acyclic (genuinely new logic; no cycle detection exists today);
- one owning heading per identifier;
- **maturity labels are carried verbatim and are never normalised.** See below;
- §0.4 axis values validate **only where the containing field declares its axis**. See below;
- evidence index conforms to Appendix H.4 and every artefact present is registered;
- projection stamps carry the four H.5 fields and the current canonical digest;
- Appendix A reconciliation, symmetrical to the existing Appendix G handling, emitting
  `appendix_a_divergences` into `identifiers.json`. **This package delivers detection only.**
  The corresponding edit to canonical prose is an owner amendment — see `SD-009` below;
- no v1 content cites a retired v0.5 identifier as live authority.

### 5. No maturity normalisation table

Revision 1 declared a mapping from 29 heading suffixes onto §0.3's six labels. That is
withdrawn. Deciding that *"Normative ambition and core architecture"* means `Normative` rather
than `Normative direction` is a requirement-strength judgement the root never makes; 24
hand-written mappings asserting strength the specoment does not assert is a second normative
specification, which is this phase's literal stop condition, and breaches Appendix D rules 4
and 5. `DIAGNOSIS-CARRIED-FORWARD.md` §2 already called it an owner decision and revision 1
contradicted its own diagnosis.

Labels stay verbatim. The rule instead asserts that every heading-declared identifier carries
a label drawn from the **observed** set, so a new unlisted suffix surfaces as a divergence
rather than being absorbed silently.

### 6. Schemas are hand-written, shape-only, single-file

Required fields are limited to the four H.5 stamp fields plus the structural keys the emitter
actually writes. `gate`, `maturity_label` and every prose field are unconstrained strings — a
closed `gate` enum would invent a vocabulary the root never declares. A schema may not require
a field the emitter does not emit.

Given the `default-features = false` pin, schemas use **same-document `#/$defs`** only. A test
asserts the stamp constraint genuinely rejects a projection missing `canonical_sha256`, so the
schema is provably live rather than inert.

## Explicit exclusions

- No edits to `specs/`, `schemas/governance/`, `AGENTS.md`, `PLANNING.md`, `DCO.md`, CI.
  All `PH-AUTH-004`.
- No specoment body edits. Divergences are recorded, including Appendix A's.
- No maturity normalisation. No invented enums.
- No reconciliation of the four-way status vocabulary; that is a specoment amendment.
- `DEV-007`'s completion-record rule is **dropped**. See Independent Review finding 8.

## Compatibility / migration / authority effects

**Recorded path divergence.** The root declares this package's likely files as
`schemas/governance/*` and `specs/registry/*`. Both are live v0.5 paths that `PH-AUTH-004`
deletes wholesale; writing v1 content into them would make the cutover non-atomic and would
make "which suite is active" unanswerable by path. v1 schemas therefore go to
`governance/schemas/`. The root's own wording is "**Likely** files", and the enclosing section
`# Near-term work-package decomposition` (line 30138) hedges at line 30140: *"Only the
adoption/foundation frontier is decomposed to file-level work now."* So no waiver is needed —
but §0.5 and `CLAUDE.md` §1 forbid resolving a conflict silently, so this is recorded as
`SD-007` rather than exercised quietly.

Revision 2 attributed the "likely files" wording to **Appendix E**. That was wrong: Appendix E
begins at line 31393 and is *Source and provenance basis*. A divergence record whose authority
citation is fabricated is the exact defect class this programme keeps catching; corrected here.

`check-v05` and `check` run side by side through the mixed window. The coverage matrix is what
licenses removing the former.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime, UI or dependency surface. Offline; reads and writes inside the repository.
Provenance improves: prose rules become executable, and the evidence index stops claiming zero
records while fourteen artefacts exist.

**Appendix D obligations.** Rule 7 (fail CI when misleadingly stale) is **Deferred to
`PH-AUTH-004`**, which owns CI. The gap is wider than revision 3 stated: class (b) registries
are outside `emit::all()` and therefore have no `project --check` protection either, so
between now and 004 nothing at all would catch an evidence index drifting out of conformance.
Closed locally by running the class (b) conformance rule inside `check`, so local gates catch
it even though CI does not yet. Rule 8 (distinguish user documentation from governance
authority) holds across every artefact this package writes, which needs saying because two of
them are not obvious: `governance/coverage-matrix.md` is Markdown but is **generated
governance**, not documentation, and `.meridian/implementation/evidence/` is derived
bookkeeping under `IMPL-STATE-001`, which that contract states must not become a second
specification. `docs/` remains the documentation tree and this package does not write to it.

## Tests and evidence

Failure classes from the phase card, each with a violating fixture: malformed IDs, duplicate
authority, phase cycles, stale evidence, invalid status promotion, legacy-ID leakage,
canonical-equivalence gaps. Plus the Appendix A reconciliation, which fails today.

No test asserts a count. Corpus assertions are on the **set** or on an invariant.

Gates: `cargo test --workspace`, `fmt`, `clippy --workspace -D warnings`, `check-v05`,
`check`, `project --check`, `metadata --locked`, `git diff --check`.

## Failure injection and recovery

A malformed schema; a cyclic phase graph; a projection missing a stamp field; an evidence
index with unregistered artefacts; a v1 file citing a retired v0.5 identifier as live; a
cross-file `$ref` that would silently fail to resolve. Each must fail loudly and name the
offender.

## Research candidates and selection metrics

**One open question, not none.** Whether the 24 unlisted heading suffixes are normalised onto
§0.3's six labels, or §0.3 is extended to name the compounds, is an owner decision adjacent to
`OD-003`. This package does not decide it; it makes the current state visible and refuses to
absorb it. Recorded as `OD-009`, and it must land before authority freezes at `PH-AUTH-004`.

## LOC estimate

Revision 2 stated ~500 production while roughly doubling the deliverable set — seven new
registries, an Appendix A reconciliation, evidence-index conformance and a coverage-matrix
generator — and justified the reduction by removing two rules that were re-implementations.
That arithmetic was not honest: the removal is worth roughly 100 lines, and the additions are
worth far more. Restated with the basis shown so it can be audited rather than believed.

| Area | Added | Removed | Basis |
|---|---|---|---|
| Class (a) registries in `emit.rs` | ~200 | 0 | `wc -l emit.rs` = 398 for 5 projections + manifest ≈ **66 per output** × **3** new (work packages, dependencies, maturity) |
| Class (b) registrar — conformance and completeness, outside `emit::all()` | ~180 | 0 | schema validation, directory scan, registration; no emitter analogue |
| Appendix A reconciliation | ~250 | 0 | its structural analogue `phases.rs` is 314 lines; Appendix A is larger (700 entries across A.1/A.2/A.2b/A.3) but reuses `index.rs` |
| Coverage-matrix generator, DAG cycle check, axis-tagged enum check, evidence-index conformance | ~250 | 0 | four rules, no existing analogue |
| `main.rs` command rename and dispatch | ~15 | ~10 | |
| **Production total** | **~895** | ~10 | |
| Tests and fixtures, incl. synthetic violating specoments | ~500 | 0 | |
| Schemas, hand-written | ~350 | 0 | 10 registries, shape-only |

**~895 production, not ~500.** Revision 3 said ~975 on a basis of 66 × 7, but at most **3**
of the 7 new registries are `emit.rs` projections; the other four are class (b) and need a
different, separately costed mechanism. The total moved little; the basis is now auditable. If that is too large for one package the honest response is to
split the Appendix A reconciliation into its own package, not to shrink the number — a scope
signal pointing the wrong way is worse than none, and revision 2's stop rule would have fired
immediately and pointlessly.

## Stop / rollback rule

Stop if the validator needs prose duplication to understand authority; if a registry or schema
becomes a second normative specification; if any rule cannot fail; if a rule requires editing
the specoment to pass; or if the derived family vocabulary admits a retired-v0.5 family.
Rollback is deleting the branch commits.

## Independent Review

- Verdict: **rethink** (revision 1), 2026-08-20
- Reviewer: fresh isolated same-model context, no access to the drafting agent's reasoning.

### Why rethink was correct

**The package's primary artefact was wrong.** The root charters "suite equivalence" — that the
v1 validator covers what the v0.5 suite covered, which is what licenses deleting it at 004.
Revision 1 substituted root-to-projection equivalence, a different thing, **already shipped**
and listed in revision 1's own Gates. The package would have re-implemented working code and
delivered none of its chartered purpose.

### Findings verified independently and accepted

| # | Finding | Verified |
|---|---|---|
| 1 | Root declares this WP's files at line 30156; revision 1 excluded them silently | Recorded as `SD-007` |
| 2 | Suite equivalence absent; rules 6-7 re-implement shipped code | Restructured |
| 3 | Maturity table is an owner decision, contradicting this plan's own diagnosis | Table deleted |
| 4 | §0.4 enum rule cannot pass: four-way vocabulary conflict | Scoped to axis-tagged fields; `SD-008` |
| 5 | 10 chartered registries, revision 1 delivered 3 | All ten named |
| 6 | Evidence index broken and unregistered | **Confirmed**: `schema_version` not `schema`, no digest, 0 records vs 14 artefacts |
| 7 | Root Appendix A is the defective 700-entry output | **Confirmed**: 700 vs 736, 36 absent incl. all `AI-POLICY-*` |
| 8 | `DEV-007` not mechanically decidable; amends H.3; `state.json` cannot enlarge phase scope | Dropped |
| 9 | `validate-v1` guarantees rename churn | `check` / `check-v05` |
| 10 | Schema strategy self-contradictory; invented enums | Shape-only, hand-written |
| 11 | `jsonschema` `default-features = false` disables `$ref` file resolution | **Confirmed** at `Cargo.toml:26` |
| 12 | Rules 1, 3, 8 cannot fail first against the live corpus | Synthetic fixtures |
| 13 | Missing Requirements, added/removed LOC, affected authorities | Added |
| 14 | "No new dependency" framed against `LEGAL-006` | Moved to diagnosis as an observation |
| 15 | Appendix D rules 7 and 8 unaddressed | Both stated |

### Factual errors in revision 1, all confirmed

- "six projections" — five, plus a manifest one level up. Revision 1 restated the tool's
  stdout as a directory fact.
- "122 identifier family prefixes" — not the output of any defensible rule. Withdrawn.
- "117 … filters to families with a declared or referenced member" — mischaracterised. It is
  115 declared plus 2 retired, and deriving the v1 vocabulary from it would admit `RG` and
  `WP-UI`. This was the most consequential of the four.
- `main.rs` 2,188 → 2,212, corrected before the review returned.
- `DIAGNOSIS-CARRIED-FORWARD.md` carries stale 2,188 / 39-test / 417-line figures.

### Disposition — revision 1

All fifteen findings adopted; restructured around suite equivalence.

---

## Independent Review — revision 2

- Verdict: **revise**. Structure accepted; one blocking design flaw plus a dishonest estimate,
  a fabricated citation, and four wrong numbers.

### Blocking finding, verified independently

**The command-level matrix was far too coarse.** `Command::Check` calls **15** sub-validators,
of which **8 have no subcommand** and are reachable only through `check`. A seven-row matrix
collapses them into one row, so deleting the v0.5 validator on that evidence would silently
drop eight enforcement units. Measured enforcement surface: **65 distinct issue ids across 67
`(check, id)` pairs**. The matrix is now rule-level and generated from the extracted id set.

The reviewer's own measurement was 64 and 66. This plan states 65 and 67 with the producing
command, for the reason in the process note below.

### Findings accepted

| # | Finding | Disposition |
|---|---|---|
| 2 | Under-scoped: ~500 production while doubling the deliverable set | Restated **~975** with the `emit.rs` 398/6 ≈ 66-per-output basis shown |
| 3 | The three empty registries were scaffolding as specified — undefined referent, and a hand-placed file in a generated tree that `project --check` cannot police | Referents declared; registries **emitted** from queries returning zero rows |
| 4 | Deferring `SD-009` into `PH-AUTH-004` breaks the cutover's atomicity and is harder after the freeze | Re-dispositioned to an owner amendment landing **before** the freeze, tracked with `OD-009` |
| 5 | Appendix D rule 8 justification did not cover the artefacts actually written | Coverage matrix and `.meridian/` both addressed |
| 6 | Diagnosis annotation incomplete | File marked superseded in full, all four figures named |

### Factual errors corrected, and where they came from

| Error | Truth | Origin |
|---|---|---|
| "15 artefacts" in `SD-010` | **14** — 15 files, one being `index.json` itself | Mine, revising the reviewer's correct 14 upward |
| `WP-V1-*` at "lines 30144-30175" | **30142-30219**, section `# Near-term work-package decomposition` at 30138 | **Reviewer's revision-1 finding**, adopted verbatim |
| "Appendix E says likely files" | Appendix E is at 31393, *Source and provenance basis*. The wording is at 30140 | Mine |
| "releases — empty" | True only for shipped-release records; the 15 `RELEASE-*` identifiers are requirements already among 527 in `requirements.json` | **Reviewer's revision-1 assertion**, adopted verbatim |

Note the shape of the first: counting the container among its contents. That is the same error
as revision 1's "six projections", which counted the manifest. Twice in one package.

### Process note, adopted

Two of the four errors originated in the reviewer's own findings and were adopted because they
came from a reviewer. **The independent-review loop does not make a reviewer's numbers true.**
Every figure in this revision is re-derived from the repository at `14d3feb` and stamped with
the command that produced it — including the three the reviewer asked to be stamped: the issue
ids, the artefact count, and the per-projection basis for the estimate.

### Disposition — revision 2

All six findings and four factual corrections adopted.

---

## Independent Review — revision 3

- Verdict: **revise**. One blocking data-destruction hazard that revision 3 actively directed,
  one wrong basis line, and a correction that propagated to two of five sites.

### The 64/65 reconciliation, and what it exposed

The reviewer's extraction required the literal token `issues` as the first argument; one call
site passes `&mut issues` — `main.rs:259`, inside `Command::Project`, emitting
`("governance", "stale-projection")`. Their pattern dropped it: 76 sites / 64 ids / 66 pairs.
The broader pattern gives **77 / 65 / 67**. The stamped figure was right.

But that site carries a consequence worth more than the count. **`stale-projection` is emitted
from `Command::Project`, not `Command::Check`** — so the 65-id set spans commands the cutover
does **not** delete. Demanding a successor for all 65 would treat retained enforcement as
deleted. The matrix is now partitioned by owning command.

### Blocking finding — verified, and it would have destroyed data

**Four of the ten registries are accumulated state, not root projections.** Revision 3's
"they are generated, not hand-placed" directed emitting all of them, and
`specoment::run` writes unconditionally (`fs::write` at `mod.rs:91`, no merge, no read-back).
The next `cargo run -p meridian-spec -- project` would have **overwritten
`.meridian/implementation/evidence/index.json` with an empty regeneration**, destroying every
registered record, after which `project --check` would report correctly-registered evidence as
hand-edited.

Confirmed against the frozen baseline: `specs/registry/evidence.json` holds **52** records
sourced from gate invocations; `waivers.json` holds `approver`, `approval_role`, `expires`,
`blocked_milestone`. None of that content is in the specoment and none of it could be — a
design document cannot contain who approved a waiver or how a test run ended. And after a
complete milestone programme through MS-02, v0.5 created **no** releases or compatibility
registry, which is evidence those two are not registry-shaped.

Revision 3's "self-populating the moment the root declares a waiver" was also false: a waivers
query over the root returns zero rows forever.

### Findings accepted

| # | Finding | Disposition |
|---|---|---|
| 2 | The second seam is the coverage matrix, not Appendix A | Seam re-declared as compound and named; Appendix A stays in; package not split |
| 3 | The mapping file is safe only if falsifiable; `dropped_because` is the real hazard | Every `v1_rule` row must name its backing fixture; `dropped_because` constrained to three categories with `no-v1-analogue` escalating to the owner |
| 4 | Partition the id set by owning command | Done |
| 5 | Appendix D rule 7's gap is wider once class (b) leaves `emit::all()` | Conformance rule runs inside `check` locally |

### Factual errors corrected

- **The 14-artefact correction reached 2 of 5 sites.** Lines 128, 230 and 318 still said 15 or
  "fifteen", including the deliverable specification itself and a row recording as *Confirmed*
  a number revision 3 withdrew elsewhere.
- **The `~460 = 66 × 7` basis was wrong**: at most **3** of the 7 new registries are `emit.rs`
  projections. Restated at ~895 with class (b) costed separately.

### Process note, adopted

Stamping caught every fresh measurement error but not figures appearing in more than one
place. The discipline is extended: **after correcting a number, grep for every occurrence of
both the old and the new value and confirm the count.** That grep found lines 128, 230 and 318
mechanically, and is re-run after each edit to this plan.

### Disposition — revision 3

All findings and both factual corrections adopted. Returned for re-review.

## Completion record

_Pending._
