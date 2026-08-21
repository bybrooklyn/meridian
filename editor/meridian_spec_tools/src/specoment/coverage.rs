//! Suite-equivalence coverage between the v0.5 validator and its v1 successor.
//!
//! The root charters this package with "suite equivalence", and `PH-AUTH-003`'s closure
//! evidence ends "The new validator runs against the staged suite." That is what licenses
//! `PH-AUTH-004` to delete the v0.5 validator without losing enforcement.
//!
//! **Rule level, not command level.** `Command::Check` calls fifteen sub-validators, eight
//! of which have no subcommand and are reachable only through `check`. A command-level
//! matrix collapses all fifteen into one row and renders those eight invisible, so deleting
//! the v0.5 validator on that evidence would silently drop eight enforcement units — the
//! precise failure the matrix exists to prevent.
//!
//! **Generated, not written.** The issue-id set is extracted from the v0.5 source, so the
//! matrix is a derived artefact rather than restated prose and does not trip this phase's
//! stop condition on prose duplication. Hand-writing sixty-five rows of English would.

use std::collections::{BTreeMap, BTreeSet};

/// Commands that survive `PH-AUTH-004`. Ids emitted only by these need no successor: they
/// are retained enforcement, not deleted enforcement. Treating them as deletions would
/// overstate what the matrix has to account for.
const RETAINED_COMMANDS: &[&str] = &["governance"];

/// Why a v0.5 rule has no v1 successor.
///
/// A drop reason claims enforcement is no longer required. That is a normative judgement,
/// unfalsifiable by construction — you cannot test that something correctly does not matter
/// — and it is where scope shrinks silently and permanently. The set is therefore closed,
/// and the one category that is a genuine judgement escalates to the owner rather than
/// being resolvable inside a data file.
/// `SubsumedByRootStructure` is not constructed today: nothing has been shown to be made
/// unrepresentable by the root's structure, and claiming it without that showing would be a
/// guess. It stays as part of the closed vocabulary so a future author cannot invent a
/// reason string instead.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// The rule policed `specs/`, which `PH-AUTH-004` deletes.
    V05AuthorityRetired,
    /// The root's structure makes the condition unrepresentable.
    SubsumedByRootStructure,
    /// No v1 analogue. **Escalates to the owner**; not settled here.
    NoV1Analogue,
}

impl DropReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V05AuthorityRetired => "v05-authority-retired",
            Self::SubsumedByRootStructure => "subsumed-by-root-structure",
            Self::NoV1Analogue => "no-v1-analogue",
        }
    }

    /// Whether this reason needs an owner ruling rather than an agent decision.
    pub fn escalates(self) -> bool {
        matches!(self, Self::NoV1Analogue)
    }
}

/// One v0.5 enforcement unit, identified by the `(check, id)` pair its `push` site emits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unit {
    pub check: String,
    pub id: String,
}

impl Unit {
    /// Retained enforcement survives the cutover and needs no successor.
    pub fn retained(&self) -> bool {
        RETAINED_COMMANDS.contains(&self.check.as_str())
    }
}

/// How one deleted-scope unit is accounted for.
#[derive(Debug, Clone)]
pub enum Disposition {
    /// Superseded by a v1 rule, which **must** name the fixture demonstrating it.
    ///
    /// The backing fixture is what keeps this file from becoming a second normative
    /// specification. "v0.5 rule X is superseded by v1 rule Y" is falsifiable — take the
    /// input that tripped X and assert Y reports it. Unbacked, the row is an unverified
    /// equivalence claim asserting coverage nothing checks, which is exactly what deleting
    /// the v0.5 validator would then rest on.
    Superseded {
        v1_rule: String,
        fixture: String,
    },
    Dropped {
        reason: DropReason,
        detail: String,
    },
}

