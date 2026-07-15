# Meridian

Meridian is a Rust engine for games and interactive applications. Project Meridian, maintained in the separate private `bybrooklyn/project-meridian` repository, is its first proving game.

The repository is in early engine development. Implemented foundations include fixed-step runtime timing, task/diagnostic primitives, native window and wgpu paths, render-graph validation, PBR materials, cascaded shadows, diffuse irradiance image-based lighting, render extraction/upload, deterministic asset/world/streaming foundations, save/recovery foundations, and a transitional Rapier-backed physics wrapper.

This is not a claim that the engine, editor, renderer, Cairn physics, or game is complete. The current renderer smoke proves structural GPU construction and may use an occluded surface; pass-level timing, visible-pixel captures, a clustered Forward+ forest viewport, and calibrated B01/B02 workloads remain open.

## Start here

- [Active implementation and evidence](PLANNING.md)
- [Meridian v0.2 master specification](specs/MERIDIAN_MASTER_SPEC.md)
- [Phase 0–29 dependency DAG](specs/IMPLEMENTATION_PHASES.md)
- [Project Meridian opening-forest integration plan](specs/PROJECT_MERIDIAN_VERTICAL_SLICE_PLAN.md)
- [Testing and benchmark contract](specs/TESTING_BENCHMARKS_AND_VALIDATION.md)
- [Migration and contradiction register](specs/SPEC_MIGRATION_AND_CONTRADICTIONS.md)
- [Repository agent policy](AGENTS.md)
- [Private Project Meridian creative repository](https://github.com/bybrooklyn/project-meridian)

The full Project Meridian creative suite is closed-source and versioned separately. This repository retains only the engine-facing proving-slice requirements and benchmark workloads needed to develop Meridian. Historical version 0.1 engine architecture has been consolidated into the v0.2 suite; legacy numeric budgets remain provisional until calibrated.

## Workspace

- engine/ — reusable runtime and subsystem crates
- editor/ — editor and authoring products/tools
- specs/ — normative version 0.2 architecture and delivery contracts
- docs/ — ADRs and developer/production records
- schemas/ — machine-readable shared contracts
- shaders/ — shader source and validation notes
- assets_source/ and assets_built/ — editable source and derived artifacts
- third_party/ and licenses/ — source provenance, modifications, and notices

Local checkouts may contain an ignored `game/` directory holding the separate Project Meridian repository. It is not a Cargo workspace member and is never part of the Meridian engine repository.

## License

Meridian is dual-licensed under either the [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE), at your option.

## Current validation

~~~sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p meridian-rhi --example clear_frame
cargo run -p meridian-renderer --example instance_upload_smoke
git diff --check
~~~

Platform/GPU availability can make native examples report an unsupported or occluded outcome. Record that outcome instead of treating it as visible-quality evidence.

## AI notice

AI was used extensively in the development of The Meridian Engine and The Project Meridian game.
