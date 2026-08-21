//! Attribution: deciding which heading owns each canonical identifier.
//!
//! The rule, measured against the corpus rather than invented:
//!
//! > A heading declares an identifier if the identifier is backticked in that heading, or
//! > if the heading text begins with it. A bare, non-initial identifier in a heading is a
//! > reference. A heading declaring a range (`FAM-001..005`) declares its members weakly;
//! > a later heading declaring a single member supersedes that weak declaration and is not
//! > a re-declaration.
//!
//! Of all identifier occurrences in headings across the body, 588 are backticked, 111 are
//! bare and heading-initial (the 99 phase cards plus the 12 Appendix E work-package
//! briefs), and exactly one is bare and mid-heading — the `PRODUCT-002` occurrence at line
//! 17707, "Ease-of-use condition inherited from PRODUCT-002".
//!
//! An identifier that appears only in body prose and never in a heading is reported as
//! undeclared rather than attributed to whichever section mentioned it first.

use std::collections::{BTreeMap, BTreeSet};

use super::scan::{self, Placement};

/// Identifier families belonging to the frozen v0.5 authority. These appear only as
/// migration history and must never be re-entered into v1 registries as live contracts.
///
/// `RG` is deliberately **absent**, unlike the reference generator's list. `RG-TOR-001` is
/// the only `RG-*` identifier in the document, and it is cited in a live v1 section —
/// "Torsant implementation research and evidence — *Open implementation research*" — as a
/// currently open gate, not as history. Classifying it as retired v0.5 on the bare prefix
/// hid a genuinely undeclared v1 identifier inside a category that is exempt from the
/// undeclared count, which is what made "0 undeclared" read as true. Recorded as SD-006.
const RETIRED_V05_PREFIXES: &[&str] = &[
    "WP-UI", "WP-REL", "WP-EDT", "WP-BLD", "WP-PRC", "WP-MDL", "WP-GAM", "MS", "WVR", "REQ", "DEP",
    "VAL", "PRG",
];

/// Where an identifier was declared.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub heading: String,
    pub line: usize,
    /// True when the only declaration is a range heading such as `OPEN-001..004`.
    pub weak: bool,
    /// The verbatim trailing maturity label, e.g. `Normative direction`, exactly as
    /// written. Never normalised: section 0.3 defines six labels while headings use 29
    /// distinct phrases, and mapping between them is `PH-AUTH-003`'s "status axes" scope.
    /// A projection that normalised here would assert a status the root file does not.
    pub maturity_label: Option<String>,
}

/// A body or heading citation of an identifier that does not declare it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub heading: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Index {
    pub declared: BTreeMap<String, Declaration>,
    pub references: BTreeMap<String, Vec<Reference>>,
    /// Cited somewhere but never declared by any heading.
    pub undeclared: Vec<String>,
    /// Retired v0.5 identifiers cited as history.
    pub retired_v05: Vec<String>,
    /// Identifiers a second heading declared as a single identifier. Expected to be empty.
    pub multiply_declared: BTreeMap<String, Vec<usize>>,
    pub families: BTreeSet<String>,
}

impl Index {
    /// Total identifiers with an owning heading. Equals the number of entries emitted,
    /// which is the invariant the reference generator violated by 31.
    pub fn declared_count(&self) -> usize {
        self.declared.len()
    }
}

fn heading_of(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.strip_prefix('#')?;
    let mut depth = 1;
    let mut rest = trimmed;
    while let Some(next) = rest.strip_prefix('#') {
        depth += 1;
        rest = next;
    }
    if depth > 6 {
        return None;
    }
    let text = rest.strip_prefix(' ')?.trim();
    Some((depth, text))
}

/// The trailing `— *label*` maturity marker, verbatim.
fn maturity_label(text: &str) -> Option<String> {
    let tail = text.rsplit_once("— *")?.1;
    let label = tail.strip_suffix('*')?;
    (!label.is_empty() && !label.contains('*')).then(|| label.to_string())
}

