# DEP-UI-003: rfd dependency review

- Owner: `WP-EDT-001`
- Upstream: <https://github.com/PolyMeilex/rfd>
- Version: `0.17.2`
- Registry checksum: `20dafead71c16a34e1ff357ddefc8afc11e7d51d6d2b9fbd07eaa48e3e540220`
- SPDX license: `MIT`
- Classification: application dependency; no upstream source is copied into Meridian.
- Meridian destination: the private native-directory-picker adapter in
  `meridian-editor` only.
- Boundary: rfd types never cross into Meridian UI, editor-core, source
  documents, commands, runtime crates, or public APIs.
- Features: default features disabled; the `xdg-portal` backend is selected for
  portable Linux builds. The dependency remains target-adapted by rfd for
  macOS and Windows.
- Validation: cancellation, invalid-project remediation, Creator hub creation
  and open paths, and public-boundary checks. Reevaluate only through
  `DEP-UI-003`; replacement requires a demonstrated platform capability or
  maintenance reason.
