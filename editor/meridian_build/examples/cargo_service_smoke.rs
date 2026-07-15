//! Executes a structured local Cargo-check through the Meridian build service.

use std::error::Error;
use std::path::{Path, PathBuf};

use meridian_build::{
    hash_file_bounded, run_cargo_json, run_cargo_metadata, BuildCancellation, BuildIdentityInput,
    BuildNode, BuildNodeId, BuildPhase, BuildRequest, BuildService, CargoCommand, CargoEnvironment,
    CargoInvocation, CargoRunStatus,
};
use meridian_core::{OperationId, TraceId};

fn main() -> Result<(), Box<dyn Error>> {
    let workspace = workspace_root()?;
    let lock_hash = hash_file_bounded(&workspace.join("Cargo.lock"))?;
    let environment = CargoEnvironment::from_host();
    let metadata = run_cargo_metadata(
        &CargoInvocation::new(
            &workspace,
            CargoCommand::Metadata,
            Vec::new(),
            environment.clone(),
        )?,
        &BuildCancellation::default(),
    )?;
    let metadata_hash = match (metadata.status, metadata.content_hash) {
        (CargoRunStatus::Succeeded, Some(hash)) => hash,
        (CargoRunStatus::Failed(code), _) => {
            return Err(format!("cargo metadata failed with status {code:?}").into())
        }
        (CargoRunStatus::Cancelled, _) => return Err("cargo metadata was cancelled".into()),
        (CargoRunStatus::Succeeded, None) => {
            return Err("cargo metadata did not return a hash".into())
        }
    };
    let mut root = BuildNode::cargo_check(BuildNodeId::new("cargo-check")?, "workspace-cargo")?;
    root.declared_environment = environment.identity_values().into_keys().collect();
    let request = BuildRequest::new(
        &BuildIdentityInput {
            source_checkpoint: "local-cargo-service-smoke".to_owned(),
            resolved_profile: "check".to_owned(),
            cargo_metadata_and_lock: format!("metadata:{metadata_hash};lock:{lock_hash}"),
            toolchain_version: "workspace-cargo".to_owned(),
            target_and_capabilities: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            environment_allowlist: environment.identity_values(),
            root_node_ids: vec![root.id.as_str().to_owned()],
        },
        OperationId::new(1),
        TraceId::new(1),
        root,
    )?;
    let mut service = BuildService::default();
    service.submit(request)?;
    service.transition(OperationId::new(1), BuildPhase::Resolving, 5)?;
    service.transition(OperationId::new(1), BuildPhase::Ready, 15)?;
    service.transition(OperationId::new(1), BuildPhase::Running, 20)?;
    let invocation = CargoInvocation::new(
        &workspace,
        CargoCommand::Check,
        vec!["-p".to_owned(), "meridian-build".to_owned()],
        environment,
    )?;
    let outcome = run_cargo_json(&invocation, &BuildCancellation::default())?;
    for message in outcome.messages {
        service.record_cargo_message(OperationId::new(1), message)?;
    }
    match outcome.status {
        CargoRunStatus::Succeeded => {
            let event = service.transition(OperationId::new(1), BuildPhase::Succeeded, 100)?;
            println!(
                "Meridian build service smoke passed: {} {}",
                event.build_id, event.sequence
            );
            Ok(())
        }
        CargoRunStatus::Failed(code) => {
            Err(format!("Cargo check failed with status {code:?}").into())
        }
        CargoRunStatus::Cancelled => Err("Cargo check was unexpectedly cancelled".into()),
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| "meridian-build crate is not below the workspace root".into())
}
