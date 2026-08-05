# Contributing to Meridian

## Developer Certificate of Origin

Every contribution to this repository must be signed off under the
[Developer Certificate of Origin (DCO)](DCO.md). Signing off certifies that
you wrote the contribution, or otherwise have the right to submit it under
this project's license ([`LICENSE-MPL-2.0`](LICENSE-MPL-2.0)).

Add a `Signed-off-by` line to every commit message:

```
Signed-off-by: Your Name <your.email@example.com>
```

Git adds this automatically with the `-s` flag:

```
git commit -s -m "Your commit message"
```

The name and email must match a real identity that can be credited for the
contribution — no anonymous or pseudonymous sign-offs. Pull requests with
unsigned commits will be asked to amend before merge.

## Where to start

Read [`specs/MERIDIAN_MASTER_SPEC.md`](specs/MERIDIAN_MASTER_SPEC.md) and
[`PLANNING.md`](PLANNING.md) first — they own current architecture and
implementation truth respectively. `specs/IMPLEMENTATION_PLANNING_SPEC.md`
explains how a `WP-*` work package moves from `Planned` to active; the
[`specs/registry/`](specs/registry/) directory holds the machine-checked
identifiers a change must stay consistent with. Run
`cargo run -p meridian-spec -- check` before submitting any change under
`specs/`.
