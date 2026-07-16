# Meridian Implementation Planning Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Delivery roadmap](DELIVERY_ROADMAP.md) · [Active plan](../PLANNING.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

version 0.5 · 2026-07-15 · Normative execution-planning authority

Documentation maturity: `ImplementationReady`. Implementation maturity:
`Implemented` for the planning/governance mechanism. Governing IDs:
`REQ-GOV-003`, `WP-GOV-002`, `WP-GOV-004`.

## 1. Purpose and authority

The delivery roadmap owns milestone order and user-visible outcomes. This
document owns how those outcomes become bounded implementation work. PLANNING
selects the currently active package; subsystem specifications own technical
contracts; registries own identifiers and typed status.

This specification closes the gap between a milestone such as “Creator Editor
Alpha” and an actionable code change. It does not invent dates, staffing, or
algorithm decisions unsupported by evidence.

## 2. Planning hierarchy

~~~text
MS-* evidence milestone
  -> workstream lane
     -> WP-* bounded work package
        -> repository-local tasks and commits
           -> EV-* evidence records
              -> REV-* integration or milestone review

PRG-* post-1.0 program
  -> future activation review
     -> future bounded WP-* packages

VAL-* validation project     DEP-* dependency strategy record
~~~

- A milestone is a user-visible evidence gate, not a sprint.
- A workstream is a long-lived ownership lane that may run in parallel.
- A work package is the smallest governed implementation result.
- Tasks are local implementation steps and do not receive permanent global IDs.
- An integration checkpoint proves multiple packages work together.
- A `PRG-*` record preserves post-1.0 intent without extending the 1.0
  milestone gate or claiming implementation readiness.
- A `VAL-*` record defines an independent proving project; definition does not
  count as implementation or execution evidence.
- A `DEP-*` record owns an external-foundation boundary and its replacement
  evidence instead of assuming either permanent dependency or mandatory rewrite.

## 3. Work-package contract

Every activated `WP-*` records:

- one user-visible or operator-visible result;
- owning role and owning crates/documents;
- requirements and milestone contribution;
- entry conditions and upstream package dependencies;
- explicit deliverables and public contracts changed;
- non-goals and forbidden adjacent work;
- failure, recovery, diagnostics, security, accessibility, and provenance impact;
- targeted tests plus required integration, benchmark, capture, or recovery evidence;
- stop conditions and rollback/abandonment rule;
- completion review and next unblocked packages.

A package should normally fit one reviewable implementation branch. If it
changes more than one primary architectural seam, requires unrelated subsystem
owners, or cannot be proven independently, split it before activation.

Near-term package dependencies are machine-readable through `depends_on` in
[`registry/work-packages.json`](registry/work-packages.json). The validator
rejects unknown dependencies, cycles, missing milestone plans, and critical
paths whose ordered package does not depend on its predecessor.

## 4. Definition of Ready

A package may move from `Planned` to `Active` only when:

1. its requirement and milestone contribution are registered;
2. dependencies are `Pass`, explicitly unsupported, or covered by a valid
   non-promoting waiver;
3. current code and evidence were inspected;
4. contract changes and compatibility consequences are named;
5. tests can fail before the implementation is accepted;
6. required hardware, corpus, tools, permissions, and private inputs are
   available or the package has a narrower honest outcome;
7. one owner can stop without leaving source authority corrupt or ambiguous;
8. PLANNING names it as the sole primary active package.

Research gates may run as sidecars, but research results cannot silently change
the active package contract.

Post-1.0 programs never become active packages directly. A future planning
review must select a bounded `WP-*`, establish entry evidence, and prove that
the program does not become a hidden dependency of a 1.0 milestone.

For `PRG-PRM-001`, that review occurs only after MS-10 and `RG-PRM-001`. No `WP-PRM-*` may be activated earlier, and Marquee documentation/governance work does not displace the active pre-1.0 package.

## 5. Definition of Done

A package is complete only when:

1. the declared result works through its public boundary;
2. targeted tests and proportional workspace gates pass;
3. required failure/recovery paths were induced and observed;
4. diagnostics identify source, operation, capability, and remediation;
5. accessibility, security, provenance, compatibility, and disabled-cost rows
   are complete or explicitly inapplicable;
