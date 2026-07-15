# Multiplayer and Server Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) · [Distributed worlds](DISTRIBUTED_WORLDS_AND_MMO_SPEC.md) · [Gameplay](GAMEPLAY_NARRATIVE_AND_SCRIPTING_SPEC.md) · [Packages/security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)

version 0.5 · 2026-07-15 · Normative architecture · Deferred to MS-09

Documentation maturity: `ResearchReady`. Implementation maturity: `Deferred`.
Governing IDs: `REQ-NET-001`, `WP-NET-001`.

## 1. Scope

Meridian supports transport-neutral client/server games, dedicated and listen servers, replication, prediction/reconciliation, rollback where selected, interest management, package/mod negotiation, and optional transport/relay adapters. [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) separately owns identity, accounts, sessions, lobbies, parties, matchmaking, presence, social, voice-room policy, analytics, and moderation services.

Project Meridian is single-player. Networking is not a MS-06/MS-07 dependency. This spec exists now so persistent IDs, clocks, gameplay schemas, saves, world streaming, Cairn, packages, and optional features keep compatible seams.

## 2. Non-goals

- peer-authoritative trust by default;
- one transport/provider embedded in gameplay APIs;
- bit-identical global rollback for every genre;
- hiding latency with unbounded client authority;
- mandatory Steam/EOS/cloud accounts;
- loading renderer/audio/editor code in dedicated servers;
- stable compatibility for unpublished internal messages.

## 3. Crates and processes

- meridian-net-core: protocol IDs, messages, channels, clocks, connection state.
- meridian-net-transport: trait and native/QUIC/loopback adapters selected by research.
- meridian-replication: schema-derived snapshots/deltas/interest.
- meridian-prediction: input history, reconciliation, rollback policies.
- meridian-server: headless world host, admission, persistence, operations.
- transport/provider adapters: sockets, QUIC or other selected transports, NAT/relay capability, and platform connection bootstrap.
- Collective adapters: optional authentication, lobby/session, matchmaking, social, voice-policy, analytics, and moderation services outside NET.

Game modules declare replicated schemas and authority policy. NET providers and Collective providers do not own simulation, replication, entity identity, or package formats.

## 4. Connection lifecycle

~~~text
Disconnected -> Resolving -> TransportConnected -> Authenticating
-> NegotiatingProtocol -> NegotiatingContent -> Synchronizing
-> Active -> Draining -> Disconnected
any -> Rejected | TimedOut | Banned | Migrating
~~~

Each transition has deadline, retry policy, user-facing diagnostic, and redacted audit record. Authentication and session discovery may come from Collective, while transport encryption and game authorization remain separate capabilities. Offline/direct/LAN projects can use local identities and discovery without Collective.

## 5. Protocol

~~~text
ProtocolHello {
  protocol_range, engine_api_hash, game_id, game_version,
  schema_set_hash, package_set, mod_set,
  capabilities, transport_properties, nonce
}

MessageEnvelope {
  type_id, schema_version, channel, sequence,
  simulation_tick, payload_length, payload
}
~~~

Messages are length-delimited, bounded, versioned, fuzzed, and unknown-message policy is explicit. Reliable ordered, reliable unordered, and unreliable sequenced semantics are declared independently of transport.

## 6. Replication

Components opt in through schema metadata: authority, relevance, frequency, quantization, delta strategy, owner visibility, reliability, priority, and migration. PersistentEntityId maps to connection-local compact IDs with generation and tombstone policy.

Server builds immutable per-client replication views after simulation commit. Interest combines spatial cells, rooms/visibility, gameplay relevance, ownership, audio/physics needs, and budgets, reusing world scheduling concepts without coupling crates.

## 7. Prediction and rollback

Clients timestamp semantic input by local sequence and intended simulation tick. Server validates and applies authority. Client prediction keeps bounded input/state history for selected components; reconciliation compares authoritative snapshots and reapplies permitted inputs.

Rollback is opt-in per system with declared state, side-effect suppression/compensation, maximum window, memory budget, and deterministic requirements. Audio, particles, achievements, saves, and external commands are not repeated blindly.

## 8. Cairn and gameplay

Physics replication uses Cairn-native IDs/state/events, never Rapier internals. Genres choose snapshot interpolation, local character prediction, rollback islands, or server-only physics. Contacts are not universally replicated; gameplay events are schema-defined.

Rust and optional Luau gameplay receive the same authority-safe API. Client code cannot invoke server-only commands or inspect hidden replicated fields.

