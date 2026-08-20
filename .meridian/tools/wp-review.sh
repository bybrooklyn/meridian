#!/usr/bin/env bash
# wp-review.sh — obtain an independent semantic review for a Meridian work-package plan.
#
# Satisfies IMPL-WP-001: the reviewer runs in a fresh isolated context that has
# never seen the drafting agent's reasoning. Independence is independence of
# reasoning context, not of vendor, so a fresh session of the same model
# qualifies. Self-review by the planning agent does not.
#
# Usage:
#   wp-review.sh <plan-file> [--reviewer claude|codex] [--specoment <path>]
#
# Exit codes:
#   0  verdict accept
#   1  verdict revise
#   2  verdict rethink
#   3  reviewer unavailable -> the work package BLOCKS (IMPL-WP-001)
#   4  usage or environment error

set -euo pipefail

REVIEWER=claude
SPECOMENT=""
PLAN=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --reviewer)  REVIEWER="${2:-}"; shift 2 ;;
    --specoment) SPECOMENT="${2:-}"; shift 2 ;;
    -h|--help)   sed -n '2,18p' "$0"; exit 0 ;;
    *)           PLAN="$1"; shift ;;
  esac
done

[[ -n "$PLAN" && -f "$PLAN" ]] || { echo "error: plan file not found: ${PLAN:-<none>}" >&2; exit 4; }

if [[ -z "$SPECOMENT" ]]; then
  for cand in ./MERIDIAN_SPECOMENT.md "$HOME/meridian/MERIDIAN_SPECOMENT.md"; do
    [[ -f "$cand" ]] && { SPECOMENT="$cand"; break; }
  done
fi
[[ -n "$SPECOMENT" && -f "$SPECOMENT" ]] || { echo "error: specoment not found; pass --specoment" >&2; exit 4; }

command -v "$REVIEWER" >/dev/null 2>&1 || {
  echo "BLOCKED: reviewer '$REVIEWER' unavailable. Per IMPL-WP-001 this work package blocks." >&2
  echo "Execute another unblocked, non-conflicting package instead." >&2
  exit 3
}

WP_ID="$(basename "$PLAN" .md)"
OUT_DIR="${MERIDIAN_REVIEW_DIR:-.meridian/implementation/reviews}"
mkdir -p "$OUT_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RECORD="$OUT_DIR/${WP_ID}-${STAMP}.md"

PROMPT_FILE="$(mktemp -t wp-review-prompt-XXXXXX)"
trap 'rm -f "$PROMPT_FILE"' EXIT

{
  echo "You are an independent semantic reviewer for the Meridian engine."
  echo "You have NOT seen the drafting agent's reasoning. Do not assume the plan is sound."
  echo
  echo "Authority: ${SPECOMENT}"
  echo "Read the sections the plan cites. Read the current source it touches."
  echo
  echo "Challenge the plan on every axis below. Be specific and cite file:line."
  echo "  - root cause / feature semantics"
  echo "  - authority and ownership"
  echo "  - dependency direction"
  echo "  - modularity and type-owned dispatch"
  echo "  - duplication and false abstraction"
  echo "  - scope, and whether a simpler alternative exists"
  echo "  - compatibility and migration"
  echo "  - security, privacy, accessibility"
  echo "  - test quality, and whether a regression test genuinely fails first"
  echo "  - evidence sufficiency"
  echo "  - research and provenance claims"
  echo
  echo "Do not affirm the plan to be agreeable. A plan with no findings is rare."
  echo "End your reply with exactly one line:"
  echo "VERDICT: accept | revise | rethink"
  echo "'rethink' requires a different approach, not cosmetic wording."
  echo
  echo "--- WORK PACKAGE PLAN (${PLAN}) ---"
  cat "$PLAN"
} > "$PROMPT_FILE"

echo "Requesting independent review of ${WP_ID} from ${REVIEWER} (fresh context)..." >&2

set +e
REVIEW_TEXT="$("$REVIEWER" -p "$(cat "$PROMPT_FILE")" 2>"$PROMPT_FILE.err")"
RC=$?
set -e

if [[ $RC -ne 0 || -z "$REVIEW_TEXT" ]]; then
  echo "BLOCKED: reviewer invocation failed (rc=$RC). Per IMPL-WP-001 this work package blocks." >&2
  head -20 "$PROMPT_FILE.err" >&2 || true
  rm -f "$PROMPT_FILE.err"
  exit 3
fi
rm -f "$PROMPT_FILE.err"

VERDICT="$(printf '%s\n' "$REVIEW_TEXT" | grep -oiE '^VERDICT:[[:space:]]*(accept|revise|rethink)' | tail -1 | awk '{print tolower($2)}')"
[[ -n "$VERDICT" ]] || VERDICT="revise"

MODEL_VERSION="$("$REVIEWER" --version 2>/dev/null | head -1)"

{
  echo "# Independent semantic review — ${WP_ID}"
  echo
  echo "- plan: \`${PLAN}\`"
  echo "- authority: \`${SPECOMENT}\`"
  echo "- reviewer: \`${REVIEWER}\` — ${MODEL_VERSION}"
  echo "- context: fresh isolated process, no drafting transcript inherited"
  echo "- timestamp: ${STAMP}"
  echo "- verdict: **${VERDICT}**"
  echo
  echo "## Findings"
  echo
  printf '%s\n' "$REVIEW_TEXT"
  echo
  echo "## Disposition"
  echo
  echo "_Record how each finding was addressed before implementation continues._"
} > "$RECORD"

echo "Review recorded: ${RECORD}" >&2
echo "VERDICT: ${VERDICT}" >&2

case "$VERDICT" in
  accept)  exit 0 ;;
  revise)  exit 1 ;;
  rethink) exit 2 ;;
esac
