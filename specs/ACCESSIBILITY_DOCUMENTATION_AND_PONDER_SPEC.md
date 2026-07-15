# Accessibility, Documentation, and Ponder Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [UI](EDITOR_AND_MERIDIAN_UI_SPEC.md) · [Agents](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)

Version 0.2 · 2026-07-14 · Normative · Planned

Research anchors: [AccessKit](https://accesskit.dev/), [AccessKit repository](https://github.com/AccessKit/accesskit), and [AccessKit Rust API](https://docs.rs/accesskit). These references support adapter terminology only; Meridian owns semantic intent, validation, documentation records, and Ponder playback.

## 1. Purpose

Accessibility and contextual learning are continuous product contracts. Meridian owns semantic intent; platform adapters expose it. Ponder is the local-first documentation and learning system that connects concepts, current context, diagnostics, examples, and safe actions.

Non-goals: treating accessibility as platform-tree generation only, replacing documentation with an AI chat, uploading project content by default, or hiding errors behind tutorial prose.

## 2. Ownership

- meridian-ui-semantics owns roles, names, values, relationships, state, actions, navigation, live regions, and text ranges.
- platform accessibility adapters map the Meridian tree to AccessKit/native APIs.
- meridian-ponder owns documentation bundles, indexing, context resolution, tours, examples, and Learn actions.
- command registry owns executable fixes; Ponder may propose or invoke only declared commands under normal permissions.
- game/editor systems provide domain semantics and documentation IDs.

## 3. Semantic model

~~~rust
pub struct SemanticNode {
    id: SemanticId,
    role: Role,
    name: LocalizedText,
    description: Option<LocalizedText>,
    value: Option<SemanticValue>,
    state: SemanticState,
    relations: Relations,
    actions: ActionSet,
    bounds: Option<LogicalRect>,
    text: Option<TextSemantics>,
}
~~~

Semantic IDs remain stable across incremental UI updates. Virtualized collections expose set size, position, current window, and on-demand descendants. Decorative visuals are excluded. Canvas/graph tools expose a logical alternate tree and keyboard operations.

## 4. Accessibility pipeline

1. widget/domain component declares semantic intent;
2. UI reconciliation derives semantic nodes;
3. validation checks names, focusability, relationships, and action support;
4. incremental semantic delta is generated;
5. platform adapter maps capabilities and reports losses;
6. assistive action returns as typed UI/command event;
7. diagnostics record latency, dropped/unsupported fields, and focus changes.

The adapter never infers hidden business meaning. Unsupported platform semantics retain an in-product accessible alternative.

## 5. Required modes

- complete keyboard operation and visible focus;
- controller navigation for runtime and core editor paths;
- text scaling without clipping or lost commands;
- high contrast and color-independent state;
- reduced motion, camera shake, flashing, blur, and analog-effect controls;
- captions/subtitles with speaker, non-speech cue, size, contrast, and timing settings;
- remappable controls and hold/toggle alternatives;
- screen-reader names, values, relationships, errors, and live progress;
- audio/visual/haptic redundancy where gameplay meaning depends on one channel.

Project Meridian additionally preserves its atmosphere while allowing restrained VHS/analog effects to be reduced or disabled.

## 6. Ponder content

Documentation bundles contain:

~~~text
Article {
  id, version, locale, title, summary, body,
  concepts, applies_to_schema, commands,
  examples, prerequisites, related, source_provenance
}
~~~

Bundles are version-matched to engine/API/schema versions and signed like other distribution content. The local index supports exact terms, concepts, commands, diagnostics, schema fields, and fuzzy search. Search remains useful offline.

Structured records are the public documentation API:

~~~rust
pub struct DocRecord {
    id: DocId,
    item_kind: DocItemKind,
    display_name: LocalizedText,
    plain_explanation: Markdown,
    technical_explanation: Markdown,
    units: Option<Unit>,
    valid_range: Option<ValueRange>,
    default_value: Option<Value>,
    performance_impact: Impact,
    memory_impact: Impact,
    determinism_impact: Impact,
    networking_impact: Impact,
    examples: Vec<ExampleId>,
    ponder: Vec<PonderId>,
    version: VersionRange,
}
~~~

## 7. Contextual learning

Ponder context is an explicit bounded object: current panel/document type, selected schema IDs, diagnostic codes, capability state, and engine version. Project values/content are excluded unless a user opens a preview and grants scope.

Learn actions can:

- open an article at an anchor;
- show a non-destructive overlay/tour;
- load a copyable example into a scratch project;
- preview a typed Fix command;
- explain a performance trace span;
- compare guided and expert workflows.

Ponder cannot directly mutate a project outside the command transaction model.

Planned CLI and MCP operations:

~~~text
meridian docs check
meridian docs explain <doc-id-or-diagnostic>
meridian accessibility check --project .
meridian ponder run <ponder-id>
mcp.docs.explain
mcp.accessibility.check
mcp.ponder.preview_fix
~~~

These are planned contracts. They become executable only after schema validation, permission policy, and example tests exist.

## 8. Documentation authoring and compatibility

Docs source is text plus schemas/examples in the repository. CI verifies links, code/schema examples, diagnostic coverage, locale fallbacks, version ranges, and command IDs. Removed APIs keep tombstone articles with migration paths for the supported compatibility window.

User annotations, history, and progress are local user data and not shipped in projects. A corrupt index is rebuildable from signed bundles.

## 9. Diagnostics and privacy

Accessibility diagnostics: unnamed focusable nodes, unreachable actions, focus traps, contrast, clipping under scale, semantic update latency, unsupported adapter mappings.

Ponder diagnostics: index version, stale/missing articles, broken examples, search timing, bundle verification, context fields included, provider/network state.

No accessibility text or Ponder context may reveal secrets. Network-backed assistance is opt-in per request, previews exact context, and records provider/retention policy.

## 10. Tiers and optionality

Semantic UI support is core and cannot be disabled in editor/runtime UI builds. Platform screen-reader adapters depend on platform availability. Ponder local docs are core editor content; optional local/cloud generative explanation adapters add no dependency, network, model process, or index work when disabled.

Algorithm gates:

| Problem | Baseline | Alternative | Gate |
|---|---|---|---|
| OS accessibility | Meridian semantic tree to AccessKit/native adapter | direct per-widget OS ownership | Adapter loss must be diagnosable before direct OS paths are considered. |
| Search | local structured index | embeddings or AI retrieval | Embeddings require versioned index, privacy review, and retrieval benchmarks. |
| Ponder replay | semantic commands and assertions | raw input or video replay | Raw capture is allowed only as an authoring input, not durable truth. |
| Explanation | deterministic docs | AI conversational layer | AI must cite local records and pass permission/privacy tests. |

## 11. Tests and benchmarks

- semantic snapshots for every core widget/panel and game menu;
- keyboard-only and controller-only golden journeys;
- screen-reader adapter integration on supported platforms;
- 100–400 percent text scaling and constrained viewport tests;
- reduced-motion/flash/analog-effect compliance;
- caption timing and non-speech cue tests;
- Ponder offline search relevance/latency corpus;
- every diagnostic code resolves to an article or deliberate no-article record;
- docs example build/run/schema validation;
- provider privacy preview and denied-capability tests.

## 12. Phases

- Phase 4: semantic core and adapter spike.
- Phase 6: keyboard/screen-reader editor foundation and local Ponder index.
- Phase 8: opening-slice settings, captions/cues, remapping, reduced effects.
- Phase 11: complex editor graph/table accessibility.
- Phase 18: optional agent-assisted explanation with permission preview.
- Phase 26: full accessibility and documentation audit.

## 13. Examples

End-to-end: focus enters a material inspector. The screen reader announces material, visual facet, roughness slider, current value, and validation. Help opens the exact schema article; changing the value invokes the same command as pointer input.

Failure/recovery: an updated docs index is corrupt. Signature/hash validation rejects it, the prior bundle remains mounted, and a background rebuild reports progress without blocking editing.

Performance debug: semantic updates spike while scrolling a virtual tree. The profiler compares visible nodes, exposed semantic window, adapter delta size, and update latency, revealing an accidental full-tree rebuild.
