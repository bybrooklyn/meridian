//! Meridian-owned, observable Cargo build-service foundation.
//!
//! Cargo remains the Rust build authority. This crate owns the typed request,
//! lifecycle, event, identity, and bounded Cargo JSON adapter contracts that
//! future Meridian editor and CLI surfaces share.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use meridian_core::{OperationId, TraceId};
use serde::{Deserialize, Serialize};

/// First version of the editor/service build protocol.
pub const BUILD_PROTOCOL_VERSION: u16 = 1;
/// Maximum accepted Cargo JSON line, before JSON parsing allocates its fields.
pub const MAX_CARGO_JSON_LINE_BYTES: usize = 1_048_576;
/// Maximum accepted `cargo metadata` payload before JSON parsing allocates its fields.
pub const MAX_CARGO_METADATA_BYTES: usize = 8 * 1_024 * 1_024;
/// Maximum serialized service snapshot accepted before JSON parsing.
pub const MAX_BUILD_SNAPSHOT_BYTES: usize = 1_048_576;
/// Maximum accepted Cargo manifest or lockfile input for this initial service.
pub const MAX_CARGO_INPUT_BYTES: usize = 8 * 1_024 * 1_024;
const MAX_FIELD_BYTES: usize = 4_096;
const MAX_ARGUMENTS: usize = 64;
const MAX_FILENAMES: usize = 256;
const MAX_CARGO_PACKAGES: usize = 4_096;
const MAX_CARGO_TARGETS: usize = 256;
const MAX_CARGO_TARGET_KINDS: usize = 64;
const MAX_OPERATIONS: usize = 1_024;

const ENVIRONMENT_ALLOWLIST: &[&str] = &[
    "CARGO_HOME",
    "HOME",
    "PATH",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "TEMP",
    "TMP",
    "TMPDIR",
];

/// Content-addressed identity for a build's declared inputs.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BuildId(String);

impl BuildId {
    /// Derives a deterministic ID from all declared build inputs.
    ///
    /// # Errors
    ///
    /// Returns an error when an input is absent, oversized, duplicated, or has
    /// a non-allowlisted environment name.
    pub fn derive(input: &BuildIdentityInput) -> Result<Self, BuildError> {
        validate_identity(input)?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"meridian-build-id-v1\0");
        hash_field(&mut hasher, "source-checkpoint", &input.source_checkpoint);
        hash_field(&mut hasher, "profile", &input.resolved_profile);
        hash_field(
            &mut hasher,
            "cargo-metadata-and-lock",
            &input.cargo_metadata_and_lock,
        );
        hash_field(&mut hasher, "toolchain", &input.toolchain_version);
        hash_field(&mut hasher, "target", &input.target_and_capabilities);
        for (name, value) in &input.environment_allowlist {
            hash_field(&mut hasher, "environment-name", name);
            hash_field(&mut hasher, "environment-value", value);
        }
        let mut roots = input.root_node_ids.clone();
        roots.sort_unstable();
        for root in roots {
            hash_field(&mut hasher, "root-node", &root);
        }

        Ok(Self(hasher.finalize().to_hex().to_string()))
    }

    /// Returns the stable hexadecimal representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for BuildId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// All declared inputs contributing to one [`BuildId`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildIdentityInput {
    /// Immutable source checkpoint or an explicit local-worktree marker.
    pub source_checkpoint: String,
    /// Resolved Meridian project/build profile.
    pub resolved_profile: String,
    /// Content hash of resolved Cargo metadata and lockfile inputs.
    pub cargo_metadata_and_lock: String,
    /// Declared Rust/Cargo toolchain version.
    pub toolchain_version: String,
    /// Target triple plus selected capability profile.
    pub target_and_capabilities: String,
    /// Explicit environment values admitted into the identity.
    pub environment_allowlist: BTreeMap<String, String>,
    /// Ordered roots requested by the caller; order does not affect identity.
    pub root_node_ids: Vec<String>,
}

/// Stable identity of one build-graph node.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BuildNodeId(String);

impl BuildNodeId {
    /// Creates a bounded, nonempty node ID.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty, oversized, or NUL-containing ID.
    pub fn new(value: impl Into<String>) -> Result<Self, BuildError> {
        let value = value.into();
        validate_text("build node ID", &value)?;
        Ok(Self(value))
    }

    /// Returns the stable node identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for BuildNodeId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Bounded build nodes implemented by the first service slice.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BuildNodeKind {
    /// Resolve workspace/package/target information through `cargo metadata`.
    CargoMetadata,
    /// Run a Cargo check and consume its JSON message stream.
    CargoCheck,
}

/// A typed graph node with declared immutable inputs and dependencies.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildNode {
    /// Stable graph node identity.
    pub id: BuildNodeId,
    /// Node implementation selected by the caller.
    pub kind: BuildNodeKind,
    /// Hashes of declared immutable inputs.
    pub input_hashes: Vec<String>,
    /// Tool identity/version used to execute the node.
    pub tool_id_version: String,
    /// Environment variable names, not raw ambient environment access.
    pub declared_environment: Vec<String>,
    /// Node dependencies that must complete before this node may run.
    pub dependencies: Vec<BuildNodeId>,
}

impl BuildNode {
    /// Builds a minimal Cargo-check node using the declared Cargo tool identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the ID or declared tool version is invalid.
    pub fn cargo_check(
        id: BuildNodeId,
        tool_id_version: impl Into<String>,
    ) -> Result<Self, BuildError> {
        let tool_id_version = tool_id_version.into();
        validate_text("tool ID and version", &tool_id_version)?;
        Ok(Self {
            id,
            kind: BuildNodeKind::CargoCheck,
            input_hashes: Vec::new(),
            tool_id_version,
            declared_environment: Vec::new(),
            dependencies: Vec::new(),
        })
    }
}

/// A caller's immutable request to start one bounded build operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildRequest {
    /// Content-addressed identity of all declared build inputs.
    pub build_id: BuildId,
    /// Process-local operation correlation ID.
    pub operation_id: OperationId,
    /// Cross-domain diagnostic correlation ID.
    pub trace_id: TraceId,
    /// The root node selected for this bounded operation.
    pub root_node: BuildNode,
}

impl BuildRequest {
    /// Creates one request and derives its deterministic identity.
    ///
    /// # Errors
    ///
    /// Returns an error when declared identity inputs are invalid.
    pub fn new(
        identity: &BuildIdentityInput,
        operation_id: OperationId,
        trace_id: TraceId,
        root_node: BuildNode,
    ) -> Result<Self, BuildError> {
        Ok(Self {
            build_id: BuildId::derive(identity)?,
            operation_id,
            trace_id,
            root_node,
        })
    }
}

/// Observable lifecycle of one build operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BuildPhase {
    /// Request accepted but not yet resolved.
    Queued,
    /// Inputs and profile are being resolved.
    Resolving,
    /// Inputs resolved and the node can start.
    Ready,
    /// A bounded worker or Cargo process is executing.
    Running,
    /// Cooperative cancellation was requested.
    CancelRequested,
    /// The operation stopped without publishing outputs.
    Cancelled,
    /// Outputs were validated and committed.
    Succeeded,
    /// The operation reached an actionable failure.
    Failed,
    /// A supervised worker disappeared before a result was committed.
    WorkerLost,
    /// A newer `BuildId` or request invalidated this operation.
    Superseded,
}

impl BuildPhase {
    /// Returns whether this phase can never accept another transition.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Succeeded | Self::Failed | Self::WorkerLost | Self::Superseded
        )
    }
}

