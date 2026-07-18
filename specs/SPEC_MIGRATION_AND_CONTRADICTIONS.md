# Meridian Specification Migration and Contradiction Register

[Master](MERIDIAN_MASTER_SPEC.md) · [Roadmap](DELIVERY_ROADMAP.md) · [v0.1 heading ledger](../docs/migrations/V0_1_DOCUMENT_MIGRATION.md) · [v0.3 roadmap ledger](../docs/migrations/V0_3_ROADMAP_MIGRATION.md) · [v0.4 Alluvium ledger](../docs/migrations/V0_4_ALLUVIUM_AMENDMENT.md) · [v0.5 general-purpose ledger](../docs/migrations/V0_5_GENERAL_PURPOSE_PLATFORM_AMENDMENT.md) · [v0.5 Marquee ledger](../docs/migrations/V0_5_MARQUEE_AMENDMENT.md) · [v0.5 performance/quality ledger](../docs/migrations/V0_5_PERFORMANCE_QUALITY_AMENDMENT.md) · [v0.5 Meridian UI 1.0 ledger](../docs/migrations/V0_5_MERIDIAN_UI_1_0_AMENDMENT.md)

version 0.5 · 2026-07-15 · Normative migration/history record

This register records winning decisions and preserves the disposition of older
architecture, roadmap, and private-game statements. Historical phase/file names
may appear here and in `docs/migrations/`; active specifications use v0.5
milestones, packages, gates, and authorities.

## 1. Authority and status migration

The v0.5 owning subsystem specification and adopted ADR win, followed by this
register, the delivery roadmap, typed registries, and PLANNING. Private Project
Meridian documents own creative decisions only. Code and evidence prove current
behavior but do not silently choose permanent architecture.

Status is now three-axis:

- documentation: `Draft`, `ArchitectureComplete`, `ResearchReady`,
  `ImplementationReady`, `VerifiedCurrent`;
- implementation: `Implemented`, `ImplementedFoundation`, `StructuralSmoke`,
  `Partial`, `Transitional`, `Scaffold`, `Planned`, `Research`, `Deferred`,
  `Unsupported`;
- evidence: `Pass`, `Fail`, `NotRun`, `UnsupportedCapability`,
  `UnsupportedPlatform`, `Occluded`, `Redacted`, `Waived`, `Stale`,
  `Inconclusive`.

The older single status vocabulary is superseded because it allowed complete
documentation to be confused with complete software.

## 2. Winning decisions

