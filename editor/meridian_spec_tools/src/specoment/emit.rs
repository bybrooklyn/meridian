//! Rendering the index into generated projections.
//!
//! Every file carries the four-field stamp Appendix H.5 mandates. Nothing here interprets
//! the specoment: labels are verbatim, dispositions are read from the document, and no
//! status the root file does not state is ever asserted.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::index::Index;
use super::phases::{Divergence, Phase, ResearchGate};
use super::{scan, CANONICAL_PATH, GENERATOR_VERSION};

/// One generated file, in memory. Written or compared, never both.
pub struct Projection {
    pub relative_path: String,
    pub contents: String,
}

/// The Appendix H.5 stamp. A stale hash makes a projection non-authoritative.
pub struct Stamp {
    pub canonical_sha256: String,
    pub source_checkpoint: String,
}

impl Stamp {
    fn markdown(&self) -> String {
        format!(
            "<!--\nGENERATED FILE - DO NOT EDIT.\nRegenerate with: cargo run -p meridian-spec -- project\n\
             canonical_path = {CANONICAL_PATH}\ncanonical_sha256 = {}\ngenerator_version = {GENERATOR_VERSION}\n\
             generated_at_source_checkpoint = {}\n-->\n\n",
            self.canonical_sha256, self.source_checkpoint
        )
    }

    fn json_fields(&self) -> String {
        format!(
            concat!(
                "  \"generated\": \"DO NOT EDIT. Regenerate with: ",
                "cargo run -p meridian-spec -- project\",\n",
                "  \"canonical_path\": \"{path}\",\n",
                "  \"canonical_sha256\": \"{digest}\",\n",
                "  \"generator_version\": \"{version}\",\n",
                "  \"generated_at_source_checkpoint\": \"{checkpoint}\""
            ),
            path = CANONICAL_PATH,
            digest = self.canonical_sha256,
            version = GENERATOR_VERSION,
            checkpoint = self.source_checkpoint
        )
    }
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            other => out.push(other),
        }
    }
    out
}

/// The traceability index, Appendix A's projection.
fn index_markdown(index: &Index, stamp: &Stamp) -> String {
    let mut out = stamp.markdown();
    out.push_str("# Traceability index\n\n");
    out.push_str(
        "An identifier is listed against the heading that **declares** it. A heading declares an \
         identifier if the identifier is backticked in it, or if the heading text begins with it; \
         a bare, non-initial identifier in a heading is a reference. A heading declaring a range \
         declares its members weakly, and a later single-identifier heading supersedes that.\n\n\
         Identifiers cited but never declared are listed under *Referenced but never declared* \
         rather than attributed to their first mention.\n\n",
    );

    let _ = writeln!(out, "## Declared identifiers\n");
    for (id, declaration) in &index.declared {
        let _ = write!(
            out,
            "- `{id}` — owned by *{}* (line {})",
            declaration.heading, declaration.line
        );
        if declaration.weak {
            out.push_str(" — declared as a range member");
        }
        if let Some(references) = index.references.get(id) {
            let names: Vec<String> = references
                .iter()
                .take(3)
                .map(|reference| format!("*{}*", reference.heading))
                .collect();
            let _ = write!(out, " — also referenced in: {}", names.join("; "));
            if references.len() > 3 {
                let _ = write!(out, "; +{} more", references.len() - 3);
            }
        }
        out.push('\n');
    }

    let _ = writeln!(out, "\n## Referenced but never declared\n");
    if index.undeclared.is_empty() {
        out.push_str("None. Every cited identifier has an owning heading.\n");
    } else {
        out.push_str(
            "Each of these MUST either receive an owning contract or be removed before \
             `PH-AUTH-002` can claim that every canonical identifier is indexable exactly once.\n\n",
        );
        for id in &index.undeclared {
            let where_cited: Vec<String> = index
                .references
                .get(id)
                .map(|refs| {
                    refs.iter()
                        .take(4)
                        .map(|r| format!("*{}* (line {})", r.heading, r.line))
                        .collect()
                })
                .unwrap_or_default();
            let _ = writeln!(out, "- `{id}` — cited in: {}", where_cited.join("; "));
        }
    }

    let _ = writeln!(out, "\n## Retired v0.5 identifiers cited as history\n");
    if index.retired_v05.is_empty() {
        out.push_str("None.\n");
    } else {
        out.push_str(
            "These belong to the frozen v0.5 authority. They appear only as migration history \
             and MUST NOT be treated as live contracts or re-entered into v1 registries.\n\n",
        );
        for id in &index.retired_v05 {
            let where_cited: Vec<String> = index
                .references
                .get(id)
                .map(|refs| {
                    refs.iter()
                        .take(3)
                        .map(|r| format!("*{}* (line {})", r.heading, r.line))
                        .collect()
                })
                .unwrap_or_default();
            let _ = writeln!(out, "- `{id}` — cited in: {}", where_cited.join("; "));
        }
    }

    let _ = writeln!(out, "\n## Identifiers declared by more than one heading\n");
    if index.multiply_declared.is_empty() {
        out.push_str("None. No identifier is declared by two headings.\n");
    } else {
        for (id, lines) in &index.multiply_declared {
            let rendered: Vec<String> = lines.iter().map(usize::to_string).collect();
            let _ = writeln!(out, "- `{id}` — declared at lines {}", rendered.join(", "));
        }
    }

    let _ = write!(
        out,
        "\n**Index totals:** {} declared, {} undeclared, {} multiply-declared, {} retired-v0.5, \
         {} identifier families.\n",
        index.declared_count(),
        index.undeclared.len(),
        index.multiply_declared.len(),
        index.retired_v05.len(),
        index.families.len()
    );
    out
}

