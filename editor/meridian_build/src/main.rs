//! Bounded helper CLI for the Meridian build-service foundation.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use meridian_build::{
    hash_file_bounded, run_cargo_metadata, ArtifactStore, BuildCancellation, BuildEvent,
    BuildGraph, BuildGraphSchedule, BuildIdentityInput, BuildNode, BuildNodeId, BuildPhase,
    BuildRequest, BuildServiceStore, CargoArtifact, CargoBuildSupervisor, CargoCommand,
    CargoEnvironment, CargoInvocation, CargoMessage, CargoMetadataOutcome, CargoRunStatus,
    DurableBuildService,
};
use meridian_core::{OperationId, TraceId};

static NEXT_DEFAULT_STATE_ID: AtomicU64 = AtomicU64::new(0);

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = CliArguments::parse(env::args().skip(1))?;
    let mut prepared = PreparedBuild::prepare(&arguments)?;
    run_cargo_operation(&arguments, &mut prepared)
}

struct PreparedBuild {
    artifact_store: Option<(ArtifactStore, PathBuf)>,
    environment: CargoEnvironment,
    root_node_id: BuildNodeId,
    schedule: BuildGraphSchedule,
    request: BuildRequest,
    service: DurableBuildService,
    state_path: PathBuf,
    remove_state_after_success: bool,
}

impl PreparedBuild {
    fn prepare(arguments: &CliArguments) -> Result<Self, Box<dyn Error>> {
        let artifact_store = optional_artifact_store(arguments)?;
        let (state_path, remove_state_after_success) = match &arguments.state {
            Some(path) => (path.clone(), false),
            None => (default_state_path(&arguments.workspace)?, true),
        };
        let recovery = DurableBuildService::open(BuildServiceStore::new(state_path.clone())?)?;
        println!("Meridian build state: {}", state_path.display());
        for event in &recovery.recovery_events {
            print_event(event);
        }
        let lock_hash = hash_file_bounded(&arguments.workspace.join("Cargo.lock"))?;
        let environment = CargoEnvironment::from_host();
        let mut metadata_node = BuildNode::cargo_metadata(
            BuildNodeId::new("cargo-metadata")?,
            arguments.toolchain.clone(),
        )?;
        metadata_node.declared_environment = environment.identity_values().into_keys().collect();
        let mut root = cargo_root_node(arguments.cargo_command, &arguments.toolchain)?;
        root.declared_environment = environment.identity_values().into_keys().collect();
        root.dependencies.push(metadata_node.id.clone());
        let graph = BuildGraph::new(
            vec![metadata_node.clone(), root.clone()],
            vec![root.id.clone()],
        )?;
        let mut schedule = graph.schedule();
        let _ = schedule.start(&metadata_node.id)?;
        let metadata = run_cargo_metadata(
            &CargoInvocation::new(
                &arguments.workspace,
                CargoCommand::Metadata,
                Vec::new(),
                environment.clone(),
            )?,
            &BuildCancellation::default(),
        )?;
        let metadata_identity =
            metadata_identity_or_error(metadata, &mut schedule, &metadata_node.id)?;
        let identity = BuildIdentityInput {
            source_checkpoint: arguments.source_checkpoint.clone(),
            resolved_profile: format!("cargo-{}", arguments.cargo_command.as_str()),
            cargo_metadata_and_lock: format!(
                "workspace-metadata:{metadata_identity};lock:{lock_hash}"
            ),
            build_graph_contract: graph.contract_hash(),
            command_arguments: arguments.cargo_arguments.clone(),
            toolchain_version: arguments.toolchain.clone(),
            target_and_capabilities: arguments.target.clone(),
            environment_allowlist: environment.identity_values(),
            root_node_ids: vec![root.id.as_str().to_owned()],
        };
        graph.validate_identity(&identity)?;
        let _ = schedule.start(&root.id)?;
        let operation_id = recovery.service.service().next_operation_id()?;
        let request = BuildRequest::new_with_graph(
            &identity,
            operation_id,
            TraceId::new(operation_id.get()),
            root.clone(),
            &graph,
        )?;
        Ok(Self {
            artifact_store,
            environment,
            root_node_id: root.id,
            schedule,
            request,
            service: recovery.service,
            state_path,
            remove_state_after_success,
        })
    }
}

