# WP-V1-RESET-002 — source diagnosis evidence

Gathered 2026-08-20T23:12:42Z at `ddbacd34361c302e72ed2accefd59fe7567b28fe`.
Subject: `MERIDIAN_SPECOMENT.md`, sha256 `782d3110b89ac23fa3f8cf80c07a72ba15e9de457717ca918a14f24e6d32692a`, 33175 lines.

This is pre-implementation diagnosis under `IMPL-AGENT-001` item 2, not a closure claim.

## Appendix F synthesis completeness gate

Appendix F states that a generation failing this gate MUST NOT replace the root canonical specoment.
Twelve of its clauses are mechanizable and were executed against the current file.

| Clause | Result | Observed |
|---|---|---|
| Markdown code fences balanced | Pass | 1,176 fence lines, even |
| 87 main-engine phase cards exist | Pass | 87 `## PH-*` cards, excluding PH-AI |
| 12 non-blocking PH-AI phase cards exist | Pass | 12 |
| Cards match embedded main phase registry | Pass | prose ID set == Appendix G `phases` ID set |
| Cards match optional AI registry | Pass | prose ID set == `optional_programs.temper_tally_tqf.phases` |
| Registry main phase count | Pass | 87 |
| Registry optional phase count | Pass | 12 |
| Every `depends_on` resolves | Pass | zero unresolved edges across all 99 phases |
| Phase graph is acyclic | Pass | DFS three-colour, zero back edges |
| `TQF-MASTER-001` embedded | Pass | present |
| `PRG-RECON-001` preserved | Pass | present |
| `SRV-017..022` preserves deferred AGPL orchestrator | Pass | all six present |

## Clauses not mechanizable at this checkpoint

| Clause | Status | Why |
|---|---|---|
| All 540 decisions from the prior canonical traceability appendix remain represented | NotRun | The prior appendix is not on this machine. The current generator reports 731 declared identifiers and 0 undeclared, which is a different and non-substitutable measure. Recorded NotRun rather than asserted from the 731 figure. |
| All 75 hard Meridian AI requirements remain represented | NotRun | The count refers to the AI ledger requirement set, not to backticked `AI-###` identifiers. Only ledger v0.1 is on this machine and it does not carry the 75-item enumeration. |
| No obsolete DCO authority survives as current law | Pass | 21 `DCO` occurrences inspected individually. All are prescriptive retirement instructions or historical description. `LEGAL-003` states the replacement. Zero occurrences assert the DCO as current policy. |

## Traceability index totals

731 declared, 0 undeclared, 2 multiply-declared, 2 retired-v0.5, 117 identifier families.

Multiply-declared: `NETPROJ-006` (five sub-contracts 006A-006D plus the base), `PRODUCT-002` (declared once, then inherited by a second heading). Residual family gaps: `AI-031..033`, `GOV-COVERAGE-001`, `ISO-004`, `ISO-005`, `MOD-001`, `TWO-001..003`. These are tracked as owner decisions OD-001 and OD-002 in `state.json` and are unresolved at this checkpoint.

## Contracts the plan must satisfy, found during diagnosis

Appendix H.5 mandates four stamp fields on every generated projection:
`canonical_path`, `canonical_sha256`, `generator_version`, `generated_at_source_checkpoint`.

Appendix D adds eight projection requirements: carry the hash; map each stable ID back to one canonical heading; be regenerable or reconciliation-checked; never silently override canonical prose; preserve research/deferred/qualification status; preserve zero-unmapped traceability; fail CI when stale in a misleading way; distinguish user documentation from governance authority.

## Maturity-label vocabulary drift (finding for PH-AUTH-003)

Section 0.3 defines exactly 6 decision maturity labels. Headings use 29 distinct suffix phrases.
5 phrases match a 0.3 label exactly; **24 do not**.

