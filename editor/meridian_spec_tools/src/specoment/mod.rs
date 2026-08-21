//! Projection of the root `MERIDIAN_SPECOMENT.md` into generated, reconcilable views.
//!
//! Owning package: `WP-V1-RESET-002` under `PH-AUTH-002`.
//!
//! Appendix D governs every projection: it must carry the canonical hash, map each stable
//! identifier back to one canonical heading, be regenerable or reconciliation-checked,
//! never silently override canonical prose, preserve research/deferred status, preserve
//! zero-unmapped traceability, fail CI when misleadingly stale, and stay distinct from
//! user documentation. Appendix H.5 fixes the stamp at four fields.
//!
//! Nothing in this module interprets the specoment. It projects what the document states
//! and refuses to invent a status, a disposition or an identity the root file does not carry.

pub mod emit;
pub mod index;
pub mod scan;
pub mod sha256;

/// Stamped into every generated file. Bump when the output shape changes, so a stale
/// projection is distinguishable from a merely out-of-date one.
pub const GENERATOR_VERSION: &str = "specoment-projection/1";

/// The canonical authority path, as recorded in every projection stamp.
pub const CANONICAL_PATH: &str = "/MERIDIAN_SPECOMENT.md";

use std::fs;
use std::path::Path;

/// Generate every projection, or verify the checked-in ones match a fresh generation.
///
/// `--check` never writes. It regenerates in memory and compares byte for byte, naming the
/// first divergent file. That comparison — not the stamped digest — is the enforcement
/// mechanism, which is why a defect in the hand-rolled digest fails closed.
pub fn run(root: &Path, check_only: bool) -> Result<Vec<String>, String> {
    let source_path = root.join("MERIDIAN_SPECOMENT.md");
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("failed to read {}: {error}", source_path.display()))?;

    let index = index::build(&source);
    let stamp = emit::Stamp {
        canonical_sha256: sha256::hex(source.as_bytes()),
        source_checkpoint: source_checkpoint(root),
    };

    let mut projections = emit::all(&source, &index, &stamp);
    projections.push(emit::manifest(&projections, &stamp));

    let mut issues = Vec::new();
    for projection in &projections {
        let target = root.join(&projection.relative_path);
        if check_only {
            match fs::read_to_string(&target) {
                Ok(existing) if existing == projection.contents => {}
                Ok(_) => issues.push(format!(
                    "{} is stale or hand-edited; regenerate with `cargo run -p meridian-spec -- project`",
                    projection.relative_path
                )),
                Err(_) => issues.push(format!("{} is missing", projection.relative_path)),
            }
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        fs::write(&target, &projection.contents)
            .map_err(|error| format!("failed to write {}: {error}", target.display()))?;
    }

    if !check_only {
        issues.push(format!(
            "wrote {} projections: {} declared, {} undeclared, {} multiply-declared, {} retired-v0.5, {} families",
            projections.len(),
            index.declared_count(),
            index.undeclared.len(),
            index.multiply_declared.len(),
            index.retired_v05.len(),
            index.families.len()
        ));
    }
    Ok(issues)
}

/// The source checkpoint for the Appendix H.5 stamp. Read from `.git/HEAD` rather than by
/// running Git, since this tool takes no ambient process authority.
fn source_checkpoint(root: &Path) -> String {
    let head = root.join(".git/HEAD");
    let Ok(contents) = fs::read_to_string(&head) else {
        return "unknown".to_string();
    };
    let trimmed = contents.trim();
    let Some(reference) = trimmed.strip_prefix("ref: ") else {
        return trimmed.to_string();
    };
    fs::read_to_string(root.join(".git").join(reference))
        .map_or_else(|_| "unknown".to_string(), |value| value.trim().to_string())
}