/// Severity supplied by Cargo or a Meridian service diagnostic.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiagnosticSeverity {
    /// Informational diagnostic.
    Note,
    /// Non-fatal warning.
    Warning,
    /// Actionable build failure.
    Error,
}

/// A bounded, redacted diagnostic transported through build events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildDiagnostic {
    /// Optional compiler error code.
    pub code: Option<String>,
    /// Severity after Cargo message mapping.
    pub severity: DiagnosticSeverity,
    /// Primary diagnostic summary.
    pub message: String,
    /// Full rendered diagnostic within the bounded input limit, with secret-like assignments redacted.
    pub rendered: Option<String>,
}

/// Cargo outputs represented without leaking Cargo types through Meridian APIs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum CargoMessage {
    /// Compiler diagnostic mapped into Meridian-owned fields.
    Diagnostic(BuildDiagnostic),
    /// Artifact path information emitted by Cargo.
    Artifact(CargoArtifact),
    /// Cargo's final success/failure message.
    Finished { success: bool },
}

/// One Cargo artifact record with bounded path strings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CargoArtifact {
    /// Cargo package identifier.
    pub package_id: String,
    /// Target name from Cargo's JSON protocol.
    pub target_name: String,
    /// Artifact filenames supplied by Cargo.
    pub filenames: Vec<String>,
    /// Optional executable path supplied by Cargo.
    pub executable: Option<String>,
}

/// Additional structured payload attached to every build event.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum BuildEventPayload {
    /// A lifecycle transition without a Cargo message.
    Lifecycle,
    /// A Cargo JSON message associated with a running operation.
    Cargo(CargoMessage),
}

/// Versioned event emitted by the build service.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildEvent {
    /// Protocol version understood by editor/service consumers.
    pub protocol_version: u16,
    /// Identity of the immutable build input set.
    pub build_id: BuildId,
    /// Process-local operation identity.
    pub operation_id: OperationId,
    /// Graph node that emitted the event.
    pub node_id: BuildNodeId,
    /// Monotonic sequence within one operation.
    pub sequence: u64,
    /// Current lifecycle phase.
    pub phase: BuildPhase,
    /// Bounded progress value from 0 to 100.
    pub progress: u8,
    /// Optional normalized diagnostic.
    pub diagnostic: Option<BuildDiagnostic>,
    /// Optional hash of a validated artifact; absent until artifact publication exists.
    pub artifact_hash: Option<String>,
    /// Cross-domain correlation identity.
    pub trace_id: TraceId,
    /// Structured non-terminal payload.
    pub payload: BuildEventPayload,
}

/// In-memory operation registry rejecting stale or invalid event flows.
#[derive(Default)]
pub struct BuildService {
    operations: BTreeMap<OperationId, BuildOperation>,
}

impl BuildService {
    /// Registers one queued operation and emits its first event.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation ID already exists.
    pub fn submit(&mut self, request: BuildRequest) -> Result<BuildEvent, BuildError> {
        if self.operations.len() == MAX_OPERATIONS {
            return Err(BuildError::TooManyOperations(MAX_OPERATIONS));
        }
        if self.operations.contains_key(&request.operation_id) {
            return Err(BuildError::DuplicateOperation(request.operation_id));
        }
        let operation_id = request.operation_id;
        let mut operation = BuildOperation::new(request);
        let event = operation.emit(BuildPhase::Queued, 0, BuildEventPayload::Lifecycle)?;
        self.operations.insert(operation_id, operation);
        Ok(event)
    }

    /// Advances an operation through a valid lifecycle transition.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown operation or invalid transition.
    pub fn transition(
        &mut self,
        operation_id: OperationId,
        phase: BuildPhase,
        progress: u8,
    ) -> Result<BuildEvent, BuildError> {
        self.operation_mut(operation_id)?
            .emit(phase, progress, BuildEventPayload::Lifecycle)
    }

    /// Records one parsed Cargo JSON message while an operation is running.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is absent or not running.
    pub fn record_cargo_message(
        &mut self,
        operation_id: OperationId,
        message: CargoMessage,
    ) -> Result<BuildEvent, BuildError> {
        let operation = self.operation_mut(operation_id)?;
        if operation.phase != BuildPhase::Running {
            return Err(BuildError::CargoMessageOutsideRunning(operation.phase));
        }
        let diagnostic = match &message {
            CargoMessage::Diagnostic(diagnostic) => Some(diagnostic.clone()),
            CargoMessage::Artifact(_) | CargoMessage::Finished { .. } => None,
        };
        operation.emit_with_diagnostic(
            BuildPhase::Running,
            50,
            diagnostic,
            BuildEventPayload::Cargo(message),
        )
    }

    /// Accepts a worker-produced event only if identity, sequence, and phase are valid.
    ///
    /// # Errors
    ///
    /// Returns an error for stale sequence numbers, mismatched identities, or invalid transitions.
    pub fn accept_external_event(&mut self, event: &BuildEvent) -> Result<(), BuildError> {
        let operation = self.operation_mut(event.operation_id)?;
        if operation.request.build_id != event.build_id
            || operation.request.trace_id != event.trace_id
        {
            return Err(BuildError::MismatchedEventIdentity);
        }
        if operation.request.root_node.id != event.node_id {
            return Err(BuildError::MismatchedNodeId);
        }
        let expected = operation.last_sequence.saturating_add(1);
        if event.sequence != expected {
            return Err(BuildError::StaleEventSequence {
                expected,
                received: event.sequence,
            });
        }
        validate_transition(operation.phase, event.phase)?;
        operation.phase = event.phase;
        operation.last_sequence = event.sequence;
        Ok(())
    }

    /// Returns the current phase for one registered operation.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation ID is unknown.
    pub fn phase(&self, operation_id: OperationId) -> Result<BuildPhase, BuildError> {
        self.operations
            .get(&operation_id)
            .map(|operation| operation.phase)
            .ok_or(BuildError::UnknownOperation(operation_id))
    }

    /// Serializes bounded operation state for a host-owned durable store.
    ///
    /// # Errors
    ///
    /// Returns an error when snapshot serialization fails or exceeds the declared limit.
    pub fn snapshot_json(&self) -> Result<String, BuildError> {
        let snapshot = PersistedBuildService {
            version: BUILD_PROTOCOL_VERSION,
            operations: self
                .operations
                .values()
                .map(PersistedBuildOperation::from)
                .collect(),
        };
        let encoded = serde_json::to_string(&snapshot)
            .map_err(|error| BuildError::SnapshotSerialization(error.to_string()))?;
        if encoded.len() > MAX_BUILD_SNAPSHOT_BYTES {
            return Err(BuildError::SnapshotTooLarge(encoded.len()));
        }
        Ok(encoded)
    }

    /// Restores one bounded snapshot and emits `WorkerLost` for interrupted operations.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, oversized, incompatible, duplicated, or invalid state.
    pub fn restore_json(snapshot: &str) -> Result<BuildServiceRecovery, BuildError> {
        if snapshot.len() > MAX_BUILD_SNAPSHOT_BYTES {
            return Err(BuildError::SnapshotTooLarge(snapshot.len()));
        }
        let persisted: PersistedBuildService = serde_json::from_str(snapshot)
            .map_err(|error| BuildError::MalformedSnapshot(error.to_string()))?;
        if persisted.version != BUILD_PROTOCOL_VERSION {
            return Err(BuildError::UnsupportedSnapshotVersion(persisted.version));
        }
        if persisted.operations.len() > MAX_OPERATIONS {
            return Err(BuildError::TooManyOperations(persisted.operations.len()));
        }
        let mut service = Self::default();
        let mut recovery_events = Vec::new();
        for persisted_operation in persisted.operations {
            validate_persisted_operation(&persisted_operation)?;
            let operation_id = persisted_operation.request.operation_id;
            if service.operations.contains_key(&operation_id) {
                return Err(BuildError::DuplicateSnapshotOperation(operation_id));
            }
            let mut operation = BuildOperation {
                request: persisted_operation.request,
                phase: persisted_operation.phase,
                last_sequence: persisted_operation.last_sequence,
            };
            if !operation.phase.is_terminal() {
                recovery_events.push(operation.emit(
                    BuildPhase::WorkerLost,
                    100,
                    BuildEventPayload::Lifecycle,
                )?);
            }
            service.operations.insert(operation_id, operation);
        }
        Ok(BuildServiceRecovery {
            service,
            recovery_events,
        })
    }