## 9. Dedicated server

Headless server profile excludes renderer, graphics assets, runtime UI, local audio, editor, and unrelated packs. It includes world/gameplay/Cairn/network/save/package verification/diagnostics and only required material facets.

Operations: config validation, immutable BuildId/package set, graceful drain, health/readiness, admin capabilities, logs/metrics/traces, backups/checkpoints, crash restart, rolling compatibility policy, rate/ban controls.

## 10. Transport Providers and Collective Seam

NET adapters map transport, relay/NAT, connection bootstrap, and platform packet capabilities into core contracts. Collective adapters map authentication, sessions/lobbies, invites, matchmaking, social, voice policy, analytics, and trust/safety into Collective contracts. A vendor SDK may implement both adapter families, but the engine boundaries remain separate and game code sees neither vendor handle.

Provider outage yields explicit offline/direct/other-provider behavior per selected module. Credentials/tickets are secrets and never stored in project/save/log/trace. SDK availability, redistribution, data location, and service operation remain gated by current agreements and project policy. Meridian does not promise to operate hosted services.

## 11. Content and mods

Before Active, peers compare game, schema, package, and mod hashes plus signature/trust policy. Missing distributable content may use an authorized source; otherwise connection fails with exact mismatch. Servers can require signed-only, approved list, capability limits, or no mods.

Package transfer is separate from gameplay traffic, bounded, resumable, hash verified, and never installs executable native code without explicit policy.

## 12. Threading and memory

Network IO uses dedicated bounded lanes or runtime integration; message parsing/decompression occurs off simulation. Simulation consumes immutable input/message batches at barriers and publishes snapshots after commit.

Per-connection memory, bandwidth, queued messages, history, decompression, and entity maps have hard limits. Slow/hostile clients degrade/disconnect rather than growing queues indefinitely.

## 13. Diagnostics and security

Expose state transitions, RTT/jitter/loss/reorder, bytes/messages/channel, queue age, snapshot/delta size, interest counts, prediction error, rollback cost, content negotiation, rate limits, and server tick timing.

Threats: malformed/flooded packets, replay, amplification, auth theft, hidden-field leaks, command spoofing, decompression bombs, mod mismatch, admin abuse, provider/Collective compromise, and dependency compromise. Protocol and parser fuzzing plus least-privilege admin are mandatory.

## 14. Editor/CLI workflow

Beginner: choose local host/join template, run two simulated clients, see connection/content errors, and start an impairment preset.

Expert: inspect schema, authority, per-field bits, interest reason, prediction history, packet capture with redaction, server tick trace, and transport/provider capability.

Planned commands cover server validate/run/drain, net simulate/replay/inspect, schema compatibility, package/mod negotiation, and soak.

Connection, invite, mismatch, moderation, and recovery flows must support
keyboard/controller navigation, screen readers, text scaling, non-color-only
status, and configurable communication/cue presentation. Network authority may
not force inaccessible timing or input assumptions into gameplay contracts.

## 15. Tests and benchmarks

- protocol compatibility and migration fixtures;
- parser/property/fuzz and hostile length/rate/decompression;
- loss, latency, jitter, duplication, reorder, disconnect/reconnect;
- prediction/reconciliation/rollback with side effects;
- interest transitions and world streaming;
- dedicated server no-presentation dependency;
- provider sandbox/outage/credential redaction;
- mod/package mismatch and authorized download;
- long soak, connection churn, server tick/CPU/memory/bandwidth.

Thresholds are genre/corpus/tier calibrated.

## 16. Delivery mapping

MS-09 delivers selected core transports/server/replication/reference samples. `WP-COL-001` may add independently selected provider-neutral Collective modules after NET, Wavefront, and security seams stabilize. MS-09 may add modded multiplayer and publishes mod policy. MS-10 certifies only declared transport/provider/server profiles. `PRG-WRL-001` distributed worlds is post-1.0 and cannot be inferred from NET completion.

## 17. Examples

End-to-end: client connects, authenticates, negotiates schema/package set, streams initial cells, receives entity map/snapshot, predicts local movement, reconciles server correction, and records one correlated trace.

Failure/recovery: a client sends oversized compressed input. The parser rejects before allocation, increments rate/security diagnostics, disconnects under policy, and server simulation remains unaffected.

Performance debug: server tick overruns. Trace separates gameplay, Cairn, interest, snapshot build, compression, and socket send by connection/cell; the operator adjusts schema frequency or relevance and reruns the same impairment workload.
