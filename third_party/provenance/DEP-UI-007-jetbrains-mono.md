# DEP-UI-007: JetBrains Mono asset review

- Owner: `WP-UI-005` corrective framework package
- Upstream: <https://github.com/JetBrains/JetBrainsMono>
- Release and revision: `v2.304`, commit `cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9`
- Retrieved archive: `JetBrainsMono-2.304.zip`, SHA-256 `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf`
- Selected source: `fonts/variable/JetBrainsMono[wght].ttf`, SHA-256 `662a196d58f1183bf2d77428b6d5283fe3f45161ab021bea4036bc98e5cac016`
- SPDX license: `OFL-1.1`; retained text: `third_party/licenses/jetbrains-mono-OFL-1.1.txt`
- Source provenance registry: `SRC-UI-007` in the retired v0.5 source-provenance registry
- Classification: immutable redistributed font asset; no upstream code copied
- Meridian destination: `engine/meridian_ui_text/assets/fonts/JetBrainsMonoVF.ttf`
- Modifications: file renamed only; font bytes unchanged
- Boundary: `UiFontRole::Monospace` is public. Font data, shaping objects, and platform font handles stay private.
- Validation: pinned importer SHA-256 verification, exact-family technical-text shaping, tabular metrics, scale raster, fallback diagnostics, public-boundary audit
- Update/exit: change only through a reviewed `DEP-UI-007` amendment and code/data typography corpus; replacement must preserve monospace editing and technical-table behavior.
- **Control status: unmet.** The v0.5 source-provenance registry was retired at `PH-AUTH-004` and no v1 registry exists. `LEGAL-005` requires machine-readable provenance for every third-party dependency; the repository has none for its 494 locked packages. Recorded as `OD-006`, pre-existing and unmet — this is a broken control, not a repaired link.
