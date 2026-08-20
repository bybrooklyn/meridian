# Normative-language audit — lowercase `must` inside strictly-Normative sections

> **Status: proposal, withdrawn from the specoment.** An earlier edit amended §0.3 so that
> requirement strength derived from the maturity label on the owning contract rather than letter
> case. That was the drafting agent's own decision and was made without owner approval, so it has
> been reverted. §0.3 stands at its original wording.
>
> The finding that prompted it still holds: the 243 candidates below are **not** a subset to
> promote. Effectively all of them carry genuine requirement force, so "audit rather than blindly
> uppercase" resolves to "promote nearly all of them" — which is why the convention itself looked
> like the wrong thing to keep.
>
> Two resolutions remain open, and both are the owner's to take:
>
> 1. Promote these 243 clauses to upper case, keeping §0.3 as written.
> 2. Reinstate the amendment, making the maturity label authoritative and casing decorative.
>
> Doing neither leaves §0.3 describing a convention the document does not follow.

Source: `MERIDIAN_SPECOMENT.md`  ·  candidates: **243** across **180** sections.

Section 0.3 reserves uppercase RFC-2119 keywords for explicit requirement strength. These are the
lines where lowercase "must" sits inside a heading explicitly labelled *Normative* (excluding
*Normative direction*), so each is a candidate for either promotion to `MUST` or rewording to
ordinary prose. This is an audit list, **not** a list of 663 bugs and **not** a mandate to
uppercase mechanically — judge each line for whether it is actually carrying requirement force.

## Densest sections

- 6 — *Basalt world scale, partitioning, publication, HLOD, and persistence `WORLD-016` — *Normative**
- 4 — *Post-1.0 Meridian Shader Language `PEN-020` — *Normative**
- 4 — *One-button Vegetation & Ecosystems capability `WORLD-015` — *Normative**
- 4 — *Automatic-first semantic rig import and qualification `ARTUS-001` — *Normative**
- 4 — *Research doctrine and clean implementation `RESEARCH-002` — *Normative**
- 3 — *Vertical playable results `PLAY-001` — *Normative**
- 3 — *Penumbra `PEN-001` — *Normative ambition and architecture**
- 3 — *Complete spatial-audio portfolio for 1.0 `WAVE-005` — *Normative**
- 3 — *Unified composable Motion Source contract `ARTUS-003` — *Normative**
- 3 — *Production motion matching and pose search in the Artus 1.0 floor `ARTUS-004` — *Normative**
- 3 — *`.gitignore`, local excludes, and `.meridianignore` `SCM-006` — *Normative**
- 2 — *Meridian’s north star `PURPOSE-001` — *Normative**
- 2 — *Full ambition remains active `PURPOSE-003` — *Normative**
- 2 — *Creator joy contract `PRODUCT-003` — *Normative**
- 2 — *Professional-software editor UX quality bar `APP-004` — *Normative**
- 2 — *Automatic performance contract `PRODUCT-004` — *Normative**
- 2 — *Systematic subsystem interoperability coverage `SUBSYS-005` — *Normative**
- 2 — *Coherent end-to-end subsystem cooperation `SUBSYS-006` — *Normative**
- 2 — *Export ergonomics `LANG-004` — *Normative model; syntax research-gated**
- 2 — *Renderer-free architecture `RUNTIME-001` — *Normative**

## Candidates


### 0.3 Normative language (heading line 97)

- `:106` - **Research-gated:** candidates remain open and must be selected by evidence.

### Meridian’s north star `PURPOSE-001` — *Normative* (heading line 207)

- `:209` Meridian is intended to become a **really powerful, general-purpose game engine** and one deeply integrated creator application. It must be capable of supporting:
- `:227` Private consumer games drive real production requirements but **must not become Meridian’s public engine contract**. Project-specific rules, movement, narrative, balance, unusual simulation, and creative data remain project-owned.

### Existing purpose is preserved `PURPOSE-002` — *Normative* (heading line 240)

- `:242` The rewrite must preserve the intent found in the existing source:

### Full ambition remains active `PURPOSE-003` — *Normative* (heading line 258)

- `:260` The complete specification rewrite must not shrink Meridian’s long-term vision merely to simplify the roadmap. It must distinguish:
- `:266` Every subsystem specification must explicitly include both:

### The old ten pillars survive `PRODUCT-001` — *Normative* (heading line 277)

- `:292` The expanded promises must also include:

### Ease contract `PRODUCT-002` — *Normative* (heading line 305)

- `:307` Meridian must be **extremely easy to use**. A beginner should be able to:

### Comparative ease acceptance target `PRODUCT-009` — *Normative* (heading line 332)

- `:352` Ease must not be achieved by deleting capability. Advanced systems remain accessible progressively through inspectors, expert workspaces, source, graphs, and extension APIs.

### Creator joy contract `PRODUCT-003` — *Normative* (heading line 354)

- `:356` Meridian must be **extremely fun to use**, not merely capable. This means:
- `:371` “Fun” must not be implemented as slow theatrical motion or superficial decoration that delays real work.

### Breath-of-Fresh-Air product doctrine `PRODUCT-005` — *Normative* (heading line 373)

- `:375` Meridian should feel like a **breath of fresh air to use**. This is a product-quality requirement, not merely a visual-style preference. Meridian must deliberately avoid three categories of pain they experience in existing engines: a visually awkward/inconsistent and sometimes unintuitive workflow in Unity, unnecessary complexity and ceremony in Unreal for many ordinary tasks, and insufficient breadth/depth in Godot for the complete class of games and production workflows Meridian intends to support. These are product-experience targets, not research claims that those engines are objectively ranked this way.

### Capability without clutter `PRODUCT-008` — *Normative* (heading line 431)

- `:433` Meridian must be **powerful without feeling bloated**. The product is intended to make real games, applications, tools, simulations and unusual hybrid projects; it is not intended to win a feature-count contest or expose every technically possible subsystem to every user.

### Professional-software editor UX quality bar `APP-004` — *Normative* (heading line 456)

- `:458` The Meridian editor must feel like a **mature, polished creative/development application**, not merely a collection of engine panels. The comparison target is the interaction quality users expect from strong modern software across creative tools, IDEs, DCCs, CAD tools, design tools, and other professional desktop applications. Meridian does not need to imitate any one product's visual design, but it must reach comparable levels of responsiveness, discoverability, consistency, and confidence.
- `:460` Ordinary tasks must be discoverable without requiring the developer to ask Meridian AI, search documentation, memorize hidden shortcuts, or understand subsystem internals. AI and Ponder can accelerate learning later, but the editor itself must teach through visible affordances, names, inspectors, previews, contextual commands, useful empty states, tooltips where appropriate, direct manipulation, and predictable menus/search.

### Automatic performance contract `PRODUCT-004` — *Normative* (heading line 539)

- `:541` Meridian must be **extremely well optimized** and save developers from routine optimization work. Developers declare intent:
- `:556` The governor must never silently destroy gameplay correctness or important authored behavior to meet a target. Every compromise must be inspectable and attributable.

### Developer optimization-burden minimization `PRODUCT-006` — *Normative* (heading line 558)

- `:562` For each subsystem, planning must explicitly ask which expert chores can become safe automatic policy. Examples include:

### Names and authority survive `SUBSYS-001` — *Normative* (heading line 638)

- `:640` The rewrite preserves Meridian’s named subsystem identity and authority model. It may redesign bad interfaces, implementation order, and crate boundaries, but it must not flatten the engine into anonymous generic capability labels.

### Built-ins must be excellent `SUBSYS-003` — *Normative* (heading line 695)

- `:697` Meridian’s systems must be good enough that developers normally **do not need to replace them**. Extensibility is not an excuse to ship weak defaults.

### Systematic subsystem interoperability coverage `SUBSYS-005` — *Normative* (heading line 725)

- `:727` Every subsystem specification must explicitly evaluate its relationship with **every other named subsystem and major cross-cutting capability**. The result is an integration matrix with one of three states for each pairing:
- `:751` A subsystem may never reach directly into another subsystem's private mutable state merely to make integration convenient. Cross-system loops must have explicit authority and publication boundaries so automatic coupling does not become hidden cyclic control.

### Coherent end-to-end subsystem cooperation `SUBSYS-006` — *Normative* (heading line 755)

