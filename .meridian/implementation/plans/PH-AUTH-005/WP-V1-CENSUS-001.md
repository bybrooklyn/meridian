# WP-V1-CENSUS-001 — Repository census (measurement)

Revision 3. **Scope split**: this package delivers the census as *measurement*; a successor,
`WP-V1-CENSUS-002`, assigns dispositions and test owners against its frozen output. The split
is on measurement-versus-judgement, not on sections — nine of ten sections are mechanically
derivable and cost generator code, while ~850 dispositions and test mappings cost reviewer
attention, and burying those inside 4,000 lines of measurement is what makes a package
approved-on-trust rather than reviewed.

## Ownership

- Owning phase: `PH-AUTH-005` — Requalify and classify the existing implementation
- Declared at `MERIDIAN_SPECOMENT.md` under `# Near-term work-package decomposition`
- Requirements: `SPEC-001`, `IMPL-WP-003`, Appendix D projection rules, §0.4 maturity axes
- Depends on: `PH-AUTH-004` — closed, merged at `e9a7ff6`
- Branch: `main`
- Primary semantic seam: **what the repository measurably contains.** Judgement about what
  each thing should become is `WP-V1-CENSUS-002`.

## User-visible / operational result

Every workspace member, public API surface, test, example, source format, evidence runner,
generated file, CI row and dependency edge is enumerated exactly once in a stamped,
regenerable record, with `disposition` and `owner` present but unassigned.

