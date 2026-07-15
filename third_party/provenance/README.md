# Renderer donor provenance policy

version 0.5 · Policy only · No donor records registered

Meridian may study or adapt externally licensed renderer ideas or source only
through an explicit provenance record. This directory intentionally contains no
invented donor entries.

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

Code cannot enter a runtime crate before provenance and license review pass.
Private or confidential material cannot be sanitized into eligibility. Missing
provenance is not waivable and cannot support implementation maturity.