- `:757` `SUBSYS-005` guarantees that every relevant pairing is reviewed. `SUBSYS-006` adds the stronger product requirement: **Meridian's first-party systems must actually work well together in complete workflows.** A technically correct pairwise API is not sufficient if the developer still has to manually glue several first-party systems together to make an ordinary game feature work.
- `:759` Relevant subsystems should share semantic identities, lifecycle/publication rules, transaction/source mapping, diagnostics vocabulary, performance-governor signals, save/network classifications, and capability discovery where doing so prevents duplicate configuration or brittle adapters. Shared foundations must not erase subsystem ownership.

### Editor sequencing `APP-002` — *Normative* (heading line 834)

- `:836` A minimal functional editor must remain available while the runtime matures. The eventual editor must be genuinely excellent, which takes time.

### Stable source and recovery `AUTHOR-001` — *Normative* (heading line 876)

- `:878` Authoritative project data must be:

### Isolated importer execution `AUTHOR-004` — *Normative* (heading line 927)

- `:931` Importer workers must support:

### Layered reimport and protected authoring work `AUTHOR-006` — *Normative* (heading line 967)

- `:983` Reimport must be previewable, cancellable, transactional, recoverable, and semantically reviewable in Meridian VCS.

### Local-first and trusted shared caches `AUTHOR-007` — *Normative* (heading line 985)

- `:999` Routine cache use is automatic after trust is granted. Every object is verified before use. Outages fall back to local work without making the project unusable. The editor must explain hits, misses, rejected/corrupt objects, rebuild causes, source latency, and why a particular cache source was selected.

### Shared authoring graph with explicit roles `AUTHOR-008` — *Normative* (heading line 1003)

- `:1033` The editor must explain role restrictions and offer safe conversions or extraction operations where possible.

### Incremental compilation into subsystem-owned runtime products `AUTHOR-009` — *Normative* (heading line 1059)

- `:1090` The user-facing acceptance target is Godot-like immediacy or better for ordinary edits, despite the stronger compiled-runtime separation. The Godot research specifically identifies this authoring-graph-to-typed-runtime synthesis as a hypothesis that must be judged on iteration latency, patch behavior, memory, debugging, merge quality, and implementation complexity.

### Automatic stable scene partitioning `AUTHOR-010` — *Normative* (heading line 1092)

- `:1119` Meridian should continuously measure when automatic repartitioning helps versus creates churn. It must not silently reshuffle the repository during ordinary edits.

### Asset Catalog, compiler, and CAS closure `AUTHOR-DER-002` — *Normative* (heading line 1194)

- `:1198` **Architecture status:** The product-level Asset Catalog, compiler, and CAS contract is complete for this specification. Prototypes must still prove corruption recovery, huge-project scale, partial workspaces, source-move stability, `why rebuilt?`, cache provenance and no-cloud-required operation.

### Vertical playable results `PLAY-001` — *Normative* (heading line 1204)

- `:1206` Every implementation phase must produce a concrete, executable, user-visible result. Structural tests and screenshots are supporting evidence, not phase completion.
- `:1208` A phase must include as applicable:
- `:1220` No phase may become an endless isolated subsystem marathon. Foundations must be consumed by a playable or usable result soon after they are created.

### First creator-loop proof `PLAY-002` — *Normative* (heading line 1222)

- `:1224` The first major visible milestone must prove a real create/edit/play/save/build workflow. The game can be ugly and tiny, but it must use supported public Meridian APIs and tools.

### User-facing source model `LANG-002` — *Normative* (heading line 1296)

- `:1298` Polyglot support must be simpler than explicit user-managed gameplay modules.

### Language-neutral interface schema `LANG-003` — *Normative* (heading line 1325)

- `:1343` Each language receives an idiomatic facade. Meridian must not force every language to imitate Rust.

### Export ergonomics `LANG-004` — *Normative model; syntax research-gated* (heading line 1347)

- `:1349` Cross-language APIs must be explicit enough to avoid exporting every public implementation detail, but extremely easy to create. Meridian must not require ugly foreign-function boilerplate, manual ABI files, language-pair bridges, or “stupid” syntax.
- `:1356` The editor must be able to promote a lightweight export into a formal interface without rewriting the implementation by hand.

### Toolchain acquisition and provenance `BUILD-002` — *Normative* (heading line 2073)

- `:2086` Advanced developers and studios may override a managed toolchain with an approved system installation, Nix environment, container, SDK image, or internally distributed toolchain. Overrides must be explicit, qualified, reproducible, and visible in the build report. Meridian must never silently use an arbitrary executable found first on `PATH` when reproducibility matters.

### Web compilation and browser runtime `BUILD-005` — *Normative* (heading line 2115)

- `:2117` Meridian games must be able to compile and run in modern web browsers. Web is an official shipping target, not a demo-only exporter.

### Canonical web artifact and deployment `BUILD-006` — *Normative* (heading line 2278)

- `:2293` The exact filenames and chunk layout may evolve, but the result must be hostable on an ordinary static server or CDN without a Meridian cloud dependency.

### Target-specific source and capability adaptation `BUILD-007` — *Normative model; syntax research-gated* (heading line 2297)

- `:2305` - target-specific behavior that changes gameplay, networking, saves, determinism, or authority must be declared rather than hidden;

### Renderer-free architecture `RUNTIME-001` — *Normative* (heading line 2314)

- `:2316` The world/ECS/gameplay/task/asset/save/network foundations must work without:
- `:2325` The current direct ECS/runtime dependence on renderer concepts must be removed. Simulation publishes presentation snapshots; rendering consumes them.

### Aggressively automatic async `RUNTIME-002` — *Normative* (heading line 2340)

- `:2356` Meridian must not secretly rewrite arbitrary mutable code into concurrent code or make all calls invisibly asynchronous.

### Interactive-first startup contract `RUNTIME-003` — *Normative* (heading line 2358)

- `:2360` Normal installed launches must become useful quickly rather than hiding eager initialization behind a long splash screen.
- `:2384` The first interactive frame must not wait for unrelated source-control synchronization, remote cache contact, documentation indexing, shader compilation, language-toolchain checks, asset discovery, telemetry, or optional subsystem startup.

### Executor selection is automatic `RUNTIME-006` — *Normative* (heading line 2444)

- `:2446` Game developers do **not** manually choose executor names, worker pools, thread counts, or subsystem queues. Meridian does not expose Unity-style game scripting as manual thread topology, and it must be at least as easy.

### Deterministic gameplay-result barriers `RUNTIME-008` — *Normative* (heading line 2486)

- `:2488` Background completion order must not mutate authoritative gameplay state arbitrarily.

### Official framework package contract `FWK-001..003` — *Normative* (heading line 2524)

- `:2539` **Architecture status:** The official-framework product contract is complete for this specification. Exact controller algorithms, presets, template tuning, and which specialized family reaches production quality first remain `PROTOTYPE / EVIDENCE` work. Language qualification must still test these packages from Rust, C#, Luau, TypeScript, Meridian Python, and C/C++.

### No AI-generated game assets `AI-004` — *Normative* (heading line 2624)

- `:2626` Meridian’s AI features must not encourage or silently create AI-generated game art, music, voice, models, textures, or other audiovisual assets. Project recommendations and built-in workflows prioritize human-created work, licensed assets, manual creation/editing, and procedural systems.

### Current implementation is not sacred `UI-001` — *Normative* (heading line 11986)

- `:12002` Every crate/part must be independently evaluated. Internals may be rewritten freely.

### `.mui` source `UI-002` — *Normative* (heading line 12025)

- `:12039` - syntax must remain genuinely easy and visually round-trippable rather than ceremonious or language-designer cleverness.

### Native nine-slice / nine-patch rendering `UI-011` — *Normative* (heading line 12182)

- `:12215` Authoring must be unusually easy. A designer can select an image/background and choose **Nine Slice**; the visual editor then overlays four draggable border guides and synchronized numeric fields. Asset import may optionally store reusable patch-border metadata so many controls can share one definition, while an individual `.mui` use may override it explicitly. Round-tripping preserves the developer's chosen source form.
- `:12223` - nine-slice evaluation must not allocate nine child controls;

### Penumbra `PEN-001` — *Normative ambition and architecture* (heading line 12273)

