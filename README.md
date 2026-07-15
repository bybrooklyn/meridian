# Meridian

Meridian is an experimental general-purpose game and interactive-application engine written in Rust and one integrated creator application named **Meridian**. Penumbra is its renderer, Wavefront its audio system, Collective its optional online-services system, and The Alluvium Engine its procedural world-authoring architecture. These adopted names and specifications are much broader than the current implementation foundations.

## Start here

- [Meridian v0.5 master specification](specs/MERIDIAN_MASTER_SPEC.md)
- [Evidence milestone and workstream roadmap](specs/DELIVERY_ROADMAP.md)
- [Implementation planning and package gates](specs/IMPLEMENTATION_PLANNING_SPEC.md)
- [Current bounded work and implementation truth](PLANNING.md)
- [Penumbra rendering architecture](specs/RENDERING_AND_GRAPHICS_SPEC.md)
- [The Alluvium Engine architecture](specs/PROCEDURAL_AUTHORING_SPEC.md)
- [Native modeling and DCC boundary](specs/NATIVE_MODELING_AND_DCC_SPEC.md)
- [Rust-first gameplay and optional Luau](specs/GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md)
- [First-class 2D](specs/TWO_DIMENSIONAL_ENGINE_SPEC.md)
- [Wavefront audio](specs/AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) and [Collective online services](specs/COLLECTIVE_ONLINE_SERVICES_SPEC.md)
- [Testing, benchmarks, and evidence policy](specs/TESTING_BENCHMARKS_AND_VALIDATION.md)
- [Canonical architecture decisions](docs/architecture/decisions/README.md)

Project Meridian is the first proving game. Its full creative suite and future game code live in the separate private `bybrooklyn/project-meridian` repository. This engine repository contains only sanitized engine-facing prototype/slice requirements and generated benchmark contracts; `game/` is ignored and excluded from the Cargo workspace.

## Workspace

~~~text
engine/       reusable runtime and subsystem crates
editor/       editor products and developer tools
specs/        normative v0.5 architecture and delivery contracts
schemas/      versioned data and governance schemas
docs/         ADRs, benchmarks, migrations, and engineering records
third_party/  provenance policy and future reviewed donor manifests
shaders/      current shader sources and validation notes
~~~

`meridian-rhi` currently uses wgpu behind Meridian-owned contracts. `meridian-renderer` is Penumbra's implementation crate. Alluvium has no implementation crate; `meridian-alluvium` is reserved for its first real package rather than a marker scaffold.

## Current validation boundary

The workspace contains meaningful runtime, RHI, render-graph,
PBR/shadow/diffuse-IBL, typed high-level pass timing, extraction/upload,
asset/world/streaming/save, and physics-wrapper foundations. Passing tests and
structural/native GPU smokes prove those boundaries only. The MS-01 `meridian`
executable now imports a public fixture, packages/streams/activates it, renders
package-derived geometry, writes an explicitly presented-or-offscreen PNG and
correlated evidence, and proves save recovery. Meridian UI, Creator Editor,
clustered Forward+, Alluvium, the native modeler, Rust gameplay modules, Luau,
first-class 2D, animation, navigation, official frameworks, Wavefront runtime,
Collective, Isobar, Basalt, Torsant, the Project Meridian prototype, native
backends, and post-1.0 programs are not implemented.

Any passing test count is evidence for covered behavior, not a percentage-
complete claim.

Run the local gates with:

~~~text
cargo run -p meridian-spec -- check
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p meridian-editor --bin meridian -- --headless-smoke
cargo run -p meridian-editor --bin meridian -- --smoke
cargo run -p meridian-editor --bin meridian -- --ui-headless-smoke
cargo run -p meridian-rt --example headless_profile_smoke
! cargo tree -p meridian-rt | grep -q meridian-ui
~~~

## License

Meridian is dual-licensed under MIT or Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE). Project Meridian content is separate and proprietary.

## AI notice

AI-assisted contributions are permitted, but the contributor remains responsible for correctness, provenance, licensing, private-content boundaries, tests, and review. Generated or donor-derived material is untrusted until recorded and validated under the same policies as human-authored work.