fn run_cargo_operation(
    arguments: &CliArguments,
    prepared: &mut PreparedBuild,
) -> Result<(), Box<dyn Error>> {
    let publication_request = prepared.request.clone();
    print_event(&prepared.service.submit(prepared.request.clone())?);
    transition_service_to_running(&mut prepared.service, prepared.request.operation_id)?;
    let invocation = CargoInvocation::new(
        &arguments.workspace,
        arguments.cargo_command,
        arguments.cargo_arguments.clone(),
        prepared.environment.clone(),
    )?;
    let mut supervisor = CargoBuildSupervisor::try_new()?;
    supervisor.submit(&prepared.service, &prepared.request, invocation)?;
    let artifact_store = prepared.artifact_store.take();
    let completion = loop {
        if let Some(completion) =
            supervisor.poll_with(&mut prepared.service, |service, operation_id, messages| {
                publish_succeeded_executable(
                    artifact_store.as_ref(),
                    messages,
                    &publication_request,
                    service,
                    operation_id,
                )
            })
        {
            break completion?;
        }
        thread::sleep(Duration::from_millis(10));
    };
    for event in completion.events() {
        print_event(event);
    }
    match completion.status() {
        CargoRunStatus::Succeeded => {
            let _ = prepared
                .schedule
                .finish(&prepared.root_node_id, BuildPhase::Succeeded)?;
            if prepared.remove_state_after_success {
                fs::remove_file(&prepared.state_path)?;
            }
        }
        CargoRunStatus::Failed(code) => {
            let _ = prepared
                .schedule
                .finish(&prepared.root_node_id, BuildPhase::Failed)?;
            return Err(format!(
                "cargo {} failed with status {code:?}",
                arguments.cargo_command.as_str()
            )
            .into());
        }
        CargoRunStatus::Cancelled => {
            let _ = prepared
                .schedule
                .finish(&prepared.root_node_id, BuildPhase::Cancelled)?;
            return Err(format!("cargo {} was cancelled", arguments.cargo_command.as_str()).into());
        }
    }
    Ok(())
}

fn transition_service_to_running(
    service: &mut DurableBuildService,
    operation_id: OperationId,
) -> Result<(), meridian_build::BuildError> {
    print_event(&service.transition(operation_id, BuildPhase::Resolving, 5)?);
    print_event(&service.transition(operation_id, BuildPhase::Ready, 15)?);
    print_event(&service.transition(operation_id, BuildPhase::Running, 20)?);
    Ok(())
}

fn optional_artifact_store(
    arguments: &CliArguments,
) -> Result<Option<(ArtifactStore, PathBuf)>, meridian_build::BuildError> {
    match (
        arguments.artifact_store.as_ref(),
        arguments.cargo_output_root.as_ref(),
    ) {
        (Some(store_root), Some(output_root)) => Ok(Some((
            ArtifactStore::new(store_root.clone())?,
            output_root.clone(),
        ))),
        (None, None) => Ok(None),
        _ => unreachable!("CLI parser requires artifact publication paths together"),
    }
}

fn publish_succeeded_executable(
    artifact_store: Option<&(ArtifactStore, PathBuf)>,
    messages: &[CargoMessage],
    request: &BuildRequest,
    service: &mut DurableBuildService,
    operation_id: OperationId,
) -> Result<Vec<BuildEvent>, meridian_build::BuildError> {
    let Some((store, output_root)) = artifact_store else {
        return Ok(Vec::new());
    };
    let cargo_executables = messages
        .iter()
        .filter_map(|message| match message {
            CargoMessage::Artifact(artifact) if artifact.executable.is_some() => Some(artifact),
            CargoMessage::Diagnostic(_)
            | CargoMessage::Artifact(_)
            | CargoMessage::Finished { .. } => None,
        })
        .cloned()
        .collect::<Vec<CargoArtifact>>();
    let [artifact] = cargo_executables.as_slice() else {
        return Err(meridian_build::BuildError::CargoArtifactExecutableCount(
            cargo_executables.len(),
        ));
    };
    let publication = store.publish_cargo_executable_for_request(request, artifact, output_root)?;
    Ok(vec![
        service.record_published_artifact(operation_id, publication)?
    ])
}

