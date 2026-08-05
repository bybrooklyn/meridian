# Project Meridian Prototype Plan

[Master](MERIDIAN_MASTER_SPEC.md) · [Roadmap](DELIVERY_ROADMAP.md) · [Production opening slice](PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Sanitized engine-facing integration contract

## 1. Authority and status

Documentation maturity: `ImplementationReady` for a future bounded prototype package. Implementation maturity: `Planned`. Delivery: `MS-06`. Governing IDs: `REQ-PRJ-001`, `REQ-CORE-004`, `REQ-GOV-002`, `WP-PRJ-001`.

This document defines what the Meridian engine must prove through the first private consumer-game prototype. It does not own route, narrative, pacing, art, audio intent, lore, or assets. Those remain in `bybrooklyn/project-meridian`.

## 2. Goal

After Creator Editor Alpha and the representative forest renderer pass, produce a reproducible private prototype that proves the engine can support a coherent end-to-end game loop:

1. open the private project in Creator Editor;
2. create or import generated/approved prototype assets through the native Meridian modeler and asset pipeline;
3. edit and save a small representative forest space;
4. enter isolated Play mode;
5. move with a grounded controller and interact with one typed object;
6. render night terrain, vegetation, flashlight/local lighting, fog, and basic weather;
7. play minimum ambient/spatial Wavefront audio;
8. execute one Rust-backed typed gameplay state transition;
9. commit and recover one save checkpoint;
10. build, package, install, launch, and replay the result.

## 3. Non-goals

- production game content or creative sign-off;
- the complete opening sequence or exact private route;
- combat, enemies, multiplayer, XR, advanced GI, Torsant simulation, native backends, or a successor renderer path;
- shipping-quality assets, final audio, final accessibility copy, certification, or release performance;
- copying private documents or assets into engine fixtures.

The prototype may use generated/redacted substitutes. It is discarded or migrated according to private production decisions; it is not automatically production content.

## 4. Entry gates

Both `MS-03` Creator Editor Alpha and `MS-05` representative forest renderer MUST pass. Required narrow dependencies also include:

- observable runtime, pass timing, and visible capture from MS-01;
- source asset/world/save/package foundations;
- minimum Cairn-owned public descriptors, even if the implementation remains transitional;
- minimum Wavefront output/mixer/spatial event seam;
- `WP-MDL-001` native editable-model baseline with stable mesh-element identity, undo/recovery, materials, and simple collision/LOD source facets;
- `WP-GAM-001` Rust gameplay API/module and isolated Play rebuild/restart foundation;
- a stable typed command registry used by editor and automation;
- private repository checkpoint and rights/provenance review.
- `WP-PRC-002` sanitized Alluvium environmental-corpus evidence; private
  recipes, seeds, constraints, and overrides remain in the game repository.

Luau is not a prototype dependency and cannot replace missing `WP-GAM-001` Rust evidence. The prototype may use typed logic documents alongside Rust, but native module lifecycle, reflection, failure, and Play isolation still require proof.

## 5. Repository and data boundary

The private game repository owns all prototype source documents and content. It consumes the engine through published workspace/dependency instructions established at activation time. The engine repository receives only:

- versioned public contract tests;
- generated/redacted corpus recipes;
- private checkpoint and corpus hashes;
- aggregated performance/evidence records with sensitive paths redacted;
- defect reports that reproduce without proprietary content where possible.

No nested `game/` path is tracked by the engine repository. No cross-private CI credential is required by this plan.

## 6. Work packages

### WP-PRJ-001 — prototype integration

Result: the private repository builds and runs a bounded consumer-game prototype against a pinned Meridian checkpoint.

Subpackages are activated later with stable IDs for project bootstrap, editor journey, world/content import, movement/interaction, rendering/environment, audio, gameplay, save/recovery, package/install, and evidence review. Each package owns one visible journey and can fail independently without corrupting source.

### 6.1 Work package brief

Mapped onto [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md)'s
contract fields for consistency with the rest of the suite; the content
already lives in this document's own sections, cross-referenced rather than
duplicated. No status change.

Result: §2's ten-step end-to-end loop. Owning repository: the private
`bybrooklyn/project-meridian` prototype, integrating against this engine
repository's published contracts only (§5). Entry conditions: §4's full gate
list (MS-03, MS-05, MS-01 observable-runtime evidence, source asset/world/
save/package foundations, minimum Cairn/Wavefront seams, `WP-MDL-001`,
`WP-GAM-001`, a stable typed command registry, private-repository checkpoint
and rights review, `WP-PRC-002` sanitized corpus evidence) — Luau is
explicitly not a dependency and cannot substitute for missing `WP-GAM-001`
evidence (§4). Deliverables: the ten numbered subpackages named in §6
(project bootstrap, editor journey, world/content import, movement/
interaction, rendering/environment, audio, gameplay, save/recovery, package/
install, evidence review), each independently failable without corrupting
source. Non-goals: the full §3 list — no production content/creative
sign-off, no complete opening sequence, no combat/enemies/multiplayer/XR/
advanced GI/Torsant/native backends, no shipping-quality assets or
certification. Failure/recovery: the full §8 table. Tests/evidence: §10 in
full. Stop condition: any subpackage that cannot independently prove its
journey blocks only that journey, not the whole prototype (§6's "can fail
independently" clause). Next unblocked: `WP-PRJ-002`'s opening-slice
production work, which cannot inherit this package's prototype-only
acceptance (`PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md` §1).

## 7. Runtime pipeline

~~~text
private source documents and assets
  -> isolated import and artifact graph
  -> world/source validation
  -> Creator Editor command transaction
  -> build/package profile
  -> runtime world and immutable snapshots
  -> Cairn simulation + Rust gameplay commands
  -> accepted Alluvium-built artifacts
  -> Penumbra/Isobar/Basalt/vegetation presentation
  -> Wavefront events + Meridian UI overlay
  -> save transaction and recovery head
  -> evidence bundle
~~~

No presentation subsystem becomes authoritative for gameplay or save state. Play mode forks runtime state and never silently writes back.

## 8. Failure and recovery

| Failure | Required behavior |
|---|---|
| import/build worker crash | preserve source and prior valid artifacts; restart with operation ID |
| unavailable GPU capability | select declared fallback or stop with typed actionable diagnostic |
| occluded/minimized surface | record non-visual outcome; do not claim image quality |
| missing optional asset/audio | use declared generated fallback and mark evidence limitation |
| required world/collision data missing | refuse play/package; identify owning source object |
| interrupted save | recover the previous committed head and explain discarded tail |
| private path in trace | redact before export and fail the publication audit |

## 9. Accessibility and workflows

The prototype is operable with remappable keyboard/controller input, scalable UI, semantic labels for its overlay, visible and non-audio-only error feedback, reduced-effects settings, and a recovery path. Beginner workflow uses Open, Import, Play, Save, Build, and Run. Expert workflow exposes IDs, capability profile, render pass timing, streaming reasons, asset provenance, command log, and save journal.

## 10. Evidence and completion

Completion requires a clean private checkout, pinned engine checkpoint,
Alluvium recipe/provenance hashes without public private-source payload,
reproducible build/package, install/launch journey, visible captures, frame/pass
and memory distributions, asset/streaming trace, controller/interaction trace,
Wavefront report, native-modeler source/undo/recovery evidence, Rust module/Play restart evidence, save-interruption recovery, keyboard/controller/accessibility
review, rights/provenance report, known limits, and zero engine-repository
private-content leakage.

This evidence proves consumer integration only. It does not complete `MS-07`, certify production quality, or calibrate the permanent Penumbra suite beyond the exact prototype corpus.

## 11. Examples

End-to-end: a creator imports a generated tree set, edits a small source world, presses Play, traverses a flashlight-lit path, triggers one interaction, hears a spatial cue, commits a checkpoint, packages the prototype, and launches the installed build.

Failure: the process terminates during save. Relaunch discards the incomplete tail, restores the previous committed state, and displays a recoverable diagnostic without mutating source documents.

Performance debug: a hitch correlates one world-cell activation, texture upload, vegetation submission burst, and audio refill under a shared trace ID; a before/after rerun uses the same checkpoint, camera/input recording, hardware, profile, and warmup state.
