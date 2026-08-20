# WP-V1-RESET-001 — Freeze and reproduce the v0.5 baseline

## Ownership

- Owning phase: `PH-AUTH-001` — Freeze the v0.5 baseline
- Requirements: `PH-AUTH-001` closure evidence, `SPEC-001`, `IMPL-BOOTSTRAP-001`
- Depends on: `WP-V1-BASE-001`, `WP-V1-BASE-002` — both complete; governance must be green before the baseline is frozen
- Branch: `main`

## User-visible / operational result

A reproducible, immutable, externally identifiable record of the final v0.5 repository state exists, and the reset branch is created from it, before any v1 authority is installed.

## Current source diagnosis

`main` is at `ddbacd3` with governance green. An earlier ad-hoc freeze tagged `e0eb184`, which was red, and that tag was withdrawn under `DEV-005`. No baseline tag currently exists.

The tag `ev-ui-20260806-001` pins `5d19cae`, a dangling commit cited by an uncommitted `PLANNING.md` evidence row. Without it `git gc` would reap the commit and orphan that evidence.

## Approach

1. Verify `main` HEAD and that all gates pass from it.
2. Tag it `v0.5-final-baseline`, annotated.
3. Point `v1-authority-reset` at the same commit.
4. Regenerate the baseline manifest: SHA, tree hash, commit count, toolchain, lockfile hash, specoment hash, workflow and crate inventory, and every gate result with honest status.
5. Record unrun evidence as `NotRun` rather than omitting it.

## Explicit exclusions

- No v1 authority installed. That is `PH-AUTH-002`.
- No push. Publication is a separate owner authorisation.
- No feature work, no cleanup, no spec reinterpretation.
- No promotion of any old `MS-*` or `WP-*` claim.

## Compatibility / migration / authority effects

None. Tags and a document only. v0.5 authority continues to govern `main` until `PH-AUTH-004`.

## Accessibility / security / privacy / provenance / disabled-cost effects

None. Provenance improves: the baseline becomes externally identifiable and the dangling evidence checkpoint stays reachable.

## Tests and evidence

Gates re-run from the frozen SHA and recorded in the manifest: `meridian-spec check`, `cargo metadata --locked`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`. GPU smokes recorded with their honest occluded and unsupported-capability statuses. CI recorded `NotRun` because nothing is pushed.

## Failure injection and recovery

If the frozen SHA cannot reproduce the gates, the freeze does not happen. Rollback is deleting one tag; no source is touched.

## Research candidates and selection metrics

None.

## LOC estimate

Production 0. Documentation ~60 lines regenerated. No code changes.

## Stop / rollback rule

Stop if any gate fails from the frozen SHA, if the tag is not immutable and externally identifiable, or if the manifest would have to overstate any evidence status.

## Independent Review

**Status: REVIEWED — accepted.**

The owner accepted `WP-V1-BASE-001` and `WP-V1-BASE-002` and directed that the remaining `PH-AUTH-001` closure work proceed. This package makes no production code change: it produces a tag, a branch pointer, and a manifest, all trivially reversible.

- Verdict: accept
- Reviewer: Brooklyn (project owner)
- Findings: none recorded

## Completion record

- Completed: 2026-08-20T19:58:15Z
- Frozen SHA: `ddbacd34361c302e72ed2accefd59fe7567b28fe`
- Tag: `v0.5-final-baseline` (annotated) · reset branch `v1-authority-reset` at the same SHA
- Gates from the frozen SHA: meridian-spec check Pass, cargo metadata --locked Pass, fmt Pass, cargo test --workspace Pass (79 suites, 674 passed, 0 failed, 0 ignored), clippy --workspace -D warnings Pass (0 warnings), git diff --check Pass
- GPU smokes recorded with honest statuses: structural Pass, surface Occluded, GPU timing UnsupportedPlatform. No visible-quality claim.
- CI reproduction: NotRun. Nothing is pushed, so hosted CI has not seen this SHA.
- Manifest: `.meridian/authority/baseline-manifest.md`
- Actual vs estimate: no production code changed, as planned.
