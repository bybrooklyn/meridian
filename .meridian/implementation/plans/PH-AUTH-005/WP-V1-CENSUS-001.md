# WP-V1-CENSUS-001 — Crate and API disposition inventory

Revision 2. Revision 1 received `revise` on three blocking findings: the census is not a
projection and would have broken a green gate; it delivered one of four declared outputs while
claiming all four; and its test map had no mechanism.

## Ownership

- Owning phase: `PH-AUTH-005` — Requalify and classify the existing implementation
- Declared at `MERIDIAN_SPECOMENT.md` under `# Near-term work-package decomposition`
- Requirements: `SPEC-001`, `IMPL-WP-003`, Appendix D projection rules, §0.4 maturity axes
- Depends on: `PH-AUTH-004` — closed, merged at `e9a7ff6`
- Branch: `main`
- Primary semantic seam: **what every existing crate, API and test is, under v1 authority.**

## User-visible / operational result

Every workspace member, public API surface, test, example, source format and CI row carries
one honest v1 disposition and a next phase, replacing maturity inherited from retired v0.5
milestones.

## Current source diagnosis

Measured at `49bdd74`, each figure with its producing command.

- **37 workspace members** (`cargo metadata --no-deps`), across `engine/` and `editor/`.
- **772 test functions** (`grep -rh '#\[test\]' --include='*.rs' engine editor | wc -l`).
  An earlier figure of 716 was wrong: the summing command was piped through `tail -30`, so it
  totalled only the last thirty files. Both counting methods now agree at 772. This is the
  third time in this programme a figure has been wrong because a pipeline silently truncated
  its own input, which is why every figure here names its command.
- **96 internal dependency edges**: 84 mandatory, 12 optional, 0 dev-only.
- **18 workspace third-party dependencies**; 494 locked packages.
- **Four marker crates** at 205-210 bytes, one `pub const SCAFFOLD_STATUS` each, zero
  dependents: `meridian-audio`, `meridian-basalt`, `meridian-isobar`, `meridian-vegetation`.
- **`meridian-ui` is a façade, and recording it as "zero public items" would state the opposite
  of the truth.** It is 1,174 bytes and declares nothing, but it is five glob `pub use`
  re-exports plus 14 named types, and the four globbed crates carry **461** top-level `pub`
  items between them. The census therefore records `declared_public_items` and
  `reexported_public_items` as separate fields — a document whose declared test is "every
  public API has one owner and disposition" cannot report `0` for a crate exporting hundreds.
- **Seven files over 240 KB**, matching the `PH-AUTH-006` card's stated 248-540 KB range
  exactly: `ui_runtime/lib.rs` 540 KB down to `rhi/lib.rs` 247 KB.
- **Two genuine mandatory reverse edges**: `meridian-ecs → meridian-renderer` and
  `meridian-rt → meridian-renderer`. A first pass wrongly flagged twelve more; those are all
  optional and feature-gated, and the CI guard `cargo tree -p meridian-rt | grep meridian-ui`
  genuinely holds.

**"Exactly two reverse edges" is not verifiable until a layer order exists**, and neither the
plan nor the specoment declares one. The census therefore emits an explicit `layers` array —
the ordering it judges against — and derives `reverse: true|false` from it rather than
asserting a count. `meridian-rhi → meridian-render-graph` is then either classified or
explicitly exempted with a reason, instead of going unmentioned. `PH-AUTH-006`'s "dependency
graph obeys new layer rules" inherits that ordering.

**Root cause of the two edges under any plausible ordering, stated because the census must not
merely list them:** the
snapshot *contract* — `RenderSnapshot`, `RenderInstanceId`, `Transform`, `MeshHandle`,
`MaterialHandle`, `SnapshotError` — is defined in `meridian-renderer`. Any crate that
*produces* a snapshot must therefore depend on the renderer that *consumes* it. The inversion
is the contract's location, not the edge. Moving the edge without moving the contract would
relocate the inversion rather than remove it.

## Approach