fn metadata_identity_or_error(
    metadata: CargoMetadataOutcome,
    schedule: &mut BuildGraphSchedule,
    metadata_node_id: &BuildNodeId,
) -> Result<String, Box<dyn Error>> {
    match (
        metadata.status,
        metadata.workspace_identity_hash,
        metadata.process_diagnostic,
    ) {
        (CargoRunStatus::Succeeded, Some(hash), _) => {
            let _ = schedule.finish(metadata_node_id, BuildPhase::Succeeded)?;
            Ok(hash)
        }
        (CargoRunStatus::Failed(code), _, diagnostic) => {
            let _ = schedule.finish(metadata_node_id, BuildPhase::Failed)?;
            let detail =
                diagnostic.map_or_else(String::new, |value| format!(": {}", value.message));
            Err(format!("cargo metadata failed with status {code:?}{detail}").into())
        }
        (CargoRunStatus::Cancelled, _, _) => {
            let _ = schedule.finish(metadata_node_id, BuildPhase::Cancelled)?;
            Err("cargo metadata was cancelled".into())
        }
        (CargoRunStatus::Succeeded, None, _) => {
            let _ = schedule.finish(metadata_node_id, BuildPhase::Failed)?;
            Err("cargo metadata did not return a workspace identity hash".into())
        }
    }
}

fn print_event(event: &meridian_build::BuildEvent) {
    if let Some(diagnostic) = &event.diagnostic {
        println!(
            "Meridian build: operation {} build {} node {} sequence {} phase {:?} diagnostic {:?}: {}",
            event.operation_id,
            event.build_id,
            event.node_id,
            event.sequence,
            event.phase,
            diagnostic.severity,
            diagnostic.message,
        );
    } else {
        println!(
            "Meridian build: operation {} build {} node {} sequence {} phase {:?}",
            event.operation_id, event.build_id, event.node_id, event.sequence, event.phase
        );
    }
}

fn cargo_root_node(
    command: CargoCommand,
    toolchain: &str,
) -> Result<BuildNode, meridian_build::BuildError> {
    match command {
        CargoCommand::Check => BuildNode::cargo_check(BuildNodeId::new("cargo-check")?, toolchain),
        CargoCommand::Build => BuildNode::cargo_build(BuildNodeId::new("cargo-build")?, toolchain),
        CargoCommand::TestNoRun => {
            BuildNode::cargo_test_no_run(BuildNodeId::new("cargo-test-no-run")?, toolchain)
        }
        CargoCommand::Metadata => unreachable!("CLI root operation excludes cargo metadata"),
    }
}

fn default_state_path(workspace: &std::path::Path) -> Result<PathBuf, meridian_build::BuildError> {
    let directory = workspace.join("target/meridian-build");
    for _ in 0..16 {
        let nonce = NEXT_DEFAULT_STATE_ID.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!("state-{}-{nonce}.json", std::process::id()));
        let store = BuildServiceStore::new(path.clone())?;
        if !store.exists()? {
            return Ok(path);
        }
    }
    Err(meridian_build::BuildError::SnapshotTemporaryExhausted)
}

struct CliArguments {
    cargo_command: CargoCommand,
    workspace: PathBuf,
    source_checkpoint: String,
    toolchain: String,
    target: String,
    cargo_arguments: Vec<String>,
    state: Option<PathBuf>,
    artifact_store: Option<PathBuf>,
    cargo_output_root: Option<PathBuf>,
}