- `:12275` Penumbra remains Meridian’s owned renderer, not a thin public wrapper. **Exceptional visual quality is a from-the-start product requirement, not an eventual destination.** Bootstrap renderer bring-up may temporarily be visually minimal, but the first serious production Penumbra milestone must already be capable of beautiful, modern, compelling real-time graphics on its qualified target hardware. Later phases increase scale, fidelity, physical richness, automation and platform breadth rather than postponing visual excellence until the end.
- `:12277` Penumbra must support scalable fallback paths suitable for small games through AAA-scale productions without treating the lowest common denominator as the artistic baseline. High-end and fallback products share one authoring model so a developer can build something visually impressive first and let Meridian derive appropriate lower-tier products automatically.
- `:12279` Research must cover Unreal, Unity, Godot, O3DE/Atom, Flax, Stride, Bevy, Fyrox, modern papers, GPU architecture, virtual geometry, GI, temporal rendering, visibility, streaming, materials, ray tracing, native APIs, web graphics, and production tooling. Meridian must take the useful production lessons without becoming Unreal, Godot, or another engine rewritten in Rust.

### Visual-quality acceptance and vertical-slice doctrine `PEN-021` — *Normative* (heading line 12281)

- `:12285` A production Penumbra slice must combine enough of the complete presentation stack to judge the image honestly:
- `:12302` Penumbra's high-end visual path and its lower-tier/web paths are developed from the same semantic source model. The renderer may substitute techniques automatically; it must not force the project to choose between “beautiful” and “portable” as unrelated authoring modes.

### Cohesive global illumination portfolio `PEN-004` — *Normative* (heading line 12324)

- `:12337` The portfolio must feel like one renderer, not unrelated renderers sharing a settings page. Techniques share source semantics, diagnostics, profile selection, fallback declarations, temporal rules, visibility assumptions, and authoring previews. The exact common scene representations and cache architecture require a dedicated Penumbra round and prototypes.

### Automatic scalability `PEN-005` — *Normative* (heading line 12339)

- `:12343` Automatic decisions must remain inspectable and explainable. Expert overrides exist, but manual construction of every quality tier is not the default workflow. Penumbra may reduce visual fidelity; it must not silently alter gameplay-critical visibility, interaction, simulation, or collision semantics.

### Backend portfolio and Vulkan requirement `PEN-007` — *Normative* (heading line 12361)

- `:12363` Penumbra is Meridian's renderer. `wgpu` is the current GPU API/backend foundation behind Meridian-owned RHI contracts; it is not the complete renderer and must not leak raw public types into game, material, asset, or extension contracts.
- `:12380` **Vulkan is not optional research.** It is a required Penumbra target wherever the operating system, device, driver ecosystem, and product profile provide a viable Vulkan path. At minimum this includes first-class qualification for Linux/Steam Deck-class systems and other Vulkan-native platforms, plus supported and benchmarked Vulkan execution on Windows alongside Direct3D 12. Direct3D 12 is the ordinary Windows default. Vulkan remains a fully supported, benchmarked, user-selectable alternative and may be chosen by an explicit project/profile rule or recovery path. Meridian must not omit or neglect Vulkan merely because Direct3D 12 is the default or because `wgpu` exists.

### Rust-native shader authoring and language roadmap `PEN-008` — *Normative* (heading line 12388)

- `:12390` Penumbra's mandatory shader path must be buildable through Cargo and Rust-native tooling. It must not require CMake or CPython merely to build Meridian, edit a material, compile a shader, run tests, or package a normal game.

### Generated-product ownership `PEN-013` — *Normative* (heading line 12500)

- `:12504` An explicit **Convert/Detach to Custom Shader** operation creates developer-owned source and records the generated origin. Once detached, Meridian no longer promises full managed regeneration or arbitrary round-trip reconstruction. The UI must show which portability, fallback, variant, and maintenance responsibilities the developer accepted.

### Asynchronous last-known-good compilation `PEN-016` — *Normative* (heading line 12524)

- `:12535` The editor must not freeze because shader compilation is active. Visible viewport and current-target jobs receive priority. Unused profile products may compile later or during Build. Workers are cancellable, restartable, isolated where appropriate, and connected to the content-addressed asset/compiler graph. Failures never replace the last valid shader. Shipping products precompile or qualify required pipelines so ordinary gameplay does not discover major shader work through stutter.

### CMake/CPython-free shader-build contract `PEN-018` — *Normative* (heading line 12598)

- `:12609` must not invoke CMake, require a CPython interpreter, or compile a C++ shader-language frontend.
- `:12611` Platform SDK tools may still be required to produce final native platform binaries where the platform itself requires them—for example Metal tools on Apple platforms or a qualified HLSL-to-DXIL compiler on Windows—but those are target-product adapters, not the source-language/compiler foundation. The production graph must state exactly which external platform tool ran and why.

### Post-1.0 Meridian Shader Language `PEN-020` — *Normative* (heading line 12640)

- `:12644` It is an independent language rather than a strict WGSL or WESL superset. WGSL and WESL remain permanently supported source languages and interoperate through Material IR, shader-interface schemas, imported modules/wrappers, stable generated bindings, and source maps. Existing WGSL/WESL source must not become silently Meridian-only.
- `:12646` The first complete release requires the professional core rather than a toy subset. Advanced research features may land through named later programs, but Meridian must not market the language as complete while modules, separate compilation, generics, interfaces/traits, specialization, reflection, capability constraints, graphics/compute stages, multi-target lowering, and excellent tooling are missing.
- `:12650` It must not be a minimal material DSL. Its planned capability envelope includes:
- `:12671` The post-1.0 language program must begin with concrete shader-library workloads from Shape of Down, Project Meridian, the Penumbra validation portfolio, web builds, native Vulkan, native Metal, and native Direct3D 12. Syntax and compiler architecture are decided through prototypes rather than aesthetics alone.

### First-class 2D `2D-001` — *Normative* (heading line 12673)

- `:12675` 2D is not flat 3D. Meridian 1.0 must include a dedicated first-class path:

### Cairn physics architecture `CAIRN-001` — *Normative ambition and core architecture* (heading line 12705)

- `:12709` Cairn's public product must be easy enough for a beginner to obtain excellent behavior by adding and configuring components, while remaining deep enough for unusual games and AAA-scale simulation without requiring an engine fork. Extensibility exists through deliberate physical contracts, typed fields, strategies, controller packages, specialized solvers, and advanced provider boundaries; the built-in path remains canonical and should normally be better than replacement.

### Shared vocabulary, dimension-specific physics `CAIRN-002` — *Normative* (heading line 12711)

- `:12739` Pure 2D profiles must not initialize hidden 3D solver state or carry avoidable 3D runtime cost. Mixed 2D/3D projects may explicitly bridge the domains without pretending their coordinates or contacts are naturally interchangeable.

### Universal hierarchical coordinate model `CAIRN-003` — *Normative* (heading line 12741)

- `:12755` The model must coordinate with Basalt, Penumbra, NAV, Wavefront, networking, saves, replays, streaming, and editor gizmos. Frame transitions need stable identity, deterministic ordering where requested, source-linked diagnostics, interpolation rules, precision budgets, and tests for high-speed crossing. Developers ordinarily author positions and relationships without manually managing origin rebasing.

### Typed fields and force providers `CAIRN-005` — *Normative* (heading line 12806)

- `:12834` The framework must support Shape of Down's proximity-based surface gravity, orbit and slingshot behavior, moving gravity sources, surface handoff, gravity-relative movement, and field visualization without hardcoding that game's rules into Cairn.
- `:12836` Hot field work is automatically batched, vectorized, parallelized, or lowered into native/GPU-capable kernels where qualified. Supported gameplay languages may author fields and policies, but Cairn must avoid a dynamic script callback per body-field pair. The editor/profiler provides vector/scalar visualization, influence volumes, orbit previews, handoff traces, source provenance, instability diagnostics, cost attribution, and deterministic replay inspection.

### Hybrid structural destruction `CAIRN-006` — *Normative* (heading line 12840)

- `:12870` The asset compiler must cache fracture products, interior geometry, collision products, network/server facets, quality tiers, and platform-specific representations. Alluvium may author or modify structural fields procedurally; Basalt and Torsant may publish material/environment state; Cairn retains structural and mechanical authority.

### Deformable ownership and coupling `CAIRN-007` — *Normative* (heading line 12872)