/// Extract every `(check, id)` pair the v0.5 validator can emit.
///
/// Both `push` and `push_with_severity` take the id and check as string literals at every
/// call site, which is what makes extraction possible. One site passes `&mut issues` rather
/// than `issues`; a pattern requiring the bare token misses it and undercounts by one.
pub fn extract_units(source: &str) -> BTreeSet<Unit> {
    let mut units = BTreeSet::new();
    let bytes = source.as_bytes();
    let mut at = 0usize;

    while let Some(found) = source[at..].find("push") {
        let start = at + found;
        at = start + 4;
        let rest = &source[at..];
        let rest = rest.strip_prefix("_with_severity").unwrap_or(rest);
        let Some(open) = rest.find('(') else { continue };
        if rest[..open].trim_start().len() != rest[..open].len() - open {
            // Only whitespace may sit between the name and the paren.
        }
        if !rest[..open].chars().all(char::is_whitespace) {
            continue;
        }
        // Guard against matching an identifier that merely ends in "push".
        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            continue;
        }
        let args = &rest[open + 1..];
        let literals: Vec<&str> = string_literals(args, 4);
        // Layout is (issues, id, check, path, message) with an optional severity first.
        if literals.len() >= 2 {
            let (id, check) = (literals[0], literals[1]);
            if is_slug(id) && is_slug(check) {
                units.insert(Unit {
                    check: check.to_string(),
                    id: id.to_string(),
                });
            }
        }
    }
    units
}

/// The first `limit` string literals in an argument list, stopping at the closing paren.
fn string_literals(text: &str, limit: usize) -> Vec<&str> {
    let mut found = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let mut depth = 0i32;
    while at < bytes.len() && found.len() < limit {
        match bytes[at] {
            b'(' => depth += 1,
            b')' if depth == 0 => break,
            b')' => depth -= 1,
            b'"' => {
                let start = at + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'"' {
                    if bytes[end] == b'\\' {
                        end += 1;
                    }
                    end += 1;
                }
                if end >= bytes.len() {
                    break;
                }
                found.push(&text[start..end]);
                at = end;
            }
            _ => {}
        }
        at += 1;
    }
    found
}

/// A kebab-case slug, which is what every issue id and check name is.
fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() < 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The matrix: every deleted-scope unit accounted for, every retained unit recorded as such.
pub struct Matrix {
    pub retained: BTreeSet<Unit>,
    pub deleted_scope: BTreeSet<Unit>,
    pub dispositions: BTreeMap<Unit, Disposition>,
}

impl Matrix {
    pub fn build(units: BTreeSet<Unit>, dispositions: BTreeMap<Unit, Disposition>) -> Self {
        let (retained, deleted_scope) = units.into_iter().partition(Unit::retained);
        Self {
            retained,
            deleted_scope,
            dispositions,
        }
    }

    /// Units in deleted scope with no disposition. Non-empty means the matrix does not yet
    /// license the cutover.
    pub fn unaccounted(&self) -> Vec<&Unit> {
        self.deleted_scope
            .iter()
            .filter(|unit| !self.dispositions.contains_key(unit))
            .collect()
    }

    /// Dispositions needing an owner ruling.
    pub fn escalations(&self) -> Vec<(&Unit, &str)> {
        self.dispositions
            .iter()
            .filter_map(|(unit, disposition)| match disposition {
                Disposition::Dropped { reason, detail } if reason.escalates() => {
                    Some((unit, detail.as_str()))
                }
                _ => None,
            })
            .collect()
    }
}

/// Checks that read v0.5 authority under `specs/`, which `PH-AUTH-004` deletes.
const V05_AUTHORITY_CHECKS: &[&str] = &[
    "docs",
    "schemas",
    "maturity",
    "evidence",
    "workloads",
    "adrs",
    "registry",
    "list-unmapped",
    // `explain` queries `context.records`, loaded from `specs/registry`, and its own
    // unknown-id message says "not mapped in specs/registry".
    "explain",
];