    fn operation_mut(
        &mut self,
        operation_id: OperationId,
    ) -> Result<&mut BuildOperation, BuildError> {
        self.operations
            .get_mut(&operation_id)
            .ok_or(BuildError::UnknownOperation(operation_id))
    }
}

/// Restored service plus explicit worker-loss events for interrupted operations.
pub struct BuildServiceRecovery {
    /// Reconstructed service state, safe for inspection or retry planning.
    pub service: BuildService,
    /// Events describing operations that were interrupted before a valid commit.
    pub recovery_events: Vec<BuildEvent>,
}

#[derive(Deserialize, Serialize)]
struct PersistedBuildService {
    version: u16,
    operations: Vec<PersistedBuildOperation>,
}

#[derive(Deserialize, Serialize)]
struct PersistedBuildOperation {
    request: BuildRequest,
    phase: BuildPhase,
    last_sequence: u64,
}

impl From<&BuildOperation> for PersistedBuildOperation {
    fn from(operation: &BuildOperation) -> Self {
        Self {
            request: operation.request.clone(),
            phase: operation.phase,
            last_sequence: operation.last_sequence,
        }
    }
}

struct BuildOperation {
    request: BuildRequest,
    phase: BuildPhase,
    last_sequence: u64,
}

impl BuildOperation {
    const fn new(request: BuildRequest) -> Self {
        Self {
            request,
            phase: BuildPhase::Queued,
            last_sequence: 0,
        }
    }

    fn emit(
        &mut self,
        phase: BuildPhase,
        progress: u8,
        payload: BuildEventPayload,
    ) -> Result<BuildEvent, BuildError> {
        self.emit_with_diagnostic(phase, progress, None, payload)
    }

    fn emit_with_diagnostic(
        &mut self,
        phase: BuildPhase,
        progress: u8,
        diagnostic: Option<BuildDiagnostic>,
        payload: BuildEventPayload,
    ) -> Result<BuildEvent, BuildError> {
        validate_transition(self.phase, phase)?;
        self.phase = phase;
        self.last_sequence = self.last_sequence.saturating_add(1);
        Ok(BuildEvent {
            protocol_version: BUILD_PROTOCOL_VERSION,
            build_id: self.request.build_id.clone(),
            operation_id: self.request.operation_id,
            node_id: self.request.root_node.id.clone(),
            sequence: self.last_sequence,
            phase,
            progress,
            diagnostic,
            artifact_hash: None,
            trace_id: self.request.trace_id,
            payload,
        })
    }
}

fn validate_persisted_operation(operation: &PersistedBuildOperation) -> Result<(), BuildError> {
    validate_build_id(&operation.request.build_id)?;
    validate_text("build node ID", operation.request.root_node.id.as_str())?;
    validate_text(
        "build node tool ID and version",
        &operation.request.root_node.tool_id_version,
    )?;
    for input_hash in &operation.request.root_node.input_hashes {
        validate_text("build node input hash", input_hash)?;
    }
    for name in &operation.request.root_node.declared_environment {
        if !is_allowlisted_environment(name) {
            return Err(BuildError::EnvironmentNotAllowlisted(name.clone()));
        }
    }
    for dependency in &operation.request.root_node.dependencies {
        validate_text("build node dependency", dependency.as_str())?;
    }
    Ok(())
}

/// Cooperative cancellation shared by a request and its Cargo worker.
#[derive(Clone, Debug, Default)]
pub struct BuildCancellation {
    cancelled: Arc<AtomicBool>,
}

impl BuildCancellation {
    /// Requests cooperative cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Cargo subcommands supported by the initial structured adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoCommand {
    /// Resolve full workspace/package/target metadata in Cargo's stable JSON format.
    Metadata,
    /// Run `cargo check` with Cargo's machine-readable JSON message protocol.
    Check,
}

impl CargoCommand {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::Check => "check",
        }
    }
}

/// Explicit environment passed to Cargo after ambient environment clearing.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CargoEnvironment {
    values: BTreeMap<String, String>,
}

impl CargoEnvironment {
    /// Copies only known-safe build variables from the host process.
    #[must_use]
    pub fn from_host() -> Self {
        let mut values = BTreeMap::new();
        for name in ENVIRONMENT_ALLOWLIST {
            if let Ok(value) = std::env::var(name) {
                if validate_text("environment value", &value).is_ok() {
                    values.insert((*name).to_owned(), value);
                }
            }
        }
        Self { values }
    }

    /// Adds one allowlisted environment value.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported name or an invalid value.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), BuildError> {
        let name = name.into();
        let value = value.into();
        if !is_allowlisted_environment(&name) {
            return Err(BuildError::EnvironmentNotAllowlisted(name));
        }
        validate_text("environment value", &value)?;
        self.values.insert(name, value);
        Ok(())
    }

    /// Returns the explicit values that must contribute to a local build identity.
    ///
    /// Only allowlisted names are present; callers must still treat path-bearing
    /// host values as local-only identity input rather than portable metadata.
    #[must_use]
    pub fn identity_values(&self) -> BTreeMap<String, String> {
        self.values.clone()
    }

    fn values(&self) -> &BTreeMap<String, String> {
        &self.values
    }
}

/// Validated, shell-free Cargo process plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoInvocation {
    workspace: PathBuf,
    command: CargoCommand,
    arguments: Vec<String>,
    environment: CargoEnvironment,
}

impl CargoInvocation {
    /// Creates a structured Cargo invocation without executing it.
    ///
    /// # Errors
    ///
    /// Returns an error for empty paths, invalid arguments, or attempts to override the JSON protocol.
    pub fn new(
        workspace: impl Into<PathBuf>,
        command: CargoCommand,
        arguments: Vec<String>,
        environment: CargoEnvironment,
    ) -> Result<Self, BuildError> {
        let workspace = workspace.into();
        if workspace.as_os_str().is_empty() {
            return Err(BuildError::EmptyWorkspace);
        }
        if arguments.len() > MAX_ARGUMENTS {
            return Err(BuildError::TooManyArguments(arguments.len()));
        }
        for argument in &arguments {
            validate_text("Cargo argument", argument)?;
            if matches!(argument.as_str(), "--locked" | "--quiet" | "--no-deps")
                || argument.starts_with("--format-version")
                || argument.starts_with("--message-format")
            {
                return Err(BuildError::ReservedCargoArgument(argument.clone()));
            }
        }
        Ok(Self {
            workspace,
            command,
            arguments,
            environment,
        })
    }

    /// Returns the exact program and argument vector to execute without a shell.
    #[must_use]
    pub fn command_plan(&self) -> CargoCommandPlan {
        let mut arguments = match self.command {
            CargoCommand::Metadata => vec![
                self.command.as_str().to_owned(),
                "--locked".to_owned(),
                "--format-version=1".to_owned(),
            ],
            CargoCommand::Check => vec![
                self.command.as_str().to_owned(),
                "--locked".to_owned(),
                "--quiet".to_owned(),
                "--message-format=json".to_owned(),
            ],
        };
        arguments.extend(self.arguments.iter().cloned());
        CargoCommandPlan {
            program: PathBuf::from(env!("CARGO")),
            workspace: self.workspace.clone(),
            arguments,
        }
    }
}

