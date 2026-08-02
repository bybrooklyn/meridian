# Editor and Meridian UI Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Roadmap](DELIVERY_ROADMAP.md) · [ADR-0028](../docs/architecture/decisions/ADR-0028-meridian-ui-retained-framework-and-shell.md) · [Accessibility](ACCESSIBILITY_DOCUMENTATION_AND_PONDER_SPEC.md) · [Commands](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md) · [Reviewed design brief](../docs/production/MERIDIAN_UI_DESIGN_BRIEF_REVIEW.md)

version 0.5 · 2026-07-17 · Normative · ImplementationReady

Implementation maturity: `ImplementedFoundation` for `WP-UI-001` and the
qualified Creator behavioral baseline; the Meridian UI 1.0 framework and final
workspace composition remain sequential package work. Governing IDs:
`REQ-UI-001`, `REQ-UI-002`, `REQ-EDT-001`, `REQ-EDT-002`, `WP-UI-001` through
`WP-UI-006`, `WP-EDT-001` through `WP-EDT-003`, and `RG-UI-001`.

This document owns Meridian application/editor UI architecture, design tokens,
components, input behavior, accessibility, workspace composition, responsive
behavior, persistence, recovery, and platform integration. Typed registries
make its fixed contracts machine-verifiable. Domain specifications continue to
own world, model, procedural, material, build, profiling, and gameplay source
semantics; UI does not manufacture capabilities those domains have not earned.

## 1. Product contract

The product is one native application named **Meridian**. World editing,
modeling, UI authoring, code, materials, Alluvium, build, profiling, VCS,
diagnostics, documentation, and later tools are workspaces in that application,
not separately branded Studio or IDE products.

Meridian UI is the in-tree retained framework shared by the application,
editor-only controls, runtime game UI, and Meridian-native tools. It owns the
logical UI document, stable node identity, reconciliation, layout, input,
focus, semantics, styling, motion, renderer-neutral display list, workspace
state, diagnostics, and recovery contracts. Adapter libraries remain private.

Goals are a modern native desktop experience, deterministic document
operations, accessible semantics, excellent high-DPI text, virtualized
professional controls, transactional docking, testability, and a small runtime
profile. Non-goals are a web production shell, permanent egui architecture,
backend widget objects in source, decorative brand geometry, a generic chat
sidebar, or hidden domain authority.

## 2. Permanent application shell

Every normal application workspace uses two distinct rows:

~~~text
┌────────────────────────────────────────────────────────────────────┐
│ ● ● ●  Meridian · Project       Play · Build · Search · Settings   │
├────────────────────────────────────────────────────────────────────┤
│ World  Modeler  UI  Code  Materials  Alluvium  Build  Profile      │
└────────────────────────────────────────────────────────────────────┘
~~~

The application row is 44 logical pixels, the workspace row is 36, and the
permanent status row is 24. They never merge at supported widths. When space is
tight, utility labels compress, workspace tabs enter a controlled overflow,
and lower-priority regions collapse before the canvas. Play is the strongest
utility; Build is visibly stateful; Search stays visible; Settings remains
quiet. No window size may reduce an interactive target below its accessible
minimum.

On macOS the application respects traffic-light placement, title-bar drag
regions, native menus, native fullscreen, platform focus rules, and native file
pickers. Window controls are not imitated in content. Other platforms retain
their native ownership and behavior. Product-specific settings, diagnostics,
recovery, import details, and commands use Meridian UI.

Panel headers are restrained: context and provenance on the left; essential
actions and a single overflow affordance on the right. Repeated chrome cannot
be louder than the working surface.

## 3. Locked design system

### 3.1 Color roles

The default dark theme uses the website palette exactly:

| Role | Value | Use |
|---|---:|---|
| Background | `#090b0b` | application and deepest canvas surround |
| Surface | `#121515` | panels and dense opaque content |
| Border | `#292d2c` | one-pixel separators and control outlines |
| Primary text | `#e3e1d8` | labels and readable content |
| Secondary text | `#929790` | supporting content |
| Muted | `#686e68` | passive metadata and disabled content |
| Destructive | `#a73732` | destructive intent |
| Destructive hover | `#c04b44` | hovered destructive intent |
| Positive | `#8d8961` | success and safe affirmative state |
| Warning/emphasis | `#c0964e` | warning, emphasis, active Build state |

