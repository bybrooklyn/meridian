# Collective Online Services Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [ADR-0020](../docs/architecture/decisions/ADR-0020-wavefront-and-collective.md) · [Networking](MULTIPLAYER_AND_SERVER_SPEC.md) · [Wavefront](AUDIO_MUSIC_AND_ACOUSTICS_SPEC.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) · [Distributed worlds](DISTRIBUTED_WORLDS_AND_MMO_SPEC.md)

Status: version 0.5 normative architecture, 2026-07-15.

Architecture status: `Adopted` by ADR-0020. Documentation maturity: `ResearchReady`. Implementation maturity: `Planned`.
Governing IDs: `REQ-COL-001` through `REQ-COL-004`; `WP-COL-001`; `PRG-COL-001`.

Current implementation status: no Collective identity, session, lobby, matchmaking, social, voice-service, analytics, moderation, provider-adapter, or self-hosted reference service exists. Meridian does not promise to operate hosted cloud infrastructure.

## 1. Authority and Scope

Collective is Meridian's optional all-in-one online-services subsystem. The `COL` domain owns provider-neutral service contracts for identity/accounts, sessions/lobbies/parties/invites/matchmaking, presence/friends/groups/messaging/social features, voice-session coordination, privacy-conscious product analytics, moderation/abuse handling, provider adapters, and self-hostable reference deployments.

Collective is one named subsystem but internally modular. Projects select only the modules and providers they need. NET owns transport, replication, prediction, server simulation, and connection primitives. Wavefront owns voice capture/playback/DSP and device behavior. SEC owns cross-suite trust policy. WRL owns distributed-world topology. Games own product policy, communities, retention choices, and moderation staffing.

## 2. Goals and Non-Goals

Goals:

- give projects stable contracts without tying game code to one commercial provider;
- support offline, LAN, dedicated-server, self-hosted, and provider-backed deployments;
- make consent, retention, deletion, export, parental/privacy settings, sanctions, and appeals explicit;
- degrade per module and preserve local/offline play when product rules allow;
- make costs, quotas, outages, trust, region, and provider behavior observable;
- provide reference schemas and deployments that projects can operate without Meridian-hosted infrastructure.

Non-goals:

- no promise that Meridian will operate accounts, matchmaking, social, analytics, relay, voice, or moderation services without sustainable funding and operations;
- no mandatory account or telemetry for local/offline projects;
- no general advertising profile, cross-project tracking identity, or sale of personal data;
- no hidden provider dependency, proprietary client lock-in, or online requirement for core engine use;
- no claim that software alone supplies community policy, legal compliance, or human moderation capacity.

## 3. Planned Contracts

```text
CollectiveCapability { module, version, region, provider, limits, policy }
IdentitySubject { local_project_id, provider_links, display_profile, consent_revision }
SessionDescriptor { id, members, roles, join_policy, regions, game_build, metadata }
MatchRequest { party, playlist, constraints, preferences, latency_samples, timeout }
RelationshipRecord { subject, other, state, scope, provenance, revision }
MessageEnvelope { id, sender, recipients, channel, content_class, policy_labels }
VoiceSessionBinding { room, participants, permissions, transport_ref, audio_policy }
AnalyticsEvent { schema_id, purpose, consent_basis, fields, retention, sampling }
ModerationCase { id, reports, evidence_refs, status, decisions, appeals, audit }
ProviderAdapter { capabilities, mappings, limits, failure_model, data_locations }
```

Public game-facing IDs are Meridian/project IDs with explicit provider links. Provider tokens, SDK handles, web request objects, and private moderation evidence never cross public gameplay interfaces.

## 4. Ownership and Forbidden Edges

| Concern | Collective owns | Neighbor owns |
|---|---|---|
| connection/replication | discovery/session metadata and authorization input | NET transport, channels, snapshots, prediction |
| voice | room membership, permissions, provider coordination | Wavefront capture, DSP, device output, accessibility |
| identity/privacy | account links, consent, retention, export/delete workflows | project legal/policy decisions and platform requirements |
| analytics | typed purpose-bound event collection policy | diagnostics for engine debugging; project analysis decisions |
| moderation | report/case/sanction contracts and provider seams | game/community policy and human decision process |
| distributed worlds | admission/identity/service lookup | WRL topology, simulation partition, migration |

