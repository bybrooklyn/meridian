# Meridian v0.5 General-Purpose Platform Amendment Migration Ledger

Version 0.5 · 2026-07-15 · Active migration record

This ledger maps every numbered review area and every resolved user decision from the general-purpose platform amendment. Dispositions are `Preserved`, `Strengthened`, `Split`, `NewAuthority`, `Superseded`, `DeferredProgram`, or `Rejected`.

## 1. Amendment Review Areas

| Area | Disposition | v0.5 authority |
|---|---|---|
| 1. Engine layering and boundaries | Strengthened | master, repository architecture, ADR-0018 |
| 2. Runtime world model | Strengthened | data/world specification and editor document contracts |
| 3. Scheduling, concurrency, and time | Preserved | core runtime specification |
| 4. Stable identity and persistent data | Strengthened | data/world and runtime identity tables |
| 5. Serialization, migration, compatibility | Preserved | data/world/save/package specification |
| 6. Gameplay programming architecture | Superseded | Rust-first gameplay specification, ADR-0019 |
| 7. Official gameplay frameworks | NewAuthority | gameplay frameworks specification, `FWK` |
| 8. Character movement and cameras | Split | framework and Cairn specifications |
| 9. Combat and shooter foundations | NewAuthority | optional framework packages |
| 10. Dedicated 2D architecture | NewAuthority | first-class 2D specification, ADR-0021 |
| 11. Rendering extensibility | Strengthened | Penumbra custom-path/pass contracts |
| 12. Meridian Shader Language | NewAuthority | shader-language specification, ADR-0023 |
| 13. Physics abstraction | Preserved | Cairn specification, with 2D seam strengthened |
| 14. Animation and cinematic characters | NewAuthority | animation/cinematics specification |
| 15. Runtime UI and text | Preserved | Meridian UI/editor specification; detailed redesign remains separate |
| 16. Audio and voice | Split | Wavefront plus Collective, ADR-0020 |
| 17. Input and local multiplayer | Strengthened | core runtime and framework specifications |
| 18. World authoring and semantic regions | Strengthened | data/world, Alluvium, Isobar, Basalt, navigation |
| 19. Environment and material semantics | Strengthened | data/facet authority plus owning runtime subsystems |
| 20. Asset pipeline and Blender tooling | Split | data/build, native modeler, optional DCC companion |
| 21. Complete programming IDE | Strengthened | one Meridian application; build/IDE authority |
| 22. Meridian-native VCS | Preserved | VCS/sync authority and post-1.0 native-storage program |
| 23. Networking foundations | Preserved | multiplayer/server specification |
| 24. Matchmaking and sessions | Split | NET core and Collective optional services |
| 25. Social platform | NewAuthority | Collective optional modules |
| 26. Privacy-conscious analytics | NewAuthority | Collective optional modules; diagnostics remain separate |
| 27. MMO/distributed world | DeferredProgram | distributed-world specification, `WRL` |
| 28. Anti-cheat/integrity | DeferredProgram | integrity specification, `INT` |
| 29. Modding and UGC | Preserved | modding/community-library specification |
| 30. Dependency internalization | Strengthened | dependency strategy registry and research policy |
| 31. Editor automation and agents | Preserved | typed command/agent specification |
| 32. Remote development | Strengthened | build/IDE and sync specifications |
| 33. Diagnostics and reproducibility | Preserved | runtime, validation, and owning subsystem diagnostics |
| 34. Accessibility and localization | Preserved | accessibility and Meridian UI specifications |
| 35. Security and trust | Strengthened | security plus new Collective/Worlds/integrity threat boundaries |
| 36. Build, package, patch, release | Preserved | build and data/package specifications |
| 37. Public/private separation | Preserved | repository split and automated audits |
| 38. Validation projects | NewAuthority | validation-project registry and validation specification |
| 39. Performance/scalability budgets | Strengthened | calibrated workload budgets; invented fixed numbers rejected |
| 40. Documentation and ADRs | Preserved | v0.5 governance and this ledger |

Mapped review areas: 40. Unmapped review areas: 0.

## 2. Resolved Decisions

| Decision | Disposition | Authority |
|---|---|---|
| Documentation-only v0.5 amendment | Preserved | `WP-GOV-004`; no runtime package activation |
| Federated suite, not one monolith | Rejected monolith | master authority order and ADR-0018 |
| Keep MS-00 through MS-10 | Preserved | delivery roadmap and ADR-0024 |
| One application named Meridian | Preserved | ADR-0018 |
| Helper processes and CLI allowed | Preserved | runtime/build/repository specs |
| Wavefront audio name | NewAuthority | ADR-0020 and Wavefront spec |
| Collective unified online name | NewAuthority | ADR-0020 and Collective spec |
| No public hosted-cloud promise without funding | Preserved | Collective non-goals/program gate |
| Rust gameplay first; Luau afterward | Superseded sequencing | ADR-0019 and gameplay spec |
| First-class 2D baseline before 1.0 | NewAuthority | ADR-0021 |
| Six framework families planned, not 1.0 blockers | DeferredProgram | framework spec and ADR-0024 |
| Native beginner-friendly modeler is core | NewAuthority | ADR-0022 and modeler spec |
| Blender optional, never required | Superseded DCC assumption | modeler and DCC contracts |
| General animation before cinematic facial suite | Preserved | animation spec and program registry |
| Text shader language and material graphs share IR | NewAuthority | ADR-0023 |
| Avoid `MSL` abbreviation | Preserved | shader-language naming rule |
| Dependency replacement remains evidence-gated | Preserved | dependency strategy registry |
| Meridian VCS remains important with Git/Jujutsu escape | Preserved | VCS spec and post-1.0 program |
| `WP-PEN-008` remains immediate next package | Preserved | PLANNING; amendment activates no feature package |
| Meridian UI redesign discussed separately | Deferred | current UI authority remains unchanged except links/boundaries |

Mapped decisions: 20. Unmapped decisions: 0.

## 3. Stale Prompt Terms

| Term | Disposition | Replacement |
|---|---|---|
| One complete canonical document | Rejected | governed federated suite |
| New Foundation/Initial/Studio phases | Rejected | MS milestones, parallel WPs, post-1.0 PRGs |
| Meridian Studio as separate app | Rejected | one application named Meridian |
| Meridian Online | Superseded | Collective |
| Wavefront as undefined audio reference | Superseded | adopted Wavefront authority |
| `MSL` acronym | Rejected | spell out working shader-language name |

Mapped stale terms: 6. Unmapped stale terms: 0.

## 4. Validation Contract

`meridian-spec list-unmapped` must report zero for v0.3, v0.4, and v0.5 ledgers. New domains have one maturity record and typed requirements. Post-1.0 programs cannot satisfy MS-00 through MS-10 or promote implementation maturity. All new validation projects begin `DefinitionOnly` / `Uncalibrated`.

Total mapped rows: 66. Total unmapped rows: 0.
