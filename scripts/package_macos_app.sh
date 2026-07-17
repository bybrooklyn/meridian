#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS app packaging must run on macOS" >&2
  exit 1
fi

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${workspace_root}"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
  if ! rustup target list --installed | grep -Fxq "${target}"; then
    echo "missing Rust target: ${target}; run rustup target add ${target}" >&2
    exit 1
  fi
done

cargo build --quiet --release --target aarch64-apple-darwin -p meridian-editor --bin meridian
cargo build --quiet --release --target x86_64-apple-darwin -p meridian-editor --bin meridian

bundle="${workspace_root}/target/Meridian.app"
if [[ -e "${bundle}" ]]; then
  rm -rf "${bundle}"
fi
mkdir -p "${bundle}/Contents/MacOS" "${bundle}/Contents/Resources"

lipo -create \
  "${workspace_root}/target/aarch64-apple-darwin/release/meridian" \
  "${workspace_root}/target/x86_64-apple-darwin/release/meridian" \
  -output "${bundle}/Contents/MacOS/meridian"

cp "${workspace_root}/scripts/macos/Info.plist" "${bundle}/Contents/Info.plist"
cp "${workspace_root}/scripts/macos/DISTRIBUTION.txt" "${bundle}/Contents/Resources/DISTRIBUTION.txt"

plutil -lint "${bundle}/Contents/Info.plist"
lipo -archs "${bundle}/Contents/MacOS/meridian"
signature_details="$(codesign -dv --verbose=2 "${bundle}" 2>&1)"
if ! grep -Fqx 'TeamIdentifier=not set' <<<"${signature_details}"; then
  echo "the preview bundle unexpectedly has a Developer ID team identity" >&2
  exit 1
fi
shasum -a 256 "${bundle}/Contents/MacOS/meridian" > "${workspace_root}/target/Meridian.app.sha256"
echo "created ${bundle} and target/Meridian.app.sha256"
