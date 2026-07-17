# macOS unsigned developer preview

`scripts/package_macos_app.sh` builds a universal `target/Meridian.app` for
Apple Silicon and Intel Macs. It intentionally creates no signing key,
certificate, notarization request, credential, or release claim.

The bundle is downloadable as an unsigned developer preview. Before opening,
verify the published SHA-256 for `Contents/MacOS/meridian` against
`target/Meridian.app.sha256`. Gatekeeper may display an unidentified-developer
warning; the person opening the app must make the explicit standard macOS
override through the Open Anyway / Privacy & Security flow.

The macOS linker may carry an automatic ad-hoc code marker so the universal
Mach-O can launch locally. It has no Developer ID team identity, certificate,
or verified developer trust; the packaging script rejects a bundle with a
Developer ID team identity.

That override does not establish trust, safety, Developer ID signing, or
notarization. A future trusted public distribution requires an externally
provisioned Apple Developer ID Application identity, hardened-runtime review,
notarization, and the separate `WP-SEC-001` release evidence. No private key or
certificate belongs in this repository.