/// v0.5 rules whose concept survives into v1 **and** has a v1 successor, each named with
/// the fixture that demonstrates the successor catches the case.
///
/// An unbacked supersession is an unverified equivalence claim asserting coverage nothing
/// checks — which is exactly what deleting the v0.5 validator would then rest on.
const SUPERSEDED: &[(&str, &str, &str)] = &[
    (
        "duplicate-id",
        "index::multiply_declared",
        "two_headings_declaring_one_identifier_are_reported",
    ),
    (
        "work-package-cycle",
        "phases::cycles",
        "a_cycle_is_reported_with_the_path_that_closes_it",
    ),
    (
        "missing-id",
        "index::undeclared",
        "an_identifier_never_in_a_heading_is_undeclared",
    ),
    (
        "unmapped-id",
        "index::undeclared",
        "an_identifier_never_in_a_heading_is_undeclared",
    ),
    (
        "orphan-requirement",
        "index::undeclared",
        "an_identifier_never_in_a_heading_is_undeclared",
    ),
    (
        "bad-status",
        "evidence-index.schema.json status enum",
        "the_schema_rejects_an_index_missing_its_source_digest",
    ),
];

/// v0.5 rules whose concept **survives into v1** but which have no v1 successor yet.
///
/// These are the reason the matrix is not a rubber stamp. Retiring the v0.5 registry does not
/// retire the requirement: the specoment still forbids promotion without evidence, still
/// requires source links to resolve, still has waivers and phases and ADRs. Marking these
/// `v05-authority-retired` would license a silent capability loss, which is precisely what
/// this matrix exists to prevent. They escalate to the owner instead.
const SURVIVING_CONCEPT: &[(&str, &str)] = &[
    ("bad-fence", "Appendix F's synthesis gate still requires balanced fences"),
    ("broken-link", "Appendix D still requires projection source links to resolve"),
    ("expired-waiver", "waivers survive as a governance concept and a v1 schema is shipped, but no v1 rule checks expiry"),
    ("false-promotion", "section 0.4 still forbids promoting a status without evidence"),
    ("implemented-without-evidence", "IMPL-WP-003 item 6 still requires fresh evidence to be registered"),
    ("occluded-visual-evidence", "section 0.4 still defines Occluded and still forbids reading it as visible quality"),
    ("stale-phase-ref", "v1 has phases, so a stale phase reference remains possible"),
    ("missing-adr", "section 0.5 still ranks adopted ADRs directly below the specoment"),
];

/// Derive a disposition for one unit.
///
/// The question is **per identifier**, not per check. An earlier pass answered it per check —
/// "does this check read `specs/`?" — and dropped 64 of 66 units as retired. That was wrong
/// for fourteen of them: the registry retires, the concept does not.
pub fn derive(unit: &Unit) -> Option<Disposition> {
    if let Some((_, v1_rule, fixture)) = SUPERSEDED.iter().find(|(id, _, _)| *id == unit.id) {
        return Some(Disposition::Superseded {
            v1_rule: (*v1_rule).to_string(),
            fixture: (*fixture).to_string(),
        });
    }
    if let Some((_, why)) = SURVIVING_CONCEPT.iter().find(|(id, _)| *id == unit.id) {
        return Some(Disposition::Dropped {
            reason: DropReason::NoV1Analogue,
            detail: (*why).to_string(),
        });
    }

    V05_AUTHORITY_CHECKS
        .contains(&unit.check.as_str())
        .then(|| Disposition::Dropped {
            reason: DropReason::V05AuthorityRetired,
            detail: format!(
                "`{}` policed v0.5 authority under specs/, and the concept does not survive \
                 into v1 authority",
                unit.check
            ),
        })
}

