# PH-AUTH-004 — cutover surface inventory

Gathered read-only at `14d3feb`, before `WP-V1-RESET-003` is planned. Not a plan record.

## Root files declaring or citing v0.5 authority

| File | Lines | Disposition |
|---|---|---|
| `AGENTS.md` | 157 | Replace with the Appendix H.1 minimal pointer |
| `PLANNING.md` | 1,504 | Replace with the Appendix H.2 derived pointer, generated from `state.json` |
| `README.md` | 90 | Repoint; strip stale milestone claims |
| `VISION.md` | 77 | Repoint |
| `CONTRIBUTING.md` | 35 | Repoint; remove DCO instructions |
| `DCO.md` | 37 | **Remove** per `LEGAL-003` |
| `ACKNOWLEDGMENTS.md` | 33 | No v0.5 authority; review only |
| `CLAUDE.md` | symlink | Untracked symlink to `AGENTS.md`; decide follow or remove |

## v0.5 suite to retire

| Tree | Count | Lines |
|---|---|---|
| `specs/*.md` | 44 | 13,843 |
| `specs/registry/*.json` | 22 | — |
| `schemas/governance/*.schema.json` | 23 | — |

`specs/registry/evidence.json` (52 records) and `waivers.json` carry **accumulated state**,
not derivable from the root. Retiring them is not the same as deleting the v0.5 documents;
their records are the v0.5 evidence history and must be preserved by tag rather than deleted
in place. See `SD-011` for why this class is different.

## CI

Two governance invocations, both in `.github/workflows/ci.yml`:

```text
line 13   cargo run -p meridian-spec -- check --output=github
line 14   cargo test -p meridian-spec
```

Line 13 changes twice across the programme: to `check-v05` when `WP-V1-GOV-001` renames the
v0.5 entry point, then back to `check` at the cutover when the v0.5 validator is deleted and
the v1 one takes the plain name. That is the churn the `check`/`check-v05` inversion was
chosen to avoid on the *surviving* command.

Appendix D rule 7 — fail CI when misleadingly stale — lands here: `project --check` and the
class (b) conformance rule must both run in CI.

## Carried into this phase

| Item | Source |
|---|---|
| `MIG-001` — live v1 prose cites the retired v0.5 gate `RG-TOR-001` | `PH-AUTH-002` |
| `OD-007` — authority preimage exists only on this machine | `WP-V1-RESET-002` |
| `OD-009` — 24 unlisted maturity suffixes must land before the freeze | `WP-V1-GOV-001` |
| `SD-009` — the root's Appendix A is the defective 700-entry index; owner amendment **before** the freeze | `WP-V1-GOV-001` |
| 20 Appendix G divergences from the prose cards | `WP-V1-RESET-002` |
| Appendix D rule 7 CI enforcement | `WP-V1-RESET-002`, `WP-V1-GOV-001` |
| Contribution terms: `LEGAL-003` requires a CLA replacing the DCO. `.meridian/legal/CLA.md` is drafted but **not in force**. The phase card is explicit: if the CLA is not legally ready, pause external contribution acceptance rather than retaining the contradictory DCO. | `PH-AUTH-004` card |

## `docs/` — measured

`docs/` holds 85 files. **38** cite v0.5 authority (`version 0.5`, an `MS-0N` milestone, or a
`WP-XXX-NNN` identifier), by subtree:

- `docs/architecture/` — 12
- `docs/benchmarks/` — 17
- `docs/migrations/` — 6
- `docs/production/` — 3

These are not one class and must not get one disposition:

- **`docs/architecture/decisions/`** — ADRs sit *inside* the §0.5 authority order, which ranks
  "adopted subsystem ADRs that explicitly cite the canonical version they refine" directly
  below the specoment. The `PH-AUTH-004` card requires updating ADRs that conflict with the v1
  contract. These need individual dispositions, not bulk retirement.
- **`docs/migrations/`** — historical records. The v0.5 validator already exempts this path
  from its retired-reference check, precisely because a migration record must be free to name
  what it migrated away from. Retain as history.
- **`docs/benchmarks/`** — cite `PEN-B*` workload identifiers, which are v0.5 registry content.
  Disposition depends on whether the v1 programme keeps the workload identifiers.

`WP-V1-RESET-003` must settle all three before claiming "no active file declares v0.5
authority", and the claim itself needs qualifying: a migration record citing v0.5 is not an
active declaration of it.