| Subject | Previous ambiguity | v0.5 winner and migration |
|---|---|---|
| Suite authority | Seven v0.1 roots and a v0.2 suite could drift. | v0.3 owning specs, registries, ADRs, and validator are canonical. v0.1 headings remain zero-unmapped in the historical ledger. |
| Delivery | P0-P29 mixed subsystem order, products, and evidence. | MS-00 through MS-10 are evidence gates; parallel domain work uses `WP-*`. Every old phase/package maps in the v0.3 ledger. |
| Status | Planned, transitional, and implemented labels were overloaded. | Separate documentation, implementation, and evidence maturity; scaffolds and structural smoke cannot promote claims. |
| Product sequencing | Earlier all-engine-first interpretations delayed game proof; later discussion risked prototype-before-editor work. | Editor-first: MS-02 UI proof, MS-03 Creator Editor Alpha, MS-05 representative forest, then MS-06 private prototype and MS-07 production opening slice. |
| Renderer name | Rendering had generic or mixed labels. | Penumbra is the Meridian-owned renderer; `meridian-renderer` remains its implementation crate. |
| Production shading | Deferred/Forward+/visibility-buffer alternatives were mixed with current direct PBR code. | Clustered Forward+ is adopted production architecture. Current direct PBR/shadow/diffuse-IBL code is Partial/Transitional foundation. |
| Successor path | A future renderer could be presumed or selected from small wins. | `RG-PEN-001` opens after MS-05, requires preregistered material thresholds, full parity, lower-tier/native evidence, and sustainable maintenance. No successor is required for 1.0. |
| Shared renderer systems | Multiple paths risked duplicating materials, scene data, streaming, lights, temporal state, and debugging. | Render graph, GPU scene/views, visibility/indirect streams, material/shader IR, light/shadow/environment snapshots, temporal/post/debug/resource systems are path-independent. |
| Material and shader authoring | Visual materials and WGSL could become path/backend authority. | One `MaterialSource -> MaterialIr`; one `ShaderIr` lowers to WGSL now and native targets later. Artists do not duplicate materials per path. |
| RHI | wgpu could leak or be mistaken for permanent public API. | `meridian-rhi` owns Meridian contracts; wgpu is current production backend implementation and remains available during native work. |
| Native backends | Native work could start before a playable corpus or stable abstraction. | `RG-RHI-001`: Metal only after MS-07/stable RHI; Vulkan and Direct3D 12 only after mature Metal/common-RHI differential and maintenance gates. |
| Hardware support | Named GPUs and feature bits could imply support. | `GpuCapabilityProfile` and measured evidence determine tiers; numeric limits remain provisional until calibration. |
| Weather/environment/simulation | One combined spec blurred weather, terrain, vegetation, and specialized simulation ownership. | Isobar owns weather/atmosphere; Basalt owns terrain/large-world geometry; vegetation remains independent; Torsant owns optional fire/fluids/thermal/smoke. |
| Scaffold names | Empty weather and terrain crates used generic names. | Empty packages are renamed `meridian-isobar` and `meridian-basalt`; no Torsant crate exists until real implementation starts. |
| Physics | Rapier wrapper could be mistaken for final engine ownership. | Cairn owns long-term contracts and provenance-controlled migration; current Rapier wrapper remains Transitional evidence. |
| ECS | bevy_ecs could become persistence/network authority. | It remains transitional behind Meridian persistent IDs, schemas, commands, and snapshots; replacement requires evidence. |
| UI | egui could become permanent editor/runtime architecture, or visual work could become an unbounded Creator rewrite. | Meridian UI is the permanent shared retained framework; any bootstrap UI is isolated and deletable. ADR-0028 fixes the two-row shell and sequential `WP-UI-002` through `WP-EDT-003` package path. |
| UI visual identity | Cyan branding, decorative rings, scene-responsive chrome, private-game imagery, and inconsistent concept screens could become accidental product authority. | The exact website palette, Mona/Hubot/JetBrains typography roles, 4px geometry system, no-ring focus policy, and generated 17-state public mockup corpus are canonical. Dense content is opaque and capability claims remain domain-owned. |
| Data | Built artifacts or one-file worlds/packages could become authority. | Schema-defined source directories/documents are authoritative; artifacts/chunks are rebuildable caches; saves are transactional journals/snapshots. |
| Gameplay | Several initial languages could multiply bindings and tooling; earlier wording let Luau block the first game path. | Rust gameplay modules, reflection, typed APIs, and isolated Play rebuild/restart are first in `WP-GAM-001`. Luau remains the first optional embedded high-level language in `WP-GAM-002` and cannot block MS-06/MS-07. |
| Commands and agents | AI-specific APIs could bypass user tools. | UI, CLI, Rust, MCP, and agents share typed commands/queries, permissions, transactions, audit, undo, and rollback. |
| Optional systems | Advanced packs could add hidden cost. | Disabled packs add no tasks, threads, listeners, GPU resources, allocations, panels, dependencies, or chunks. |
| VCS/sync | Telepo/cloud or live sessions could replace source history. | Git interoperability plus Meridian change/operation UX; optional encrypted sync augments durable checkpoints, with no required account/cloud. |
| Benchmarks | B01/B02 and provisional budgets could look calibrated. | Permanent `PEN-B01` through `PEN-B16` definitions remain DefinitionOnly/Uncalibrated until executable evidence exists. |
| AMI benchmark | Private lore/interiors could leak into engine fixtures. | PEN-B04 is a generated redacted functional surrogate. Only private creative authority defines or expands AMI; engine records contain no proprietary content. |
| Donor code | Renderer/physics donor code could be imported without durable provenance. | No borrowed source before complete `third_party/provenance` record and license/revision/hash/modification/test/exit review. |
| Ambition claims | Meridian's ambition could become an unmeasured superiority claim. | Architecture and goals may be ambitious; performance/quality/competitor claims require reproducible like-for-like evidence. |
| Competitive leadership | A request to guarantee superiority could become a permanent global promise or distort the 1.0 roadmap. | `PRG-REL-001` begins only after MS-10; it permits scoped iso-quality, iso-cost, or matched-workflow claims with exact versions, raw evidence, expiry, and retraction. |
| Environmental rendering duplication | Isobar and Torsant could create separate fog/cloud/smoke/steam volume resources, histories, and raymarches. | Penumbra owns one path-independent `ParticipatingMediaSourceSnapshot` consumption/residency/lighting/temporal/compositing contract; producers retain simulation authority. |
| Environmental scheduling and water authority | Global/per-frame simulation and overlapping Isobar/Torsant water state could create unbounded cost or double advancement. | Sparse/multirate budgeted tiles plus `SurfaceFluidHandoff` require one dynamic owner per region/epoch and explicit promotion/demotion/recovery. |
| Environmental material and cost authoring | Fire/fluid behavior could be inferred from pixels and optimization could begin only after runtime failure. | Alluvium authors coherent combustion/fluid facets and derived `RuntimeCostManifest` predictions; Torsant retains live state and runtime traces reconcile predictions. |
| Procedural authoring | Procedural work was narrow, mostly deferred, and overlapped Basalt source ownership. | Alluvium is adopted as core editor/build procedural authoring. It owns recipes, evaluation, fields, generated identity, overrides, provenance, and cooking; runtime systems retain live authority. |
| Alluvium runtime cost | “Core” could imply every game ships an evaluator. | Core first-party authoring is always available; baked-only shipping profiles omit editor/compiler/runtime evaluator and incur zero recurring Alluvium cost. |
| Procedural formats | A definition-only graph example and many possible branded extensions could become accidental commitments. | Logical `meridian.procedural-recipe/v1`; `.mproc` recipe source and `.mfield` derived artifacts are reserved. Other extensions require owning schemas and evidence. |
| Procedural game boundary | Project Meridian targets could leak AMI facilities, private recipes, seeds, or curation into engine fixtures. | Public engine specs retain sanitized functional targets and hashes only; proprietary recipes and creative constraints remain private. |
| Creator application identity | Editor, Studio, and IDE wording could imply separate products. | There is one user-facing application named Meridian. Editor, IDE, modeler, graph, profiler, debugger, VCS, build, and Play are workspaces; bounded helpers/CLI are allowed. |
| Build package scope | A single long-lived BLD package put MS-08 multi-node/remote-service work on the path to the MS-03 local-Cargo prerequisite. | `WP-BLD-001` owns only the bounded MS-03 local-Cargo service seam. `WP-BLD-002` is a separate planned MS-08 package for multi-node result lineage, general artifact/cache policy, service-process/remote-worker supervision, and team profiles. |
| Native modeling | Blender/DCC assumptions could exclude beginners or make imported binaries the only source. | `MDL` owns a core native editable-model document and beginner modeler with stable element lineage. Blender/OpenUSD/glTF are optional interchange with explicit loss reports. |
| Animation and navigation | Cross-system behavior lacked dedicated authority. | `ANI` owns animation/pose/retarget/sequencing; `NAV` owns traversability/artifacts/queries. Gameplay/frameworks retain decisions. Advanced facial work is post-1.0. |
| Official frameworks | Ambition could imply six complete genre stacks before 1.0. | `FWK` delivers shared/selected foundations before 1.0; completion and maintenance of all six families is `PRG-FWK-001`. |
| First-class 2D | 2D could be treated as flattened 3D or postponed indefinitely. | `TWO` coordinates dedicated Penumbra and Cairn 2D paths and a public proving project before 1.0; disabled 3D has zero cost. |
| Shader language | WGSL or graph tools could become competing source authorities, and `MSL` conflicts with established terminology. | The Meridian Shader Language text frontend and material graphs lower to one `ShaderIr`; the full working name is used. WGSL/Naga remains a target/compiler boundary. |
| Audio name and voice | Audio lacked a stable subsystem name; online voice could mix device, transport, and service authority. | Wavefront owns audio/capture/DSP/mixer/device behavior; NET owns transport; Collective owns optional voice-room identity/policy/provider coordination. |
| Online services | Provider adapters, social, analytics, moderation, and sessions could fragment or imply hosted infrastructure. | Collective is one internally modular optional subsystem for those services, provider-neutral and self-hostable. Meridian promises no hosted cloud without funded operations. |
| Distributed worlds and integrity | MMO/anti-cheat ambition could distort the 1.0 roadmap. | `WRL` and `INT` are separate post-1.0 research authorities with explicit failure/privacy/cost/false-positive gates. |
| Post-1.0 work | Adding more MS phases would flatten long-horizon programs into 1.0 delivery. | MS-00 through MS-10 remain stable. `PRG-*` records cannot satisfy, block, or promote milestones. |
| Promotional production | Screenshots, trailers, copy, press material, approvals, and service files existed only as project duties and could leak into renderer/build/service authority. | Marquee (`PRM`) owns post-capture campaigns, deterministic variants, claims, approvals, and local exports under `PRG-PRM-001`. Capture is manual, publishing remains external, and AI is text/analysis-only. |
| Dependency ownership | “Own everything” could force unnecessary rewrites. | Strategic dependencies are `InternalizeEarly`, `InternalizeEventually`, or `WrapIndefinitelyUnlessNecessary`; replacement requires measured product and maintenance evidence. |

