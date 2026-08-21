//! Identifier scanning for the specoment body.
//!
//! Why this is hand-written rather than `regex`: `regex` is already in `Cargo.lock` via
//! `jsonschema`, so promoting it would cost nothing in compiled code, and `LEGAL-006` is
//! explicit that avoiding a dependency is not itself a goal. The deciding ground is the one
//! `LEGAL-006` names as **API fit**. This is not a matching problem; it is a
//! context-sensitive tokenizer. The same character sequence means different things
//! depending on whether it sits in a heading or a body line, whether it is backticked,
//! heading-initial or mid-heading, whether it expands as a range, and whether a trailing
//! letter belongs to the identity.
//!
//! That last distinction is not hypothetical. The reference generator expressed its right
//! boundary as the lookahead `(?![0-9-])`, which excludes digits and hyphens but not
//! letters. It therefore read `NETPROJ-006A` as `NETPROJ-006` and lost five canonical
//! identifiers — `NETPROJ-006A..D` and `SCM-010A` — in two different failure shapes. Here
//! the boundary is a named predicate with its own test rather than an interaction between
//! a lookahead and a conditional.
//!
//! No claim is made that this is safer than `regex`, which guarantees linear time by
//! construction. On that axis this is at best parity.

/// Where an identifier occurrence sits, which decides whether it declares or references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Written between backticks. Declares, in a heading.
    Backticked,
    /// Bare, and the heading text begins with it. Declares, in a heading.
    /// This is the phase-card and work-package-brief form: `## PH-AUTH-001 — ...`.
    BareInitial,
    /// Bare and not at the start. References, never declares.
    /// Exactly one occurrence in the current corpus: "inherited from PRODUCT-002".
    BareMid,
}

/// One identifier occurrence, before range expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub token: String,
    pub placement: Placement,
}

impl Occurrence {
    /// A range token such as `FAM-001..005` declares its members only weakly: a later
    /// heading naming a single member supersedes it and is not a re-declaration.
    pub fn is_range(&self) -> bool {
        self.token.contains("..")
    }

    /// `FAM-001..003` becomes three identifiers; anything else passes through.
    pub fn expand(&self) -> Vec<String> {
        expand(&self.token)
    }
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// The right boundary. Letters are excluded here deliberately: without that, a trailing
/// letter is silently dropped and a distinct identifier collapses onto its stem.
fn boundary_ok(bytes: &[u8], at: usize) -> bool {
    at >= bytes.len() || !is_ident_byte(bytes[at])
}

/// Match one identifier starting exactly at `start`, returning its end offset.
///
/// Grammar: an uppercase segment, then any number of `-SEGMENT` groups, then `-NNN`,
/// then optional trailing uppercase letters, then an optional `..NNN` range tail.
fn match_at(bytes: &[u8], start: usize) -> Option<usize> {
    let mut at = start;
    if at >= bytes.len() || !bytes[at].is_ascii_uppercase() {
        return None;
    }
    while at < bytes.len() && (bytes[at].is_ascii_uppercase() || bytes[at].is_ascii_digit()) {
        at += 1;
    }

    // At least one `-SEGMENT`, the last of which must be exactly three digits.
    //
    // The digit run is measured on its own before falling back to a general segment.
    // Consuming letters and digits together would read `SCM-010A` as one four-character
    // segment `010A`, fail the three-digit test, and reject the identifier outright —
    // which is how the first draft of this function lost the very identifiers it exists
    // to preserve.
    let mut saw_number = false;
    loop {
        if at >= bytes.len() || bytes[at] != b'-' {
            break;
        }
        let segment_start = at + 1;

        let mut digits = segment_start;
        while digits < bytes.len() && bytes[digits].is_ascii_digit() {
            digits += 1;
        }
        if digits - segment_start == 3 {
            at = digits;
            // Trailing letters belong to the identity: SCM-010A is not SCM-010.
            while at < bytes.len() && bytes[at].is_ascii_uppercase() {
                at += 1;
            }
            saw_number = true;
            break;
        }

        let mut cursor = segment_start;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_uppercase() || bytes[cursor].is_ascii_digit())
        {
            cursor += 1;
        }
        if cursor == segment_start {
            break;
        }
        at = cursor;
    }
    if !saw_number {
        return None;
    }

    // Optional range tail `..NNN`.
    if bytes.len() >= at + 5
        && &bytes[at..at + 2] == b".."
        && bytes[at + 2..at + 5].iter().all(u8::is_ascii_digit)
    {
        at += 5;
    }

    boundary_ok(bytes, at).then_some(at)
}

/// Expand a range token into its members. Non-range tokens pass through unchanged.
pub fn expand(token: &str) -> Vec<String> {
    let Some((head, tail)) = token.split_once("..") else {
        return vec![token.to_string()];
    };
    let Some((family, low)) = head.rsplit_once('-') else {
        return vec![token.to_string()];
    };
    let (Ok(low), Ok(high)) = (low.parse::<u32>(), tail.parse::<u32>()) else {
        return vec![token.to_string()];
    };
    if high < low {
        return vec![token.to_string()];
    }
    (low..=high).map(|n| format!("{family}-{n:03}")).collect()
}