| Heading suffix | Count | In 0.3? |
|---|---|---|
| Normative | 437 | yes |
| Normative direction | 54 | yes |
| Derived normative contract | 48 | yes |
| Derived contract; prototype-gated | 10 | **no** |
| Research gate | 8 | **no** |
| Open implementation research | 4 | yes |
| Research/prototype gate | 3 | **no** |
| Normative model; syntax research-gated | 2 | **no** |
| Normative long-term direction | 2 | **no** |
| Required migration | 2 | **no** |
| Rejected | 1 | **no** |
| Research-gated leading direction | 1 | **no** |
| Normative language contract; implementation research-gated | 1 | **no** |
| Normative 1.0 floor | 1 | **no** |
| Normative development policy | 1 | **no** |
| Normative ambition and architecture | 1 | **no** |
| Normative ambition and core architecture | 1 | **no** |
| Research-gated | 1 | yes |
| Normative architecture | 1 | **no** |
| Normative ambition and initial architecture | 1 | **no** |
| Research-gated direction | 1 | **no** |
| Normative scope boundary | 1 | **no** |
| Normative product direction | 1 | **no** |
| Normative surface; parser details prototype-gated | 1 | **no** |
| Derived contract; evidence-gated | 1 | **no** |
| Post-1.0 normative direction; separate product | 1 | **no** |
| Post-1.0 planning seed; not a 1.0 requirement | 1 | **no** |
| Normative derived tooling contract | 1 | **no** |
| Non-normative clarification | 1 | **no** |

Consequence for this package: `requirements.json` cannot project a clean status enum at
`PH-AUTH-002` without inventing a vocabulary the specoment does not define. The projection
therefore carries the **verbatim** heading label as `maturity_label`. Normalization and
status-axis validation belong to `PH-AUTH-003`, whose phase card explicitly owns "status axes",
and whose validator must fail on any label outside its normalization table rather than
defaulting one. This satisfies Appendix D requirement 5 (preserve research/deferred/
qualification status) without a projection asserting a status the root file does not state.

## Identifier-vocabulary divergence (finding for PH-AUTH-003)

`editor/meridian_spec_tools/src/main.rs` hard-codes a 37-entry `DOMAINS` list and a
~40-entry `VALID_STATUSES` list, both v0.5 vocabulary.

The v1 specoment uses **122 identifier families**. Only **10** appear in both
vocabularies: `FWK`, `ISO`, `MOD`, `NAV`, `NET`, `PEN`, `PRJ`, `TOR`, `TWO`, `UI`.
Twenty-seven v0.5 domains have no v1 counterpart at all: `AGT`, `ANI`, `AUD`, `BAS`,
`BLD`, `COL`, `CORE`, `DAT`, `DCC`, `EDT`, `GAM`, `GOV`, `INT`, `MDL`, `PHY`, `PRC`,
`PRM`, `REL`, `RHI`, `RUN`, `SEC`, `SHD`, `SYN`, `VCS`, `VEG`, `WRL`, `XR`.

This is quantified support for the `PH-AUTH-003` phase card's instruction to "replace its
hard-coded v0.5 domain/status/ID assumptions". A 73% vocabulary replacement is not a
patch to the existing lists; the family vocabulary must be **derived from the specoment**
rather than hard-coded, or it will drift the moment a contract is added.

## Attribution rule confirmed against the reference generator

A naive reimplementation of the attribution rule was prototyped and produced **620 declared
and 31 multiply-declared**, against the reference generator's **731 and 2**. The divergence
is not noise: the reference generator distinguishes a heading that declares a **single**
identifier (a strong owner) from a heading that declares a **range** such as
`ED-AOT-001..005` (a weak family fallback, recorded only when no single-id heading claims
the member). Ranges plus per-member headings coexist throughout the document, so a port
that treats both as equal declarations reports 31 false duplicates.

Consequence: the port must reproduce the reference logic, and the closure test asserting
exact totals of 731 / 0 / 2 / 2 / 117 is a genuine falsifiable check rather than a
restatement of the implementation. A naive port fails it.

## Provenance dispositions are hard-coded in the reference generator

`gen_appendix_a.py` embeds the eleven-row A.4 identifier-gap disposition table as a Python
literal. Those rows are **owner-level provenance rulings** — which lost identifiers were
restored from which ledger, which were absorbed and must not be resurrected, which have no
provenance at all — not generator logic.

Porting that literal into Rust would put governance decisions inside a compiled tool, where
changing a ruling requires a code change and a rebuild, and where the ruling is invisible to
anyone reading the authority. It would also make the generator non-deterministic with respect
to the specoment: the same input would produce output containing assertions not derivable
from that input, which breaks Appendix D requirement 4 (a projection must never silently
override canonical prose) in the subtle direction — the projection would carry claims the
root file does not make.

