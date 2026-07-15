#!/usr/bin/env bash
set -euo pipefail

remote=0
if [[ "${1:-}" == "--remote" ]]; then
  remote=1
  shift
fi

engine_root="${1:-/Users/brooklyn/meridian}"
game_root="${engine_root}/game"
gh_bin="/opt/homebrew/bin/gh"

if [[ ! -d "${engine_root}/.git" ]]; then
  printf 'error: engine repository not found at %s\n' "${engine_root}" >&2
  exit 1
fi

printf 'Engine repository: %s\n' "${engine_root}"
git -C "${engine_root}" status --short --branch
printf 'Engine HEAD: %s\n' "$(git -C "${engine_root}" log -1 --oneline --decorate)"

if [[ -d "${game_root}/.git" ]]; then
  printf '\nGame repository: %s\n' "${game_root}"
  git -C "${game_root}" status --short --branch
  printf 'Game HEAD: %s\n' "$(git -C "${game_root}" log -1 --oneline --decorate)"
else
  printf '\nGame repository: absent (allowed for an engine-only checkout)\n'
fi

if [[ -f "${engine_root}/PLANNING.md" ]]; then
  printf '\nActive-plan signals:\n'
  if command -v rg >/dev/null 2>&1; then
    rg -n -m 8 '^(Status:|The current closure candidate|`WP-[A-Z]+-[0-9]+` is the immediate|`WP-[A-Z]+-[0-9]+` follows|## [0-9]+\. MS-[0-9]+)' "${engine_root}/PLANNING.md" || true
  else
    grep -En -m 8 '^(Status:|The current closure candidate|`WP-[A-Z]+-[0-9]+` is the immediate|`WP-[A-Z]+-[0-9]+` follows|## [0-9]+\. MS-[0-9]+)' "${engine_root}/PLANNING.md" || true
  fi
fi

if (( remote == 0 )); then
  exit 0
fi

printf '\nGitHub state:\n'
if [[ ! -x "${gh_bin}" ]]; then
  printf 'error: expected GitHub CLI at %s\n' "${gh_bin}" >&2
  exit 1
fi

"${gh_bin}" auth status
"${gh_bin}" repo view bybrooklyn/meridian \
  --json nameWithOwner,isPrivate,defaultBranchRef,url
"${gh_bin}" repo view bybrooklyn/project-meridian \
  --json nameWithOwner,isPrivate,defaultBranchRef,url
"${gh_bin}" run list -R bybrooklyn/meridian --limit 5 \
  --json databaseId,headSha,status,conclusion,workflowName,url
