# v1 authority reset ledger

`PH-AUTH-004` / `WP-V1-RESET-003`. Frozen old SHA: **`ddbacd34361c302e72ed2accefd59fe7567b28fe`**,
tag `v0.5-final-baseline`.

`IMPL-BOOTSTRAP-001` step 1 names `e0eb184` as the SHA to freeze. `DEV-005` records that the
ad-hoc freeze at that commit was withdrawn and re-run inside the protocol at `ddbacd3`, after
governance went green. `ddbacd3` is authoritative; the divergence is recorded here rather than
left for a reader to discover.

## Retained

| Concept | Where it lives now |
|---|---|
| Architecture decision records | `docs/architecture/decisions/`, re-cited to the specoment. §0.5 ranks adopted ADRs directly below the specoment **only where they cite the version they refine**, so leaving them citing v0.5 would have silently demoted them out of the authority order. Appendix D item 3 says *update*, not retire. |
| Benchmark records | `docs/benchmarks/`, re-cited. The `PEN-B*` workload identifiers survive into v1; `PEN-B04` is named in the specoment as a generated surrogate. |
| Migration history | `docs/migrations/`, retained as history. A migration record citing v0.5 is not an active declaration of it. |
| Evidence and waiver records | Accumulated state, preserved by tag and off-repository copy. Not derivable from a design document, so never regenerable. |

## Redesigned

| Concept | v0.5 form | v1 form |
|---|---|---|
| Canonical authority | 44 documents under `specs/` | one `MERIDIAN_SPECOMENT.md` plus generated `governance/` projections |
| Traceability | Appendix A pasted from a Python script | generated, stamped, reconciled by `project --check` |
| Governance validation | `check` over `specs/registry/` | `check` over the specoment and its projections |
| Contribution terms | DCO sign-off | CLA, drafted and **not yet in force**; acceptance paused |

## Deferred

| Concept | Status |
|---|---|
| `WP-UI-006` | Not completed under v0.5 authority. Its code and evidence are frozen and classified; `IMPL-BOOTSTRAP-001` routes them to the later `.mui`/UI phases. |
| `MIG-001` — `RG-TOR-001` | A live v1 sentence defers the Torsant solver portfolio to a v0.5 gate that no longer exists. It controls no implementation status, so it is a dangling citation rather than old authority still in force. A v1 gate is not yet allocated. |
| `LEGAL-MIG-001` items 1, 2, 3, 5 | Open. Only item 4 (retire the DCO) and item 7 (do not accept contradictory terms) are discharged here. |
| v0.5 validator code and its test suite | ~1,900 lines plus a 728-line suite, still present and reachable as `check-v05`. Removal waits on `OD-010`. |
| `OD-001`, `OD-002`, `OD-003`, `OD-004`, `OD-005`, `OD-006`, `OD-009`, `OD-010` | Open owner decisions carried past the freeze. |

## Removed

| Concept | Count | Preservation |
|---|---|---|
| `specs/` | 66 files | tag `v0.5-final-baseline`, off-repo bundle |
| `schemas/governance/` | 23 files | same |
| `DCO.md` | 1 file | same |
| Two Marquee fixture tests | — | validated a deleted v0.5 schema; Marquee is deferred post-1.0 and returns with its own |

Total deleted: **90 files, 15,448 lines** — 13,843 markdown plus 1,605 JSON.

## Enforcement changes, stated rather than implied

`governance/coverage-matrix.md` accounts for every rule the v0.5 validator could emit. Of 66
units in deleted scope: **7 superseded** with a named backing fixture, **9 escalated** to the
owner as `OD-010` because the concept survives into v1 with no successor, and the rest retired
with the authority they policed.

The nine are not a formality. §0.4 still forbids promoting a status without evidence and still
forbids reading `Occluded` as visible quality; `IMPL-WP-003` still requires fresh evidence to be
registered; Appendix D still requires source links to resolve; waivers still exist. Until
`OD-010` is ruled, those concepts are unenforced, and the v0.5 code is deliberately left in
place so the ruling is not made by deletion.

## Preservation

`git ls-remote --tags origin` returned nothing: the remote carries no tags, so both
preservation tags existed only in one working copy. `OD-007` option (c) was executed before any
deletion — an off-repository copy at `~/meridian-v05-preservation` holding a bundle of every
ref and tag, the v0.5 suite as a tarball, and both specoment preimages.
