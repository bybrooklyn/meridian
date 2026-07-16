# Editor and Meridian UI Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [ADR-0018](../docs/architecture/decisions/ADR-0018-general-purpose-single-application.md) · [Accessibility](ACCESSIBILITY_DOCUMENTATION_AND_PONDER_SPEC.md) · [Native modeler](NATIVE_MODELING_AND_DCC_SPEC.md) · [Shader language](MERIDIAN_SHADER_LANGUAGE_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Marquee](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md) · [Commands](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)

version 0.5 · 2026-07-15 · Normative · Meridian UI core proof ImplementedFoundation, egui shell Transitional

Documentation maturity: `ImplementationReady`. Implementation maturity:
`ImplementedFoundation`. Governing IDs: `REQ-UI-001`, `REQ-EDT-001`, `WP-UI-001`,
`WP-EDT-001`, `RG-UI-001`.

Current package truth: `WP-EDT-001` is `ImplementedFoundation` after GitHub
Actions run `29508496428` passed governance and Linux, Windows, and macOS rows
for `ec2a6334`. `WP-PRC-001` is active after that qualified prerequisite.
`meridian-editor-core` now owns the UI-free Creator Alpha project session,
typed transactions/inverses/checkpoints, generation-checked selection, Play
fork/apply/discard, and durable recovery. `meridian-ui-editor` declares the
accessible project, hierarchy, viewport, inspector/history, asset/import/build,
recipe, modeler, diagnostics, and recovery panels; `meridian-editor` composes
them. The public Creator Alpha smoke is also included in the qualified workspace
suite. It does not qualify a presented native surface or the modeler package.
The active Alluvium package adds textual recipe semantics and a basic inspector.

