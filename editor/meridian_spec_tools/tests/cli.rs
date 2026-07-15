use std::path::Path;
use std::process::Command;

fn run(root: impl AsRef<Path>, args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_meridian-spec"))
        .args(args)
        .arg("--root")
        .arg(root.as_ref())
        .output()
        .expect("run meridian-spec");
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text)
}

#[test]
fn duplicate_ids_are_rejected() {
    let (ok, output) = run("tests/fixtures/duplicate_ids", &["validate", "docs"]);
    assert!(!ok, "{output}");
    assert!(output.contains("duplicate-id"), "{output}");
}

#[test]
fn missing_requirement_ids_are_rejected() {
    let (ok, output) = run("tests/fixtures/missing_ids", &["validate", "docs"]);
    assert!(!ok, "{output}");
    assert!(output.contains("missing-id"), "{output}");
}

#[test]
fn broken_links_and_fences_are_rejected() {
    let (ok, output) = run("tests/fixtures/link_fence", &["validate", "docs"]);
    assert!(!ok, "{output}");
    assert!(output.contains("broken-link"), "{output}");
    assert!(output.contains("bad-fence"), "{output}");
}

#[test]
fn unknown_statuses_are_rejected() {
    let (ok, output) = run("tests/fixtures/statuses", &["validate", "maturity"]);
    assert!(!ok, "{output}");
    assert!(output.contains("bad-status"), "{output}");
}

#[test]
fn missing_adrs_are_rejected() {
    let (ok, output) = run("tests/fixtures/adr_absence", &["validate", "adrs"]);
    assert!(!ok, "{output}");
    assert!(output.contains("missing-adr"), "{output}");
}

#[test]
fn expired_waivers_are_rejected() {
    let (ok, output) = run("tests/fixtures/expired_waiver", &["validate", "maturity"]);
    assert!(!ok, "{output}");
    assert!(output.contains("expired-waiver"), "{output}");
}

#[test]
fn bad_schemas_are_rejected() {
    let (ok, output) = run("tests/fixtures/bad_schema", &["validate", "schemas"]);
    assert!(!ok, "{output}");
    assert!(output.contains("bad-schema"), "{output}");
}

#[test]
fn stale_phase_refs_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/stale_phase_refs",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("stale-phase-ref"), "{output}");
}

#[test]
fn stale_current_suite_versions_are_rejected() {
    let (ok, output) = run("tests/fixtures/stale_suite_version", &["validate", "docs"]);
    assert!(!ok, "{output}");
    assert!(output.contains("stale-current-version"), "{output}");
}

#[test]
fn stale_v04_suite_versions_are_rejected() {
    let (ok, output) = run("tests/fixtures/stale_v04_version", &["validate", "docs"]);
    assert!(!ok, "{output}");
    assert!(output.contains("stale-current-version"), "{output}");
}

#[test]
fn post_one_programs_cannot_leak_into_milestones() {
    let (ok, output) = run(
        "tests/fixtures/program_milestone_leak",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("program-milestone-leak"), "{output}");
}

#[test]
fn missing_validation_projects_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/missing_validation_project",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("missing-validation-project"), "{output}");
}

#[test]
fn missing_dependency_strategy_records_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/missing_dependency_strategy",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("missing-dependency-strategy"), "{output}");
}

#[test]
fn missing_v05_migration_ledger_is_rejected() {
    let (ok, output) = run("tests/fixtures/missing_v05_ledger", &["list-unmapped"]);
    assert!(!ok, "{output}");
    assert!(output.contains("missing-migration-ledger"), "{output}");
}

#[test]
fn incomplete_or_cyclic_delivery_plans_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/bad_delivery_plan",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(
        output.contains("missing-delivery-plan-milestone"),
        "{output}"
    );
    assert!(output.contains("broken-critical-path"), "{output}");
    assert!(output.contains("work-package-cycle"), "{output}");
    assert!(output.contains("orphan-work-package"), "{output}");
}

#[test]
fn invalid_alluvium_package_dependencies_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/invalid_prc_dependency",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("invalid-package-dependency"), "{output}");
}

#[test]
fn orphan_requirements_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/orphan_requirements",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("orphan-requirement"), "{output}");
}

#[test]
fn orphan_alluvium_ids_are_rejected() {
    let (ok, output) = run("tests/fixtures/orphan_prc_id", &["validate", "workloads"]);
    assert!(!ok, "{output}");
    assert!(output.contains("REQ-PRC-999"), "{output}");
    assert!(output.contains("orphan-requirement"), "{output}");
}

#[test]
fn missing_alluvium_report_provenance_is_rejected() {
    let (ok, output) = run(
        "tests/fixtures/missing_alluvium_report_fields",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(
        output.contains("missing-recipe-provenance-field"),
        "{output}"
    );
}

#[test]
fn private_workload_payload_fields_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/private_workload_leak",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("private-content-field"), "{output}");
}

#[test]
fn false_promotions_are_rejected() {
    let (ok, output) = run("tests/fixtures/false_promotion", &["validate", "maturity"]);
    assert!(!ok, "{output}");
    assert!(output.contains("false-promotion"), "{output}");
}

#[test]
fn implemented_without_evidence_is_rejected() {
    let (ok, output) = run(
        "tests/fixtures/implemented_without_evidence",
        &["validate", "evidence"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("implemented-without-evidence"), "{output}");
}

#[test]
fn occluded_visual_evidence_is_rejected() {
    let (ok, output) = run(
        "tests/fixtures/occluded_visual_evidence",
        &["validate", "evidence", "--output=json"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("occluded-visual-evidence"), "{output}");
    assert!(output.contains("\"issues\""), "{output}");
}

#[test]
fn check_command_and_github_output_are_supported() {
    let (ok, output) = run("tests/fixtures/clean", &["check"]);
    assert!(ok, "{output}");
    assert!(output.contains("ok"), "{output}");

    let (ok, output) = run(
        "tests/fixtures/duplicate_ids",
        &["validate", "docs", "--output=github"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("::error"), "{output}");
}

#[test]
fn list_unmapped_and_explain_use_registry_ids() {
    let (ok, output) = run("tests/fixtures/orphan_requirements", &["list-unmapped"]);
    assert!(!ok, "{output}");
    assert!(output.contains("unmapped-id"), "{output}");

    let (ok, output) = run(
        "tests/fixtures/orphan_requirements",
        &["explain", "REQ-999"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("unknown-id"), "{output}");
}
