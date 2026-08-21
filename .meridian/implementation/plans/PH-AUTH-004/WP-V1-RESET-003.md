# WP-V1-RESET-003 — Atomic v1 authority cutover

## Ownership

- Owning phase: `PH-AUTH-004` — Atomic v1 authority cutover
- Work package declared at `MERIDIAN_SPECOMENT.md:30163`
- Requirements: `SPEC-001`, `SPEC-002`, `LEGAL-003` (CLA replaces DCO), `IMPL-BOOTSTRAP-001`
  (which expires here), Appendix D rule 7, Appendix H.1 and H.2 templates
- Depends on: `PH-AUTH-002`, `PH-AUTH-003` — both closed
- Branch: `v1-authority-reset` at `e845fe4`; merges to `main`
- Primary semantic seam: **which authority the repository declares.**

## User-visible / operational result

`main` carries one coherent v1 authority. A reader, an agent or a contributor arriving at the
repository cannot accidentally follow v0.5, because v0.5 is no longer there.

## Current source diagnosis

Measured at `e845fe4`; full inventory in `CUTOVER-INVENTORY.md` beside this file.

- **Root:** `AGENTS.md` (157), `PLANNING.md` (1,504), `README.md` (90), `VISION.md` (77),
  `CONTRIBUTING.md` (35) cite v0.5 authority. `DCO.md` (37) exists. `CLAUDE.md` is an
  **untracked symlink** to `AGENTS.md`.
- **v0.5 suite:** 44 `specs/*.md` totalling 13,843 lines, 22 `specs/registry/*.json`,
  23 `schemas/governance/*.schema.json`.
- **`docs/`:** 38 of 85 files cite v0.5 authority, in **three classes** needing three
  dispositions — ADRs sit inside the §0.5 authority order; `docs/migrations/` is history the
  v0.5 validator already exempts; benchmarks cite `PEN-B*` workload identifiers.
- **CI:** two `meridian-spec` invocations in `.github/workflows/ci.yml`, lines 13-14.
- **Coverage matrix:** 70 units, 66 deleted scope, 4 retained, **0 unaccounted**. This is what
  licenses deleting the v0.5 validator.

## Approach

One atomic change on `main`. Ordered so the repository never lacks authority at any commit.

1. **Merge the reset branch**, which installs the specoment, `governance/`, and `.meridian/`.
2. **Install the derived pointers**: `AGENTS.md` from Appendix H.1, `PLANNING.md` from
   Appendix H.2 generated from `state.json`. Repoint `README.md`, `VISION.md`.
3. **Contribution terms.** `LEGAL-003` requires a CLA replacing the DCO.
   `.meridian/legal/CLA.md` is drafted but **not legally reviewed and not in force**. The
   phase card is explicit: *"If final CLA language is not legally ready, explicitly pause
   external contribution acceptance rather than retaining the contradictory DCO."* So:
   remove `DCO.md`, rewrite `CONTRIBUTING.md` to state that external contributions are
   **paused pending CLA review**, and do not present the draft CLA as operative.
4. **Retire the v0.5 suite**: delete `specs/`, `schemas/governance/`, and the v0.5 validator
   code paths. History is preserved by tag `v0.5-final-baseline`; `evidence.json` and
   `waivers.json` are accumulated state, preserved by tag rather than merely deleted.
5. **`check` becomes the v1 validator**, and the v0.5 entry point is removed rather than
   renamed — with the suite deleted there is nothing for `check-v05` to check, so the
   `check`/`check-v05` inversion deferred from `PH-AUTH-003` is satisfied by deletion.
6. **CI**: retarget the governance invocation, and add `project --check` plus the class (b)
   conformance run. This discharges Appendix D rule 7.
7. **Reset ledger** at `docs/migrations/v1-reset-ledger.md`: retained, redesigned, deferred,
   removed, and the frozen old SHA.
8. **`docs/` dispositions** by class, as diagnosed.

## Explicit exclusions

- No production code redesign beyond build fixes required by governance paths.
- No specoment body edits. `SD-009` and `OD-009` are owner amendments and are **not** bundled
  into this change; bundling a canonical amendment into the atomic cutover would give the
  merge a second unrelated semantic seam.
- No push. Merging to `main` locally is authorised; publication is not.
- No feature work, no census. `PH-AUTH-005` owns classification.

## Compatibility / migration / authority effects

This is the largest authority change in the programme. `IMPL-BOOTSTRAP-001` expires here: after
this change the ordinary authority order in §0.5 governs a single coherent tree.

`MIG-001` — a live v1 sentence deferring an open question to the retired v0.5 gate
`RG-TOR-001` — must be resolved here or the cutover's own stop condition fires ("old IDs still
control implementation status"). Since editing the specoment is excluded, this is recorded as
the one item that **must** go to the owner before the merge lands.

## Accessibility / security / privacy / provenance / disabled-cost effects

- **Accessibility:** none; no runtime or UI surface.
- **Security:** removing `DCO.md` without an operative CLA means the repository accepts no
  external contributions until counsel review. That is the safe direction, and stating it
  plainly is the point.
- **Provenance:** strengthened. History is preserved by tag rather than by keeping a competing
  authority tree. The reset ledger names what happened to every concept.
- **Disabled cost:** deleting 89 files and the v0.5 validator reduces the graph; no runtime
  path changes.

## Tests and evidence

- **Fresh checkout** of `main` after the merge passes governance, build, workspace tests and
  dependency checks. This is the phase's headline closure row and is run from a clean clone,
  not from the working tree.
- **No active file declares v0.5 authority** — asserted by a rule, with `docs/migrations/`
  explicitly exempt because a migration record citing v0.5 is not an active declaration of it.
- **Both suites are not active** — asserted by the absence of `specs/` and
  `schemas/governance/`.
- Coverage matrix shows 0 unaccounted before the deletion; the deletion is not performed
  otherwise.
- CI runs `check`, `project --check` and the conformance rule.

## Failure injection and recovery

- Delete `specs/` without installing the pointers and confirm governance fails, proving the
  repository is never authority-less by accident.
- A file citing an `MS-0N` identifier as live must be reported; the same citation inside
  `docs/migrations/` must not.
- Revert the merge and confirm the tree returns to a coherent v0.5 state.

## Research candidates and selection metrics

None. One open owner decision, `MIG-001`, stated above.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| Root pointers (`AGENTS.md`, `PLANNING.md`, `README.md`, `VISION.md`, `CONTRIBUTING.md`) | ~120 | ~1,860 |
| `DCO.md` | 0 | 37 |
| v0.5 suite (`specs/`, `schemas/governance/`) | 0 | ~14,000 + 45 files |
| v0.5 validator paths in `main.rs` | ~0 | ~1,900 |
| CI | ~10 | ~2 |
| Reset ledger | ~120 | 0 |

Dominated by deletion, which is the shape a cutover should have.

## Stop / rollback rule

Stop if `main` would temporarily contain no authority; if both suites would be active; if
external contributors would face ambiguous terms; if old IDs still control implementation
status (`MIG-001`); if the coverage matrix has any unaccounted unit; or if the change would
require a specoment body edit. Rollback is reverting one merge commit.

## Independent Review

_Pending._

## Completion record

_Pending._