/// The family prefix of an identifier: `PH-AUTH-001` becomes `PH-AUTH`.
pub fn family(id: &str) -> &str {
    id.rsplit_once('-').map_or(id, |(head, _)| head)
}

/// Scan a heading's text, classifying each occurrence by placement.
pub fn heading(text: &str) -> Vec<Occurrence> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        if at > 0 && is_ident_byte(bytes[at - 1]) {
            at += 1;
            continue;
        }
        let Some(end) = match_at(bytes, at) else {
            at += 1;
            continue;
        };
        let backticked = at > 0 && bytes[at - 1] == b'`' && end < bytes.len() && bytes[end] == b'`';
        let placement = if backticked {
            Placement::Backticked
        } else if at == 0 {
            Placement::BareInitial
        } else {
            Placement::BareMid
        };
        found.push(Occurrence {
            token: text[at..end].to_string(),
            placement,
        });
        at = end;
    }
    found
}

/// Scan a body line. Only backticked identifiers count as references; a bare prose mention
/// such as "AI-031 through AI-033" is discussion, and treating it as an assignment would
/// mask a genuine family gap.
pub fn body(text: &str) -> Vec<String> {
    heading(text)
        .into_iter()
        .filter(|occurrence| occurrence.placement == Placement::Backticked)
        .flat_map(|occurrence| occurrence.expand())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{body, expand, family, heading, Placement};

    fn tokens(text: &str) -> Vec<(String, Placement)> {
        heading(text)
            .into_iter()
            .map(|o| (o.token, o.placement))
            .collect()
    }

    #[test]
    fn backticked_heading_identifier_declares() {
        assert_eq!(
            tokens("Ease contract `PRODUCT-002` — *Normative*"),
            vec![("PRODUCT-002".to_string(), Placement::Backticked)]
        );
    }

    #[test]
    fn heading_initial_bare_identifier_declares() {
        assert_eq!(
            tokens("PH-AUTH-001 — Freeze the v0.5 baseline"),
            vec![("PH-AUTH-001".to_string(), Placement::BareInitial)]
        );
    }

    /// The `PRODUCT-002`@17707 case. A heading that mentions a contract mid-sentence
    /// references it; it does not declare a second time.
    #[test]
    fn bare_mid_heading_identifier_references() {
        assert_eq!(
            tokens("Ease-of-use condition inherited from PRODUCT-002 — *Normative*"),
            vec![("PRODUCT-002".to_string(), Placement::BareMid)]
        );
    }

    /// Defect 4, shape A. The reference generator's `(?![0-9-])` guard let these four
    /// distinct Normative contracts collapse onto `NETPROJ-006`.
    #[test]
    fn letter_suffixed_identifier_is_distinct_from_its_stem() {
        assert_eq!(
            tokens("Prediction-safe code sharing `NETPROJ-006A` — *Normative*"),
            vec![("NETPROJ-006A".to_string(), Placement::Backticked)]
        );
        assert_eq!(
            tokens("Bounded explicit compatibility windows `NETPROJ-006` — *Normative*"),
            vec![("NETPROJ-006".to_string(), Placement::Backticked)]
        );
    }

    /// Defect 4, shape B. One heading declaring both; the reference generator silently
    /// overwrote `SCM-010A` because the duplicate guard compares line numbers.
    #[test]
    fn one_heading_can_declare_a_stem_and_its_letter_suffixed_sibling() {
        assert_eq!(
            tokens("Source-control CLI family `SCM-010` `SCM-010A` — *Normative*"),
            vec![
                ("SCM-010".to_string(), Placement::Backticked),
                ("SCM-010A".to_string(), Placement::Backticked),
            ]
        );
    }

    #[test]
    fn ph_ai_005_does_not_yield_ai_005() {
        let found = tokens("PH-AI-005 — Retrieval");
        assert_eq!(
            found,
            vec![("PH-AI-005".to_string(), Placement::BareInitial)]
        );
    }

    #[test]
    fn a_longer_number_does_not_yield_a_shorter_identifier() {
        assert!(
            tokens("`AI-0051`").is_empty(),
            "AI-0051 must not yield AI-005"
        );
    }

    #[test]
    fn ranges_expand_to_members() {
        assert_eq!(
            expand("FAM-001..003"),
            vec!["FAM-001", "FAM-002", "FAM-003"]
        );
        assert_eq!(expand("FAM-001"), vec!["FAM-001"]);
    }

    #[test]
    fn range_occurrence_is_marked_as_such() {
        let found = heading("Open source, not open core `OPEN-001..004` — *Normative*");
        assert_eq!(found.len(), 1);
        assert!(found[0].is_range());
        assert_eq!(found[0].expand().len(), 4);
    }

    #[test]
    fn body_lines_count_only_backticked_identifiers() {
        assert_eq!(body("see `AI-031` for detail"), vec!["AI-031".to_string()]);
        assert!(
            body("AI-031 through AI-033 remain unassigned").is_empty(),
            "a bare prose mention is discussion, not an assignment, and must not mask a gap"
        );
    }

    #[test]
    fn families_are_the_prefix() {
        assert_eq!(family("PH-AUTH-001"), "PH-AUTH");
        assert_eq!(family("AI-005"), "AI");
    }
}
