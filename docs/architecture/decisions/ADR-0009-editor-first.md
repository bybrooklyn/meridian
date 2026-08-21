# ADR-0009: Editor-first Product Architecture

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Qualified Creator behavioral foundation; Meridian UI 1.0 sequential packages active/planned
- Owners: meridian-editor-core, meridian-ui, meridian-editor-egui-bootstrap
- Supersedes: none
- Superseded by: none

Amendment notice: [ADR-0017](ADR-0017-alluvium.md) applies this command/schema parity to Alluvium textual recipes, headless execution, the basic inspector, and the later visual graph editor. [ADR-0028](ADR-0028-meridian-ui-retained-framework-and-shell.md) owns the retained-framework package sequence, locked design system, permanent shell, and adapter boundaries.

## Context

Meridian is meant for creators who can open, edit, play, recover, and export
without learning internal engine details. At the same time, expert workflows
need schemas, CLI, traces, and Rust APIs.

## Decision

Meridian is editor-first, not editor-only. The editor uses the same typed
commands, schemas, diagnostics, build graph, and validation as CLI, Rust tools,
MCP, and future agents. The current egui shell is transitional. The permanent
Meridian UI framework owns retained UI documents, layout, text, rendering,
focus, semantics, accessibility, commands, and persistence.

## Current Evidence

- [Editor and Meridian UI spec](../../../MERIDIAN_SPECOMENT.md)
- [Planning ledger](../../../PLANNING.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Editor workflows cannot bypass schemas or commands. Shipping runtime builds
must not pull editor panels or egui unless explicitly enabled. Accessibility and
recovery are part of the architecture, not release polish.

## Status Review

Review after the first Meridian-native editor panel replaces its egui path with
tested command, accessibility, recovery, and performance evidence.
