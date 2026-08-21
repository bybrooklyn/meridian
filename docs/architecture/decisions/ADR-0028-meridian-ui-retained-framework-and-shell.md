# ADR-0028: Meridian UI Retained Framework and Permanent Application Shell

- Status: Adopted
- Date: 2026-07-17
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Planned sequential packages after qualified foundations
- Owners: UI, editor, platform, accessibility, renderer
- Amends: ADR-0009, ADR-0018
- Supersedes: none

## Context

Meridian has a qualified retained UI proof and a Creator behavioral foundation,
but its current application surface is transitional. Expanding the Creator
package into an unbounded visual rewrite would mix persistence/recovery truth
with framework architecture and make cross-platform qualification difficult.
The application also needs one stable design contract before panels proliferate.

## Decision

Adopt a Meridian-owned retained UI framework with stable node identities,
incremental reconciliation, immutable frame snapshots, renderer-neutral display
lists, Meridian semantics, typed input, and versioned workspace state. Split the
work into `WP-UI-002` through `WP-UI-005`, then compose the application in
`WP-EDT-002` and `WP-EDT-003`.

The application shell permanently keeps a 44px application row and separate
36px workspace row. It uses the locked website palette, Mona Sans, restrained
Hubot Sans headings, JetBrains Mono technical text, a Meridian-owned audited
Lucide subset, a four-pixel spacing system, one-pixel borders, and hierarchical
radii. No cyan identity, decorative ring, generic chat sidebar, web production
shell, or permanent egui layer is adopted.

Windowing, rendering, fonts, icons, pickers, and accessibility libraries remain
private adapters. `RG-UI-001` must evaluate the real display-list corpus before
a production renderer decision. AccessKit maps Meridian semantics; it does not
become the public semantic model.

## Consequences

- `WP-EDT-001` remains the behavioral Creator baseline and is not expanded.
- Only one package is active; every source package requires Linux, Windows, and
  macOS evidence before the next activation.
- Runtime UI remains editor-free while `meridian-ui` provides a compatibility
  facade during crate migration.
- Incomplete domain work appears as a typed unavailable state rather than a
  simulated feature.
- Mockups precede framework code and share one generated component/token source,
  but are visual contracts rather than implementation evidence.
- Platform ownership, accessibility, Reduced Motion, high contrast, scaling,
  persistence, and recovery are package exit requirements, not polish.

## Rejected alternatives

- Big-bang replacement inside `WP-EDT-001`.
- Web-rendered production application chrome.
- Permanent immediate-mode editor architecture.
- Backend or third-party widget types in public APIs.
- Separate Studio/IDE products.
- Cyan branding, scene-responsive chrome, and decorative focus geometry.
- Selecting a display-list renderer from a toy benchmark or one platform.

## Evidence required

Registry/schema validation, generated mockup consistency, framework unit and
integration tests, UI-free runtime audits, 1×/2× output, native accessibility
adapters, renderer corpus evidence and ADR, complete editor journeys, presented
native captures, visible review, and fresh cross-platform CI for every source
package.
