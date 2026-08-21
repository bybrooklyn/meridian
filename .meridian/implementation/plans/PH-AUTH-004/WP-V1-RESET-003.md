# WP-V1-RESET-003 — Atomic v1 authority cutover

Revision 2. Revision 1 received `rethink`. Its stop rule tested the wrong function, it
asserted a merge authorization the contract does not grant, and it bundled a separately
blocked deletion into the cutover. See Independent Review.

## Ownership

- Owning phase: `PH-AUTH-004` — Atomic v1 authority cutover
- Work package declared at `MERIDIAN_SPECOMENT.md:30163`
- Requirements: `SPEC-001`, `SPEC-002`, `LEGAL-003` (CLA replaces DCO), `IMPL-BOOTSTRAP-001`
  (which expires here), Appendix D rule 7, Appendix H.1 and H.2 templates
- Depends on: `PH-AUTH-002`, `PH-AUTH-003` — both closed
- Branch: `v1-authority-reset` at `cb82c9d`, **20 commits ahead of `main`**, 22 ahead of
  `origin/main`
- **Merge authorization: NOT HELD.** See Compatibility below.
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
3. **Contribution terms**, governed by `LEGAL-003` **and `LEGAL-MIG-001`**, a *Required
   migration* record revision 1 never cited. `CLA.md:4` states "Not yet in force."

   This is not an agent business decision: the card mandates the pause in terms, and
   `LEGAL-MIG-001` item 7 independently reads *"do not accept external contributions under
   contradictory terms during the transition."*

   Scoped precisely, because revision 1's wording over-claimed. On a **public MPL-2.0
   repository** you cannot prevent forks, pull requests or issues — only decline to merge
   them. The wording is therefore that external contributions cannot be **accepted** until
   the CLA completes legal review, while issues, discussions and security reports remain
   open. "External contributions are paused" reads as hostile and is not true.

   Surfaces: remove `DCO.md`; rewrite `CONTRIBUTING.md`; **fix `README.md:86`**, which reads
   "Contributions require [DCO](DCO.md) sign-off" and which revision 1 missed, leaving a
   broken link in the public README. Verified as a checked result rather than an assumption:
   `.github/` contains only two workflow files, so `LEGAL-MIG-001`'s "PR templates, bots"
   surface is genuinely empty. Items 1, 2, 3 and 5 of `LEGAL-MIG-001` remain open and go in
   the ledger as *deferred*, not silently omitted.
4. **Retire the v0.5 authority**: delete `specs/` (66 tracked files) and
   `schemas/governance/` (23), totalling **89 files and 15,448 lines** — 13,843 markdown plus
   1,605 JSON. Revision 1 said "45 files" in its table while saying 89 in its prose.

5. **Do NOT delete the v0.5 validator code.** Revision 1 bundled this in. It is unbundled for
   the same reason revision 1 itself gave for excluding `SD-009`: it is a second, unrelated
   semantic seam, and it is independently blocked.

   Neither the phase card nor `IMPL-BOOTSTRAP-001` mandates deleting the validator *code*.
   The card says retire "old CI assumptions"; the bootstrap says the cutover "removes active
   v0.5 specs/registries/DCO/old IDs and installs v1." **Deleting `specs/` makes every v0.5
   check inert automatically** — the authority is gone whether or not the Rust survives one
   more phase. The code and its 728-line test suite are removed by a follow-up package once
   `OD-010` is ruled.
6. **CI**: retarget the governance invocation, and add `project --check` plus the class (b)
   conformance run. This discharges Appendix D rule 7.
7. **Reset ledger** at `docs/migrations/v1-reset-ledger.md`: retained, redesigned, deferred,
   removed, and the frozen old SHA.