1. **The census is class (c): derived-from-source. It does not live in `governance/generated/`
   and it is not in `emit::all()`.**

   `SD-011` split artefacts into (a) root projections and (b) accumulated state. A census is
   neither: derived, so not (b); derived from `cargo metadata` and the source tree rather than
   from the specoment, so not (a). Revision 1 put it in `emit::all()` and would have broken a
   gate that is green today:

   - **The staleness key would be inverted.** `run()` sets
     `source_checkpoint = "specoment:{canonical_sha256}"`. Change 37 crates and the census
     reports fresh; fix a typo in the specoment and it reports stale. That defeats Appendix D
     rule 7 in the precise direction that misleads.
   - **`project --check` would become machine-dependent.** `cargo metadata` emits absolute
     `manifest_path` values (`/Users/brooklyn/meridian/…`). Byte-identity could never pass in
     CI — and `mod.rs` already rejected exactly this when it refused to stamp HEAD: *"A check
     that can never pass trains people to ignore it."*
   - **It would contaminate `governance/manifest.json`**, which hashes every class (a) file.
   - **Appendix D rules 2 and 6 would be unsatisfiable.** `meridian-ui-runtime`,
     `meridian-vegetation` and `meridian-basalt` appear **zero** times in the specoment. §4
     says outright: *"Domain codes are governance IDs, not mandatory code names."*

   Location: `.meridian/implementation/census.json`, beside `state.json` and
   `evidence/index.json`. Schema at `governance/schemas/census.schema.json`. Policed by a
   conformance and reconciliation rule in `accumulated.rs`, exactly as class (b) is. Stamped
   with a **`source_tree_checkpoint`** as the staleness key, carrying `specoment_sha256` as a
   cross-reference the way Appendix H.4 does. Workspace-root prefixes stripped from every path.

   Recorded as `SD-013`.

2. **Multi-keyed, not crate-keyed.** The specoment's own `WP-V1-CENSUS-001` entry names a
   *"machine-readable crate/API/format/test inventory, generated dependency graph,
   retained-test map, provisional maturity registry"* and tests *"every workspace member/public
   API/source format/test has one owner and disposition."* Revision 1 delivered the crate axis
   only, and its ~900-line estimate — about 24 lines per crate — proved it.

   Sections: `crates`, `public_api`, `formats`, `targets`, `evidence_runners`,
   `generated_files`, `ci_rows`, `edges`, `layers`, `tests`. Each row carries a disposition and
   an owner.

3. **`format_migrations` is delivered, not dropped.** The phase card says *"identify direct
   forbidden edges and format migrations."* Revision 1 did the edge half, skipped the other,
   and its own User-visible result claimed both — a claim-versus-deliverable mismatch, not an
   honest deferral.

   This matters beyond bookkeeping: `PH-AUTH-006`'s stop condition is *"Stop if decomposition
   changes serialized output."* **That catch cannot fire unless this phase enumerates the
   serialized outputs.** Omitting formats disarms the next phase's safety mechanism.

   **18 versioned format constants** are present (`grep -rhoE '[A-Z_]*(SCHEMA_VERSION|
   FORMAT_VERSION|_VERSION|_MAGIC)' engine editor | sort -u`), including
   `UI_DOCUMENT_SCHEMA_VERSION`, `COMPILED_CELL_MAGIC`, `JOURNAL_MAGIC`, `VISUAL_FACET_MAGIC`,
   `COLLISION_FACET_MAGIC`, `RECIPE_SCHEMA_VERSION`, `BUILD_PROTOCOL_VERSION`. Every format not
   dispositioned `retain` names its migration or records `undecided` and escalates.
2. Assign one disposition per crate from the specoment's own vocabulary:
   `retain` / `refactor` / `replace` / `merge` / `split` / `remove`.
