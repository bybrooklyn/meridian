#!/usr/bin/env bash
# Meridian status.
#
# Rewritten at PH-AUTH-004. The previous version grepped PLANNING.md for a milestone table
# (`^## [0-9]+\. MS-[0-9]+`) and a work-package pattern. PLANNING.md is now a generated
# Appendix H.2 pointer, so those greps matched nothing and the script reported empty output
# with exit 0 — the "misleadingly stale" failure Appendix D rule 7 exists to catch, in an
# executable surface an agent trusts. Status now comes from the authority and its projections.

set -euo pipefail
cd "$(dirname "$0")/../../../.." || exit 1

echo "== Canonical authority =="
if [[ -f MERIDIAN_SPECOMENT.md ]]; then
  printf '  MERIDIAN_SPECOMENT.md  %s lines  sha256 %s\n' \
    "$(wc -l < MERIDIAN_SPECOMENT.md | tr -d ' ')" \
    "$(shasum -a 256 MERIDIAN_SPECOMENT.md | cut -c1-16)…"
else
  echo "  MISSING — the repository has no canonical authority" >&2
  exit 1
fi

echo
echo "== Implementation state =="
if [[ -f PLANNING.md ]]; then
  sed -n '/^- Active phase/,/^- Blockers/p' PLANNING.md
else
  echo "  PLANNING.md missing" >&2
fi

echo
echo "== Governance =="
cargo run -q -p meridian-spec -- check
