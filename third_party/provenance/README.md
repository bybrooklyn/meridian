# Renderer donor provenance policy

version 1.0 · Policy and registered dependency/source-asset records

Meridian may study or adapt externally licensed renderer ideas or source only
through an explicit provenance record. This directory intentionally contains no
invented donor entries.

Immutable UI fonts and icon sources use the same hash, license, notice,
modification, owner, test, and exit-strategy controls. Their machine-validated
source records live in
[the retired v0.5 source-provenance registry](../../the retired v0.5 source-provenance registry);
the Markdown files here are human review notes for those records, not a
separate source of truth.

Every future donor record must include:

- `SRC-<DOMAIN>-NNN`, owning `WP-*` or `RG-*`, reviewer, and date;
- upstream project, canonical URL, exact revision, original paths, and retrieved
  artifact hash;
- copyright holders, license text/version, compatibility analysis, notices, and
  attribution obligations;
- classification as idea study, clean rewrite, adapted source, generated output,
  patch, test corpus, or tool-only dependency;
- Meridian destination, modifications, retained notices, and removal/update
  procedure;
- private-corpus and confidential-source screening;
- tests proving behavior and boundaries without treating origin as correctness.

Code or immutable assets cannot enter a runtime crate before provenance and
license review pass.
Private or confidential material cannot be sanitized into eligibility. Missing
provenance is not waivable and cannot support implementation maturity.
- **Control status: unmet.** The v0.5 source-provenance registry was retired at `PH-AUTH-004` and no v1 registry exists. `LEGAL-005` requires machine-readable provenance for every third-party dependency; the repository has none for its 494 locked packages. Recorded as `OD-006`, pre-existing and unmet — this is a broken control, not a repaired link.
