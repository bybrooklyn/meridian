# Distributed Worlds and MMO Architecture Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Networking](MULTIPLAYER_AND_SERVER_SPEC.md) · [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) · [World data](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) · [Post-1.0 programs](DELIVERY_ROADMAP.md)

Status: version 0.5 post-1.0 research authority, 2026-07-15.

Documentation maturity: `ResearchReady`. Implementation maturity: `Deferred`.
Governing IDs: `REQ-WRL-001` through `REQ-WRL-003`; `PRG-WRL-001`.

Current implementation status: no distributed-world coordinator, shard/region service, cross-server migration, persistent MMO world, interest federation, or globally operated service exists. This program is not required for MS-00 through MS-10.

## 1. Authority and Scope

The `WRL` domain owns future contracts for partitioning one logical live world across authoritative simulation processes, directory and placement, cross-partition entity handoff, replicated persistent-world transactions, failure containment, capacity management, and distributed-world observability. NET owns transport/replication primitives. Collective owns identity, sessions, social, and service adapters. DAT owns source world/cell/package schemas. Gameplay owns game rules. Projects own operational deployment and economy/community policy.

Goals are evidence-driven scale, explicit consistency, recoverable migration, regional placement, bounded failure domains, deterministic replay where promised, and provider-neutral deployment. Non-goals are an MMO checkbox, infinite scale, transparent distributed transactions, free global hosting, or hiding distributed-system tradeoffs from project authors.

## 2. Planned Contracts

```text
WorldPartitionId { world, region, cell, generation }
PartitionLease { owner, epoch, bounds, capabilities, expires }
EntityAuthorityRecord { entity, partition, epoch, revision }
MigrationTransaction { id, entity_set, source, destination, snapshot, phase }
WorldDirectorySnapshot { revision, partitions, routes, capacity, health }
PersistentWorldTransaction { id, subjects, preconditions, changes, consistency }
DistributedTrace { trace_id, epochs, hops, queues, decisions, failures }
```

Every authority transition is epoch-checked and idempotent. Cross-partition semantics declare consistency, ordering, latency, retry, rollback/compensation, and loss behavior; no API implies impossible global atomicity.

## 3. State Machines and Failure Model

```text
discover partition and authority epoch
-> reserve destination capacity
-> quiesce or snapshot migratable state at barrier
-> transfer authenticated bounded snapshot
-> validate destination dependencies and schema
-> activate destination epoch
-> redirect clients/services
-> retire source authority after acknowledgement window
```

Partitions move among provisioning, warming, active, draining, unavailable, recovering, and retired. Split brain, stale epochs, duplicate delivery, partial migration, network partition, clock skew, storage lag, provider outage, overload, and malicious clients are first-class test conditions.

## 4. Time, Data, Security, and Operations

Simulation uses explicit tick/epoch authority; wall clocks are for deadlines and observability, not conflict correctness. Persistent changes use typed transactions, idempotency keys, event/audit records, backup/restore, schema migrations, and project-selected consistency. Private state is minimized and encrypted.

Security requires mutual service identity, least privilege, partition admission, signed deployment artifacts, secret rotation, DDoS/abuse controls, authoritative validation, regional/legal policy, audit, incident response, and restore exercises. No hosted deployment is production-ready without funded operations, on-call, capacity, backups, legal review, and red-team evidence.

Diagnostics correlate client, NET, Collective, partition, storage, and build traces while redacting personal/private data. Required metrics include tick distributions, migration latency, queues, replication lag, storage lag, retries, handoff failures, capacity, memory, egress, and cost.

## 5. Requirements and Program Gates

- `REQ-WRL-001`: explicit authority epochs, partition/migration contracts, and failure semantics with differential simulator evidence.
- `REQ-WRL-002`: provider-neutral persistence, deployment, recovery, observability, security, and cost contracts with regional failure exercises.
- `REQ-WRL-003`: gameplay/network/Collective/world-data boundaries that prevent authority duplication and preserve offline/single-server operation.
- `PRG-WRL-001`: post-1.0 distributed-world research and implementation; it cannot satisfy or block MS-00 through MS-10.

Program entry requires mature NET, Collective, save/world transactions, dedicated server, security/update, production telemetry, and a funded operational model. Tests use deterministic cluster simulation, fault injection, stale epochs, split brain, retries, migration, rolling upgrades, backup/restore, load/soak, cost reports, and adversarial clients. Success at small scale cannot support MMO-scale claims.

## 6. Accessibility and Zero-Cost Behavior

Distributed-world user flows must preserve accessibility settings, communication controls, language/region choices, reconnect clarity, and recoverable progress. Administration tools require keyboard and semantic access.

Projects not selecting WRL contain no coordinator, directory, migration, distributed storage, telemetry, or service deployment artifacts. A normal offline, peer, or single-server project remains the baseline.

## 7. Examples

End to end: a funded project migrates a player group between authoritative regions using an epoch-checked transaction while clients receive a bounded handoff.

Failure: destination activation succeeds but source acknowledgement is lost. Epoch authority prevents dual simulation; idempotent retry finishes retirement.

Performance debug: a region-transition spike decomposes source quiescence, snapshot serialization, transfer, destination validation, activation, and client redirect.
