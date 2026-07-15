# Meridian v0.3 Roadmap and Subsystem Migration Ledger

Version 0.3 · 2026-07-15 · Historical record

This file is the only active repository location, besides older immutable migration history, where the retired numeric phase and renderer subpackage tokens are normative lookup keys. Active delivery uses `MS-*`, `WP-*`, and `RG-*` identifiers.

Disposition vocabulary: `Preserved`, `Split`, `Merged`, `Superseded`, or `Retired`.

## 1. Roadmap migration

| Legacy key | Disposition | v0.3 destination |
|---|---|---|
| P0 | Superseded | `MS-00`, GOV workstream |
| P1 | Split | `MS-01`, RUN workstream |
| P2 | Split | `MS-01`, `MS-04`, `MS-05`; RHI and PEN workstreams |
| P3 | Split | `MS-01`, `MS-04`, `MS-06`; PHY workstream |
| P4 | Superseded | `MS-03`, EDT workstream |
| P5 | Split | `MS-01`, `MS-03`, `MS-04`; DAT workstream |
| P6 | Split | `MS-02`, `MS-03`; UI workstream |
| P7 | Split | `MS-04`, `MS-06`; GAM workstream |
| P8 | Split | `MS-06` prototype and `MS-07` production opening slice; PRJ workstream |
| P9 | Split | `MS-03`, `MS-08`; UI and EDT workstreams |
| P10 | Split | `MS-04`, `MS-06`, `MS-08`; AUD workstream |
| P11 | Split | `MS-04`, `MS-05`, `MS-08`; ISO, BAS, VEG, PRC workstreams |
| P12 | Split | `RG-PEN-001` after `MS-05`; native work in `MS-08`/`MS-09` |
| P13 | Preserved | `MS-08`, PHY destruction packages |
| P14 | Preserved | `MS-08`, PRC packages |
| P15 | Preserved | `MS-08`, DCC packages |
| P16 | Split | `MS-03`, `MS-08`; BLD packages |
| P17 | Preserved | `MS-08`, VCS packages |
| P18 | Split | `MS-08`, `MS-09`; SYN packages |
| P19 | Preserved | `MS-09`, XR packages |
| P20 | Preserved | `MS-08`, AUD acoustics packages |
| P21 | Split | `MS-08`; TOR, PHY, VEG research/packages |
| P22 | Preserved | `MS-09`, NET packages |
| P23 | Preserved | `MS-09`, NET provider packages |
| P24 | Preserved | `MS-09`, MOD packages |
| P25 | Split | `MS-08`, `MS-09`; AGT packages |
| P26 | Split | `MS-08`; PRC, TOR, VEG packages |
| P27 | Preserved | `MS-08`, TOR packages/research |
| P28 | Preserved | `MS-09`, GAM language research |
| P29 | Superseded | `MS-10`, REL workstream |

Mapped rows: 30. Unmapped rows: 0.

## 2. Renderer-package migration

| Legacy key | Disposition | v0.3 destination |
|---|---|---|
| P2.1 | Preserved | `WP-RHI-001` |
| P2.2 | Preserved | `WP-PEN-001` |
| P2.3 | Preserved | `WP-PEN-002` |
| P2.4 | Preserved | `WP-PEN-003` |
| P2.5 | Split | `WP-PEN-002`, `WP-PEN-004` |
| P2.6 | Preserved | `WP-PEN-005` |
| P2.7 | Preserved | `WP-PEN-006`, `ImplementedFoundation` |
| P2.8 | Preserved | `WP-PEN-007`, next runtime package |
| P2.9 | Preserved | `WP-PEN-008` |
| P2.10 in the retired roadmap | Merged | `WP-PEN-010` clustered Forward+ |
| P2.10 in the retired active queue | Merged | `WP-PEN-011` executable corpus |
| P2.11 in the retired roadmap | Merged | `WP-PEN-011` executable corpus |
| P2.11 in the retired active queue | Preserved | `WP-PEN-009` specular IBL |
| P2.12 | Merged | `WP-PEN-010` clustered Forward+ |

Mapped rows: 14. Unmapped rows: 0.

## 3. Combined weather/environment/simulation heading migration

| Retired heading | Disposition | v0.3 destination |
|---|---|---|
| Context | Split | Isobar, Basalt, Torsant specifications |
| Goals and Non-Goals | Split | all three owning specifications |
| Ownership and Crate Boundaries | Split | all three specs; vegetation remains separate consumer |
| Public Types and Data Structures | Split | atmosphere/field contracts to Isobar; terrain/geometry to Basalt; thermal/fire/fluid to Torsant |
| Runtime Pipeline | Split | each owning pipeline plus cross-system immutable snapshots/events |
| Threading, Memory, and Lifetime | Split | each owning specification |
| Persistence, Versioning, and Compatibility | Split | each owning specification |
| Editor, CLI, MCP, and Workflows | Split | each owning specification |
| Diagnostics, Failure Recovery, and Security | Split | each owning specification |
| Capability Tiers and Zero-Cost-Disabled Behavior | Split | each owning specification |
| Algorithm Alternatives and Research Gates | Split | `RG-ISO-*`, `RG-BAS-*`, and `RG-TOR-*` |
| Tests, Benchmarks, and Acceptance Evidence | Split | owning specs and `PEN-B05` through `PEN-B07` |
| Phased Implementation | Superseded | milestone/workstream mapping in `DELIVERY_ROADMAP.md` |
| Examples | Split | all three owning specifications |

Mapped rows: 14. Unmapped rows: 0.

## 4. File and authority migration

| Retired authority | Disposition | v0.3 authority |
|---|---|---|
| `specs/IMPLEMENTATION_PHASES.md` | Superseded | `specs/DELIVERY_ROADMAP.md` |
| `specs/WEATHER_ENVIRONMENT_AND_SIMULATION_SPEC.md` | Split | Isobar, Basalt, and Torsant specifications |
| `docs/adr/` | Superseded | `docs/architecture/decisions/` |
| B01/B02 short benchmark IDs | Superseded | `PEN-B01` and `PEN-B02` |
| engine weather scaffold | Superseded | `meridian-isobar` scaffold |
| engine terrain scaffold | Superseded | `meridian-basalt` scaffold |

Mapped rows: 6. Unmapped rows: 0.

## 5. Validation contract

`meridian-spec list-unmapped` MUST report zero. Active Markdown outside `docs/migrations/` MUST contain no retired phase tokens or deleted authority links. The validator also confirms the combined heading ledger, workload suite, ADR index, requirements, packages, and maturity registry are complete.