There is no cyan/teal identity, decorative glowing ring, orb, scene-derived
ambient tint over product chrome, or color-only state. Selection, focus,
warnings, source authority, and destructive actions combine contrast with
shape, text, or edge indicators.

High Contrast resolves these same semantic roles through the registered
`token.color.high-contrast.*` mapping. Focus keeps a rectangular outline,
selection adds a leading edge, and invalid state adds a heavier outline plus a
bottom edge, so none of those states depends on color discrimination.

### 3.2 Typography and icons

Mona Sans is the interface face. Hubot Sans is limited to restrained display
headings. JetBrains Mono is used for code, logs, identifiers, measurements, and
technical tabular data. Fallbacks are declared and substitution is diagnosed.
Fonts are pinned, hashed, licensed, and packaged only in their owning source
slice.

Icons use a pinned audited Lucide SVG subset behind Meridian-owned `IconId`
values, plus necessary custom domain icons. No third-party SVG parser or icon
type enters a public API. Every shipped asset has an exact source, revision,
hash, SPDX identity, notice, modification record, tests, and update strategy.
Icon size, stroke width, and icon-to-text gap resolve from the active theme's
registered tokens rather than inheriting text size or the default theme.

### 3.3 Geometry, density, and effects

Spacing uses a four-pixel base. Borders are one logical pixel. Dock gutters are
eight. Radii are hierarchical: 4 for compact controls, 6 for fields and small
cards, 10 for panels and menus, and 14 for large floating surfaces. Dense code,
tables, property grids, and timelines remain crisp rather than becoming a grid
of rounded capsules.

Default shell geometry is:

| Region | Logical size |
|---|---:|
| Application row | 44 px |
| Workspace row | 36 px |
| Status row | 24 px |
| Activity rail | 44 px collapsed; 160 px expanded |
| Browser | 264 px |
| World Inspector | 344 px |
| Bottom shelf | 32 px peek; 240 px initial expanded |

Dense content is opaque. GPU blur is allowed only for floating overlays and
title chrome, is bounded in area and layer count, and always has an opaque
fallback. Shadows communicate hierarchy; they do not replace boundaries.

## 4. Crate and dependency boundaries

- `meridian-ui-core`: retained document, stable IDs, properties, state,
  reconciliation, layout contracts, focus, commands, input, preferences.
- `meridian-ui-text`: shaping, fallback, line breaking, editing, selection,
  clipboard requests, IME, and text cache contracts.
- `meridian-ui-semantics`: Meridian semantic roles, actions, reading order,
  live regions, and platform-neutral accessible tree.
- `meridian-ui-render`: display list, clipping, layers, effects, cache keys,
  renderer bridge, diagnostics, and immutable frame snapshots.
- `meridian-ui-runtime`: shipping controls and runtime document loading without
  editor dependencies.
- `meridian-ui`: compatibility facade while callers migrate.
- `meridian-ui-editor`: editor-only docking, complex controls, workspaces, and
  platform adapter composition.
- `meridian-editor-core`: source-authoritative project sessions, typed command
  transactions, selection, undo, Play forks, persistence, and recovery.
- `meridian-editor`: native application composition and private adapters.

Runtime crates never depend on editor controls, picker adapters, or bootstrap
UI. egui remains a bounded transitional bootstrap until parity evidence allows
its deletion. AccessKit, windowing, rendering, font, icon, picker, and platform
types stop at private adapters.

## 5. Public Meridian interfaces

Public APIs use Meridian-owned types. Exact representations may evolve through
compatible package work, but these responsibilities may not be omitted:

