# WP-V1-RESET-002 — Canonical suite split (staged root authority + generated projections)

## Ownership

- Owning phase: `PH-AUTH-002` — Install the root specoment and generate validated projections
- Requirements: `SPEC-001`, `SPEC-002`, `SPEC-004`, `GOV-COVERAGE-002`, `IMPL-BOOTSTRAP-001`
- Branch: `v1-authority-reset`, currently at `ddbacd3`, identical to `main` and to tag `v0.5-final-baseline`
- Predecessor: `PH-AUTH-001` / `WP-V1-RESET-001` — closed; baseline frozen at `ddbacd34361c302e72ed2accefd59fe7567b28fe`
- Reviewer: see Independent Review below

## User-visible / operational result

On `v1-authority-reset`, `MERIDIAN_SPECOMENT.md` is tracked at the repository root as the staged v1 authority, and a generated `governance/` tree beside it makes that authority navigable and machine-readable. Every generated file names its canonical source, that source's sha256, and the generator version. Running the generator twice produces byte-identical output, and a hand edit to any projection fails a reconciliation check by name.

`main` is not touched. v0.5 authority continues to govern `main` until `PH-AUTH-004`.

## Current source diagnosis

Verified at plan time, not assumed. Counts corrected after independent review.

- `MERIDIAN_SPECOMENT.md` is present at the repository root, **untracked**, 33,175 lines,
  sha256 `782d3110b89ac23fa3f8cf80c07a72ba15e9de457717ca918a14f24e6d32692a`, matching
  `state.json`.
- `main`, `v1-authority-reset` and `v0.5-final-baseline` all point at `ddbacd3`. The branch
  has no commits of its own.
- `specs/` holds **44** v0.5 `.md` documents plus `specs/registry/` with 22 JSON registries.
  `schemas/governance/` holds **23** schema files; 22 correspond to a registry and
  `marquee-validation-fixture.schema.json` does not. Both trees remain authoritative on `main`.
- `editor/meridian_spec_tools` is one 2,188-line `src/main.rs` with a 417-line integration
  test. Its Markdown walking, heading parsing, link checking and JSON-Schema utilities are
  reusable; its 37-entry `DOMAINS` list and 46-entry `VALID_STATUSES` list are v0.5-specific.
  The v1 specoment uses **122 identifier family prefixes** counting every backticked
  identifier including retired-v0.5 citations such as `REUSE`, `SECURITY` and `WP-UI`, or
  **117** by the reference generator's rule, which filters to families having a declared or
  referenced member. Both rules are stated because the raw number is not self-explanatory.
  Under either rule the load-bearing fact is unchanged: only **10** families appear in the
  37-entry `DOMAINS` list, and **27** v0.5 domains have no v1 counterpart at all. Replacing
  that vocabulary is `PH-AUTH-003`.
- Appendix G's embedded JSON contains 87 objects under `phases` **and** 12 `PH-AI-*` objects
  under `optional_programs.temper_tally_tqf.phases`, matching the 99 prose cards by ID.
- Appendix D governs projections with eight requirements; Appendix H.5 fixes the stamp shape
  at four fields. Neither was cited by the previous plan revision.

### Reference-generator defects, all four in scope for the port

`.meridian/tools/gen_appendix_a.py` implements the attribution rule this package needs, but
its reported totals cannot be used as a port-fidelity target unchanged.

**Defect 1 — self-referencing declarations.** A declaring heading appears in its own
*also referenced in* list. 44 entries affected.

**Defect 2 — no-argument crash.** `main(sys.argv[1])` raises `IndexError` instead of printing
usage. A generator invoked by CI must diagnose, not traceback.

**Defect 3 — 31 identifiers counted but never indexed.** `all_ids` is computed *before*
`fam_owner` is merged into `owner`, so a range-declared identifier appearing nowhere else
inflates the totals line but never reaches the A.1 emission loop. Measured: totals line
reports 731, A.1 emits **700**. The 31 lost identifiers are five complete families —
`AI-POLICY-001..008`, `NORM-MIG-001..012`, `CODEHEALTH-001..004`, `OPEN-001..004`,
`FWK-001..003`. `NORM-MIG-*` is the contradiction and migration map; it is indexable zero
times.

**Defect 4 — letter-suffixed identifiers collapse, in two distinct shapes.**
`BARE_ID_RE`'s trailing guard `(?![0-9-])` excludes digits and hyphens but not letters, so
`NETPROJ-006A` matches as `NETPROJ-006`.

An exhaustive scan finds **exactly five** letter-suffixed identifiers in the document, all
five declared by a heading, and they fail in two different ways:

*Shape A — collapse across headings, surfacing as a false duplicate.* Lines
25396/25412/25426/25448 declare `NETPROJ-006A`/`B`/`C`/`D`, four distinct `— *Normative*`
contracts. All four collapse onto `NETPROJ-006` at 25468, vanish from the index, and the
collapse is then misreported as a duplicate declaration.

*Shape B — collapse within one heading, surfacing as nothing at all.* Line 14869 is
`### Source-control CLI family `SCM-010` `SCM-010A` — *Normative*`, declaring both on the
same line. Both match as `SCM-010`; the duplicate guard is
`if i in owner and owner[i][1] != hline`, and the line numbers are equal, so no duplicate is
reported and `SCM-010A` is **silently overwritten**. It appears nowhere in the index and
raises no signal.

Shape B matters beyond the one identifier: a test written only against the reported-duplicate
shape passes while shape B still loses identifiers. This is the second time in this review
cycle that a hard-coded enumeration proved incomplete, which is why the fidelity rule below is
now a derived invariant rather than a list.

The second reported duplicate, `PRODUCT-002`, is a different artifact: line 305 declares it,
line 17707 is *"Ease-of-use condition inherited from PRODUCT-002"*, a heading that
**references** rather than declares. The attribution rule cannot distinguish the two.

### Corrected figures

| Measure | Reference reports | Verified truth |
|---|---|---|
| Declared identifiers | 731 | **736** (700 indexed + 31 lost + 5 collapsed) |
| Entries actually indexed | 731 | **700** |
| Multiply declared | 2 | **0 real** |
| Undeclared | 0 | 0 |

## Closure position

**This package delivers `PH-AUTH-002`'s projections. It does not close the phase.**
Stated explicitly because `IMPL-AGENT-001` item 10 requires honest status and `IMPL-WP-003`
item 8 forbids a completion claim implying adjacent unimplemented capability.

| `PH-AUTH-002` closure row | Position after this package |
|---|---|
| Every canonical identifier is indexable exactly once | Achievable, and achieved only if defects 3 and 4 are fixed. Currently 36 identifiers are indexable zero times. |
| Headings, status, links, deferred/research markers pass | Partially. Heading and marker projection lands here; link and status validation is `PH-AUTH-003`. |
| Projection hashes pass | Yes, via the Appendix H.5 stamp and `project --check`. |
| Zero-unmapped checks pass | Yes for the index; enforcement as a gate is `PH-AUTH-003`. |
| No accepted decision is lost or duplicated | **Open.** See below. |

