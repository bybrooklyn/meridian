# Meridian vision

This document states where Meridian is headed, not what exists today. It is
aspirational and non-normative: it creates no requirement, does not satisfy
or promote any milestone, and is outside the [normative spec
suite](MERIDIAN_SPECOMENT.md)'s authority order and validation.
[README.md](README.md) and [PLANNING.md](PLANNING.md) remain authoritative
for current implementation truth; each pillar below links to its owning spec
and real current status.

## The ten pillars

**Performance without hand-tuning.** The goal is for developers to ship
well-performing games without manually optimizing them. Owned by
[COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md](MERIDIAN_SPECOMENT.md)
as `PRG-REL-001`, a `Deferred` post-1.0 program. Any future claim here must
stay reproducible, scoped, and calibrated — the spec explicitly forbids a
permanent, unscoped superiority promise.

**Write games in the language you already know.** Rust is the first
gameplay implementation language today, with optional Luau following once
its contracts stabilize, per
[GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md](MERIDIAN_SPECOMENT.md)
and [ADR-0019](docs/architecture/decisions/ADR-0019-rust-first-luau-after.md).
TypeScript, Python, and C# are not committed bindings — the spec marks
additional languages beyond Luau as later, independent research, not a
scheduled deliverable.

**Smart procedural authoring.** The Alluvium Engine authors spatial fields,
recipes, and derived artifacts so creators don't hand-place everything. Owned
by [PROCEDURAL_AUTHORING_SPEC.md](MERIDIAN_SPECOMENT.md),
currently `ImplementedFoundation` — a real but partial foundation, not the
full system envisioned here.

**Physics and liquids built in.** Rigid-body and fluid simulation as
first-class engine systems. Owned by
[CAIRN_PHYSICS_SPEC.md](MERIDIAN_SPECOMENT.md) (physics, early-stage)
and
[TORSANT_FIRE_FLUIDS_AND_THERMAL_SIMULATION_SPEC.md](MERIDIAN_SPECOMENT.md)
(fire/fluids/thermal), currently `Research` maturity — the least mature
status in the suite.

**AI agent integration from day one.** Editor, CLI, and agents sharing one
typed command surface with no privileged AI backdoor. Owned by
[AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md](MERIDIAN_SPECOMENT.md),
currently `Deferred` to MS-08/MS-09.

**Multiplayer server built in.** Transport-neutral client/server support,
replication, and a headless dedicated-server crate as a standard part of the
engine. Owned by
[MULTIPLAYER_AND_SERVER_SPEC.md](MERIDIAN_SPECOMENT.md),
currently `Deferred` to MS-09; Project Meridian, the first proving game, is
single-player.

**Promotional material generation.** Tooling to turn captured gameplay into
promotional media without leaving Meridian. Owned by
[MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md](MERIDIAN_SPECOMENT.md)
as `PRG-PRM-001`, `Deferred` post-1.0; no Marquee runtime crate or editor
implementation exists yet. Its scoped design imports manually supplied,
approved captures and source media and produces local files, with AI limited
to non-authoritative text and analysis suggestions — not open-ended AI
generation.

**A built-in asset store.** Discovering and installing community content
without leaving the editor. Owned by
[MODDING_AND_COMMUNITY_LIBRARY_SPEC.md](MERIDIAN_SPECOMENT.md),
currently `Deferred` to MS-09. The spec scopes this as a free,
provider-neutral community library, explicitly not a commercial marketplace.

**Free and open source.** True today, under
[LICENSE-MPL-2.0](LICENSE-MPL-2.0).

**A built-in UI framework.** One Meridian-owned UI system spanning the
editor shell and in-game UI. Owned by
[EDITOR_AND_MERIDIAN_UI_SPEC.md](MERIDIAN_SPECOMENT.md),
currently `ImplementedFoundation` for its first work package
(`WP-UI-001`) only.
