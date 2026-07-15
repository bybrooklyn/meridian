# Research and Algorithm Decisions

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Phases](IMPLEMENTATION_PHASES.md)

Version 0.2 · Research verified against linked primary sources on 2026-07-14

## 1. Method

Research distinguishes:

- Adopted baseline: selected for the named phase and stable seam.
- Transitional: current repository implementation retained behind a replaceable boundary.
- Research gate: prototypes must share corpus/metrics/deadline/owner.
- Deferred: no prototype needed before its phase.
- Rejected: conflicts with product invariants.

Primary project documentation, official specifications, official repositories, and original research papers are preferred. A link establishes research context, not redistribution rights, production fitness, or a frozen dependency version. Every implementation phase rechecks current version, license, advisories, platform terms, and API behavior.

## 2. Current decision register

| ID | Subject | Decision | Status and phase |
|---|---|---|---|
| R-UI-01 | Permanent UI | Meridian-owned retained UI core; editor/runtime share core. egui is disposable bootstrap. | Adopted architecture; P6/P9/P14 migration |
| R-UI-02 | UI renderer | Backend-neutral display list; evaluate Vello/native paths after text/layout/semantics corpus. | Research gate P6/P9 |
| R-A11Y-01 | Accessibility | Meridian owns semantic tree; AccessKit/native are adapters. | Adopted P6 |
| R-RENDER-01 | Current graphics backend | Keep wgpu behind Meridian RHI while capabilities/evidence grow. | Transitional adopted P1/P2 |
| R-RENDER-02 | Opening renderer | Depth prepass plus clustered Forward+ evolving from current direct PBR. | Adopted P2/P8 |
| R-RENDER-03 | IBL order | Diffuse irradiance is implemented foundation; pass timing next; specular prefilter/LUT later bounded work. | Adopted active plan |
| R-RENDER-04 | Visibility/geometry/GI/rays | Shared P12 prototypes; no mandatory vendor/algorithm. | Research P12 |
| R-PHYS-01 | Physics ownership | Cairn hard-fork path from pinned Rapier plus selected Box2D study/ports; no Rapier API compatibility goal. | Adopted migration P3+ |
| R-PHYS-02 | Determinism | Explicit modes/envelopes; no universal cross-platform bit identity claim. | Adopted; calibrate P3/P13 |
| R-ECS-01 | ECS | Current bevy_ecs is transitional behind Meridian IDs/commands/schemas; replacement requires benchmark. | Research deadline after P8 |
| R-SCRIPT-01 | Initial scripting | Luau broad Lua-compatible subset is the only initial high-level runtime. | Adopted P7 |
| R-XR-01 | XR | OpenXR-first adapter and predicted-time pipeline. | Adopted architecture, deferred P19 |
| R-BUILD-01 | Rust build/IDE | Cargo files authoritative; cargo metadata/JSON and rust-analyzer are integrated, not replaced. | Adopted P16 |
| R-VCS-01 | VCS | Git-compatible interoperability plus Jujutsu-derived changes/operation log; provenance/reimplementation gate. | Adopted architecture, research P17 |
| R-SYNC-01 | Sync | meridian-sync direct encrypted P2P first, optional self-hosted relay; no Telepo/account/cloud/inbound requirement. | Adopted P18 |
| R-AGENT-01 | Tool semantics | One typed registry for UI/CLI/Rust/MCP/agents; no privileged AI API. | Adopted P6/P16/P25 |
| R-AGENT-02 | Ollama | Local/cloud/OpenAI-compatible/web-search are distinct capability/trust profiles. | Adopted P25 |
| R-SEC-01 | Updates | TUF-inspired role model; exact crypto/library/key policy requires threat-model gate. | Adopted architecture; gate P5/P29 |
| R-DATA-01 | Hash/compression | Content-addressed design; BLAKE3 and Zstandard are candidates/current plan, verified by format/profile evidence. | Baseline candidate P5 |
| R-SIM-01 | Advanced simulations | Portfolio of specialized optional solvers; no universal solver/graph. | Research P20/P21/P27 |
| R-NET-01 | Networking | Transport/provider-neutral core; Steam/EOS optional adapters. | Adopted architecture P22/P23 |

## 3. UI research

Official/reference sources:

- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) describes Zed’s hybrid UI framework and is useful architecture evidence, not a selected Meridian dependency.
- [Vello repository](https://github.com/linebender/vello) is a GPU compute-oriented 2D renderer candidate for display-list experiments.
- [Slint backend/renderer documentation](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/) informs backend separation.
- [AccessKit crate documentation](https://docs.rs/accesskit/latest/accesskit/) defines its cross-platform accessibility adapter role.

R-UI-G1 compares:

- Meridian display list rendered through current renderer;
- a Vello-backed path;
- a conservative CPU tessellation/native GPU path.

Stable seam: DisplayList, glyph/image handles, semantic tree. Corpus: editor panels, large virtualized tree/table, text editor, graphs, runtime HUD, DPI/locales. Metrics: correctness, text quality, latency distribution, GPU/CPU/memory, cache behavior, device recovery, platform support, maintenance/license. Deadline: before broad P9 migration. Owner: UI/render leads. Losing prototypes remain in research branch/artifact report, not production dependency graph.

## 4. Rendering research

Sources:

- [wgpu official repository](https://github.com/gfx-rs/wgpu) and [releases](https://github.com/gfx-rs/wgpu/releases) govern current backend behavior/version review.
- Burns and Hunt’s [visibility buffer paper](https://jcgt.org/published/0002/02/04/) is a primary algorithm reference.
- Majercik et al. [Dynamic Diffuse Global Illumination](https://jcgt.org/published/0008/02/01/) informs probe/radiance-cache research.
- Bitterli et al. [ReSTIR](https://cwyman.org/papers/sig20_ReSTIR.pdf) informs later direct-light sampling research, not opening scope.

R-RENDER-G1, deadline P12:

- production Forward+ baseline;
- visibility-buffer prototype;
- hybrid path only if a measured use case requires it.

Corpus: B01/B02, dense materials/vegetation, transparency, skinning, decals, editor picking, MSAA/XR-shaped views. Metrics: pass/frame CPU/GPU distribution, memory/bandwidth, material/shader complexity, draw/visibility cost, feature compatibility, image reference, implementation/maintenance. Stable seam: extracted scene, material facets, render graph, visibility results.

R-RENDER-G2 evaluates virtual geometry page/hierarchy/raster/deformation alternatives on a public/generated corpus. R-RENDER-G3 evaluates baked/probe/DDGI/radiance-cache/hardware-ray portfolios against a path-traced reference. No one GI technique is assumed to serve all tiers.

## 5. Cairn research and provenance

Sources:

- [Rapier determinism guidance](https://rapier.rs/docs/user_guides/templates/determinism), [simulation structures](https://rapier.rs/docs/user_guides/rust/simulation_structures/), and [CCD](https://rapier.rs/docs/user_guides/rust/rigid_body_ccd/) define relevant upstream behavior and caveats.
- [Box2D 3.1 announcement](https://box2d.org/posts/2025/04/box2d-3.1/) and [3.1 release notes](https://box2d.org/documentation/md_release__notes__v310.html) inform modern data-oriented API/solver study.
- Parker and O’Brien’s [real-time deformation/fracture paper](https://graphics.berkeley.edu/papers/Parker-RTD-2009-08/) is a primary destruction reference.

Before source transfer:

1. record exact upstream repository/revision/archive hash;
2. archive license/notices and file-level provenance;
3. enumerate local patches and generated code;
4. build unmodified baseline;
5. capture differential scene/test/benchmark corpus;
6. define Cairn-native seam and remove compatibility objective;
7. review redistribution/attribution.

R-PHYS-G1 compares broadphase/narrowphase/solver layout changes only behind body/shape/query/snapshot contracts. R-PHYS-G2 defines stable, deterministic, and strict modes from measured results, including compiler/features/thread count/platform.

## 6. ECS and scheduling research

The repository uses bevy_ecs 0.19 as a current implementation aid. The stable seam is Meridian PersistentEntityId, component schema, query descriptors, command buffers, fixed barriers, extraction, save/network mapping.

R-ECS-G1 begins after Phase 8 evidence:

- retain/wrap current ECS;
- Meridian-owned archetype chunk prototype;
- hybrid migration.

Corpus: opening world, high-entity synthetic, streaming activation, save/extraction, change tracking, editor queries. Metrics: fixed-step CPU distribution, memory/layout, command merge, determinism, serialization/network fit, tooling, migration and maintenance. Replacement occurs only on measured total product value.

## 7. Luau research

Sources:

- [Luau sandbox guidance](https://luau.org/sandbox/)
- [Luau embedding API](https://luau.org/api/)
- [Luau performance](https://luau.org/performance/)
- [Lua compatibility](https://luau.org/compatibility/)
- [official repository](https://github.com/luau-lang/luau)

The selected baseline is embedding behind generated Meridian bindings. Research remains for VM isolation granularity, allocator/instruction hooks, bytecode/source distribution, debugger integration, hot-reload state migration, deterministic APIs, and exact broad Lua-compatible subset.

Additional language work is prohibited before Phase 28. Each candidate must beat its maintenance/runtime/package/debug/security cost on actual user demand.

## 8. Cargo, rust-analyzer, and VCS

Sources:

- [Cargo reference](https://doc.rust-lang.org/cargo/reference/), [features](https://doc.rust-lang.org/stable/cargo/reference/features.html), [workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html), and [cargo metadata](https://doc.rust-lang.org/stable/cargo/commands/cargo-metadata.html)
- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html), [configuration](https://rust-analyzer.github.io/book/configuration), and [diagnostics](https://rust-analyzer.github.io/book/diagnostics.html)
- Jujutsu [operation log](https://docs.jj-vcs.dev/latest/operation-log/), [conflicts](https://jj-vcs.github.io/jj/latest/conflicts/), and [concurrency](https://jj-vcs.github.io/jj/latest/technical/concurrency/)
- Unreal documentation for [source control](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-source-control-in-the-unreal-editor), [Multi-User Editing](https://dev.epicgames.com/documentation/en-us/unreal-engine/multi-user-editing-overview-for-unreal-engine), and [Virtual Assets](https://dev.epicgames.com/documentation/en-us/unreal-engine/overview-of-virtual-assets-in-unreal-engine) as product comparisons, not implementation authority
- Forgejo [documentation](https://forgejo.org/docs/latest/), [API use](https://forgejo.org/docs/latest/user/api-usage/), [releases](https://forgejo.org/docs/latest/user/releases/), and [pull requests/Git flow](https://forgejo.org/docs/latest/user/pull-requests-and-git-flow/) for optional self-hosted integration research

R-VCS-G1 deadline P17 compares direct integration/reuse/fork/reimplementation under license/provenance, semantic-data requirements, Git interoperability, operation recovery, large assets, embedded-product UX, and maintenance. Stable user concepts remain ChangeId, OperationId, workspace, semantic diff, and Git remote interoperability.

Live collaboration separately chooses OT/CRDT/locks/operation streams per document type; source control remains authoritative.

## 9. Agent, MCP, and Ollama research

Sources:

- Ollama [web search](https://docs.ollama.com/capabilities/web-search), [cloud](https://docs.ollama.com/cloud), [OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility), and [FAQ](https://docs.ollama.com/faq)
- OpenAI [Codex app server](https://developers.openai.com/codex/app-server), [MCP](https://developers.openai.com/codex/mcp), and [agent approvals/security](https://developers.openai.com/codex/agent-approvals-security)

R-AGENT-G1 compares provider adapters on schema/tool correctness, local/offline behavior, structured output/tool calls, model discovery, streaming/cancellation, privacy/trust, latency/cost, and maintenance. It never changes command authority.

R-AGENT-G2 evaluates retrieval chunking/embedding models on exact source attribution, recall/precision task corpus, index size/build time, local hardware, stale update behavior, and sensitivity filtering. Exact symbol/schema/diagnostic search is always available without embeddings.

## 10. OpenXR

The official [OpenXR registry](https://registry.khronos.org/OpenXR/) exposed OpenXR 1.1 specification revision 1.1.61 at the 2026-07-14 research pass; use the current registry at implementation. The [OpenXR 1.1 specification](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html) is authoritative for lifecycle, timing, views, actions, spaces, and extensions.

P19 runtime matrix selects required extensions only after testing. Vendor-specific features remain adapters. Stable seams are XR session/view/action/space/capability descriptors and renderer/input/interaction snapshots.

## 11. Updates and security

The official [TUF specification](https://theupdateframework.github.io/specification/) and [TUF site](https://theupdateframework.org/spec/) reported specification version 1.0.32 at the research pass. Meridian adopts the role/freshness/delegation threat model, not an unreviewed homegrown cryptosystem.

R-SEC-G1 before package/update format freeze selects implementation library, algorithms, thresholds, expiration, key storage, metadata canonicalization, and compromise procedure from threat model, platform support, audit maturity, licensing, performance, and operational test.

## 12. Networking and services

Sources:

- Steam [multiplayer](https://partner.steamgames.com/doc/features/multiplayer?l=english), [authentication](https://partner.steamgames.com/doc/features/auth?l=english), and [Networking Sockets](https://partner.steamgames.com/doc/api/ISteamNetworkingSockets?l=english&language=english)
- Epic Online Services [introduction](https://onlineservices.epicgames.com/en-US/news/introduction-to-epic-online-services-eos?lang=en-US) and [trust and safety](https://onlineservices.epicgames.com/trust-safety)

R-NET-G1 evaluates native UDP/QUIC and provider transport properties on impairment, NAT/relay, encryption/auth seams, platform support, headless operations, maintenance, and license. Replication is stable above transport. Steam/EOS are optional Phase 23 adapters.

## 13. Hashing and compression

Sources:

- [BLAKE3 official repository](https://github.com/BLAKE3-team/BLAKE3)
- Zstandard [project](https://facebook.github.io/zstd/index.html) and [API manual](https://facebook.github.io/zstd/zstd_manual.html)

Content hash and codec fields are format identifiers, not hardwired implementation assumptions. P5 tests collision-resistant identity use, streaming/range behavior, dictionary/version policy, decompression limits, platform support, and migration. Security does not rely on a fast non-cryptographic checksum.

## 14. Simulation and acoustics research

Primary references:

- Jos Stam [Stable Fluids publications](https://www.josstam.com/publications) and [Real-Time Fluid Dynamics for Games](https://graphics.cs.cmu.edu/nsp/course/15-464/Spring07/papers/StamFluidforGames.pdf)
- Bridson [fluid simulation resources](https://www.cs.ubc.ca/~rbridson/fluidsimulation/)
- Ando et al. [stream-function liquids](https://doi.org/10.1145/1185657.1185730)
- Jiang et al. [APIC](https://www.cs.ucr.edu/~craigs/papers/2015-apic/paper.pdf)
- Macklin et al. [XPBD](https://matthias-research.github.io/pages/publications/XPBD.pdf)
- Nguyen et al. [physically based fire](https://graphics.stanford.edu/papers/fire-sg02/)
- [GSound](https://gamma.cs.unc.edu/GSOUND/gsound_aes41st.pdf) and [precomputed wave simulation](https://gamma.cs.unc.edu/PrecompWaveSim/docs/paper_docs/paper.pdf)
- shallow-water references [ANZIAM article](https://journal.austms.org.au/ojs/index.php/ANZIAMJ/article/view/645) and [arXiv 1401.4125](https://arxiv.org/abs/1401.4125)

These papers define candidates and validation ideas, not one production solver. Gates:

- R-AUDIO-G1: authored zones/portals/probes versus geometric/wave hybrid on impulse-response and game usefulness corpus, deadline P20.
- R-DEFORM-G1: XPBD/related deformable formulations versus authored/rigid approximations, deadline P21.
- R-FIRE-G1: cellular/field/particle visual-thermal portfolios, deadline P21/P27.
- R-FLUID-G1: shallow-water grid, particle-grid/APIC-like, and authored/baked tiers by scale, deadline P27.
- R-SNOW-G1: heightfield/granular/deformable tiers, deadline P27.

Metrics include stability envelope, conservation where meaningful, visual/reference error, authoring control, CPU/GPU/memory, streaming, determinism, save/network, accessibility impact, fallback, and zero-cost-disabled proof.

## 15. Open decisions and owners

| Gate | Deadline | Owner role | Stable seam |
|---|---|---|---|
| UI renderer backend | P9 | UI + rendering | display list/semantic tree |
| Native graphics backend threshold | P12 | rendering/platform | RHI descriptors/handles |
| Visibility/virtual geometry/GI/rays | P12 | rendering | extracted scene/render graph/material facets |
| Meridian ECS replacement | after P8, before freeze | runtime/world | IDs/schema/commands/query/extraction |
| Cairn solver/layout/determinism | P3/P13 | physics | Cairn descriptors/handles/snapshot |
| VCS implementation lineage | P17 | VCS/legal/security | changes/operations/Git interop |
| Sync transport/NAT/relay | P18 | sync/security | peer/object/chunk/session protocol |
| XR extensions/runtime matrix | P19 | XR/platform | session/view/action/space |
| Hybrid acoustics | P20 | audio/simulation | acoustic scene/snapshot/DSP graph |
| Deform/fire/thermal | P21 | physics/environment | field/structure/event contracts |
| Multiplayer transports/providers | P22/P23 | network/security | protocol/transport/provider traits |
| Update crypto/library/key policy | before release package freeze | security/release | role metadata/verifier/signer |
| Mod runtime/sandbox | P24 | mod/security/gameplay | manifest/capabilities/published API |
| Agent providers/retrieval | P25 | tooling/security | command/query/context/audit |
| Buildings/ecosystems | P26 | procedural/content | domain graph/candidate/override |
| Fluid/flood/erosion/snow | P27 | simulation/world | field/solver snapshot/coupling |
| Additional languages | P28 | gameplay/tooling | API schema/module/capability |

## 16. Decision output

Each completed gate produces an ADR containing raw evidence links, prototype revisions, corpus hash, hardware/software, metrics/statistics, qualitative review, security/accessibility/maintenance/licensing, winner and limits, migration plan, and losing-prototype archive. PLANNING changes only after the ADR and relevant normative specs are updated.