3. **All implementation maturity is recorded as `ExistingUnqualified`, and that term is not a
   §0.4 value — which the census must state rather than paper over.**

   The `PH-AUTH-005` card says *"Treat all current code as ExistingUnqualified after reset."*
   §0.4's implementation-maturity enum is `Implemented, ImplementedFoundation,
   StructuralSmoke, Partial, Transitional, Scaffold, Planned, Research, Deferred,
   Unsupported`. **`ExistingUnqualified` is not among them**; it appears in the specoment only
   twice, both times inside this phase card and its Appendix G serialization.

   Projecting it as if it were a §0.4 value would invent a status, which is precisely
   `PH-AUTH-003`'s stop condition. Projecting a §0.4 value instead would assert a maturity the
   card explicitly refuses to grant.

   Resolution: the census emits **two distinct fields**. `card_disposition` carries the
   card's verbatim `ExistingUnqualified`. `implementation_maturity` is emitted as **`null`**,
   with a note that no §0.4 value has been earned under v1 evidence. Nothing is invented and
   nothing is promoted. Recorded as `SD-012`, and whether §0.4 gains an `ExistingUnqualified`
   value or the card is reworded is an owner decision, `OD-012`, adjacent to `OD-009`.

   The disposition vocabulary itself **is** the specoment's own, verified at line 28722 and
   line 31389: *"classify every existing crate/system retain/refactor/replace/merge/split/
   remove"*. Assigning one per crate is mechanical where the evidence is mechanical — a
   200-byte crate with one public item and no dependents is `remove`; a 540 KB file the
   `PH-AUTH-006` card names for decomposition is `split`. Where it is not mechanical, the
   census emits `undecided` and escalates rather than guessing.
4. Generate a dependency graph as checked-in data, with mandatory and optional edges
   distinguished — the distinction a first pass got wrong.
5. **The test map is keyed at test granularity and targets requirement IDs, not phases.**

   Revision 1 offered one sentence, zero assertions, and a scalar `test count` per crate — from
   which no individual test maps to anything. 37 crate-level assignments presented as 772 is
   the bulk default that means nothing.

   The card says *"map useful tests to new **requirements**"*, and
   `governance/generated/requirements.json` holds **527** requirement IDs to validate against,
   which makes the requirement version mechanically checkable and the phase version not.

   Each of the **772** test functions gets a row keyed by `file`, `line`, `module` and `fn` —
   all three mechanically extractable. Each maps to an id **validated against
   `requirements.json`**, or records `undecided`. Assignment is at module granularity, not
   crate. **The `undecided` count is a reported figure**, so a bulk default is visible rather
   than hidden.
6. Record forbidden edges and their root cause, not just their existence.

## Explicit exclusions

- No feature work. No mass renames. No decomposition — `PH-AUTH-006` owns that.
- No promotion of any crate's maturity. The census records state; it does not improve it.
- No removal of the marker crates; that is `PH-AUTH-006`'s `WP-V1-ARCH-001`.

## Compatibility / migration / authority effects

Revision 1 said "None", which was wrong: relocating the census establishes a **third artefact
class**, and that is an authority effect worth naming. Class (a) is projected from the
specoment and policed by byte-identity; class (b) is accumulated state policed by conformance;
class (c) is derived from the source tree and policed by regeneration against a
`source_tree_checkpoint`. Recorded as `SD-013` so the next phase inherits the distinction
rather than rediscovering it.

No runtime effect. Nothing under `governance/generated/` changes, so `project --check` stays
green and machine-independent.

## Accessibility / security / privacy / provenance / disabled-cost effects

No runtime, UI or dependency surface. Provenance improves: crate disposition stops being
implicit in retired milestone records and becomes a stamped, regenerable artefact.

## Tests and evidence

- Every workspace member appears exactly once — count asserted against `cargo metadata`, so
  a crate added later fails the check rather than being silently absent.
- Every crate has a disposition drawn from the closed vocabulary; an unmapped crate is an error.
- Every crate has a next phase.
- Mandatory and optional edges are distinguished; a test asserts the twelve optional edges are
  not reported as violations, guarding the mistake the first pass made.
- The two genuine reverse edges are reported with their root cause.
- Every row carries `implementation_maturity: null` and `card_disposition: ExistingUnqualified`.
  Revision 1 still asserted "no crate carries an implementation maturity other than
  `ExistingUnqualified`" after its own step 3 made that field `null` — the `SD-012` fix had not
  propagated.
- Every test row resolves to an id present in `requirements.json`, or is explicitly
  `undecided`, and the `undecided` count is reported.
- Every format, target, evidence runner, generated file and CI row carries a disposition.

Gates: `cargo test --workspace`, `fmt`, `clippy --workspace -D warnings`, `check`,
`project --check`, `metadata --locked`, `git diff --check`.

## Failure injection and recovery

Add a synthetic crate with no disposition and confirm the check fails naming it. Mark a crate
`Implemented` and confirm the maturity rule rejects it. Present an optional edge as mandatory
and confirm the distinction is asserted rather than assumed.

## Research candidates and selection metrics

None.

## LOC estimate

| Area | Added | Removed |
|---|---|---|
| `census` module plus conformance rule in `accumulated.rs` | ~450 | 0 |
| Schema, hand-written, shape-only | ~120 | 0 |
| Tests | ~200 | 0 |
| Generated `census.json` — ten sections, 772 test rows | ~4,000 | 0 |

Revision 1 estimated ~1,350 with an `Added` column only. `IMPL-WP-001` requires added **and**
removed; removed is zero here and is now stated.

## Stop / rollback rule

Stop if a crate is retained solely because it exists; if a test has no behavioural contract or
next phase; if any implementation maturity is promoted without new v1 evidence; or if the
census asserts a disposition the specoment's vocabulary does not contain. Rollback is one
commit; nothing outside `governance/` changes.

## Independent Review

_Pending._

## Completion record

_Pending._