fn is_retired_v05(id: &str) -> bool {
    RETIRED_V05_PREFIXES
        .iter()
        .any(|prefix| id.strip_prefix(prefix).is_some_and(|r| r.starts_with('-')))
}

/// Build the index from the specoment body.
///
/// The scan stops at `# Appendix A`, so the document's own embedded traceability index is
/// never treated as a source of declarations. Without that, the generated index would feed
/// on a previous copy of itself.
pub fn build(text: &str) -> Index {
    let lines: Vec<&str> = text.split('\n').collect();
    let body_end = lines
        .iter()
        .position(|line| line.starts_with("# Appendix A"))
        .unwrap_or(lines.len());

    let mut index = Index::default();
    let mut strong: BTreeMap<String, Declaration> = BTreeMap::new();
    let mut weak: BTreeMap<String, Declaration> = BTreeMap::new();
    let mut section = (String::from("(preamble)"), 0usize);

    for (offset, raw) in lines[..body_end].iter().enumerate() {
        let number = offset + 1;
        match heading_of(raw) {
            Some((depth, text)) => {
                if depth <= 4 {
                    section = (text.to_string(), number);
                }
                let label = maturity_label(text);
                for occurrence in scan::heading(text) {
                    let declares = matches!(
                        occurrence.placement,
                        Placement::Backticked | Placement::BareInitial
                    );
                    if !declares {
                        // A referencing heading. Recorded as a citation, never a declaration.
                        for id in occurrence.expand() {
                            push_reference(&mut index, &id, text, number);
                        }
                        continue;
                    }
                    let ranged = occurrence.is_range();
                    for id in occurrence.expand() {
                        index.families.insert(scan::family(&id).to_string());
                        let declaration = Declaration {
                            heading: text.to_string(),
                            line: number,
                            weak: ranged,
                            maturity_label: label.clone(),
                        };
                        if ranged {
                            weak.entry(id).or_insert(declaration);
                        } else if let Some(existing) = strong.get(&id) {
                            index
                                .multiply_declared
                                .entry(id)
                                .or_insert_with(|| vec![existing.line])
                                .push(number);
                        } else {
                            strong.insert(id, declaration);
                        }
                    }
                }
            }
            None => {
                for id in scan::body(raw) {
                    index.families.insert(scan::family(&id).to_string());
                    let (heading, line) = (section.0.clone(), section.1);
                    push_reference(&mut index, &id, &heading, line);
                }
            }
        }
    }

    // A range heading owns only those members no single heading claimed.
    index.declared = strong;
    for (id, declaration) in weak {
        index.declared.entry(id).or_insert(declaration);
    }

    // Defect 1: a declaring heading must not appear in its own reference list.
    for (id, declaration) in &index.declared {
        if let Some(references) = index.references.get_mut(id) {
            references.retain(|reference| reference.line != declaration.line);
            if references.is_empty() {
                index.references.remove(id);
            }
        }
    }

    let cited: Vec<String> = index.references.keys().cloned().collect();
    for id in cited {
        if index.declared.contains_key(&id) {
            continue;
        }
        if is_retired_v05(&id) {
            index.retired_v05.push(id);
        } else {
            index.undeclared.push(id);
        }
    }
    index.undeclared.sort();
    index.retired_v05.sort();
    index
}

fn push_reference(index: &mut Index, id: &str, heading: &str, line: usize) {
    let entry = index.references.entry(id.to_string()).or_default();
    let reference = Reference {
        heading: heading.to_string(),
        line,
    };
    if !entry.contains(&reference) {
        entry.push(reference);
    }
}

#[cfg(test)]
mod tests {
    use super::build;

    #[test]
    fn a_heading_declaration_beats_an_earlier_prose_mention() {
        let index = build("Prose cites `FOO-001` first.\n\n## Contract `FOO-001` — *Normative*\n");
        assert_eq!(index.declared["FOO-001"].line, 3);
        assert!(!index.declared["FOO-001"].weak);
    }