Disposition for the port: the A.4 table becomes **data**, read from a provenance record
outside the generator, and the generator emits only what it can derive from the specoment
plus that declared input. The provenance record is itself reviewable and diffable.

This also matters for owner decisions OD-001 and OD-002: when those are ruled, the ruling
edits a data file, not a Rust source file.

---

# CORRECTION — the traceability index is not clean

An earlier section of this file, and a statement made to the owner, reported
**"731 declared, 0 undeclared"** and treated that as evidence the specoment satisfies
`PH-AUTH-002`'s closure row *"every canonical identifier is indexable exactly once."*

That was wrong. Independent review challenged the figure and both of its component
claims fail verification. The reference generator's totals line disagrees with its own
output, and neither reported duplicate is real.

## Defect 3 — 31 identifiers are counted as declared and never indexed

`gen_appendix_a.py` computes

```python
all_ids = sorted(set(list(owner) + list(refs)), ...)
```

**before**

```python
for i, v in fam_owner.items():
    owner.setdefault(i, v)
```

merges range-declared identifiers into `owner`. A range-declared identifier that appears
nowhere else therefore inflates `len(owner)` in the totals line but never enters `all_ids`,
so the A.1 emission loop never reaches it.

Measured: the totals line reports **731 declared**; A.1 contains **700 entries**. Delta **31**.

The 31 lost identifiers are five complete families:

| Family | Count | Declaring heading |
|---|---|---|
| `AI-POLICY-001..008` | 8 | Latest locked/derived product policies |
| `NORM-MIG-001..012` | 12 | Normative contradiction and migration map |
| `CODEHEALTH-001..004` | 4 | Code health and architectural decomposition |
| `OPEN-001..004` | 4 | Open source, not open core |
| `FWK-001..003` | 3 | Official framework package contract |

`NORM-MIG-*` is the contradiction and migration map — the family whose whole purpose is to
record what conflicts with what. It is indexable **zero** times. `AI-POLICY-*` is the family
onto which `AI-027..030` were restored earlier in this program; that restoration is
currently invisible in the index it was meant to appear in.

## Defect 4 — both reported duplicates are tooling artifacts, and four identifiers are erased

`BARE_ID_RE`'s trailing guard is `(?![0-9-])`. It excludes digits and hyphens but **not
letters**, so `NETPROJ-006A` matches as `NETPROJ-006`.

The specoment declares five distinct contracts here, each `— *Normative*`:

| Line | Heading | True identifier |
|---|---|---|
| 25396 | Prediction-safe code sharing | `NETPROJ-006A` |
| 25412 | Server-only secrecy and artifact closure | `NETPROJ-006B` |
| 25426 | Unified portable Authority path | `NETPROJ-006C` |
| 25448 | External services and authoritative barriers | `NETPROJ-006D` |
| 25468 | Bounded explicit compatibility windows | `NETPROJ-006` |

Four of the five are collapsed onto the fifth. `NETPROJ-006A..D` do not appear in the index
at all, and the collapse is then reported as a duplicate declaration of `NETPROJ-006`.

The second duplicate, `PRODUCT-002`, is a different artifact. Line 305 declares it; line
17707 is *"Ease-of-use condition inherited from PRODUCT-002 — *Normative*"*, a heading that
**references** the contract rather than declaring it. The attribution rule as written —
"the heading text contains the identifier" — cannot distinguish declaration from reference
in a heading.

## Corrected figures

| Measure | Reported | Verified |
|---|---|---|
| Declared identifiers | 731 | **735** (700 indexed + 31 lost range members + 4 collapsed `NETPROJ-006A..D`) |
| Entries actually indexed | 731 | **700** |
| Multiply declared | 2 | **0 real**; both are tooling artifacts |
| Undeclared | 0 | 0 — this one holds |

## Consequence

`PH-AUTH-002`'s closure row *"every canonical identifier is indexable exactly once"* is
**not currently satisfiable**, and was not satisfiable at the moment it was reported as
satisfied. Thirty-five canonical identifiers are indexable zero times.

The port must fix defects 3 and 4, and the closure test must assert the invariant
`len(A.1 entries) == declared_total` — the invariant whose absence let a 31-identifier
gap sit inside a number that read as clean.
