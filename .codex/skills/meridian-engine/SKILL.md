---
name: meridian-engine
description: Work safely and truthfully on the Meridian Rust engine and its separate private Project Meridian game repository. Use for implementation, debugging, planning, specification amendments, architecture reviews, validation, milestone or work-package status, repository navigation, evidence closure, CI, commits, pushes, or any task under /Users/brooklyn/meridian or /Users/brooklyn/meridian/game.
---

# Meridian Engine

Use live repository authority to complete bounded work without blurring plans,
implementation, evidence, or private game content.

## Repository roots

- Engine: `/Users/brooklyn/meridian`, remote `bybrooklyn/meridian`.
- Game: `/Users/brooklyn/meridian/game`, an independent ignored nested repository,
  remote `bybrooklyn/project-meridian`.
- Treat both repositories as private unless live GitHub metadata says otherwise.
- Run Git commands with an explicit working directory. Never stage the nested
  game repository from the engine repository.
- GitHub CLI is installed at `/opt/homebrew/bin/gh` but is not reliably on
  `PATH`. Use that absolute path; do not assume bare `gh` works.

## Start every task from live state

1. Read `/Users/brooklyn/meridian/AGENTS.md` and any narrower `AGENTS.md`.
2. Run `scripts/project-status.sh`; add `--remote` when GitHub/CI state matters.
3. Read the current versions of:
   - `specs/MERIDIAN_MASTER_SPEC.md`;
   - `PLANNING.md`;
   - `specs/DELIVERY_ROADMAP.md`;
   - `specs/IMPLEMENTATION_PLANNING_SPEC.md`;
   - the owning subsystem specification and applicable ADR/registry records.
4. Inspect the current diff before editing. Preserve unrelated user changes.
5. Prefer `rg` and `rg --files` for repository discovery.

Never freeze a milestone, package status, test count, dependency version, CI
result, or implementation claim from this skill. Recheck it. PLANNING can lag a
new CI run; code and evidence prove current behavior, while specs own intended
architecture.

## Apply the authority model

Use this conflict order:

1. owning current subsystem spec plus adopted ADR;
2. migration and contradiction register;
3. delivery roadmap;
4. implementation-planning spec;
5. typed registries under `specs/registry/`;
6. root `PLANNING.md` for active bounded work and current evidence;
7. private game documents for creative decisions only;
8. history/migration records;
9. code and evidence for current behavior, not automatic permanent design.

Do not silently resolve a contradiction. Keep documentation maturity,
implementation maturity, and evidence status independent. A plan, scaffold,
constructor, smoke, occluded capture, or passing test count is never proof of a
finished product or visual quality.

## Choose the workflow

### Implement or fix code

- Work from one activated `WP-*` package and its Definition of Ready/Done.
- Trace requirements, dependencies, non-goals, failure modes, tests, evidence,
  integration checkpoint, and stop rule before changing code.
- Reuse Meridian-owned contracts and existing utilities. Keep third-party types
  behind adapters and do not add dependencies without the owning decision.
- Run targeted checks first, then proportional workspace gates.
- Update evidence and PLANNING only for results actually produced.
- Work directly. Do not use subagents unless the user explicitly asks or a
  genuinely independent verification lane materially improves safety.

### Amend specifications or planning

- Put normative prose in one owning spec; link elsewhere instead of duplicating.
- Update affected requirements, work packages, ADRs, registries, migration or
  contradiction records, validation contracts, roadmap mapping, and PLANNING.
- Use stable IDs and machine-valid status vocabulary.
- Keep planned contract examples explicitly illustrative unless compiled.
- Run `cargo run -p meridian-spec -- check` before claiming coherence.

### Work on Project Meridian

- Read `/Users/brooklyn/meridian/game/docs/README.md` first.
- Keep narrative, route, art, AMI identity, proprietary recipes/seeds, assets,
  hero overrides, and game code in the private game repository.
- Put only sanitized functional contracts, generated surrogates, controlled
  identifiers, and hashes in the engine repository.
- Treat `PEN-B04` as a generated redacted AMI-interior surrogate. Never copy
  private AMI lore, logos, documents, narrative, routes, or assets into engine
  fixtures, benchmarks, docs, or tests.

### Report status or review readiness

- Refresh Git, PLANNING, registries, tests, and CI first.
- State separately: implemented behavior, local evidence, remote evidence,
  missing qualification, unsupported/occluded rows, and next unblocked package.
- Never turn ambition or documentation completeness into implementation claims.

## Preserve core product invariants

- Meridian is one general-purpose engine and one creator application named
  **Meridian**, not separate Studio/IDE products.
- Penumbra is the renderer; Forward+ is adopted but current foundations are not
  the complete production renderer.
- Alluvium owns procedural authoring/evaluation/provenance, not live runtime
  authority; baked-only use has zero Alluvium runtime cost.
- Wavefront owns audio. Collective owns optional online-service policy/provider
  seams. Cairn owns physics direction. Isobar, Basalt, and Torsant own weather,
  terrain, and coupled fire/fluid/thermal domains.
- Rust gameplay comes before optional Luau.
- The native beginner-friendly modeler is core; Blender remains optional.
- Disabled optional packs add zero tasks, threads, listeners, allocations, GPU
  resources, panels, dependencies, or package chunks.
- Source documents are authoritative; compiled artifacts and caches are derived.
- Stable persistent IDs cross durable boundaries; generational handles remain
  process-local. Cross-domain mutation uses typed commands and barriers.

Read `references/project-map.md` for the architecture, milestone, crate, and ID
map. Read `references/workflows.md` before implementation, validation, evidence,
CI, commit, or push work.

## Git and external-action boundary

- Never commit, push, tag, publish, create a PR/release, deploy, change
  credentials, or message externally unless the user explicitly authorizes it.
- When authorized, commit engine and game changes independently with accurate
  scope and verify each local SHA equals its upstream SHA after push.
- Use `/opt/homebrew/bin/gh` for authentication, repository, Actions, issue, PR,
  and release commands.