- `:12910` Ownership boundaries prevent Cairn from swallowing Torsant and prevent Artus from becoming a hidden physics solver. They must not force developers to manually wire routine interactions.

### Layered first-party vehicle framework `CAIRN-008` — *Normative* (heading line 12912)

- `:12944` The system must also support motorcycles, tracked vehicles, hovercraft, spacecraft, boats/amphibious vehicles, rail/constrained vehicles, and gravity-relative vehicles without forcing them through fake wheel semantics. Non-wheel vehicles share only concepts that genuinely apply.

### Qualified hybrid CPU/GPU physics `CAIRN-010` — *Normative* (heading line 12974)

- `:13001` Every essential Cairn feature must either have a viable selected-platform implementation or fail the profile with a precise explanation. Meridian does not silently drop gameplay physics on unsupported targets.

### Unified immediate, snapshot, and batched queries `CAIRN-013` — *Normative* (heading line 13061)

- `:13091` The ordinary API must stay straightforward. Manual selection of synchronous, snapshot, SIMD, worker, or GPU APIs is an advanced override, not normal gameplay boilerplate.

### Stable simulation clock and automatic refinement `CAIRN-014` — *Normative* (heading line 13093)

- `:13109` The editor and profiler must explain the effective schedule, for example:

### Cairn explanation and observability suite `CAIRN-015` — *Normative* (heading line 13121)

- `:13123` Cairn must be unusually understandable. Selecting a physical object should answer concrete questions such as:
- `:13157` A high-level **Explain this motion** action uses Ponder to summarize the causal chain without hiding the raw evidence. Instrumentation must remain cheap enough for development and controlled shipping diagnostics, and must integrate with Meridian’s cross-subsystem tracing rather than becoming an isolated physics profiler.

### Unified World Environment authoring `WORLD-002` — *Normative* (heading line 13175)

- `:13217` Every coupling must be inspectable, explainable, source-mapped, individually disableable, and replaceable at a deliberate seam. The editor must answer **what is connected, who owns the state, why a change occurred, what it costs, and which profile/fallback is active**.

### Basalt multi-representation terrain and world geometry `WORLD-003` — *Normative* (heading line 13219)

- `:13253` Initial delivery may begin with strong heightfield and mesh paths, but source schemas, IDs, streaming, editing, and consumer contracts must not hard-code those as the only possible terrain forms.

### Water, snow, ice, mud, and flooding ownership handoff `WORLD-005` — *Normative* (heading line 13272)

- `:13320` The transition must preserve mass/energy within declared tolerances, save/replay identity, network authority, source provenance, and clear diagnostics. A developer should be able to inspect why a puddle became a fluid simulation or why a flood region was reduced back to coarse state.

### Built-in plant profile and example library `WORLD-007` — *Normative* (heading line 13368)

- `:13370` Meridian must ship a useful, extensible library of **profiles for real existing plants**, reusable plant archetypes, and example communities. A developer should be able to choose a known plant or ecosystem profile and immediately receive coherent generation, growth, weather, fire, physics, rendering, and optimization defaults rather than building every species from zero.
- `:13455` Actual numerical values and ecological claims must come from versioned, attributable botanical, forestry, agricultural, and ecological sources. Meridian should not invent precise species behavior because a profile name sounds plausible. Profiles need confidence, geographic scope, source provenance, and an explicit distinction among measured facts, broad defaults, simplified gameplay approximations, and project-authored overrides.

### Hybrid plant geometry `WORLD-011` — *Normative* (heading line 13547)

- `:13558` The biological/profile layer constrains plausible form and response; the appearance pack controls style and quality. Meridian must support realistic, stylized, low-poly, 2D, web, and project-specific art without binding ecological meaning to one mesh set.

### Selectable growth tiers `WORLD-012` — *Normative* (heading line 13560)

- `:13579` The selected tier may vary by project, biome, species, population, or promoted individual. Distant plants may remain aggregate even when nearby specimens use parametric or ecological state. Tier changes must preserve durable identity and compatible state or produce an explicit migration report.

### Aggregate ecosystem simulation `WORLD-014` — *Normative* (heading line 13592)

- `:13608` Only important specimens are promoted to detailed persistent entities. The aggregate model must remain explainable, saveable, deterministic under declared profiles, headless-capable, and compatible with representation promotion/demotion.

### One-button Vegetation & Ecosystems capability `WORLD-015` — *Normative* (heading line 13610)

- `:13619` The exact manifest key and UI wording remain open, but **Off must be a genuine build boundary**, not merely a hidden panel.
- `:13621` When Off, Meridian must exclude or avoid initializing:
- `:13647` Narrow generic shared foundations may remain only where removing them would create a worse architecture, and their exact size/cost must be measured and documented.
- `:13659` If a project already contains vegetation-specific content, the editor must preview consequences and offer deliberate actions:

### Basalt world scale, partitioning, publication, HLOD, and persistence `WORLD-016` — *Normative* (heading line 13672)

- `:13674` Basalt uses one durable world architecture for tiny rooms, ordinary levels, giant facilities, open worlds, planetary spaces, and orbital-scale projects. The architecture must collapse automatically when a project does not need large-world machinery.
- `:13690` The authored source still uses durable world/entity/feature identities and compatible transforms, so the project can grow later without a destructive format migration. The build report must prove which large-world systems were removed and their measured residual cost.
- `:13719` Automatic partitioning must use stability/hysteresis rules so trivial edits do not reshuffle thousands of files, cells, hashes, or review diffs. Repartitioning must not change durable world, entity, prefab, asset, save, or network identities. The editor continues to present one logical world; physical source sharding and runtime cells remain inspectable implementation products.
- `:13739` Preparation may occur concurrently. A region crosses a declared publication barrier only when every product required by the active project/profile is valid. Meridian must never create a world where a visible floor lacks collision, an interactive door lacks authoritative gameplay state, navigation points into unloaded geometry, or audio/occlusion data references a different region version.
- `:13778` The editor must answer **why was this merged, simplified, retained, loaded, evicted, or rebuilt?** and show visual, physical, memory, storage, bandwidth, and patch-size consequences. Experts may pin, replace, or author a derived product deliberately, but ordinary projects rely on the compiler and performance governor.
- `:13804` The save system streams, indexes, compacts, migrates, and validates patches by durable region and identity. It does not save one enormous arbitrary live object graph. Procedural regeneration must preserve protected overrides and report conflicts. Multiplayer authority, late join, replay, rollback, branch migration, and unloaded-region updates all use the same semantic state contract.

### Wavefront audio, music, acoustics, and web runtime `WAVE-001` — *Normative ambition and initial architecture* (heading line 13830)

- `:13834` Wavefront must feel simple for ordinary games while still being deep enough for production audio. A small project should be able to import a sound, assign it to Music/SFX/Ambience/Dialogue, place a spatial source, and play it without understanding real-time DSP scheduling. An audio programmer or technical sound designer must be able to inspect the complete graph, timing, latency, routing, source priority, spatialization, decode state, and device path.
- `:13849` The core real-time rule is stricter than “audio should be fast”: the output/render callback must operate from prevalidated bounded state and must not depend on ordinary game/editor locks, unbounded allocation, filesystem access, network access, asset import, arbitrary scripting, or graph mutation.

### Competing bounded implementation prototypes `WAVE-002` — *Normative* (heading line 13851)

- `:13869` The prototypes must implement equivalent representative contracts before comparison. At minimum, both need:

### Immutable compiled real-time graph `WAVE-003` — *Normative* (heading line 13903)

- `:13944` Snapshot retirement must be epoch/fence based or otherwise safe for the audio thread; the exact reclamation mechanism remains open pending prototype measurements.

### Simple bus inspector and expert typed graph `WAVE-004` — *Normative* (heading line 13946)

- `:13980` The graph editor must expose “why” information: why a conversion node exists, why a path has additional latency, why a node was disabled for web, why a source was virtualized, or why a feedback loop is rejected.

### Complete spatial-audio portfolio for 1.0 `WAVE-005` — *Normative* (heading line 13982)

