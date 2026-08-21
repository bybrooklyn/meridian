//! Phase and research-gate extraction from the specoment's prose.
//!
//! Phases are derived from the 99 prose phase cards, **not** from the Appendix G JSON
//! fence. Appendix G's own preamble states: "If a serialization defect conflicts with a
//! prose phase card, the prose phase card wins until the registry is regenerated." A
//! checked-in copy of that fence would therefore be a projection of a projection, and the
//! root file already declares which of the two is subordinate.
//!
//! The two do diverge today, which `reconcile_with_appendix_g` reports.

use std::collections::BTreeMap;

/// One phase card, exactly as the prose states it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Phase {
    pub id: String,
    pub title: String,
    pub line: usize,
    pub gate: String,
    pub depends_on: Vec<String>,
    pub fields: BTreeMap<String, String>,
}

/// A research gate. Many carry no identifier at all, so identity is heading plus line.
#[derive(Debug, Clone)]
pub struct ResearchGate {
    pub heading: String,
    pub line: usize,
    pub label: String,
    pub identifiers: Vec<String>,
}

const FIELDS: &[(&str, &str)] = &[
    ("User-visible result", "user_visible_result"),
    ("Current-code disposition", "current_code_disposition"),
    ("Implementation scope", "implementation_scope"),
    ("Closure evidence", "closure_evidence"),
    ("Explicit exclusions", "explicit_exclusions"),
    ("Stop/rollback condition", "stop_conditions"),
];

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip inline backticks so prose and a JSON serialization of it compare equal.
fn plain(text: &str) -> String {
    collapse(&text.replace('`', "")).replace('\u{2019}', "'")
}

/// Parse every `## PH-*` card in the document.
///
/// A card body ends at the next heading of level 1 or 2. Terminating only at `## ` lets an
/// intervening `# Epoch N` heading bleed into the last card of an epoch, which produced a
/// phantom divergence in `PH-REL-008` during planning.
pub fn parse(source: &str) -> Vec<Phase> {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut phases = Vec::new();

    for (offset, raw) in lines.iter().enumerate() {
        let Some(rest) = raw.strip_prefix("## ") else {
            continue;
        };
        let Some((id, title)) = rest.split_once(" — ") else {
            continue;
        };
        if !id.starts_with("PH-") || !super::scan::heading(id).iter().any(|o| o.token == id) {
            continue;
        }

        let mut body = Vec::new();
        for line in &lines[offset + 1..] {
            if line.starts_with("# ") || line.starts_with("## ") {
                break;
            }
            body.push(*line);
        }
        let text = body.join("\n");

        let mut phase = Phase {
            id: id.to_string(),
            // `plain`, not `collapse`: the comparison side normalises backticks out of the
            // fence value, so normalising only one side reports four phantom divergences.
            title: plain(title),
            line: offset + 1,
            ..Phase::default()
        };

        for line in &body {
            if let Some(value) = line.trim().strip_prefix("**Gate:**") {
                phase.gate = plain(value);
            }
            if let Some(value) = line.trim().strip_prefix("**Depends on:**") {
                phase.depends_on = super::scan::body(value);
                if phase.depends_on.is_empty() {
                    // "None" is written in prose rather than as an identifier list.
                    phase.depends_on = Vec::new();
                }
            }
        }

        for (label, key) in FIELDS {
            let marker = format!("**{label}.**");
            let Some(at) = text.find(&marker) else {
                continue;
            };
            let after = &text[at + marker.len()..];
            let end = after.find("\n\n**").unwrap_or(after.len());
            phase
                .fields
                .insert((*key).to_string(), plain(&after[..end]));
        }
        phases.push(phase);
    }
    phases
}

/// Research gates: headings whose maturity label mentions research, plus their identifiers.
pub fn research_gates(source: &str) -> Vec<ResearchGate> {
    let mut gates = Vec::new();
    for (offset, raw) in source.split('\n').enumerate() {
        let Some(text) = raw.trim_start_matches('#').strip_prefix(' ') else {
            continue;
        };
        if !raw.starts_with('#') {
            continue;
        }
        let Some(label) = text
            .rsplit_once("— *")
            .and_then(|(_, tail)| tail.strip_suffix('*'))
        else {
            continue;
        };
        let lowered = label.to_ascii_lowercase();
        if !(lowered.contains("research") || lowered.contains("prototype-gated")) {
            continue;
        }
        gates.push(ResearchGate {
            heading: collapse(text),
            line: offset + 1,
            label: label.to_string(),
            identifiers: super::scan::heading(text)
                .into_iter()
                .flat_map(|o| o.expand())
                .collect(),
        });
    }
    gates
}

/// One field of one phase where the prose and the Appendix G fence disagree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Divergence {
    pub phase: String,
    pub field: String,
}

