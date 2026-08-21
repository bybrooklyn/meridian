# PH-AUTH-001 baseline manifest — frozen v0.5 authority

generated: 2026-08-20T19:57:52Z
work_package: WP-V1-RESET-001
frozen_sha: ddbacd34361c302e72ed2accefd59fe7567b28fe
tag: v0.5-final-baseline
reset_branch: v1-authority-reset (same SHA, no commits yet)
tree_hash: 0536e6e8f3ecffd7e72bca37176c8d329f279656
commit_count: 75
prior_public_head: e0eb184b48e7b61d0b63f18352e95e9b06c7c4e0
toolchain: [toolchain] channel = "stable" components = ["rustfmt", "clippy"] profile = "minimal" 
cargo_lock_sha256: 32156cbbcf75f744a23c9f05d442fa40490fb03a4ca8166b2ffd1d6ec124e751
specoment_sha256: 782d3110b89ac23fa3f8cf80c07a72ba15e9de457717ca918a14f24e6d32692a
specoment_lines: 33175
specoment_original_sha256: d7329e2366868feeb3705bbdae29cff727a1e0038339b7dabb91be439483b72b
appendix_a_sha256: 880d0a96217d0d826227ca7db4fb0fec1d585fe635d6f043ff044242290234ea
authority_preimage_tag: v1-authority-preimage -> 0882050a57d8232bb3d23157c7e2796434bd57e2
preserved_evidence_tag: ev-ui-20260806-001 -> 5d19cae2bd006a14423fa1e6e04ff7f49fce37e2
out_of_process_commit_preserved: origin/error -> c53af25e303900adc28fecce0de9fdacee69b2ad

## gate evidence at frozen_sha

| Gate | Result |
|---|---|
| meridian-spec check | Pass |
| cargo metadata --locked | Pass |
| cargo fmt --all -- --check | Pass |
| cargo test --workspace | Pass — 79 suites, 674 passed, 0 failed, 0 ignored |
| cargo clippy --workspace --all-targets -D warnings | Pass — 0 warnings |
| git diff --check | Pass |
| meridian-rhi example clear_frame | Structural Pass; surface Occluded; GPU timing UnsupportedPlatform |
| meridian-renderer example instance_upload_smoke | Structural Pass; offscreen capture 64x64; surface Occluded; GPU timing UnsupportedPlatform |
| CI reproduction from frozen SHA | NotRun — nothing pushed |

## baseline repair included in this freeze

- 4cc7d55 — WP-V1-BASE-001, retired waivers no longer fail governance
- ddbacd3 — WP-V1-BASE-002, v0.5 governance no longer scans staged v1 authority

## workflows
- .github/workflows/ci.yml
- .github/workflows/discord-ci.yml

## crates
- Cargo.toml
- editor/meridian_asset_tools/Cargo.toml
- editor/meridian_benchmark/Cargo.toml
- editor/meridian_build/Cargo.toml
- editor/meridian_editor/Cargo.toml
- editor/meridian_editor_core/Cargo.toml
- editor/meridian_shader_tools/Cargo.toml
- editor/meridian_spec_tools/Cargo.toml
- editor/meridian_ui_editor/Cargo.toml
- editor/meridian_world_tools/Cargo.toml
- engine/meridian_alluvium/Cargo.toml
- engine/meridian_assets/Cargo.toml
- engine/meridian_audio/Cargo.toml
- engine/meridian_basalt/Cargo.toml
- engine/meridian_core/Cargo.toml
- engine/meridian_diagnostics/Cargo.toml
- engine/meridian_ecs/Cargo.toml
- engine/meridian_input/Cargo.toml
- engine/meridian_isobar/Cargo.toml
- engine/meridian_modeler/Cargo.toml
- engine/meridian_package/Cargo.toml
- engine/meridian_physics/Cargo.toml
- engine/meridian_platform/Cargo.toml
- engine/meridian_render_graph/Cargo.toml
- engine/meridian_renderer/Cargo.toml
- engine/meridian_rhi/Cargo.toml
- engine/meridian_rt/Cargo.toml
- engine/meridian_save/Cargo.toml
- engine/meridian_streaming/Cargo.toml
- engine/meridian_tasks/Cargo.toml
- engine/meridian_ui/Cargo.toml
- engine/meridian_ui_core/Cargo.toml
- engine/meridian_ui_render/Cargo.toml
- engine/meridian_ui_runtime/Cargo.toml
- engine/meridian_ui_semantics/Cargo.toml
- engine/meridian_ui_text/Cargo.toml
- engine/meridian_vegetation/Cargo.toml
- engine/meridian_world/Cargo.toml

## Correction, 2026-08-20 — stale specoment digest

This manifest originally recorded `specoment_sha256: 475c91c8a99f…` and
`specoment_lines: 33168`. Both were stale. They described a specoment revision predating the
`AI-027..030` restoration and the §0.3 revert, and that revision **exists nowhere on disk**;
no file in the repository hashes to `475c91c8…`. The manifest is `PH-AUTH-001` closure
evidence, so it was recording a digest for a document that could not be produced.

Surfaced by independent review of `WP-V1-RESET-002`, round 3, while checking that plan's
claim that the excluded authority files were "retained by content hash" in this manifest.
They were not: this manifest recorded neither digest.

Corrected above to the measured values, with the two previously unrecorded digests added.

## Correction, 2026-08-20 — authority preimage was not in Git at all

The same review established three further facts, each verified directly:

- `MERIDIAN_SPECOMENT.md` has **never been committed**; `git log --all -- MERIDIAN_SPECOMENT.md`
  is empty. The only specoment inside tag `v0.5-final-baseline` is a four-line test fixture at
  `editor/meridian_spec_tools/tests/fixtures/v1_staging/MERIDIAN_SPECOMENT.md`.
- `.meridian/authority/MERIDIAN_SPECOMENT.original.md` was **absent from the object database**.
  Its only copy was the untracked working tree on one machine.
- `state.json` lists 14 `unreviewed_specoment_amendments` whose only audit method is diffing
  that original against the canonical file, and `DEV-003` names that diff as the residual
  exposure to be closed at `PH-AUTH-004`.

A single `git clean -xdf` — routine during a `cargo clean`-heavy session, which repository
policy actively encourages — would therefore have destroyed the sole basis for auditing those
amendments.

Remediated by writing the bytes into the object database and making them reachable:

```text
tag    v1-authority-preimage -> 0882050a57d8232bb3d23157c7e2796434bd57e2  (orphan, no parent)
blob   d5a4edff89f36ce34343905d71a4071bde625818  MERIDIAN_SPECOMENT.original.md
blob   1d8412b772aadb41a1034ff67efba86cb367ca97  appendix_a.md
blob   3bf7cec416ddb11b651f38484c0f5d25dfafb0dd  MERIDIAN_SPECOMENT.canonical.md
```

The commit is orphaned and on no branch, so the bytes are preserved and gc-safe without any
tracked working-tree path being named as authority — which is what the `PH-AUTH-002` exclusion
requires. Verified: `git fsck --unreachable` does not list them.
