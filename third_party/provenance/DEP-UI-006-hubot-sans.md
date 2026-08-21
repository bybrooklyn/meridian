# DEP-UI-006: Hubot Sans asset review

- Owner: `WP-UI-005` corrective framework package
- Upstream: <https://github.com/github/hubot-sans>
- Release and revision: `v1.0.1`, commit `05d5ea150c20e6434485db8ffd2277ed18a9e911`
- Retrieved archive: `Hubot-Sans.zip`, SHA-256 `b460d36097a5c9a3e45710cbe1554589eaa5765d7c2c88df364516f3e27159b1`
- Selected source: `Hubot Sans/Hubot-Sans.ttf`, SHA-256 `2cbf834f750ae1201a8d6193b004584bd530bbad7907e02991d4a97da44784ce`
- SPDX license: `OFL-1.1`; retained text: `third_party/licenses/hubot-sans-OFL-1.1.txt`
- Source provenance registry: `SRC-UI-006` in the retired v0.5 source-provenance registry
- Classification: immutable redistributed font asset; no upstream code copied
- Meridian destination: `engine/meridian_ui_text/assets/fonts/HubotSansVF.ttf`
- Modifications: file renamed only; font bytes unchanged
- Boundary: `UiFontRole::Display` is public and restricted to restrained headings. Font data and adapter handles stay private.
- Validation: pinned importer SHA-256 verification, exact-family heading shaping, scale raster, fallback diagnostics, public-boundary audit
- Update/exit: change only through a reviewed `DEP-UI-006` amendment and heading corpus; Mona Sans remains the approved fallback if Hubot cannot be redistributed or qualified.
- **Control status: unmet.** The v0.5 source-provenance registry was retired at `PH-AUTH-004` and no v1 registry exists. `LEGAL-005` requires machine-readable provenance for every third-party dependency; the repository has none for its 494 locked packages. Recorded as `OD-006`, pre-existing and unmet — this is a broken control, not a repaired link.
