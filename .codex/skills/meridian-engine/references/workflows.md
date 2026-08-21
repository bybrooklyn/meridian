# Meridian Workflows

## Fast resume

Run:

```bash
./.codex/skills/meridian-engine/scripts/project-status.sh
sed -n '1,220p' AGENTS.md
sed -n '1,260p' PLANNING.md
```

For remote truth:

```bash
./.codex/skills/meridian-engine/scripts/project-status.sh --remote
```

Then read the owning spec, package registry entry, requirement records, ADRs,
and relevant implementation/tests. Do not start from memory or an old plan.

## Bounded implementation loop

1. Name the active `WP-*`, user-visible result, dependencies, requirements,
   explicit non-goals, and stop/rollback rule.
2. Confirm Definition of Ready in `MERIDIAN_SPECOMENT.md` `IMPL-WP-001` and typed
   status in the registries.
3. Inspect current code, tests, and dirty changes. Identify the smallest owning
   crates/files and preserve unrelated work.
4. Add or tighten a targeted regression test when behavior is missing or broken.
5. Implement through Meridian-owned contracts. Keep third-party types at
   adapters, bounded allocations/queues, typed failure outcomes, deterministic
   source authority, and recovery paths.
6. Run targeted tests immediately. Fix warnings rather than suppressing them.
7. Run proportional gates below.
8. Record fresh evidence with honest unsupported, occluded, or inconclusive
   rows. Update package/milestone status only when all required evidence exists.
9. Use the work-package plan template in Appendix H.3 of `MERIDIAN_SPECOMENT.md`.

## Specification amendment loop

For a normative change, update all affected surfaces:

1. owning subsystem spec and requirement prose;
2. adopted/new ADR when architecture changes;
3. typed registries and matching JSON schemas if structure changes;
4. work-package dependencies and delivery-plan mapping;
5. research gate/risk/waiver/provenance records where applicable;
6. migration and contradiction disposition;
7. validation fixtures, API examples, and cross-links;
8. root `AGENTS.md`, `README.md` and `PLANNING.md` when their authority changes;
9. private game docs only for creative/production boundary changes.

Do not version-bump historical ADR text merely because the current suite bumps.
Do not delete legacy authority until every heading has a mapped disposition and
link validation passes.

## Validation ladder

Start narrow, then expand in proportion to risk:

```bash
cargo run -p meridian-spec -- check
cargo metadata --locked
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p meridian-rhi --example clear_frame
cargo run -p meridian-renderer --example instance_upload_smoke
cargo run -p meridian-editor --bin meridian -- --headless-smoke --frames 4
cargo run -p meridian-editor --bin meridian -- --smoke --frames 4
git diff --check
```

Use native smokes only where the environment supports them. Record minimized,
occluded, unsupported, device-lost, or unavailable surfaces explicitly. Never
claim visible quality from offscreen/occluded structural evidence.

Also audit:

- broken Markdown links/fences and stale IDs/filenames;
- `git ls-files game` returns no engine-tracked game paths;
- secrets, personal paths, machine-local files, private game content, and large
  generated artifacts are absent;
- `Cargo.lock` and metadata contain no private game packages;
- no blocking wait was introduced in asynchronous timing/capture paths;
- source/package/save migrations and recovery remain tested when touched.

CI currently runs governance first, then format, check, workspace tests, clippy,
and headless smoke on Linux, Windows, and macOS. Read `.github/workflows/ci.yml`
instead of assuming this matrix remains unchanged.

## GitHub CLI and CI

The executable is outside the dependable PATH:

```bash
GH=/opt/homebrew/bin/gh
"$GH" auth status
"$GH" repo view bybrooklyn/meridian --json nameWithOwner,isPrivate,defaultBranchRef,url
"$GH" repo view bybrooklyn/project-meridian --json nameWithOwner,isPrivate,defaultBranchRef,url
"$GH" run list -R bybrooklyn/meridian --limit 10
"$GH" run watch -R bybrooklyn/meridian RUN_ID --exit-status
```

Never print tokens or credential files. Do not infer authorization to create,
edit, close, publish, release, or message merely because authentication works.

## Commit and push

Only act after explicit user authorization.

1. Inspect `git status -sb`, unstaged diff, staged diff, remote, and branch in
   each repository separately.
2. Confirm every changed file belongs to the requested scope. Avoid broad
   staging when unrelated work exists.
3. Run relevant validation and `git diff --cached --check`.
4. Create separate truthful commits for engine and game changes.
5. Fetch and ensure the remote did not advance unexpectedly.
6. Push the requested branch.
7. Fetch again and verify `HEAD` equals its upstream; verify CI when required.

Use explicit roots:

```bash
git status -sb
git -C game status -sb
/opt/homebrew/bin/gh run list -R bybrooklyn/meridian --limit 5
```

Never include `game/` in an engine commit. Never copy engine licensing onto
proprietary Project Meridian content.

## Completion report

Lead with outcome. Include:

- package/result completed;
- files or crates materially changed;
- targeted and full validation actually run;
- evidence/limitations, including unsupported and occluded rows;
- commit/push/CI state only when verified;
- next unblocked package from current authority.

Keep it concise and never restate the entire roadmap.
