use std::process::Command;

use serde_json::Value;

const FIRST_GENERATED_ID: &str = "efad7c2235daa5e23d9386551c5d8b3a";

fn project_recipe() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root")
        .join("examples/creator-alpha/recipes/public-placement.mproc")
        .to_string_lossy()
        .into_owned()
}

fn command(arguments: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_meridian"))
        .args(arguments)
        .output()
        .expect("Alluvium command starts");
    assert!(
        output.status.success(),
        "Alluvium command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("structured JSON result")
}

#[test]
fn all_public_alluvium_commands_share_the_recipe_contract() {
    let recipe = project_recipe();
    let inspect = command(&["alluvium", "inspect", &recipe]);
    assert_eq!(inspect["command"], "inspect");
    assert_eq!(command(&["alluvium", "validate", &recipe])["valid"], true);
    assert_eq!(
        command(&["alluvium", "migrate", &recipe, "--to", "1"])["command"],
        "migrate"
    );
    assert_eq!(
        command(&["alluvium", "preview", &recipe, "--region", "0,0,0:4000,0,0"])["result"]["field"]
            ["samples"]
            .as_array()
            .expect("samples")
            .len(),
        3
    );
    assert_eq!(
        command(&["alluvium", "bake", &recipe, "--profile", "public"])["license_audit"]["accepted"],
        true
    );
    assert_eq!(
        command(&["alluvium", "dirty", &recipe, "--since", &recipe])["report"]["dirty"],
        false
    );
    assert_eq!(
        command(&["alluvium", "diff", &recipe, "--against", &recipe])["equal"],
        true
    );
    assert!(command(&[
        "alluvium",
        "explain",
        &recipe,
        "--object",
        FIRST_GENERATED_ID
    ])["object"]
        .is_object());
    assert_eq!(
        command(&[
            "alluvium",
            "provenance",
            &recipe,
            "--output",
            FIRST_GENERATED_ID
        ])["provenance"]["license"],
        "CC0-1.0"
    );
    assert_eq!(
        command(&["alluvium", "license-audit", &recipe, "--target", "public"])["audit"]["accepted"],
        true
    );
}