## 3. Current code classification

| Area | v0.5 classification | Boundary |
|---|---|---|
| Fixed-step runtime, diagnostics, tasks, platform contracts | `ImplementedFoundation` / `Partial` | Current tests and native smoke do not complete all platforms/recovery. |
| RHI and render graph | `ImplementedFoundation` | wgpu-backed current implementation; stable native-ready contract and broad execution features remain. |
| Direct PBR, cascaded shadows, diffuse irradiance IBL, extraction/upload, current high-level pass timing | `ImplementedFoundation` / `Transitional` | Penumbra foundation only; complete render-graph timing coverage, Forward+, visible quality, and specular IBL remain open packages. |
| Native renderer smoke | `StructuralSmoke` | Six-face upload and pipeline/bind-group construction; occluded outcome is not visual evidence. |
| Asset/world/streaming/save | `Partial` | Useful foundations; authoritative source/import/package pipeline incomplete. |
| Rapier wrapper/controller | `Transitional` | Not Cairn-owned implementation. |
| Meridian UI | `ImplementedFoundation` / `Partial` / `Active` | MS-02 qualified retained UI core; `WP-UI-002` through `WP-UI-005` are locally implemented but unqualified because hosted CI could not allocate runners. `RG-UI-001` is decided by ADR-0029; `WP-UI-005` is reactivated as the sole corrective framework package under non-promoting `WVR-UI-001`, while `WP-EDT-002` is `Partial` and paused. Direct renderer completion, real platform screen-reader evidence, and the production shell remain open. |
| Creator Editor | `ImplementedFoundation` | `WP-EDT-001` persistent hub, source persistence, typed Creator journey, and bundle structure passed run `29605881704`; visual quality and platform screen-reader integration remain open UI/editor packages. |
| Wavefront, Isobar, Basalt, vegetation | `Scaffold` unless narrower evidence is registered | Crate presence does not prove a usable product/system. |
| Alluvium | `Partial` | `WP-PRC-001` has an active source delivery for text recipes, strict scalar evaluation, recovery, CLI, and a basic inspector; its CI evidence and all production/domain work remain open. |
| Native modeler, Rust gameplay modules, optional Luau, animation, navigation, frameworks, first-class 2D, Meridian Shader Language, Collective | `Planned` or `Deferred` | Documentation/contracts and registries only; no runtime/editor implementation claim. |
| Torsant, networking, XR, mods, VCS/sync, agents | `Planned`, `Research`, or `Deferred` | No production implementation claim. |
| Distributed worlds, advanced integrity, and other `PRG-*` programs | `Deferred` or `Research` | Post-1.0 authority only; no milestone or implementation evidence. |
| Marquee | `Deferred` | ResearchReady post-1.0 architecture only; no crate, service integration, active package, or promotional-quality evidence. |
| Competitive performance and quality program | `Deferred` | ResearchReady post-1.0 comparison/claim architecture only; no active package, calibrated comparator corpus, optimization, or superiority evidence. |