Forbidden edges include provider SDK types in game/runtime schemas, analytics events without purpose and retention, voice capture without permission, social features silently activating network services, client authority over sanctions or inventory, and engine CI requiring private service credentials.

## 5. State Machines and Pipelines

Session:

```text
offline/local
-> resolve selected Collective modules and providers
-> authenticate or establish explicit guest/local identity
-> negotiate policy/capabilities/build compatibility
-> create/join party or lobby
-> perform bounded discovery/matchmaking
-> authorize NET connection
-> maintain presence/voice/social modules independently
-> leave, revoke, expire, or recover with audit record
```

Analytics:

```text
typed event request
-> schema and purpose validation
-> consent/age/region/project policy
-> redact/minimize fields
-> bounded local queue and sampling
-> encrypted provider dispatch or local sink
-> retention/deletion/export accounting
```

Moderation:

```text
report or automated signal
-> preserve bounded evidence with provenance
-> triage under project policy
-> human/authorized decision where required
-> apply scoped sanction through typed authority
-> notify and support appeal
-> retain/delete according to policy and law
```

Provider outages transition each module independently through available, degraded, retrying, unavailable, or disabled. They never create unbounded retry storms.

## 6. Threads, Memory, Failure, Security, and Diagnostics

Online I/O runs asynchronously with bounded queues, deadlines, cancellation, backoff, circuit breaking, and regional/provider quotas. Credentials use platform-secure storage and isolated service processes where appropriate. Sensitive records have purpose, owner, retention, encryption, access, and deletion metadata.

Failures include provider outage, quota, region mismatch, auth expiration, policy rejection, incompatible build, stale membership, voice denial, moderation-service delay, consent withdrawal, and local queue saturation. Typed outcomes distinguish retryable, user-action, policy, unavailable, and permanent failures.

Diagnostics include module/provider/version, request/trace ID, state transition, queue and service latency distributions, error class, quota/backoff, region, redacted policy decision, and data lifecycle status. Secrets, message content, biometric voice data, and moderation evidence are redacted by default.

Security requirements include least privilege, token rotation, replay resistance, encrypted transport, server-side authority, abuse rate limits, audit logs, dependency provenance, provider exit plans, incident response, and explicit threat models. Self-hosted references ship secure defaults but do not claim unattended production readiness.

## 7. Accessibility, Privacy, and Zero-Cost Behavior

Invite, lobby, privacy, block/mute/report, voice, consent, export, deletion, and appeal workflows are keyboard and screen-reader accessible, localization-ready, and usable without color or audio alone. Voice integrates captions/transcription only under explicit policy and consent; text alternatives remain available where product design permits.

Offline projects compile and run without Collective initialization, accounts, telemetry, workers, SDKs, network calls, secrets, or package chunks. Selecting one module does not activate the others. Provider adapters are optional capability packs.

## 8. Requirements, Delivery, and Evidence

- `REQ-COL-001`: provider-neutral modular identity, session, social, voice-coordination, analytics, and moderation contracts with adapter differential evidence.
- `REQ-COL-002`: explicit consent, privacy, retention, export/deletion, safety, audit, and accessibility behavior with failure and policy fixtures.
- `REQ-COL-003`: offline/self-hostable operation, per-module degradation, provider exit, and zero-cost-disabled proof.
- `REQ-COL-004`: no hosted-service promise without funded operational, legal, security, reliability, and moderation evidence.
- `WP-COL-001`: provider-neutral/self-hostable baseline after NET/Wavefront/SEC contracts stabilize.
- `PRG-COL-001`: post-1.0 provider ecosystem and any funded hosted-scale service program.

Tests cover offline stripping, mock/self-hosted/provider adapters, auth/session state machines, latency/outage/quota, reconnect/idempotency, consent withdrawal, data export/delete, privacy redaction, block/mute/report, sanctions/appeals, voice permissions, secrets, accessibility, and adversarial clients. No production-service claim can be based solely on mocks.

## 9. Examples

End to end: a project enables parties, lobbies, and voice coordination with a self-hosted adapter; Collective authorizes membership; NET connects the game; Wavefront handles audio.

Failure: the social provider fails while a match is active. Presence becomes unavailable, retry is bounded, and NET gameplay continues because module authority is separate.

Performance debug: a join delay decomposes identity refresh, policy evaluation, lobby lookup, match search, region probe, and NET connection instead of reporting one online wait.
