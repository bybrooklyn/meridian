# Meridian Agent Entry Point

Read [`/MERIDIAN_SPECOMENT.md`](MERIDIAN_SPECOMENT.md) completely enough to identify the active
phase, owning contracts, work-package protocol, evidence requirements and deferred boundaries.

`MERIDIAN_SPECOMENT.md` is canonical. `.meridian/implementation/state.json` is derived progress
state and is not product authority. `governance/` holds generated projections of the specoment;
every one carries the canonical digest and is regenerable with:

```text
cargo run -p meridian-spec -- project
cargo run -p meridian-spec -- project --check
```

Do not follow retired v0.5 `MS-*`/`WP-*` authority; it was removed at `PH-AUTH-004` and is
preserved only as history under tag `v0.5-final-baseline`. Do not stop at scaffolding. Do not
commit, push, merge, publish, deploy or perform destructive or external actions without
explicit authorization.