## 4. Historical migration disposition

The seven v0.1 root files remain deleted. Their 953 recorded headings are
mapped by [V0_1_DOCUMENT_MIGRATION.md](../docs/migrations/V0_1_DOCUMENT_MIGRATION.md),
which preserves engine destinations, private creative destinations, superseded
claims, and rejected assumptions.

The retired `IMPLEMENTATION_PHASES.md`, its P0-P29 roadmap, conflicting renderer
subpackage labels, combined weather/environment/simulation headings,
`docs/adr/`, old benchmark shorthand, and generic scaffold names are mapped by
[V0_3_ROADMAP_MIGRATION.md](../docs/migrations/V0_3_ROADMAP_MIGRATION.md).
Both historical ledgers report zero unmapped entries. The
[v0.4 Alluvium amendment ledger](../docs/migrations/V0_4_ALLUVIUM_AMENDMENT.md)
maps every prior procedural heading and amendment subject with zero unmapped rows.
The [v0.5 general-purpose platform ledger](../docs/migrations/V0_5_GENERAL_PURPOSE_PLATFORM_AMENDMENT.md) maps all 40 review areas, 20 resolved decisions, and 6 stale prompt terms: 66 mapped rows and zero unmapped rows.
The [v0.5 Marquee amendment ledger](../docs/migrations/V0_5_MARQUEE_AMENDMENT.md) maps all promotional decisions and prior headings with zero unmapped rows.
The [v0.5 competitive performance and environmental quality amendment ledger](../docs/migrations/V0_5_PERFORMANCE_QUALITY_AMENDMENT.md) maps all 11 amendment subjects with zero unmapped rows.
The [v0.5 Meridian UI 1.0 amendment ledger](../docs/migrations/V0_5_MERIDIAN_UI_1_0_AMENDMENT.md) maps all 24 interview/brief/package subjects with zero unmapped rows.