- `:13984` Wavefront 1.0 must provide a staged but production-qualified spatial portfolio rather than basic left/right panning with HRTF deferred indefinitely.
- `:14013` Selection may consider source importance, distance, audibility, dialogue priority, player ownership, cinematic status, CPU budget, and target capability. Gameplay-critical semantic cues must not disappear silently because a spatial quality tier changes.
- `:14015` HRTF datasets/resources need clear provenance and licensing. Meridian must not bundle questionable measured datasets merely because a renderer supports them.

### Production adaptive-music transport for 1.0 `WAVE-006` — *Normative* (heading line 14017)

- `:14039` This is **authored interactive music** infrastructure. It does not require generative AI music and must work entirely offline.

### Cohesive tiered acoustics `WAVE-007` — *Normative* (heading line 14041)

- `:14078` The baseline acoustic tier must be cheap enough for ordinary games and entirely disableable as a capability family where not needed. Higher tiers degrade to lower tiers through qualified policies, not by silently disabling dialogue audibility or gameplay-critical sound cues.

### Dedicated Web AudioWorklet product `WAVE-008` — *Normative* (heading line 14080)

- `:14094` The web product must explicitly handle:
- `:14120` The default policy remains honest portability: ask or apply only qualified non-gameplay-breaking fallbacks according to the project's web fallback policy. Unsupported audio behavior must not simply fail at runtime in the browser.

### Automatic-first semantic rig import and qualification `ARTUS-001` — *Normative* (heading line 14164)

- `:14181` Every inferred mapping or value that materially affects runtime behavior carries provenance and confidence. The editor must distinguish at least:
- `:14192` Artus performs automatic validation motions and static checks after canonicalization. The qualification set must grow over time but includes representative poses and movements such as neutral stance, deep crouch, arm reach, cross-body reach, walk, run, turn, stair/step motion, foot plant, sitting alignment, head-look extremes, and ragdoll transition/recovery entry. Validation identifies flipped axes, twisted limbs, wrong hierarchy, implausible limits, foot penetration, handedness errors, scale mismatch, missing semantics, broken retarget chains, and severe corrective-bone artifacts.
- `:14194` Manual corrections are durable source data. Reimport must never silently overwrite confirmed mappings or contact edits. If source topology changes enough to invalidate an override, the asset compiler produces a structured conflict explaining what changed and which semantic role needs review.
- `:14196` The automatic-first contract does **not** lock one rig-detection algorithm. Name heuristics, topology analysis, geometry measurements, known-rig profiles, statistical methods, or later optional models may compete behind the same contract. The output must remain deterministic or explicitly provenance-recorded where deterministic reproduction is impossible.

### Motion Profiles as the ordinary authoring workflow `ARTUS-002` — *Normative* (heading line 14200)

- `:14240` Automatic configuration must be source-preserving. Artus distinguishes profile-managed state, explicit project overrides, and a fully detached/custom pipeline. Regeneration cannot silently delete or rewrite manual technical-animation work. When a profile update conflicts with project overrides, Meridian presents a semantic diff and migration choice.
- `:14244` Profile simplicity must not hide runtime cost. The inspector exposes estimated CPU/GPU/memory/database cost and the effective LOD/network/browser products. Disabled profile features must compile away under the engine-wide zero-cost capability principle where practical.

### Unified composable Motion Source contract `ARTUS-003` — *Normative* (heading line 14246)

- `:14288` Composition is typed and ordered by semantic masks, priorities, additive/replacement semantics, contact ownership, timing, authority, and constraint policy. The implementation must not devolve into arbitrary components mutating bone arrays in unspecified order.
- `:14290` Canonical semantic rig space is the preferred interoperability domain where it preserves correctness. Sources that require target-rig or physical-body space declare that explicitly. Retargeting and canonicalization must preserve source provenance so a final bad wrist pose can be traced back to the contributing source and retarget step.
- `:14294` The Artus debugger must expose the final contribution map by body region, including source identity, blend weight or authority, contact constraints, physical influence, and relevant fallback reason.

### Production motion matching and pose search in the Artus 1.0 floor `ARTUS-004` — *Normative* (heading line 14296)

- `:14313` The motion-database compiler must support a versioned feature schema and content fingerprint. Candidate features include current pose/joint positions and velocities, root velocity/angular velocity, sampled future trajectory, facing, gait, motion phase, semantic contact state, style, carried-object state, slope/environment class, injury/load state, and other project-defined features through typed extension points.
- `:14328` The editor must visualize requested trajectories, selected candidates, top alternatives, cost terms, hard-filter rejections, contact mismatches, continuity penalties, and areas where the library has inadequate motion coverage. Ponder may explain a selection, but raw costs and source clips remain inspectable.
- `:14332` Qualification includes representative locomotion libraries, poor/incomplete libraries, retargeted libraries, arbitrary-gravity trajectories where applicable, abrupt direction changes, terrain variation, carried objects, multiplayer correction, web products, and long-run candidate stability. Search performance must report tail latency rather than only average query time.

### First-party Smart Interactions `ARTUS-005` — *Normative* (heading line 14334)

- `:14375` Low-importance interactions may use runtime inference for reachable surfaces, handles, rails, flat seats, step edges, brace planes, or grasp regions. Important interactions can specify exact authored contacts, timing, and variants. Automatic inference must never overwrite or silently weaken explicit authored precision.
- `:14377` The architecture must support future paired/multi-character interactions—cooperative carry, assisted climbing, handoff, struggle, medical assistance, synchronized cinematic contact—without requiring a separate body-motion system. Shared timing, contact ownership, network authority, and failure/recovery are explicit parts of that future extension.

### Continuous region-specific physical-animation composition `ARTUS-006` — *Normative* (heading line 14381)

- `:14406` - which contacts must remain physically authoritative.

### Artus implementation boundaries and reopening conditions — *Normative* (heading line 14473)

- `:14489` A locked contract may be reopened only if prototypes demonstrate a material failure in performance, authoring latency, determinism, portability, web viability, networking correctness, maintenance complexity, or achievable quality. Reopening must name the affected ``ARTUS-*`` decision, present the evidence, and propose a replacement that preserves as much of the accepted user-facing intent as possible.

### AAA games `AAA-001` — *Normative* (heading line 14491)

- `:14493` AAA games must be technically makable in Meridian. AAA capability includes much more than rendering:

### Authoritative source, large assets, and derived data `SCM-003` — *Normative* (heading line 14672)

- `:14676` The source model must support:

### `.gitignore`, local excludes, and `.meridianignore` `SCM-006` — *Normative* (heading line 14737)

- `:14739` Git compatibility includes ignore behavior. Meridian VCS must honor ordinary `.gitignore` files, including nested ignore files, and preserve compatibility with repository-local and user-level Git exclusions where applicable.
- `:14754` If a `.meridianignore` rule would also need to prevent source tracking, Meridian must either add/offer the corresponding `.gitignore` rule or diagnose the mismatch. It must not silently create a repository that looks clean in Meridian but dirty or trackable in Git.
- `:14756` The editor must provide:

### Source-control validation `SCM-014` — *Normative* (heading line 14938)

- `:14940` Qualification must include:

### Complete Steam friend-session provider `NET-003` — *Normative* (heading line 15227)

- `:15261` The adapter must not force Steam DLLs/SDKs into unrelated builds. A non-Steam build using the same game project resolves different platform/social/transport providers.

### Privacy-first Internet routing `NET-005` — *Normative* (heading line 15291)

- `:15303` The connection inspector must show the actual route class without requiring packet capture:

### Built-in host migration `NET-006` — *Normative* (heading line 15316)

- `:15335` The exact checkpoint format, election algorithm, and whether some games use “warm standby” peers remain open. Migration must not pretend arbitrary native server processes, non-transferable platform objects, or external services can always move.

### `meridian-sync` remains separate — *Normative* (heading line 15337)

- `:15351` They must not share one protocol/state machine merely because both are P2P-capable. Their latency, authority, replay, data-integrity, anti-cheat, bandwidth, trust, persistence, and failure requirements are materially different.

### Safety and human/vendor gates `RELEASE-007` — *Normative* (heading line 15533)

- `:15542` - CI must not publish merely because a commit reached `main`.

### Engine license and relicensing `LEGAL-001` — *Normative* (heading line 15580)

- `:15585` - The project owner must retain enough inbound rights to license future Meridian-owned releases under MPL-2.0, Apache-2.0, MIT, GPL-3.0-or-later, LGPL-3.0-or-later, BSD, dual/multi-license, commercial, proprietary, or other chosen terms. GPL-2.0-only and LGPL-2.x are not desired Meridian release targets.

