use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[test]
fn public_creator_alpha_journey_is_reproducible() {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let project = repository_root.join("examples/creator-alpha");
    let source_before = fs::read(project.join("project.meridian.json"))
        .expect("public Creator source reads before smoke");
    let model_before = fs::read(project.join("models/public-box.model.json"))
        .expect("public Creator model reads before smoke");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let evidence = std::env::temp_dir().join(format!(
        "meridian-creator-alpha-smoke-{}-{nonce}",
        std::process::id()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_meridian"))
        .args([
            "--creator-alpha-smoke",
            "--project",
            project.to_str().expect("UTF-8 project path"),
            "--evidence",
            evidence.to_str().expect("UTF-8 evidence path"),
        ])
        .output()
        .expect("Creator Alpha process starts");
    assert!(
        output.status.success(),
        "Creator Alpha smoke failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(
        &fs::read(evidence.join("creator-alpha-evidence.json")).expect("evidence reads"),
    )
    .expect("evidence is JSON");
    assert_eq!(summary["outcome"], "LocalPass");
    assert_eq!(summary["source_persistence"], "pass");
    let journey = summary["journey"].as_array().expect("journey");
    for required in [
        "open",
        "import",
        "edit",
        "undo",
        "redo",
        "play-apply",
        "play-stop-discard",
        "reimport",
        "reopen",
        "recover",
        "build",
    ] {
        assert!(
            journey.iter().any(|step| step.as_str() == Some(required)),
            "Creator Alpha journey omitted {required}"
        );
    }
    assert_eq!(summary["procedural"]["generated_placement_count"], 3);
    assert_eq!(summary["procedural"]["license_audit"], "pass");
    assert_eq!(summary["modeler"]["topology_lineage"], "pass");
    assert_eq!(summary["modeler"]["override_migration"], "pass");
    assert_eq!(summary["modeler"]["semantic_undo_recovery"], "pass");
    assert_eq!(summary["modeler"]["semantic_inspector"], "pass");
    assert_eq!(summary["modeler"]["penumbra_preview"], "pass");
    assert_eq!(summary["modeler"]["preview_triangle_count"], 14);
    assert_eq!(summary["build"]["worker_count"], 1);
    assert!(summary["build"]["artifact_hash"].as_str().is_some());
    assert_eq!(
        fs::read(project.join("project.meridian.json"))
            .expect("public Creator source reads after smoke"),
        source_before
    );
    assert_eq!(
        fs::read(project.join("models/public-box.model.json"))
            .expect("public Creator model reads after smoke"),
        model_before
    );
    fs::remove_dir_all(evidence).expect("remove temporary evidence");
}