**On OD-001 and OD-002.** `state.json` records both as blocking this row. The honest reading
is narrower than that record implies: both concern identifiers for which **no accepted
decision was found in any ledger on this machine** — `TWO-001..003`, `GOV-COVERAGE-001`,
`MOD-001`, `AI-031..033`. Nothing accepted is known to be lost, because nothing accepted was
ever located. The row cannot be *proved* without the master ledger, so it is recorded
**Inconclusive**, not Pass and not Fail, and the phase stays open pending owner ruling.

The real, newly discovered threat to this row is not OD-001/OD-002 at all — it is defects 3
and 4, which lose 36 identifiers that *are* declared and *do* carry accepted decisions.
Those are fixed by this package.

## Approach

One primary semantic seam: **the root specoment becomes tracked authority and is projected
without any second authority arising.**

Stated as one clause deliberately. The previous wording joined two ideas with "and", which
invited the question of whether the package had split. It has not: the ~210 lines of SHA-256
and scanner code are utility modules with no independent user-visible result, and
`IMPL-WP-003` forbids closing a package on those. The only genuine split candidate — installing
the authority versus projecting it — is rejected because `project --check`,
`appendix_a_matches_generated_index` and the byte-identical regeneration test all require the
tracked file to exist, rollback is shared (delete the branch commits), and the split would buy
ordering constraints with no independent verification benefit. Findings 1 and 2 of review
round 3 both test exactly this one clause: no second authority in the working tree, and no
second authority smuggled in through an attribution rule that defines its own truth.

1. Set `active_phase` and `active_work_package` in `state.json`. The repository's own
   pre-commit hook blocks every `editor/**` commit until they are set, so this is the first
   deliverable, not bookkeeping.
2. Commit the specoment to `v1-authority-reset` at repository root, byte-for-byte unmodified,
   together with a **narrowed** subset of `.meridian/`.

   `IMPL-WP-003` item 7 requires implementation state to agree with source, which an untracked
   `.meridian/` cannot do. But committing the directory wholesale would do the opposite of this
   phase's purpose. `.meridian/authority/` contains:

   | Path | Size | Disposition |
   |---|---|---|
   | `MERIDIAN_SPECOMENT.original.md` | 1.7 MB, 32,804 lines | **Excluded** |
   | `appendix_a.md` | 748 lines | **Excluded** |
   | `baseline-manifest.md` | 126 lines | Committed |
   | `normative-language-audit.md` | 834 lines | Committed |

   `MERIDIAN_SPECOMENT.original.md` is a second full-length document, *named as authority*,
   differing from the canonical file, carrying no derived marker and no H.5 stamp. Tracking it
   on the branch whose entire purpose is establishing one root authority would trip this plan's
   own first stop condition harder than any generated projection could, and would contradict
   Appendix D item 8 and `PH-AUTH-001`'s "Git remains the historical source".
   `appendix_a.md` is a **third** copy of the traceability index, alongside the root's
   Appendix A and the new `governance/generated/index.md`.

   **Retention mechanism, corrected.** An earlier revision of this plan claimed these files were
   "retained by content hash" in `baseline-manifest.md` and reachable "through Git history at
   tag `v0.5-final-baseline`". Both claims were false and were caught by review:

   - `MERIDIAN_SPECOMENT.md` has **never been committed**. `git log --all --` on it is empty.
     The only specoment inside `v0.5-final-baseline` is a four-line test fixture.
   - `.meridian/authority/MERIDIAN_SPECOMENT.original.md` was **absent from the object
     database**; its only copy was the untracked working tree on one machine.
   - `baseline-manifest.md` recorded neither digest, and its `specoment_sha256` was a stale
     `475c91c8…` matching no file that exists.

   A content hash without the content is a receipt for something discarded. One `git clean -xdf`
   — routine in a `cargo clean`-heavy session, which repository policy encourages — would have
   destroyed the sole basis for auditing the 14 `unreviewed_specoment_amendments`, which
   `DEV-003` names as the residual exposure to be closed at `PH-AUTH-004`.

   **Remediated before this plan was revised**, because the risk was live regardless of the
   plan's fate. Both files, plus the canonical text, are written into the object database and
   made reachable by an orphan commit on no branch:

   ```text
   tag  v1-authority-preimage -> 0882050a57d8232bb3d23157c7e2796434bd57e2
   blob d5a4edff89f36ce34343905d71a4071bde625818  MERIDIAN_SPECOMENT.original.md
   blob 1d8412b772aadb41a1034ff67efba86cb367ca97  appendix_a.md
   blob 3bf7cec416ddb11b651f38484c0f5d25dfafb0dd  MERIDIAN_SPECOMENT.canonical.md
   ```

   This puts the bytes in Git and makes them gc-safe (verified: `git fsck --unreachable` does not
   list them) while creating **no tracked working-tree path named as authority**, which was the
   entire point of the exclusion. `baseline-manifest.md` is corrected to record all three real
   digests and both corrections are stated in it rather than silently applied.

   Committed: `.meridian/implementation/**`, `.meridian/tools/**`, `.meridian/legal/**`,
   `.meridian/authority/baseline-manifest.md`,
   `.meridian/authority/normative-language-audit.md`, `.meridian/HANDOFF.md`.
   Excluded, with a `.gitignore` entry naming the reason:
   `.meridian/authority/MERIDIAN_SPECOMENT.original.md`, `.meridian/authority/appendix_a.md`.

   Unrelated dirty changes on `PLANNING.md`, `specs/registry/evidence.json` and
   `specs/registry/work-packages.json` are preserved across the branch switch and not committed.
3. Add `is_under(path, "governance")` to `is_excluded_context_path` in `main.rs`. **This is a
   named deliverable with its own regression test, not an incidental edit.** It is mechanically
   required, and this was verified rather than assumed:
   `has_retired_reference` (`main.rs:470`) is a plain substring test including
   `lower.contains("v0.2")`; the generated index carries the A.4 provenance text
   *"spec-rewrite v0.22, v0.26, v0.53…"*, which contains that substring. The
   `migration_record` exemption at `main.rs:384` applies only to `docs/migrations/**` and to
   `SPEC_MIGRATION_AND_CONTRADICTIONS.md`, so `governance/generated/index.md` does not
   qualify for it. `cargo run -p meridian-spec -- check` therefore fails the moment that file
   exists unexcluded. Relocating the tree under `docs/migrations/` would earn the exemption
   but would misfile a traceability index as a migration record; the exclusion is the honest fix.
4. Port the attribution logic into new modules under `src/specoment/`. Preserve the rule that
   an identifier is owned by the heading that **declares** it, never by first mention, and
   that identifiers never appearing in a heading are reported undeclared. Fix defects 1-4.
   **Bound on `main.rs`: no more than ~20 added lines** — a `Command::Project { check }` arm
   and a dispatch call. Zero v1 logic in `main.rs`; the central-dispatch growth pattern is
   exactly what `CODEHEALTH-*` and `PH-AUTH-006` warn against.