Research anchors: [AccessKit](https://accesskit.dev/) and [AccessKit Rust API](https://docs.rs/accesskit) inform accessibility adapter boundaries; [OpenXR 1.1](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html) informs later XR panel timing and composition boundaries. Meridian owns UI IR, semantics, layout, commands, and persistence.

## 1. Goals and non-goals

Meridian UI is the in-tree retained application framework shared by the editor, runtime game UI, and Meridian-native tools. The single user-facing creator application is named **Meridian**. Project management, IDE, world editor, native modeler, material/shader, animation, Alluvium, profiler, debugger, build, VCS, and future Marquee surfaces are Meridian workspaces and panels, not separate Studio/IDE applications. Shared UI means layout, text, rendering, input routing, focus, semantics, styling, animation, document binding, and diagnostics. Editor and runtime widget libraries remain separable so shipping games do not pull editor code.

Goals: responsive native desktop UI, deterministic document operations, accessible semantics, high-DPI text, virtualized large views, docking, command integration, theming, testability, and minimal runtime builds.

Non-goals: embedding egui as permanent architecture, duplicating a game UI engine, storing backend widget objects in project files, forcing web technology, or making every visual change a mutable global style operation.

## 2. Crate boundaries

- meridian-ui-core: tree, IDs, properties, events, focus, layout interfaces.
- meridian-ui-text: shaping, font fallback, line breaking, editing, IME.
- meridian-ui-render: display list, clipping, caching, renderer bridge.
- meridian-ui-semantics: accessible tree and actions.
- meridian-ui-runtime: shipping widgets and runtime document loader.
- meridian-ui-editor: docking, inspector, outliner, graphs, asset browser, diagnostics.
- meridian-editor-core: document sessions, selection, commands, undo, play mode.
- meridian-editor-egui-bootstrap: current temporary shell and migration adapters.

No egui type enters ui-core, editor-core, source documents, commands, or plugins. AccessKit is consumed only by a platform adapter from Meridian semantics. `WP-UI-001` adopts `cosmic-text` under `DEP-UI-001` only as the text shaping, fallback, layout, and rasterization adapter, and `unicode-segmentation` under `DEP-UI-002` only for extended-grapheme editing boundaries; Meridian-owned text, display-list, semantic, event, clipboard-policy, and diagnostic types remain the public boundary. Platform accessibility adapters are a separately scoped MS-03 spike, and `RG-UI-001` selects no production display-list renderer before its MS-02 entry gate.

## 3. Data model

~~~rust
pub struct UiNodeId(pub StableId);
pub struct UiDocument { schema: SchemaVersion, root: UiNodeId, nodes: NodeTable, styles: StyleSheets }
pub struct UiNode {
    id: UiNodeId,
    kind: WidgetKind,
    properties: PropertyBag,
    children: Vec<UiNodeId>,
    semantics: Semantics,
    bindings: Vec<Binding>,
}
pub struct UiFrameInput { viewport: Viewport, events: Vec<UiEvent>, time: PresentationTime }
pub struct UiFrameOutput { display: DisplayList, semantics: SemanticsDelta, commands: Vec<CommandRequest> }
~~~

Node IDs are persistent in editable documents. Runtime nodes created from repeated data use stable composite IDs. Layout/render cache handles are process-local and generation checked.

## 4. Frame pipeline

1. apply committed document/model changes;
2. reconcile logical tree and component instances;
3. resolve inherited style and state selectors;
4. shape changed text and measure intrinsic sizes;
5. run incremental constraint/layout passes;
6. build hit-test and focus indexes;
7. route queued input through capture, target, bubble;
8. execute semantic commands through the registry;
9. update animation on presentation time;
10. emit retained display-list delta and semantic-tree delta;
11. submit UI render pass and platform accessibility adapter update;
12. record invalidation, layout, shaping, draw, and event diagnostics.

Mutation during traversal is prohibited. Event handlers enqueue commands/state updates for the next reconciliation barrier.

## 5. Layout and rendering

Required layout modes: block/stack, flex row/column, grid, overlay, scroll, virtual list/tree/table, absolute canvas, and docking. Constraints are min/preferred/max plus aspect and alignment. Cycles produce a diagnostic with the constraint chain and a bounded fallback.

Rendering uses a backend-neutral display list: glyph runs, paths, rounded rectangles, images, meshes, clips, layers, and effects. Text shaping and glyph raster/cache ownership remain in UI crates; the renderer consumes immutable batches. A future Vello/native path is a measured backend option, not a public API.

Large lists MUST virtualize data and accessibility nodes. Caches are keyed by content, font/style, scale, and capability. Cache eviction is visible in diagnostics.

## 6. Text and input

Text supports Unicode shaping, bidirectional text, grapheme navigation, fallback fonts, IME composition, selection, clipboard policy, password redaction, undo, and locale-aware line breaking. Key bindings use semantic commands and context, not raw widget callbacks.

Pointer capture, hover, drag/drop, keyboard focus, gamepad navigation, touch, pen, and assistive actions converge on typed UiEvent. Runtime UI MAY disable device classes but cannot fork the semantic model.

## 7. Editor shell

Core panels:

- project/start/recovery;
- world viewport and hierarchy;
- inspector and property history;
- asset browser/import/build;
- logic/material/Alluvium/UI editors;
- native modeler, UV/material-region, collision/LOD, animation/rig, navigation, and 2D scene tools;
- Rust IDE/debug/test/profile surfaces and later optional Luau language tools;
- diagnostics, profiler, build output, tests;
- source/VCS/collaboration;
- Ponder documentation.
- post-1.0 Marquee campaign, source/rights, template, proof, approval, and export panels after `PRG-PRM-001` activation.

Panels declare ID, commands, query dependencies, layout hints, permissions, serialization version, and unavailable-state view. Workspace layout is user state, separate from project source. A corrupted layout can reset without changing project data.

Alluvium sequencing is explicit: `WP-PRC-001` first provides textual recipe
editing, typed parameter controls, preview, diagnostics, field/object/dependency/
cache/override/provenance/license inspection, and headless parity. The full
visual graph editor belongs to `WP-PRC-007`. It may not become the only source,
migration, validation, bake, recovery, or accessibility path.

The native modeler follows [its owning specification](NATIVE_MODELING_AND_DCC_SPEC.md). Meridian UI owns accessible interaction, commands, layout, and workspaces; `MDL` owns editable mesh semantics and topology lineage. Shader/material text and graphs follow [the ShaderIr authority](MERIDIAN_SHADER_LANGUAGE_SPEC.md). Animation, navigation, and 2D panels likewise edit typed source owned by their domains. UI state cannot become their hidden source authority.

## 8. Documents, commands, and undo

All edits flow through typed commands with preview, validation, transaction, inverse or checkpoint, affected stable IDs, and audit metadata. Property controls are generated from schema metadata but can be specialized without changing storage.

Play mode creates a forked runtime world. Apply-back is an explicit semantic diff; stopping play discards runtime changes by default. Asset/build operations are not smuggled into UI callbacks.

## 9. Beginner and expert workflows

Beginner: select object, edit labeled property, see immediate preview, press Undo, and receive a plain diagnostic with Fix and Learn actions.

Expert: inspect schema path, command payload, provenance, invalidation cost, and generated source; invoke the same command from CLI/Rust/MCP; pin profiler fields and compare traces.

The UI may guide but must not hide destructive scope, external service use, package cost, or permission changes.

Planned CLI and MCP operations use the same command registry:

~~~text
meridian ui check <document.mui>
meridian ui bake <document.mui> --target desktop|runtime|xr
meridian ui trace --panel <panel-id>
mcp.ui.insert_node
mcp.ui.set_property
mcp.ui.run_accessibility_check
~~~

These names are interface contracts, not implemented commands today. They are valid only after the command schema, permission policy, and undo transaction are implemented.

## 10. Persistence

Source UI documents use a versioned human-readable structure plus binary sidecars only for large immutable assets. Canonicalization preserves stable IDs and unknown optional properties. Theme/style tokens are named and versioned.

Editor workspace state stores panel layout, recent views, selections, and local preferences under user state. It is not shipped unless explicitly exported. Runtime settings and accessibility preferences have separate schemas.

## 11. Threading and memory

Logical UI mutation occurs on the owning presentation/editor thread. Background tasks may shape batches, load images, query indexes, or compile documents from immutable inputs; results are generation checked. Render submission consumes immutable display-list snapshots.

Per-frame transient allocation uses arenas. Long-lived node state is slab/arena owned by document epoch. Text/glyph/image caches have budgets and attribution. No UI work blocks the audio callback or fixed simulation.

## 12. Diagnostics and recovery

Required metrics: node count, reconciled nodes, layout roots, layout time, text shaping, glyph cache, display primitives, batches, overdraw estimate, clip/layer count, event latency, focus changes, semantic nodes, and virtualized range.

Failure examples:

- malformed UI source opens read-only with schema path diagnostics;
- plugin panel crash is isolated or disabled and the workspace reopens;
- missing font uses declared fallback and reports substitution;
- layout cycle highlights involved nodes and uses last valid geometry;
- GPU UI backend loss rebuilds caches from logical documents.

## 13. Security

Rich text and documentation do not execute script by default. Links show destination and require policy checks. Clipboard, file drop, external process, network image, agent, and package actions use capabilities. Secret fields never enter accessibility text, logs, undo snapshots, or agent context.

## 14. Tiers and zero-cost behavior

- Core runtime: text, layout, focus, semantics, basic widgets.
- Editor: docking, complex inspectors, graphs, profiling.
- Rich effects: optional path/effect backend.
- Remote/collaborative cursors: optional sync pack.

Editor widgets and bootstrap egui are absent from minimal runtime package/dependency graph. A headless build includes no UI task.

## 15. Tests and benchmarks

- golden layout fixtures over DPI, locale, fonts, and viewport sizes;
- event capture/bubble, focus, gamepad, IME, drag/drop tests;
- semantic tree/action snapshots;
- property command and undo/redo round trips;
- virtualized million-row synthetic view with bounded memory;
- glyph cache churn and display-list benchmark;
- renderer device-loss recovery;
- bootstrap migration test proving each migrated panel has no egui data dependency.

Thresholds are calibrated on the UI corpus, not invented globally.

## 16. Algorithm alternatives and research gates

| Problem | Baseline | Alternative | Gate |
|---|---|---|---|
| Layout | flex/grid/virtualized primitives | general constraint solver | Add solver only after editor-panel corpus proves need and diagnostics remain understandable. |
| Text | Meridian-owned text pipeline using focused shaping libraries behind adapters | custom shaping stack | Custom stack requires correctness tests for Unicode, bidi, IME, fallback, and selection. |
| Renderer bridge | retained display list consumed by renderer | immediate GPU widget callbacks | Raw GPU callbacks require trusted capability and measured need. |
| Editor migration | panel-by-panel egui replacement | big-bang rewrite | Big-bang migration is rejected unless bootstrap panel state can be preserved and tested. |

## 17. Delivery mapping

- MS-02: UI core proof, text/layout/semantics, first runtime overlay.
- MS-01/MS-03/MS-04: editor core and bootstrap bridge.
- MS-02/MS-03: first Meridian-native inspector/diagnostics panels.
- MS-03/MS-05: Alluvium textual recipe and basic inspector foundation.
- MS-03/MS-05: native editable-model foundation required before the Project Meridian prototype.
- MS-06/MS-07: accessible pause/settings/interaction UI for the opening slice.
- MS-04/MS-05/MS-08: migrate remaining core editor panels and remove their egui paths.
- MS-08: selected modeler, animation, navigation, 2D, ShaderIr, Rust IDE, and Alluvium visual-authoring panels after their source/headless foundations;
  delete bootstrap crate after parity, accessibility, recovery, and performance evidence.
- Post-1.0: Marquee panels use the same commands, semantics, accessibility, and recovery model; they do not alter MS-00 through MS-10 or create a separate application.

## 18. Examples

End-to-end: editing fog density in the inspector generates SetProperty, validates range and capability, commits to the weather document, invalidates only affected preview data, renders the change, and is undoable from UI or CLI.

Failure/recovery: a custom panel emits an invalid command. Validation rejects it before mutation, the UI links to the schema field, and the transaction remains clean.

Performance debug: scrolling a large asset browser shows virtual range, query latency, thumbnail decode jobs, glyph/image cache churn, and display batches. Selecting a trace span jumps to the responsible panel and query.