### AI-assisted contributions `LEGAL-004` — *Normative* (heading line 15621)

- `:15623` AI assistance is allowed. Accepted canonical contribution history must not credit AI systems as authors, co-authors, assistants, or contributors.
- `:15637` External contributions are rejected until **all AI-attributed commits are removed from the submitted branch/history**. The contributor must rewrite and force-push their own branch. This is not a complex adoption workflow.

### Relicensing envelope and third-party intake `LEGAL-005` — *Normative* (heading line 15641)

- `:15643` Meridian-owned code and CLA-covered contributions must preserve Dead Signal Works's **practical ability to issue future versions under substantially different licensing strategies** without asking every past contributor for new permission.

### Useful dependency adoption versus implementation mining `LEGAL-006` — *Normative* (heading line 15778)

- `:15786` This produces three distinct questions that must not be conflated:
- `:15804` - Source-available/non-open licenses require explicit legal/product review and must not quietly become required foundations.

### Public-source status `SITE-001` — *Normative* (heading line 15812)

- `:15814` The Meridian engine repository is public. Website/docs/press copy that still says the engine source is private or awaiting an open-source release must be updated.

### Documentation rewrite `SITE-004` — *Normative* (heading line 15846)

- `:15848` Meridian docs must prioritize making games:

### Human voice and anti-AI-copy standard `SITE-005` — *Normative* (heading line 15869)

- `:15871` Every public page outside the untouched footer receives a page-by-page copy audit. The rewrite must sound like a real project maintained by real people rather than a rigid generated product brief.

### Separate website/documentation improvement specification `SITE-008` — *Normative* (heading line 15912)

- `:15914` The final Meridian engine specification must cross-reference, not absorb, a separate detailed website/documentation improvement specification. That sibling document covers:

### Creator-led technical voice `SITE-009` — *Normative* (heading line 15930)

- `:15932` Public marketing and project writing should sound creator-led, technically literate, direct, and human. It may be enthusiastic, informal, or blunt where appropriate, but it must remain accurate and specific. API reference and procedural documentation use a calmer technical register while avoiding robotic phrasing.

### Single-root specoment plus derived projections `SPEC-002` — *Normative* (heading line 16432)

- `:16446` These projections must point back to stable identities in this specoment and must not become competing truth by accident. A derived file may be edited only through an explicit workflow that updates/reconciles the canonical source or through a later adopted authority change.

### Research doctrine and clean implementation `RESEARCH-002` — *Normative* (heading line 16605)

- `:16607` Research derived from source-available/proprietary engines must follow these rules:
- `:16615` - research must actively attempt to falsify Meridian's preferred ideas.
- `:16631` Before finalizing a subsystem design, research must consult the strongest available sources for that domain rather than relying on recollection or one engine. When relevant and available, this includes:
- `:16638` The research record must distinguish three layers:

### Production hybrid hand and finger system `ARTUS-008` — *Normative* (heading line 16943)

- `:16945` Artus 1.0 must ship a production-capable hand system that is more expressive than hand IK plus fixed pose clips, but it does not require fully physical per-finger simulation as the default path.

### CPU-authoritative hybrid CPU/GPU Artus execution `ARTUS-013` — *Normative* (heading line 17071)

- `:17073` Artus is designed for heterogeneous execution, but gameplay-critical semantics must remain available without requiring GPU readback or even graphics hardware.

### Full Artus motion causality, timeline, and explanation suite `ARTUS-014` — *Normative* (heading line 17099)

- `:17101` Artus automation requires unusually strong observability. The editor must provide more than graph-state inspection and skeleton overlays.

### Hierarchical long-distance path planning for streamed worlds `NAV-005` — *Normative* (heading line 17388)

- `:17390` Large-world routes must not require every fine navigation polygon/node across the entire path to be simultaneously resident. NAV supports hierarchical planning that can reason at coarse world scale and refine the route as relevant regions become available.

### First-class oriented-surface and volumetric navigation in the complete architecture `NAV-007` — *Normative* (heading line 17492)

- `:17494` NAV's architecture must not hard-code global `+Y`/world-up walking as the universal meaning of traversal. Conventional ground navigation remains the simplest and earliest implementation, while the complete system supports navigation on arbitrarily oriented surfaces and through 3D volumes.

### Unified semantic query API over versioned snapshots `NAV-008` — *Normative* (heading line 17579)

- `:17585` The final API must support cancellation, deadlines/budgets, structured failure reasons, query tracing, and deterministic/replay-qualified modes where required. No public query may accidentally force a global NAV rebuild or wait on mutable authoring state.

### Navigation regions with moving/local reference frames `NAV-011` — *Normative* (heading line 17603)

- `:17609` Basalt/world coordinates remain the durable world identity; Cairn owns physical transforms/motion; NAV owns traversal connectivity in/among navigation frames. Frame changes must integrate with streaming, server simulation, replay, and large-world precision without making a moving vehicle a separate hidden world.

### Multiplayer navigation authority profiles `NAV-012` — *Normative* (heading line 17611)

- `:17617` Navigation results used for authoritative gameplay carry stable agent/world/link identities and version information. Host migration, replay, save restoration, and streamed-world promotion must not depend on process-local NAV pointers.

### Staged first-party crowd-routing toolkit `NAV-013` — *Normative* (heading line 17619)

- `:17625` Delivery is staged: robust local avoidance and flow/shared-destination tools can arrive before sophisticated congestion/lane/group optimization. Crowd systems must degrade by representation tier rather than requiring one expensive per-agent solver for thousands of agents.

### NAV path-causality and explanation suite `NAV-014` — *Normative* (heading line 17627)

- `:17629` NAV must be able to explain reachability, route choice, route invalidation, high-cost corridors, stuck agents, failed capabilities, congestion, avoidance decisions, and rebuild/publication state. A green-navmesh overlay alone is not sufficient for an automatic multi-representation system.

### Ease-of-use condition inherited from PRODUCT-002 — *Normative* (heading line 17647)

- `:17663` Advanced developers can inspect and override goals, utility scores, action schemas, planner bounds, fact lifetimes, sensors, group hierarchy, scheduling, and debugging traces. Easy defaults and advanced depth must coexist.

### Explicit determinism profiles; Stable is the default `BEAR-007` — *Normative* (heading line 17867)

- `:17878` The exact formal guarantees of each profile still require implementation research and must align with Cairn, gameplay simulation, networking, save/replay, and task scheduling.

### Profile-first NPC authoring that remains directly editable `BEAR-008` — *Normative* (heading line 17910)

- `:17914` This convenience has a hard editability requirement: a developer must be able to understand and edit the resulting behavior through the editor **without AI and without reading documentation for ordinary changes**. Relevant goals, senses, facts, action settings, and planner/tier parameters appear through discoverable inspectors/workspaces. Advanced graph/planner internals may be progressively disclosed, but the profile may not become an opaque blob.
- `:17916` Profile-managed values, project overrides, and fully custom/detached behavior state must be distinguished so Meridian never silently overwrites deliberate edits. Common edits such as changing patrol behavior, sight/hearing range, priorities, allowed actions, reactions, or group role should be obvious from the normal editor surface.

### One typed asynchronous semantic action lifecycle `BEAR-010` — *Normative* (heading line 17926)

- `:17932` The contract must support long-running actions, cancellation on goal changes, temporary blocking, replanning, save/network classification, diagnostics, and deterministic/stable replay where required.

### Automatic cognition tiers for scale `BEAR-011` — *Normative* (heading line 17934)

- `:17940` Promotion/demotion must avoid obvious discontinuities and must cooperate with Artus motion LOD, NAV/world tiers, networking, save state, and the performance governor. Developer overrides exist, but manual per-distance tuning is not the normal workflow.

### Typed semantic context/fact ingress from the rest of Meridian `BEAR-014` — *Normative* (heading line 17958)

- `:17968` The ease gate remains binding: a normal NPC workflow must remain discoverable and editable without AI or documentation, even as advanced systems grow underneath it.

### Modding, development, and accessibility compatibility `PROTECT-011` — *Normative* (heading line 18171)