/// Inspectable process plan used by the adapter and its tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoCommandPlan {
    /// Direct Cargo executable path; never a shell program.
    pub program: PathBuf,
    /// Project root passed to `Command::current_dir`.
    pub workspace: PathBuf,
    /// Structured Cargo arguments, in execution order.
    pub arguments: Vec<String>,
}

/// Final outcome of a bounded Cargo JSON invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoRunOutcome {
    /// Process outcome, including cancellation.
    pub status: CargoRunStatus,
    /// Bounded Cargo messages parsed from stdout.
    pub messages: Vec<CargoMessage>,
}

/// Cargo process outcome represented without `ExitStatus` as a public API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoRunStatus {
    /// Cargo returned a success status.
    Succeeded,
    /// Cargo returned a nonzero or signal-only status.
    Failed(Option<i32>),
    /// Cancellation killed the child before output publication.
    Cancelled,
}

/// Typed summary of Cargo workspace metadata used by the build identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoMetadataSnapshot {
    /// Workspace root reported by Cargo.
    pub workspace_root: String,
    /// Workspace members reported by Cargo.
    pub workspace_members: Vec<String>,
    /// Packages and targets resolved by Cargo.
    pub packages: Vec<CargoPackage>,
}

/// One package returned by `cargo metadata`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoPackage {
    /// Cargo package ID.
    pub id: String,
    /// Package name.
    pub name: String,
    /// Manifest path reported by Cargo.
    pub manifest_path: String,
    /// Declared build targets.
    pub targets: Vec<CargoTarget>,
}

/// One target returned by `cargo metadata`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoTarget {
    /// Target name.
    pub name: String,
    /// Cargo target kinds, such as `lib`, `bin`, or `example`.
    pub kinds: Vec<String>,
}

/// Final outcome of a bounded `cargo metadata` invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoMetadataOutcome {
    /// Process outcome, including cancellation.
    pub status: CargoRunStatus,
    /// Parsed metadata when Cargo completed successfully.
    pub snapshot: Option<CargoMetadataSnapshot>,
    /// BLAKE3 hash of the exact bounded metadata payload used by the identity.
    pub content_hash: Option<String>,
}

/// Runs Cargo with structured arguments and parses its bounded JSON message stream.
///
/// # Errors
///
/// Returns an error when Cargo cannot start, its JSON violates the bounded protocol,
/// or its process output cannot be read safely.
pub fn run_cargo_json(
    invocation: &CargoInvocation,
    cancellation: &BuildCancellation,
) -> Result<CargoRunOutcome, BuildError> {
    if invocation.command != CargoCommand::Check {
        return Err(BuildError::UnexpectedCargoCommand {
            expected: CargoCommand::Check,
            received: invocation.command,
        });
    }
    if cancellation.is_cancelled() {
        return Ok(CargoRunOutcome {
            status: CargoRunStatus::Cancelled,
            messages: Vec::new(),
        });
    }

    let plan = invocation.command_plan();
    let mut command = Command::new(&plan.program);
    command
        .current_dir(&plan.workspace)
        .args(&plan.arguments)
        .env_clear()
        .envs(invocation.environment.values())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| BuildError::CargoSpawn(error.to_string()))?;
    let stdout = child.stdout.take().ok_or(BuildError::MissingCargoStdout)?;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || read_cargo_lines(stdout, sender));
    let mut messages = Vec::new();

    loop {
        if cancellation.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Ok(CargoRunOutcome {
                status: CargoRunStatus::Cancelled,
                messages: Vec::new(),
            });
        }
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(line)) => {
                if let Some(message) = parse_cargo_json_line(&line)? {
                    messages.push(message);
                }
            }
            Ok(Err(error)) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(error);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    reader.join().map_err(|_| BuildError::CargoReaderPanicked)?;
    let status = child
        .wait()
        .map_err(|error| BuildError::CargoWait(error.to_string()))?;
    Ok(CargoRunOutcome {
        status: if status.success() {
            CargoRunStatus::Succeeded
        } else {
            CargoRunStatus::Failed(status.code())
        },
        messages,
    })
}

/// Runs `cargo metadata` with structured arguments and maps its bounded JSON output.
///
/// # Errors
///
/// Returns an error when Cargo cannot start, its metadata violates the bounded
/// protocol, or process output cannot be read safely.
pub fn run_cargo_metadata(
    invocation: &CargoInvocation,
    cancellation: &BuildCancellation,
) -> Result<CargoMetadataOutcome, BuildError> {
    if invocation.command != CargoCommand::Metadata {
        return Err(BuildError::UnexpectedCargoCommand {
            expected: CargoCommand::Metadata,
            received: invocation.command,
        });
    }
    if cancellation.is_cancelled() {
        return Ok(CargoMetadataOutcome {
            status: CargoRunStatus::Cancelled,
            snapshot: None,
            content_hash: None,
        });
    }

    let plan = invocation.command_plan();
    let mut command = Command::new(&plan.program);
    command
        .current_dir(&plan.workspace)
        .args(&plan.arguments)
        .env_clear()
        .envs(invocation.environment.values())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| BuildError::CargoSpawn(error.to_string()))?;
    let stdout = child.stdout.take().ok_or(BuildError::MissingCargoStdout)?;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let _ = sender.send(read_bounded_stream(stdout, MAX_CARGO_METADATA_BYTES));
    });
    let bytes = loop {
        if cancellation.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Ok(CargoMetadataOutcome {
                status: CargoRunStatus::Cancelled,
                snapshot: None,
                content_hash: None,
            });
        }
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(result) => break result?,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(BuildError::CargoReaderPanicked),
        }
    };
    reader.join().map_err(|_| BuildError::CargoReaderPanicked)?;
    let status = child
        .wait()
        .map_err(|error| BuildError::CargoWait(error.to_string()))?;
    if !status.success() {
        return Ok(CargoMetadataOutcome {
            status: CargoRunStatus::Failed(status.code()),
            snapshot: None,
            content_hash: None,
        });
    }
    let text = String::from_utf8(bytes).map_err(|_| BuildError::CargoOutputNotUtf8)?;
    let snapshot = parse_cargo_metadata(&text)?;
    let content_hash = blake3::hash(text.as_bytes()).to_hex().to_string();
    Ok(CargoMetadataOutcome {
        status: CargoRunStatus::Succeeded,
        snapshot: Some(snapshot),
        content_hash: Some(content_hash),
    })
}