5. Add `meridian-spec project` and `meridian-spec project --check`.
6. Generate, from the root file and nothing else:

   ```text
   governance/manifest.json                  stamp + per-file hashes
   governance/generated/index.md             traceability index
   governance/generated/identifiers.json     declared ID -> owning heading, line, references
   governance/generated/requirements.json    contracts with verbatim unnormalised labels
   governance/generated/phases.json          derived from the 99 prose phase cards
   governance/generated/research-gates.json  Appendix B plus research-labelled headings
   ```

7. Stamp every generated file with the four fields Appendix H.5 mandates:
   `canonical_path`, `canonical_sha256`, `generator_version`, `generated_at_source_checkpoint`.
8. Add reconciliation: regenerate to a temporary tree, compare byte-for-byte, name the first
   divergent file.

### Projection decisions taken under review

**`phases.json` derives from the prose cards, not from Appendix G.** Appendix G's own preamble
states *"If a serialization defect conflicts with a prose phase card, the prose phase card
wins until the registry is regenerated."* Checking in a copy of Appendix G would check in a
projection of a projection. Deriving from the 99 prose cards makes the root prose the single
source, and enables the reconciliation check below.

**Appendix G versus prose is a real, currently-failing reconciliation.** A field-level diff
across all 99 phases, normalising backticks and smart quotes and excluding a harness artefact
where a naive card parser bleeds a following top-level heading, found **20** genuinely
divergent fields:

| Field | Count | Phases |
|---|---|---|
| `gate` | 12 | every `PH-AI-001..012` — Appendix G adds *"; does not block main-engine 1.0"* |
| `implementation_scope` | 3 | `PH-AUTH-001`, `PH-AUTH-002`, `PH-AUTH-003` |
| `stop_conditions` | 1 | `PH-AUTH-002` |
| `user_visible_result` | 1 | `PH-AUTH-002` |
| `current_code_disposition` | 1 | `PH-AUTH-001` |
| `closure_evidence` | 1 | `PH-AUTH-002` |
| `explicit_exclusions` | 1 | `PH-AUTH-002` |

The distribution is itself the finding: outside the 12 uniform `PH-AI-*` gate strings, **all
eight remaining divergences fall in `PH-AUTH-001`, `PH-AUTH-002` or `PH-AUTH-003`** — with no
counterexample anywhere in the other 84 cards. The Epoch 0 cards were revised after Appendix G
was serialised and the fence was never regenerated.

An earlier count said 21 and included `PH-REL-008`. That was wrong: `PH-REL-008` is the last
card of Epoch 7, followed by a top-level heading, and one epoch-bleed artefact survived in
exactly the field the previous revision claimed to have corrected for. Terminating card bodies
at any `^#{1,2} ` removes it. The correction **strengthens** the conclusion by removing its only
apparent counterexample.

The sharpest instance is in `PH-AUTH-002`'s own card: prose `closure_evidence` reads
*"indexable exactly once; **headings,** status, links…"* and Appendix G drops **"headings,"**.
That is a dropped check, not formatting — and it is a dropped check in the very phase this
package closes.

This reconciliation therefore fails on first run, which makes it real must-fail-first evidence.
Divergences are **recorded as specoment defects for `PH-AUTH-003`**, never silently patched.

**The DAG test moves out of the exclusion list.** The previous revision excluded DAG checks as
`PH-AUTH-003` scope while simultaneously asserting acyclicity in an integration test. Resolved
in favour of keeping it: a phase registry that does not resolve is not a usable projection.
`PH-AUTH-003` still owns DAG validation *as an enforced governance gate*.

**`glossary.json` is cut.** It served no closure row, and "named subsystems and defined terms"
was unpinned enough that generating it risked producing interpretive content. Appendix C owns
the glossary and is already readable.

**Declaration versus reference is decided by an explicit, measured rule.** The previous
revision asserted that `PRODUCT-002`@17707 is a reference and tested for it, but never stated
the rule by which the code decides — leaving the implementer to invent one, with fragile
obvious candidates like first-line-wins.

The rule is derived from the corpus, not invented. Of all identifier occurrences in headings
across the body: **588 are backticked**, **111 are bare but heading-initial** — the 99 phase
cards plus the 12 Appendix E `WP-V1-*` work-package briefs at lines 30142-30219, and **exactly one is bare and mid-heading** — line 17707,
*"Ease-of-use condition inherited from PRODUCT-002"*. That one occurrence is also the only
identifier appearing in more than one heading at all.

> **A heading declares an identifier if the identifier is backticked in that heading, or if the
> heading text begins with it. A bare, non-initial identifier in a heading is a reference.
> A heading declaring a range (`FAM-001..005`) declares its members weakly; a later heading
> declaring a single member supersedes that weak declaration and is not a re-declaration.**

This resolves `PRODUCT-002` mechanically and yields 0 multiply-declared with no special case.
An independent implementation of this rule, written separately from the port, yields **736
declared** — 685 strong single-identifier declarations plus 51 members declared only by a
range heading. That is the same total the four fidelity invariants predict, arrived at by a
different code path.

**Consequence, stated rather than left emergent:** the 12 Appendix E `WP-V1-*` briefs are
heading-initial, so under this rule `WP-V1-RESET-002` — this package — is itself a declared
canonical identifier owned by an Appendix E heading. That is correct and intended.
`WP-V1-*` identifiers are **canonical specoment content**, not implementation bookkeeping;
`state.json` tracks their *progress*, which is the derived state `IMPL-STATE-001` describes,
and must not restate their definitions. `PH-AUTH-003`'s registries therefore include them.

**Guard against the rule's one dangerous limb.** The rule has three limbs of very different
support. *Bare-and-mid ⇒ reference* is fitted to a single case, but only ever **demotes**, and
its failure mode — under-declaring — surfaces immediately as an invariant-2 mismatch.
*Backticked ⇒ declares* is fitted to 588 occurrences with no counterexample, but its failure
mode is **silently promoting a reference to a declaration**, manufacturing a false duplicate.
That matters because `PH-AUTH-003` and `PH-AUTH-004` are the migration and cutover phases, and
headings of the form *"retired X superseded by `Y-001`"* are their native output. The break is
one phase away, not hypothetical.

Therefore a fifth invariant: **no heading declares a single identifier already declared by an
earlier heading.** Verified against the current corpus: **0 hits.** A first formulation that
ignored range precedence produced 31 false hits from the `ED-AOT`, `SAVE-DER`, `PRJ-DER`,
`MOD-DER`, `RELEASE-SUP` and `SRV-017..022` families, where a range heading is followed by
per-member headings — which is why the range clause is part of the rule above rather than an
implementation detail.

**`requirements.json` carries verbatim, unnormalised labels.** §0.3 defines six maturity
labels; headings use 29 distinct suffix phrases, 24 of which §0.3 does not define. Normalising
them is interpretation, is `PH-AUTH-003`'s explicit "status axes" scope, and is exactly how a
projection acquires meaning absent from the root file. The field is an opaque string.

**`research-gates.json` keys on heading text plus line, not on ID.** Several gate headings
carry no identifier at all (*"Rust-authored shaders — *Research gate*"*, and two headings both
titled *"Open implementation research"*). Appendix D requirement 2 maps each *included stable
ID* to one canonical heading; ID-less gates are recorded with heading text and line as their
identity, and are explicitly marked as carrying no stable ID rather than being assigned one.

