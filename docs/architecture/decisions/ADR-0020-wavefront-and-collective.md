# ADR-0020: Wavefront Audio and Collective Online Boundaries

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
- Implementation status: Wavefront Scaffold; Collective Deferred
- Owners: audio, networking, security, services
- Amends: ADR-0014, ADR-0015
- Supersedes: none

## Context

Meridian needs reusable names and clear authority for offline audio and optional online capabilities. Audio-device processing and network/social service authority have different clocks, trust boundaries, availability, and deployment obligations.

## Decision

Wavefront is Meridian's audio, music, acoustics, device, mixer, spatialization, and audio-signal-processing subsystem. It may provide capture, echo cancellation, gain control, noise suppression, encoding adapters, and accessible audio presentation for voice communication.

Collective is Meridian's unified optional online subsystem. It owns provider-neutral identity abstraction, sessions, lobbies, parties, invitations, matchmaking, presence, social graphs, messaging, voice-channel policy and transport, privacy-conscious analytics, moderation, abuse controls, provider adapters, and self-hostable reference services.

Wavefront does not own accounts, channels, encryption, moderation, transport, retention, or service deployment. Collective does not own the audio callback, device graph, spatial mixer, or game-audio source authority. Typed packets and immutable audio frames cross the seam.

Meridian does not promise a publicly operated Collective cloud before independent funding, legal/privacy readiness, operations staffing, incident response, and a separate adopted decision. Offline projects require no account, listener, telemetry, service process, SDK, or Collective package.

## Consequences

- Existing audio specification is retitled around Wavefront without renaming the scaffold crate in this documentation pass.
- Collective uses governance domain `COL`; social and analytics remain optional modules inside it rather than separate domains.
- Core networking remains `NET` and can be used without Collective.
- Distributed worlds (`WRL`) and integrity (`INT`) integrate with Collective but retain separate authority.
