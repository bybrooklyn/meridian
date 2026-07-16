# Marquee Promotional Media and Export Specification

Version 0.5 · 2026-07-15 · Canonical `PRM` authority

[Master](MERIDIAN_MASTER_SPEC.md) · [Data and packages](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Penumbra](RENDERING_AND_GRAPHICS_SPEC.md) · [Animation](ANIMATION_CINEMATICS_AND_FACIAL_SYSTEMS_SPEC.md) · [Wavefront](AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) · [Build](CARGO_IDE_BUILD_AND_TEAM_WORKFLOWS.md) · [Agents and AI](AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [ADR-0025](../docs/architecture/decisions/ADR-0025-marquee-promotional-media.md)

## 1. Authority, status, goals, and non-goals

Marquee is Meridian's adopted promotional-material authoring and export architecture. It owns the `PRM` domain and the future reserved crate name `meridian-marquee`.

Documentation maturity is `ResearchReady`. Implementation maturity is `Deferred`. Delivery authority is `PRG-PRM-001`, a post-1.0 program. No Marquee runtime crate, editor implementation, external-service adapter, quality evidence, or active work package exists. Marquee creates no Meridian 1.0 obligation and cannot satisfy, block, or promote `MS-00` through `MS-10`.

Marquee's goal is to turn manually supplied, approved source material into a complete, reproducible local promotional kit:

- screenshots, thumbnails, key art, capsule art, icons, short clips, and trailers;
- store descriptions, feature lists, content warnings, captions, and alt text;
- PDF press kits, review books, and proof books;
- Steam-file, itch.io-file, YouTube-ready, generic social-ratio, and portable archive bundles;
- source, provenance, claim, approval, AI-assistance, and export manifests.

Marquee is export-only. It does not:

- launch, control, or navigate a game to discover or capture shots;
- log into storefront, video, social, advertising, or publishing services;
- upload, schedule, publish, purchase advertisements, or own accounts;
- generate websites or replace website production;
- generate or alter images, video, voice, music, or sound through AI;
- make a Draft artifact final, infer rights, approve claims, or waive human review;
- make marketing, legal, ratings, localization, accessibility, or creative decisions for a project.

Competitive products and service requirements are capability references only. No parity, superiority, service compatibility, or promotional-quality claim exists without evidence.

## 2. Ownership, dependencies, and forbidden edges

### 2.1 Marquee ownership

Marquee owns:

- campaign identity and campaign-local configuration;
- brand-kit references and promotional templates;
- post-capture edit timelines and deterministic transform recipes;
- target export profiles and profile revision state;
- promotional claims and their evidence bindings;
- draft/final approval state and invalidation;
- locale and variant expansion;
- promotional jobs, artifacts, proofs, and export manifests.

### 2.2 Composed authorities

| Authority | Retained ownership | Marquee use |
|---|---|---|
| DAT | source identity, hashes, rights, provenance, immutable source records | references approved source records and derived-recipe identity |
| Penumbra | rendering and manually initiated capture | imports completed captures; never drives capture |
| ANI | in-engine cinematic sequencing and camera performance | imports rendered output; owns only post-capture promotion timelines |
| Wavefront | audio processing, loudness measurement, and device/DSP contracts | requests bounded offline audio work through adapters |
| BLD | jobs, workers, artifact storage, cancellation, reproducibility | executes Marquee jobs after program activation |
| AGT | provider capabilities, consent, permissions, and AI execution | optional text/analysis suggestions only |
| SEC | privacy, credentials, worker isolation, redaction, and external-tool policy | applies policy to imported media, cloud requests, tools, and exports |
| REL | qualified BuildIds, evidence, and product-claim authority | binds claims to qualified evidence and rejects stale claims |
| Project repository | branding, creative direction, spoilers, embargoes, marketing policy | remains private source authority |

### 2.3 Forbidden edges

- Runtime, game, simulation, Penumbra, ANI, Wavefront, or DAT crates MUST NOT depend on Marquee.
- Marquee MUST NOT become source authority for imported media, project branding, game facts, release qualification, or service credentials.
- Media/PDF tool types MUST stop at adapters and MUST NOT enter Meridian public contracts.
- Project-private source material MUST NOT enter engine fixtures, registries, documentation, or public evidence.
- AI providers MUST NOT receive media or project text without an explicit capability grant and exact disclosure of transmitted data.
- No profile may contain login, upload, publish, scheduling, account-ownership, or advertising actions.

## 3. Planned public contracts and data authority

These are logical contracts. They are not implemented Rust types.

### 3.1 Campaign and source contracts

`CampaignId` is a stable project-scoped identity. `PromotionCampaign` references a target product, approved BuildIds, locales, brand kit, templates, target profiles, source set, claim set, and approval state. `BrandKit` references approved logos, colors, typography, spacing, legal marks, and usage rules without copying project-private assets into engine authority.

`PromotionSource` contains:

- stable source identity and content hash;
- one source class: gameplay capture, staged engine render, composite, external art, licensed stock, logo/brand asset, audio, or authored copy;
- rights owner, license, attribution, redistribution, and transformation disposition;
- consent record where a person, voice, performance, or personal data is present;
- embargo and spoiler classification;
- permitted transforms and target restrictions;
- provenance chain and immutable-original location;
- approval state, approver, and approval time.

Every source MUST be classified, provenance-bearing, and explicitly approved before variant generation. Original inputs are immutable. Crops, grades, overlays, layouts, transitions, resizing, and transcoding are deterministic derived recipes.

### 3.2 Claims and templates

`PromotionClaim` binds claim text to an approved source, qualified `BuildId`, evidence references, locale, review owner, and expiry. Claims include feature, platform, performance, accessibility, content, availability, and compatibility statements. Unqualified superlatives, unsupported comparisons, and evidence-free performance claims are rejected.

`LayoutTemplate`, `PromotionTimeline`, and `CopyTemplate` are versioned project source. A promotion timeline edits imported footage only; it is not an ANI sequence and cannot control runtime cameras or gameplay.

### 3.3 Profiles, jobs, and artifacts

`ExportProfile` records:

- target name and profile revision;
- official primary-source URL and last verification date;
- dimensions, aspect ratios, safe areas, codecs, color/audio expectations, and size limits;
- required metadata, locale rules, and filename policy;
- unsupported features, fallback policy, and stale-after rule.

Profiles describe file outputs. They contain no credentials or publishing calls. Target names such as Steam, itch.io, and YouTube mean ready-to-review files, never direct integration.

`PromotionJob` is bounded, cancellable, reproducible, and worker-safe. `PromotionArtifact` records source hashes, recipe hash, tool versions, output hash, dimensions/duration/format, locale, target, and state. `ApprovalRecord` records the human decision, exact inputs, scope, time, and invalidation keys. `AiAssistRecord` records provider class, model/tool identity where known, prompt/input classification, exact data disclosure, output hash, human disposition, and whether cloud execution occurred. `PromotionManifest` closes the complete bundle.

Artifacts have two explicit states:

- `Draft`: local proof, incomplete or unapproved, never represented as final;
- `ReleaseReady`: all required validations and explicit human approval passed for exact inputs.

## 4. Ordered pipeline and state machines

The canonical pipeline is:

```text
create campaign
-> import and classify approved sources
-> validate rights, consent, spoilers, embargoes, and target BuildId
-> bind truthful claims and approved copy
-> select locales, templates, and service profiles
-> generate deterministic variants
-> optionally request untrusted AI text/analysis suggestions
-> human review
-> export Draft proofs
-> explicit ReleaseReady approval
-> revalidate source hashes, claims, profiles, rights, and approvals
-> atomically export files, PDFs, and manifest
```

The campaign state machine is `Draft -> Reviewable -> Approved -> Exporting -> ReleaseReady`. Any failed validation returns to `Draft` or a typed blocked state. `ReleaseReady` is not inferred from successful encoding.

Any changed source, source hash, claim, BuildId, template, locale, target profile, AI suggestion, approval, tool version, font, or transform recipe invalidates only the affected final approvals and dependent artifacts. The invalidation graph MUST be inspectable before rebuilding.

Failed encoding, cancellation, disk exhaustion, stale profiles, missing fonts, invalid rights, worker loss, or manifest failure preserves the previous accepted bundle. Atomic replacement occurs only after every selected output and the manifest verify.

## 5. Clocks, threading, memory, lifetime, and reproducibility

Marquee has no frame-loop or runtime clock. It uses monotonic operation time for diagnostics and wall-clock timestamps only for provenance, approvals, embargoes, expiries, and profile freshness.

Jobs execute through BLD-owned bounded workers. Each stage declares input limits, decoded dimensions/duration, estimated memory, scratch storage, output limits, cancellation points, and concurrency. Decode metadata is untrusted until bounded validation completes. Large media is streamed or tiled where adapters support it; whole-corpus residency is forbidden by default.

Campaign source documents are durable project authority. Imported originals are immutable project data. Previews, thumbnails, transcodes, PDF pages, and archives are derived artifacts. Caches may be deleted and rebuilt. Accepted bundles use atomic directory or archive replacement with manifest verification.

Reproducibility records include campaign revision, source and claim hashes, BuildId, locale, profile revision, recipe/template versions, font identities, adapter/tool versions, AI-assist records, approvals, and output hashes. Tool nondeterminism is reported, not hidden.

## 6. Failure, recovery, diagnostics, security, and provenance

Every failure is typed and identifies the affected source, target, locale, operation, stage, recovery action, and whether the prior accepted bundle remains valid. Required diagnostics include:

- source classification and provenance failures;
- missing, conflicting, expired, or incompatible rights and consent;
- spoiler or embargo conflicts;
- stale BuildId, claim, source hash, profile, approval, or tool identity;
- unsupported codec, color space, font, PDF feature, or adapter;
- estimated and actual memory, scratch, duration, and output size;
- cancellation latency, worker loss, retry, and atomic-replacement outcome;
- AI request disclosure, provider result, rejection/acceptance, and invalidation;
- Draft/ReleaseReady state and the exact human approver.

Imported media, fonts, archives, project copy, templates, AI output, and external-tool output are untrusted. Workers receive capability-scoped paths, no ambient network or credentials, bounded resources, and sanitized environments. Secret scanning covers source metadata and manifests. Export paths cannot escape the selected destination. Archives reject traversal, links, duplicate names, and decompression bombs.

The private Project Meridian repository retains campaign sources, AMI branding, spoilers, claims, captures, layouts, copy, approvals, and final exports. Engine evidence may expose only sanitized IDs, hashes, profile results, redacted failure classes, and generic fixtures.

## 7. Human workflows, accessibility, localization, and AI

### 7.1 Beginner workflow

1. Create a campaign from a project-owned preset.
2. Import manually captured screenshots, clips, approved art, audio, and copy.
3. Resolve a plain-language checklist for rights, consent, spoilers, embargo, and BuildId.
4. Choose target bundles and locales.
5. Preview generated variants and accessibility metadata.
6. Export Draft proofs.
7. Review differences and errors with keyboard and screen-reader support.
8. Approve exact outputs as ReleaseReady and export the atomic bundle.

### 7.2 Expert workflow

Experts may edit textual campaign/template/profile source, run headless validation and export, inspect dependency graphs and hashes, compare tool versions, pin adapters/fonts, and consume machine-readable manifests. Expert paths use the same approval and policy checks.

Every visible workflow defines keyboard order, focus restoration, semantic labels, scalable previews, contrast warnings, motion-reduction playback, caption/audio inspection, error summaries, and recovery. Safe-area and crop previews cannot rely on color alone. Localization previews expose truncation, missing glyphs, reading direction, captions, alt text, and fallback fonts.

### 7.3 Optional AI policy

AI MAY:

- draft or revise copy;
- suggest tags, shot order, layouts, crops, captions, and alt text;
- inspect imported media for consistency or technical problems;
- assist localization review.

AI MUST NOT generate or alter image, video, voice, music, or sound artifacts. AI returns non-authoritative text or metadata suggestions only. Visual and audiovisual transforms remain deterministic and human-approved.

Local providers and explicitly approved cloud providers use AGT capabilities. Cloud execution is opt-in and discloses exactly what leaves the machine before submission. Disabling AI adds no dependency, task, listener, allocation, panel, network access, or package content. An AI suggestion never approves itself, changes source authority, or satisfies `ReleaseReady` review.

## 8. Capability tiers and zero-cost-disabled behavior

Planned tiers are:

| Tier | Capability |
|---|---|
| Core local | campaign/source/claim records, deterministic image variants, copy, manifests, portable archive |
| Media local | bounded clip/trailer composition, transcode, audio processing, captions |
| Publication proof | PDF press/review books, locale/target matrices, ReleaseReady approval |
| Optional AI | text and analysis suggestions through AGT only |

Unsupported formats or tools return typed capability outcomes and offer an explicit manual/export fallback. A missing optional adapter cannot corrupt a campaign. AI, video, PDF, audio, and target-profile packs are separately omittable. Disabled packs add no code, processes, panels, workers, network access, or artifacts to unrelated profiles.

## 9. Requirements, program, research, validation, and risks

| Requirement | Normative result |
|---|---|
| `REQ-PRM-001` | Approved provenance-bearing source classification and immutable originals drive deterministic derived variants. |
| `REQ-PRM-002` | Claims bind to qualified BuildIds and evidence; stale or misleading claims are rejected. |
| `REQ-PRM-003` | Draft/ReleaseReady states, explicit human approval, dependency invalidation, and atomic recovery are enforced. |
| `REQ-PRM-004` | Export profiles are revisioned, primary-source verified, local, reproducible, accessible, and export-only. |
| `REQ-PRM-005` | Optional AI is limited to non-authoritative text/analysis suggestions with disclosure, approval, and zero-cost-disabled behavior. |
| `REQ-PRM-006` | Public fixtures and evidence remain generic while private campaign content stays in the consumer repository. |

`PRG-PRM-001` is `Deferred` and opens only after `MS-10`, a future planning review, funded ownership, approved security/licensing policy, and `RG-PRM-001`. Activation creates bounded future `WP-PRM-*` records; the program itself is never an active work package.

`RG-PRM-001` selects mature local image, video, audio, and PDF adapters. It compares cross-platform coverage, deterministic behavior, licensing and patent exposure, sandboxability, performance, memory, accessibility, maintenance, and escape paths. Marquee-owned contracts remain stable. Custom Meridian codecs are not presumed.

`VAL-PRM-001` is `DefinitionOnly` / `Uncalibrated`. Its public generic corpus uses synthetic branding, manually supplied generic captures, licensed test media, multiple locales, stale-profile cases, AI-disabled cases, and no Project Meridian creative content.

Registered risks are:

| Risk | Subject |
|---|---|
| `RISK-PRM-001` | stale, misleading, or unsupported claims |
| `RISK-PRM-002` | rights, license, attribution, or consent contamination |
| `RISK-PRM-003` | spoilers, embargoes, private data, or private-corpus leakage |
| `RISK-PRM-004` | service profile drift and invalid exports |
| `RISK-PRM-005` | tool nondeterminism and irreproducible output |
| `RISK-PRM-006` | AI boundary violation, hallucination, or cloud disclosure failure |
| `RISK-PRM-007` | codec, font, PDF, patent, or adapter licensing/availability failure |
| `RISK-PRM-008` | source misclassification or misleading staged/composite media |
| `RISK-PRM-009` | locale/target expansion exceeds memory, time, or review capacity |
| `RISK-PRM-010` | Draft/final confusion or approval bypass |

No PRM evidence exists for output quality, service compliance, adapter suitability, or implementation. `EV-GOV-20260715-005` proves only documentation/governance coherence.

## 10. End-to-end, failure, and performance-debug examples

### 10.1 End-to-end export

A creator manually records a generic gameplay clip and screenshots, imports approved logo/audio/copy sources, binds claims to a qualified BuildId, selects two locales and local export profiles, generates deterministic variants, accepts a suggested alt-text revision, exports proofs, and approves exact hashes. Marquee revalidates all dependencies and atomically writes images, video, PDFs, copy, and a manifest. No service is contacted.

### 10.2 Stale claim recovery

A campaign claims a supported feature from Build A. Build B removes it. BuildId invalidation marks the claim and dependent outputs stale; Draft proofs remain inspectable, ReleaseReady export is blocked, and the previous accepted bundle remains untouched until a reviewer replaces or removes the claim.

### 10.3 Worker failure

A video worker exits during transcode. The job reports stage, tool, input hash, retryability, and scratch cleanup. Other independent variants may continue within policy, but the atomic bundle is not replaced. Restarting uses the same recipe and records whether output hashes differ.

### 10.4 Performance diagnosis

A multi-locale trailer matrix exceeds memory. Diagnostics attribute decoded-frame residency, adapter buffers, font caches, concurrency, scratch use, and output throughput by stage. The operator reduces concurrency or selects a lower-memory adapter profile; Marquee never silently drops locales, frames, captions, or quality settings.