**The A.4 provenance table is parsed from the root file, not ported as a source literal.**
Those eleven rows are owner-level provenance rulings. Compiling them into Rust would put
governance decisions where changing one needs a rebuild, and would make the generator emit
claims not derivable from its declared input — breaking Appendix D requirement 4 in the subtle
direction. The generator reads Appendix A's A.4 table from the specoment.

### Dependency decision

Two capabilities are needed that the workspace does not provide directly. **The governing
contract is `LEGAL-006`, not `LEGAL-005`**, and the previous revision of this plan got that
backwards.

`LEGAL-006 — *Normative*` (line 15824) states: *"Meridian does **not** pursue dependency
elimination as a goal in itself. A mature dependency should be used when it materially saves
development/maintenance work or provides better correctness, portability, security,
performance, standards compliance, tooling, interoperability, or user experience than a
Meridian rewrite can reasonably justify."* It closes: *"'We could write it ourselves' is not
sufficient justification for replacement."*

The previous revision argued from `LEGAL-005`'s provenance burden — that is, it used
"avoiding paperwork" as a reason not to take a dependency. That is precisely the posture
`LEGAL-006` forbids, and it would have set a precedent recurring across 86 remaining phases.
The argument is withdrawn. Each capability is decided separately below, on `LEGAL-006`'s own
stated grounds.

**Regex — hand-written, on correctness grounds.** `regex` 1.13.0 is already in `Cargo.lock`,
pulled by `jsonschema`, which `meridian-spec` already depends on. Marginal compiled cost of
promoting it to a direct dependency is **zero**. So the licensing argument is not merely
one-sided, it is void: the transitive obligation exists today and declining to promote the
crate discharges nothing.

The decision stands on `LEGAL-006`'s **API fit** ground, which it names explicitly among the
legitimate reasons to prefer ownership.

A weaker argument was tried first and is withdrawn: that hand-writing is *more correct*
because `(?![0-9-])` was misread twice. It is not. The defect was a wrong character class in a
hand-written pattern, and the fix is one class — `(?![0-9A-Za-z-])`. Hand-writing confers
auditability, not correctness, and `LEGAL-006` closes with *"'We could write it ourselves' is
not sufficient justification for replacement."* Read straight, zero marginal cost plus ~120
lines saved tilts `LEGAL-006` **toward** promoting `regex`.

What survives is API fit. This is not a pattern-matching problem; it is a context-sensitive
tokenizer. The same character sequence means different things depending on whether it sits in
a heading or a body line, whether it is backticked, bare-initial or bare-mid-heading, whether
it expands as a range, and whether a trailing letter is part of the identity. Two patterns
plus surrounding conditional logic is precisely the arrangement that lost `NETPROJ-006A..D`
and `SCM-010A`. A scanner that returns a typed token carrying its context makes each of those
distinctions a named, separately-tested predicate rather than an interaction between a
lookahead and an `if`.

**SHA-256 — hand-written, on a different and quantitative ground.** The two decisions are
**not** symmetric and must not share a rationale. Unlike `regex`, `sha2` is absent from
`Cargo.lock` entirely, along with `digest`, `generic-array` and `typenum`. Adding it pulls
roughly 5-7 new crates into the graph for one digest of one file. That marginal cost — not
provenance paperwork — is the argument.

Appendix H.5 names the field `canonical_sha256`, and the digest is already published in
`state.json`, the baseline manifest and three plan records; switching to blake3 would orphan
all of them.

*Why hand-rolling is safe here, stated rather than assumed.* The usual prohibition targets
secret-dependent code — MACs, signatures, key derivation — where a defect is exploitable and
timing is observable. This digest authenticates nothing. It answers "was this projection
generated from the current specoment?" There is no secret, no adversary and no timing channel.

More importantly, **the digest is not the enforcement mechanism.** `project --check`
regenerates every projection and compares byte-for-byte; the stamp is a fast staleness hint in
front of that. A defective digest therefore fails **closed** — a noisy false mismatch against
the independently published `shasum -a 256` value — and cannot fail open in the path that
actually gates correctness.

The one genuine fail-open mode is a padding or length-encoding bug that drops trailing bytes,
which would make edits near the end of a 33,175-line file invisible to a digest comparison.
The tests below close it specifically.

**Considered and rejected:** using the already-registered `blake3` for the per-file
`manifest.json` hashes and hand-rolled SHA-256 only for the single `canonical_sha256` field.
It would shrink the hand-rolled blast radius to one independently cross-checked value.
Rejected because it puts two hash algorithms in one manifest for no reader benefit, and
because `blake3`'s registration is v0.5 authority — an input, not an override, under
`IMPL-BOOTSTRAP-001` — so it would need its own v1 provenance record regardless.

**Recorded as a `PH-AUTH-003` finding, not fixed here:** `LEGAL-005` requires machine-readable
provenance including a transitive-license summary for *every* third-party dependency. The
repository has 494 locked packages and no such records. That obligation is pre-existing,
currently unmet, and independent of this package. It is logged rather than silently inherited.

## Explicit exclusions

- No `specs/legacy/` tree. Git history plus tag `v0.5-final-baseline` preserve the old suite.
- No edits to `specs/`, `schemas/`, `AGENTS.md`, `PLANNING.md`, `DCO.md`, `README.md`,
  `VISION.md`, `CONTRIBUTING.md`. Those belong to `PH-AUTH-004`.
- No edits to the specoment body. Defects found are recorded, never silently patched.
- No decomposition of the existing `main.rs` beyond the ~20-line dispatch bound.
- No status-axis normalisation, no link validation, no enforced governance gates. `PH-AUTH-003`.
- No new third-party dependency.
- No old-ID-to-new-ID bureaucratic map.
- No commit to `main`. No push to any remote.

## Compatibility / migration / authority effects

`main` is unchanged, so v0.5 authority is uninterrupted. On the reset branch the root file
wins over every projection by construction. The `governance/` exclusion added in step 3 joins
the `.meridian` and `MERIDIAN_SPECOMENT.md` exclusions from `WP-V1-BASE-002`; all three retire
with the tool at `PH-AUTH-004`.

`governance/` keeps its name through `PH-AUTH-004` and is not renamed at cutover; a rename
would be churn against a path that CI, the validator and the manifest all reference.

## Accessibility / security / privacy / provenance / disabled-cost effects

- **Accessibility:** none. No runtime or UI surface. `index.md` is plain Markdown with real
  headings.
- **Security:** reads one file inside the repository, writes inside the repository. No network,
  no shell interpolation, no untrusted input, no new dependency. The input is a bounded
  in-memory string of known size. No claim is made that hand-written scanning is safer than
  the `regex` crate here — `regex` is finite-automata based and guarantees linear time by
  construction, which a hand-written scanner does not; on that axis this is at best parity,
  and the decision rests on API fit rather than on safety.
