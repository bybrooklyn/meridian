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
fn v1_staging_authority_is_not_scanned() {
    let (_, output) = run("tests/fixtures/v1_staging", &["list-unmapped"]);
    assert!(
        !output.contains("PRG-RECON-001"),
        "root MERIDIAN_SPECOMENT.md must be outside v0.5 scope: {output}"
    );
    assert!(
        !output.contains("VAL-PORTFOLIO-001"),
        ".meridian/ must be outside v0.5 scope: {output}"
    );
    assert!(
        output.contains("REQ-999"),
        "genuine v0.5 documents must still be reported: {output}"
    );
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

#[test]
fn generated_governance_projections_are_not_scanned() {
    // `governance/` holds v1 projections generated from the root specoment. A v0.5
    // validator judging them is SPEC-001's forbidden competition running backwards.
    // The index legitimately cites drafting ledgers ("spec-rewrite v0.22"), which
    // `has_retired_reference` matches on the substring "v0.2".
    let (_, output) = run(
        "tests/fixtures/governance_projection",
        &["validate", "docs"],
    );
    assert!(
        !output.contains("governance/generated/index.md"),
        "generated v1 projections must be outside v0.5 scope: {output}"
    );

    let (_, unmapped) = run("tests/fixtures/governance_projection", &["list-unmapped"]);
    assert!(
        !unmapped.contains("PRG-RECON-001"),
        "v1 identifiers in projections must not be reported: {unmapped}"
    );
    assert!(
        unmapped.contains("REQ-999"),
        "genuine v0.5 documents must still be reported: {unmapped}"
    );
}

/// The repository root, from the crate directory.
fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Tests that read or write the real `governance/` projections share mutable state on
/// disk, and Rust runs tests in parallel by default. Without this lock the tampering test
/// races the determinism test and both fail intermittently for the wrong reason.
static PROJECTIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn projection_guard() -> std::sync::MutexGuard<'static, ()> {
    PROJECTIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn projections_regenerate_byte_identically() {
    let _guard = projection_guard();
    let root = repo_root();
    let files = [
        "governance/generated/index.md",
        "governance/generated/identifiers.json",
        "governance/generated/requirements.json",
        "governance/manifest.json",
    ];
    let before: Vec<String> = files
        .iter()
        .map(|name| std::fs::read_to_string(root.join(name)).expect("projection exists"))
        .collect();
    let (ok, output) = run(&root, &["project"]);
    assert!(ok, "{output}");
    for (name, original) in files.iter().zip(&before) {
        let regenerated = std::fs::read_to_string(root.join(name)).expect("projection exists");
        assert_eq!(&regenerated, original, "{name} is not deterministic");
    }
}

#[test]
fn project_check_passes_against_current_projections() {
    let _guard = projection_guard();
    let (ok, output) = run(repo_root(), &["project", "--check"]);
    assert!(
        ok,
        "project --check must pass on committed projections: {output}"
    );
    assert!(!output.contains("stale"), "{output}");
    assert!(!output.contains("missing"), "{output}");
}

/// The reconciliation must name the offending file, not merely fail.
#[test]
fn project_check_names_a_hand_edited_projection() {
    let _guard = projection_guard();
    let root = repo_root();
    let target = root.join("governance/generated/index.md");
    let original = std::fs::read_to_string(&target).expect("projection exists");

    let mut tampered = original.clone();
    tampered.push_str("\nhand-edited\n");
    std::fs::write(&target, &tampered).expect("write tampered projection");
    let (_, output) = run(&root, &["project", "--check"]);
    std::fs::write(&target, &original).expect("restore projection");

    assert!(
        output.contains("governance/generated/index.md"),
        "reconciliation must name the divergent file: {output}"
    );
}

/// The invariant whose absence let a 31-identifier gap hide inside a total that read clean:
/// the reference generator reported 731 declared while emitting only 700 entries.
#[test]
fn every_declared_identifier_is_indexed_exactly_once() {
    let _guard = projection_guard();
    let index = std::fs::read_to_string(repo_root().join("governance/generated/index.md"))
        .expect("index exists");
    let section = index
        .split("## Declared identifiers")
        .nth(1)
        .and_then(|rest| rest.split("## Referenced but never declared").next())
        .expect("declared section");

    let mut ids: Vec<&str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("- `"))
        .filter_map(|rest| rest.split('`').next())
        .collect();
    let emitted = ids.len();
    ids.sort_unstable();
    let before_dedup = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before_dedup, "an identifier is indexed twice");

    let total: usize = index
        .split("**Index totals:** ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|value| value.parse().ok())
        .expect("totals line");
    assert_eq!(
        emitted, total,
        "the totals line must equal the number of entries actually emitted"
    );
}

/// Defect 4 in the specoment corpus itself: all five letter-suffixed identifiers must
/// survive as distinct identities rather than collapsing onto their stems.
#[test]
fn letter_suffixed_identifiers_survive_in_the_real_index() {
    let _guard = projection_guard();
    let index = std::fs::read_to_string(repo_root().join("governance/generated/index.md"))
        .expect("index exists");
    for id in [
        "NETPROJ-006A",
        "NETPROJ-006B",
        "NETPROJ-006C",
        "NETPROJ-006D",
        "SCM-010A",
        "NETPROJ-006",
        "SCM-010",
    ] {
        assert!(
            index.contains(&format!("- `{id}` —")),
            "{id} must be indexed as its own identity"
        );
    }
}

/// Defect 3 in the corpus: five whole families were counted but never emitted.
#[test]
fn range_declared_families_reach_the_index() {
    let _guard = projection_guard();
    let index = std::fs::read_to_string(repo_root().join("governance/generated/index.md"))
        .expect("index exists");
    for id in [
        "NORM-MIG-001",
        "NORM-MIG-012",
        "AI-POLICY-001",
        "AI-POLICY-008",
        "CODEHEALTH-001",
        "OPEN-004",
        "FWK-003",
    ] {
        assert!(
            index.contains(&format!("- `{id}` —")),
            "{id} was counted but never indexed by the reference generator"
        );
    }
}

/// Appendix H.5 mandates four stamp fields on every generated projection.
#[test]
fn every_projection_carries_the_four_appendix_h5_fields() {
    let _guard = projection_guard();
    let root = repo_root();
    for name in [
        "governance/generated/index.md",
        "governance/generated/identifiers.json",
        "governance/generated/requirements.json",
        "governance/manifest.json",
    ] {
        let text = std::fs::read_to_string(root.join(name)).expect("projection exists");
        for field in [
            "canonical_path",
            "canonical_sha256",
            "generator_version",
            "generated_at_source_checkpoint",
        ] {
            assert!(
                text.contains(field),
                "{name} is missing the {field} stamp field"
            );
        }
        // The stamp must carry the real digest, not a placeholder.
        assert!(
            text.contains("782d3110b89ac23fa3f8cf80c07a72ba15e9de457717ca918a14f24e6d32692a"),
            "{name} does not stamp the canonical specoment digest"
        );
    }
}