~~~rust
pub struct UiNodeId(pub StableId);
pub struct UiDocument { /* schema, root, nodes, styles */ }
pub struct UiFrameInput { /* viewport, device events, frame-boundary timing */ }
pub struct UiFrameSnapshot { /* immutable layout, display, semantics */ }
pub struct ThemeId(pub StableId);
pub struct TokenId(pub StableId);
pub enum UiDensity { Compact, Standard, Comfortable }
pub enum UiContrast { Standard, High }
pub enum MotionPreference { Full, Reduced }
pub enum LayoutMode { Flex, Grid, Overlay, Absolute, Scroll }
pub enum DevicePhase { Press, Move, Release, Cancel }
pub enum ScrollUnit { Pixel, Line }
pub struct FocusId(pub StableId);
pub struct CommandId(pub StableId);
pub struct UiAssistiveRequest { /* target and Meridian-owned semantic action */ }
pub struct SemanticNode { /* role, name, value, state, actions */ }
pub struct DisplayList { /* renderer-neutral ordered primitives */ }
pub enum DisplayPrimitive { RoundedRect, Path, GlyphRun, Image, Mesh, Clip, Layer, Shadow, Backdrop }
pub struct DockTree { /* split, tab, floating and collapsed nodes */ }
pub struct PanelId(pub StableId);
pub struct WorkspaceLayout { /* versioned named layout and region state */ }
~~~

Command names are canonical ASCII identifiers validated at the retained
document boundary before a frame can observe them: the first byte is
alphanumeric, later bytes may be alphanumeric or `.`, `:`, `-`, `_`, or `/`,
and the UTF-8 representation is bounded to 256 bytes. Runtime activation
emits the typed `CommandId`; it never executes an unvalidated action string.

Public text-input construction and the direct text-shaping adapter independently
enforce the retained `MAX_TEXT_BYTES` limit before retaining source or invoking
their private implementation. An oversized direct request is a typed rejection;
a future corrupted or incremental document therefore recovers its prior
immutable frame rather than discarding source or allocating inside an adapter.

Input also has typed pointer IDs, positions, buttons, modifiers, capture,
scroll momentum, text/composition events, drag payload descriptors, and device
classes. Optional event source timestamps are indexed, bounded, and comparable
only against a declared reconciliation boundary in the same monotonic epoch.
Editor interfaces include panel sizing, preview/pinned tabs, focus layouts,
companion-window identity, and versioned workspace-state persistence.
Application-local hub preferences are a separate versioned document. They may
migrate prior local-only preference state atomically, but can never alter or
stand in for authoritative project source.

## 6. Retained document and frame pipeline

`UiDocument` is the versioned canonical editable UI source. It owns the schema
version, root, stable nodes, named authored styles, reusable component
definitions and explicit component instances, token references, semantic
bindings, and bounded packaged raster-asset references. Repeated runtime nodes
use stable composite identities. The compiled frame, display list, glyph atlas,
GPU caches, and process-local image handles are derived and rebuildable.

Authoring APIs must make the locked system easier to use than raw renderer-like
construction: typed four-pixel spacing, registered 4/6/10/14 radii, layout
helpers, named style/variant references, component instantiation with explicit
stable IDs, and diagnostics that name the source node and property. Legacy raw
constructors remain only as migration and fixture compatibility. Packaged
rasters use stable `UiAssetRef` references that a private resource adapter
lowers to process-local image handles before an authored image node emits a
renderer-neutral `Image` primitive. A missing or wrong-kind asset is a typed
unavailable result and cannot partially advance the retained frame. Vectors are bounded native paths or
audited `IconId` values; source documents never contain loose paths, generic
SVG, backend handles, or executable content.

Persisted `UiDocument` source uses a bounded versioned envelope separate from
the document schema. Decode validates size, envelope version, stable source
identities, and the entire retained document before a frame can compile. The
only accepted legacy shape is the pre-envelope direct source snapshot; it
migrates in memory and is re-emitted in the current envelope by the next
successful authoritative write. Renderer resources, display lists, glyph
atlases, and caches are never serialized.

Reconciliation compares immutable prior and accepted logical state, retains
identity where semantics match, resolves authored tokens/styles/components, and
reports duplicate or unstable keys before a frame can observe them.

Each frame:

