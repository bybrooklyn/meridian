# Meridian UI Design Brief Review

Version: 2026-07-17

Status: non-normative reviewed production input. The normative contract is
[Editor and Meridian UI Specification](../../MERIDIAN_SPECOMENT.md).

Source reviewed: the user-supplied `MERIDIAN_UI_DESIGN_BRIEF.md`. This review
preserves useful intent while preventing concept-art language, private game
direction, or unsupported capability claims from silently becoming engine
architecture.

Disposition vocabulary:

- **Adopted**: accepted as written in substance.
- **Amended**: intent accepted with the listed binding change.
- **Rejected**: conflicts with the user-approved design or engine boundary.
- **Deferred**: valid only after an owning package or domain is ready.

## Review matrix

| Brief proposal | Disposition | Production interpretation |
|---|---|---|
| One application named Meridian | Adopted | World, Modeler, UI, Code, Materials, Alluvium, Build, Profile, VCS, diagnostics, and docs are workspaces, not separate products. |
| Design for the eventual full vision | Amended | The design language may be complete; controls must expose typed unavailable states until their domains are implemented. |
| Powerful but easier to understand | Adopted | Progressive disclosure, searchable commands, predictable layout, and source/provenance clarity are requirements. No competitor-quality claim is implied. |
| Living, morphing instrument | Adopted | Stable outer shell, remembered workspace state, bounded shared movement, and preserved selection/context. |
| Stable outer frame | Adopted | Exact 44px application row, 36px workspace row, and 24px status row. |
| Tool reveal inside current context | Adopted | Contextual first activation where useful; second activation enters a remembered focus layout. |
| Permanent shell proposed by the brief | Amended | Replaced by the exact two-row shell in the normative spec; utilities and workspaces are never merged. |
| Rail plus morphing browser | Adopted | 44/160px rail and one 264px contextual browser; pins and explicit user layouts win. |
| Adaptive right module | Adopted | The World Inspector defaults to 344px; contextual replacement is transactional and pinnable. |
| Bottom tool shelf | Adopted | 32px peek, 240px initial expansion, no focus theft on automatic reveal. |
| Premium dark charcoal/deep navy | Amended | Exact website palette; deep navy is not an identity color. |
| Glassy layered surfaces | Amended | Dense content is opaque. Bounded blur is restricted to overlays/title chrome with an opaque fallback. |
| Modular rounded surfaces | Adopted | Hierarchical 4/6/10/14px radii; code, fields, tables, and timelines remain crisp. |
| Teal/cyan-blue accents | Rejected | Conflicts with the locked website palette and user decision. |
| Glowing Meridian ring | Rejected | No ring or orb. Focus is rectangular/edge/contrast/shape based. |
| Storm-responsive ambient tint | Rejected | Product chrome does not inherit scene colors. This would destabilize contrast and import private-game art direction. |
| Project Meridian storm/VHS imagery | Rejected | Private creative content is not copied into the public engine application, fixtures, or evidence. Public mockups use abstract generic scenes. |
| Shared-element morphing | Amended | Only bounded physical relocation uses interruptible springs; Reduced Motion removes spatial movement. |
| Collapsed/Compact/Standard/Wide/Focus module states | Adopted | Stored as typed panel sizing/focus layout state with accessible minimums. |
| Smooth springs for all movement | Amended | Springs only for physical panels/shared elements; small state changes use 100–160ms transitions. |
| Pin modules | Adopted | Explicit pins outrank adaptation and learned preferences. |
| World as canonical foundation | Adopted | World is the first production workspace after framework qualification. |
| Unified left navigation | Adopted | One rail/browser system prevents duplicate hierarchy/outliner chrome. |
| Multiple placement entry points | Amended | Typed drag/drop, Add/search, context command, and command palette are planned. Natural-language search remains unavailable until its typed AGT boundary exists. |
| Minimal normal viewport overlays | Adopted | Selection, gizmo, bounds/light volumes only when relevant. |
| Deliberate Debug Visualization mode | Adopted | Debug overlays are explicit, named, and capability-owned. |
| Friendly card-based Inspector | Adopted | Advanced data is progressively disclosed per section, not hidden behind a global expert mode. |
| Inspect Source | Adopted | Source, stable IDs, command payload, provenance, invalidation, and diagnostics remain inspectable. |
| SOURCE/GENERATED/DERIVED/BUILT statuses | Adopted | Compact text/shape badges with full provenance on demand; never color alone. |
| Play Fork status and explicit Apply/Discard | Adopted | Discard remains the safe default; Apply uses an explicit semantic diff. |
| Code first contextual, then focused | Adopted | First activation beside live World; second enters remembered full IDE; Escape returns. |
| Full IDE feature list | Deferred | UI composition can exist with unavailable states; Rust analysis/debug/hot reload require their domain packages. |
| Native Modeler quick/full layouts | Adopted | Canvas-first layout with compact real-world preview and source-aware history. |
| Full topology/UV/modifier/collision/LOD capability | Deferred | `WP-MDL-001` remains Partial; UI cannot imitate later modeler behavior. |
| Alluvium compact and full workspace | Adopted | Graph, parameters, canonical source, preview, provenance, license, and diagnostics stay synchronized. |
| Visual graph as sole source | Rejected | Canonical text/parameters remain first-class and accessible; generated views are not source authority. |
| Small assistant entry points | Adopted | Assistant remains quiet until invoked. |
| Generic chat sidebar | Rejected | Review is typed proposal, scope, preview/diff, validation, Apply/Adjust/Cancel, audit, undo, and rollback. |
| Understandable VCS terminology | Adopted | Raw provider details remain available in expert inspection. |
| Auto-expanding shelf | Amended | May reveal relevant status without stealing focus; user pins and Reduced Motion apply. |
| Seven-screen concept sequence | Amended | Expanded into the required 17-state consistent SVG corpus covering hub, workspaces, recovery, contrast, scale, and widths. |

## Production conclusions

The brief’s strongest contribution is continuity: a stable shell with contextual
workspaces, preserved selection, and serious tools that do not feel like
unrelated applications. Its weakest proposals are cyan branding, a decorative
ring, scene-driven chrome, and private-game imagery. Those are explicitly
rejected.

Framework implementation must therefore begin from tokens, stable identities,
layout, display lists, semantics, and input—not from one polished concept
screen. The mockup corpus is a consistency oracle and visual review surface,
not evidence that the represented domain behavior is implemented.
