# PH-AUTH-003 — diagnosis carried forward from PH-AUTH-002

**This is not a plan record.** It is verified diagnosis gathered while `WP-V1-RESET-002` was
blocked on review, parked here so `WP-V1-GOV-001` starts from measurement rather than
assumption. The plan itself cannot reach Definition of Ready until `PH-AUTH-002` lands, because
`IMPL-WP-002` item 2 requires inspecting the *current* source and the projections do not exist yet.

Gathered at `ddbacd3`. Every figure below was measured, not estimated.

## 1. The v0.5 vocabulary is 73% obsolete

`editor/meridian_spec_tools/src/main.rs` hard-codes a 37-entry `DOMAINS` list and a ~40-entry
`VALID_STATUSES` list. The v1 specoment uses **122 identifier families**.

- Families in both vocabularies: **10** — `FWK`, `ISO`, `MOD`, `NAV`, `NET`, `PEN`, `PRJ`, `TOR`, `TWO`, `UI`.
- v0.5 domains with no v1 counterpart: **27** — `AGT`, `ANI`, `AUD`, `BAS`, `BLD`, `COL`, `CORE`,
  `DAT`, `DCC`, `EDT`, `GAM`, `GOV`, `INT`, `MDL`, `PHY`, `PRC`, `PRM`, `REL`, `RHI`, `RUN`,
  `SEC`, `SHD`, `SYN`, `VCS`, `VEG`, `WRL`, `XR`.

This is not a list to patch. The family vocabulary must be **derived from the specoment**, or it
drifts the moment a contract is added. That is the substance of the phase card's instruction to
"replace its hard-coded v0.5 domain/status/ID assumptions".

## 2. The maturity vocabulary has no closed enum, and the validator will need one

§0.3 defines exactly **6** decision maturity labels. Headings use **29** distinct suffix phrases;
**24** of them are not in §0.3. Distribution is extremely skewed: `Normative` 437,
`Normative direction` 54, `Derived normative contract` 48, then a long tail of near-singletons
such as *"Normative ambition and initial architecture"*, *"Normative surface; parser details
prototype-gated"*, *"Post-1.0 planning seed; not a 1.0 requirement"*.

`WP-V1-RESET-002` therefore projects the label as a **verbatim opaque string**. `PH-AUTH-003`
owns "status axes" and must supply the normalisation table. Two hard requirements fall out:

- the table must **fail on an unmapped label**, never default one — a silent default is how a
  projection acquires a status the root file never stated;
- §0.4's three axes (documentation / implementation / evidence maturity) are separate enums from
  §0.3's decision labels. Conflating them would collapse the independence the specoment insists on.

Whether the long tail gets normalised into the 6 labels, or §0.3 gets extended to name the
compounds, is an **owner decision**, adjacent to the still-open `OD-003`.

## 3. Four generator defects must be validated against, not reproduced

`WP-V1-RESET-002` fixes all four. `PH-AUTH-003`'s validator must have a rule that would catch
each if it regressed:

| Defect | Rule the validator needs |
|---|---|
| 31 counted-but-unindexed identifiers | indexed count must equal declared total |
| `NETPROJ-006A..D` collapsed by a letter-blind guard | letter-suffixed identifiers are distinct identities |
| referencing heading read as a declaration | declaration requires a declaration marker, not mere containment |
| self-referencing declarations | an identifier's declaring heading may not appear in its own reference set |

## 4. Appendix G diverges from the prose phase cards — SD-004

21 genuinely divergent fields across 99 phases. Outside 12 uniform `PH-AI-*` gate strings, every
divergence sits in `PH-AUTH-001`, `PH-AUTH-002`, `PH-AUTH-003` or `PH-REL-008` — the Epoch 0
cards were revised after the Appendix G fence was serialised and it was never regenerated.

Appendix G's own preamble already resolves the conflict: *"If a serialization defect conflicts
with a prose phase card, the prose phase card wins until the registry is regenerated."*
`PH-AUTH-003` must therefore either regenerate the fence from the prose cards, or delete it in
favour of the generated registry. Leaving a stale hand-serialised fence inside the canonical
authority is the "second normative source" failure mode the phase exists to prevent.

## 5. Existing v0.5 governance surface to be replaced

- `schemas/governance/` — 23 schema files, 22 of which pair with a registry;
  `marquee-validation-fixture.schema.json` has no registry pair.
- `specs/registry/` — 22 JSON registries.
- `editor/meridian_spec_tools/src/main.rs` — 2,188 lines, single file, 39 passing tests in a
  417-line integration suite. Its Markdown walking, heading parsing, link checking and
  JSON-Schema utilities are reusable; its ID/domain/status logic is not.
- The `Command` enum plus `parse_args` is a central-dispatch growth point that `CODEHEALTH-*`
  and `PH-AUTH-006` both warn about. `PH-AUTH-003` is the right place to decompose it, since it
  is already rewriting the tool's semantics.

## 6. Exclusions inherited from PH-AUTH-002 that PH-AUTH-004 must retire

`is_excluded_context_path` will carry three v1 exclusions — `.meridian`,
`MERIDIAN_SPECOMENT.md` (both from `WP-V1-BASE-002`) and `governance` (from `WP-V1-RESET-002`).
All three exist only because a v0.5 validator would otherwise judge v1 content, and all three
**must disappear** when the tool is replaced. If any survives `PH-AUTH-004`, it has become
permanent hidden scope — a stop condition, not a nuisance.