/// Hashes a file through a fixed byte limit before allocating its full contents.
///
/// # Errors
///
/// Returns an error when the file cannot be read or exceeds the public build-input limit.
pub fn hash_file_bounded(path: &Path) -> Result<String, BuildError> {
    let mut file = File::open(path).map_err(|error| BuildError::InputRead(error.to_string()))?;
    let mut hasher = blake3::Hasher::new();
    let mut bytes_read = 0_usize;
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| BuildError::InputRead(error.to_string()))?;
        if read == 0 {
            break;
        }
        bytes_read = bytes_read.saturating_add(read);
        if bytes_read > MAX_CARGO_INPUT_BYTES {
            return Err(BuildError::InputTooLarge(bytes_read));
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Parses bounded `cargo metadata --format-version=1` output into Meridian-owned types.
///
/// # Errors
///
/// Returns an error for oversized, malformed, or structurally incomplete metadata.
pub fn parse_cargo_metadata(text: &str) -> Result<CargoMetadataSnapshot, BuildError> {
    if text.len() > MAX_CARGO_METADATA_BYTES {
        return Err(BuildError::CargoMetadataTooLarge(text.len()));
    }
    let raw: RawCargoMetadata = serde_json::from_str(text)
        .map_err(|error| BuildError::MalformedCargoMetadata(error.to_string()))?;
    if raw.packages.len() > MAX_CARGO_PACKAGES {
        return Err(BuildError::TooManyCargoPackages(raw.packages.len()));
    }
    validate_text("Cargo metadata workspace root", &raw.workspace_root)?;
    let mut workspace_members = raw.workspace_members;
    for member in &workspace_members {
        validate_text("Cargo metadata workspace member", member)?;
    }
    workspace_members.sort_unstable();
    workspace_members.dedup();
    let mut packages = Vec::with_capacity(raw.packages.len());
    for package in raw.packages {
        validate_text("Cargo metadata package ID", &package.id)?;
        validate_text("Cargo metadata package name", &package.name)?;
        validate_text("Cargo metadata manifest path", &package.manifest_path)?;
        if package.targets.len() > MAX_CARGO_TARGETS {
            return Err(BuildError::TooManyCargoTargets(package.targets.len()));
        }
        let mut targets = Vec::with_capacity(package.targets.len());
        for target in package.targets {
            validate_text("Cargo metadata target name", &target.name)?;
            if target.kind.len() > MAX_CARGO_TARGET_KINDS {
                return Err(BuildError::TooManyCargoTargetKinds(target.kind.len()));
            }
            for kind in &target.kind {
                validate_text("Cargo metadata target kind", kind)?;
            }
            targets.push(CargoTarget {
                name: target.name,
                kinds: target.kind,
            });
        }
        packages.push(CargoPackage {
            id: package.id,
            name: package.name,
            manifest_path: package.manifest_path,
            targets,
        });
    }
    packages.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(CargoMetadataSnapshot {
        workspace_root: raw.workspace_root,
        workspace_members,
        packages,
    })
}

/// Parses one bounded Cargo JSON message line into Meridian-owned types.
///
/// # Errors
///
/// Returns an error for oversized, invalid UTF-8, malformed, or invalid Cargo messages.
pub fn parse_cargo_json_line(line: &str) -> Result<Option<CargoMessage>, BuildError> {
    if line.len() > MAX_CARGO_JSON_LINE_BYTES {
        return Err(BuildError::CargoOutputTooLarge(line.len()));
    }
    let raw: RawCargoMessage = serde_json::from_str(line)
        .map_err(|error| BuildError::MalformedCargoJson(error.to_string()))?;
    match raw.reason.as_str() {
        "compiler-message" => {
            let message = raw
                .message
                .ok_or(BuildError::MissingCargoField("compiler-message.message"))?;
            Ok(Some(CargoMessage::Diagnostic(map_diagnostic(message)?)))
        }
        "compiler-artifact" => Ok(Some(map_artifact(raw)?)),
        "build-finished" => Ok(Some(CargoMessage::Finished {
            success: raw.success.unwrap_or(false),
        })),
        _ => Ok(None),
    }
}

#[derive(Deserialize)]
struct RawCargoMessage {
    reason: String,
    package_id: Option<String>,
    message: Option<RawCargoDiagnostic>,
    target: Option<RawCargoTarget>,
    filenames: Option<Vec<String>>,
    executable: Option<String>,
    success: Option<bool>,
}

#[derive(Deserialize)]
struct RawCargoMetadata {
    workspace_root: String,
    workspace_members: Vec<String>,
    packages: Vec<RawCargoPackage>,
}

#[derive(Deserialize)]
struct RawCargoPackage {
    id: String,
    name: String,
    manifest_path: String,
    targets: Vec<RawCargoTarget>,
}

#[derive(Deserialize)]
struct RawCargoDiagnostic {
    code: Option<RawCargoDiagnosticCode>,
    level: String,
    message: String,
    rendered: Option<String>,
}

#[derive(Deserialize)]
struct RawCargoDiagnosticCode {
    code: Option<String>,
}

#[derive(Deserialize)]
struct RawCargoTarget {
    name: String,
    #[serde(default)]
    kind: Vec<String>,
}

fn map_diagnostic(raw: RawCargoDiagnostic) -> Result<BuildDiagnostic, BuildError> {
    validate_text("Cargo diagnostic message", &raw.message)?;
    let rendered = raw
        .rendered
        .map(|rendered| {
            validate_text("Cargo rendered diagnostic", &rendered)?;
            Ok(redact_sensitive_assignments(&rendered))
        })
        .transpose()?;
    let code = raw.code.and_then(|code| code.code);
    if let Some(code) = &code {
        validate_text("Cargo diagnostic code", code)?;
    }
    Ok(BuildDiagnostic {
        code,
        severity: match raw.level.as_str() {
            "error" | "failure-note" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            _ => DiagnosticSeverity::Note,
        },
        message: redact_sensitive_assignments(&raw.message),
        rendered,
    })
}

fn map_artifact(raw: RawCargoMessage) -> Result<CargoMessage, BuildError> {
    let package_id = raw.package_id.ok_or(BuildError::MissingCargoField(
        "compiler-artifact.package_id",
    ))?;
    let target_name = raw
        .target
        .map(|target| target.name)
        .ok_or(BuildError::MissingCargoField(
            "compiler-artifact.target.name",
        ))?;
    let filenames = raw
        .filenames
        .ok_or(BuildError::MissingCargoField("compiler-artifact.filenames"))?;
    if filenames.len() > MAX_FILENAMES {
        return Err(BuildError::TooManyArtifactFilenames(filenames.len()));
    }
    validate_text("Cargo package ID", &package_id)?;
    validate_text("Cargo target name", &target_name)?;
    for filename in &filenames {
        validate_text("Cargo artifact filename", filename)?;
    }
    if let Some(executable) = &raw.executable {
        validate_text("Cargo artifact executable", executable)?;
    }
    Ok(CargoMessage::Artifact(CargoArtifact {
        package_id,
        target_name,
        filenames,
        executable: raw.executable,
    }))
}

#[allow(clippy::needless_pass_by_value)]
fn read_cargo_lines(mut stdout: impl Read, sender: mpsc::Sender<Result<String, BuildError>>) {
    let mut bytes = Vec::with_capacity(512);
    loop {
        match read_bounded_line(&mut stdout, &mut bytes) {
            Ok(false) => break,
            Ok(true) => {
                if let Ok(mut line) = String::from_utf8(bytes.clone()) {
                    while matches!(line.chars().last(), Some('\n' | '\r')) {
                        line.pop();
                    }
                    if sender.send(Ok(line)).is_err() {
                        break;
                    }
                } else {
                    let _ = sender.send(Err(BuildError::CargoOutputNotUtf8));
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send(Err(error));
                break;
            }
        }
    }
}

fn read_bounded_line(reader: &mut impl Read, output: &mut Vec<u8>) -> Result<bool, BuildError> {
    output.clear();
    let mut byte = [0_u8; 1];
    loop {
        let read = reader
            .read(&mut byte)
            .map_err(|error| BuildError::CargoRead(error.to_string()))?;
        if read == 0 {
            return Ok(!output.is_empty());
        }
        if output.len() == MAX_CARGO_JSON_LINE_BYTES {
            return Err(BuildError::CargoOutputTooLarge(
                MAX_CARGO_JSON_LINE_BYTES.saturating_add(1),
            ));
        }
        output.push(byte[0]);
        if byte[0] == b'\n' {
            return Ok(true);
        }
    }
}

fn read_bounded_stream(mut reader: impl Read, limit: usize) -> Result<Vec<u8>, BuildError> {
    let mut output = Vec::with_capacity(8_192);
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| BuildError::CargoRead(error.to_string()))?;
        if read == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(read) > limit {
            return Err(BuildError::CargoMetadataTooLarge(
                output.len().saturating_add(read),
            ));
        }
        output.extend_from_slice(&buffer[..read]);
    }
}

