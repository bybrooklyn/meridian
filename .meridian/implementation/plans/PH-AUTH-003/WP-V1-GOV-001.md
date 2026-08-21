# WP-V1-GOV-001 — v1 governance schemas and validator

Revision 2. Revision 1 received `rethink`: it substituted a deliverable the phase did not
charter, and re-implemented working code. See Independent Review.

## Ownership

- Owning phase: `PH-AUTH-003` — Build the v1 governance schemas and validator
- Work package declared at `MERIDIAN_SPECOMENT.md:30156`
- Requirements and governing authorities: `SPEC-ROOT-001`, `SPEC-001`, `SPEC-002`,
  `GOV-COVERAGE-002`, `IMPL-BOOTSTRAP-001`, `IMPL-STATE-001`, Appendix D (projection rules
  1-8), Appendix H.4 (evidence index shape), Appendix H.5 (projection stamp)
- Depends on: `PH-AUTH-001`, `PH-AUTH-002` — both closed
- Branch: `v1-authority-reset` at `14d3feb`
- Primary semantic seam: **what the v1 validator considers authority, and what it refuses.**

## User-visible / operational result

`cargo run -p meridian-spec -- check` runs the **v1** validator. The v0.5 entry point becomes
`check-v05` for the mixed window. A recorded coverage matrix maps every v0.5 check to its v1
successor or to an explicitly recorded intentional drop, so `PH-AUTH-004` can delete the v0.5
validator as a pure deletion, without losing enforcement and without renaming the surviving
command.

## Current source diagnosis

Measured at `14d3feb`. `DIAGNOSIS-CARRIED-FORWARD.md` in this directory carries two stale
figures (2,188 lines and 39 tests / 417 lines) and is annotated accordingly; this section
supersedes it.

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
  is misnamed, the mandatory digest is absent, and `records` is empty while **15 artefacts sit
  in that directory** and four work packages are recorded complete. This violates
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

### 1. Suite equivalence is the primary artefact

The root's Tests line for this package names "suite equivalence", and the phase card's closure
evidence ends "The new validator runs against the staged suite." That is the precondition
letting `PH-AUTH-004` delete the v0.5 validator without losing enforcement.

Deliver `governance/coverage-matrix.md`, generated, mapping each v0.5 check —
`check`, `validate docs`, `validate schemas`, `validate maturity`, `validate evidence`,
`validate workloads`, `validate adrs` — to its v1 successor rule **or** to an explicitly
recorded intentional drop with a reason. That matrix is what licenses the deletion at 004.

Revision 1 substituted *root-to-projection* equivalence, which is a different thing and is
**already shipped**: `project --check` regenerates, compares byte-for-byte and names
divergences. It is cited here, not rebuilt.

### 2. `check` becomes the v1 validator; `check-v05` is the mixed-window name

Inverting revision 1's `validate-v1`. At `PH-AUTH-004` the v0.5 command is deleted and the
surviving command needs no rename, so no v1 name ever carries a migration-era suffix.

### 3. Ten chartered registries, honestly

| Registry | Disposition |
|---|---|
| requirements | shipped at `PH-AUTH-002` |
| phases | shipped at `PH-AUTH-002` |
| research gates | shipped at `PH-AUTH-002` |
| near-term work packages | **new** — the root declares 12 `WP-V1-*` identifiers at lines 30144-30175 |
| dependencies | **new** — 98 of 99 phase cards carry `depends_on` |
| maturity | **new** — verbatim labels plus the §0.3/§0.4/§0.7 axis content |
| evidence | **new** — Appendix H.4 shape, with the 15 existing artefacts registered |
| waivers | **empty typed registry** with schema and stamp |
| releases | **empty typed registry** with schema and stamp |
| compatibility | **empty typed registry** with schema and stamp |

An empty typed registry is the honest deliverable for the last three. Omitting them makes
`PH-AUTH-004` inherit undeclared scope.

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
  `appendix_a_divergences` into `identifiers.json`;
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
`governance/schemas/`. Appendix E says *likely* files, so no waiver is needed — but §0.5 and
`CLAUDE.md` §1 forbid resolving a conflict silently, so this is recorded as `SD-007` rather
than exercised quietly.

`check-v05` and `check` run side by side through the mixed window. The coverage matrix is what
licenses removing the former.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime, UI or dependency surface. Offline; reads and writes inside the repository.
Provenance improves: prose rules become executable, and the evidence index stops claiming zero
records while fifteen artefacts exist.

**Appendix D obligations.** Rule 7 (fail CI when misleadingly stale) is **Deferred to
`PH-AUTH-004`**, which owns CI; between now and then nothing enforces staleness in CI, and
that is stated rather than left silent. Rule 8 (distinguish user documentation from governance
authority) is satisfied by path: `governance/` is governance, `docs/` is documentation.

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

| Area | Added | Removed |
|---|---|---|
| Production — rules, registries, coverage matrix | ~500 | ~0 |
| Production — `main.rs` command rename and dispatch | ~15 | ~10 |
| Tests and fixtures, incl. synthetic violating specoments | ~500 | ~0 |
| Schemas, hand-written | ~350 | 0 |
| Generated registries | ~1,500 | 0 |

Lower than revision 1's ~600 production because rules 6 and 7 were re-implementations of
shipped code. The genuinely new logic is the DAG check, the axis-tagged enum check, the
evidence-index conformance check and the Appendix A reconciliation.

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
| 6 | Evidence index broken and unregistered | **Confirmed**: `schema_version` not `schema`, no digest, 0 records vs 15 artefacts |
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

### Disposition

All fifteen findings adopted. Returned for re-review.

## Completion record

_Pending._
