# DEP-UI-008: Lucide SVG subset review

- Owner: `WP-UI-005` corrective framework package
- Upstream: <https://github.com/lucide-icons/lucide>
- Release and revision: `1.25.0`, commit `5136572c10214634858fcf5f726b2a9d26683918`
- Retrieved archive: `lucide-icons-1.25.0.zip`, SHA-256 `070ca0b59b5b9c6587f9d09a033d8085596938784b71cfb3da9837c02b2b3a71`
- Selected sources: `play`, `square`, `hammer`, `search`, `settings`, `ellipsis`, `x`, `chevron-down`, `chevron-right`, `triangle-alert`, `circle-x`, and `circle-check`; exact per-file SHA-256 values are enforced by `scripts/import_ui_assets.py`.
- SPDX/notice record: `ISC` with retained Feather MIT notice; retained text: `third_party/licenses/lucide-ISC-MIT.txt`
- Source provenance registry: `SRC-UI-008` in `specs/registry/source-provenance.json`
- Classification: immutable reviewed SVG source subset for a Meridian-owned generated geometry adapter; no runtime SVG parser
- Meridian destination: `engine/meridian_ui_render/assets/icons/`
- Modifications: source files unchanged; `scripts/generate_ui_icons.py` lowers the reviewed subset into bounded `UiPathCommand` data in `engine/meridian_ui_render/src/generated_icons.rs`. The generated file records its generator/schema/version and is reproducibly checked.
- Boundary: only `IconId` and Meridian display primitives cross public APIs. SVG filenames, XML, and upstream types stay private.
- Validation: allowlist, archive and per-file hashes, license files, generated-file freshness, bounded normalized geometry for every `IconId`, accessible-name retention, display-list validation, 1x/2x raster output, high contrast, and public-boundary audit
- Update/exit: update only through a reviewed `DEP-UI-008` amendment and regenerated icon corpus. Custom domain icons remain Meridian-owned and separately reviewed.