- **Privacy:** the specoment is engine architecture only; the generator does not read `game/`.
- **Provenance:** strengthened. The index moves from an untracked Python script pasted in by
  hand to a tracked, tested, versioned generator whose output names its source, hash, version
  and source checkpoint. The A.4 provenance rulings stay in the authority rather than in code.
- **Disabled cost:** new modules compile into the `meridian-spec` binary only. No engine or
  editor crate gains a dependency; no runtime path is touched.

## Tests and evidence

**Must fail first, and demonstrably do:**

- `declaring_heading_absent_from_own_refs` — defect 1. Asserts the **property** for every
  entry, not a violation count. (The reference currently violates it 44 times; that figure is
  diagnosis, not a test input.)
- `no_args_prints_usage_not_panic` — defect 2.
- `indexed_count_equals_declared_total` — defect 3. This is the invariant whose absence let a
  31-identifier gap hide inside a number that read as clean.
- `netproj_006a_is_distinct_from_netproj_006` — defect 4, shape A (collapse across headings).
- `scm_010a_and_scm_010_both_declared_by_one_heading` — defect 4, shape B. Distinct because
  the shape-A test passes while shape B still silently overwrites.
- `no_heading_redeclares_an_already_declared_identifier` — invariant 5. Passes today with 0
  hits and fails with file, line and identifier the day a migration heading backticks an
  existing contract, forcing an explicit decision instead of a silent miscount.
- `every_letter_suffixed_heading_identifier_is_indexed` — the **derived invariant** covering
  both shapes and any future sixth case, rather than an enumerated list that has now been
  incomplete twice.
- `appendix_g_matches_prose_derived_phases` — asserts the divergence **set**, never a count:
  *every reported divergence is either a `PH-AI-*` gate string or a card in `PH-AUTH-001`,
  `PH-AUTH-002` or `PH-AUTH-003`.* This is falsifiable, survives a specoment edit that changes
  the tally, and encodes the actual structural finding rather than a snapshot number.
  It fails today, which is the point.
- `appendix_a_matches_generated_index` — the root's Appendix A is simultaneously a generator
  **input** (A.4 is parsed from it) and a stale copy of the generator's **output**. Nothing
  else closes that loop: if A.4's dispositions drift from the emitted index, no check notices.
  Expected to fail today, which makes it further free must-fail-first evidence.

**Attribution unit tests:**

- declaration beats an earlier prose mention;
- an identifier never in a heading is reported undeclared, not attributed to first mention;
- `FAM-001..005` expands to five members;
- a single-id heading beats a range heading for the same member;
- `PH-AI-005` does not yield `AI-005`; `AI-0051` does not yield `AI-005`;
- a referencing heading (*"inherited from PRODUCT-002"*) is not treated as a declaration;
- retired-v0.5 identifiers are segregated from declared ones.

**Infrastructure unit tests:**

- SHA-256, six named cases rather than "the published vectors": the empty input; `"abc"`;
  the 448-bit multi-block vector; **lengths 55, 56, 63, 64 and 65** — the padding cliff, where
  56..63 forces a second block and where a length-encoding bug hides; the specoment itself
  against the `shasum -a 256` digest recorded in `state.json`; and a **differential property
  test** over random inputs of length 0..1000 asserting that mutating any single byte,
  **including the last**, changes the digest. The last of these is what closes the only
  genuine fail-open mode — dropped trailing bytes;
- both hand-written scanners against the reference generator's output as oracle, for every
  case where the reference is not defective.

**Integration:**

- generate twice into separate temporary directories, assert byte-identical;
- `project --check` passes on fresh output;
- `project --check` fails and names the file after a one-byte edit;
- `project --check` fails when the source changed but projections were not regenerated;
- `phases.json` carries 87 main plus 12 optional phases, every `depends_on` resolves, acyclic;
- `governance/` exclusion regression: `spec check` stays green with `index.md` present, and a
  genuine `docs/` file carrying a retired reference is still reported.

**Fidelity rule, as a derived invariant rather than an enumerated constant.** The previous
revision froze "735 declared" and "the four enumerated defects". Both were wrong — `SCM-010A`
made it five defects and 736 — and a correct port would have tripped its own stop rule. A
hard-coded enumeration has now been incomplete twice, so it is replaced by invariants that
hold regardless of how many cases exist:

1. `indexed_count == declared_total` — no identifier is counted without being emitted;
2. every identifier declared by any heading appears in the index exactly once, where
   **declared** is defined independently of the attribution module (see the declaration rule
   below) and the invariant's oracle is a separate naive scan over heading lines. Without that
   independence the invariant would restate the implementation and could not fail;
3. every letter-suffixed heading identifier appears in the index as its own identity;
4. no identifier's declaring heading appears in its own reference set;
5. no heading declares a single identifier already declared by an earlier heading — the guard
   against the declaration rule's one dangerous limb, currently 0 hits.

Current expectation from these invariants is **736 declared, 736 indexed, 0 multiply-declared,
0 undeclared**, and that number is a *prediction of the invariants*, not an input to them. If
the port yields a different total, the invariants decide whether it is a port defect or a
sixth reference defect — the number does not.

Gates: `cargo test -p meridian-spec`, `cargo fmt --all -- --check`,
`cargo clippy -p meridian-spec --all-targets -- -D warnings`,
`cargo run -p meridian-spec -- check`, `cargo metadata --locked`, `git diff --check`.

Red and green logs for every must-fail-first test under `.meridian/implementation/evidence/`.

**Deferred evidence, recorded not omitted:** Appendix D requirement 7 — *"fail CI when stale
in a way that could mislead implementation"* — is **Deferred to `PH-AUTH-004`**. This package
excludes CI edits and the branch is unpushed, so `project --check` is wired into
`meridian-spec check` locally but not into any workflow.

## Failure injection and recovery

Induced and observed, not asserted:

- corrupt one projection byte; `--check` names that file;
- truncate the specoment mid-heading; generation fails with a line number, emits no partial index;
- introduce a second declaring heading for one identifier; reported as multiply-declared;
- point the generator at a nonexistent path; diagnostic, not panic;
- run with `governance/` absent; created rather than erroring;
- remove the A.4 table from the specoment; generation reports the missing provenance input
  rather than silently emitting an index with no dispositions.

## Research candidates and selection metrics

None. Output format, attribution rule, stamp shape and dependency posture are all decided.

## LOC estimate

Revised upward from the previous revision after the dependency decision and the four defect
fixes. Scope signal, not a quota.

| Area | Added | Removed |
|---|---|---|
| Production — `src/specoment/*` | ~800 | ~0 |
| Production — `main.rs` dispatch | ~20 | ~0 |
| Tests and fixtures | ~450 | ~0 |
| Generated — `governance/**` | ~4,500 | 0 |
| Tracked authority — `MERIDIAN_SPECOMENT.md` | 33,175 | 0 |
| Tracked bookkeeping — `.meridian/**` after exclusions | order ~5k, measured at commit | 0 |

These rows have been wrong twice and are now measured directly rather than derived.
A first estimate collapsed them into "~35,000". A second corrected that to "~913 committed"
by subtracting the two excluded files from 34,465 — but 34,465 was `wc -l` of
`.meridian/authority/` **only**, not the whole tree, so the subtraction was meaningless and
understated the result by roughly 5x.