fn validate_identity(input: &BuildIdentityInput) -> Result<(), BuildError> {
    validate_text("source checkpoint", &input.source_checkpoint)?;
    validate_text("resolved profile", &input.resolved_profile)?;
    validate_text(
        "Cargo metadata and lock hash",
        &input.cargo_metadata_and_lock,
    )?;
    validate_text("toolchain version", &input.toolchain_version)?;
    validate_text("target and capabilities", &input.target_and_capabilities)?;
    if input.root_node_ids.is_empty() {
        return Err(BuildError::NoRootNodes);
    }
    let mut roots = BTreeSet::new();
    for root in &input.root_node_ids {
        validate_text("root node ID", root)?;
        if !roots.insert(root) {
            return Err(BuildError::DuplicateRootNode(root.clone()));
        }
    }
    for (name, value) in &input.environment_allowlist {
        if !is_allowlisted_environment(name) {
            return Err(BuildError::EnvironmentNotAllowlisted(name.clone()));
        }
        validate_text("environment value", value)?;
    }
    Ok(())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), BuildError> {
    if value.is_empty() {
        return Err(BuildError::EmptyField(field));
    }
    if value.len() > MAX_FIELD_BYTES {
        return Err(BuildError::FieldTooLong {
            field,
            length: value.len(),
        });
    }
    if value.contains('\0') {
        return Err(BuildError::NulByte(field));
    }
    Ok(())
}

fn validate_build_id(build_id: &BuildId) -> Result<(), BuildError> {
    let value = build_id.as_str();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(BuildError::InvalidBuildId(value.to_owned()));
    }
    Ok(())
}

