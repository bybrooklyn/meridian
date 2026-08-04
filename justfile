# Meridian Workspace Justfile

# Set shell to bash with safe execution settings
set shell := ["bash", "-c"]

# Show list of recipes by default
default:
    @just --list

# ==============================================================================
# 1. Code Quality & Auditing
# ==============================================================================

# Run formatting checks across the workspace
fmt-check:
    cargo fmt --all -- --check

# Format all workspace code
fmt:
    cargo fmt --all

# Run clippy checks on all targets with strict warning denial
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run Meridian spec coherence check
spec-check:
    cargo run -p meridian-spec -- check

# Verify Cargo lock and metadata integrity
metadata-check:
    cargo metadata --locked

# Check for git diff formatting and whitespace anomalies
diff-check:
    git diff --check

# Verify engine-rt does not depend on meridian-ui
verify-ui-isolation:
    ! cargo tree -p meridian-rt | grep -q meridian-ui

# Remove Meridian-owned build caches and redownloadable generated data.
clean:
    @echo "Cleaning Meridian workspace build caches and generated data"
    cargo clean
    if [[ -d examples/creator-alpha/target ]]; then rm -rf -- examples/creator-alpha/target; fi
    if [[ -d assets_built ]]; then find assets_built -mindepth 1 -maxdepth 1 ! -name '.gitkeep' -exec rm -rf -- {} +; fi
    if [[ -d benchmarks/results ]]; then find benchmarks/results -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +; fi

# Run all non-executing static analysis and quality gates
audit: fmt-check clippy spec-check metadata-check diff-check verify-ui-isolation

# ==============================================================================
# 2. Testing
# ==============================================================================

# Run all tests in the workspace
test:
    cargo test --workspace

# Run tests for a specific package (e.g. `just test-package meridian-ui`)
test-package package:
    cargo test -p {{package}}

# ==============================================================================
# 3. Graphics & Render Smokes
# ==============================================================================

# Run RHI clear frame smoke example
rhi-clear-frame:
    cargo run -p meridian-rhi --example clear_frame

# Run renderer instance upload smoke example
renderer-smoke:
    cargo run -p meridian-renderer --example instance_upload_smoke

# ==============================================================================
# 4. Editor Smoke Tests
# ==============================================================================

# Run editor in headless smoke mode (4 frames)
editor-headless-smoke:
    cargo run -p meridian-editor --bin meridian -- --headless-smoke --frames 4

# Run editor in standard smoke mode (4 frames, requires display)
editor-smoke:
    cargo run -p meridian-editor --bin meridian -- --smoke --frames 4

# Run editor UI in headless smoke mode (4 frames)
editor-ui-headless-smoke:
    cargo run -p meridian-editor --bin meridian -- --ui-headless-smoke --frames 4

# Run runtime headless profile smoke test
rt-headless-profile-smoke:
    cargo run -p meridian-rt --example headless_profile_smoke

# ==============================================================================
# 5. Creator Alpha Samples
# ==============================================================================

# Launch the interactive Meridian Creator application with the public sample project.
# Override the project with `just creator project=path/to/project` when needed.
creator project="examples/creator-alpha":
    cargo run -p meridian-editor --bin meridian -- \
      --project "{{project}}"

# Launch the bounded 2x Creator UI review surface and write its capture artifact.
creator-ui-review project="examples/creator-alpha":
    cargo run -p meridian-editor --bin meridian -- \
      --creator-alpha-ui-review \
      --project "{{project}}" \
      --review-workspace world \
      --review-size 1440x900 \
      --capture target/meridian-evidence/creator-alpha-ui-review/creator-alpha-ui.png

# Run end-to-end Creator Alpha smoke test with explicit output path
creator-alpha-smoke:
    cargo run -p meridian-editor -- --creator-alpha-smoke \
      --project examples/creator-alpha \
      --evidence target/meridian-evidence/creator-alpha/manual

# Render Creator Alpha workspace through native UI raster bridge (requires display)
creator-alpha-ui-smoke:
    cargo run -p meridian-editor -- --creator-alpha-ui-smoke \
      --project examples/creator-alpha

# ==============================================================================
# 6. Build Service / Tooling Smokes
# ==============================================================================

# Run build system cargo service smoke test
build-cargo-service-smoke:
    cargo run -p meridian-build --example cargo_service_smoke

# Run build system artifact event smoke test
build-artifact-event-smoke:
    cargo run -p meridian-build --example artifact_event_smoke

# Run compiler build action wrapper tool
build-cargo-compile:
    cargo run -p meridian-build --bin meridian-build -- --cargo-build \
      --workspace . \
      --source-checkpoint local-cargo-build \
      --toolchain local \
      --target host -- -p meridian-core

# Run compiler test-no-run action wrapper tool
build-cargo-test-no-run:
    cargo run -p meridian-build --bin meridian-build -- --cargo-test-no-run \
      --workspace . \
      --source-checkpoint local-cargo-test-no-run \
      --toolchain local \
      --target host \
      --artifact-store target/meridian-build-artifacts \
      --cargo-output-root target/meridian-build-cargo-test -- \
      -p meridian-core --target-dir target/meridian-build-cargo-test

# ==============================================================================
# 7. Packaging
# ==============================================================================

# Package macOS application bundle (universal binary lipo)
package-macos:
    ./scripts/package_macos_app.sh

# ==============================================================================
# 8. Combined CI Validation Gates
# ==============================================================================

# Run all headless gates, unit tests, and software render smoke tests
ci: audit test rhi-clear-frame renderer-smoke editor-headless-smoke editor-ui-headless-smoke rt-headless-profile-smoke creator-alpha-smoke build-cargo-service-smoke build-artifact-event-smoke build-cargo-compile build-cargo-test-no-run

# Run all CI gates + GUI-required smoke tests (requires desktop/display surface)
ci-gui: ci editor-smoke creator-alpha-ui-smoke