8. **`docs/` dispositions, per file, not per class.** Revision 1 covered this in one line —
   the largest remaining surface, and the sole support for the phase's headline closure row.
   It does not survive contact with the files: `docs/architecture/decisions/README.md` line 3
   reads verbatim **"Version: v0.5 canonical ADR set."** and instructs readers to use the
   status vocabulary from `specs/MERIDIAN_MASTER_SPEC.md`, which this same change deletes.
   That is not a dangling citation; it is a live index declaring v0.5 authority and pointing
   at a file the commit removes.

   84 files are tracked (`git ls-files docs | wc -l`); the 85th on disk is `.DS_Store`, which
   `.gitignore` excludes. 38 tracked files cite v0.5 by the criterion
   `grep -lE 'version 0\.5|MS-0[0-9] |WP-[A-Z]{3}-[0-9]'`.

   | Class | Disposition |
   |---|---|
   | **ADRs**, including `decisions/README.md` | **Re-cite, neither delete nor leave.** §0.5 rank 2 is "adopted subsystem ADRs *that explicitly cite the canonical version they refine*". An ADR citing v0.5 after the cutover no longer satisfies that predicate, so leaving them silently demotes them out of the authority order — and §0.5 closes "Conflicts are never resolved silently." Deleting is equally wrong: Appendix D adoption item 3 and `NORM-MIG-001..012` item 12 both use the verb **update**. Each ADR is re-cited to `MERIDIAN_SPECOMENT.md` and its digest, or headed `Superseded by <section>`. `decisions/README.md` loses "v0.5 canonical ADR set" and sources its vocabulary from §0.7. |
   | **`docs/migrations/`** | Retain as history, exempt by rule. A migration record citing v0.5 is not an active declaration of it. |
   | **`docs/benchmarks/`** | The `PEN-B*` question is **decided here**, not deferred: the v1 specoment retains `PEN-B04` as a named surrogate, so the workload identifiers survive and the files are re-cited. |
   | **`docs/production/`** | Unaddressed anywhere previously. These cite `MS-0N` review outcomes; they become history under `docs/migrations/` or are re-cited. |

9. **`state.json` refresh before generating `PLANNING.md`.** Appendix H.2 stamps the source
   checkpoint from `state.json`, whose `source_checkpoint` currently trails HEAD, so the
   generated pointer would ship stale on day one. `preserved_out_of_process_commit` also still
   says "main remains at `e0eb184`" when `main` is `ddbacd3`.

10. **`CLAUDE.md`**, an untracked symlink to `AGENTS.md`. After step 2 it would silently point
    at completely different content, and it is a live agent-instruction surface. Decided here:
    the symlink is retained, because the Appendix H.1 pointer is exactly what an agent should
    read, and it is recorded in the ledger.

## Explicit exclusions

- No production code redesign beyond build fixes required by governance paths.
- No specoment body edits. `SD-009` and `OD-009` are owner amendments and are **not** bundled
  into this change; bundling a canonical amendment into the atomic cutover would give the
  merge a second unrelated semantic seam.
- No push. Merging to `main` locally is authorised; publication is not.
- No feature work, no census. `PH-AUTH-005` owns classification.

## Compatibility / migration / authority effects

This is the largest authority change in the programme. `IMPL-BOOTSTRAP-001` expires here.

### Merge authorization is not held

Revision 1 asserted "Merging to `main` locally is authorised". That was wrong.
`IMPL-SCM-001` reads verbatim: *"It MUST NOT commit, push, **merge**, publish, release, deploy,
change credentials/permissions or rewrite history unless the execution environment/user has
explicitly granted **that action**."* Merge is enumerated separately from commit, and the
clause gates on *that action*. Authorization to commit does not imply authorization to merge,
and none was given.

`main` is additionally already 2 commits ahead of `origin/main` and the branch is 22 ahead, so
a local merge moves an already-diverged public branch further from its published state.

**Consequence for this package:** everything is prepared on `v1-authority-reset` and the merge
itself requires an explicit owner grant. `IMPL-SCM-001`'s own instruction for this situation is
followed — *"leave a clean reviewable working tree plus implementation/evidence records."*

### The v0.5 authority would exist in only one place

`git ls-remote --tags origin` returns **nothing**: the remote has no tags at all. Both
`v0.5-final-baseline` (`ddbacd3`) and `v1-authority-preimage` (`0882050a`) exist only on this
machine. Appendix D adoption item 5 requires preserving old evidence **immutably**, and a local
annotated tag on the same disk that just deleted 15,448 lines is not immutable preservation.

`OD-007` records exactly this and states it must resolve at `PH-AUTH-004` — this phase —
because it is the sole audit basis for the 14 `unreviewed_specoment_amendments` that `DEV-003`
names as residual exposure scheduled to close here. Revision 1 mentioned none of it.