6. evidence is fresh, scoped, and registered;
7. owning specifications, examples, ADRs, schemas, and PLANNING agree;
8. no adjacent planned feature is implied by the completion claim.

Construction, an API declaration, an occluded frame, or a benchmark definition
cannot complete a user-visible package.

## 6. Dependency and concurrency rules

Each milestone plan has four execution elements:

- **critical path:** the shortest dependency chain to the milestone result;
- **parallel lanes:** independent work that can proceed without shared mutable
  authority;
- **integration checkpoints:** explicit convergence points with combined
  evidence;
- **stop conditions:** failures that require redesign, research, or scope
  reduction before more implementation.

At most one primary package is active in a single working context. Separate
owners may work in parallel only with disjoint write authority and a named
integration checkpoint. Shared schemas, command registries, RHI contracts,
source formats, and public APIs require an owning package before dependents
merge against them.

## 7. Planning horizons

| Horizon | Required detail |
|---|---|
| Active | exact files/crates, entry evidence, tests, failure cases, commands, stop conditions |
| Next | stable package IDs, dependencies, deliverables, acceptance classes, likely integration point |
| Milestone-ready | critical path, parallel lanes, entry/exit gates, required package families |
| Research/deferred | owner, stable seam, research gate, corpus, decision rule; no fake implementation task list |

Detailed tasks are written only when their inputs are stable enough to satisfy
Definition of Ready. This prevents years of speculative pseudo-precision while
keeping every milestone executable when it approaches.

## 8. MS-01 executable implementation plan

MS-01 is the only milestone decomposed to near-term package precision in v0.5.

### 8.1 Critical path

~~~text
WP-PEN-007 pass timing and reliable timestamp outcomes
  -> WP-PEN-008 visible capture and surface outcomes
     -> WP-RUN-004 observable integration application
        -> WP-REL-002 MS-01 qualification review
~~~

### 8.2 Parallel runtime lane

`WP-RUN-002` proves platform/surface lifecycle, focus, resize, minimize,
occlusion, timeout, device/surface loss, cancellation, and actionable
diagnostics. `WP-RUN-003` correlates clocks, task queues, input, frame/pass,
streaming, and recovery events under shared operation/frame identifiers.

### 8.3 Parallel source-data lane

`WP-DAT-002` establishes a minimal source identity/import transaction.
`WP-DAT-003` loads and activates a minimal world cell through bounded streaming.
`WP-DAT-004` journals, recovers, packages, and reloads that source-derived
sample without treating caches or runtime handles as authority.

### 8.4 Integration checkpoint

`WP-RUN-004` opens one minimal application, accepts semantic input, advances
bounded clocks/tasks, loads the source-derived sample, presents through the RHI,
reports CPU/GPU/pass and surface outcomes, emits a visible capture where
supported, performs one save/recovery cycle, and exports a correlated evidence
bundle.

### 8.5 MS-01 stop conditions

Stop and redesign rather than continuing when timestamp results are silently
invalid, visible capture cannot distinguish surface outcomes, source/caches are
conflated, recovery mutates authority, diagnostics cannot correlate the
failure, or a required backend type leaks into a public engine/data API.

### 8.6 Closed qualification record

`WP-PEN-008`, `WP-RUN-002`, `WP-RUN-003`, `WP-RUN-004`, `WP-DAT-002`,
`WP-DAT-003`, and `WP-DAT-004` are closed as `ImplementedFoundation`.
`WP-REL-002` is `Implemented` and MS-01 is `Pass`: GitHub Actions run
`29452928922` passed governance and the macOS, Linux, and Windows workspace /
headless-smoke rows for `010db80`. The native surface was occluded or
unavailable, so the qualification preserves that surface outcome and uses a
separately labeled offscreen-visible PNG. `WP-UI-001` is
`ImplementedFoundation` and MS-02 is `Pass`: GitHub Actions run `29457181283`
passed governance plus Linux, Windows, and macOS UI-headless, UI-free runtime,
and minimal-runtime dependency rows for `fb8323f`. `WP-BLD-001` is the sole
active package once its Definition of Ready is recorded in `PLANNING.md`.

## 9. Milestone execution map

The machine-readable companion is
[`registry/delivery-plan.json`](registry/delivery-plan.json).

