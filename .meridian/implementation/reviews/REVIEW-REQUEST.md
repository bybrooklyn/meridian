# Review request — two blocked work packages

Both packages are implementation-ready and blocked solely on `IMPL-WP-001`'s independent-review requirement. The pre-commit hook enforces the block; neither can be committed until a verdict is recorded.

You are a valid independent reviewer under the rule as written: a human, in a fresh context, who has not seen the drafting agent's reasoning.

## To accept a package

Add this line anywhere in its plan file:

```text
- Verdict: accept
```

The hook then permits the commit for that package. To reject, write `revise` or `rethink` with findings instead.

To use the automated reviewer instead, lift the no-CLI-agent restriction and run:

```text
.meridian/tools/wp-review.sh .meridian/implementation/plans/PH-AUTH-001/WP-V1-BASE-001.md
```

---

## WP-V1-BASE-001 — Retired waivers must not fail governance

**Plan:** `.meridian/implementation/plans/PH-AUTH-001/WP-V1-BASE-001.md`
**State:** implemented in the working tree, uncommitted.

**Change.** `editor/meridian_spec_tools/src/main.rs`: add `RETIRED_WAIVER_STATUSES` (`Closed`, `Retired`, `Superseded`, `Rejected`) and `live_expiries`/`collect_live_expiries`, which skip any object carrying a retired status whole, including nested waivers. Replace one `strings_for_keys` call in the maturity check.

**Why.** `WVR-UI-001` and `WVR-EDT-001` are both already `status: "Closed"` and both still failed governance after their `expires` date, because the check never consulted status. A retired waiver grants nothing, so its expiry is historical fact. The registry data was left untouched — backdating or deleting those records would destroy evidence to silence a tool defect.

**Evidence.** Red before the fix: `evidence/WP-V1-BASE-001-red.log` — `closed_waivers_do_not_expire` FAILED. Green after: `evidence/WP-V1-BASE-001-green.log` — 38 passed. `fmt`, `clippy -D warnings` clean.

**The question worth pressing.** Is suppression too broad? The guard is the retained `expired_waivers_are_rejected` test: its fixture waiver has no `status`, so open expired waivers are still reported. If you think an unrecognised status should also suppress, or that `Rejected` does not belong in the list, say so.

---

## WP-V1-BASE-002 — v0.5 governance must not scan v1 staging authority

**Plan:** `.meridian/implementation/plans/PH-AUTH-001/WP-V1-BASE-002.md`
**State:** diagnosed only. No production change made.

**Change.** Add `.meridian` and `MERIDIAN_SPECOMENT.md` to `is_excluded_context_path` in the same file.

**Why.** Consolidating the v1 authority into `~/meridian` put it inside the v0.5 tool's scan path. `spec check` emits 28 errors — `stale-phase-ref` against the specoment, and `unmapped-id` for v1 identifiers such as `PRG-RECON-001` and `VAL-PORTFOLIO-001` that by design do not exist in `specs/registry`. `SPEC-001` forbids old and new authority competing; a v0.5 validator judging v1 content is that competition running backwards. `.gitignore` does not help — `WalkDir` does not consult it, verified.

**Evidence.** `evidence/WP-V1-BASE-002-red.log` — 28 errors, exit 1.

**The question worth pressing.** This exclusion must not outlive the v0.5 tool. It disappears at `PH-AUTH-004` when the tool is replaced. If you would rather the v1 files live somewhere the tool never walks, that is a different and arguably cleaner fix — but it conflicts with keeping everything in `~/meridian`.

---

## Sequencing

Both touch `editor/meridian_spec_tools/src/main.rs`. `IMPL-WP-002` §7 forbids two active packages owning the same mutable seam, so run `BASE-001` to completion first, then `BASE-002`.

## Still open, and not reviewable

`DEV-003` — the review requirement itself. It closes when you record a verdict or lift the CLI restriction. Nothing else in the working tree depends on it.
