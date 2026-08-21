# Contributing to Meridian

## Contribution acceptance is paused

Meridian is transitioning from a Developer Certificate of Origin to a Contributor License
Agreement, as required by `LEGAL-003` in [`MERIDIAN_SPECOMENT.md`](MERIDIAN_SPECOMENT.md). The
CLA has been drafted but **has not completed legal review and is not in force**.

`LEGAL-MIG-001` requires that external contributions are not accepted under contradictory terms
during the transition. Retaining the old DCO alongside an accepted CLA policy would be exactly
that contradiction, so the DCO has been retired rather than kept.

**Until the CLA completes legal review, external contributions cannot be accepted.** Concretely:

- **Pull requests will not be merged.** You are welcome to open them; they will be reviewed on
  their technical merits and held.
- **Issues, discussions and security reports remain open and welcome.** Nothing here restricts
  reporting a bug or a vulnerability.
- The repository remains MPL-2.0. Forking and use under that licence are unaffected.

This is a licensing-process constraint, not a judgement about any contribution.

## What to read

[`MERIDIAN_SPECOMENT.md`](MERIDIAN_SPECOMENT.md) is the single canonical authority.
[`AGENTS.md`](AGENTS.md) is the entry point for automated contributors.
[`PLANNING.md`](PLANNING.md) reports current implementation state and is generated.

## Gates

```text
cargo run -p meridian-spec -- check
cargo run -p meridian-spec -- project --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```