impl CliArguments {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut arguments = arguments.into_iter();
        let cargo_command = match arguments.next().as_deref() {
            Some("--cargo-check") => CargoCommand::Check,
            Some("--cargo-build") => CargoCommand::Build,
            Some("--cargo-test-no-run") => CargoCommand::TestNoRun,
            _ => return Err(CliError::Usage),
        };
        let mut workspace = None;
        let mut source_checkpoint = None;
        let mut toolchain = None;
        let mut target = None;
        let mut cargo_arguments = Vec::new();
        let mut state = None;
        let mut artifact_store = None;
        let mut cargo_output_root = None;
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--workspace" => workspace = Some(next_value(&mut arguments, "--workspace")?),
                "--source-checkpoint" => {
                    source_checkpoint = Some(next_value(&mut arguments, "--source-checkpoint")?);
                }
                "--toolchain" => toolchain = Some(next_value(&mut arguments, "--toolchain")?),
                "--target" => target = Some(next_value(&mut arguments, "--target")?),
                "--state" => state = Some(PathBuf::from(next_value(&mut arguments, "--state")?)),
                "--artifact-store" => {
                    artifact_store = Some(PathBuf::from(next_value(
                        &mut arguments,
                        "--artifact-store",
                    )?));
                }
                "--cargo-output-root" => {
                    cargo_output_root = Some(PathBuf::from(next_value(
                        &mut arguments,
                        "--cargo-output-root",
                    )?));
                }
                "--" => {
                    cargo_arguments.extend(arguments);
                    break;
                }
                _ => return Err(CliError::UnknownArgument(argument)),
            }
        }
        if artifact_store.is_some() != cargo_output_root.is_some() {
            return Err(CliError::ArtifactPublicationPathsMustPair);
        }
        if artifact_store.is_some() && cargo_command == CargoCommand::Check {
            return Err(CliError::ArtifactPublicationRequiresOutputCommand);
        }
        Ok(Self {
            cargo_command,
            workspace: PathBuf::from(workspace.ok_or(CliError::MissingArgument("--workspace"))?),
            source_checkpoint: source_checkpoint
                .ok_or(CliError::MissingArgument("--source-checkpoint"))?,
            toolchain: toolchain.ok_or(CliError::MissingArgument("--toolchain"))?,
            target: target.ok_or(CliError::MissingArgument("--target"))?,
            cargo_arguments,
            state,
            artifact_store,
            cargo_output_root,
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
    ArtifactPublicationPathsMustPair,
    ArtifactPublicationRequiresOutputCommand,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage => formatter.write_str(
                "usage: meridian-build <--cargo-check|--cargo-build|--cargo-test-no-run> --workspace <path> --source-checkpoint <id> --toolchain <version> --target <target> [--state <path>] [--artifact-store <path> --cargo-output-root <path>] [-- <cargo arguments>]",
            ),
            Self::MissingArgument(flag) => write!(formatter, "missing required argument {flag}"),
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument {argument}"),
            Self::ArtifactPublicationPathsMustPair => formatter.write_str(
                "--artifact-store and --cargo-output-root must be supplied together",
            ),
            Self::ArtifactPublicationRequiresOutputCommand => formatter.write_str(
                "artifact publication requires --cargo-build or --cargo-test-no-run",
            ),
        }
    }
}

impl Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parser_preserves_an_explicit_caller_owned_state_path() {
        let arguments = CliArguments::parse(
            [
                "--cargo-check",
                "--workspace",
                ".",
                "--source-checkpoint",
                "local",
                "--toolchain",
                "workspace",
                "--target",
                "host",
                "--state",
                "target/caller-owned-state.json",
                "--",
                "-p",
                "meridian-core",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .expect("arguments parse");
        assert_eq!(
            arguments.state,
            Some(PathBuf::from("target/caller-owned-state.json"))
        );
        assert_eq!(arguments.cargo_arguments, ["-p", "meridian-core"]);
    }

    #[test]
    fn default_state_paths_are_unique_under_the_workspace_target_directory() {
        let workspace = std::env::temp_dir().join(format!(
            "meridian-build-cli-state-test-{}",
            std::process::id()
        ));
        let first = default_state_path(&workspace).expect("first path");
        let second = default_state_path(&workspace).expect("second path");
        assert_ne!(first, second);
        assert!(first.starts_with(workspace.join("target/meridian-build")));
        assert!(second.starts_with(workspace.join("target/meridian-build")));
    }
}