fn identifiers_json(index: &Index, stamp: &Stamp) -> String {
    let mut out = format!("{{\n{},\n  \"identifiers\": [\n", stamp.json_fields());
    let mut first = true;
    for (id, declaration) in &index.declared {
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let label = declaration.maturity_label.as_ref().map_or_else(
            || "null".to_string(),
            |value| format!("\"{}\"", escape(value)),
        );
        let references: Vec<String> = index
            .references
            .get(id)
            .map(|refs| {
                refs.iter()
                    .map(|r| {
                        format!(
                            "{{ \"heading\": \"{}\", \"line\": {} }}",
                            escape(&r.heading),
                            r.line
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let _ = write!(
            out,
            concat!(
                "    {{\n",
                "      \"id\": \"{id}\",\n",
                "      \"family\": \"{family}\",\n",
                "      \"declared_by\": \"{heading}\",\n",
                "      \"line\": {line},\n",
                "      \"range_member\": {weak},\n",
                "      \"maturity_label\": {label},\n",
                "      \"references\": [{references}]\n",
                "    }}"
            ),
            id = id,
            family = scan::family(id),
            heading = escape(&declaration.heading),
            line = declaration.line,
            weak = declaration.weak,
            label = label,
            references = references.join(", ")
        );
    }
    out.push_str("\n  ]\n}\n");
    out
}

fn requirements_json(index: &Index, stamp: &Stamp) -> String {
    let mut by_label: BTreeMap<&str, usize> = BTreeMap::new();
    for declaration in index.declared.values() {
        if let Some(label) = &declaration.maturity_label {
            *by_label.entry(label.as_str()).or_default() += 1;
        }
    }
    let mut out = format!("{{\n{},\n", stamp.json_fields());
    out.push_str(
        "  \"note\": \"maturity_label is the verbatim heading suffix. It is NOT normalised: \
         section 0.3 defines six labels while headings use many more phrases, and mapping \
         between them is PH-AUTH-003 status-axis scope.\",\n  \"label_counts\": {\n",
    );
    let mut first = true;
    for (label, count) in &by_label {
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let _ = write!(out, "    \"{}\": {count}", escape(label));
    }
    out.push_str("\n  },\n  \"requirements\": [\n");
    let mut first = true;
    for (id, declaration) in &index.declared {
        let Some(label) = &declaration.maturity_label else {
            continue;
        };
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let _ = write!(
            out,
            "    {{ \"id\": \"{id}\", \"heading\": \"{}\", \"line\": {}, \"maturity_label\": \"{}\" }}",
            escape(&declaration.heading),
            declaration.line,
            escape(label)
        );
    }
    out.push_str("\n  ]\n}\n");
    out
}

fn phases_json(phases: &[Phase], divergences: &[Divergence], stamp: &Stamp) -> String {
    let mut out = format!("{{\n{},\n", stamp.json_fields());
    out.push_str(
        "  \"source\": \"Derived from the prose phase cards, not from the Appendix G JSON fence. Appendix G declares itself subordinate: a serialization defect conflicting with a prose card loses until the registry is regenerated.\",\n  \"appendix_g_divergences\": [\n",
    );
    for (position, divergence) in divergences.iter().enumerate() {
        if position > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"phase\": \"{}\", \"field\": \"{}\" }}",
            divergence.phase, divergence.field
        );
    }
    out.push_str("\n  ],\n  \"phases\": [\n");
    for (position, phase) in phases.iter().enumerate() {
        if position > 0 {
            out.push_str(",\n");
        }
        let depends: Vec<String> = phase
            .depends_on
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect();
        let _ = write!(
            out,
            concat!(
                "    {{\n",
                "      \"id\": \"{id}\",\n",
                "      \"title\": \"{title}\",\n",
                "      \"line\": {line},\n",
                "      \"gate\": \"{gate}\",\n",
                "      \"depends_on\": [{depends}]"
            ),
            id = phase.id,
            title = escape(&phase.title),
            line = phase.line,
            gate = escape(&phase.gate),
            depends = depends.join(", ")
        );
        for (key, value) in &phase.fields {
            let _ = write!(out, ",\n      \"{key}\": \"{}\"", escape(value));
        }
        out.push_str("\n    }");
    }
    out.push_str("\n  ]\n}\n");
    out
}

fn research_gates_json(gates: &[ResearchGate], stamp: &Stamp) -> String {
    let mut out = format!("{{\n{},\n", stamp.json_fields());
    out.push_str(
        "  \"identity\": \"Many research gates carry no identifier, so identity is the heading text plus its line. Gates without a stable identifier are recorded as such rather than assigned one.\",\n  \"gates\": [\n",
    );
    for (position, gate) in gates.iter().enumerate() {
        if position > 0 {
            out.push_str(",\n");
        }
        let ids: Vec<String> = gate
            .identifiers
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect();
        let _ = write!(
            out,
            "    {{ \"heading\": \"{}\", \"line\": {}, \"label\": \"{}\", \"identifiers\": [{}] }}",
            escape(&gate.heading),
            gate.line,
            escape(&gate.label),
            ids.join(", ")
        );
    }
    out.push_str("\n  ]\n}\n");
    out
}

/// Build every projection from one source text.
pub fn all(source: &str, index: &Index, stamp: &Stamp) -> Result<Vec<Projection>, String> {
    let phases = super::phases::parse(source);
    let divergences = super::phases::reconcile_with_appendix_g(source, &phases)?;
    let gates = super::phases::research_gates(source);
    Ok(vec![
        Projection {
            relative_path: "governance/generated/index.md".to_string(),
            contents: index_markdown(index, stamp),
        },
        Projection {
            relative_path: "governance/generated/identifiers.json".to_string(),
            contents: identifiers_json(index, stamp),
        },
        Projection {
            relative_path: "governance/generated/requirements.json".to_string(),
            contents: requirements_json(index, stamp),
        },
        Projection {
            relative_path: "governance/generated/phases.json".to_string(),
            contents: phases_json(&phases, &divergences, stamp),
        },
        Projection {
            relative_path: "governance/generated/research-gates.json".to_string(),
            contents: research_gates_json(&gates, stamp),
        },
    ])
}

/// The manifest, which hashes every other projection.
pub fn manifest(projections: &[Projection], stamp: &Stamp) -> Projection {
    let mut out = format!("{{\n{},\n  \"projections\": [\n", stamp.json_fields());
    for (position, projection) in projections.iter().enumerate() {
        if position > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "    {{ \"path\": \"{}\", \"sha256\": \"{}\" }}",
            projection.relative_path,
            super::sha256::hex(projection.contents.as_bytes())
        );
    }
    out.push_str("\n  ]\n}\n");
    Projection {
        relative_path: "governance/manifest.json".to_string(),
        contents: out,
    }
}