1. accepts committed document and model changes;
2. incrementally reconciles logical nodes and component instances;
3. resolves tokens, inherited styles, variants, and state selectors;
4. shapes changed text and measures intrinsic size;
5. computes incremental layout and clips;
6. builds hit-test, focus, and semantic indexes;
7. routes queued input through capture, target, and bubble;
8. enqueues typed commands/state updates for the next barrier;
9. advances bounded presentation motion;
10. emits an immutable display list and semantic delta;
11. submits renderer and accessibility adapter updates;
12. records layout, shaping, drawing, event, focus, and cache diagnostics.

Mutation during traversal is prohibited. A rejected command or layout update
preserves the last accepted snapshot.

## 7. Layout, display list, and rendering

Required layout modes are Flex row/column, Grid, Overlay, Absolute, and Scroll.
They support minimum/preferred/maximum constraints, alignment, aspect, padding,
gap, and clipping. Constraint cycles emit the involved chain and use the last
accepted geometry or a bounded fallback.

The renderer-neutral display list supports rounded rectangles, paths, glyph
runs, images, meshes, nested clips, layers, shadows, and bounded backdrop
effects. It contains no backend resource or command encoder. Cache keys include
content, token/font state, scale, contrast, and renderer capability. Device loss
rebuilds caches from logical state. Accepted frame snapshots include bounded
diagnostics for layout nodes, display primitives, semantic nodes, routed
effects, text/control requests, scale, contrast, motion, and whether a rejected
frame recovered the previous immutable snapshot. The RHI exposes a
Meridian-owned render identity so UI renderer caches can detect device,
surface, format, size, and configured-state changes without observing backend
types.

UI token and image color is authored as sRGB with linear alpha. Production
render adapters decode authored RGB to linear light exactly once, composite
premultiplied-alpha content and isolated layers, and encode through an sRGB
target. A direct adapter that cannot obtain a compatible sRGB target rejects
that path explicitly instead of presenting misencoded color. Direct image
resources are bounded straight-alpha RGBA8 sRGB inputs; premultiplication occurs
exactly once at the shader/blend boundary.

Direct atlas preparation reuses byte-identical glyph and image payloads only
after collision-safe content comparison. Vertex/index geometry, individual
image sources, and the aggregate RGBA atlas have distinct typed bounds; the
atlas shares the registered full-frame RGBA service guard rather than borrowing
the geometry budget. Valid zero-area glyph masks, such as whitespace, allocate
no atlas region or draw geometry while retaining their text primitive and
semantic identity.

Axis-aligned control, image, glyph, clip, and focus geometry snaps to physical
pixel edges before NDC conversion. Rounded content uses adaptive physical-radius
tessellation and a one-physical-pixel coverage fringe; freeform curves flatten
against physical-pixel error, and declared joins and caps retain their actual
geometry. Shadow spread uses bounded alpha falloff rather than a solid expanded
block. The bounded backdrop path uses a fixed 3x3 tent kernel over a reconstructed
parent-prefix target, requires at least one physical texel of declared sample
padding on every edge (`1 / scale_factor` logical pixels), shares the registered
aggregate offscreen-target guard, and preserves an opaque High Contrast or
unsupported-capability fallback. Negative shadow spread is invalid geometry.

`RG-UI-001` evaluated the real editor/runtime display-list contract and is
decided by `ADR-0029`: a Penumbra-owned direct GPU consumer is the production
direction, while the bounded full-frame CPU raster bridge remains structural
and recovery-only. The decision does not claim the direct path complete or
establish visual/performance qualification. Corpus correctness, text quality,
accessibility compatibility, latency, memory, recovery, and platform evidence
remain required before implementation promotion.

## 8. Input and activation

Pointer press, move, release, capture, cancel, and activation are distinct.
Press may capture a target; activation occurs only on a valid release over the
appropriate enabled target. Focus loss, modal takeover, device removal, or
escape cancels capture and any preview transaction. Hover is never activation.

Focus uses stable semantic identity, not row index. Filtering, virtualization,
Play transitions, reconciliation, workspace changes, and companion-window moves
restore the same focus where it remains valid; otherwise focus moves by a
documented deterministic rule.