fn hash_field(hasher: &mut blake3::Hasher, name: &str, value: &str) {
    hasher.update(&(name.len() as u64).to_le_bytes());
    hasher.update(name.as_bytes());
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn is_allowlisted_environment(name: &str) -> bool {
    ENVIRONMENT_ALLOWLIST.contains(&name)
}

fn validate_transition(current: BuildPhase, next: BuildPhase) -> Result<(), BuildError> {
    let valid = matches!(
        (current, next),
        (
            BuildPhase::Queued,
            BuildPhase::Queued
                | BuildPhase::Resolving
                | BuildPhase::CancelRequested
                | BuildPhase::Failed
                | BuildPhase::WorkerLost
                | BuildPhase::Superseded
        ) | (
            BuildPhase::Resolving,
            BuildPhase::Ready
                | BuildPhase::CancelRequested
                | BuildPhase::Failed
                | BuildPhase::WorkerLost
                | BuildPhase::Superseded
        ) | (
            BuildPhase::Ready,
            BuildPhase::Running
                | BuildPhase::CancelRequested
                | BuildPhase::Failed
                | BuildPhase::WorkerLost
                | BuildPhase::Superseded
        ) | (
            BuildPhase::Running,
            BuildPhase::Running
                | BuildPhase::Succeeded
                | BuildPhase::Failed
                | BuildPhase::WorkerLost
                | BuildPhase::Superseded
                | BuildPhase::CancelRequested
        ) | (
            BuildPhase::CancelRequested,
            BuildPhase::Cancelled
                | BuildPhase::Failed
                | BuildPhase::WorkerLost
                | BuildPhase::Superseded
        )
    );
    if valid {
        Ok(())
    } else {
        Err(BuildError::InvalidTransition { current, next })
    }
}

fn redact_sensitive_assignments(input: &str) -> String {
    const KEYS: &[&str] = &["authorization:", "password=", "secret=", "token="];
    let lower = input.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some((start, key)) = KEYS
        .iter()
        .filter_map(|key| {
            lower[cursor..]
                .find(key)
                .map(|offset| (cursor + offset, *key))
        })
        .min_by_key(|(start, _)| *start)
    {
        result.push_str(&input[cursor..start]);
        let end_of_key = start + key.len();
        result.push_str(&input[start..end_of_key]);
        result.push_str("[REDACTED]");
        let value_end = input[end_of_key..]
            .find(char::is_whitespace)
            .map_or(input.len(), |offset| end_of_key + offset);
        cursor = value_end;
    }
    result.push_str(&input[cursor..]);
    result
}

/// Errors surfaced by the Meridian-owned build-service boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildError {
    /// A required text field was empty.
    EmptyField(&'static str),
    /// A text field exceeded the bounded service input limit.
    FieldTooLong { field: &'static str, length: usize },
    /// A text field contained a NUL byte.
    NulByte(&'static str),
    /// No root node was declared for identity calculation.
    NoRootNodes,
    /// Root nodes were duplicated before identity calculation.
    DuplicateRootNode(String),
    /// An environment name falls outside the explicit allowlist.
    EnvironmentNotAllowlisted(String),
    /// The caller registered the same process-local operation twice.
    DuplicateOperation(OperationId),
    /// The caller referenced an unknown operation.
    UnknownOperation(OperationId),
    /// Lifecycle transition violates the declared operation state machine.
    InvalidTransition {
        current: BuildPhase,
        next: BuildPhase,
    },
    /// A Cargo message arrived before the operation reached Running.
    CargoMessageOutsideRunning(BuildPhase),
    /// An external event has mismatched build or trace identity.
    MismatchedEventIdentity,
    /// An external event has a node ID outside the registered request.
    MismatchedNodeId,
    /// An external event skipped or replayed a sequence number.
    StaleEventSequence { expected: u64, received: u64 },
    /// Workspace path was empty.
    EmptyWorkspace,
    /// Structured Cargo argument list exceeded the service limit.
    TooManyArguments(usize),
    /// Caller attempted to override an adapter-owned Cargo protocol argument.
    ReservedCargoArgument(String),
    /// Cargo could not be spawned.
    CargoSpawn(String),
    /// Cargo stdout pipe was unavailable after a successful spawn.
    MissingCargoStdout,
    /// Cargo stdout reader terminated unexpectedly.
    CargoReaderPanicked,
    /// Cargo process could not be waited on.
    CargoWait(String),
    /// Cargo stdout could not be read.
    CargoRead(String),
    /// Cargo emitted a line larger than the declared protocol limit.
    CargoOutputTooLarge(usize),
    /// Cargo output was not valid UTF-8 JSON text.
    CargoOutputNotUtf8,
    /// Cargo emitted malformed JSON.
    MalformedCargoJson(String),
    /// Required Cargo JSON field was absent.
    MissingCargoField(&'static str),
    /// Cargo input file could not be read.
    InputRead(String),
    /// Cargo input file exceeded the declared byte limit.
    InputTooLarge(usize),
    /// Cargo emitted too many artifact filenames in one event.
    TooManyArtifactFilenames(usize),
    /// Cargo command does not match the selected adapter protocol.
    UnexpectedCargoCommand {
        /// Command required by the called adapter.
        expected: CargoCommand,
        /// Command supplied by the caller.
        received: CargoCommand,
    },
    /// Cargo metadata exceeded the declared byte limit.
    CargoMetadataTooLarge(usize),
    /// Cargo metadata was malformed JSON.
    MalformedCargoMetadata(String),
    /// Cargo metadata listed too many packages.
    TooManyCargoPackages(usize),
    /// Cargo metadata package listed too many targets.
    TooManyCargoTargets(usize),
    /// Cargo metadata target listed too many kinds.
    TooManyCargoTargetKinds(usize),
    /// Service snapshot exceeded the declared byte limit.
    SnapshotTooLarge(usize),
    /// Service snapshot could not be serialized.
    SnapshotSerialization(String),
    /// Service snapshot was malformed JSON.
    MalformedSnapshot(String),
    /// Service snapshot protocol version is unsupported.
    UnsupportedSnapshotVersion(u16),
    /// Service snapshot exceeded the operation limit.
    TooManyOperations(usize),
    /// Service snapshot contained a duplicated operation ID.
    DuplicateSnapshotOperation(OperationId),
    /// Service snapshot contained a malformed build identity.
    InvalidBuildId(String),
}

impl Display for BuildError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "{field} must not be empty"),
            Self::FieldTooLong { field, length } => {
                write!(
                    formatter,
                    "{field} exceeds the {MAX_FIELD_BYTES}-byte limit ({length} bytes)"
                )
            }
            Self::NulByte(field) => write!(formatter, "{field} contains a NUL byte"),
            Self::NoRootNodes => {
                formatter.write_str("a build identity needs at least one root node")
            }
            Self::DuplicateRootNode(node) => {
                write!(formatter, "build identity duplicates root node {node}")
            }
            Self::EnvironmentNotAllowlisted(name) => {
                write!(formatter, "environment variable {name} is not allowlisted")
            }
            Self::DuplicateOperation(id) => write!(formatter, "operation {id} already exists"),
            Self::UnknownOperation(id) => write!(formatter, "operation {id} is unknown"),
            Self::InvalidTransition { current, next } => {
                write!(
                    formatter,
                    "invalid build transition from {current:?} to {next:?}"
                )
            }
            Self::CargoMessageOutsideRunning(phase) => {
                write!(
                    formatter,
                    "Cargo message cannot arrive while build is {phase:?}"
                )
            }
            Self::MismatchedEventIdentity => {
                formatter.write_str("event build or trace identity does not match")
            }
            Self::MismatchedNodeId => {
                formatter.write_str("event node ID does not match the request")
            }
            Self::StaleEventSequence { expected, received } => {
                write!(
                    formatter,
                    "event sequence {received} is stale; expected {expected}"
                )
            }
            Self::EmptyWorkspace => formatter.write_str("Cargo workspace path must not be empty"),
            Self::TooManyArguments(count) => write!(
                formatter,
                "Cargo invocation has too many arguments ({count})"
            ),
            Self::ReservedCargoArgument(argument) => {
                write!(
                    formatter,
                    "Cargo argument {argument} is owned by the Meridian adapter"
                )
            }
            Self::CargoSpawn(message) => write!(formatter, "failed to start Cargo: {message}"),
            Self::MissingCargoStdout => formatter.write_str("Cargo stdout pipe was unavailable"),
            Self::CargoReaderPanicked => formatter.write_str("Cargo stdout reader panicked"),
            Self::CargoWait(message) => {
                write!(formatter, "failed while waiting for Cargo: {message}")
            }
            Self::CargoRead(message) => {
                write!(formatter, "failed while reading Cargo output: {message}")
            }
            Self::CargoOutputTooLarge(length) => {
                write!(
                    formatter,
                    "Cargo JSON line exceeds {MAX_CARGO_JSON_LINE_BYTES} bytes ({length} bytes)"
                )
            }
            Self::CargoOutputNotUtf8 => formatter.write_str("Cargo JSON output is not UTF-8"),
            Self::MalformedCargoJson(message) => {
                write!(formatter, "malformed Cargo JSON: {message}")
            }
            Self::MissingCargoField(field) => {
                write!(formatter, "Cargo JSON field {field} is missing")
            }
            Self::InputRead(message) => write!(formatter, "failed to read build input: {message}"),
            Self::InputTooLarge(length) => {
                write!(
                    formatter,
                    "build input exceeds {MAX_CARGO_INPUT_BYTES} bytes ({length} bytes)"
                )
            }
            Self::TooManyArtifactFilenames(count) => {
                write!(formatter, "Cargo artifact has too many filenames ({count})")
            }
            Self::UnexpectedCargoCommand { expected, received } => write!(
                formatter,
                "Cargo adapter expects {}, received {}",
                expected.as_str(),
                received.as_str()
            ),
            Self::CargoMetadataTooLarge(length) => write!(
                formatter,
                "Cargo metadata exceeds {MAX_CARGO_METADATA_BYTES} bytes ({length} bytes)"
            ),
            Self::MalformedCargoMetadata(message) => {
                write!(formatter, "malformed Cargo metadata: {message}")
            }
            Self::TooManyCargoPackages(count) => {
                write!(formatter, "Cargo metadata has too many packages ({count})")
            }
            Self::TooManyCargoTargets(count) => {
                write!(
                    formatter,
                    "Cargo metadata package has too many targets ({count})"
                )
            }
            Self::TooManyCargoTargetKinds(count) => {
                write!(
                    formatter,
                    "Cargo metadata target has too many kinds ({count})"
                )
            }
            Self::SnapshotTooLarge(length) => write!(
                formatter,
                "build-service snapshot exceeds {MAX_BUILD_SNAPSHOT_BYTES} bytes ({length} bytes)"
            ),
            Self::SnapshotSerialization(message) => {
                write!(
                    formatter,
                    "failed to serialize build-service snapshot: {message}"
                )
            }
            Self::MalformedSnapshot(message) => {
                write!(formatter, "malformed build-service snapshot: {message}")
            }
            Self::UnsupportedSnapshotVersion(version) => write!(
                formatter,
                "build-service snapshot protocol version {version} is unsupported"
            ),
            Self::TooManyOperations(count) => {
                write!(formatter, "build service has too many operations ({count})")
            }
            Self::DuplicateSnapshotOperation(operation_id) => {
                write!(formatter, "snapshot duplicates operation {operation_id}")
            }
            Self::InvalidBuildId(build_id) => {
                write!(formatter, "snapshot build ID {build_id} is malformed")
            }
        }
    }
}

