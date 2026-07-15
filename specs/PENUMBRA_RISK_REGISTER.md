# Penumbra Risk Register

[Rendering](RENDERING_AND_GRAPHICS_SPEC.md) · [Roadmap](DELIVERY_ROADMAP.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Research](RESEARCH_AND_ALGORITHM_DECISIONS.md)

version 0.5 · 2026-07-15 · Normative risk authority

Documentation maturity: `ResearchReady`. Penumbra architecture: `Adopted`.
Implementation: `Partial` / `Transitional`. Registry metadata:
[`specs/registry/risks.json`](registry/risks.json).

## 1. Risk policy

Every risk records owner, affected milestone, trigger/indicator, mitigation,
evidence, fallback, and closure decision. A risk may stay open while work
continues, but it cannot be hidden by a waiver or converted into a capability
claim. Performance and visual risks use permanent Penumbra workloads and
preregistered thresholds; private corpora use redacted hashes only.

## 2. Register

| ID | Risk and trigger | Mitigation and evidence gate |
|---|---|---|
| RISK-PEN-001 | Alpha-tested foliage and transparency produce pathological overdraw or unstable lower-tier pacing. | Measure PEN-B02, PEN-B03, and PEN-B14 with overdraw, fragment cost, temporal artifacts, and tiered content/algorithm policy before MS-05. |
| RISK-PEN-002 | Cluster storage, indexing, or pathological light occupancy exceeds memory/bandwidth limits. | Bound dimensions/counts, expose occupancy/overflow diagnostics, test PEN-B04/PEN-B13, and preserve a correctness fallback before `WP-PEN-010` closes. |
| RISK-PEN-003 | Material, light, path, backend, and capability combinations create unmaintainable shader permutations. | One `MaterialIr`/`ShaderIr`, explicit specialization domains, deterministic cache keys, variant reports, and PEN-B12 stress evidence. |
| RISK-PEN-004 | Pipeline compilation or cache misses stall active traversal. | Build/warmup manifest, runtime-creation diagnostics, cache provenance, cold/warm PEN-B12, and shipping-profile prohibition after declared warmup. |
| RISK-PEN-005 | Temporal reconstruction ghosts, shimmers, pumps, or fails under disocclusion, foliage, rotation, traversal, weather, and rebasing. | History validity contract, debug views, reference captures, and PEN-B03/PEN-B08/PEN-B09/PEN-B15 evidence. |
| RISK-PEN-006 | Apple unified-memory pressure or low-memory profiles cause churn, stalls, or unsafe eviction of required content. | Domain budgets, residency reasons, staged degradation, memory-pressure recovery, and PEN-B11 on named capability profiles. |
| RISK-PEN-007 | Metal, Vulkan, Direct3D 12, and wgpu behavior diverge in image, synchronization, limits, recovery, or timing semantics. Apple M4/macOS 27 reproduced invalid legacy Metal timestamp data consistent with open [wgpu issue 9414](https://github.com/gfx-rs/wgpu/issues/9414). | Common differential suite, capability records, backend-specific known-limit rows, rollback, and `RG-RHI-001`. `WP-PEN-007` rejects invalid raw pairs, reports `UnsupportedPlatform`, disables GPU timing for that RHI lifetime, and preserves CPU timing; this mitigation is not backend-parity closure. |
| RISK-PEN-008 | RHI abstraction imposes material CPU/GPU cost or blocks backend features. | Measure call/translation/allocation cost, retain typed escape-free capabilities, and require `RG-RHI-001` evidence before expanding abstraction. |
| RISK-PEN-009 | Multiple native backends exceed available maintenance and validation capacity. | Metal-first staffing/maintenance review; Vulkan/D3D12 only after common-RHI maturity; retain wgpu and declare unsupported rows. |
| RISK-PEN-010 | Successor research fragments shared systems, materials, debugging, workloads, or artist workflows. | Enforce path-independent contracts, dev-only experimental profiles, complete `RG-PEN-001` parity, and a separate post-promotion Forward+ ADR. |
| RISK-PEN-011 | Borrowed/donor renderer code has incompatible license, unclear provenance, generated-source uncertainty, or no exit path. | No import before a complete `third_party/provenance` record, license review, exact revision/hash, modifications, tests, owner, and exit strategy. |
| RISK-PEN-012 | Renderer architecture cannot meet stereo consistency, multiview/foveation, predicted timing, late pose, memory, and comfort constraints. | Preserve VR capability tier and render-view seams; run PEN-B16 only when XR begins; do not claim XR before MS-09 evidence. |
| RISK-PEN-013 | Private Project Meridian corpus, AMI creative material, paths, documents, or assets leak into engine records. | Generated/redacted corpora only, source-hash boundary, private-content audit, `ADR-0016`, and immediate unwaivable failure on leakage. |

## 3. Current foundation risks

The current direct PBR, cascaded shadows, diffuse irradiance IBL, extraction,
upload, pass timing, asynchronous capture, RHI, and render graph are useful foundations.
An offscreen-visible smoke may still be mistaken for presented or production visual quality, diffuse IBL may be mistaken for
complete environment lighting, and high-level pass instrumentation may be
mistaken for complete graph coverage. `REQ-PEN-003` requires visible captures;
`REQ-PEN-004` keeps `WP-PEN-006` diffuse IBL separate from `WP-PEN-009`
specular IBL/BRDF LUT; future render-graph execution must reuse the
`WP-PEN-007` timing contract for every claimed production pass.

`WP-PEN-007` and `WP-PEN-008` are implemented foundations. `WP-PEN-008`
closes the bounded RGBA8 readback/metadata slice, not HDR capture, tone mapping,
presented-surface proof on every platform, or visual-quality qualification.
Starting Forward+ or successor work does not erase those later image and graph-
coverage evidence debts.

## 4. Successor-path risk gate

`RG-PEN-001` opens after MS-05. Candidates use identical generated/redacted
corpora, capabilities, settings, resolution, warmup/cache policy, statistical
method, and raw-evidence retention. Thresholds are preregistered per workload
and platform before measurement.

Promotion requires feature parity, equal-or-better artistic results, meaningful
capability/performance advantage, lower-tier and native-backend stability,
acceptable shader/pipeline behavior, no material frame-time/memory/debugging
regression, and sustainable maintenance. A roughly two-percent isolated win
cannot justify substantial complexity. High-end-only success cannot compensate
for regressions on supported lower tiers.

## 5. Native-backend risk gate

`RG-RHI-001` remains closed until MS-07 and stable RHI review. Native Metal is
first. Native Vulkan and Direct3D 12 begin only after Metal and the common RHI
pass differential image, benchmark, surface/device recovery, backend-divergence,
staffing, provenance, and maintenance gates. wgpu remains an available backend
throughout.

## 6. Closure and review

Risk closure requires linked evidence IDs, review owner, affected requirement
and work package, residual risk, and ADR where architecture changes. Waivers
cannot close risks or promote maturity. Open release-critical risks appear in
the MS-10 qualification review with explicit supported/unsupported profiles.