Visible focus-owning transient surfaces (combo boxes, menus, context menus, and
the command palette) scope keyboard and assistive focus to their retained
subtree. The first pointer press outside the top surface dismisses that surface
without activating the underlying control; dismissal restores the recorded
focus target or uses the normal deterministic recovery rule.
Programmatic focus and host-delivered target interactions—value edits,
property/timeline/canvas actions, drag/drop, and scroll targeting—are confined
to the same scope or reject without mutating retained or source authority. A
new focus-owning transient cancels prior pointer, scroll, drag, timeline-scrub,
and canvas-preview capture before it takes focus. Document replacement restores
an invalidated focus target to the visible top transient before considering a
background control; unrealized collection selection remains recoverable without
weakening that modal rule.
Its assistive projection retains the document root plus only that active
transient subtree, reparenting a nested surface to the nearest retained
structural ancestor. Background controls and their relationships are absent
until the focus-owning transient closes.
Each such surface must retain at least one enabled focusable descendant; an
empty focus-owning surface is rejected before it can enter a frame.

Precise trackpad pixel deltas and platform momentum are preserved. Discrete
wheel lines are normalized separately. A gesture locks its intended target;
nested scroll regions hand remaining delta to an ancestor at bounds. Meridian
does not apply a second smoothing curve over OS momentum.

Controller navigation follows the same focus graph and semantic commands as
keyboard navigation. Device-specific bindings cannot fork command meaning.

## 9. Text, IME, clipboard, and editing

Text supports Unicode shaping, bidirectional content, grapheme navigation,
fallback, line breaking, IME composition/commit/cancel, selection, validation,
completion, editing commands, password redaction, and bounded undo. Composition
state follows the focused editor and platform candidate-window location.

Clipboard access is explicit policy-mediated read/write. If no platform adapter
exists, the command returns a truthful diagnostic; success is never fabricated.
Secret fields do not enter the clipboard, semantic tree, logs, undo snapshots,
or agent context.

## 10. Drag, drop, and professional controls

Drag/drop uses typed payloads, accepted-operation negotiation, preview,
auto-scroll, cancellation, transaction commit, rollback, and a keyboard
alternative. Raw file paths, process commands, or arbitrary serialized objects
are not accepted as internal authority.

The professional component set includes buttons, icon buttons, toggles, fields,
search, combo boxes, menus, menu bars, context menus, tooltips, toasts, tabs,
trees, tables, property grids, virtual lists, timelines, splitters, progress,
command palette, and graph/canvas primitives. Each component registry entry
defines states, semantics, input, motion, keyboard behavior, disabled behavior,
validation, and ownership. Components remain composable; workspaces do not
fork look-alike private controls.

## 11. Docking, workspaces, and companion windows

Editor docking supports split, tab, floating, collapsed, and maximized nodes;
preview tabs; pinning; reorder; tear-off; minimum sizes; reset; and
transactional rollback. A failed move never loses a panel. Named layouts are
versioned and migratable. Corruption, unknown versions, missing panels, and
monitor loss recover visibly without changing project source.

One primary application frame may own session-sharing native companion windows.
Companions use the same command/session authority, can re-dock, and restore to a
visible monitor. They are not separate product instances.

Workspace state preserves selection, active document, camera, browser query,
tree expansion, scroll, panel pins, focus layout, and companion placement.
Explicit pins and named user layouts outrank rule-based adaptation or learned
preferences. Reset, history, migration, and recovery are always available.

## 12. Responsive behavior

Supported layouts preserve the permanent two-row shell and working canvas.
Adaptation first tightens nonessential spacing, then shortens low-priority
labels, then collapses low-priority regions, then uses controlled workspace
overflow. It never overlays dense panels indiscriminately, shrinks targets
below accessible sizes, hides active errors, or merges application and workspace
rows.

The default activity rail is 44 pixels and may expand to 160. The World browser
is compact at 264; its Inspector is deliberately wider at 344 because it owns
detailed editing controls. The viewport receives remaining space. A bottom
shelf peeks at 32 and initially expands to 240. At 100–400% text scaling,
regions reflow, scroll, or collapse according to their registry priorities.

## 13. Motion and effects