- `:18182` Accessibility software and assistive devices are not presumed hostile. Where a competitive rule must restrict a class of modification, the project must document the reason, provide the least restrictive practical path, and test real assistive workflows. Protection failures caused by Meridian bugs require recovery and support evidence, not blame shifted to the user.

### Diagnostics and explainability `PROTECT-014` — *Normative* (heading line 18208)

- `:18210` Developer diagnostics must answer:

### Task-first normal authoring with graph/source underneath `ALLU-001` — *Normative* (heading line 18290)

- `:18324` Every task surface must expose an obvious **Open Recipe / Open Graph / Inspect Generation** route. Advanced users can inspect:

### Bake by default; bounded runtime-safe evaluation is explicit opt-in `ALLU-003` — *Normative* (heading line 18389)

- `:18405` Projects may explicitly enable **runtime-safe recipes** for procedural games. A runtime recipe must declare and validate:
- `:18420` If a product does not use runtime Alluvium, shipping must exclude its runtime evaluator, graph compiler, preview caches, runtime services/listeners, unnecessary node libraries and package data except narrow shared foundations already needed for cooked artifacts. This must be proven through dependency, binary-size, task/thread, memory and package audits.

### Semantic stable generated identity with explicit ephemeral modes `ALLU-004` — *Normative* (heading line 18422)

- `:18433` The exact hashing/derivation algorithm remains open, but the contract must support persistence, overrides, saves, VCS diffs, network references and downstream artifact attribution.
- `:18437` Not every pebble or transient particle-like placement needs durable identity. Recipes can explicitly declare outputs/branches as **ephemeral**, allowing cheaper identity/rebuild behavior where persistence/curation is irrelevant. Ephemeral identity must not be used for objects later referenced by saves, gameplay, manual overrides or network authority without an explicit promotion/identity transition.

### World-first 1.0 product scope; broader general authoring experimental later `ALLU-006` — *Normative* (heading line 18470)

- `:18500` Experimental expansion must prove a coherent task workflow, ownership boundary, source/recovery story, licensing/provenance behavior, performance and actual demand before graduating to a supported first-party product surface.

### Adaptive, cancellable progressive live preview `ALLU-007` — *Normative* (heading line 18504)

- `:18529` A missing node library, worker crash, device loss, invalid parameter or failed output must not erase the previous valid preview or authored overrides. The recipe remains inspectable; stale products are clearly marked; recovery/retry uses the normal dependency graph.
- `:18533` Preview scheduling participates in the engine/editor task system and performance governor. It must not freeze MUI, audio preview, basic viewport navigation or editor input while background procedural work runs. GPU evaluation, when used, cannot starve interactive Penumbra rendering.

### Weather Profile / World Environment is the normal authoring surface `ISO-001` — *Normative* (heading line 18597)

- `:18601` The editor may use presets/templates internally, but the resulting weather remains real editable project state. A profile is not an opaque generated blob. Beginner workflow must work without AI or documentation, consistent with `APP-004` and the Unreal/Godot UX evidence already recorded.

### Rich common Field contract with representation portfolio `ISO-003` — *Normative* (heading line 18613)

- `:18615` The public Isobar field semantics do not freeze one storage/simulation topology. The **production baseline** remains sparse regular tiled/hierarchical data because it is straightforward to stream, budget, batch and accelerate, but Isobar must remain rich and highly customizable. Behind the same typed field/query contracts, qualified implementations may include:

### Low/zero-configuration procedural weather is first-class `ISO-006` — *Normative* (heading line 18629)

- `:18631` Isobar must support games that do not want to hand-author weather. A project can choose something equivalent to:

### One logical atmosphere; finest active representation is local truth `ISO-007` — *Normative* (heading line 18671)

- `:18673` Isobar owns **one logical atmospheric/weather state** whose runtime representation changes automatically with relevance, target capability, numerical need, gameplay significance, and quality budget. The public/product model must not expose the historical planning shorthand “B weather” and “C weather” as separate systems.

### Criticality-aware atmospheric persistence `ISO-010` — *Normative* (heading line 18760)

- `:18775` For **Deterministic** projects, each selected Isobar algorithm must document whether equivalent resolved state can be regenerated from durable state/history or whether resolved checkpoints are required. Strict Determinism certification fails if the active authoritative weather portfolio cannot satisfy the declared envelope.

### Production Torsant belongs in Meridian 1.0 `TOR-001` — *Normative* (heading line 18893)

- `:18895` Torsant is optional **per project**, but Meridian 1.0 must qualify a useful production-grade first-party simulation portfolio rather than shipping only contracts for later plugins. The production floor targets at least coherent fire/smoke, thermal state and heat propagation, broad surface/shallow water, localized detailed liquid interaction, steam/boiling hooks, wetness/suppression, and automatic integration with the rest of the environment stack.
- `:18899` Acceptance must be based on representative visible/gameplay evidence, not solver demos in isolation. A feature is not production-qualified merely because a fluid box runs. It must survive authoring, streaming, save/load, networking where applicable, downgrade, recovery, web/native profile handling, editor iteration and cross-subsystem use.

### Rich game combustion by default; deep combustion experimental `TOR-004` — *Normative* (heading line 18936)

- `:18952` A separate first-party **Experimental Deep Combustion** toggle/capability *(working label; exact public name open)* may pursue substantially richer oxygen availability, combustion products, chemistry-like species, buoyant reactive flow, radiative/convective heat transfer, fuel mixtures and related research. It is deliberately allowed—not rejected—but it must stay out of the normal authoring complexity and create zero baggage when disabled. Promotion into the production/default path requires measured visual/gameplay value, stability, performance, maintainability and clear semantics.

### First-class practical phase/material transitions; generalized matter experimental `TOR-006` — *Normative* (heading line 18968)

- `:18983` A separate **Experimental Matter/Chemistry** capability *(working label; exact UI/name open)* may explore a much more generalized material-transformation/chemistry system. This possibility is intentionally retained. It is not a default 1.0 complexity burden and must be zero-cost/zero-baggage when disabled. Its eventual product status is evidence-gated.

### Curated task workspaces with optional freeform customization `EDUX-001` — *Normative* (heading line 19052)

- `:19068` Qualification must include a clean-profile novice path: open Meridian for the first time and perform the core game-creation loop without first rearranging the shell.

### Universal ultra-fast launcher / command-discovery surface `EDUX-003` — *Normative* (heading line 19092)

- `:19110` The query syntax may support light scope prefixes/filters, but discoverability must not depend on memorizing punctuation. Contextual ranking may consider active workspace, selection, recent use, exact prefix/word boundaries and user history without making globally valid commands disappear mysteriously.

### Ponder-backed contextual learning; self-explanatory UI remains the first line `EDUX-011` — *Normative* (heading line 19370)

- `:19372` Meridian must be learnable without AI and without requiring external documentation for ordinary workflows.

### Broad device architecture, selective qualification `INP-005` — *Normative* (heading line 20362)

- `:20368` `PRODUCT-008` applies directly: unused specialist-device UI/providers must remain invisible/stripped from ordinary projects.

### Cross-system consequences — *Normative* (heading line 20370)

- `:20377` - The clean-room Celeste-like C# proving project `TWO-015` must validate keyboard and controller rebinding, prompt switching, low-latency action delivery and preferences persistence from C# rather than relying on Rust-only privileged paths.

### One canonical timestamped input stream with purpose-specific immutable views `INP-009` — *Normative* (heading line 20472)

- `:20503` Meridian must expose at minimum:

### Small semantic haptic/feedback layer plus advanced capability facets `INP-010` — *Normative* (heading line 20516)

- `:20533` - one simple feedback event must degrade to ordinary rumble or no output without gameplay breakage;

### Tiny project root, flexible contents `PRJ-001` — *Normative* (heading line 21275)

- `:21277` A normal Meridian game project has a deliberately small set of first-party root concepts. The engine's subsystem topology must never become the game's filesystem topology. The default conceptual shape remains:

### One derived-state boundary `PRJ-003` — *Normative* (heading line 21293)

- `:21297` `.meridian/` must have explicit retention and authority classes. A future clean operation must distinguish safely reconstructable data from recovery data, expensive-but-reconstructable data, user-local configuration, and anything that requires confirmation before removal.

### The game is one codebase; language is an implementation property `PRJ-005` — *Normative* (heading line 21314)

- `:21333` Meridian must never require this merely because multiple languages are selected:

### Native-tooling escape hatch and R2 fallback are mandatory `PRJ-007` — *Normative* (heading line 21399)

- `:21403` The qualification prototype must cover at least:

### Project root anti-complexity contract `PRJ-008` — *Normative* (heading line 21451)

- `:21453` A normal game checkout must answer four questions at a glance:
- `:21462` The game root must not mirror engine subsystem names. The following are specifically forbidden as mandatory normal-game organization concepts:

### Small platform-native built product `PKG-001` — *Normative* (heading line 21597)

- `:21620` 1. **No engine-subsystem folder topology.** Shipping output must not contain `Penumbra/`, `Cairn/`, `Isobar/`, `Torsant/`, `Artus/`, `Wavefront/`, or similar folders merely because those implementation systems contributed to the build.

### C# deployment is player-runtime-free by default `PKG-002` — *Normative* (heading line 21648)

- `:21650` Using C# in a Meridian game must **not normally require the player to preinstall .NET**.

### Steam Auto-Cloud first-class zero-code save-sync path `STEAM-001` — *Normative* (heading line 21812)

- `:21814` Steam builds must support a configuration mode in which:
- `:21824` 9. Conflict handling respects the persistence-scope semantics from SAVE-005/SAVE-006; Meridian must not infer that provider synchronization makes arbitrary world-state merging safe.

### One-session/process default with explicit high-density mode `SRV-006` — *Normative* (heading line 24031)

- `:24055` High-density qualification must account for:
- `:24065` Meridian must never silently increase `max_sessions_per_process` merely to reduce hosting cost. Changing the isolation/blast-radius model is a consequential production decision and remains inspectable.

### Latency/fairness-first global session placement `SRV-009` — *Normative* (heading line 24112)

- `:24127` Different game genres may ship different semantic placement profiles. Meridian must not force a twitch-shooter latency envelope onto asynchronous strategy games, but the ordinary real-time profile protects the group rather than optimizing a misleading mean.

### Incremental multi-root Multiplayer Product Set `NETPROJ-005` — *Normative* (heading line 25284)

- `:25296` Developer-experience qualification must answer:

### Compact brace/block `.mui` syntax `UI-SRC-001` — *Normative* (heading line 26258)

- `:26260` Canonical hand-written `.mui` source uses explicit brace-delimited blocks with concise property/binding syntax. The exact token spellings may still receive parser/formatter refinement when implementation evidence requires it, but the language must preserve the following surface character:

### Later Developer Ergonomics & Language Experience requirements added from this audit — *Normative* (heading line 26582)

- `:26584` When `DEVLANG-QUAL-001` activates, the source/IDE portion must explicitly compare representative workflows against current Zed and at least the other relevant current IDE/editor baselines selected during that future research pass.

### Meridian 1.0 first-class language floor `DEVLANG-001` — *Normative* (heading line 26618)

- `:26620` Rust, Luau, and C# are guaranteed first-class Meridian 1.0 release blockers. TypeScript and Meridian Python remain high-priority and may also ship as first-class in 1.0 when they pass the same qualification contract, but they do not block 1.0. C/C++ must have the stable native extension/ABI path required by existing extension contracts; full gameplay-language parity is not a 1.0 release blocker.

### Built-in Code workspace is a professional primary IDE `DEVLANG-002` — *Normative* (heading line 26622)

- `:26624` A developer must be able to build and maintain a serious Meridian project entirely inside Meridian's Code workspace without requiring VS Code, Rider, Visual Studio, Zed, or another external IDE. External editors remain fully supported. Meridian's IDE owns the engine-aware overlay and workflow integration while reusing mature language tooling/protocols where appropriate rather than reimplementing each language frontend.

### Remote development is architecture-now, polish-later `DEVLANG-003` — *Normative* (heading line 26626)

- `:26628` Meridian's source/build/LSP/debug/test/import/service boundaries must remain compatible with future first-party remote development and may be prototyped before 1.0, but polished SSH/remote-project workflows are not a Meridian 1.0 release blocker.

### Honest divergence and migration `MPY-PHIL-004` — *Normative* (heading line 26733)

- `:26735` When `.mpy` differs from Python, the difference must be:

### Intentional divergence must be explicit and versioned `MPY-004` — *Normative* (heading line 26815)

- `:26819` Intentional divergences must be documented, testable, source-mapped, versioned, and migration-aware. Differential Python tests remain useful as comparison evidence, but a deliberate Meridian divergence is not a defect merely because CPython behaves differently.

### Zero duplicated truth; abstract only real shared semantics `AGENT-SEM-004` — *Normative* (heading line 26861)

- `:26863` Meridian treats duplicated authority as a defect. A schema, invariant, capability meaning, protocol semantic, authoritative mapping or algorithm whose results must remain synchronized has one authoritative home and consumers query/use that authority rather than maintaining mirrored copies.

### Honest regression-first evidence `AGENT-SEM-005` — *Normative* (heading line 26869)

- `:26882` When the original failure genuinely depends on unavailable hardware, driver behavior, restricted platform access, nondeterministic external infrastructure or a timing condition that cannot be faithfully captured, the agent must identify the violated invariant and add the strongest reliable deterministic test/fault-injection defense available. The final evidence explicitly states what could and could not be reproduced. A test that did not exercise the bug or violated invariant may not be presented as regression evidence merely to satisfy process wording.

### Meridian repository owns technical documentation authority `WEBDOC-001` — *Normative* (heading line 27343)

- `:27347` The website must not become a second manually synchronized technical specification database.
- `:27368` The existing architecture/specification/ADR/benchmark/evidence documents keep their own roles. User-facing manuals/reference/tutorials must not be conflated with normative design authority or benchmark evidence merely because all are documentation-like text.

### Generated facts; authored explanation `WEBDOC-003` — *Normative* (heading line 27399)

- `:27401` Facts already owned by authoritative machine-readable Meridian sources must be generated or imported into documentation rather than copied by hand wherever practical. Examples include:

### Meridian has real releases; web docs are release products `WEBDOC-004` — *Normative* (heading line 27417)

- `:27433` The public documentation system must make that product reality obvious.

### Status/maturity is explicit `WEBDOC-008` — *Normative* (heading line 27541)

- `:27543` Feature and page state must visibly distinguish at least the concepts needed to prevent planned architecture from being mistaken for shipped functionality, such as:

### Governing quality rule `QUALITY-001` — *Normative* (heading line 27594)

- `:27598` A capability advertised as Stable/first-class/production-ready must satisfy its declared correctness, integration, recovery, accessibility, security, performance, portability, documentation, compatibility and maintenance evidence.

### Quality is not unlimited feature accumulation `QUALITY-002` — *Normative* (heading line 27608)

- `:27613` - A generated blocker report must show every unsatisfied gate, its owner, evidence state, dependency path, approved waiver if any, and user consequence.

### Release maturity and claim states `QUALITY-003` — *Normative* (heading line 27618)

- `:27636` - `Experimental` may be incomplete, risky or backend-limited and must be isolated from ordinary projects.

### End-to-end qualification portfolio `QUALITY-004` `VAL-PORTFOLIO-001` — *Normative* (heading line 27642)

- `:27646` The pre-1.0 portfolio must include, at minimum, evidence-shaped projects/journeys for:
- `:27659` Each first-class platform/language profile must be represented by real evidence. The portfolio can share projects and fixtures where one project genuinely proves several contracts; it is not permission to fake coverage with one giant bespoke demo.

### Consumer-project independence `QUALITY-005` — *Normative* (heading line 27661)

- `:27670` - A public generic proving project must exist wherever private evidence would otherwise make an engine claim impossible for contributors/users to reproduce.

### Normative contradiction and migration map `NORM-MIG-001..012` (heading line 27810)

- `:27812` The current v0.5 suite remains authoritative until replaced, but the following later locked decisions must be reconciled explicitly rather than layered on top:
- `:27827` The rewrite must update every affected authority together. It may not leave v0.5 and replacement-era promises simultaneously normative.

### Internal adaptive execution remains unnamed `EXEC-INT-001` — *Normative* (heading line 27840)

- `:27844` Normal developers see language semantics, diagnostics and performance outcomes. Advanced tools may expose descriptive facts needed to debug behavior, but there is no branded subsystem they must learn or configure for ordinary work.