| Milestone | Entry | Critical implementation path | Principal parallel lanes | Exit integration |
|---|---|---|---|---|
| MS-00 | current repository | `WP-GOV-001` -> `WP-GOV-002` -> `WP-GOV-003` -> `WP-GOV-004` | ADRs, docs, private boundary, program/validation/dependency registries | locally passing v0.5 suite |
| MS-01 | MS-00 | `WP-PEN-007` -> `WP-PEN-008` -> `WP-RUN-004` -> `WP-REL-002` | runtime lifecycle; source/import/stream/save | minimal observable application |
| MS-02 | MS-01 contracts | `WP-UI-001` | accessibility adapters, fixtures, diagnostics | accessible native panel and overlay |
| MS-03 | MS-02 plus data/build seams | `WP-BLD-001` -> `WP-EDT-001` -> `WP-PRC-001` -> `WP-MDL-001` | import/browser, recovery, accessibility, Alluvium text/headless/basic inspector | Creator Editor Alpha plus native-modeler baseline |
| MS-04 | MS-01 instrumentation | `WP-PEN-003` -> `WP-PEN-010` | renderer foundations, shadows/IBL, Alluvium/Isobar/Basalt/vegetation | production-shaped Penumbra scene |
| MS-05 | MS-04 | `WP-PEN-010` -> `WP-PEN-011` | `WP-PRC-001` through `WP-PRC-004`, `WP-MDL-001`, terrain, vegetation, weather, streaming, quality tiers | measured representative forest and accepted native model sources |
| MS-06 | MS-03 and MS-05 | `WP-GAM-001` -> `WP-PRJ-001` | Cairn, Wavefront, Rust gameplay, save, accessibility | reproducible private Rust prototype |
| MS-07 | MS-06 plus private creative lock | `WP-PRJ-002` | platform, accessibility, audio, provenance | complete opening playable slice |
| MS-08 | MS-07 and native entry gate | `WP-RHI-002` -> `WP-REL-003` | `WP-BLD-002` advanced build service; modeler, animation, navigation, official-framework foundation, first-class 2D, shader language, editor/data maturity, selected simulation/DCC/VCS/agents | Engine Alpha with wgpu retained |
| MS-09 | mature Metal/common RHI | `WP-RHI-003` -> `WP-REL-004` | Collective baseline, optional Luau, advanced animation/navigation/shader lowering, networking, mods, XR, sync | declared Engine Beta profiles without a hosted-cloud promise |
| MS-10 | Beta profile freeze | `WP-SEC-001` -> `WP-REL-001` | documentation, migration, support, reproducibility | qualified 1.0 profiles |

## 10. Milestone review and replanning

At each integration checkpoint, compare actual dependencies, defects, measured
cost, and unsupported rows with the plan. Packages may be split, merged,
retired, or reordered through registry and roadmap updates. A milestone does not
absorb a failed package merely to preserve schedule appearance.

Later milestones receive active-package precision only as their entry gates
approach. Until then, their tables define architectural sequencing and evidence
obligations, not implementation estimates.

The complete official framework families, advanced modeler/cinematic tooling,
native VCS replacement, Collective ecosystem expansion, distributed worlds,
advanced integrity capabilities, Marquee exports, and competitive
performance/quality leadership remain visible through `PRG-*`. They do
not add milestones after `MS-10`, satisfy a 1.0 exit gate, or authorize runtime
work without a future bounded package.

`PRG-REL-001` is intentionally not decomposed into implementation tasks now.
After MS-10, its future Definition of Ready must freeze comparator access,
`VAL-REL-001`, exact claim classes, raw-evidence retention, legal/provenance
review, stable environmental and cost-manifest seams, and one bounded first
study. Until then only contract-preserving specification work is permitted.

## 11. Examples

Ready package: `WP-PEN-007` names the known Metal timestamp defect, supported
outcomes, affected RHI/diagnostic contracts, tests, native smoke, and stop rule.
It can be activated without deciding Forward+ clustering or specular IBL.

Not ready: “implement weather.” It lacks an Isobar field tier, corpus, update
clock, consumers, failure mode, disabled-cost proof, and research decision.

Replanning example: while a visible-capture package is active, surface recovery
cannot preserve frame identity. The package stops, a bounded RHI lifecycle
package is inserted, and its milestone remains open rather than accepting
misleading screenshots.