## 5. Open research, not contradictions

These remain deliberately unresolved behind registered gates:

- Penumbra successor candidates after MS-05 (`RG-PEN-001`);
- native backend entry/maintenance (`RG-RHI-001`);
- virtual geometry, GI, ray, shadow, and upscaling portfolios;
- Meridian UI display-list renderer;
- Cairn solver/layout/determinism envelopes;
- ECS replacement timing;
- Isobar/Basalt/Torsant production algorithms;
- Alluvium evaluator/kernel portfolio (`RG-PRC-001`) and evidence-gated
  dependency replacement (`RG-PRC-002`);
- signing cryptography and key-management implementation;
- native-modeling kernels, animation compression/retargeting, navigation representation, 2D algorithms, and shader compiler/target implementation behind their adopted seams;
- VCS lineage, sync conflict models, network transports/providers, Collective providers, mod sandbox, XR extensions, Wavefront propagation, and additional gameplay languages;
- every post-1.0 `PRG-*` program after its entry gates.
- Marquee media/audio/PDF adapter selection after MS-10 (`RG-PRM-001`).
- competitive comparator/method/expiry/automation selection after MS-10
  (`RG-REL-001`); every performance, quality, and workflow result remains
  workload/version/profile-specific;

ResearchReady means candidates, seams, corpus, metrics, owner, decision rule,
and archive policy are specified. It does not mean a candidate is adopted.

## 6. Acceptance

This migration closes only when `meridian-spec check` validates documents,
schemas, maturity, evidence, workloads, ADRs, and zero-unmapped IDs; active docs
contain no retired authority; private content remains absent; Cargo and Rust
gates pass; current smokes retain honest limits; and the private game index
records the AMI/redacted benchmark boundary. Later changes update this register
rather than silently reviving older decisions.
