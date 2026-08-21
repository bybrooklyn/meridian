//! Class (b) registries: accumulated state, not projections of the root.
//!
//! Evidence records, waivers, releases and compatibility windows are sourced from test runs,
//! approvals and shipped artefacts. A design document cannot contain who approved a waiver or
//! how a gate run ended, so none of it is derivable from `MERIDIAN_SPECOMENT.md`.
//!
//! **These are deliberately kept outside `emit::all()`.** `specoment::run` writes every
//! projection unconditionally, with no merge and no read-back, so emitting an evidence index
//! would overwrite accumulated records with an empty regeneration on the next `project` run
//! and then report the correctly-registered originals as hand-edited. That is `SD-011`.
//!
//! They are policed instead by conformance and completeness rules: does the file match its
//! declared shape, is the stamp current, and is every artefact present actually registered.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// A conformance failure, reported by path and cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub path: String,
    pub detail: String,
}

/// Files in the evidence directory that are not evidence artefacts.
const NOT_ARTEFACTS: &[&str] = &["index.json"];

/// Check the evidence index against Appendix H.4 and against the directory beside it.
///
/// Appendix H.4 mandates `{"schema": 1, "specoment_sha256": "...", "records": [...]}`.
pub fn check_evidence_index(root: &Path, canonical_sha256: &str) -> Vec<Problem> {
    let directory = root.join(".meridian/implementation/evidence");
    let index_path = directory.join("index.json");
    let display = ".meridian/implementation/evidence/index.json".to_string();
    let mut problems = Vec::new();

    let Ok(text) = fs::read_to_string(&index_path) else {
        return vec![Problem {
            path: display,
            detail: "evidence index is missing; Appendix H.4 requires one".to_string(),
        }];
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![Problem {
            path: display,
            detail: "evidence index is not valid JSON".to_string(),
        }];
    };

    if value.get("schema").is_none() {
        let hint = if value.get("schema_version").is_some() {
            " (found `schema_version`, which Appendix H.4 does not define)"
        } else {
            ""
        };
        problems.push(Problem {
            path: display.clone(),
            detail: format!("Appendix H.4 requires a `schema` field{hint}"),
        });
    }
    match value
        .get("specoment_sha256")
        .and_then(serde_json::Value::as_str)
    {
        None => problems.push(Problem {
            path: display.clone(),
            detail: "Appendix H.4 requires `specoment_sha256`; a projection or registry that \
                     cannot name its source revision cannot be shown to be current"
                .to_string(),
        }),
        Some(recorded) if recorded != canonical_sha256 => problems.push(Problem {
            path: display.clone(),
            detail: format!("stamps {recorded}, but the current specoment is {canonical_sha256}"),
        }),
        Some(_) => {}
    }

    // Shape, against the checked-in schema. Validating here rather than trusting the
    // hand-rolled field checks above is what proves the schema is live rather than inert.
    let schema_path = root.join("governance/schemas/evidence-index.schema.json");
    if let Ok(schema_text) = fs::read_to_string(&schema_path) {
        if let Ok(schema) = serde_json::from_str::<serde_json::Value>(&schema_text) {
            match jsonschema::validator_for(&schema) {
                Ok(validator) => {
                    for error in validator.iter_errors(&value) {
                        problems.push(Problem {
                            path: display.clone(),
                            detail: format!("schema: {error}"),
                        });
                    }
                }
                Err(error) => problems.push(Problem {
                    path: "governance/schemas/evidence-index.schema.json".to_string(),
                    detail: format!("schema does not compile: {error}"),
                }),
            }
        }
    }

    problems.extend(unregistered_artefacts(&directory, &value, &display));
    problems
}

