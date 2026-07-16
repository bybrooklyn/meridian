# ADR-0025: Adopt Marquee as a Post-1.0 Promotional Export System

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.5
- Documentation status: ResearchReady
- Implementation status: Deferred
- Owners: product, release, build, data, security, accessibility
- Amends: ADR-0011, ADR-0013, ADR-0014, ADR-0018, ADR-0024
- Supersedes: none

## Context

Meridian projects need reproducible screenshots, trailers, store-ready files, copy, accessibility metadata, press material, and approval records. These activities currently appear only as scattered project production duties. Folding them into Penumbra, ANI, BLD, DAT, or a service integration would duplicate authority and risk misleading publishing claims.

The user requires one Meridian application, manual capture selection, local export, no automatic publishing, and no AI-generated or AI-modified audiovisual media. Optional AI text and analysis assistance is acceptable when untrusted, disclosed, and human-approved.

## Decision

Adopt **Marquee** as the `PRM` domain and reserve future `meridian-marquee`. Marquee owns campaigns, promotion templates and post-capture timelines, claims, approvals, variants, target profiles, and export manifests. It composes DAT provenance, Penumbra manual capture, ANI sequencing, Wavefront processing, BLD jobs, AGT permissions, SEC isolation, REL evidence, and private project authority without replacing them.

Marquee is `PRG-PRM-001`, `Deferred`, and post-1.0. It cannot satisfy, block, or promote `MS-00` through `MS-10`. No placeholder crate is created.

Capture is manual: Marquee imports supplied screenshots, clips, renders, art, audio, and copy. It never launches or navigates a game to find shots. It exports local files only and never logs in, uploads, schedules, publishes, purchases advertising, generates websites, or owns accounts.

Optional AI is limited to non-authoritative text and analysis suggestions. AI cannot generate or alter images, video, voice, music, or sound. Cloud execution is explicit opt-in with exact data disclosure. Human approval is mandatory for `ReleaseReady` output, and changed inputs invalidate approval.

Media, audio, and PDF implementations are adapter-first behind Meridian contracts. `RG-PRM-001` selects mature tools after `MS-10`; custom Meridian codecs are not presumed.

## Consequences

- Promotional production gains one typed, recoverable, provenance-bearing authority.
- Existing runtime and authoring subsystem boundaries remain intact.
- Project branding, spoilers, claims, sources, and exports remain private.
- Service profile names describe file contracts, not service integrations.
- Marquee adds no 1.0 dependency or present implementation claim.
- A future planning review must create bounded `WP-PRM-*` packages before code begins.
