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
/// Returns the problems found. An empty vector means the projections are current.
///
/// `--check` never writes. It regenerates in memory and compares, naming every divergent
/// file. That comparison — not the stamped digest — is the enforcement mechanism, which is
/// why a defect in the hand-rolled digest fails closed.
///
/// The stamp is fully deterministic: regenerating without changing the specoment produces
/// byte-identical output, so the comparison needs no ignored fields and therefore has no
/// hole where a hand-edit goes undetected.
///
/// That required not stamping the repository HEAD. HEAD moves for reasons unrelated to the
/// specoment — including the very commit that lands these projections — so a HEAD-stamped
/// projection is stale the instant it is committed, and `--check` can never pass. A check
/// that can never pass trains people to ignore it, which is worse than not having one.
///
/// Appendix H.5 asks for `generated_at_source_checkpoint = <revision>`. In a single-root-
/// authority repository the revision *of the source* is the specoment itself, identified by
/// its content digest. That is what is stamped.
pub fn run(root: &Path, check_only: bool) -> Result<Vec<String>, String> {
    let source_path = root.join("MERIDIAN_SPECOMENT.md");
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("failed to read {}: {error}", source_path.display()))?;

    let canonical_sha256 = sha256::hex(source.as_bytes());
    let index = index::build(&source);
    let stamp = emit::Stamp {
        canonical_sha256: canonical_sha256.clone(),
        source_checkpoint: format!("specoment:{canonical_sha256}"),
    };

    let mut projections = emit::all(&source, &index, &stamp);
    projections.push(emit::manifest(&projections, &stamp));

    let mut problems = Vec::new();
    for projection in &projections {
        let target = root.join(&projection.relative_path);
        if check_only {
            match fs::read_to_string(&target) {
                Ok(existing) => {
                    if existing != projection.contents {
                        problems.push(format!(
                            "{} is stale or hand-edited; regenerate with `cargo run -p meridian-spec -- project`",
                            projection.relative_path
                        ));
                    } else if !existing.contains(&canonical_sha256) {
                        problems.push(format!(
                            "{} stamps a specoment digest that is not the current one",
                            projection.relative_path
                        ));
                    }
                }
                Err(_) => problems.push(format!("{} is missing", projection.relative_path)),
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
        println!(
            "wrote {} projections: {} declared, {} undeclared, {} multiply-declared, {} retired-v0.5, {} families",
            projections.len(),
            index.declared_count(),
            index.undeclared.len(),
            index.multiply_declared.len(),
            index.retired_v05.len(),
            index.families.len()
        );
    }
    Ok(problems)
}