impl Error for BuildError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> BuildIdentityInput {
        BuildIdentityInput {
            source_checkpoint: "abc123".to_owned(),
            resolved_profile: "debug".to_owned(),
            cargo_metadata_and_lock: "lock-hash".to_owned(),
            toolchain_version: "rustc 1.90.0".to_owned(),
            target_and_capabilities: "aarch64-apple-darwin/default".to_owned(),
            environment_allowlist: BTreeMap::new(),
            root_node_ids: vec!["cargo-check".to_owned()],
        }
    }

    fn request(operation_id: u64) -> BuildRequest {
        BuildRequest::new(
            &identity(),
            OperationId::new(operation_id),
            TraceId::new(7),
            BuildNode::cargo_check(BuildNodeId::new("cargo-check").expect("node"), "cargo 1.90")
                .expect("node"),
        )
        .expect("request")
    }

    #[test]
    fn build_identity_is_order_independent_for_roots_and_environment() {
        let mut first = identity();
        first.root_node_ids = vec!["b".to_owned(), "a".to_owned()];
        first
            .environment_allowlist
            .insert("PATH".to_owned(), "/usr/bin".to_owned());
        first
            .environment_allowlist
            .insert("HOME".to_owned(), "/tmp/home".to_owned());
        let mut second = first.clone();
        second.root_node_ids.reverse();
        assert_eq!(
            BuildId::derive(&first).expect("first"),
            BuildId::derive(&second).expect("second")
        );
    }

    #[test]
    fn identity_rejects_duplicate_roots_and_unallowlisted_environment() {
        let mut duplicate = identity();
        duplicate.root_node_ids.push("cargo-check".to_owned());
        assert!(matches!(
            BuildId::derive(&duplicate),
            Err(BuildError::DuplicateRootNode(_))
        ));

        let mut environment = identity();
        environment
            .environment_allowlist
            .insert("AWS_SECRET_ACCESS_KEY".to_owned(), "not-allowed".to_owned());
        assert!(matches!(
            BuildId::derive(&environment),
            Err(BuildError::EnvironmentNotAllowlisted(_))
        ));
    }

    #[test]
    fn identity_changes_when_an_allowlisted_environment_value_changes() {
        let mut first = identity();
        first
            .environment_allowlist
            .insert("PATH".to_owned(), "/one".to_owned());
        let mut second = first.clone();
        second
            .environment_allowlist
            .insert("PATH".to_owned(), "/two".to_owned());
        assert_ne!(
            BuildId::derive(&first).expect("first"),
            BuildId::derive(&second).expect("second")
        );
    }

    #[test]
    fn lifecycle_rejects_stale_events_and_post_cancel_success() {
        let mut service = BuildService::default();
        let queued = service.submit(request(1)).expect("queued");
        assert_eq!(queued.sequence, 1);
        let resolving = service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        assert_eq!(resolving.sequence, 2);
        assert!(matches!(
            service.accept_external_event(&resolving),
            Err(BuildError::StaleEventSequence { .. })
        ));
        service
            .transition(OperationId::new(1), BuildPhase::CancelRequested, 5)
            .expect("cancel requested");
        service
            .transition(OperationId::new(1), BuildPhase::Cancelled, 5)
            .expect("cancelled");
        assert!(matches!(
            service.transition(OperationId::new(1), BuildPhase::Succeeded, 100),
            Err(BuildError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn snapshot_marks_interrupted_operations_worker_lost_and_rejects_late_results() {
        let mut service = BuildService::default();
        service.submit(request(1)).expect("queued");
        service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(1), BuildPhase::Ready, 10)
            .expect("ready");
        service
            .transition(OperationId::new(1), BuildPhase::Running, 20)
            .expect("running");
        let snapshot = service.snapshot_json().expect("snapshot");
        let mut recovered = BuildService::restore_json(&snapshot).expect("recovery");
        assert_eq!(recovered.recovery_events.len(), 1);
        assert_eq!(recovered.recovery_events[0].phase, BuildPhase::WorkerLost);
        assert_eq!(
            recovered.service.phase(OperationId::new(1)).expect("phase"),
            BuildPhase::WorkerLost
        );
        assert!(matches!(
            recovered
                .service
                .transition(OperationId::new(1), BuildPhase::Succeeded, 100),
            Err(BuildError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn snapshot_rejects_invalid_identity_and_oversized_input() {
        let invalid = r#"{"version":1,"operations":[{"request":{"build_id":"not-a-build-id","operation_id":1,"trace_id":7,"root_node":{"id":"cargo-check","kind":"CargoCheck","input_hashes":[],"tool_id_version":"cargo 1.90","declared_environment":[],"dependencies":[]}},"phase":"Running","last_sequence":4}]}"#;
        assert!(matches!(
            BuildService::restore_json(invalid),
            Err(BuildError::InvalidBuildId(_))
        ));
        let oversized = "x".repeat(MAX_BUILD_SNAPSHOT_BYTES.saturating_add(1));
        assert!(matches!(
            BuildService::restore_json(&oversized),
            Err(BuildError::SnapshotTooLarge(_))
        ));
    }

    #[test]
    fn cargo_message_requires_running_operation() {
        let mut service = BuildService::default();
        service.submit(request(1)).expect("queued");
        assert!(matches!(
            service.record_cargo_message(
                OperationId::new(1),
                CargoMessage::Finished { success: true }
            ),
            Err(BuildError::CargoMessageOutsideRunning(BuildPhase::Queued))
        ));
    }

    #[test]
    fn parser_maps_diagnostics_artifacts_and_redacts_assignments() {
        let diagnostic = parse_cargo_json_line(
            r#"{"reason":"compiler-message","message":{"code":{"code":"E0001"},"level":"error","message":"token=abc","rendered":"error: password=hunter2"}}"#,
        )
        .expect("diagnostic")
        .expect("message");
        let CargoMessage::Diagnostic(diagnostic) = diagnostic else {
            panic!("expected diagnostic");
        };
        assert_eq!(diagnostic.code.as_deref(), Some("E0001"));
        assert_eq!(diagnostic.message, "token=[REDACTED]");
        assert_eq!(
            diagnostic.rendered.as_deref(),
            Some("error: password=[REDACTED]")
        );

        let artifact = parse_cargo_json_line(
            r#"{"reason":"compiler-artifact","package_id":"path+file:///repo#crate@0.1.0","target":{"name":"crate"},"filenames":["/tmp/crate.rlib"],"executable":null}"#,
        )
        .expect("artifact")
        .expect("message");
        assert!(matches!(artifact, CargoMessage::Artifact(_)));
    }

    #[test]
    fn parser_rejects_malformed_and_oversized_messages() {
        assert!(matches!(
            parse_cargo_json_line("not json"),
            Err(BuildError::MalformedCargoJson(_))
        ));
        let oversized = "x".repeat(MAX_CARGO_JSON_LINE_BYTES.saturating_add(1));
        assert!(matches!(
            parse_cargo_json_line(&oversized),
            Err(BuildError::CargoOutputTooLarge(_))
        ));
    }

    #[test]
    fn metadata_parser_preserves_workspace_package_and_target_contracts() {
        let metadata = parse_cargo_metadata(
            r#"{"workspace_root":"/repo","workspace_members":["path+file:///repo#crate@0.1.0"],"packages":[{"id":"path+file:///repo#crate@0.1.0","name":"crate","manifest_path":"/repo/Cargo.toml","targets":[{"name":"crate","kind":["lib"]}]}]}"#,
        )
        .expect("metadata");
        assert_eq!(metadata.workspace_root, "/repo");
        assert_eq!(metadata.packages[0].targets[0].kinds, ["lib"]);
    }

    #[test]
    fn adapters_reject_a_mismatched_cargo_command() {
        let metadata = CargoInvocation::new(
            "/workspace",
            CargoCommand::Metadata,
            Vec::new(),
            CargoEnvironment::default(),
        )
        .expect("metadata invocation");
        assert!(matches!(
            run_cargo_json(&metadata, &BuildCancellation::default()),
            Err(BuildError::UnexpectedCargoCommand { .. })
        ));
    }

    #[test]
    fn structured_cargo_plan_clears_ambient_protocol_overrides() {
        let invocation = CargoInvocation::new(
            "/workspace",
            CargoCommand::Check,
            vec!["-p".to_owned(), "meridian-build".to_owned()],
            CargoEnvironment::default(),
        )
        .expect("invocation");
        let plan = invocation.command_plan();
        assert_eq!(plan.arguments[0], "check");
        assert!(plan
            .arguments
            .iter()
            .any(|argument| argument == "--message-format=json"));
        assert!(matches!(
            CargoInvocation::new(
                "/workspace",
                CargoCommand::Check,
                vec!["--message-format=human".to_owned()],
                CargoEnvironment::default(),
            ),
            Err(BuildError::ReservedCargoArgument(_))
        ));
    }

    #[test]
    fn pre_cancelled_cargo_never_spawns_a_process() {
        let invocation = CargoInvocation::new(
            "/workspace",
            CargoCommand::Check,
            Vec::new(),
            CargoEnvironment::default(),
        )
        .expect("invocation");
        let cancellation = BuildCancellation::default();
        cancellation.cancel();
        let result = run_cargo_json(&invocation, &cancellation).expect("cancelled outcome");
        assert_eq!(result.status, CargoRunStatus::Cancelled);
    }
}