Interruptible springs communicate physical panel movement and shared-element
relocation only. Hover, focus, selection, color, and opacity transitions are
restrained to 100–160ms. Reversal begins from current presentation state;
interruption cannot leave invisible input surfaces or stale hit regions.

Reduced Motion removes spatial animation, applies layout immediately, and may
use only a brief opacity transition. High contrast disables nonessential blur,
strengthens boundaries, and preserves semantic hierarchy without creating a
separate component system.

## 14. Accessibility contract

Meridian owns semantic roles, names, descriptions, values, states, actions,
relationships, reading order, live regions, and focus. A private AccessKit
adapter maps accepted semantic snapshots to each supported platform.

Every visible workflow has keyboard operation, pane cycling, focus restoration,
Home/End/Page behavior where applicable, accessible names, non-color state,
error recovery, high-contrast behavior, text scaling, and Reduced Motion. The
visual focus indicator is a rectangular outline, edge indicator, contrast, or
shape change—never a decorative ring.

Advertised assistive actions are executable contracts, not descriptive metadata.
Focus, activation, value editing, expansion/collapse, increment/decrement, and
context-menu requests route through Meridian-owned events and typed frame
effects. `ScrollIntoView` is handled by the retained runtime against the
nearest scroll ancestor; the remaining actions become bounded
`UiAssistiveRequest` values for the host command/session adapter. Every
advertised host-bound action names its own canonical command binding; an
adapter must never substitute the target's ordinary activation command.
In particular, a context-menu binding neither requires nor implies ordinary
primary activation.
Unsupported or malformed requests produce diagnostics and cannot mutate source
authority.

Virtualization keeps the active/focused semantic neighborhood available and
reports collection position/size without instantiating millions of nodes.
Asynchronous build, import, validation, and recovery status uses bounded live
regions that do not repeatedly steal focus.

## 15. Canonical workspace composition

| Workspace | Canonical composition and truth boundary |
|---|---|
| Hub | Create, Open, bounded Recents, invalid-recent remediation, recovery, and native picker actions; no project opens implicitly. |
| World | Slim rail, compact browser, dominant live viewport, wider Inspector, bottom shelf, permanent status. Debug overlays are deliberate, not permanent labels over the scene. |
| Code | First activation opens beside the live viewport for contextual edits; second activation enters its remembered full IDE layout. Same action or Escape returns. |
| Modeler | Dominant modeling canvas; tools/structure left; properties/history right; compact real-world preview. Incomplete topology/UV/modifier features are typed unavailable states. |
| UI | Hierarchy/components left, visual canvas center, properties/states right, responsive/state/animation tools below. |
| Materials | Synchronized graph, parameters, readable source, and visual preview. All lower to the domain-owned material/shader authority. |
| Alluvium | Synchronized recipe graph, parameters, canonical source, generated result, provenance, license status, and diagnostics. Text and parameters remain first-class. |
| Build | Dense tasks, artifact tables, logs, filters, progress, recovery, and comparison using asynchronous BLD authority. |
| Profile | Timelines, traces, flame graphs, counters, filters, drill-down, and comparison; unsupported capture data is labeled unavailable. |
| Settings | Searchable product preferences, theme/accessibility/input/workspace controls, reset/history, and platform-owned links where required. |
| Recovery | Source authority, autosave/checkpoint state, affected scope, safe choices, diagnostics, and keyboard-first restoration. |

Workspace switching preserves context and morphs between remembered layouts.
Second activation enters a remembered focus layout. A polished workspace may
show an explicit typed unavailable state for an incomplete domain, but cannot
simulate that domain or present planned controls as operational.

## 16. Assistant behavior

The assistant is quiet until explicitly invoked. It is not a permanent generic
chat sidebar. It operates through the same typed commands, permissions,
selection, transaction, preview, validation, audit, undo, and rollback as human
tools. The review surface prioritizes the request, Meridian’s typed
interpretation, visual/source diff, affected scope, validation, and Apply,
Adjust, or Cancel. Conversation is secondary to the work.

## 17. Persistence, source authority, and recovery

Project/editor source documents are versioned, canonical, bounded, validated,
and atomically written. An accepted edit validates, commits the in-memory
transaction, writes authoritative source, then updates recovery state. Write or
recovery failure restores the in-memory transaction and preserves the last
accepted source authority.