The row is now deliberately **not** a constant. It counts a tree that contains this plan file,
which is itself ~800 lines and grows with every review round, so any figure stated here is
stale before it is committed. The lesson from four rounds is exact: **every figure expressed as
an invariant has held; every figure derived once and restated has been wrong** — 735/736,
~35,000/67,640, ~913/4,993/5,088, 21/20. The actual line count is measured once against the
real commit and recorded in the completion record.

The specoment is 33,175 lines. Excluded: `MERIDIAN_SPECOMENT.original.md` (32,804) and
`appendix_a.md` (748), both preserved as Git objects under tag `v1-authority-preimage`.

Of the ~820 production lines, ~210 (26%) are general infrastructure rather than the declared
seam — SHA-256 and the two scanners. That is acceptable only because it is isolated in
`src/specoment/sha256.rs` and `src/specoment/scan.rs` with their own test modules, and it is
bounded by the converse stop trigger below.

## Stop / rollback rule

Stop if:

- a projection could silently override the root file, or carries normative prose absent from it;
- decomposition changes the meaning of any canonical decision;
- any canonical identifier becomes unreachable from the index;
- the indexed count does not equal the declared total;
- generation needs prose duplication to determine authority;
- any of the four fidelity invariants above fails, or a total diverges from them in a way that
  cannot be traced to a **demonstrated** reference defect. Stated as an invariant deliberately:
  the enumerated-constant form of this rule was wrong twice, and in its previous form a
  *correct* port would have stopped itself;
- `main.rs` growth exceeds the ~20-line dispatch bound;
- the port would require editing the specoment body;
- a new third-party dependency turns out to be unavoidable — that is an owner decision under
  `LEGAL-006`, not an implementation choice;
- **the converse:** hand-written infrastructure materially exceeds ~210 lines
  (`sha256.rs` plus `scan.rs`). The dependency trade then re-opens under `LEGAL-006` rather
  than the estimate being quietly absorbed. `LEGAL-006` weighs a rewrite against what a
  dependency saves; that weighing is invalid if the rewrite's true cost is discovered later
  and never re-examined.

Rollback is deleting the branch commits. `main` is never touched, so rollback cannot leave
source authority ambiguous.

## Independent Review

- Verdict: accept
- Reviewer: fresh isolated same-model context (Opus 5), no access to the drafting agent's
  reasoning or transcript. Qualifies under `IMPL-WP-001` "a fresh isolated session or
  context of the same model qualifies".
- Reviewed: 2026-08-20, four rounds, against plan revision 4 at source `ddbacd3`.
- Accepted with five carry-in items, none of which changes approach, authority or risk
  boundary. Two are required and verified in the completion record: the backticked-reference
  guard, and the `OD-007`/`DEV-006` governance records.
- Method: reviewer independently re-verified every factual claim in the plan rather than
  accepting it, and ran the reference generator itself.

### Verified correct by the reviewer

Specoment sha256 and 33,175 line count; untracked status; `main` = `v1-authority-reset` =
`v0.5-final-baseline` = `ddbacd3` with no branch-local commits; `main.rs` at 2,188 lines and
`cli.rs` at 417; `specs/registry` holding 22 registries; both named Python defects
reproducing; `PH-AI-005` correctly not yielding `AI-005`; and no CRLF or `.gitattributes`
hazard for the byte-identical commit claim.

### Blocking findings — all three accepted, two after independent confirmation

**B1. The fidelity target "731 declared" enshrines a defect.** Confirmed by direct
measurement: the totals line reports 731, A.1 emits 700, delta 31. Cause is `all_ids` being
computed before `fam_owner` is merged into `owner`. The 31 lost identifiers are five whole
families — `AI-POLICY-001..008`, `NORM-MIG-001..012`, `CODEHEALTH-001..004`, `OPEN-001..004`,
`FWK-001..003`. Freezing 731 as a port-fidelity target would have reproduced the defect and
called it success.

**B2. Both reported duplicates are artifacts, and four identifiers are erased.**
Confirmed. `BARE_ID_RE`'s trailing guard `(?![0-9-])` does not exclude letters, so
`NETPROJ-006A..D` — four distinct `— *Normative*` contracts — collapse onto `NETPROJ-006`
and vanish from the index. `PRODUCT-002`'s second "declaration" at line 17707 is a heading
that *references* the contract. The plan's stop rule would have fired the moment the port
became correct.

**B3. `phases.json` sourcing.** Accepted in substance, with its stated premise corrected
below. Appendix G's own preamble declares itself subordinate to the prose cards, so a
checked-in copy of it is a projection of a projection.

### Finding disputed, with evidence

The reviewer stated that Appendix G "contains exactly 87 phase objects and zero `PH-AI-*`
entries", and called this the plan's most consequential factual error.

**This is incorrect, and the plan was right.** Appendix G's top-level keys are
`schema_version`, `roadmap_version`, `reviewed_repository`, `reviewed_commit`, `phase_count`,
`epochs`, `phases`, `optional_programs`, `canonical_authority`, `specoment_version`. The 12
`PH-AI-*` phases are present under `optional_programs.temper_tally_tqf.phases`, complete and
matching the 12 prose cards by ID. The reviewer read `phases` alone. The 87 + 12 test the
plan specifies can pass, and does.

Recorded rather than quietly dropped, because this specific narrow reading of Appendix G has
now produced a wrong conclusion twice in this program and is worth naming as a recurring trap.

**However, the reviewer's underlying structural point survives its wrong premise, and was
independently verified.** Appendix G and the prose cards do diverge in wording. A field-level
diff across all 99 phases, after normalising backticks and smart quotes and fixing a harness
artefact where a naive card parser bleeds the following `# Epoch N` heading, found **21**
genuinely divergent fields: `gate` (12), `implementation_scope` (3), `stop_conditions` (2),
and one each in `user_visible_result`, `current_code_disposition`, `closure_evidence` and
`explicit_exclusions`. A first, less careful count said 24; the three-field difference was
backtick formatting and is not a divergence. Outside the 12 uniform `PH-AI-*` gate strings,
every remaining divergence falls in `PH-AUTH-001/002/003` or `PH-REL-008`. Two examples, both
in `PH-AUTH-002`'s own card:

- prose `closure_evidence` reads *"indexable exactly once; **headings,** status, links…"*;
  Appendix G drops **"headings,"** — a dropped check, not a formatting difference.
- all 12 `PH-AI-*` gates read *"Optional first-party AI program"* in prose and
  *"Optional first-party AI program; does not block main-engine 1.0"* in Appendix G.

So the reconciliation check the reviewer asks for is worth building, and it genuinely fails
today. That makes it real must-fail-first evidence rather than a restatement.

### Disposition — round 1

All six accept conditions adopted. Findings 6, 7 and 10-13 also adopted.

---

## Independent Review — round 2

- Verdict: **revise**
- Same fresh isolated same-model reviewer, re-reading the rewritten plan in full, plus the
  dependency decision which had not previously existed.

