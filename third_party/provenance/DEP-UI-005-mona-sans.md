# DEP-UI-005: Mona Sans asset review

- Owner: `WP-UI-005` corrective framework package
- Upstream: <https://github.com/github/mona-sans>
- Release and revision: `v2.0.27`, commit `0f7dc66ddd766605eb0e75c3f47bf9d1dd38ceca`
- Retrieved archive: `mona-sans-variable-v2.0.27.zip`, SHA-256 `a95127550b2957ff84cd636d4532b227ddc33d3485082437fa27816ef1d066ec`
- Selected source: `fonts/variable/MonaSansVF[opsz,wght].ttf`, SHA-256 `84aae10d4427a1947e96b1fd9b26c3109ffa0f50f2faae8ce460ca1e34889ed5`
- SPDX license: `OFL-1.1`; retained text: `third_party/licenses/mona-sans-OFL-1.1.txt`
- Source provenance registry: `SRC-UI-005` in `specs/registry/source-provenance.json`
- Classification: immutable redistributed font asset; no upstream code copied
- Meridian destination: `engine/meridian_ui_text/assets/fonts/MonaSansVF.ttf`
- Modifications: file renamed only; font bytes unchanged
- Boundary: `UiFontRole::Interface` is public. Font data and Cosmic Text/fontdb handles stay private.
- Validation: pinned importer SHA-256 verification, exact-family shaping, 1x/2x raster, fallback diagnostics, public-boundary audit
- Update/exit: change only through a reviewed `DEP-UI-005` amendment and golden typography corpus; a replacement must preserve interface metrics and source redistribution rights.