Workspace layouts and local preferences are user state, not project source.
They are independently versioned, atomically written, and recoverable. Unknown
optional fields survive compatible round trips. Derived previews, display
lists, layout caches, glyphs, and compiled artifacts are rebuildable caches.

Play creates a runtime fork. Apply-back is an explicit semantic diff; Discard is
the safe default. UI state cannot silently change source authority.

## 18. Security and platform integration

Project, import, source, layout, font, icon, clipboard, drop, link, build, and
agent input is untrusted. Validate type and limits before allocation. Rich text
does not execute script. Links expose destination and pass policy. File/process/
network actions use explicit capabilities and typed commands. No shell string,
ambient filesystem authority, secret, or third-party object crosses a public
UI boundary.

Native menus, pickers, fullscreen, window chrome, screen-reader connection,
clipboard, IME, and drag sources are platform-owned workflows behind Meridian
adapters. Product workflows remain consistent Meridian UI.

## 19. Diagnostics and performance evidence

Required diagnostics include accepted/reconciled node count, layout roots and
time, shaping work, glyph/image cache state, display primitives and batches,
clips/layers/effects, overdraw estimate, event latency, capture state, focus
changes, semantic nodes, virtualized ranges, animation count, and recovery
events. Thresholds are calibrated on the registered UI corpus; no global number
is invented in this specification.

Every accepted retained frame reports saturated monotonic nanosecond durations
for the Meridian-owned reconciliation, layout, text-shaping, text-rasterization,
display-validation, and semantic-delta phases. Layout includes intrinsic text
measurement, so phase values may overlap and must not be summed into a total.
Source-to-reconciliation-boundary event latency is available only when the
platform adapter supplies a bounded event-index side table and one frame-boundary
timestamp from the same monotonic epoch. Index order, source-time order, bounds,
and source times after that boundary are rejected before retained state changes.
Partial timestamp coverage is reported explicitly by count; untimestamped frames
remain `Unavailable`. The presentation interval is never reinterpreted as
latency, and true input-to-presented-surface latency remains platform/renderer
evidence because presentation occurs after retained reconciliation.

The framework supports deterministic 1× and 2× snapshots, logical/physical
scale separation, device loss, font substitution, corrupt-state recovery, and
display-list replay. Visual evidence must be presented; headless or occluded
submission cannot claim visual quality.

`WP-UI-005` has four separate, non-promoting native evidence runners for the
direct display-list path. The hidden qualification runner captures only an
opt-in copy-source offscreen target, emits raw RGBA8 plus PNG/metadata, and
compares all current corpus cases exactly against a versioned fixture. A fixture
binds its schema, generator identity, input schema, corpus hashes, unique case
IDs, dimensions, raw hash, and complete RHI profile (backend, adapter, driver,
vendor/device IDs, surface format, OS, and architecture). Report-level `Pass`
is a runner outcome only and requires every required case to pass. Missing or
different profiles are `NotRun`; an exact-profile pixel mismatch is `Fail` and
leaves a durable machine-readable failure report. Evidence source checkpoints
are required, path-free identifiers; portable reports contain only relative
artifact names. The source state and checkpoint are caller-declared labels, not
trusted source attestation. Local or dirty reports remain `Inconclusive`
qualification evidence and cannot promote `WP-UI-005`.

The hidden device-loss runner invokes the non-default, test-only controlled
`wgpu::Device::destroy` seam, waits for the real `Destroyed` callback, verifies
typed rejection of old submission and stale identity after RHI rebuild, then
replays the immutable corpus and compares baseline/recovered pixels exactly.
It records baseline/recovered hardware/configuration profiles and render
identities. This is controlled destruction only: it does not establish
hardware, driver, power, or spontaneous-loss behavior.

The hidden performance runner records sequential raw JSONL samples for distinct
resource-setup and steady-reuse modes, plus wall-clock preparation/upload/
submission/readback-wait observations, typed RHI timing outcomes, capture
diagnostics, and exact payload accounting. It records unavailable backend
allocation, VRAM, driver-residency, and unsupported/inconclusive timing as
such. It sets no numeric threshold, does not convert readback wait into GPU or
interactive latency, and is not calibrated performance evidence.