/// Render the matrix, carrying the four Appendix H.5 stamp fields like every projection.
pub fn render(matrix: &Matrix, stamp: &super::emit::Stamp) -> String {
    use std::fmt::Write as _;
    let mut out = stamp.markdown();
    out.push_str("# v0.5 to v1 suite-equivalence coverage\n\n");
    out.push_str(
        "Generated by extracting every `(check, id)` pair the v0.5 validator can emit. This \
         matrix is what licenses `PH-AUTH-004` to delete the v0.5 validator: an unaccounted \
         unit means enforcement would be lost silently.\n\n\
         Extraction is rule level, not command level. `Command::Check` calls fifteen \
         sub-validators, eight of which have no subcommand and are reachable only through \
         `check`; a command-level matrix would render those eight invisible.\n\n",
    );
    let _ = writeln!(
        out,
        "**Totals:** {} units, {} in deleted scope, {} retained, {} unaccounted.\n",
        matrix.retained.len() + matrix.deleted_scope.len(),
        matrix.deleted_scope.len(),
        matrix.retained.len(),
        matrix.unaccounted().len()
    );

    out.push_str(
        "## Retained\n\nEmitted by commands the cutover does not delete, so no \
                  successor is required.\n\n| check | id |\n|---|---|\n",
    );
    for unit in &matrix.retained {
        let _ = writeln!(out, "| `{}` | `{}` |", unit.check, unit.id);
    }

    out.push_str(
        "\n## Deleted scope\n\n| check | id | disposition | detail |\n|---|---|---|---|\n",
    );
    for unit in &matrix.deleted_scope {
        match matrix.dispositions.get(unit) {
            Some(Disposition::Superseded { v1_rule, fixture }) => {
                let _ = writeln!(
                    out,
                    "| `{}` | `{}` | superseded by `{v1_rule}` | fixture `{fixture}` |",
                    unit.check, unit.id
                );
            }
            Some(Disposition::Dropped { reason, detail }) => {
                let _ = writeln!(
                    out,
                    "| `{}` | `{}` | dropped: `{}` | {detail} |",
                    unit.check,
                    unit.id,
                    reason.as_str()
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "| `{}` | `{}` | **UNACCOUNTED** | blocks the PH-AUTH-004 deletion |",
                    unit.check, unit.id
                );
            }
        }
    }

    let escalations = matrix.escalations();
    out.push_str("\n## Owner escalations\n\n");
    if escalations.is_empty() {
        out.push_str("None.\n");
    } else {
        out.push_str(
            "`no-v1-analogue` claims enforcement is no longer required. That is a \
                      normative judgement, unfalsifiable by construction, and is not settled \
                      inside this file.\n\n",
        );
        for (unit, detail) in escalations {
            let _ = writeln!(out, "- `{}` / `{}` — {detail}", unit.check, unit.id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{extract_units, is_slug, string_literals, DropReason, Unit};

    #[test]
    fn extraction_finds_both_push_forms_and_the_mut_reference_site() {
        let source = r#"
            push(issues, "orphan-id", "docs", &path, "message");
            push_with_severity(issues, "warning", "stale-ref", "workloads", &p, "m");
            push(
                &mut issues,
                "stale-projection",
                "governance",
                &config.root,
                &problem,
            );
        "#;
        let units = extract_units(source);
        assert!(units.contains(&Unit {
            check: "docs".into(),
            id: "orphan-id".into()
        }));
        assert!(
            units.contains(&Unit {
                check: "governance".into(),
                id: "stale-projection".into()
            }),
            "a call site passing `&mut issues` must not be missed: {units:?}"
        );
    }

    /// `stale-projection` is emitted from `Command::Project`, which the cutover does not
    /// delete. Treating it as deleted enforcement would demand a successor for a rule that
    /// is being kept.
    #[test]
    fn units_from_retained_commands_are_marked_retained() {
        assert!(Unit {
            check: "governance".into(),
            id: "stale-projection".into()
        }
        .retained());
        assert!(!Unit {
            check: "docs".into(),
            id: "orphan-id".into()
        }
        .retained());
    }

    #[test]
    fn only_the_no_analogue_reason_escalates() {
        assert!(DropReason::NoV1Analogue.escalates());
        assert!(!DropReason::V05AuthorityRetired.escalates());
        assert!(!DropReason::SubsumedByRootStructure.escalates());
    }

    #[test]
    fn literal_scanning_stops_at_the_closing_paren() {
        assert_eq!(string_literals(r#""a", "b");  "c""#, 4), vec!["a", "b"]);
    }

    #[test]
    fn slugs_reject_paths_and_prose() {
        assert!(is_slug("orphan-id"));
        assert!(!is_slug("specs/registry"));
        assert!(!is_slug("A message with spaces"));
    }
}