**The reviewer retracted its own round-1 error unprompted**, confirming that Appendix G does
carry all 12 `PH-AI-*` phases under `optional_programs.temper_tally_tqf.phases`, and that its
round-1 structural point "survived only by accident of the plan being right for a different
reason than I gave". Recorded because a review process that cannot correct itself is not a
control.

### Blocking findings — both confirmed by independent measurement, both accepted

**B1. `SCM-010A` is a fifth collapsed identifier, of a different shape.** Confirmed. Line
14869 declares `SCM-010` and `SCM-010A` in one heading; equal line numbers defeat the
duplicate guard, so `SCM-010A` is silently overwritten rather than reported. The planned
`netproj_006a_*` test would have passed while this case still lost an identifier. Corrected
total is **736**, and — decisively — the previous stop rule would have fired on a *correct*
port. An exhaustive scan confirms exactly five letter-suffixed identifiers exist, all five
heading-declared, so 736 is complete; but the fidelity rule is now a derived invariant rather
than a constant, because a hard-coded enumeration has been incomplete twice.

**B2. Committing `.meridian/` wholesale would install a second full-length specoment.**
Confirmed: `.meridian/authority/MERIDIAN_SPECOMENT.original.md` is 1.7 MB and 32,804 lines,
is named as authority, carries no derived marker or H.5 stamp, and would trip this plan's own
first stop condition harder than any generated projection could.
`.meridian/authority/appendix_a.md` is a third copy of the traceability index. The round-1
finding asked the plan to state a position; the round-1 revision adopted a blanket "commit it"
without inspecting the contents. That was the correct criticism to make.

### Finding accepted against my own reasoning

**B3. The dependency rationale was one-sided.** `LEGAL-006` (line 15824) states that Meridian
*"does **not** pursue dependency elimination as a goal in itself"* and closes *"'We could
write it ourselves' is not sufficient justification for replacement."* The previous revision
cited `LEGAL-005`, which creates the provenance burden, and never cited `LEGAL-006`, which
forbids the posture. Arguing from the paperwork cost of a dependency in order to avoid it is
exactly what `LEGAL-006` rules out, and it would have set a precedent across 86 remaining
phases. The argument is withdrawn and both decisions re-derived on `LEGAL-006`'s own grounds —
correctness for the scanner, marginal graph cost for SHA-256. The reviewer's summary is
adopted verbatim as the standard: *"reasoning that survives only by reaching the right answer
will not survive the next 86 phases."*

The reviewer also noted the licensing argument was self-defeating for `regex`: the transitive
provenance obligation already exists via `jsonschema`, so declining to promote the crate
discharges nothing. Recorded as a `PH-AUTH-003` finding.

### Disposition — round 2

All five accept conditions adopted, plus recommendations 8-11.

---

## Independent Review — round 3

- Verdict: **revise**
- One blocking finding, and it was a factual error in the plan about Git state rather than a
  design flaw.

### Blocking finding — confirmed, and remediated before the plan was touched

**The retention mechanism did not exist.** The round-2 revision claimed the excluded authority
files were "retained by content hash" in `baseline-manifest.md` and reachable "through Git
history at tag `v0.5-final-baseline`". Every part of that was false, and all of it was
verified directly:

- `git log --all -- MERIDIAN_SPECOMENT.md` is **empty**; the root specoment has never been
  committed. The only specoment in the tag tree is a four-line test fixture.
- `git cat-file -e $(git hash-object .meridian/authority/MERIDIAN_SPECOMENT.original.md)`
  reports the blob **absent from the object database**. Its only copy was the untracked
  working tree.
- `baseline-manifest.md` recorded **neither** excluded digest, and its `specoment_sha256`
  was `475c91c8…` — a stale third revision matching no file that exists anywhere.

So the plan proposed to exclude, on the strength of a retention guarantee that was fiction,
the only copy of the document needed to audit 14 amendments that `DEV-003` names as open
residual exposure. A single `git clean -xdf` would have ended that audit trail.

Remediated immediately rather than at implementation time, because the exposure was live
independent of the plan: both files plus the canonical text are now blobs in the object
database, reachable via orphan tag `v1-authority-preimage`, gc-safe and verified by
`git fsck`. `baseline-manifest.md` — which is `PH-AUTH-001` closure evidence — is corrected,
with both corrections stated in it rather than silently applied.

### Findings accepted against the plan's own reasoning

**The regex rationale was still backwards, on the third pass.** The plan argued hand-writing
is *more correct* because the lookahead guard was misread twice. It is not: the defect was one
wrong character class and the fix is one class. Hand-writing confers auditability, not
correctness, and `LEGAL-006` closes with *"'We could write it ourselves' is not sufficient
justification for replacement."* Read straight, `LEGAL-006` tilts toward promoting `regex`.
Re-derived on **API fit**, which `LEGAL-006` names explicitly: this is a context-sensitive
tokenizer, not a matching problem. The security bullet's "no backtracking" claim is withdrawn —
`regex` guarantees linear time by construction and a hand-written scanner does not.

**Invariant 2 was circular.** "Every identifier declared by any heading appears in the index"
left `declared` undefined; if `declared` means whatever the attribution code computes, the
invariant restates the implementation and cannot fail. The plan asserted `PRODUCT-002` is a
reference and tested for it but never stated the deciding rule. Now stated and measured
against the corpus: 588 backticked, 111 bare heading-initial, exactly **one** bare mid-heading.
Invariant 2's oracle is an independent naive heading scan, not the attribution module.

**Every unverified constant in the plan was wrong, three rounds running** — 735/736,
~35,000/67,640, ~913/4,993, 21/20. The identifier fidelity rule had been converted to
invariants; the same pattern survived everywhere else. No test now asserts a count. The
Appendix G check asserts the divergence **set** instead.

**`PH-REL-008` was a phantom divergence.** One epoch-bleed artefact survived in exactly the
field the round-2 revision claimed to have corrected for. Corrected to 20, and the correction
removes the only apparent counterexample to the Epoch 0 conclusion.

### Confirmed, not changed

The reviewer independently re-ran the letter-suffix scan with a broader pattern (1-3 trailing
letters, any case) and found no sixth identifier, corroborating that 736 is complete. It also
ruled the package should **not** be split, and that 598 lines of plan is not evidence of
scope since ~135 are the review record `IMPL-WP-001` requires retaining. The seam statement is
tightened to one clause on its recommendation.

### Disposition — round 3

All five accept conditions adopted; findings 6 and 7 adopted; seam recommendation adopted.

---

## Independent Review — round 4 — ACCEPTED

- Verdict: **accept**, conditional on five carry-in items, none of which changes approach,
  authority or risk boundary.
- The reviewer independently verified the remediation: annotated tag → orphan commit
  `0882050a` with `parents=[]`, on no branch, carrying all three blobs at
  1,679,200 / 116,110 / 1,758,939 bytes, hashing to `d7329e23…` and `782d3110…` matching disk,
  and absent from `git fsck --unreachable`.

### Why the risky parts were judged acceptable