    #[test]
    fn an_identifier_never_in_a_heading_is_undeclared() {
        let index = build("Prose cites `FOO-001` and nothing declares it.\n");
        assert!(index.declared.is_empty());
        assert_eq!(index.undeclared, vec!["FOO-001".to_string()]);
    }

    /// Defect 1. The reference generator listed a declaring heading in its own reference
    /// set 44 times.
    #[test]
    fn a_declaring_heading_is_absent_from_its_own_reference_set() {
        let index = build("## Contract `FOO-001` — *Normative* and `FOO-001` again\n");
        assert!(
            !index.references.contains_key("FOO-001"),
            "the declaration site is not a reference to itself"
        );
    }

    /// A single-identifier heading supersedes a range heading; that is not a duplicate.
    #[test]
    fn a_single_heading_supersedes_a_range_heading_without_a_duplicate() {
        let index =
            build("## Family `FOO-001..002` — *Normative*\n\n### One `FOO-001` — *Normative*\n");
        assert!(
            index.multiply_declared.is_empty(),
            "{:?}",
            index.multiply_declared
        );
        assert_eq!(index.declared["FOO-001"].line, 3);
        assert!(!index.declared["FOO-001"].weak);
        assert!(
            index.declared["FOO-002"].weak,
            "unclaimed member stays weak"
        );
    }

    /// Invariant 5. Two headings declaring the same single identifier is a real duplicate.
    #[test]
    fn two_headings_declaring_one_identifier_are_reported() {
        let index = build("## A `FOO-001` — *Normative*\n\n## B `FOO-001` — *Normative*\n");
        assert_eq!(index.multiply_declared["FOO-001"], vec![1, 3]);
    }

    /// The `PRODUCT-002` case: a heading that references does not declare.
    #[test]
    fn a_referencing_heading_does_not_declare() {
        let index = build(
            "## Ease contract `PRODUCT-002` — *Normative*\n\n## Inherited from PRODUCT-002 — *Normative*\n",
        );
        assert!(index.multiply_declared.is_empty());
        assert_eq!(index.declared["PRODUCT-002"].line, 1);
        assert_eq!(index.references["PRODUCT-002"][0].line, 3);
    }

    #[test]
    fn maturity_labels_are_carried_verbatim() {
        let index = build("## Thing `FOO-001` — *Normative ambition and core architecture*\n");
        assert_eq!(
            index.declared["FOO-001"].maturity_label.as_deref(),
            Some("Normative ambition and core architecture")
        );
    }

    #[test]
    fn retired_v05_identifiers_are_segregated() {
        let index = build("History mentions `WP-UI-006` and `REQ-004`.\n");
        assert!(index.undeclared.is_empty(), "{:?}", index.undeclared);
        assert_eq!(
            index.retired_v05,
            vec!["REQ-004".to_string(), "WP-UI-006".to_string()]
        );
    }

    /// SD-006. A live v1 identifier must not be absorbed into the retired-v0.5 category
    /// by a bare prefix match, because that category is exempt from the undeclared count
    /// and absorbing it there manufactures a clean "0 undeclared".
    #[test]
    fn a_live_v1_research_gate_is_undeclared_not_retired() {
        let index = build("Still open under `RG-TOR-001` and implementation research.\n");
        assert_eq!(index.undeclared, vec!["RG-TOR-001".to_string()]);
        assert!(index.retired_v05.is_empty(), "{:?}", index.retired_v05);
    }

    #[test]
    fn appendix_a_is_outside_the_scan_window() {
        let index =
            build("## Real `FOO-001` — *Normative*\n\n# Appendix A — index\n\n## Fake `BAR-001`\n");
        assert!(index.declared.contains_key("FOO-001"));
        assert!(
            !index.declared.contains_key("BAR-001"),
            "the document's own index must not feed the generated one"
        );
    }
}
