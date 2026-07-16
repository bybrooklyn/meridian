# Meridian v0.5 Marquee Amendment Migration Ledger

Version 0.5 · 2026-07-15 · Active in-version amendment record

This ledger maps every Marquee amendment decision and every existing promotional-material authority. Dispositions are `Preserved`, `Strengthened`, `Split`, `NewAuthority`, `DeferredProgram`, or `Rejected`.

## 1. Amendment Decisions

| Decision | Disposition | v0.5 authority |
|---|---|---|
| Dedicated name Marquee and domain PRM | NewAuthority | canonical Marquee specification and ADR-0025 |
| Documentation remains v0.5 | Preserved | master and this in-version ledger |
| Entire implementation after 1.0 | DeferredProgram | `PRG-PRM-001`; no pre-1.0 package |
| Reserve `meridian-marquee` without a crate | Preserved | repository architecture |
| One Meridian application | Preserved | ADR-0018; Marquee is a future workspace, not another product |
| Manual screenshots and clips only | NewAuthority | Marquee capture boundary |
| Marquee never launches or navigates a game | Rejected automation | Marquee non-goals and policy registry |
| Deterministic post-capture transforms | NewAuthority | Marquee source and recipe contracts |
| Screenshots, art, icons, clips, trailers, copy, captions, alt text | NewAuthority | Marquee output contract |
| PDF press kits and review/proof books | NewAuthority | Marquee output contract |
| Steam/itch.io/YouTube-ready and portable bundles | NewAuthority | export profiles; files only |
| No login, upload, scheduling, publishing, advertising, or account ownership | Rejected integration | ADR-0025 and policy registry |
| No website generation | Rejected scope | Marquee non-goals |
| Optional AI text and analysis | NewAuthority | AGT-composed policy |
| No AI image, video, voice, music, or sound generation | Rejected capability | Marquee AI policy and validator |
| No AI media alteration | Rejected capability | Marquee AI policy and validator |
| AI suggestions require human approval | Strengthened | approval and AI-assist records |
| Local and approved cloud AI | Preserved with limits | AGT permissions and SEC disclosure |
| AI-disabled zero-cost behavior | Strengthened | Marquee capability policy |
| Draft and ReleaseReady states | NewAuthority | Marquee artifact state machine |
| Human ReleaseReady approval | NewAuthority | approval contract and validator |
| Changed inputs invalidate approval | NewAuthority | dependency/invalidation contract |
| Adapter-first media/PDF tools | NewAuthority | `RG-PRM-001` |
| No custom codec presumption | Rejected assumption | `RG-PRM-001` decision rule |
| Private Project Meridian campaign data | Preserved | private game production authority |
| Synthetic public validation corpus | NewAuthority | `VAL-PRM-001` |
| No promotional-quality evidence from docs | Preserved truth boundary | maturity/evidence registries |

Mapped amendment decisions: 26. Unmapped amendment decisions: 0.

## 2. Existing Promotional Material Headings

| Existing authority or heading | Disposition | Destination |
|---|---|---|
| Private `PRODUCTION_AND_ASSETS.md` Marketing and storefront truthfulness | Split | game owns policy/content; Marquee owns future tooling |
| Private `PRODUCTION_AND_ASSETS.md` Publishing and commercial control | Preserved | remains manual/external project authority |
| Private `PRODUCTION_AND_ASSETS.md` Release channels and records | Strengthened | Marquee exports evidence-bearing bundles; publishing remains external |
| Private release screenshots and trailer gates | Strengthened | private acceptance plus future Marquee manifests |
| Penumbra editor captures | Split | Penumbra captures manually requested frames; Marquee imports them |
| Penumbra GPU capture security | Strengthened | DAT/SEC provenance and private export policy |
| ANI cinematic sequencing | Preserved | ANI owns in-engine sequence; Marquee owns post-capture edit timeline |
| Wavefront audio capture/processing | Preserved | Wavefront owns audio behavior; Marquee consumes approved output |
| BLD artifact jobs | Strengthened | future Marquee jobs use BLD-owned workers and artifacts |
| DAT provenance and package source identity | Preserved | DAT remains source/right/hash authority |
| REL product claims | Strengthened | Marquee claim records bind to qualified BuildIds/evidence |
| AGT AI capability policy | Strengthened | text/analysis-only Marquee profile |
| SEC external-tool and privacy policy | Strengthened | imported media, cloud disclosure, and isolated workers |
| README and suite indexes | NewAuthority | link to canonical Marquee spec and status |

Mapped existing headings: 14. Unmapped existing headings: 0.

## 3. Governance Records

| Record set | Mapping |
|---|---|
| Requirements | `REQ-PRM-001` through `REQ-PRM-006` |
| Program | `PRG-PRM-001`, Deferred, Post1.0 |
| Research | `RG-PRM-001`, opens after MS-10 |
| Validation | `VAL-PRM-001`, DefinitionOnly/Uncalibrated |
| Risks | `RISK-PRM-001` through `RISK-PRM-010` |
| Governance closure | `WP-GOV-005`, `EV-GOV-20260715-005`, `REV-GOV-20260715-003` |
| Decision | ADR-0025 |
| Maturity | PRM ResearchReady / Deferred |

Mapped governance rows: 8. Unmapped governance rows: 0.

## 4. Validation Contract

`meridian-spec list-unmapped` requires this ledger to report zero unmapped rows. The validator enforces one PRM maturity record, a registered post-1.0 program, no milestone leakage, no premature PRM work package, manual capture, export-only behavior, explicit human approval, complete source classification, complete ReleaseReady approval invalidation including fonts and transform recipes, and text/analysis-only AI.

Total mapped rows: 48. Total unmapped rows: 0.
