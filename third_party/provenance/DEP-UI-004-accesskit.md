# DEP-UI-004: AccessKit dependency review

- Owner: `WP-UI-005`
- Upstream: <https://github.com/AccessKit/accesskit>
- Packages: `accesskit 0.24.1`, `accesskit_winit 0.33.2`
- Registry checksums:
  - `accesskit`: `d3b7f7f85a7e5f68090000ed7622545829afd484d210358702ae4cb97dd0c320`
  - `accesskit_winit`: `d5b41e63a69f36d9f1f41e70464c7e5f72eee485ef26aab19f0b4f86e6c0a84c`
- SPDX licenses: `accesskit` is `MIT OR Apache-2.0`; `accesskit_winit`
  is `Apache-2.0`.
- Classification: private platform adapter dependencies; no upstream source is
  copied into Meridian.
- Meridian destination: the optional `accessibility` adapter in
  `meridian-platform`, enabled only by the visible editor application.
- Boundary: Meridian owns semantic roles, trees, stable node identity, focus,
  actions, validation, diagnostics, and platform events. AccessKit and winit
  accessibility types remain private adapter details.
- Features: `accesskit_winit` defaults are disabled. Meridian selects the Unix
  adapter, its `async-io` executor, and raw-window-handle 0.6. Existing winit
  target features remain owned by `meridian-platform`.
- Lifecycle: the adapter is created while the winit window is still hidden,
  before the first visible presentation. Every update is projected from a
  validated immutable semantic tree. Untrusted platform actions are mapped to
  authorized Meridian actions or a non-fatal typed rejection.
- Validation: stable identity and focus projection, reading-order children,
  action allowlisting and payload limits, native adapter construction, editor
  action routing, warning-denied Clippy, and public-boundary audits. Real
  VoiceOver, NVDA, and AT-SPI review remains platform evidence and cannot be
  inferred from structural tests.
- Update and exit: preserve the Meridian semantic/action fixtures across an
  update. Replace AccessKit only through a recorded dependency decision when a
  measured platform gap, maintenance failure, or license issue outweighs the
  cross-platform adapter value.