That is deliberately less than the phase needs. `WP-V1-CENSUS-002` assigns the dispositions and
test owners against this frozen measurement, and `PH-AUTH-005` closes on the two together.
Splitting this way means the second review examines ~850 judgements against an
already-verified base, rather than hunting them inside 4,000 lines of measurement.

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
  re-exports plus 14 named types. The census records `declared_public_items` and
  `reexported_public_items` as separate fields, because a document whose declared test is
  "every public API has one owner and disposition" cannot report `0` for a crate exporting
  hundreds.

  **Both fields are defined in the schema, because the number depends entirely on the
  definition and revision 2 quoted one it had not stated.** Across the four globbed crates:
  `^pub ` at root = **213**; `pub` item keywords at any indentation = **461**; any `pub` token
  = 961. Revision 2's 461 came from the any-indentation form, which counts `pub fn` methods
  inside `impl` blocks and items inside *private* modules — **neither of which a glob
  `pub use ...::*` re-exports**, so it inflates a façade's count with things the façade does
  not export.

  Definition adopted: a glob re-export forwards **root-namespace public items**, so
  `reexported_public_items` counts items declared at module root in the globbed crate's public
  module tree — the `^pub ` form, **213**. Stated in the schema rather than implied by a
  command. A resolution-based count is the honest long-term answer and is recorded as a
  limitation, not silently approximated.
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

   **The enumeration criterion was wrong and is corrected.** Revision 2 used
   `grep -rhoE '[A-Z_]*(SCHEMA_VERSION|FORMAT_VERSION|_VERSION|_MAGIC)'` and reported 18. Both
   the `_MAGIC` and `_VERSION` alternatives require a leading underscore, so a **bare** `MAGIC`
   cannot match — and two exist:

   - `engine/meridian_package/src/lib.rs:15` — `const MAGIC: &[u8; 8] = b"MERIDN\0\0"`
   - `engine/meridian_save/src/lib.rs:18` — `const MAGIC: &[u8; 4] = b"MSAV"`

   The package container and the save file: **the two serialized outputs whose corruption would
   be least recoverable, and precisely the ones `PH-AUTH-006`'s stop condition exists to
   protect.** The measurement built to arm that catch was blind to exactly them.

   Three of the 18 are also not formats: `CARGO_PKG_VERSION` (a Cargo env var),
   `ENGINE_VERSION` (literally `env!("CARGO_PKG_VERSION")`), and `GENERATOR_VERSION` (the
   projection generator's own version string).

   **This is the third time in this programme a figure was made defensible by a criterion that
   quietly excluded the hard cases** — after a `tail -30` truncating a sum, and a `docs/` grep
   narrowed until the number agreed with the claim. Stamping a command beside a figure only
   helps if someone runs that command against the thing it claims to measure. Stamping is not
   validation.

   Corrected method: a grep is a **discovery tool, not a completeness proof**. The enumeration
   is cross-checked against a hand-listed set of known on-disk formats before the count is
   trusted, and a row is a *format* — a magic-plus-version pair is one row, not two. Formats
   with no version constant are included by source: `schemas/benchmark-result.schema.json`, and
   any serde type that reaches disk.
4. Assign one disposition per crate from the specoment's own vocabulary:
   `retain` / `refactor` / `replace` / `merge` / `split` / `remove`.
5. **All implementation maturity is recorded as `ExistingUnqualified`, and that term is not a
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
6. Generate a dependency graph as checked-in data, with mandatory and optional edges
   distinguished — the distinction a first pass got wrong.
7. **The test map is keyed at test granularity and targets requirement IDs, not phases.**

   Revision 1 offered one sentence, zero assertions, and a scalar `test count` per crate — from
   which no individual test maps to anything. 37 crate-level assignments presented as 772 is
   the bulk default that means nothing.

   The card says *"map useful tests to new **requirements**"*, and
   `governance/generated/requirements.json` holds **527** requirement IDs to validate against,
   which makes the requirement version mechanically checkable and the phase version not.

   Each of the **772** test functions gets a row keyed by `file`, `line`, `module` and `fn`.
   **Assignment is `WP-V1-CENSUS-002`'s work**; this package emits the rows with `owner` null
   and asserts every test is present exactly once.

### `undecided` is withdrawn: it is outside the authoritative vocabulary

Revision 2 used `undecided` as an escape valve for dispositions, formats and test mappings.
**`undecided` occurs zero times in `MERIDIAN_SPECOMENT.md`.** The vocabulary is
`retain/refactor/replace/merge/split/remove`, and revision 2 simultaneously asserted "every
crate has a disposition drawn from the closed vocabulary" and stopped if "the census asserts a
disposition the specoment's vocabulary does not contain." Those cannot both hold with
`undecided` admitted. **This is `SD-012`'s exact shape recurring a third time** — a term
outside the authoritative vocabulary used as though it were inside it.

The fix is the one already built for `SD-012`. Two fields:

- `disposition` — nullable, drawn **only** from the closed vocabulary;
- `escalation` — null, or an `OD-*` identifier.

A row is valid iff **exactly one is non-null**. `undecided` then cannot masquerade as a
disposition, and — the part that makes this principled rather than a valve — **every escalation
must name an owner-decision record**, so the escalation count equals the count of open `OD-*`
entries in `state.json` and cannot be inflated silently.

**An escalation budget, with overshoot as a stop condition.** Reporting a count is not a
control; a number with no threshold is decoration. Most escalations are predictable now: four
marker crates with zero dependents are `remove`; seven named oversized files are `split`; the
12 optional and 2 mandatory reverse edges are classified; generated-file and CI-row sections
are fully determined. `WP-V1-CENSUS-002` states its expected escalation count in its Definition
of Done, and a large overshoot is the signal that the judgement work was skipped rather than
done.
8. Record forbidden edges and their root cause, not just their existence.

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

Completeness and reconciliation only; no assertion depends on a judgement, because this
package makes none.

- Every workspace member appears exactly once, count asserted against `cargo metadata`, so a
  crate added later fails rather than being silently absent.
- Every one of the **772** test functions appears exactly once, keyed by file, line, module and
  function.
- Every row in all ten sections carries `disposition: null` and `escalation: null` — this
  package measures, and a non-null value here means judgement leaked in.
- Every row carries `implementation_maturity: null` and `card_disposition:
  "ExistingUnqualified"`. Revision 1 still asserted "no crate carries an implementation
  maturity other than `ExistingUnqualified`" after its own step made that field null; the
  `SD-012` fix had not propagated.
- Mandatory and optional edges are distinguished, with a test asserting the 12 optional edges
  are not reported as violations — the mistake a first pass made.
- `reverse` is derived from the declared `layers` array, never asserted as a count.
- The format enumeration matches a hand-listed known-format set, **including `MSAV` and
  `MERIDN\0\0`**, which the previous criterion could not see.
- Regeneration is byte-identical, and every count matches the command stamped beside it.
- Schema conformance, with a test proving the schema rejects a row missing its stamp.

Gates: `cargo test --workspace`, `fmt`, `clippy --workspace -D warnings`, `metadata --locked`,
`git diff --check`, and **`cargo run -p meridian-spec -- check`, which is what runs the census
rule** — stated explicitly because the point of relocating the census was that it leaves the
`project --check` path.

## Failure injection and recovery

Extended to all ten sections; revision 2's cases covered three.

- **Mutate a source file, re-run `check`, confirm the census reports stale.** That is the whole
  premise of `source_tree_checkpoint` and nothing previously tested it.
- Add a crate and confirm the completeness assertion fails naming it.
- Add a `#[test]` and confirm the test-row count assertion fails.
- Remove a format constant and confirm the enumeration fails against the known-format set.
- Set both `disposition` and `escalation` on one row and confirm rejection; set neither in
  `CENSUS-002` and confirm rejection there.
- Reorder `layers` and confirm previously-forward edges are reported reverse.
- Stale a `ci_rows` entry and confirm it is reported.
- Inject a non-null `implementation_maturity` and confirm rejection.
- Point the generator at a tree with no `Cargo.toml` and confirm a diagnostic, not a panic.

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
census asserts a disposition the specoment's vocabulary does not contain. Rollback is one commit. Files touched: `.meridian/implementation/census.json`,
`governance/schemas/census.schema.json`, and the generator; nothing under
`governance/generated/` changes, so `project --check` stays green.

## Independent Review

_Pending._

## Completion record

_Pending._
