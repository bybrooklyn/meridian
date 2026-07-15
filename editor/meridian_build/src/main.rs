//! Bounded helper CLI for the Meridian build-service foundation.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use meridian_build::{
    hash_file_bounded, run_cargo_json, run_cargo_metadata, BuildCancellation, BuildIdentityInput,
    BuildNode, BuildNodeId, BuildPhase, BuildRequest, BuildService, CargoCommand, CargoEnvironment,
    CargoInvocation, CargoRunStatus,
};
use meridian_core::{OperationId, TraceId};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = CliArguments::parse(env::args().skip(1))?;
    let lock_hash = hash_file_bounded(&arguments.workspace.join("Cargo.lock"))?;
    let environment = CargoEnvironment::from_host();
    let metadata = run_cargo_metadata(
        &CargoInvocation::new(
            &arguments.workspace,
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
    let mut root = BuildNode::cargo_check(
        BuildNodeId::new("cargo-check")?,
        arguments.toolchain.clone(),
    )?;
    root.declared_environment = environment.identity_values().into_keys().collect();
    let identity = BuildIdentityInput {
        source_checkpoint: arguments.source_checkpoint,
        resolved_profile: "cargo-check".to_owned(),
        cargo_metadata_and_lock: format!("metadata:{metadata_hash};lock:{lock_hash}"),
        toolchain_version: arguments.toolchain,
        target_and_capabilities: arguments.target,
        environment_allowlist: environment.identity_values(),
        root_node_ids: vec![root.id.as_str().to_owned()],
    };
    let request = BuildRequest::new(&identity, OperationId::new(1), TraceId::new(1), root)?;
    let mut service = BuildService::default();
    print_event(&service.submit(request)?);
    print_event(&service.transition(OperationId::new(1), BuildPhase::Resolving, 5)?);
    print_event(&service.transition(OperationId::new(1), BuildPhase::Ready, 15)?);
    print_event(&service.transition(OperationId::new(1), BuildPhase::Running, 20)?);

    let invocation = CargoInvocation::new(
        &arguments.workspace,
        CargoCommand::Check,
        arguments.cargo_arguments,
        environment,
    )?;
    let outcome = run_cargo_json(&invocation, &BuildCancellation::default())?;
    for message in outcome.messages {
        print_event(&service.record_cargo_message(OperationId::new(1), message)?);
    }
    match outcome.status {
        CargoRunStatus::Succeeded => {
            print_event(&service.transition(OperationId::new(1), BuildPhase::Succeeded, 100)?);
        }
        CargoRunStatus::Failed(_) => {
            print_event(&service.transition(OperationId::new(1), BuildPhase::Failed, 100)?);
        }
        CargoRunStatus::Cancelled => {
            print_event(&service.transition(
                OperationId::new(1),
                BuildPhase::CancelRequested,
                100,
            )?);
            print_event(&service.transition(OperationId::new(1), BuildPhase::Cancelled, 100)?);
        }
    }
    Ok(())
}

fn print_event(event: &meridian_build::BuildEvent) {
    println!(
        "Meridian build: operation {} build {} node {} sequence {} phase {:?}",
        event.operation_id, event.build_id, event.node_id, event.sequence, event.phase
    );
}

struct CliArguments {
    workspace: PathBuf,
    source_checkpoint: String,
    toolchain: String,
    target: String,
    cargo_arguments: Vec<String>,
}

impl CliArguments {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut arguments = arguments.into_iter();
        if arguments.next().as_deref() != Some("--cargo-check") {
            return Err(CliError::Usage);
        }
        let mut workspace = None;
        let mut source_checkpoint = None;
        let mut toolchain = None;
        let mut target = None;
        let mut cargo_arguments = Vec::new();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--workspace" => workspace = Some(next_value(&mut arguments, "--workspace")?),
                "--source-checkpoint" => {
                    source_checkpoint = Some(next_value(&mut arguments, "--source-checkpoint")?);
                }
                "--toolchain" => toolchain = Some(next_value(&mut arguments, "--toolchain")?),
                "--target" => target = Some(next_value(&mut arguments, "--target")?),
                "--" => {
                    cargo_arguments.extend(arguments);
                    break;
                }
                _ => return Err(CliError::UnknownArgument(argument)),
            }
        }
        Ok(Self {
            workspace: PathBuf::from(workspace.ok_or(CliError::MissingArgument("--workspace"))?),
            source_checkpoint: source_checkpoint
                .ok_or(CliError::MissingArgument("--source-checkpoint"))?,
            toolchain: toolchain.ok_or(CliError::MissingArgument("--toolchain"))?,
            target: target.ok_or(CliError::MissingArgument("--target"))?,
            cargo_arguments,
        })
    }
}

fn next_value(
    arguments: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<String, CliError> {
    arguments.next().ok_or(CliError::MissingArgument(flag))
}

#[derive(Debug)]
enum CliError {
    Usage,
    MissingArgument(&'static str),
    UnknownArgument(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => formatter.write_str(
                "usage: meridian-build --cargo-check --workspace <path> --source-checkpoint <id> --toolchain <version> --target <target> [-- <cargo arguments>]",
            ),
            Self::MissingArgument(flag) => write!(formatter, "missing required argument {flag}"),
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument {argument}"),
        }
    }
}

impl Error for CliError {}