/// Compare the prose-derived phases against the Appendix G JSON fence.
///
/// Divergence is expected and is reported as a **set**, never a count. A count is a
/// snapshot that goes stale on the next specoment edit; the set encodes the structural
/// finding — that Epoch 0 cards were revised after the fence was serialised.
pub fn reconcile_with_appendix_g(
    source: &str,
    phases: &[Phase],
) -> Result<Vec<Divergence>, String> {
    let fence = appendix_g(source)?;
    let registry: serde_json::Value = serde_json::from_str(&fence)
        .map_err(|error| format!("Appendix G is not valid JSON: {error}"))?;

    let mut lists: Vec<&serde_json::Value> = Vec::new();
    if let Some(list) = registry.get("phases") {
        lists.push(list);
    }
    // The optional-program phases live under `optional_programs`, not `phases`. Reading
    // only the top-level array concludes the fence holds zero PH-AI-* entries, which is
    // wrong; it holds all twelve.
    if let Some(programs) = registry
        .get("optional_programs")
        .and_then(|v| v.as_object())
    {
        for program in programs.values() {
            if let Some(list) = program.get("phases") {
                lists.push(list);
            }
        }
    }
    let mut serialized: BTreeMap<String, &serde_json::Value> = BTreeMap::new();
    for list in lists {
        for entry in list.as_array().into_iter().flatten() {
            if let Some(id) = entry.get("id").and_then(serde_json::Value::as_str) {
                serialized.insert(id.to_string(), entry);
            }
        }
    }

    let mut divergences = Vec::new();
    for phase in phases {
        let Some(entry) = serialized.get(&phase.id) else {
            divergences.push(Divergence {
                phase: phase.id.clone(),
                field: "absent-from-appendix-g".to_string(),
            });
            continue;
        };
        let mut compare = |field: &str, prose: &str| {
            let fence_value = entry
                .get(field)
                .and_then(serde_json::Value::as_str)
                .map(plain)
                .unwrap_or_default();
            if fence_value != prose {
                divergences.push(Divergence {
                    phase: phase.id.clone(),
                    field: field.to_string(),
                });
            }
        };
        compare("gate", &phase.gate);
        compare("title", &phase.title);
        for (_, key) in FIELDS {
            if let Some(value) = phase.fields.get(*key) {
                compare(key, value);
            }
        }
    }
    divergences.sort();
    Ok(divergences)
}

fn appendix_g(source: &str) -> Result<String, String> {
    let at = source
        .find("# Appendix G")
        .ok_or_else(|| "Appendix G is absent".to_string())?;
    let after = &source[at..];
    let start = after
        .find("```json\n")
        .ok_or_else(|| "Appendix G has no JSON fence".to_string())?
        + "```json\n".len();
    let end = after[start..]
        .find("\n```")
        .ok_or_else(|| "Appendix G's JSON fence is unterminated".to_string())?;
    Ok(after[start..start + end].to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse, plain, research_gates};

    const CARD: &str = "## PH-TEST-001 — A title\n\n**Gate:** Required 1.0  \n\
        **Depends on:** `PH-TEST-000`\n\n**User-visible result.** Something happens.\n\n\
        **Closure evidence.** It is proven.\n\n# Epoch 9 — next\n\n## PH-TEST-002 — Another\n\n\
        **Gate:** Conditional  \n**Depends on:** None\n";

    #[test]
    fn titles_are_normalised_the_same_way_as_the_comparison_side() {
        let phases = parse("## PH-TEST-003 — A `.mui` title\n\n**Gate:** Required 1.0  \n");
        assert_eq!(
            phases[0].title, "A .mui title",
            "titles must be normalised identically on both sides of the reconciliation"
        );
    }

    #[test]
    fn cards_are_parsed_with_their_fields() {
        let phases = parse(CARD);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].id, "PH-TEST-001");
        assert_eq!(phases[0].title, "A title");
        assert_eq!(phases[0].gate, "Required 1.0");
        assert_eq!(phases[0].depends_on, vec!["PH-TEST-000".to_string()]);
        assert_eq!(
            phases[0].fields.get("closure_evidence").map(String::as_str),
            Some("It is proven.")
        );
    }

    /// A card body must end at any level-1 or level-2 heading. Terminating only at `## `
    /// lets an intervening `# Epoch N` bleed into the last card of an epoch, which produced
    /// a phantom `PH-REL-008` divergence during planning.
    #[test]
    fn an_epoch_heading_does_not_bleed_into_the_previous_card() {
        let phases = parse(CARD);
        let evidence = phases[0].fields.get("closure_evidence").unwrap();
        assert!(
            !evidence.contains("Epoch"),
            "epoch heading bled into the card: {evidence}"
        );
    }

    #[test]
    fn a_dependency_free_phase_has_no_dependencies() {
        let phases = parse(CARD);
        assert!(phases[1].depends_on.is_empty());
    }

    #[test]
    fn backticks_do_not_make_prose_differ_from_its_serialization() {
        assert_eq!(plain("the `FOO-001` contract"), "the FOO-001 contract");
    }

    #[test]
    fn research_labelled_headings_are_collected() {
        let gates = research_gates(
            "### Rust-authored shaders — *Research gate*\n### Settled `FOO-001` — *Normative*\n",
        );
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].label, "Research gate");
        assert!(
            gates[0].identifiers.is_empty(),
            "many gates carry no identifier"
        );
    }
}