/// Every artefact beside the index must be registered. `IMPL-WP-003` item 6.
fn unregistered_artefacts(
    directory: &Path,
    value: &serde_json::Value,
    display: &str,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let registered: BTreeSet<String> = value
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(|records| {
            records
                .iter()
                .flat_map(|record| {
                    record
                        .get("artifacts")
                        .and_then(serde_json::Value::as_array)
                        .map(|list| {
                            list.iter()
                                .filter_map(|a| a.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    let mut present: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if NOT_ARTEFACTS.contains(&name.as_str()) {
                continue;
            }
            present.push(name);
        }
    }
    present.sort();

    let unregistered: Vec<&String> = present
        .iter()
        .filter(|name| !registered.contains(*name))
        .collect();
    if !unregistered.is_empty() {
        problems.push(Problem {
            path: display.to_string(),
            detail: format!(
                "{} of {} artefacts are unregistered, against IMPL-WP-003 item 6: {}",
                unregistered.len(),
                present.len(),
                unregistered
                    .iter()
                    .take(4)
                    .map(|n| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    problems
}

/// Paths where citing v0.5 is legitimate rather than an active declaration.
///
/// A migration record naming what it migrated away from is history, not authority — the v0.5
/// validator exempted the same path for the same reason. The preservation ledger and the
/// implementation bookkeeping are likewise records *about* the retirement.
const V05_CITATION_EXEMPT: &[&str] = &[
    "docs/migrations/",
    ".meridian/",
    "governance/",
    "MERIDIAN_SPECOMENT.md",
    "editor/meridian_spec_tools/tests/fixtures/",
    // The retained v0.5 validator's own source. Its comments necessarily name the authority
    // it policed, and this rule's pattern table necessarily contains the patterns it matches.
    // Code describing a retired rule is not a file declaring that authority — but that is only
    // defensible because `check-v05` refuses to run once `specs/` is gone, so the retained code
    // cannot judge v1 content. SPEC-001 forbids the two authorities competing; retention
    // without that gate would be exactly the competition, running backwards.
    "editor/meridian_spec_tools/src/",
];

/// Patterns that declare v0.5 authority as live, rather than citing it as history.
const V05_DECLARATIONS: &[(&str, &str)] = &[
    (
        "Spec version: v0.",
        "an ADR header declaring the version it refines",
    ),
    ("version 0.5", "a document header declaring v0.5 authority"),
    (
        "specs/registry",
        "a path under the retired v0.5 registry tree",
    ),
    (
        "specs/MERIDIAN_",
        "a path under the retired v0.5 specification tree",
    ),
    (
        "IMPLEMENTATION_PLANNING_SPEC.md",
        "a retired v0.5 specification",
    ),
    ("DELIVERY_ROADMAP.md", "a retired v0.5 specification"),
];

/// Files that still declare v0.5 authority after the cutover.
///
/// `PH-AUTH-004`'s headline closure row is "No active file declares v0.5 authority." An
/// earlier pass asserted that row with no rule behind it, and it was false: fifteen ADRs
/// still carried `Spec version: v0.5` at authority rank 2, seventeen benchmark headers still
/// declared v0.5, and a `sed` pass over links had moved the count not at all. The row is
/// only as good as the check that enforces it.
pub fn v05_declarations(root: &Path, tracked: &[String]) -> Vec<Problem> {
    let mut problems = Vec::new();
    for relative in tracked {
        if V05_CITATION_EXEMPT
            .iter()
            .any(|exempt| relative.starts_with(exempt))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            continue;
        };
        for (pattern, why) in V05_DECLARATIONS {
            if let Some(at) = text.find(pattern) {
                let line = text[..at].lines().count();
                problems.push(Problem {
                    path: relative.clone(),
                    detail: format!("line {line}: {why} (`{pattern}`)"),
                });
                break;
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::check_evidence_index;
    use std::fs;

    fn fixture(index: &str) -> tempdir::Dir {
        let dir = tempdir::Dir::new();
        let evidence = dir.path().join(".meridian/implementation/evidence");
        fs::create_dir_all(&evidence).expect("create fixture");
        fs::write(evidence.join("index.json"), index).expect("write index");
        fs::write(evidence.join("some-run.log"), "log").expect("write artefact");
        dir
    }

    #[test]
    fn the_misnamed_schema_key_is_reported_with_a_hint() {
        let dir = fixture(r#"{"schema_version": 1, "records": []}"#);
        let problems = check_evidence_index(dir.path(), "abc");
        assert!(
            problems.iter().any(|p| p.detail.contains("schema_version")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_missing_source_digest_is_reported() {
        let dir = fixture(r#"{"schema": 1, "records": []}"#);
        let problems = check_evidence_index(dir.path(), "abc");
        assert!(
            problems
                .iter()
                .any(|p| p.detail.contains("specoment_sha256")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_stale_digest_is_reported_with_both_values() {
        let dir = fixture(r#"{"schema": 1, "specoment_sha256": "old", "records": []}"#);
        let problems = check_evidence_index(dir.path(), "new");
        assert!(
            problems.iter().any(|p| p.detail.contains("old")),
            "{problems:?}"
        );
    }

    /// The live defect: zero records beside real artefacts.
    #[test]
    fn unregistered_artefacts_are_reported() {
        let dir = fixture(r#"{"schema": 1, "specoment_sha256": "abc", "records": []}"#);
        let problems = check_evidence_index(dir.path(), "abc");
        assert!(
            problems.iter().any(|p| p.detail.contains("unregistered")),
            "{problems:?}"
        );
    }

    /// Proves the schema is live rather than inert. `jsonschema` is pinned
    /// `default-features = false`, which disables cross-file `$ref` resolution; a schema
    /// factored across files would silently never load and every rule resting on it would
    /// be unfailable. Same-document `$defs` are unaffected, and this asserts it.
    #[test]
    fn the_schema_rejects_an_index_missing_its_source_digest() {
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../governance/schemas/evidence-index.schema.json"),
            )
            .expect("schema is checked in"),
        )
        .expect("schema parses");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");

        let missing: serde_json::Value =
            serde_json::from_str(r#"{"schema": 1, "records": []}"#).expect("instance parses");
        assert!(
            validator.validate(&missing).is_err(),
            "a schema that accepts an index with no source digest is inert"
        );

        let ok: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"schema": 1, "specoment_sha256": "{}", "records": []}}"#,
            "a".repeat(64)
        ))
        .expect("instance parses");
        assert!(
            validator.validate(&ok).is_ok(),
            "a conforming index must pass"
        );
    }

    /// `PH-AUTH-004`'s headline closure row. An earlier pass asserted it with no rule behind
    /// it and it was false: fifteen ADRs still carried `Spec version: v0.5` at authority
    /// rank 2, and a link-rewriting pass had moved the count not at all.
    #[test]
    fn a_file_declaring_v05_authority_is_reported() {
        let dir = tempdir::Dir::new();
        std::fs::create_dir_all(dir.path().join("docs/architecture")).expect("fixture");
        std::fs::write(
            dir.path().join("docs/architecture/adr.md"),
            "# ADR\n- Spec version: v0.5\n",
        )
        .expect("write");
        let problems = super::v05_declarations(dir.path(), &["docs/architecture/adr.md".into()]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].detail.contains("Spec version"), "{problems:?}");
    }

    /// A migration record naming what it migrated away from is history, not authority. The
    /// v0.5 validator exempted the same path for the same reason.
    #[test]
    fn a_migration_record_citing_v05_is_not_a_declaration() {
        let dir = tempdir::Dir::new();
        std::fs::create_dir_all(dir.path().join("docs/migrations")).expect("fixture");
        std::fs::write(
            dir.path().join("docs/migrations/ledger.md"),
            "Retired the version 0.5 suite and specs/registry at PH-AUTH-004.\n",
        )
        .expect("write");
        assert!(
            super::v05_declarations(dir.path(), &["docs/migrations/ledger.md".into()]).is_empty()
        );
    }

    #[test]
    fn a_conforming_and_complete_index_reports_nothing() {
        let dir = fixture(
            r#"{"schema": 1, "specoment_sha256": "abc",
                "records": [{"id": "EV-1", "artifacts": ["some-run.log"]}]}"#,
        );
        assert_eq!(check_evidence_index(dir.path(), "abc"), Vec::new());
    }

    /// Minimal scoped temporary directory, removed on drop. The workspace has no tempfile
    /// dependency and LEGAL-005 would require a provenance record to add one for four tests.
    mod tempdir {
        use std::path::{Path, PathBuf};
        pub struct Dir(PathBuf);

        /// Monotonic, not clock-based. Two tests created in the same nanosecond would
        /// otherwise share a directory and read each other's fixture — which is exactly what
        /// happened, and it presented as one test failing for another test's reason.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        impl Dir {
            pub fn new() -> Self {
                let ordinal = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let base = std::env::temp_dir()
                    .join(format!("meridian-spec-{}-{ordinal}", std::process::id()));
                std::fs::create_dir_all(&base).expect("create temp dir");
                Self(base)
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
