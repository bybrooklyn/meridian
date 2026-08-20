# WP-V1-BASE-002 — v0.5 governance must not scan v1 staging authority

## Ownership

- Owning phase: `PH-AUTH-001` — Freeze the v0.5 baseline
- Requirements: `PH-AUTH-001` closure evidence, `IMPL-BOOTSTRAP-001`, `SPEC-001` (old and new authority must not compete)
- Branch: `main` (baseline repair)
- Depends on: `WP-V1-BASE-001` (same file, same crate — sequence them, do not run in parallel)
- Reviewer: **pending** — see Independent Review below

## User-visible / operational result

`cargo run -p meridian-spec -- check` passes on the frozen baseline while the corrected v1 authority is present in the working tree, without the v0.5 tool interpreting v1 documents as v0.5 authority.

## Current source diagnosis

`load_context` in `editor/meridian_spec_tools/src/main.rs:255` walks the whole tree and ingests every `.md` file. `is_excluded_context_path` (line 294) excludes only `game`, `target`, `.git`, `website` and the tool's own fixtures.

The v1 authority now lives in the repo per the owner's instruction that all work happen in `~/meridian`. The v0.5 tool therefore ingests `MERIDIAN_SPECOMENT.md` and `.meridian/**`, and reports:

- `docs:stale-phase-ref` on `MERIDIAN_SPECOMENT.md` and `.meridian/authority/MERIDIAN_SPECOMENT.original.md` — they legitimately reference retired v0.5 phases while describing the migration away from them;
- `list-unmapped:unmapped-id` for v1 identifiers such as `PRG-RECON-001` and `VAL-PORTFOLIO-001`, which by design do not exist in `specs/registry`;
- `list-unmapped:unmapped-id` for `REQ-004`, a work-package plan quoting its own test fixture.

`.gitignore` does not suppress this. `WalkDir` reads the filesystem directly and does not consult ignore rules — verified.

This is not a defect in the v1 documents. `SPEC-001` requires that old and new authority never compete; a v0.5 validator judging v1 content is exactly that competition, running backwards.

## Approach

Add `.meridian` and `MERIDIAN_SPECOMENT.md` to `is_excluded_context_path`, with a comment stating that these are staged v1 authority outside v0.5 governance scope and that the exclusion is retired together with the tool at `PH-AUTH-004`.

One semantic seam: what the v0.5 validator considers v0.5 authority.

## Explicit exclusions

- No change to any validation rule, status vocabulary, or registry schema.
- No `.gitignore` change. The v1 files must remain trackable for `PH-AUTH-002`.
- No suppression of any genuine v0.5 document. Only the two named v1 paths.
- No v1 validator work. `PH-AUTH-003` owns that.

## Compatibility / migration / authority effects

The v0.5 tool stops seeing v1 material. Every existing v0.5 document is still scanned. The exclusion disappears when the tool is replaced at `PH-AUTH-004`, so it cannot become permanent hidden scope.

## Accessibility / security / privacy / provenance / disabled-cost effects

None. Offline path filtering.

## Tests and evidence

- New fixture with a v1-shaped document at `MERIDIAN_SPECOMENT.md` carrying an unmapped identifier, plus a genuine v0.5 doc carrying one. Assert the v0.5 doc is still reported and the v1 file is not.
- Red evidence: `.meridian/implementation/evidence/WP-V1-BASE-002-red.log`.
- Green evidence: `.meridian/implementation/evidence/WP-V1-BASE-002-green.log`.
- Gates: `cargo test -p meridian-spec`, `cargo fmt --all -- --check`, `cargo clippy -p meridian-spec --all-targets -- -D warnings`, `cargo run -p meridian-spec -- check`.

## Failure injection and recovery

- A real `specs/` document with an unmapped identifier must still fail. This is the guard against over-broad exclusion.
- A file merely *named* like the specoment but nested elsewhere must still be scanned.

## Research candidates and selection metrics

None.

## LOC estimate

Production +4 / -0. Tests +12. Fixtures +10. Scope signal, not a quota.

## Stop / rollback rule

Stop if the exclusion suppresses any `specs/`, `docs/`, root policy, or ADR document, or if it would need to persist beyond `PH-AUTH-004`. Rollback is reverting one commit.

## Independent Review

**Status: REVIEWED — accepted.**

Same condition as `WP-V1-BASE-001`. CLI-agent reviewers exist on this machine but the owner has instructed they not be invoked. Self-review does not satisfy `IMPL-WP-001`. Submitted for owner review.

- Verdict: accept
- Reviewer: Brooklyn (project owner) — human, fresh context, did not see the drafting agent reasoning
- Reviewed: 2026-08-20T19:42:44Z
- Findings: none recorded; accepted as planned

## Completion record

- Completed: 2026-08-20T19:47:46Z
- Commit: `ddbacd3`
- Result: Committed. meridian-spec check now exits 0 (was 30 errors). 39 tests pass including v1_staging_authority_is_not_scanned, which asserts a genuine v0.5 document is still reported. Red evidence WP-V1-BASE-002-red.log and -red2.log precede the fix.
- Actual vs estimate: within the recorded scope signal.
