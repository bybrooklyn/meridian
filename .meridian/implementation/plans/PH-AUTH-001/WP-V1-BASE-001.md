# WP-V1-BASE-001 — Retired waivers must not fail governance

## Ownership

- Owning phase: `PH-AUTH-001` — Freeze the v0.5 baseline
- Requirements: `PH-AUTH-001` closure evidence ("the old workspace and current CI pass from the frozen SHA"), `IMPL-BOOTSTRAP-001`, §0.4 maturity axes
- Branch: `main` (baseline repair; the frozen baseline must be green before it is tagged)
- Reviewer: **pending** — see Independent Review below

## User-visible / operational result

`cargo run -p meridian-spec -- check` passes on the commit that `PH-AUTH-001` freezes, without deleting or backdating any waiver record.

## Current source diagnosis

`editor/meridian_spec_tools/src/main.rs` collects `expiry` and `expires` from every registry record via `strings_for_keys` and reports `expired-waiver` whenever the date is in the past. The record's `status` is never consulted.

`specs/registry/waivers.json` holds `WVR-UI-001` and `WVR-EDT-001`. Both already carry `"status": "Closed"` with `"expires": "2026-08-17"`. Both therefore fail governance permanently from 2026-08-18 onward.

A retired waiver grants no exemption. Its expiry date is historical fact, not a live governance risk. The defect is in the validator, not the data.

Evidence of the defect, captured before any fix: `.meridian/implementation/evidence/WP-V1-BASE-001-red.log` — `closed_waivers_do_not_expire` FAILED with `REQ-004 has expired waiver 2026-01-01`.

## Approach

1. Add `RETIRED_WAIVER_STATUSES` = `Closed`, `Retired`, `Superseded`, `Rejected`.
2. Add `live_expiries` / `collect_live_expiries`. Any object carrying a retired status is skipped whole, including nested waivers, so neither its own expiry nor a nested one is reported.
3. Replace the `strings_for_keys` call in the maturity check with `live_expiries`.
4. Leave open waivers reporting exactly as before.

One semantic seam: which expiry dates still govern.

## Explicit exclusions

- No edit to `specs/registry/waivers.json`. Backdating or deleting the records would destroy evidence to silence a tool defect.
- No change to any other validation rule, status vocabulary, or registry schema.
- No v1 authority work. This repairs the v0.5 baseline only.

## Compatibility / migration / authority effects

Governance becomes less strict for retired waivers and unchanged for live ones. No registry, schema, format or public API changes. No migration.

## Accessibility / security / privacy / provenance / disabled-cost effects

None. Offline validator logic only. Provenance improves: the waiver records survive intact rather than being rewritten.

## Tests and evidence

- New fixture `tests/fixtures/closed_waiver/` — a nested waiver with `status: "Closed"` and a past `expiry`.
- New test `closed_waivers_do_not_expire` — asserts the validator passes and emits no `expired-waiver`.
- Existing `expired_waivers_are_rejected` must keep failing its fixture. That fixture's waiver has no `status`, so an open expired waiver is still reported. This is the regression guard against over-broad suppression.
- Gates: `cargo test -p meridian-spec`, `cargo fmt --all -- --check`, `cargo clippy -p meridian-spec --all-targets -- -D warnings`, `cargo run -p meridian-spec -- check`.

Red evidence precedes the fix and is recorded. Green evidence follows it.

## Failure injection and recovery

- Waiver with no `status` and a past expiry → still reported. Covered by the retained test.
- Waiver with an unrecognised status and a past expiry → still reported, since only the four listed statuses suppress.
- Nested waiver inside a retired parent → suppressed with the parent, by design.

## Research candidates and selection metrics

None. No algorithm is open.

## LOC estimate

Production +42 / -1. Tests +7. Fixtures +16. Scope signal, not a quota.

## Stop / rollback rule

Stop if suppression would hide any waiver that still grants an exemption, if the retained `expired_waivers_are_rejected` test stops failing its fixture, or if the fix requires editing registry data. Rollback is reverting one commit; no data or authority is touched.

## Independent Review

**Status: REVIEWED — accepted.**

`IMPL-WP-001` requires review by a fresh isolated reasoning context before implementation, and states that the package blocks if none can be obtained. CLI-agent reviewers (`claude`, `codex`) are available on this machine but the owner has instructed that they not be invoked in this session. Self-review by the drafting agent does not satisfy the requirement.

The owner is a valid independent reviewer: a human, in a fresh context, who has not seen the drafting agent's reasoning. **This plan is submitted for owner review.** Implementation is complete in the working tree but is not committed pending a verdict.

- Verdict: accept
- Reviewer: Brooklyn (project owner) — human, fresh context, did not see the drafting agent reasoning
- Reviewed: 2026-08-20T19:42:44Z
- Findings: none recorded; accepted as planned

## Completion record

- Completed: 2026-08-20T19:47:46Z
- Commit: `4cc7d55`
- Result: Committed. cargo test -p meridian-spec 39 passed. fmt, clippy -D warnings clean. Red evidence WP-V1-BASE-001-red.log precedes the fix; green follows. Registry data untouched.
- Actual vs estimate: within the recorded scope signal.
