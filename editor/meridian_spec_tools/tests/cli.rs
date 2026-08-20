use std::path::Path;
use std::process::Command;

fn marquee_fixture_is_valid(name: &str) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            root.join("../../schemas/governance/marquee-validation-fixture.schema.json"),
        )
        .expect("read Marquee fixture schema"),
    )
    .expect("parse Marquee fixture schema");
    let instance: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/fixtures/marquee_cases").join(name))
            .expect("read Marquee fixture"),
    )
    .expect("parse Marquee fixture");
    jsonschema::validator_for(&schema)
        .expect("compile Marquee fixture schema")
        .validate(&instance)
        .is_ok()
}

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
fn ignored_website_tree_is_not_a_documentation_source() {
    let (ok, output) = run("tests/fixtures/ignored_website", &["validate", "docs"]);
    assert!(ok, "{output}");
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
fn closed_waivers_do_not_expire() {
    let (ok, output) = run("tests/fixtures/closed_waiver", &["validate", "maturity"]);
    assert!(ok, "{output}");
    assert!(!output.contains("expired-waiver"), "{output}");
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
fn ui_contracts_require_all_three_registries() {
    let (ok, output) = run(
        "tests/fixtures/ui_missing_registries",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("missing-ui-design-tokens"), "{output}");
    assert!(output.contains("missing-ui-components"), "{output}");
    assert!(output.contains("missing-ui-workspaces"), "{output}");
}

#[test]
fn ui_contracts_reject_palette_and_component_drift() {
    let (ok, output) = run(
        "tests/fixtures/ui_invalid_contracts",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("invalid-ui-token"), "{output}");
    assert!(output.contains("incomplete-ui-components"), "{output}");
    assert!(output.contains("incomplete-ui-workspaces"), "{output}");
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

#[test]
fn valid_marquee_public_fixture_is_accepted() {
    assert!(marquee_fixture_is_valid("valid.json"));
}

#[test]
fn invalid_marquee_policy_and_evidence_fixtures_are_rejected() {
    for name in [
        "invalid_ai_capture_publish.json",
        "invalid_approval_stale.json",
        "invalid_source_private.json",
        "invalid_draft_mapping.json",
    ] {
        assert!(
            !marquee_fixture_is_valid(name),
            "{name} unexpectedly passed"
        );
    }
}

#[test]
fn marquee_programs_cannot_leak_into_milestones() {
    let (ok, output) = run(
        "tests/fixtures/marquee_program_milestone",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("program-milestone-leak"), "{output}");
}

#[test]
fn marquee_requirements_need_the_registered_program() {
    let (ok, output) = run(
        "tests/fixtures/marquee_missing_program",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("orphan-requirement"), "{output}");
    assert!(output.contains("PRG-PRM-001"), "{output}");
}

#[test]
fn marquee_packages_cannot_activate_before_post_one_review() {
    let (ok, output) = run(
        "tests/fixtures/marquee_premature_package",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("premature-marquee-package"), "{output}");
}

#[test]
fn marquee_policy_requires_every_approval_invalidator() {
    let (ok, output) = run(
        "tests/fixtures/marquee_incomplete_invalidation",
        &["validate", "workloads"],
    );
    assert!(!ok, "{output}");
    assert!(
        output.contains("incomplete-marquee-approval-invalidation"),
        "{output}"
    );
}

#[test]
fn duplicate_marquee_maturity_records_are_rejected() {
    let (ok, output) = run(
        "tests/fixtures/marquee_duplicate_maturity",
        &["validate", "maturity"],
    );
    assert!(!ok, "{output}");
    assert!(output.contains("duplicate-maturity"), "{output}");
}

#[test]
fn missing_marquee_maturity_record_is_rejected() {
    let (ok, output) = run(
        "tests/fixtures/marquee_missing_maturity",
        &["validate", "maturity"],
    );
    assert!(!ok, "{output}");
    assert!(
        output.contains("domain PRM has no maturity record"),
        "{output}"
    );
}

#[test]
fn missing_marquee_amendment_ledger_is_rejected() {
    let (ok, output) = run("tests/fixtures/marquee_missing_ledger", &["list-unmapped"]);
    assert!(!ok, "{output}");
    assert!(output.contains("missing-migration-ledger"), "{output}");
    assert!(output.contains("Marquee amendment ledger"), "{output}");
}