Option (c) of `OD-007` — an off-repository local copy — needs no push authorization and is
taken as the default, executed before any deletion.

### Frozen SHA divergence

`IMPL-BOOTSTRAP-001` step 1 names `e0eb184` as the SHA to freeze; the tag is at `ddbacd3`,
produced inside the protocol per `DEV-005` after governance went green. Defensible, but the
reset ledger names `ddbacd3` and records the divergence rather than letting the two disagree
silently.

### Escalations, re-routed

Revision 1 escalated `MIG-001` as the sole pre-merge blocker and excluded `SD-009`, `OD-003`
and `OD-009` without a gate — inverting the priority.

`MIG-001` is **demoted from stop condition to a reset-ledger row** in the *deferred* column.
`RG-TOR-001` appears once, in a deferred post-1.0 subsystem section, classified citation-only.
Nothing in `state.json`, the phase graph or any work package keys off it. It controls no
implementation status, so it does not trip "old IDs still control implementation status", and
holding a 15,000-line cutover for one dangling word was over-cautious.

`SD-009`, `OD-003` and `OD-009` all record deadlines of "before the freeze". **This is the
freeze.** They become the pre-merge owner batch, with `MIG-001` appended as low priority.
`SD-009` — the defective Appendix A index — bears on the integrity of the *surviving*
authority far more than a dangling Torsant citation does.

**Nine owner decisions are open**, not the one revision 1 claimed: `OD-001` through `OD-010`
less `OD-008` (withdrawn). Four are scheduled at or before this phase.

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

None. Owner decisions are enumerated in Compatibility above.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| Root pointers (`AGENTS.md`, `PLANNING.md`, `README.md`, `VISION.md`, `CONTRIBUTING.md`) | ~120 | ~1,860 |
| `DCO.md` | 0 | 37 |
| v0.5 authority: `specs/` 66 files + `schemas/governance/` 23 files | 0 | **15,448** across **89 files** |
| `docs/` re-citation across 38 tracked files | ~80 | ~80 |
| `main.rs` — rewire `Check` to the v1 governance rules | ~40 | ~10 |
| `state.json` refresh | ~10 | ~10 |
| CI | ~10 | ~2 |
| Reset ledger | ~150 | 0 |

**Not in scope, and therefore not costed here:** deleting the v0.5 validator (~1,900 lines) and
its 728-line `tests/cli.rs` suite, which `ci.yml:14` runs. Revision 1 budgeted neither while
proposing the deletion — a second silent capability loss on top of the escalations, since those
tests exercise largely the same nine concepts `OD-010` escalates. Both move to the follow-up
package.

Revision 1's table said "~14,000 + 45 files" while its own prose said 89 files. Corrected:
**89 files, 15,448 lines** — 13,843 markdown plus 1,605 JSON.

Dominated by deletion, which is the shape a cutover should have.

## Stop / rollback rule

Stop if `main` would temporarily contain no authority; if both suites would be active; if
external contributors would face ambiguous terms; if the change would require a specoment body
edit; **if `governance/coverage-matrix.md` lists any unresolved owner escalation**; or if the
v0.5 authority would exist in only one location at merge time.

Revision 1's stop rule read "if the coverage matrix has any unaccounted unit". That tests
`unaccounted()`, which returns **0**, and ignores `escalations()`, which returns **9**. Those
are different functions and revision 1 tested the one that passes. The nine escalations are
`OD-010`, whose `blocks` field reads verbatim *"PH-AUTH-004 deletion of the v0.5 validator"* —
so the plan proposed a deletion the repository's own state records as blocked, on a licence it
had misread. Corrected, and the deletion is unbundled instead.

**Rollback.** Revision 1 said "reverting one merge commit", which is false as written:
`git merge-base --is-ancestor main HEAD` succeeds, so a default merge **fast-forwards and
creates no merge commit at all**. This package therefore mandates `--no-ff`, **prohibits
squash** — under squash the pre-deletion trees would be reachable only through the branch and
two unpushed tags — and states rollback concretely as `git revert -m 1 <merge-sha>`, with
`git reset --hard ddbacd3` as the fallback.

## Independent Review

_Pending._

## Completion record

_Pending._