Recorded because that is what an `accept` is for. The hand-rolled SHA-256 was accepted because
the argument was finally made honestly — no secret, no adversary, no timing channel; the digest
is a staleness hint in front of `project --check`'s byte-for-byte regeneration, so a defect
fails closed; and the one fail-open mode is closed by a named differential test rather than a
vague appeal to "the published vectors". The hand-written scanner was accepted because the
correctness claim was **withdrawn rather than softened**, `LEGAL-006`'s closing sentence was
quoted against the plan's own position, and the decision was re-derived on API fit.

### Carry-in items

1. **Backticked-reference guard — required.** Added as invariant 5 with its own test. The
   reviewer stated it "passes today with zero hits"; that was wrong as first formulated —
   it produced **31** hits from range-then-member families (`ED-AOT`, `SAVE-DER`, `PRJ-DER`,
   `MOD-DER`, `RELEASE-SUP`, `SRV-017..022`). Refined with range precedence it is 0 hits, and
   the refinement is now part of the stated declaration rule rather than buried in code.
2. **`OD-007` — required.** The preimage tag is local-only; annotated tags are not carried by a
   default `git push` and there is no push authorization. Three options recorded, none chosen
   by the agent. Not a blocker.
3. **`DEV-006` — required.** Amending closed-phase evidence is exactly the "stale evidence"
   class `PH-AUTH-003` is chartered to detect, and `state.json` did not disclose that it
   happened.
4. **LOC row** — de-constantised, since the row counts a tree containing this plan file.
5. **`WP-V1-*` canonicality** — stated explicitly rather than left emergent.

### One reviewer argument rejected, and the plan's own reasoning corrected with it

The plan had defended `PH-AUTH-001` on the ground that "the specoment was not part of what
`PH-AUTH-001` froze". **That defence is wrong and is withdrawn.** The phase card's
implementation scope explicitly requires a manifest naming *"…known unsupported rows, **and
specoment hash**."* The hash was in scope and the manifest named a wrong one.

`PH-AUTH-001` is nevertheless not reopened, on two sounder grounds:

- its closure row is *"every baseline artifact is hash-addressed"*; the frozen SHA `ddbacd3`,
  the tree hash and the `Cargo.lock` digest were all independently correct, so the frozen
  **state** was never in doubt — one cross-reference field was mis-addressed;
- the specoment is not an artifact *of the v0.5 baseline*. It is v1 staged authority, untracked
  at freeze time, and explicitly excluded from the v0.5 validator by `WP-V1-BASE-002`. A stale
  cross-reference, corrected in place with the correction stated, is proportionate.

Recorded as `DEV-006` so a reader of `state.json` can see that closed-phase evidence was
amended, rather than discovering it later through `PH-AUTH-003`'s own stale-evidence check.

## Completion record

- Completed: 2026-08-20
- Branch: `v1-authority-reset`, four commits, `main` untouched
- Source checkpoint: `15f60d1`

### Commits

| Commit | Content |
|---|---|
| `f3be604` | Install the specoment as tracked root authority; narrowed `.meridian/` scope |
| `4e6bb2e` | Exclude generated projections from v0.5 governance scope |
| `deeea6b` | `src/specoment/` port and the generated projections |
| `15f60d1` | Deterministic stamps; `project --check` fails closed |

### Result

736 declared, 736 indexed, 0 multiply-declared, 1 undeclared, 1 retired-v0.5, 117 families.

The total was **predicted from the fidelity invariants before the port existed** and is
reproduced by an independent implementation. That agreement is the evidence; a matched
constant would not have been.

### Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | **Pass** — 79 suites, exit 0 |
| `cargo test -p meridian-spec` | Pass — 48 tests, from 39 |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo fmt --all -- --check` | Pass |
| `cargo run -p meridian-spec -- check` | Pass |
| `cargo run -p meridian-spec -- project --check` | Pass, and fails closed on tamper |
| `cargo metadata --locked` | Pass |
| `git diff --check` | Pass |

### Failure and recovery paths induced and observed

- one-line tamper to a projection → exit **1**, `stale-projection`, file named;
- regeneration twice → byte-identical;
- generator with no specoment → diagnostic naming the path, no panic;
- `governance/` exclusion → `spec check` green with the index present, while a genuine
  `docs/` file carrying a retired reference is still reported.

### Actual versus estimate

| Area | Estimated | Actual |
|---|---|---|
| Production | ~820 | **829** |
| `main.rs` delta | ≤ ~20 | **11** |
| Tests and fixtures | ~450 | ~480 |
| Generated | ~4,500 | 7,976 |

The production estimate held to nine lines. The generated figure was low because the index
carries a reference list per identifier.

**The converse stop trigger fired.** Hand-written infrastructure reached 290 lines against a
~210 bound. The trade was re-examined under `LEGAL-006` rather than absorbed: SHA-256 holds
unchanged, since the argument was the five-to-seven-crate graph cost and not line count; the
scanner holds, but the margin is narrower than the plan claimed — realistically ~90 lines
with `regex` against 138 without. Disclosed in the evidence rather than buried.

### Two defects found after the first commit

The first commit passed six gates and still carried two defects, both caught by
`cargo test --workspace`:

- the H.5 stamp recorded repository HEAD, so every projection was stale the moment it was
  committed and `project --check` could never pass;
- `project --check` printed staleness and returned success — a fail-open that would have let
  CI go green on projections misrepresenting the authority.

Both fixed in `15f60d1`. The concrete lesson is the plan's own: a check that cannot fail is
not evidence, and targeted gates are not a substitute for proportional ones.

### Limitations and honest status

**`PH-AUTH-002` does not close.** Two rows remain open:

| Closure row | Status |
|---|---|
| Every canonical identifier is indexable exactly once | **Fail** — `RG-TOR-001` has no owning contract (`SD-006`, `OD-008`) |
| No accepted decision is lost or duplicated | **Inconclusive** — `OD-001`, `OD-002` unresolved without the master ledger |
| Headings, status, links, deferred/research markers pass | Partial — link and status validation is `PH-AUTH-003` |
| Projection hashes pass | Pass |
| Zero-unmapped checks pass | Pass for the index; enforcement as a gate is `PH-AUTH-003` |

`SD-006` was found during implementation, not by any of the four review rounds. The
reference generator classified retired-v0.5 identifiers by bare prefix including `RG`, and
`RG-TOR-001` — the only `RG-*` identifier in the document, cited in a live v1 section as an
open research gate — was absorbed into a category exempt from the undeclared count. That is
what made "0 undeclared" read as true across this entire package's planning.

Deferred and recorded, not omitted: Appendix D requirement 7 (fail CI when misleadingly
stale) is **Deferred to `PH-AUTH-004`**; `project --check` is wired locally but no workflow
runs it, because this package excludes CI edits and the branch is unpushed.

### Carry-in items from review round 4

| Item | Status |
|---|---|
| Backticked-reference guard (invariant 5) | Done — implemented, tested, 0 hits |
| `OD-007` / `DEV-006` recorded | Done |
| `WP-V1-*` canonicality stated | Done |
| LOC row de-constantised | Done — measured here instead |
| Review header tracks the current round | Done |