The visible presented-review runner maps a native window at the canonical 2x
corpus size, requests advisory platform focus without assuming it succeeds,
and writes PNG, raw RGBA, metadata, hashes, and a profile-bound report only
after `Presented` or `PresentedSuboptimal` surface readback succeeds. Occluded
or unavailable presentation remains a durable `Inconclusive` failure. Runner
success leaves `review_status` as `AwaitingHumanReview`; it cannot manufacture
a visual-quality verdict. None of the four runners replaces real screen-reader,
accessibility, calibrated-performance, or cross-platform qualification.

The separate `ui_accessibility_review` runner keeps a bounded native AccessKit
fixture alive for an explicit review interval and records only actions delivered
through the private platform adapter. Its semantic tree covers reading order,
an editable value, relationships, expandable state, typed activation, progress,
focus, and a live region. Action payload text is redacted from evidence. A
timeout without a real assistive-client action is `NotRun`; an observed action
remains `Inconclusive` until the reviewer confirms spoken names, values, states,
ordering, live updates, and focus recovery. Synthetic adapter calls cannot
satisfy this contract.

## 20. Delivery packages and completion

The rewrite is sequential and has one active package at a time:

| Package | Deliverable | Dependency |
|---|---|---|
| `WP-UI-002` | Modular retained core, locked tokens, typography/icon contracts, layout, display list, basic controls | `WP-UI-001` |
| `WP-UI-003` | Input, scrolling, text/IME, drag/drop, professional controls, virtualization | `WP-UI-002` |
| `WP-UI-004` | Docking, workspaces, responsive layouts, persistence, companion windows | `WP-UI-003` |
| `WP-UI-005` | Motion, effects, high contrast, platform accessibility, renderer qualification | `WP-UI-004`, `RG-UI-001` |
| `WP-UI-006` | Ergonomic versioned `UiDocument` authoring, source-to-frame compilation, and packaged-asset lowering | `WP-UI-005` |
| `WP-EDT-002` | Exact application shell, hub, and production-quality World workspace | `WP-EDT-001`, `WP-UI-006` |
| `WP-EDT-003` | Remaining current Creator workspace composition and cross-workspace consistency | `WP-EDT-002`, `WP-UI-006`, and applicable domain foundations |

`WP-EDT-001` remains the behavioral baseline for project/session persistence,
recovery, Creator commands, and build integration. It is not expanded into the
framework rewrite. `WP-MDL-001` remains `Partial` until its broader scope is
complete.

Each package runs schema/token/mockup checks; targeted tests; locked metadata;
format; full workspace tests; warning-denied Clippy; UI, editor, RHI, renderer,
and dependency-boundary smokes; supported native/accessibility checks; and
`git diff --check`. Every pushed source package requires Linux, Windows, and
macOS CI before evidence and the next activation are recorded.

The interaction matrix covers layout rejection, press/release/cancel/capture,
line/pixel/momentum and nested scrolling, IME, clipboard, focus stability,
drag rollback, virtualization, docking migration/corruption/monitor loss,
animation interruption, semantic adapters, 100–400% scaling, high contrast,
Reduced Motion, and 1×/2× output.

The editor journey covers create/open/import/edit/undo/redo, Play Apply/Discard,
workspace/focus switching, Code contextual/full activation, recipe/model
actions, asynchronous build/artifacts, crash recovery, and explicit exit.
`WP-EDT-002` resumes only after the active `WP-UI-006` source package. Then
`WP-EDT-003` composes Hub, Settings, contextual/focused Code, Modeler, UI,
Materials, Alluvium, Build, Profile, and Recovery from the same authored
component vocabulary. The initial UI workspace inspects `UiDocument` source,
component tree, styles/tokens, assets, responsive states, and compiled preview;
direct-manipulation canvas editing is deferred to its own later bounded package.
`MS-03` stays open until the Creator packages, native evidence, accessibility
review, and visible application approval pass. Workspace presentation never
closes a domain package.
