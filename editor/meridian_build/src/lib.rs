//! Meridian-owned, observable Cargo build-service foundation.
//!
//! Cargo remains the Rust build authority. This crate owns the typed request,
//! lifecycle, event, identity, and bounded Cargo JSON adapter contracts that
//! future Meridian editor and CLI surfaces share.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use meridian_core::{OperationId, TraceId};
use meridian_tasks::{Task, TaskError, TaskPool};
use serde::{Deserialize, Serialize};

/// First version of the editor/service build protocol.
pub const BUILD_PROTOCOL_VERSION: u16 = 1;
/// Maximum accepted Cargo JSON line, before JSON parsing allocates its fields.
pub const MAX_CARGO_JSON_LINE_BYTES: usize = 1_048_576;
/// Maximum accepted `cargo metadata` payload before JSON parsing allocates its fields.
pub const MAX_CARGO_METADATA_BYTES: usize = 8 * 1_024 * 1_024;
/// Maximum serialized service snapshot accepted before JSON parsing.
pub const MAX_BUILD_SNAPSHOT_BYTES: usize = 1_048_576;
/// Maximum retained Cargo stderr payload for one failed process invocation.
///
/// This reuses the service's existing snapshot boundary: fresh Cargo checks can
/// emit more than a small terminal-sized status stream, but diagnostics still
/// remain bounded before they are retained or redacted.
pub const MAX_CARGO_STDERR_BYTES: usize = MAX_BUILD_SNAPSHOT_BYTES;
/// Maximum aggregate raw Cargo JSON retained for one operation.
pub const MAX_CARGO_JSON_OUTPUT_BYTES: usize = MAX_BUILD_SNAPSHOT_BYTES;
/// Maximum Cargo JSON lines accepted for one operation.
pub const MAX_CARGO_JSON_LINES: usize = MAX_OPERATIONS;
/// Maximum accepted Cargo manifest or lockfile input for this initial service.
pub const MAX_CARGO_INPUT_BYTES: usize = 8 * 1_024 * 1_024;
const MAX_FIELD_BYTES: usize = 4_096;
const MAX_ENVIRONMENT_VALUE_BYTES: usize = 32 * 1_024;
const MAX_ARGUMENTS: usize = 64;
const MAX_FILENAMES: usize = 256;
const MAX_CARGO_PACKAGES: usize = 4_096;
const MAX_CARGO_TARGETS: usize = 256;
const MAX_CARGO_TARGET_KINDS: usize = 64;
const MAX_OPERATIONS: usize = 1_024;
const MAX_SNAPSHOT_TEMPORARY_ATTEMPTS: usize = 16;
#[cfg(unix)]
const CARGO_CANCELLATION_GRACE: Duration = Duration::from_millis(250);
#[cfg(unix)]
const CARGO_CANCELLATION_POLL: Duration = Duration::from_millis(10);
static NEXT_SNAPSHOT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

const ENVIRONMENT_ALLOWLIST: &[&str] = &[
    "CARGO_HOME",
    "HOME",
    "PATH",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "USERPROFILE",
    // Visual Studio's developer environment supplies the linker and SDK search
    // roots through these names. They remain explicit local build inputs rather
    // than becoming ambient child-process inheritance.
    "INCLUDE",
    "LIB",
    "LIBPATH",
    "UniversalCRTSdkDir",
    "UCRTVersion",
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "VSINSTALLDIR",
    "WindowsSdkDir",
    "WindowsSDKVersion",
    "VSCMD_ARG_HOST_ARCH",
    "VSCMD_ARG_TGT_ARCH",
    "VSCMD_VER",
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
        hash_field(
            &mut hasher,
            "build-graph-contract",
            &input.build_graph_contract,
        );
        hash_field(
            &mut hasher,
            "command-argument-count",
            &input.command_arguments.len().to_string(),
        );
        for argument in &input.command_arguments {
            hash_field(&mut hasher, "command-argument", argument);
        }
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
    /// Canonical hash of the validated graph that will execute under this ID.
    pub build_graph_contract: String,
    /// Ordered, structured adapter arguments that affect the requested command.
    pub command_arguments: Vec<String>,
    /// Declared Rust/Cargo toolchain version.
    pub toolchain_version: String,
    /// Target triple plus selected capability profile.
    pub target_and_capabilities: String,
    /// Explicit environment values admitted into the identity.
    pub environment_allowlist: BTreeMap<String, String>,
    /// Ordered roots requested by the caller; order does not affect identity.
    pub root_node_ids: Vec<String>,
}

/// Secret-safe, durable description of the inputs that produced one `BuildId`.
///
/// Raw command arguments and environment values remain transient because they
/// can contain credentials or host-specific paths. Their ordered digests let a
/// host compare declared inputs without persisting those values.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildInputProvenance {
    source_checkpoint: String,
    resolved_profile: String,
    cargo_metadata_and_lock: String,
    command_arguments_hash: String,
    #[serde(default)]
    build_graph_contract: Option<String>,
    #[serde(default)]
    build_graph: Option<BuildGraphProvenance>,
    toolchain_version: String,
    target_and_capabilities: String,
    environment_value_hashes: BTreeMap<String, String>,
    root_node_ids: Vec<String>,
}

impl BuildInputProvenance {
    fn from_identity(identity: &BuildIdentityInput) -> Result<Self, BuildError> {
        validate_identity(identity)?;
        let mut environment_value_hashes = BTreeMap::new();
        for (name, value) in &identity.environment_allowlist {
            environment_value_hashes.insert(
                name.clone(),
                hash_provenance_value("environment", name, value),
            );
        }
        let mut root_node_ids = identity.root_node_ids.clone();
        root_node_ids.sort_unstable();
        Ok(Self {
            source_checkpoint: identity.source_checkpoint.clone(),
            resolved_profile: identity.resolved_profile.clone(),
            cargo_metadata_and_lock: identity.cargo_metadata_and_lock.clone(),
            command_arguments_hash: hash_provenance_values(
                "command-arguments",
                &identity.command_arguments,
            ),
            build_graph_contract: Some(identity.build_graph_contract.clone()),
            build_graph: None,
            toolchain_version: identity.toolchain_version.clone(),
            target_and_capabilities: identity.target_and_capabilities.clone(),
            environment_value_hashes,
            root_node_ids,
        })
    }

    fn from_identity_and_graph(
        identity: &BuildIdentityInput,
        graph: &BuildGraph,
    ) -> Result<Self, BuildError> {
        graph.validate_identity(identity)?;
        let mut provenance = Self::from_identity(identity)?;
        provenance.build_graph = Some(graph.provenance());
        Ok(provenance)
    }

    /// Returns the immutable source checkpoint recorded for this request.
    #[must_use]
    pub fn source_checkpoint(&self) -> &str {
        &self.source_checkpoint
    }

    /// Returns the resolved build profile recorded for this request.
    #[must_use]
    pub fn resolved_profile(&self) -> &str {
        &self.resolved_profile
    }

    /// Returns the declared Cargo metadata and lockfile identity input.
    #[must_use]
    pub fn cargo_metadata_and_lock(&self) -> &str {
        &self.cargo_metadata_and_lock
    }

    /// Returns the ordered command-argument digest without exposing raw arguments.
    #[must_use]
    pub fn command_arguments_hash(&self) -> &str {
        &self.command_arguments_hash
    }

    /// Returns the canonical build-graph hash when this request recorded one.
    ///
    /// Older local snapshots predate graph-contract provenance and therefore
    /// return `None`; new requests require this field before publication.
    #[must_use]
    pub fn build_graph_contract(&self) -> Option<&str> {
        self.build_graph_contract.as_deref()
    }

    /// Returns the full declared execution graph when this request recorded one.
    ///
    /// New requests retain this canonical manifest without raw environment values.
    /// Older local snapshots predate graph manifests and therefore return `None`.
    #[must_use]
    pub fn build_graph(&self) -> Option<&BuildGraphProvenance> {
        self.build_graph.as_ref()
    }

    /// Returns the declared toolchain identity.
    #[must_use]
    pub fn toolchain_version(&self) -> &str {
        &self.toolchain_version
    }

    /// Returns the declared target and capability profile.
    #[must_use]
    pub fn target_and_capabilities(&self) -> &str {
        &self.target_and_capabilities
    }

    /// Returns name-to-digest mappings for the declared environment values.
    #[must_use]
    pub fn environment_value_hashes(&self) -> &BTreeMap<String, String> {
        &self.environment_value_hashes
    }

    /// Returns the sorted requested root-node identifiers.
    #[must_use]
    pub fn root_node_ids(&self) -> &[String] {
        &self.root_node_ids
    }
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
    /// Run a Cargo build and consume its JSON message stream.
    CargoBuild,
    /// Compile Cargo test targets without executing a test harness.
    CargoTestNoRun,
}

impl BuildNodeKind {
    const fn identity_name(self) -> &'static str {
        match self {
            Self::CargoMetadata => "cargo-metadata",
            Self::CargoCheck => "cargo-check",
            Self::CargoBuild => "cargo-build",
            Self::CargoTestNoRun => "cargo-test-no-run",
        }
    }
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
    /// Builds a minimal Cargo-metadata node using the declared Cargo tool identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the ID or declared tool version is invalid.
    pub fn cargo_metadata(
        id: BuildNodeId,
        tool_id_version: impl Into<String>,
    ) -> Result<Self, BuildError> {
        let tool_id_version = tool_id_version.into();
        validate_text("tool ID and version", &tool_id_version)?;
        Ok(Self {
            id,
            kind: BuildNodeKind::CargoMetadata,
            input_hashes: Vec::new(),
            tool_id_version,
            declared_environment: Vec::new(),
            dependencies: Vec::new(),
        })
    }

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

    /// Builds a minimal Cargo-build node using the declared Cargo tool identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the ID or declared tool version is invalid.
    pub fn cargo_build(
        id: BuildNodeId,
        tool_id_version: impl Into<String>,
    ) -> Result<Self, BuildError> {
        let tool_id_version = tool_id_version.into();
        validate_text("tool ID and version", &tool_id_version)?;
        Ok(Self {
            id,
            kind: BuildNodeKind::CargoBuild,
            input_hashes: Vec::new(),
            tool_id_version,
            declared_environment: Vec::new(),
            dependencies: Vec::new(),
        })
    }

    /// Builds a Cargo test-compilation node using the declared Cargo tool identity.
    ///
    /// The node uses Cargo's `test --no-run` mode so its observable output remains
    /// the machine-readable compiler-message protocol rather than test-harness text.
    ///
    /// # Errors
    ///
    /// Returns an error when the ID or declared tool version is invalid.
    pub fn cargo_test_no_run(
        id: BuildNodeId,
        tool_id_version: impl Into<String>,
    ) -> Result<Self, BuildError> {
        let tool_id_version = tool_id_version.into();
        validate_text("tool ID and version", &tool_id_version)?;
        Ok(Self {
            id,
            kind: BuildNodeKind::CargoTestNoRun,
            input_hashes: Vec::new(),
            tool_id_version,
            declared_environment: Vec::new(),
            dependencies: Vec::new(),
        })
    }
}

/// Immutable, dependency-validated graph for one requested build root set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildGraph {
    nodes: BTreeMap<BuildNodeId, BuildNode>,
    requested_roots: Vec<BuildNodeId>,
}

impl BuildGraph {
    /// Validates and constructs a deterministic graph for the requested roots.
    ///
    /// # Errors
    ///
    /// Returns an error for absent/duplicate nodes or roots, invalid node fields,
    /// unknown or duplicate dependencies, dependency cycles, or nodes unrelated to
    /// every requested root.
    pub fn new(
        nodes: impl IntoIterator<Item = BuildNode>,
        requested_roots: impl IntoIterator<Item = BuildNodeId>,
    ) -> Result<Self, BuildError> {
        let mut mapped_nodes = BTreeMap::new();
        for node in nodes {
            if mapped_nodes.len() == MAX_OPERATIONS {
                return Err(BuildError::TooManyBuildGraphNodes(
                    MAX_OPERATIONS.saturating_add(1),
                ));
            }
            validate_build_node(&node)?;
            let id = node.id.clone();
            if mapped_nodes.insert(id.clone(), node).is_some() {
                return Err(BuildError::DuplicateBuildGraphNode(id));
            }
        }
        if mapped_nodes.is_empty() {
            return Err(BuildError::EmptyBuildGraph);
        }

        let mut requested_roots = requested_roots.into_iter().collect::<Vec<_>>();
        if requested_roots.is_empty() {
            return Err(BuildError::NoRootNodes);
        }
        requested_roots.sort_unstable();
        for roots in requested_roots.windows(2) {
            if roots[0] == roots[1] {
                return Err(BuildError::DuplicateRootNode(roots[0].as_str().to_owned()));
            }
        }
        for root in &requested_roots {
            if !mapped_nodes.contains_key(root) {
                return Err(BuildError::UnknownBuildGraphRoot(root.clone()));
            }
        }

        for node in mapped_nodes.values() {
            for dependency in &node.dependencies {
                if !mapped_nodes.contains_key(dependency) {
                    return Err(BuildError::UnknownBuildGraphDependency {
                        node: node.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }

        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for root in &requested_roots {
            validate_graph_dependencies(root, &mapped_nodes, &mut visiting, &mut visited)?;
        }
        if visited.len() != mapped_nodes.len() {
            if let Some(node) = mapped_nodes.keys().find(|node| !visited.contains(*node)) {
                return Err(BuildError::UnreachableBuildGraphNode(node.clone()));
            }
        }

        Ok(Self {
            nodes: mapped_nodes,
            requested_roots,
        })
    }

    /// Returns the requested roots in canonical identifier order.
    #[must_use]
    pub fn requested_roots(&self) -> &[BuildNodeId] {
        &self.requested_roots
    }

    /// Returns one immutable node by identifier.
    #[must_use]
    pub fn node(&self, node_id: &BuildNodeId) -> Option<&BuildNode> {
        self.nodes.get(node_id)
    }

    /// Returns a canonical hash of every node and dependency that may execute.
    ///
    /// This hash enters the caller's [`BuildIdentityInput`] so that a changed
    /// graph cannot run under the same `BuildId` merely because its requested
    /// root names happen to match.
    #[must_use]
    pub fn contract_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"meridian-build-graph-contract-v1\0");
        hash_field(
            &mut hasher,
            "requested-root-count",
            &self.requested_roots.len().to_string(),
        );
        for root in &self.requested_roots {
            hash_field(&mut hasher, "requested-root", root.as_str());
        }
        hash_field(&mut hasher, "node-count", &self.nodes.len().to_string());
        for node in self.nodes.values() {
            hash_field(&mut hasher, "node-id", node.id.as_str());
            hash_field(&mut hasher, "node-kind", node.kind.identity_name());
            hash_field(&mut hasher, "node-tool", &node.tool_id_version);
            hash_sorted_graph_values(&mut hasher, "node-input", &node.input_hashes);
            hash_sorted_graph_values(&mut hasher, "node-environment", &node.declared_environment);
            let dependencies = node
                .dependencies
                .iter()
                .map(|dependency| dependency.as_str().to_owned())
                .collect::<Vec<_>>();
            hash_sorted_graph_values(&mut hasher, "node-dependency", &dependencies);
        }
        hasher.finalize().to_hex().to_string()
    }

    /// Returns graph node IDs in deterministic dependency-before-dependent order.
    #[must_use]
    pub fn execution_order(&self) -> Vec<BuildNodeId> {
        let mut visited = BTreeSet::new();
        let mut order = Vec::with_capacity(self.nodes.len());
        for root in &self.requested_roots {
            append_execution_order(root, &self.nodes, &mut visited, &mut order);
        }
        order
    }

    /// Verifies that the graph roots exactly match the `BuildId`'s declared roots.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identity input or a root mismatch that would
    /// otherwise let a graph execute under an unrelated `BuildId`.
    pub fn validate_identity(&self, identity: &BuildIdentityInput) -> Result<(), BuildError> {
        validate_identity(identity)?;
        let declared = identity
            .root_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let graph_roots = self
            .requested_roots
            .iter()
            .map(BuildNodeId::as_str)
            .collect::<BTreeSet<_>>();
        if declared != graph_roots {
            return Err(BuildError::BuildGraphIdentityRootsMismatch);
        }
        if identity.build_graph_contract != self.contract_hash() {
            return Err(BuildError::BuildGraphIdentityContractMismatch);
        }
        Ok(())
    }

    /// Returns the canonical secret-safe manifest for this declared graph.
    #[must_use]
    pub fn provenance(&self) -> BuildGraphProvenance {
        BuildGraphProvenance::from_graph(self)
    }

    /// Creates a deterministic, dependency-aware scheduler for this graph.
    #[must_use]
    pub fn schedule(&self) -> BuildGraphSchedule {
        BuildGraphSchedule::new(self.clone())
    }
}

/// Canonical declared graph retained with durable build-input provenance.
///
/// It contains stable node IDs, node kinds, tool identities, declared input
/// hashes, allowlisted environment *names*, and dependency topology. Raw
/// environment values remain outside this manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildGraphProvenance {
    contract_hash: String,
    requested_roots: Vec<BuildNodeId>,
    nodes: Vec<BuildNode>,
}

impl BuildGraphProvenance {
    fn from_graph(graph: &BuildGraph) -> Self {
        Self {
            contract_hash: graph.contract_hash(),
            requested_roots: graph.requested_roots.clone(),
            nodes: graph.nodes.values().map(canonical_graph_node).collect(),
        }
    }

    fn validate(&self) -> Result<(), BuildError> {
        validate_artifact_hash(&self.contract_hash)?;
        let graph = BuildGraph::new(self.nodes.clone(), self.requested_roots.clone())?;
        if graph.contract_hash() != self.contract_hash {
            return Err(BuildError::MismatchedBuildGraphProvenance);
        }
        if self != &graph.provenance() {
            return Err(BuildError::NonCanonicalBuildGraphProvenance);
        }
        Ok(())
    }

    /// Returns the canonical graph-contract hash retained with this manifest.
    #[must_use]
    pub fn contract_hash(&self) -> &str {
        &self.contract_hash
    }

    /// Returns declared roots in canonical identifier order.
    #[must_use]
    pub fn requested_roots(&self) -> &[BuildNodeId] {
        &self.requested_roots
    }

    /// Returns declared nodes in canonical identifier order.
    #[must_use]
    pub fn nodes(&self) -> &[BuildNode] {
        &self.nodes
    }
}

fn canonical_graph_node(node: &BuildNode) -> BuildNode {
    let mut node = node.clone();
    node.input_hashes.sort_unstable();
    node.declared_environment.sort_unstable();
    node.dependencies.sort_unstable();
    node
}

/// Per-node state in the bounded dependency scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildGraphNodeState {
    /// Waiting for successful dependencies.
    Waiting,
    /// All dependencies succeeded and the node may start.
    Ready,
    /// A worker is executing the node.
    Running,
    /// The node completed with validated output.
    Succeeded,
    /// The node encountered an actionable failure.
    Failed,
    /// The node was cancelled before publication.
    Cancelled,
    /// The executing worker disappeared before a valid commit.
    WorkerLost,
    /// A newer graph or `BuildId` invalidated the node.
    Superseded,
    /// A required dependency cannot produce a successful input.
    Blocked,
}

impl BuildGraphNodeState {
    const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Failed
                | Self::Cancelled
                | Self::WorkerLost
                | Self::Superseded
                | Self::Blocked
        )
    }

    const fn blocks_dependents(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Cancelled | Self::WorkerLost | Self::Superseded | Self::Blocked
        )
    }
}

/// One state transition emitted by the deterministic graph scheduler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildGraphNodeEvent {
    /// Node whose scheduler state changed.
    pub node_id: BuildNodeId,
    /// New scheduler state.
    pub state: BuildGraphNodeState,
}

/// Single-host dependency scheduler with no hidden worker or resource authority.
pub struct BuildGraphSchedule {
    graph: BuildGraph,
    states: BTreeMap<BuildNodeId, BuildGraphNodeState>,
}

impl BuildGraphSchedule {
    fn new(graph: BuildGraph) -> Self {
        let mut schedule = Self {
            states: graph
                .nodes
                .keys()
                .cloned()
                .map(|node_id| (node_id, BuildGraphNodeState::Waiting))
                .collect(),
            graph,
        };
        schedule.refresh_waiting_nodes();
        schedule
    }

    /// Returns a node's current scheduler state.
    ///
    /// # Errors
    ///
    /// Returns an error when `node_id` is absent from the validated graph.
    pub fn state(&self, node_id: &BuildNodeId) -> Result<BuildGraphNodeState, BuildError> {
        self.states
            .get(node_id)
            .copied()
            .ok_or_else(|| BuildError::UnknownBuildGraphNode(node_id.clone()))
    }

    /// Returns every node currently ready to start in deterministic order.
    #[must_use]
    pub fn ready_nodes(&self) -> Vec<BuildNodeId> {
        self.states
            .iter()
            .filter(|(_, state)| **state == BuildGraphNodeState::Ready)
            .map(|(node_id, _)| node_id.clone())
            .collect()
    }

    /// Starts one ready node.
    ///
    /// # Errors
    ///
    /// Returns an error when the node is absent or not ready, including when a
    /// dependency failed or remains incomplete.
    pub fn start(&mut self, node_id: &BuildNodeId) -> Result<BuildGraphNodeEvent, BuildError> {
        self.transition(
            node_id,
            BuildGraphNodeState::Ready,
            BuildGraphNodeState::Running,
        )
    }

    /// Finishes one running node with a valid terminal operation phase.
    ///
    /// # Errors
    ///
    /// Returns an error when the node is absent, not running, or supplied a
    /// non-terminal phase that cannot conclude one graph node.
    pub fn finish(
        &mut self,
        node_id: &BuildNodeId,
        phase: BuildPhase,
    ) -> Result<BuildGraphNodeEvent, BuildError> {
        let target = match phase {
            BuildPhase::Succeeded => BuildGraphNodeState::Succeeded,
            BuildPhase::Failed => BuildGraphNodeState::Failed,
            BuildPhase::Cancelled => BuildGraphNodeState::Cancelled,
            BuildPhase::WorkerLost => BuildGraphNodeState::WorkerLost,
            BuildPhase::Superseded => BuildGraphNodeState::Superseded,
            BuildPhase::Queued
            | BuildPhase::Resolving
            | BuildPhase::Ready
            | BuildPhase::Running
            | BuildPhase::CancelRequested => {
                return Err(BuildError::InvalidBuildGraphCompletion(phase));
            }
        };
        let event = self.transition(node_id, BuildGraphNodeState::Running, target)?;
        self.refresh_waiting_nodes();
        Ok(event)
    }

    /// Returns whether every node has reached a terminal scheduler state.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.states
            .values()
            .copied()
            .all(BuildGraphNodeState::is_terminal)
    }

    fn transition(
        &mut self,
        node_id: &BuildNodeId,
        expected: BuildGraphNodeState,
        target: BuildGraphNodeState,
    ) -> Result<BuildGraphNodeEvent, BuildError> {
        let state = self
            .states
            .get_mut(node_id)
            .ok_or_else(|| BuildError::UnknownBuildGraphNode(node_id.clone()))?;
        if *state != expected {
            return Err(BuildError::InvalidBuildGraphNodeTransition {
                node: node_id.clone(),
                current: *state,
                next: target,
            });
        }
        *state = target;
        Ok(BuildGraphNodeEvent {
            node_id: node_id.clone(),
            state: target,
        })
    }

    fn refresh_waiting_nodes(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            for (node_id, node) in &self.graph.nodes {
                let Some(state) = self.states.get(node_id).copied() else {
                    continue;
                };
                if state != BuildGraphNodeState::Waiting {
                    continue;
                }
                let dependency_states = node
                    .dependencies
                    .iter()
                    .filter_map(|dependency| self.states.get(dependency).copied())
                    .collect::<Vec<_>>();
                let next = if dependency_states
                    .iter()
                    .copied()
                    .any(BuildGraphNodeState::blocks_dependents)
                {
                    BuildGraphNodeState::Blocked
                } else if dependency_states
                    .iter()
                    .copied()
                    .all(|state| state == BuildGraphNodeState::Succeeded)
                {
                    BuildGraphNodeState::Ready
                } else {
                    continue;
                };
                self.states.insert(node_id.clone(), next);
                changed = true;
            }
        }
    }
}

/// A caller's immutable request to start one bounded build operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildRequest {
    /// Content-addressed identity of all declared build inputs.
    pub build_id: BuildId,
    /// Secret-safe durable description of the declared build inputs.
    ///
    /// Legacy local snapshots predate this field and recover without it; new
    /// requests and published artifact events require it.
    #[serde(default)]
    pub input_provenance: Option<BuildInputProvenance>,
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
        let graph = BuildGraph::new(vec![root_node.clone()], vec![root_node.id.clone()])?;
        Self::new_with_graph(identity, operation_id, trace_id, root_node, &graph)
    }

    /// Creates one request after binding it to its validated execution graph.
    ///
    /// The initial service supports one root operation per request. The graph
    /// contract is already a declared `BuildId` input, and this constructor
    /// prevents a caller from submitting a different root than the graph that
    /// produced that contract hash.
    ///
    /// # Errors
    ///
    /// Returns an error when the graph or identity is invalid, their roots or
    /// graph contract disagree, or this request would contain multiple roots.
    pub fn new_with_graph(
        identity: &BuildIdentityInput,
        operation_id: OperationId,
        trace_id: TraceId,
        root_node: BuildNode,
        graph: &BuildGraph,
    ) -> Result<Self, BuildError> {
        graph.validate_identity(identity)?;
        if graph.requested_roots() != [root_node.id.clone()] {
            return Err(BuildError::BuildGraphRequestRootMismatch);
        }
        Ok(Self {
            build_id: BuildId::derive(identity)?,
            input_provenance: Some(BuildInputProvenance::from_identity_and_graph(
                identity, graph,
            )?),
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
    /// A bounded process-level Cargo failure diagnostic, carried in [`BuildEvent::diagnostic`].
    ProcessDiagnostic,
    /// A verified artifact publication bound to the running request.
    Artifact(PublishedArtifact),
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
#[derive(Clone, Default)]
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
        validate_build_request(&request, true)?;
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
        validate_progress(progress)?;
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
        if let CargoMessage::Artifact(artifact) = &message {
            validate_cargo_artifact(artifact)?;
        }
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

    /// Records a bounded Cargo process-failure diagnostic while an operation is running.
    ///
    /// Unlike [`Self::record_cargo_message`], this represents Cargo stderr from
    /// an unsuccessful process status rather than a Cargo JSON message.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is absent or not running.
    pub fn record_process_diagnostic(
        &mut self,
        operation_id: OperationId,
        diagnostic: BuildDiagnostic,
    ) -> Result<BuildEvent, BuildError> {
        let diagnostic = sanitize_process_diagnostic(diagnostic)?;
        let operation = self.operation_mut(operation_id)?;
        if operation.phase != BuildPhase::Running {
            return Err(BuildError::CargoMessageOutsideRunning(operation.phase));
        }
        operation.emit_with_diagnostic(
            BuildPhase::Running,
            50,
            Some(diagnostic),
            BuildEventPayload::ProcessDiagnostic,
        )
    }

    /// Records one verified artifact publication while an operation is running.
    ///
    /// The publication must carry the same immutable `BuildId` and root-node ID as
    /// the request. This method does not publish or prove the source itself; use
    /// [`ArtifactStore`] for the bounded file-copy and object/reference checks.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is absent or not running, or when the
    /// publication is malformed or belongs to a different request.
    pub fn record_published_artifact(
        &mut self,
        operation_id: OperationId,
        publication: PublishedArtifact,
    ) -> Result<BuildEvent, BuildError> {
        validate_published_artifact(&publication)?;
        let operation = self.operation_mut(operation_id)?;
        if operation.phase != BuildPhase::Running {
            return Err(BuildError::CargoMessageOutsideRunning(operation.phase));
        }
        if publication.build_id != operation.request.build_id {
            return Err(BuildError::MismatchedEventIdentity);
        }
        if publication.node_id != operation.request.root_node.id {
            return Err(BuildError::MismatchedNodeId);
        }
        let expected_provenance = operation
            .request
            .input_provenance
            .as_ref()
            .ok_or(BuildError::MissingBuildInputProvenance)?;
        let publication_provenance = publication
            .build_input_provenance
            .as_deref()
            .ok_or(BuildError::MissingBuildInputProvenance)?;
        if publication_provenance != expected_provenance {
            return Err(BuildError::MismatchedBuildInputProvenance);
        }
        operation.emit_with_artifact(BuildPhase::Running, 75, publication)
    }

    /// Accepts a worker-produced event only if identity, sequence, and phase are valid.
    ///
    /// # Errors
    ///
    /// Returns an error for stale sequence numbers, mismatched identities, or invalid transitions.
    pub fn accept_external_event(&mut self, event: &BuildEvent) -> Result<(), BuildError> {
        validate_external_event(event)?;
        let operation = self.operation_mut(event.operation_id)?;
        if operation.request.build_id != event.build_id
            || operation.request.trace_id != event.trace_id
        {
            return Err(BuildError::MismatchedEventIdentity);
        }
        if operation.request.root_node.id != event.node_id {
            return Err(BuildError::MismatchedNodeId);
        }
        validate_external_event_for_operation(event, &operation.request, operation.phase)?;
        let expected = operation.last_sequence.saturating_add(1);
        if event.sequence != expected {
            return Err(BuildError::StaleEventSequence {
                expected,
                received: event.sequence,
            });
        }
        operation.validate_next_phase(event.phase)?;
        operation.record_cargo_finished(&event.payload)?;
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

    /// Allocates the next unused process-local operation ID for this bounded service.
    ///
    /// # Errors
    ///
    /// Returns an error when the service has reached its operation bound or a
    /// caller supplied an unrepresentable prior operation ID.
    pub fn next_operation_id(&self) -> Result<OperationId, BuildError> {
        if self.operations.len() == MAX_OPERATIONS {
            return Err(BuildError::TooManyOperations(MAX_OPERATIONS));
        }
        let next = self
            .operations
            .keys()
            .next_back()
            .map_or(Ok(1), |operation_id| {
                operation_id
                    .get()
                    .checked_add(1)
                    .ok_or(BuildError::OperationIdExhausted)
            })?;
        Ok(OperationId::new(next))
    }

    fn validate_running_request(&self, request: &BuildRequest) -> Result<(), BuildError> {
        validate_build_request(request, true)?;
        let operation = self
            .operations
            .get(&request.operation_id)
            .ok_or(BuildError::UnknownOperation(request.operation_id))?;
        if operation.request != *request {
            return Err(BuildError::MismatchedEventIdentity);
        }
        if operation.phase != BuildPhase::Running {
            return Err(BuildError::CargoMessageOutsideRunning(operation.phase));
        }
        Ok(())
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
                cargo_finished: persisted_operation.cargo_finished,
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

/// Project-hosted, local durable storage for one build-service operation snapshot.
///
/// The host selects this file beneath a project-owned state directory. Publication
/// writes and syncs a unique sibling temporary file before a same-directory rename;
/// callers must still report any platform/filesystem durability limitation rather
/// than treating this portable foundation as a remote or signing-grade store.
pub struct BuildServiceStore {
    path: PathBuf,
}

impl BuildServiceStore {
    /// Creates a store rooted at one concrete state-file path.
    ///
    /// # Errors
    ///
    /// Returns an error when `path` is empty or does not name a file.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, BuildError> {
        let path = path.into();
        if path.as_os_str().is_empty() || path.file_name().is_none() {
            return Err(BuildError::InvalidSnapshotPath);
        }
        Ok(Self { path })
    }

    /// Returns the host-selected state-file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns whether a regular, non-symlinked state file exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured path is not a regular file or cannot
    /// be inspected safely.
    pub fn exists(&self) -> Result<bool, BuildError> {
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) => {
                validate_regular_snapshot_metadata(&metadata)?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(BuildError::SnapshotIo {
                operation: "inspect state file",
                message: error.to_string(),
            }),
        }
    }

    /// Writes a bounded service snapshot through a synced temporary file.
    ///
    /// # Errors
    ///
    /// Returns an error without intentionally retaining a partial primary state
    /// file when directory creation, temporary creation, writing, syncing, or
    /// replacement fails.
    pub fn save(&self, service: &BuildService) -> Result<(), BuildError> {
        let snapshot = service.snapshot_json()?;
        self.ensure_parent_directory()?;
        let _ = self.exists()?;
        let (temporary_path, mut temporary_file) = self.create_temporary_file()?;
        let write_result = temporary_file
            .write_all(snapshot.as_bytes())
            .and_then(|()| temporary_file.sync_all());
        drop(temporary_file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(BuildError::SnapshotIo {
                operation: "write state file",
                message: error.to_string(),
            });
        }
        if let Err(error) = fs::rename(&temporary_path, &self.path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(BuildError::SnapshotIo {
                operation: "replace state file",
                message: error.to_string(),
            });
        }
        Ok(())
    }

    /// Loads one bounded snapshot and marks interrupted operations `WorkerLost`.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing, non-regular, oversized, non-UTF-8, or
    /// invalid versioned snapshot.
    pub fn load(&self) -> Result<BuildServiceRecovery, BuildError> {
        if !self.exists()? {
            return Err(BuildError::SnapshotMissing);
        }
        let file = File::open(&self.path).map_err(|error| BuildError::SnapshotIo {
            operation: "open state file",
            message: error.to_string(),
        })?;
        let length = file
            .metadata()
            .map_err(|error| BuildError::SnapshotIo {
                operation: "inspect opened state file",
                message: error.to_string(),
            })?
            .len();
        if length > MAX_BUILD_SNAPSHOT_BYTES as u64 {
            return Err(BuildError::SnapshotTooLarge(
                usize::try_from(length).unwrap_or(usize::MAX),
            ));
        }
        let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
        file.take(MAX_BUILD_SNAPSHOT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| BuildError::SnapshotIo {
                operation: "read state file",
                message: error.to_string(),
            })?;
        if bytes.len() > MAX_BUILD_SNAPSHOT_BYTES {
            return Err(BuildError::SnapshotTooLarge(bytes.len()));
        }
        let snapshot = String::from_utf8(bytes).map_err(|_| BuildError::SnapshotNotUtf8)?;
        BuildService::restore_json(&snapshot)
    }

    fn ensure_parent_directory(&self) -> Result<(), BuildError> {
        let parent = self.parent_directory();
        fs::create_dir_all(parent).map_err(|error| BuildError::SnapshotIo {
            operation: "create state directory",
            message: error.to_string(),
        })?;
        let metadata = fs::symlink_metadata(parent).map_err(|error| BuildError::SnapshotIo {
            operation: "inspect state directory",
            message: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(BuildError::SnapshotPathSymlink);
        }
        if !metadata.is_dir() {
            return Err(BuildError::SnapshotParentNotDirectory);
        }
        Ok(())
    }

    fn create_temporary_file(&self) -> Result<(PathBuf, File), BuildError> {
        let file_name = self
            .path
            .file_name()
            .ok_or(BuildError::InvalidSnapshotPath)?;
        for _ in 0..MAX_SNAPSHOT_TEMPORARY_ATTEMPTS {
            let temporary_id = NEXT_SNAPSHOT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
            let mut temporary_name = file_name.to_os_string();
            temporary_name.push(format!(".{}-{temporary_id}.tmp", std::process::id()));
            let temporary_path = self.parent_directory().join(temporary_name);
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary_path)
            {
                Ok(file) => return Ok((temporary_path, file)),
                Err(error) => {
                    if error.kind() != std::io::ErrorKind::AlreadyExists {
                        return Err(BuildError::SnapshotIo {
                            operation: "create temporary state file",
                            message: error.to_string(),
                        });
                    }
                }
            }
        }
        Err(BuildError::SnapshotTemporaryExhausted)
    }

    fn parent_directory(&self) -> &Path {
        self.path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    }
}

/// Project-hosted content-addressed artifact publication store.
///
/// This bounded foundation publishes a fully copied, hashed object before an
/// non-overwriting BuildId/node reference. It intentionally does not provide cache
/// eviction, remote transfer, signing, or unconstrained Cargo-artifact selection.
pub struct ArtifactStore {
    root: PathBuf,
}

/// Non-overwriting reference to one atomically published artifact object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublishedArtifact {
    build_id: BuildId,
    node_id: BuildNodeId,
    schema: String,
    tool_id_version: String,
    content_hash: String,
    byte_length: u64,
    cargo_provenance: Option<Box<CargoArtifactProvenance>>,
    build_input_provenance: Option<Box<BuildInputProvenance>>,
}

/// Bounded Cargo identity retained for one Cargo-reported artifact publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CargoArtifactProvenance {
    package_id: String,
    target_name: String,
}

impl CargoArtifactProvenance {
    fn from_artifact(artifact: &CargoArtifact) -> Self {
        Self {
            package_id: artifact.package_id.clone(),
            target_name: artifact.target_name.clone(),
        }
    }

    /// Returns Cargo's package identity that reported the artifact.
    #[must_use]
    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    /// Returns Cargo's target name that reported the artifact.
    #[must_use]
    pub fn target_name(&self) -> &str {
        &self.target_name
    }
}

impl PublishedArtifact {
    /// Returns the immutable `BuildId` bound to this verified publication.
    #[must_use]
    pub fn build_id(&self) -> &BuildId {
        &self.build_id
    }

    /// Returns the graph node that produced this verified publication.
    #[must_use]
    pub fn node_id(&self) -> &BuildNodeId {
        &self.node_id
    }

    /// Returns the declared schema or format identity for the artifact bytes.
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the declared producer tool identity/version from the graph node.
    #[must_use]
    pub fn tool_id_version(&self) -> &str {
        &self.tool_id_version
    }

    /// Returns the BLAKE3 hash of the exact published object bytes.
    #[must_use]
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    /// Returns the published object size in bytes.
    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Returns the Cargo package/target identity when Cargo reported this artifact.
    #[must_use]
    pub fn cargo_provenance(&self) -> Option<&CargoArtifactProvenance> {
        self.cargo_provenance.as_deref()
    }

    /// Returns the secret-safe request inputs stored atomically with this reference.
    #[must_use]
    pub fn build_input_provenance(&self) -> Option<&BuildInputProvenance> {
        self.build_input_provenance.as_deref()
    }
}

#[derive(Deserialize)]
struct StoredPublishedArtifact {
    build_id: BuildId,
    node_id: BuildNodeId,
    schema: String,
    tool_id_version: String,
    content_hash: String,
    byte_length: u64,
    #[serde(default)]
    cargo_provenance: Option<CargoArtifactProvenance>,
    #[serde(default)]
    build_input_provenance: Option<BuildInputProvenance>,
}

impl From<StoredPublishedArtifact> for PublishedArtifact {
    fn from(stored: StoredPublishedArtifact) -> Self {
        Self {
            build_id: stored.build_id,
            node_id: stored.node_id,
            schema: stored.schema,
            tool_id_version: stored.tool_id_version,
            content_hash: stored.content_hash,
            byte_length: stored.byte_length,
            cargo_provenance: stored.cargo_provenance.map(Box::new),
            build_input_provenance: stored.build_input_provenance.map(Box::new),
        }
    }
}

impl ArtifactStore {
    /// Creates a store rooted at a host-selected project-owned directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the root path is empty.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, BuildError> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(BuildError::InvalidArtifactRoot);
        }
        Ok(Self { root })
    }

    /// Returns the host-selected root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Copies one regular source file into a verified object and atomically
    /// publishes its BuildId/node reference.
    ///
    /// The source must fit the first-slice bounded file limit. Existing object
    /// and reference paths are validated and never silently overwritten. This
    /// API records declared BuildId/schema/tool associations; it does not prove
    /// that a particular Cargo invocation produced the source file.
    ///
    /// # Errors
    ///
    /// Returns an error for untrusted paths, source/read/write failures,
    /// oversized inputs, corrupted existing objects, or conflicting references.
    pub fn publish_file(
        &self,
        build_id: &BuildId,
        node: &BuildNode,
        schema: impl Into<String>,
        source: impl AsRef<Path>,
    ) -> Result<PublishedArtifact, BuildError> {
        self.publish_file_with_provenance(build_id, node, schema, source, None, None)
    }

    /// Copies one regular source file into a verified object and atomically
    /// stores the requesting build's secret-safe input provenance with its
    /// BuildId/node reference.
    ///
    /// # Errors
    ///
    /// Returns an error when the request lacks current input provenance or
    /// artifact publication validation fails.
    pub fn publish_file_for_request(
        &self,
        request: &BuildRequest,
        schema: impl Into<String>,
        source: impl AsRef<Path>,
    ) -> Result<PublishedArtifact, BuildError> {
        let input_provenance = request
            .input_provenance
            .as_ref()
            .ok_or(BuildError::MissingBuildInputProvenance)?;
        validate_build_request(request, true)?;
        self.publish_file_with_provenance(
            &request.build_id,
            &request.root_node,
            schema,
            source,
            None,
            Some(input_provenance.clone()),
        )
    }

    fn publish_file_with_provenance(
        &self,
        build_id: &BuildId,
        node: &BuildNode,
        schema: impl Into<String>,
        source: impl AsRef<Path>,
        cargo_provenance: Option<CargoArtifactProvenance>,
        build_input_provenance: Option<BuildInputProvenance>,
    ) -> Result<PublishedArtifact, BuildError> {
        validate_build_id(build_id)?;
        validate_build_node(node)?;
        let schema = schema.into();
        validate_text("artifact schema", &schema)?;
        if let Some(provenance) = &cargo_provenance {
            validate_cargo_artifact_provenance(provenance)?;
        }
        if let Some(provenance) = &build_input_provenance {
            validate_build_input_provenance(provenance, true)?;
        }
        let objects_directory = self.objects_directory()?;
        let reference_directory = self.reference_directory(build_id)?;
        let (temporary_object, content_hash, byte_length) =
            copy_artifact_to_temporary(source.as_ref(), &objects_directory)?;
        let object_path = objects_directory.join(&content_hash);
        let object_result =
            publish_immutable_artifact(&temporary_object, &object_path, &content_hash, byte_length);
        if let Err(error) = object_result {
            let _ = fs::remove_file(&temporary_object);
            return Err(error);
        }
        let published = PublishedArtifact {
            build_id: build_id.clone(),
            node_id: node.id.clone(),
            schema,
            tool_id_version: node.tool_id_version.clone(),
            content_hash,
            byte_length,
            cargo_provenance: cargo_provenance.map(Box::new),
            build_input_provenance: build_input_provenance.map(Box::new),
        };
        Self::publish_reference(&reference_directory, &published)
    }

    /// Validates and publishes one Cargo-reported executable from a declared output root.
    ///
    /// The Cargo artifact record is untrusted until its bounded fields validate, the
    /// executable is listed in the record's filenames, and its canonical regular-file
    /// path lies beneath the explicit non-symlink output root. The method records that
    /// Cargo reported the path; it does not establish remote provenance, signing, or a
    /// reusable cache policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the record lacks a listed absolute executable, the output
    /// root or executable path is unsafe, the executable escapes the root, or normal
    /// artifact publication validation fails.
    pub fn publish_cargo_executable(
        &self,
        build_id: &BuildId,
        node: &BuildNode,
        artifact: &CargoArtifact,
        cargo_output_root: impl AsRef<Path>,
    ) -> Result<PublishedArtifact, BuildError> {
        validate_cargo_artifact(artifact)?;
        let executable = artifact
            .executable
            .as_deref()
            .ok_or(BuildError::CargoArtifactExecutableMissing)?;
        if !artifact
            .filenames
            .iter()
            .any(|filename| filename == executable)
        {
            return Err(BuildError::CargoArtifactExecutableNotListed);
        }
        let executable_path = Path::new(executable);
        if !executable_path.is_absolute() {
            return Err(BuildError::CargoArtifactExecutableNotAbsolute);
        }
        validate_regular_artifact_file(executable_path)?;
        let output_root = canonical_cargo_output_root(cargo_output_root.as_ref())?;
        let executable_path =
            fs::canonicalize(executable_path).map_err(|error| BuildError::ArtifactIo {
                operation: "canonicalize Cargo executable",
                message: error.to_string(),
            })?;
        validate_regular_artifact_file(&executable_path)?;
        if !executable_path.starts_with(&output_root) {
            return Err(BuildError::CargoArtifactExecutableOutsideOutputRoot);
        }
        self.publish_file_with_provenance(
            build_id,
            node,
            "cargo-executable-v1",
            executable_path,
            Some(CargoArtifactProvenance::from_artifact(artifact)),
            None,
        )
    }

    /// Validates and publishes one Cargo executable with the request input manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the request lacks durable input provenance or the
    /// Cargo artifact cannot satisfy the normal bounded publication checks.
    pub fn publish_cargo_executable_for_request(
        &self,
        request: &BuildRequest,
        artifact: &CargoArtifact,
        cargo_output_root: impl AsRef<Path>,
    ) -> Result<PublishedArtifact, BuildError> {
        let input_provenance = request
            .input_provenance
            .as_ref()
            .ok_or(BuildError::MissingBuildInputProvenance)?;
        validate_build_request(request, true)?;
        validate_cargo_artifact(artifact)?;
        let executable = artifact
            .executable
            .as_deref()
            .ok_or(BuildError::CargoArtifactExecutableMissing)?;
        if !artifact
            .filenames
            .iter()
            .any(|filename| filename == executable)
        {
            return Err(BuildError::CargoArtifactExecutableNotListed);
        }
        let executable_path = Path::new(executable);
        if !executable_path.is_absolute() {
            return Err(BuildError::CargoArtifactExecutableNotAbsolute);
        }
        validate_regular_artifact_file(executable_path)?;
        let output_root = canonical_cargo_output_root(cargo_output_root.as_ref())?;
        let executable_path =
            fs::canonicalize(executable_path).map_err(|error| BuildError::ArtifactIo {
                operation: "canonicalize Cargo executable",
                message: error.to_string(),
            })?;
        validate_regular_artifact_file(&executable_path)?;
        if !executable_path.starts_with(&output_root) {
            return Err(BuildError::CargoArtifactExecutableOutsideOutputRoot);
        }
        self.publish_file_with_provenance(
            &request.build_id,
            &request.root_node,
            "cargo-executable-v1",
            executable_path,
            Some(CargoArtifactProvenance::from_artifact(artifact)),
            Some(input_provenance.clone()),
        )
    }

    /// Returns the local object path for a validated content hash.
    ///
    /// This method does not claim that the object exists or remains valid; a
    /// caller must validate it before use across trust boundaries.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid object hash or an invalid store root.
    pub fn object_path(&self, content_hash: &str) -> Result<PathBuf, BuildError> {
        validate_artifact_hash(content_hash)?;
        Ok(self.objects_directory()?.join(content_hash))
    }

    fn objects_directory(&self) -> Result<PathBuf, BuildError> {
        let directory = self.root.join("objects");
        ensure_artifact_directory(&self.root)?;
        ensure_artifact_directory(&directory)?;
        Ok(directory)
    }

    fn reference_directory(&self, build_id: &BuildId) -> Result<PathBuf, BuildError> {
        let root = self.root.join("references");
        let directory = root.join(build_id.as_str());
        ensure_artifact_directory(&self.root)?;
        ensure_artifact_directory(&root)?;
        ensure_artifact_directory(&directory)?;
        Ok(directory)
    }

    fn publish_reference(
        directory: &Path,
        published: &PublishedArtifact,
    ) -> Result<PublishedArtifact, BuildError> {
        let path = directory.join(reference_file_name(&published.node_id));
        match read_published_reference(&path) {
            Ok(existing) => {
                if existing == *published {
                    return Ok(existing);
                }
                if matching_artifact_identity(&existing, published)
                    && existing_provenance_is_compatible(
                        existing.cargo_provenance.as_deref(),
                        published.cargo_provenance.as_deref(),
                    )
                    && existing_provenance_is_compatible(
                        existing.build_input_provenance.as_deref(),
                        published.build_input_provenance.as_deref(),
                    )
                {
                    return Ok(existing);
                }
                return Err(BuildError::ArtifactReferenceConflict);
            }
            Err(BuildError::ArtifactReferenceMissing) => {}
            Err(error) => return Err(error),
        }
        let encoded = serde_json::to_vec(published)
            .map_err(|error| BuildError::ArtifactSerialization(error.to_string()))?;
        if encoded.len() > MAX_BUILD_SNAPSHOT_BYTES {
            return Err(BuildError::ArtifactReferenceTooLarge(encoded.len()));
        }
        let temporary = write_artifact_temporary(directory, "reference", &encoded)?;
        let result = publish_immutable_file(&temporary, &path);
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        match read_published_reference(&path) {
            Ok(existing) if existing == *published => Ok(existing),
            Ok(_) => Err(BuildError::ArtifactReferenceConflict),
            Err(error) => Err(error),
        }
    }
}

/// Automatically persistent local build service for a project-owned state file.
///
/// Every accepted mutation is persisted before it is reported to the caller.
/// A failed state write restores the pre-mutation in-memory service, so a caller
/// cannot observe an event that is absent from the durable snapshot.
pub struct DurableBuildService {
    store: BuildServiceStore,
    service: BuildService,
}

impl DurableBuildService {
    /// Opens existing state or initializes and persists an empty service.
    ///
    /// # Errors
    ///
    /// Returns an error when the local store cannot be read, recovered, or
    /// atomically updated.
    pub fn open(store: BuildServiceStore) -> Result<DurableBuildServiceRecovery, BuildError> {
        if store.exists()? {
            let recovered = store.load()?;
            store.save(&recovered.service)?;
            Ok(DurableBuildServiceRecovery {
                service: Self {
                    store,
                    service: recovered.service,
                },
                recovery_events: recovered.recovery_events,
            })
        } else {
            let service = BuildService::default();
            store.save(&service)?;
            Ok(DurableBuildServiceRecovery {
                service: Self { store, service },
                recovery_events: Vec::new(),
            })
        }
    }

    /// Exposes current service state for read-only inspection.
    #[must_use]
    pub fn service(&self) -> &BuildService {
        &self.service
    }

    /// Submits and durably records a queued operation.
    ///
    /// # Errors
    ///
    /// Returns an error when the request is invalid, duplicate, or cannot be
    /// persisted without losing the previous durable state.
    pub fn submit(&mut self, request: BuildRequest) -> Result<BuildEvent, BuildError> {
        self.persist(|service| service.submit(request))
    }

    /// Applies and durably records one lifecycle transition.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or its resulting state
    /// cannot be persisted without losing the previous durable state.
    pub fn transition(
        &mut self,
        operation_id: OperationId,
        phase: BuildPhase,
        progress: u8,
    ) -> Result<BuildEvent, BuildError> {
        self.persist(|service| service.transition(operation_id, phase, progress))
    }

    /// Records and durably stores one Cargo message from a running operation.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is not running or its resulting state
    /// cannot be persisted without losing the previous durable state.
    pub fn record_cargo_message(
        &mut self,
        operation_id: OperationId,
        message: CargoMessage,
    ) -> Result<BuildEvent, BuildError> {
        self.persist(|service| service.record_cargo_message(operation_id, message))
    }

    /// Records and durably stores one Cargo process-failure diagnostic.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is not running or its resulting state
    /// cannot be persisted without losing the previous durable state.
    pub fn record_process_diagnostic(
        &mut self,
        operation_id: OperationId,
        diagnostic: BuildDiagnostic,
    ) -> Result<BuildEvent, BuildError> {
        self.persist(|service| service.record_process_diagnostic(operation_id, diagnostic))
    }

    /// Records and durably stores one verified artifact-publication event.
    ///
    /// # Errors
    ///
    /// Returns an error when the publication does not match the running request
    /// or the resulting event cannot be durably persisted.
    pub fn record_published_artifact(
        &mut self,
        operation_id: OperationId,
        publication: PublishedArtifact,
    ) -> Result<BuildEvent, BuildError> {
        self.persist(|service| service.record_published_artifact(operation_id, publication))
    }

    /// Validates and durably records one external worker event.
    ///
    /// # Errors
    ///
    /// Returns an error when the event is stale or mismatched, or its resulting
    /// state cannot be persisted without losing the previous durable state.
    pub fn accept_external_event(&mut self, event: &BuildEvent) -> Result<(), BuildError> {
        self.persist(|service| service.accept_external_event(event))
    }

    fn persist<T>(
        &mut self,
        mutation: impl FnOnce(&mut BuildService) -> Result<T, BuildError>,
    ) -> Result<T, BuildError> {
        let previous = self.service.clone();
        match mutation(&mut self.service) {
            Ok(value) => match self.store.save(&self.service) {
                Ok(()) => Ok(value),
                Err(error) => {
                    self.service = previous;
                    Err(error)
                }
            },
            Err(error) => {
                self.service = previous;
                Err(error)
            }
        }
    }
}

/// Result of opening a durable service, including explicit crash-recovery events.
pub struct DurableBuildServiceRecovery {
    /// The recovered service, ready to accept only valid next operations.
    pub service: DurableBuildService,
    /// `WorkerLost` events emitted for interrupted persisted operations.
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
    #[serde(default)]
    cargo_finished: Option<bool>,
}

impl From<&BuildOperation> for PersistedBuildOperation {
    fn from(operation: &BuildOperation) -> Self {
        Self {
            request: operation.request.clone(),
            phase: operation.phase,
            last_sequence: operation.last_sequence,
            cargo_finished: operation.cargo_finished,
        }
    }
}

#[derive(Clone)]
struct BuildOperation {
    request: BuildRequest,
    phase: BuildPhase,
    last_sequence: u64,
    cargo_finished: Option<bool>,
}

impl BuildOperation {
    const fn new(request: BuildRequest) -> Self {
        Self {
            request,
            phase: BuildPhase::Queued,
            last_sequence: 0,
            cargo_finished: None,
        }
    }

    fn validate_next_phase(&self, phase: BuildPhase) -> Result<(), BuildError> {
        validate_transition(self.phase, phase)?;
        validate_cargo_finished_phase(self.cargo_finished, phase)
    }

    fn record_cargo_finished(&mut self, payload: &BuildEventPayload) -> Result<(), BuildError> {
        let BuildEventPayload::Cargo(CargoMessage::Finished { success }) = payload else {
            return Ok(());
        };
        if let Some(previous) = self.cargo_finished {
            if previous != *success {
                return Err(BuildError::ConflictingCargoFinishedOutcome {
                    previous,
                    received: *success,
                });
            }
            return Ok(());
        }
        self.cargo_finished = Some(*success);
        Ok(())
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
        validate_progress(progress)?;
        self.validate_next_phase(phase)?;
        self.record_cargo_finished(&payload)?;
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

    fn emit_with_artifact(
        &mut self,
        phase: BuildPhase,
        progress: u8,
        publication: PublishedArtifact,
    ) -> Result<BuildEvent, BuildError> {
        self.validate_next_phase(phase)?;
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
            diagnostic: None,
            artifact_hash: Some(publication.content_hash.clone()),
            trace_id: self.request.trace_id,
            payload: BuildEventPayload::Artifact(publication),
        })
    }
}

fn validate_persisted_operation(operation: &PersistedBuildOperation) -> Result<(), BuildError> {
    validate_build_request(&operation.request, false)?;
    validate_cargo_finished_phase(operation.cargo_finished, operation.phase)
}

fn validate_build_node(node: &BuildNode) -> Result<(), BuildError> {
    validate_text("build node ID", node.id.as_str())?;
    validate_text("build node tool ID and version", &node.tool_id_version)?;
    let mut inputs = BTreeSet::new();
    for input in &node.input_hashes {
        validate_text("build node input hash", input)?;
        if !inputs.insert(input) {
            return Err(BuildError::DuplicateBuildGraphInput {
                node: node.id.clone(),
                input: input.clone(),
            });
        }
    }
    let mut environment = BTreeSet::new();
    for name in &node.declared_environment {
        if !is_allowlisted_environment(name) {
            return Err(BuildError::EnvironmentNotAllowlisted(name.clone()));
        }
        if !environment.insert(name) {
            return Err(BuildError::DuplicateBuildGraphEnvironment {
                node: node.id.clone(),
                name: name.clone(),
            });
        }
    }
    let mut dependencies = BTreeSet::new();
    for dependency in &node.dependencies {
        validate_text("build node dependency", dependency.as_str())?;
        if *dependency == node.id {
            return Err(BuildError::BuildGraphSelfDependency(node.id.clone()));
        }
        if !dependencies.insert(dependency) {
            return Err(BuildError::DuplicateBuildGraphDependency {
                node: node.id.clone(),
                dependency: dependency.clone(),
            });
        }
    }
    Ok(())
}

fn validate_graph_dependencies(
    node_id: &BuildNodeId,
    nodes: &BTreeMap<BuildNodeId, BuildNode>,
    visiting: &mut BTreeSet<BuildNodeId>,
    visited: &mut BTreeSet<BuildNodeId>,
) -> Result<(), BuildError> {
    if visited.contains(node_id) {
        return Ok(());
    }
    if !visiting.insert(node_id.clone()) {
        return Err(BuildError::CyclicBuildGraphDependency(node_id.clone()));
    }
    let node = nodes
        .get(node_id)
        .ok_or_else(|| BuildError::UnknownBuildGraphNode(node_id.clone()))?;
    for dependency in &node.dependencies {
        validate_graph_dependencies(dependency, nodes, visiting, visited)?;
    }
    visiting.remove(node_id);
    visited.insert(node_id.clone());
    Ok(())
}

fn append_execution_order(
    node_id: &BuildNodeId,
    nodes: &BTreeMap<BuildNodeId, BuildNode>,
    visited: &mut BTreeSet<BuildNodeId>,
    order: &mut Vec<BuildNodeId>,
) {
    if !visited.insert(node_id.clone()) {
        return;
    }
    if let Some(node) = nodes.get(node_id) {
        for dependency in &node.dependencies {
            append_execution_order(dependency, nodes, visited, order);
        }
    }
    order.push(node_id.clone());
}

fn ensure_artifact_directory(path: &Path) -> Result<(), BuildError> {
    fs::create_dir_all(path).map_err(|error| BuildError::ArtifactIo {
        operation: "create artifact directory",
        message: error.to_string(),
    })?;
    let metadata = fs::symlink_metadata(path).map_err(|error| BuildError::ArtifactIo {
        operation: "inspect artifact directory",
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(BuildError::ArtifactPathSymlink);
    }
    if !metadata.is_dir() {
        return Err(BuildError::ArtifactPathNotDirectory);
    }
    Ok(())
}

fn copy_artifact_to_temporary(
    source_path: &Path,
    directory: &Path,
) -> Result<(PathBuf, String, u64), BuildError> {
    validate_regular_artifact_file(source_path)?;
    let mut source = File::open(source_path).map_err(|error| BuildError::ArtifactIo {
        operation: "open artifact source",
        message: error.to_string(),
    })?;
    let metadata = source.metadata().map_err(|error| BuildError::ArtifactIo {
        operation: "inspect opened artifact source",
        message: error.to_string(),
    })?;
    if !metadata.is_file() {
        return Err(BuildError::ArtifactSourceNotRegular);
    }
    let (temporary_path, mut temporary) = create_artifact_temporary(directory, "object")?;
    let mut hasher = blake3::Hasher::new();
    let mut byte_length = 0_usize;
    let mut buffer = [0_u8; 8_192];
    let copy_result = loop {
        let read = match source.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => break Err(error),
        };
        if read == 0 {
            break Ok(());
        }
        byte_length = byte_length.saturating_add(read);
        if byte_length > MAX_CARGO_INPUT_BYTES {
            break Err(std::io::Error::other(
                "artifact input exceeds bounded limit",
            ));
        }
        if let Err(error) = temporary.write_all(&buffer[..read]) {
            break Err(error);
        }
        hasher.update(&buffer[..read]);
    };
    if let Err(error) = copy_result.and_then(|()| temporary.sync_all()) {
        drop(temporary);
        let _ = fs::remove_file(&temporary_path);
        if byte_length > MAX_CARGO_INPUT_BYTES {
            return Err(BuildError::ArtifactTooLarge(byte_length));
        }
        return Err(BuildError::ArtifactIo {
            operation: "copy artifact object",
            message: error.to_string(),
        });
    }
    drop(temporary);
    Ok((
        temporary_path,
        hasher.finalize().to_hex().to_string(),
        byte_length as u64,
    ))
}

fn publish_immutable_artifact(
    temporary_path: &Path,
    object_path: &Path,
    content_hash: &str,
    byte_length: u64,
) -> Result<(), BuildError> {
    match publish_immutable_file(temporary_path, object_path) {
        Ok(()) => return Ok(()),
        Err(BuildError::ArtifactReferenceConflict) => {}
        Err(error) => return Err(error),
    }
    let (existing_hash, existing_length) = hash_artifact_file(object_path)?;
    if existing_hash == content_hash && existing_length == byte_length {
        fs::remove_file(temporary_path).map_err(|error| BuildError::ArtifactIo {
            operation: "remove reused temporary artifact file",
            message: error.to_string(),
        })
    } else {
        Err(BuildError::ArtifactObjectHashMismatch)
    }
}

fn publish_immutable_file(temporary_path: &Path, destination: &Path) -> Result<(), BuildError> {
    match fs::hard_link(temporary_path, destination) {
        Ok(()) => fs::remove_file(temporary_path).map_err(|error| BuildError::ArtifactIo {
            operation: "remove temporary artifact file",
            message: error.to_string(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(BuildError::ArtifactReferenceConflict)
        }
        Err(error) => Err(BuildError::ArtifactIo {
            operation: "publish immutable artifact file",
            message: error.to_string(),
        }),
    }
}

fn hash_artifact_file(path: &Path) -> Result<(String, u64), BuildError> {
    validate_regular_artifact_file(path)?;
    let mut file = File::open(path).map_err(|error| BuildError::ArtifactIo {
        operation: "open published artifact object",
        message: error.to_string(),
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut byte_length = 0_usize;
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| BuildError::ArtifactIo {
                operation: "read published artifact object",
                message: error.to_string(),
            })?;
        if read == 0 {
            break;
        }
        byte_length = byte_length.saturating_add(read);
        if byte_length > MAX_CARGO_INPUT_BYTES {
            return Err(BuildError::ArtifactTooLarge(byte_length));
        }
        hasher.update(&buffer[..read]);
    }
    Ok((hasher.finalize().to_hex().to_string(), byte_length as u64))
}

fn validate_regular_artifact_file(path: &Path) -> Result<(), BuildError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| BuildError::ArtifactIo {
        operation: "inspect artifact file",
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(BuildError::ArtifactPathSymlink);
    }
    if !metadata.is_file() {
        return Err(BuildError::ArtifactSourceNotRegular);
    }
    Ok(())
}

fn canonical_cargo_output_root(path: &Path) -> Result<PathBuf, BuildError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| BuildError::ArtifactIo {
        operation: "inspect Cargo output root",
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(BuildError::ArtifactPathSymlink);
    }
    if !metadata.is_dir() {
        return Err(BuildError::ArtifactPathNotDirectory);
    }
    fs::canonicalize(path).map_err(|error| BuildError::ArtifactIo {
        operation: "canonicalize Cargo output root",
        message: error.to_string(),
    })
}

fn create_artifact_temporary(directory: &Path, label: &str) -> Result<(PathBuf, File), BuildError> {
    for _ in 0..MAX_SNAPSHOT_TEMPORARY_ATTEMPTS {
        let temporary_id = NEXT_SNAPSHOT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            ".meridian-{label}-{}-{temporary_id}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(BuildError::ArtifactIo {
                    operation: "create temporary artifact file",
                    message: error.to_string(),
                });
            }
        }
    }
    Err(BuildError::ArtifactTemporaryExhausted)
}

fn write_artifact_temporary(
    directory: &Path,
    label: &str,
    contents: &[u8],
) -> Result<PathBuf, BuildError> {
    let (path, mut file) = create_artifact_temporary(directory, label)?;
    let write_result = file.write_all(contents).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&path);
        return Err(BuildError::ArtifactIo {
            operation: "write artifact reference",
            message: error.to_string(),
        });
    }
    Ok(path)
}

fn reference_file_name(node_id: &BuildNodeId) -> String {
    format!(
        "{}.json",
        blake3::hash(node_id.as_str().as_bytes()).to_hex()
    )
}

fn read_published_reference(path: &Path) -> Result<PublishedArtifact, BuildError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(BuildError::ArtifactReferenceMissing);
        }
        Err(error) => {
            return Err(BuildError::ArtifactIo {
                operation: "inspect artifact reference",
                message: error.to_string(),
            });
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(BuildError::ArtifactPathSymlink);
    }
    if !metadata.is_file() {
        return Err(BuildError::ArtifactSourceNotRegular);
    }
    let length = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
    if length > MAX_BUILD_SNAPSHOT_BYTES {
        return Err(BuildError::ArtifactReferenceTooLarge(length));
    }
    let bytes = fs::read(path).map_err(|error| BuildError::ArtifactIo {
        operation: "read artifact reference",
        message: error.to_string(),
    })?;
    let reference = serde_json::from_slice::<StoredPublishedArtifact>(&bytes)
        .map(PublishedArtifact::from)
        .map_err(|error| BuildError::MalformedArtifactReference(error.to_string()))?;
    validate_published_artifact(&reference)?;
    Ok(reference)
}

fn validate_published_artifact(reference: &PublishedArtifact) -> Result<(), BuildError> {
    validate_build_id(&reference.build_id)?;
    validate_text("published artifact node ID", reference.node_id.as_str())?;
    validate_text("published artifact schema", &reference.schema)?;
    validate_text(
        "published artifact tool ID and version",
        &reference.tool_id_version,
    )?;
    validate_artifact_hash(&reference.content_hash)?;
    if let Some(provenance) = reference.cargo_provenance.as_deref() {
        validate_cargo_artifact_provenance(provenance)?;
    }
    if let Some(provenance) = reference.build_input_provenance.as_deref() {
        validate_build_input_provenance(provenance, false)?;
    }
    Ok(())
}

fn matching_artifact_identity(first: &PublishedArtifact, second: &PublishedArtifact) -> bool {
    first.build_id == second.build_id
        && first.node_id == second.node_id
        && first.schema == second.schema
        && first.tool_id_version == second.tool_id_version
        && first.content_hash == second.content_hash
        && first.byte_length == second.byte_length
}

fn existing_provenance_is_compatible<T: PartialEq>(
    existing: Option<&T>,
    requested: Option<&T>,
) -> bool {
    match (existing, requested) {
        (Some(existing), Some(requested)) => existing == requested,
        (Some(_), None) => false,
        (None, _) => true,
    }
}

fn validate_cargo_artifact_provenance(
    provenance: &CargoArtifactProvenance,
) -> Result<(), BuildError> {
    validate_text("published Cargo package ID", &provenance.package_id)?;
    validate_text("published Cargo target name", &provenance.target_name)
}

fn validate_artifact_event(event: &BuildEvent) -> Result<(), BuildError> {
    match &event.payload {
        BuildEventPayload::Artifact(publication) => {
            validate_published_artifact(publication)?;
            if publication.build_id != event.build_id
                || publication.node_id != event.node_id
                || event.artifact_hash.as_deref() != Some(publication.content_hash.as_str())
            {
                return Err(BuildError::MismatchedArtifactEvent);
            }
            Ok(())
        }
        BuildEventPayload::Lifecycle
        | BuildEventPayload::Cargo(_)
        | BuildEventPayload::ProcessDiagnostic
            if event.artifact_hash.is_some() =>
        {
            Err(BuildError::MismatchedArtifactEvent)
        }
        BuildEventPayload::Lifecycle
        | BuildEventPayload::Cargo(_)
        | BuildEventPayload::ProcessDiagnostic => Ok(()),
    }
}

/// Validates an untrusted worker event before it can enter durable service state.
///
/// Public event types are deliberately Meridian-owned, but worker-produced
/// values are still untrusted: accepting an event must not bypass the parser's
/// Cargo field bounds, diagnostic redaction, or payload-to-event consistency
/// checks. The caller validates operation identity, sequence, and lifecycle
/// after this structural boundary succeeds.
fn validate_external_event(event: &BuildEvent) -> Result<(), BuildError> {
    validate_progress(event.progress)?;
    if event.protocol_version != BUILD_PROTOCOL_VERSION {
        return Err(BuildError::UnsupportedEventProtocolVersion(
            event.protocol_version,
        ));
    }
    validate_artifact_event(event)?;
    match &event.payload {
        BuildEventPayload::Cargo(CargoMessage::Diagnostic(diagnostic)) => {
            validate_external_cargo_diagnostic(diagnostic)?;
            if event.diagnostic.as_ref() != Some(diagnostic) {
                return Err(BuildError::MismatchedCargoDiagnosticEvent);
            }
        }
        BuildEventPayload::Cargo(CargoMessage::Artifact(artifact)) => {
            validate_cargo_artifact(artifact)?;
            if event.diagnostic.is_some() {
                return Err(BuildError::UnexpectedExternalEventDiagnostic);
            }
        }
        BuildEventPayload::ProcessDiagnostic => {
            let diagnostic = event
                .diagnostic
                .as_ref()
                .ok_or(BuildError::MissingExternalProcessDiagnostic)?;
            if sanitize_process_diagnostic(diagnostic.clone())? != *diagnostic {
                return Err(BuildError::ExternalDiagnosticNotRedacted);
            }
        }
        BuildEventPayload::Lifecycle
        | BuildEventPayload::Cargo(CargoMessage::Finished { .. })
        | BuildEventPayload::Artifact(_) => {
            if event.diagnostic.is_some() {
                return Err(BuildError::UnexpectedExternalEventDiagnostic);
            }
        }
    }
    Ok(())
}

fn validate_progress(progress: u8) -> Result<(), BuildError> {
    if progress > 100 {
        return Err(BuildError::InvalidProgress(progress));
    }
    Ok(())
}

/// Applies request and lifecycle constraints that only the receiving service can know.
///
/// Structural validation is deliberately separate because an external worker can
/// construct every public event field. The registered request remains the authority
/// for whether a payload is admissible at this lifecycle point and whether an
/// externally replayed publication has the same secret-safe input provenance.
fn validate_external_event_for_operation(
    event: &BuildEvent,
    request: &BuildRequest,
    current_phase: BuildPhase,
) -> Result<(), BuildError> {
    match &event.payload {
        BuildEventPayload::Cargo(_) | BuildEventPayload::ProcessDiagnostic => {
            validate_external_payload_running_phase(current_phase, event.phase)?;
        }
        BuildEventPayload::Artifact(publication) => {
            validate_external_payload_running_phase(current_phase, event.phase)?;
            let expected = request
                .input_provenance
                .as_ref()
                .ok_or(BuildError::MissingBuildInputProvenance)?;
            let received = publication
                .build_input_provenance
                .as_deref()
                .ok_or(BuildError::MissingBuildInputProvenance)?;
            if received != expected {
                return Err(BuildError::MismatchedBuildInputProvenance);
            }
        }
        BuildEventPayload::Lifecycle => {}
    }
    Ok(())
}

fn validate_external_payload_running_phase(
    current_phase: BuildPhase,
    event_phase: BuildPhase,
) -> Result<(), BuildError> {
    if current_phase != BuildPhase::Running || event_phase != BuildPhase::Running {
        return Err(BuildError::ExternalPayloadOutsideRunning {
            current: current_phase,
            event: event_phase,
        });
    }
    Ok(())
}

fn validate_external_cargo_diagnostic(diagnostic: &BuildDiagnostic) -> Result<(), BuildError> {
    if let Some(code) = &diagnostic.code {
        validate_text("Cargo diagnostic code", code)?;
    }
    validate_text("Cargo diagnostic message", &diagnostic.message)?;
    if redact_sensitive_assignments(&diagnostic.message) != diagnostic.message {
        return Err(BuildError::ExternalDiagnosticNotRedacted);
    }
    if let Some(rendered) = &diagnostic.rendered {
        validate_text("Cargo rendered diagnostic", rendered)?;
        if redact_sensitive_assignments(rendered) != *rendered {
            return Err(BuildError::ExternalDiagnosticNotRedacted);
        }
    }
    Ok(())
}

fn validate_artifact_hash(content_hash: &str) -> Result<(), BuildError> {
    if content_hash.len() != 64 || !content_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(BuildError::InvalidArtifactHash(content_hash.to_owned()));
    }
    Ok(())
}

fn validate_regular_snapshot_metadata(metadata: &fs::Metadata) -> Result<(), BuildError> {
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SnapshotPathSymlink);
    }
    if !metadata.is_file() {
        return Err(BuildError::SnapshotPathNotRegular);
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
    /// Run `cargo build` with Cargo's machine-readable JSON message protocol.
    Build,
    /// Compile `cargo test --no-run` with Cargo's machine-readable JSON message protocol.
    TestNoRun,
}

impl CargoCommand {
    /// Returns the Cargo subcommand name used in a structured process plan.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::Check => "check",
            Self::Build => "build",
            Self::TestNoRun => "test",
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
                if validate_environment_value(&value).is_ok() {
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
        validate_environment_value(&value)?;
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
            CargoCommand::Check | CargoCommand::Build | CargoCommand::TestNoRun => {
                let mut arguments = vec![
                    self.command.as_str().to_owned(),
                    "--locked".to_owned(),
                    "--quiet".to_owned(),
                    "--message-format=json".to_owned(),
                ];
                if self.command == CargoCommand::TestNoRun {
                    arguments.push("--no-run".to_owned());
                }
                arguments
            }
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
    /// Redacted Cargo process failure detail or cancellation-recovery warning.
    ///
    /// This is distinct from compiler diagnostics in the JSON stream. It is
    /// It is not a compiler diagnostic, artifact, or persistent build record.
    pub process_diagnostic: Option<BuildDiagnostic>,
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

impl CargoMetadataSnapshot {
    /// Hashes the workspace-local Cargo contract without checkout-specific paths.
    ///
    /// The returned value deliberately covers only workspace packages. Cargo's
    /// lockfile remains the authority for resolved external dependencies, while
    /// environment, toolchain, target, and source-checkpoint inputs remain
    /// local `BuildId` inputs.
    ///
    /// # Errors
    ///
    /// Returns an error when Cargo's reported workspace membership is
    /// incomplete or a workspace package manifest falls outside the reported
    /// workspace root.
    pub fn workspace_identity_hash(&self) -> Result<String, BuildError> {
        let members = self
            .workspace_members
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut packages = self
            .packages
            .iter()
            .filter(|package| members.contains(package.id.as_str()))
            .map(|package| {
                let manifest_path = workspace_relative_manifest_path(
                    Path::new(&self.workspace_root),
                    Path::new(&package.manifest_path),
                )?;
                let mut targets = package
                    .targets
                    .iter()
                    .map(|target| {
                        let mut kinds = target.kinds.clone();
                        kinds.sort_unstable();
                        CanonicalCargoTarget {
                            name: target.name.clone(),
                            kinds,
                        }
                    })
                    .collect::<Vec<_>>();
                targets.sort_by(|left, right| {
                    left.name
                        .cmp(&right.name)
                        .then_with(|| left.kinds.cmp(&right.kinds))
                });
                Ok(CanonicalCargoPackage {
                    manifest_path,
                    name: package.name.clone(),
                    version: package.version.clone(),
                    targets,
                })
            })
            .collect::<Result<Vec<_>, BuildError>>()?;
        if packages.len() != members.len() {
            return Err(BuildError::CargoMetadataWorkspaceMembershipMismatch);
        }
        packages.sort_by(|left, right| {
            left.manifest_path
                .cmp(&right.manifest_path)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.version.cmp(&right.version))
        });

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"meridian-cargo-workspace-identity-v1\0");
        hash_field(&mut hasher, "package-count", &packages.len().to_string());
        for package in packages {
            hash_field(&mut hasher, "manifest-path", &package.manifest_path);
            hash_field(&mut hasher, "package-name", &package.name);
            hash_field(&mut hasher, "package-version", &package.version);
            hash_field(
                &mut hasher,
                "target-count",
                &package.targets.len().to_string(),
            );
            for target in package.targets {
                hash_field(&mut hasher, "target-name", &target.name);
                hash_field(
                    &mut hasher,
                    "target-kind-count",
                    &target.kinds.len().to_string(),
                );
                for kind in target.kinds {
                    hash_field(&mut hasher, "target-kind", &kind);
                }
            }
        }
        Ok(hasher.finalize().to_hex().to_string())
    }
}

#[derive(Debug)]
struct CanonicalCargoPackage {
    manifest_path: String,
    name: String,
    version: String,
    targets: Vec<CanonicalCargoTarget>,
}

#[derive(Debug)]
struct CanonicalCargoTarget {
    name: String,
    kinds: Vec<String>,
}

/// One package returned by `cargo metadata`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoPackage {
    /// Cargo package ID.
    pub id: String,
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: String,
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
    /// BLAKE3 hash of the exact bounded metadata payload retained for traces.
    pub content_hash: Option<String>,
    /// BLAKE3 hash of the checkout-independent workspace metadata contract.
    ///
    /// The `BuildId` combines this with the lockfile hash, while its other local
    /// inputs (for example toolchain and environment) remain unchanged.
    pub workspace_identity_hash: Option<String>,
    /// Redacted Cargo process failure detail or cancellation-recovery warning.
    ///
    /// This is not a source of build identity.
    pub process_diagnostic: Option<BuildDiagnostic>,
}

/// Runs a structured Cargo check, build, or test compilation and parses its bounded JSON stream.
///
/// # Errors
///
/// Returns an error when Cargo cannot start, its JSON violates the bounded protocol,
/// or its process output cannot be read safely.
pub fn run_cargo_json(
    invocation: &CargoInvocation,
    cancellation: &BuildCancellation,
) -> Result<CargoRunOutcome, BuildError> {
    if !matches!(
        invocation.command,
        CargoCommand::Check | CargoCommand::Build | CargoCommand::TestNoRun
    ) {
        return Err(BuildError::UnexpectedCargoCommand {
            expected: CargoCommand::Check,
            received: invocation.command,
        });
    }
    if cancellation.is_cancelled() {
        return Ok(CargoRunOutcome {
            status: CargoRunStatus::Cancelled,
            messages: Vec::new(),
            process_diagnostic: None,
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
        .stderr(Stdio::piped());
    configure_cargo_child_process(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| BuildError::CargoSpawn(error.to_string()))?;
    let streams = match collect_cargo_json_streams(&mut child, cancellation)? {
        CargoStreamOutcome::Completed(streams) => streams,
        CargoStreamOutcome::Cancelled(process_diagnostic) => {
            return Ok(CargoRunOutcome {
                status: CargoRunStatus::Cancelled,
                messages: Vec::new(),
                process_diagnostic,
            });
        }
    };
    let status = child
        .wait()
        .map_err(|error| BuildError::CargoWait(error.to_string()))?;
    let status = if status.success() {
        CargoRunStatus::Succeeded
    } else {
        CargoRunStatus::Failed(status.code())
    };
    Ok(CargoRunOutcome {
        process_diagnostic: match status {
            CargoRunStatus::Succeeded | CargoRunStatus::Cancelled => None,
            CargoRunStatus::Failed(_) => cargo_process_failure_diagnostic(&streams.stderr),
        },
        status,
        messages: streams.messages,
    })
}

/// Fixed-worker, long-lived local coordinator for bounded Cargo jobs.
///
/// The supervisor owns no editor or project source state. It runs only validated
/// [`CargoInvocation`] values, keeps their cancellation handles, and applies
/// completed outcomes through [`DurableBuildService`] so worker loss and terminal
/// lifecycle state are persisted before they are returned to a host.
pub struct CargoBuildSupervisor {
    pool: TaskPool,
    pending: BTreeMap<OperationId, PendingCargoBuild>,
}

struct PendingCargoBuild {
    cancellation: BuildCancellation,
    task: Task<Result<CargoRunOutcome, BuildError>>,
}

/// One completed Cargo worker outcome and its durable service events.
pub struct CargoBuildCompletion {
    operation_id: OperationId,
    status: CargoRunStatus,
    events: Vec<BuildEvent>,
}

impl CargoBuildCompletion {
    /// Returns the completed operation.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// Returns Cargo's terminal status.
    #[must_use]
    pub const fn status(&self) -> CargoRunStatus {
        self.status
    }

    /// Returns accepted durable events in their emission order.
    #[must_use]
    pub fn events(&self) -> &[BuildEvent] {
        &self.events
    }
}

impl CargoBuildSupervisor {
    /// Starts the one bounded local Cargo worker permitted by `WP-BLD-001`.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the host cannot create the worker.
    pub fn try_new() -> Result<Self, BuildError> {
        Self::from_task_pool(TaskPool::try_new(NonZeroUsize::MIN))
    }

    fn from_task_pool(pool: Result<TaskPool, TaskError>) -> Result<Self, BuildError> {
        Ok(Self {
            pool: pool.map_err(|_| BuildError::CargoWorkerStart)?,
            pending: BTreeMap::new(),
        })
    }

    /// Returns the exact worker count owned by this supervisor.
    #[must_use]
    pub const fn worker_count(&self) -> NonZeroUsize {
        self.pool.worker_count()
    }

    /// Returns the number of Cargo jobs that have not yet produced a terminal outcome.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Starts one already-running request on a fixed worker.
    ///
    /// The request must be the service's exact running request and its root node
    /// must match the structured Cargo command. The worker receives a cloned,
    /// explicit invocation and cancellation handle; it inherits neither a shell
    /// nor ambient environment.
    ///
    /// # Errors
    ///
    /// Returns an error without replacing another pending job when the request is
    /// not the registered running request, its Cargo command does not match its
    /// root node, another operation is active, the operation is already
    /// supervised, or the worker pool closed.
    pub fn submit(
        &mut self,
        service: &DurableBuildService,
        request: &BuildRequest,
        invocation: CargoInvocation,
    ) -> Result<(), BuildError> {
        service.service.validate_running_request(request)?;
        if !cargo_command_matches_node(invocation.command, request.root_node.kind) {
            return Err(BuildError::CargoWorkerCommandMismatch);
        }
        if self.pending.contains_key(&request.operation_id) {
            return Err(BuildError::DuplicateSupervisedCargoOperation(
                request.operation_id,
            ));
        }
        if !self.pending.is_empty() {
            return Err(BuildError::CargoWorkerBusy);
        }
        let cancellation = BuildCancellation::default();
        let worker_cancellation = cancellation.clone();
        let task = self
            .pool
            .submit(move || run_cargo_json(&invocation, &worker_cancellation))
            .map_err(|_| BuildError::CargoWorkerPoolClosed)?;
        self.pending.insert(
            request.operation_id,
            PendingCargoBuild { cancellation, task },
        );
        Ok(())
    }

    /// Requests cancellation for one pending Cargo job and durably records it.
    ///
    /// Returns `Ok(None)` when the operation is not currently supervised.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation cannot enter its durable cancellation
    /// state; in that case the worker's cancellation handle is left unchanged.
    pub fn cancel(
        &mut self,
        service: &mut DurableBuildService,
        operation_id: OperationId,
    ) -> Result<Option<BuildEvent>, BuildError> {
        if !self.pending.contains_key(&operation_id) {
            return Ok(None);
        }
        let event = service.transition(operation_id, BuildPhase::CancelRequested, 100)?;
        if let Some(pending) = self.pending.get(&operation_id) {
            pending.cancellation.cancel();
        }
        Ok(Some(event))
    }

    /// Polls one completed worker in deterministic operation-ID order.
    ///
    /// The returned events have already passed the normal durable service
    /// validation. A panicked or disconnected task becomes a durable
    /// [`BuildPhase::WorkerLost`] transition; Cargo's nonzero exit remains a
    /// normal [`CargoRunStatus::Failed`] completion.
    pub fn poll(
        &mut self,
        service: &mut DurableBuildService,
    ) -> Option<Result<CargoBuildCompletion, BuildError>> {
        self.poll_with(service, |_, _, _| Ok(Vec::new()))
    }

    /// Polls one worker and gives the host one bounded publication hook before
    /// a successful or failed Cargo result becomes terminal.
    ///
    /// The hook receives only parsed Cargo messages after they were durably
    /// recorded. It may create request-bound artifacts while the operation is
    /// still Running. A hook failure durably fails the operation.
    pub fn poll_with<F>(
        &mut self,
        service: &mut DurableBuildService,
        before_terminal: F,
    ) -> Option<Result<CargoBuildCompletion, BuildError>>
    where
        F: FnOnce(
            &mut DurableBuildService,
            OperationId,
            &[CargoMessage],
        ) -> Result<Vec<BuildEvent>, BuildError>,
    {
        let (operation_id, task_result) =
            self.pending
                .iter_mut()
                .find_map(|(operation_id, pending)| {
                    pending.task.poll().map(|result| (*operation_id, result))
                })?;
        self.pending.remove(&operation_id);
        Some(match task_result {
            Ok(Ok(outcome)) => {
                complete_cargo_worker_outcome(service, operation_id, outcome, before_terminal)
            }
            Ok(Err(error)) => service
                .transition(operation_id, BuildPhase::Failed, 100)
                .map_or_else(Err, |_| Err(error)),
            Err(_) => service
                .transition(operation_id, BuildPhase::WorkerLost, 100)
                .map_or_else(Err, |_| Err(BuildError::CargoWorkerLost(operation_id))),
        })
    }
}

impl Drop for CargoBuildSupervisor {
    fn drop(&mut self) {
        for pending in self.pending.values() {
            pending.cancellation.cancel();
        }
    }
}

fn cargo_command_matches_node(command: CargoCommand, kind: BuildNodeKind) -> bool {
    matches!(
        (command, kind),
        (CargoCommand::Check, BuildNodeKind::CargoCheck)
            | (CargoCommand::Build, BuildNodeKind::CargoBuild)
            | (CargoCommand::TestNoRun, BuildNodeKind::CargoTestNoRun)
    )
}

fn complete_cargo_worker_outcome(
    service: &mut DurableBuildService,
    operation_id: OperationId,
    outcome: CargoRunOutcome,
    before_terminal: impl FnOnce(
        &mut DurableBuildService,
        OperationId,
        &[CargoMessage],
    ) -> Result<Vec<BuildEvent>, BuildError>,
) -> Result<CargoBuildCompletion, BuildError> {
    let mut events = Vec::new();
    let cancellation_requested =
        service.service().phase(operation_id)? == BuildPhase::CancelRequested;
    let status = if cancellation_requested {
        CargoRunStatus::Cancelled
    } else {
        outcome.status
    };
    let mut before_terminal = Some(before_terminal);
    match status {
        CargoRunStatus::Cancelled => {
            events.push(service.transition(operation_id, BuildPhase::Cancelled, 100)?);
        }
        CargoRunStatus::Succeeded | CargoRunStatus::Failed(_) => {
            if let Err(error) = validate_cargo_outcome_finished(&outcome) {
                service.transition(operation_id, BuildPhase::Failed, 100)?;
                return Err(error);
            }
            for message in &outcome.messages {
                events.push(service.record_cargo_message(operation_id, message.clone())?);
            }
            if let Some(diagnostic) = outcome.process_diagnostic {
                events.push(service.record_process_diagnostic(operation_id, diagnostic)?);
            }
            if outcome.status == CargoRunStatus::Succeeded {
                match before_terminal.take().expect("publication hook is present")(
                    service,
                    operation_id,
                    &outcome.messages,
                ) {
                    Ok(publication_events) => events.extend(publication_events),
                    Err(error) => {
                        let _ = service.transition(operation_id, BuildPhase::Failed, 100);
                        return Err(error);
                    }
                }
            }
            let phase = match outcome.status {
                CargoRunStatus::Succeeded => BuildPhase::Succeeded,
                CargoRunStatus::Failed(_) => BuildPhase::Failed,
                CargoRunStatus::Cancelled => unreachable!("cancelled outcome handled above"),
            };
            events.push(service.transition(operation_id, phase, 100)?);
        }
    }
    Ok(CargoBuildCompletion {
        operation_id,
        status,
        events,
    })
}

fn validate_cargo_outcome_finished(outcome: &CargoRunOutcome) -> Result<(), BuildError> {
    if outcome.status == CargoRunStatus::Cancelled {
        return Ok(());
    }
    let mut reported = None;
    for message in &outcome.messages {
        let CargoMessage::Finished { success } = message else {
            continue;
        };
        if let Some(previous) = reported {
            if previous != *success {
                return Err(BuildError::ConflictingCargoFinishedOutcome {
                    previous,
                    received: *success,
                });
            }
            continue;
        }
        reported = Some(*success);
    }
    if let Some(reported_success) = reported {
        validate_cargo_finished_phase(
            Some(reported_success),
            terminal_phase_for_cargo_status(outcome.status),
        )?;
    }
    Ok(())
}

fn terminal_phase_for_cargo_status(status: CargoRunStatus) -> BuildPhase {
    match status {
        CargoRunStatus::Succeeded => BuildPhase::Succeeded,
        CargoRunStatus::Failed(_) => BuildPhase::Failed,
        CargoRunStatus::Cancelled => BuildPhase::Cancelled,
    }
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
            workspace_identity_hash: None,
            process_diagnostic: None,
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
        .stderr(Stdio::piped());
    configure_cargo_child_process(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| BuildError::CargoSpawn(error.to_string()))?;
    let streams = match collect_cargo_metadata_streams(&mut child, cancellation)? {
        CargoMetadataStreamOutcome::Completed(streams) => streams,
        CargoMetadataStreamOutcome::Cancelled(process_diagnostic) => {
            return Ok(CargoMetadataOutcome {
                status: CargoRunStatus::Cancelled,
                snapshot: None,
                content_hash: None,
                workspace_identity_hash: None,
                process_diagnostic,
            });
        }
    };
    let status = child
        .wait()
        .map_err(|error| BuildError::CargoWait(error.to_string()))?;
    if !status.success() {
        return Ok(CargoMetadataOutcome {
            status: CargoRunStatus::Failed(status.code()),
            snapshot: None,
            content_hash: None,
            workspace_identity_hash: None,
            process_diagnostic: cargo_process_failure_diagnostic(&streams.stderr),
        });
    }
    let text = String::from_utf8(streams.bytes).map_err(|_| BuildError::CargoOutputNotUtf8)?;
    let snapshot = parse_cargo_metadata(&text)?;
    let content_hash = blake3::hash(text.as_bytes()).to_hex().to_string();
    let workspace_identity_hash = snapshot.workspace_identity_hash()?;
    Ok(CargoMetadataOutcome {
        status: CargoRunStatus::Succeeded,
        snapshot: Some(snapshot),
        content_hash: Some(content_hash),
        workspace_identity_hash: Some(workspace_identity_hash),
        process_diagnostic: None,
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
        validate_text("Cargo metadata package version", &package.version)?;
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
            version: package.version,
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

fn workspace_relative_manifest_path(
    workspace_root: &Path,
    manifest_path: &Path,
) -> Result<String, BuildError> {
    let relative = manifest_path
        .strip_prefix(workspace_root)
        .map_err(|_| BuildError::CargoMetadataWorkspaceManifestOutsideRoot)?;
    let mut segments = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(segment) => segments.push(
                segment
                    .to_str()
                    .ok_or(BuildError::CargoMetadataWorkspaceManifestOutsideRoot)?,
            ),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(BuildError::CargoMetadataWorkspaceManifestOutsideRoot);
            }
        }
    }
    if segments.is_empty() {
        return Err(BuildError::CargoMetadataWorkspaceManifestOutsideRoot);
    }
    Ok(segments.join("/"))
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
    version: String,
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
    let artifact = CargoArtifact {
        package_id,
        target_name,
        filenames,
        executable: raw.executable,
    };
    validate_cargo_artifact(&artifact)?;
    Ok(CargoMessage::Artifact(artifact))
}

fn validate_cargo_artifact(artifact: &CargoArtifact) -> Result<(), BuildError> {
    if artifact.filenames.len() > MAX_FILENAMES {
        return Err(BuildError::TooManyArtifactFilenames(
            artifact.filenames.len(),
        ));
    }
    validate_text("Cargo package ID", &artifact.package_id)?;
    validate_text("Cargo target name", &artifact.target_name)?;
    for filename in &artifact.filenames {
        validate_text("Cargo artifact filename", filename)?;
    }
    if let Some(executable) = &artifact.executable {
        validate_text("Cargo artifact executable", executable)?;
    }
    Ok(())
}

struct CargoStreams {
    messages: Vec<CargoMessage>,
    stderr: String,
}

struct CargoOutputLine {
    text: String,
    raw_bytes: usize,
}

#[derive(Default)]
struct CargoOutputBudget {
    bytes: usize,
    lines: usize,
}

impl CargoOutputBudget {
    fn accept_line(&mut self, bytes: usize) -> Result<(), BuildError> {
        self.lines = self.lines.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes);
        if self.lines > MAX_CARGO_JSON_LINES || self.bytes > MAX_CARGO_JSON_OUTPUT_BYTES {
            return Err(BuildError::CargoOutputAggregateTooLarge {
                bytes: self.bytes,
                lines: self.lines,
            });
        }
        Ok(())
    }
}

enum CargoStreamOutcome {
    Completed(CargoStreams),
    Cancelled(Option<BuildDiagnostic>),
}

fn collect_cargo_json_streams(
    child: &mut std::process::Child,
    cancellation: &BuildCancellation,
) -> Result<CargoStreamOutcome, BuildError> {
    let stdout = child.stdout.take().ok_or(BuildError::MissingCargoStdout)?;
    let stderr = child.stderr.take().ok_or(BuildError::MissingCargoStderr)?;
    let (stdout_sender, stdout_receiver) = mpsc::sync_channel(1);
    let mut stdout_reader = Some(thread::spawn(move || {
        read_cargo_lines(stdout, stdout_sender);
    }));
    let (stderr_sender, stderr_receiver) = mpsc::channel();
    let mut stderr_reader = Some(thread::spawn(move || {
        let _ = stderr_sender.send(read_cargo_stderr(stderr));
    }));
    let mut messages = Vec::new();
    let mut output_budget = CargoOutputBudget::default();
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    let mut process_stderr = None;

    macro_rules! stop_streams {
        () => {{
            drop(stdout_receiver);
            stop_cargo_process(child, stdout_reader.take(), stderr_reader.take())
        }};
    }

    while !stdout_closed || !stderr_closed {
        if cancellation.is_cancelled() {
            let process_diagnostic = stop_streams!();
            return Ok(CargoStreamOutcome::Cancelled(process_diagnostic));
        }
        if !stderr_closed {
            match stderr_receiver.try_recv() {
                Ok(result) => {
                    process_stderr = match result {
                        Ok(stderr) => Some(stderr),
                        Err(error) => {
                            stop_streams!();
                            return Err(error);
                        }
                    };
                    stderr_closed = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    stop_streams!();
                    return Err(BuildError::CargoReaderPanicked);
                }
            }
        }
        if stdout_closed {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        match stdout_receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(line)) => {
                if let Err(error) = output_budget.accept_line(line.raw_bytes) {
                    stop_streams!();
                    return Err(error);
                }
                let message = match parse_cargo_json_line(&line.text) {
                    Ok(message) => message,
                    Err(error) => {
                        stop_streams!();
                        return Err(error);
                    }
                };
                if let Some(message) = message {
                    messages.push(message);
                }
            }
            Ok(Err(error)) => {
                stop_streams!();
                return Err(error);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => stdout_closed = true,
        }
    }

    let stdout_reader = stdout_reader
        .take()
        .ok_or(BuildError::CargoReaderPanicked)?;
    let stderr_reader = stderr_reader
        .take()
        .ok_or(BuildError::CargoReaderPanicked)?;
    stdout_reader
        .join()
        .map_err(|_| BuildError::CargoReaderPanicked)?;
    stderr_reader
        .join()
        .map_err(|_| BuildError::CargoReaderPanicked)?;
    Ok(CargoStreamOutcome::Completed(CargoStreams {
        messages,
        stderr: process_stderr.ok_or(BuildError::CargoReaderPanicked)?,
    }))
}

struct CargoMetadataStreams {
    bytes: Vec<u8>,
    stderr: String,
}

enum CargoMetadataStreamOutcome {
    Completed(CargoMetadataStreams),
    Cancelled(Option<BuildDiagnostic>),
}

fn collect_cargo_metadata_streams(
    child: &mut std::process::Child,
    cancellation: &BuildCancellation,
) -> Result<CargoMetadataStreamOutcome, BuildError> {
    let stdout = child.stdout.take().ok_or(BuildError::MissingCargoStdout)?;
    let stderr = child.stderr.take().ok_or(BuildError::MissingCargoStderr)?;
    let (stdout_sender, stdout_receiver) = mpsc::channel();
    let mut stdout_reader = Some(thread::spawn(move || {
        let _ = stdout_sender.send(read_bounded_stream(stdout, MAX_CARGO_METADATA_BYTES));
    }));
    let (stderr_sender, stderr_receiver) = mpsc::channel();
    let mut stderr_reader = Some(thread::spawn(move || {
        let _ = stderr_sender.send(read_cargo_stderr(stderr));
    }));
    let mut metadata_bytes = None;
    let mut process_stderr = None;

    while metadata_bytes.is_none() || process_stderr.is_none() {
        if cancellation.is_cancelled() {
            let process_diagnostic =
                stop_cargo_process(child, stdout_reader.take(), stderr_reader.take());
            return Ok(CargoMetadataStreamOutcome::Cancelled(process_diagnostic));
        }
        if metadata_bytes.is_none() {
            match stdout_receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(result) => match result {
                    Ok(bytes) => metadata_bytes = Some(bytes),
                    Err(error) => {
                        stop_cargo_process(child, stdout_reader.take(), stderr_reader.take());
                        return Err(error);
                    }
                },
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    stop_cargo_process(child, stdout_reader.take(), stderr_reader.take());
                    return Err(BuildError::CargoReaderPanicked);
                }
            }
        }
        if process_stderr.is_none() {
            match stderr_receiver.try_recv() {
                Ok(result) => match result {
                    Ok(stderr) => process_stderr = Some(stderr),
                    Err(error) => {
                        stop_cargo_process(child, stdout_reader.take(), stderr_reader.take());
                        return Err(error);
                    }
                },
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    stop_cargo_process(child, stdout_reader.take(), stderr_reader.take());
                    return Err(BuildError::CargoReaderPanicked);
                }
            }
        }
    }

    let stdout_reader = stdout_reader
        .take()
        .ok_or(BuildError::CargoReaderPanicked)?;
    let stderr_reader = stderr_reader
        .take()
        .ok_or(BuildError::CargoReaderPanicked)?;
    stdout_reader
        .join()
        .map_err(|_| BuildError::CargoReaderPanicked)?;
    stderr_reader
        .join()
        .map_err(|_| BuildError::CargoReaderPanicked)?;
    Ok(CargoMetadataStreamOutcome::Completed(
        CargoMetadataStreams {
            bytes: metadata_bytes.ok_or(BuildError::CargoReaderPanicked)?,
            stderr: process_stderr.ok_or(BuildError::CargoReaderPanicked)?,
        },
    ))
}

#[allow(clippy::needless_pass_by_value)]
fn read_cargo_lines(
    mut stdout: impl Read,
    sender: mpsc::SyncSender<Result<CargoOutputLine, BuildError>>,
) {
    let mut bytes = Vec::with_capacity(512);
    loop {
        match read_bounded_line(&mut stdout, &mut bytes) {
            Ok(false) => break,
            Ok(true) => {
                let raw_bytes = bytes.len();
                if let Ok(mut line) = String::from_utf8(bytes.clone()) {
                    while matches!(line.chars().last(), Some('\n' | '\r')) {
                        line.pop();
                    }
                    if sender
                        .send(Ok(CargoOutputLine {
                            text: line,
                            raw_bytes,
                        }))
                        .is_err()
                    {
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

fn read_cargo_stderr(mut stderr: impl Read) -> Result<String, BuildError> {
    let mut output = Vec::with_capacity(512);
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = stderr
            .read(&mut buffer)
            .map_err(|error| BuildError::CargoRead(error.to_string()))?;
        if read == 0 {
            return String::from_utf8(output).map_err(|_| BuildError::CargoStderrNotUtf8);
        }
        let length = output.len().saturating_add(read);
        if length > MAX_CARGO_STDERR_BYTES {
            return Err(BuildError::CargoStderrTooLarge(length));
        }
        output.extend_from_slice(&buffer[..read]);
    }
}

fn cargo_process_failure_diagnostic(stderr: &str) -> Option<BuildDiagnostic> {
    let rendered = stderr.trim();
    if rendered.is_empty() {
        return None;
    }
    let rendered = redact_sensitive_assignments(rendered);
    let summary = rendered
        .lines()
        .find(|line| !line.trim().is_empty())
        .map_or_else(
            || "Cargo process failed".to_owned(),
            bounded_diagnostic_summary,
        );
    Some(BuildDiagnostic {
        code: None,
        severity: DiagnosticSeverity::Error,
        message: summary,
        rendered: Some(rendered),
    })
}

fn sanitize_process_diagnostic(
    mut diagnostic: BuildDiagnostic,
) -> Result<BuildDiagnostic, BuildError> {
    if let Some(code) = &diagnostic.code {
        validate_text("Cargo process diagnostic code", code)?;
    }
    validate_text("Cargo process diagnostic message", &diagnostic.message)?;
    diagnostic.message = redact_sensitive_assignments(&diagnostic.message);
    if let Some(rendered) = &diagnostic.rendered {
        validate_process_stderr(rendered)?;
        diagnostic.rendered = Some(redact_sensitive_assignments(rendered));
    }
    Ok(diagnostic)
}

fn validate_process_stderr(value: &str) -> Result<(), BuildError> {
    if value.is_empty() {
        return Err(BuildError::EmptyField("Cargo process diagnostic stderr"));
    }
    if value.len() > MAX_CARGO_STDERR_BYTES {
        return Err(BuildError::CargoStderrTooLarge(value.len()));
    }
    if value.contains('\0') {
        return Err(BuildError::NulByte("Cargo process diagnostic stderr"));
    }
    Ok(())
}

fn bounded_diagnostic_summary(input: &str) -> String {
    if input.len() <= MAX_FIELD_BYTES {
        return input.to_owned();
    }
    let mut end = MAX_FIELD_BYTES.saturating_sub("...".len());
    while !input.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}...", &input[..end])
}

/// Places Cargo in an isolated process group where the host supports it.
///
/// Cargo has no stable Rust API for cancelling its build-script and compiler
/// descendants. On Unix, a separate process group lets the cancellation path
/// terminate that bounded child tree without signalling the Meridian host.
#[cfg(unix)]
fn configure_cargo_child_process(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_cargo_child_process(_command: &mut Command) {}

fn stop_cargo_process(
    child: &mut std::process::Child,
    stdout_reader: Option<thread::JoinHandle<()>>,
    stderr_reader: Option<thread::JoinHandle<()>>,
) -> Option<BuildDiagnostic> {
    let process_diagnostic = terminate_cargo_process_tree(child);
    let _ = child.kill();
    let _ = child.wait();
    if let Some(reader) = stdout_reader {
        let _ = reader.join();
    }
    if let Some(reader) = stderr_reader {
        let _ = reader.join();
    }
    process_diagnostic
}

/// Enforces cancellation for the Cargo process and its inherited child tree.
///
/// The child process remains the authoritative PID. The platform-specific
/// launcher is always an explicit program plus argument vector; no command
/// shell or formatted command string is involved. If the platform tool cannot
/// run, the caller still terminates and reaps the direct Cargo child and emits
/// a typed warning instead of silently claiming tree cancellation.
fn terminate_cargo_process_tree(child: &mut std::process::Child) -> Option<BuildDiagnostic> {
    #[cfg(unix)]
    {
        let process_group = format!("-{}", child.id());
        if run_termination_command("/bin/kill", ["-TERM", process_group.as_str()]) {
            let deadline = std::time::Instant::now() + CARGO_CANCELLATION_GRACE;
            while std::time::Instant::now() < deadline {
                thread::sleep(CARGO_CANCELLATION_POLL);
            }
            // A group can outlive its Cargo leader when a build script or
            // compiler descendant ignores TERM. A nonzero second `kill` means
            // the TERM already removed that group for this same-user child.
            let _ = run_termination_command("/bin/kill", ["-KILL", process_group.as_str()]);
            return None;
        }
    }

    #[cfg(windows)]
    {
        if let Some(taskkill) = windows_taskkill_program() {
            let pid = child.id().to_string();
            if run_termination_command(taskkill, ["/PID", pid.as_str(), "/T", "/F"]) {
                return None;
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    let _ = child;

    Some(BuildDiagnostic {
        code: Some("build.cargo.process_tree_fallback".to_owned()),
        severity: DiagnosticSeverity::Warning,
        message: "Cargo process-tree cancellation could not be enforced; terminated the direct Cargo child only"
            .to_owned(),
        rendered: None,
    })
}

fn run_termination_command<I, S>(program: impl AsRef<std::ffi::OsStr>, arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(program)
        .args(arguments)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn windows_taskkill_program() -> Option<PathBuf> {
    std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("SYSTEMROOT"))
        .map(|root| PathBuf::from(root).join("System32").join("taskkill.exe"))
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
    validate_artifact_hash(&input.build_graph_contract)?;
    if input.command_arguments.len() > MAX_ARGUMENTS {
        return Err(BuildError::TooManyArguments(input.command_arguments.len()));
    }
    for argument in &input.command_arguments {
        validate_text("build command argument", argument)?;
    }
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
        validate_environment_value(value)?;
    }
    Ok(())
}

fn validate_build_request(
    request: &BuildRequest,
    require_provenance: bool,
) -> Result<(), BuildError> {
    validate_build_id(&request.build_id)?;
    validate_build_node(&request.root_node)?;
    match &request.input_provenance {
        Some(provenance) => {
            validate_build_input_provenance(provenance, require_provenance)?;
            if !provenance
                .root_node_ids
                .iter()
                .any(|root| root == request.root_node.id.as_str())
            {
                return Err(BuildError::BuildGraphIdentityRootsMismatch);
            }
        }
        None if require_provenance => return Err(BuildError::MissingBuildInputProvenance),
        None => {}
    }
    Ok(())
}

fn validate_build_input_provenance(
    provenance: &BuildInputProvenance,
    require_graph_contract: bool,
) -> Result<(), BuildError> {
    validate_text(
        "provenance source checkpoint",
        &provenance.source_checkpoint,
    )?;
    validate_text("provenance resolved profile", &provenance.resolved_profile)?;
    validate_text(
        "provenance Cargo metadata and lock",
        &provenance.cargo_metadata_and_lock,
    )?;
    validate_artifact_hash(&provenance.command_arguments_hash)?;
    match (&provenance.build_graph_contract, &provenance.build_graph) {
        (Some(contract), Some(graph)) => {
            validate_artifact_hash(contract)?;
            graph.validate()?;
            if graph.contract_hash() != contract {
                return Err(BuildError::MismatchedBuildGraphProvenance);
            }
        }
        (Some(contract), None) => {
            validate_artifact_hash(contract)?;
            if require_graph_contract {
                return Err(BuildError::MissingBuildGraphManifest);
            }
        }
        (None, Some(_)) => return Err(BuildError::MissingBuildGraphContract),
        (None, None) if require_graph_contract => {
            return Err(BuildError::MissingBuildGraphContract);
        }
        (None, None) => {}
    }
    validate_text(
        "provenance toolchain version",
        &provenance.toolchain_version,
    )?;
    validate_text(
        "provenance target and capabilities",
        &provenance.target_and_capabilities,
    )?;
    for (name, value_hash) in &provenance.environment_value_hashes {
        if !is_allowlisted_environment(name) {
            return Err(BuildError::EnvironmentNotAllowlisted(name.clone()));
        }
        validate_artifact_hash(value_hash)?;
    }
    if provenance.root_node_ids.is_empty() {
        return Err(BuildError::NoRootNodes);
    }
    let mut roots = BTreeSet::new();
    for root in &provenance.root_node_ids {
        validate_text("provenance root node ID", root)?;
        if !roots.insert(root) {
            return Err(BuildError::DuplicateRootNode(root.clone()));
        }
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

fn validate_environment_value(value: &str) -> Result<(), BuildError> {
    if value.is_empty() {
        return Err(BuildError::EmptyField("environment value"));
    }
    if value.len() > MAX_ENVIRONMENT_VALUE_BYTES {
        return Err(BuildError::FieldTooLong {
            field: "environment value",
            length: value.len(),
        });
    }
    if value.bytes().any(|byte| byte == 0) {
        return Err(BuildError::NulByte("environment value"));
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

fn hash_sorted_graph_values(hasher: &mut blake3::Hasher, name: &str, values: &[String]) {
    let mut values = values.iter().map(String::as_str).collect::<Vec<_>>();
    values.sort_unstable();
    hash_field(hasher, &format!("{name}-count"), &values.len().to_string());
    for value in values {
        hash_field(hasher, name, value);
    }
}

fn hash_provenance_values(label: &str, values: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"meridian-build-provenance-v1\0");
    hash_field(&mut hasher, "label", label);
    hash_field(&mut hasher, "value-count", &values.len().to_string());
    for value in values {
        hash_field(&mut hasher, "value", value);
    }
    hasher.finalize().to_hex().to_string()
}

fn hash_provenance_value(label: &str, name: &str, value: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"meridian-build-provenance-v1\0");
    hash_field(&mut hasher, "label", label);
    hash_field(&mut hasher, "name", name);
    hash_field(&mut hasher, "value", value);
    hasher.finalize().to_hex().to_string()
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

fn validate_cargo_finished_phase(
    cargo_finished: Option<bool>,
    phase: BuildPhase,
) -> Result<(), BuildError> {
    let Some(reported_success) = cargo_finished else {
        return Ok(());
    };
    let expected_success = match phase {
        BuildPhase::Succeeded => Some(true),
        BuildPhase::Failed => Some(false),
        BuildPhase::Queued
        | BuildPhase::Resolving
        | BuildPhase::Ready
        | BuildPhase::Running
        | BuildPhase::CancelRequested
        | BuildPhase::Cancelled
        | BuildPhase::WorkerLost
        | BuildPhase::Superseded => None,
    };
    if expected_success.is_some_and(|expected| expected != reported_success) {
        return Err(BuildError::CargoFinishedOutcomeMismatch {
            reported_success,
            phase,
        });
    }
    Ok(())
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
    /// Build graph declared no nodes.
    EmptyBuildGraph,
    /// Build graph exceeded the bounded node count.
    TooManyBuildGraphNodes(usize),
    /// Build graph repeated one node ID.
    DuplicateBuildGraphNode(BuildNodeId),
    /// A requested graph root does not name a graph node.
    UnknownBuildGraphRoot(BuildNodeId),
    /// A graph node declares a dependency absent from the graph.
    UnknownBuildGraphDependency {
        /// Dependent node declaring the invalid edge.
        node: BuildNodeId,
        /// Missing dependency target.
        dependency: BuildNodeId,
    },
    /// A graph node declares itself as a dependency.
    BuildGraphSelfDependency(BuildNodeId),
    /// A graph node repeated one dependency edge.
    DuplicateBuildGraphDependency {
        /// Node declaring the duplicate edge.
        node: BuildNodeId,
        /// Repeated dependency target.
        dependency: BuildNodeId,
    },
    /// A graph node repeated one immutable input hash declaration.
    DuplicateBuildGraphInput {
        /// Node declaring the duplicate input.
        node: BuildNodeId,
        /// Repeated immutable input hash.
        input: String,
    },
    /// A graph node repeated one declared environment name.
    DuplicateBuildGraphEnvironment {
        /// Node declaring the duplicate environment name.
        node: BuildNodeId,
        /// Repeated allowlisted environment name.
        name: String,
    },
    /// A dependency cycle prevents deterministic graph ordering.
    CyclicBuildGraphDependency(BuildNodeId),
    /// A graph node is unrelated to every requested build root.
    UnreachableBuildGraphNode(BuildNodeId),
    /// A caller queried or transitioned a node outside the graph.
    UnknownBuildGraphNode(BuildNodeId),
    /// A graph scheduler transition does not match the current node state.
    InvalidBuildGraphNodeTransition {
        /// Node whose state cannot change.
        node: BuildNodeId,
        /// Current scheduler state.
        current: BuildGraphNodeState,
        /// Requested scheduler state.
        next: BuildGraphNodeState,
    },
    /// A graph completion attempted to use a non-terminal operation phase.
    InvalidBuildGraphCompletion(BuildPhase),
    /// `BuildId` roots do not equal the graph's requested root set.
    BuildGraphIdentityRootsMismatch,
    /// `BuildId` did not include the canonical contract for its execution graph.
    BuildGraphIdentityContractMismatch,
    /// A single-root request named a different root than its validated graph.
    BuildGraphRequestRootMismatch,
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
    /// An event progress value falls outside the closed 0 through 100 range.
    InvalidProgress(u8),
    /// A new operation ID cannot be allocated after a caller supplied the maximum ID.
    OperationIdExhausted,
    /// A Cargo message arrived before the operation reached Running.
    CargoMessageOutsideRunning(BuildPhase),
    /// Cargo reported a final result that conflicts with the terminal lifecycle phase.
    CargoFinishedOutcomeMismatch {
        /// Result carried by Cargo's `build-finished` message.
        reported_success: bool,
        /// Terminal phase the build service attempted to persist.
        phase: BuildPhase,
    },
    /// One operation received conflicting Cargo `build-finished` results.
    ConflictingCargoFinishedOutcome {
        /// First result retained for the operation.
        previous: bool,
        /// Later conflicting result.
        received: bool,
    },
    /// A supervised Cargo command did not match the request's Cargo root node.
    CargoWorkerCommandMismatch,
    /// The caller attempted to supervise the same operation twice.
    DuplicateSupervisedCargoOperation(OperationId),
    /// The one-operation Cargo supervisor already has an active operation.
    CargoWorkerBusy,
    /// The fixed Cargo worker pool has already begun shutting down.
    CargoWorkerPoolClosed,
    /// The host could not create the bounded Cargo worker.
    CargoWorkerStart,
    /// A supervised Cargo worker panicked or disconnected before returning a result.
    CargoWorkerLost(OperationId),
    /// An external event has mismatched build or trace identity.
    MismatchedEventIdentity,
    /// An external event has a node ID outside the registered request.
    MismatchedNodeId,
    /// An external worker attached a non-lifecycle payload outside the running phase.
    ExternalPayloadOutsideRunning {
        current: BuildPhase,
        event: BuildPhase,
    },
    /// An external event declared a protocol version this service does not support.
    UnsupportedEventProtocolVersion(u16),
    /// An external event carried a diagnostic for a payload that cannot emit one.
    UnexpectedExternalEventDiagnostic,
    /// An external Cargo diagnostic did not match the event diagnostic field.
    MismatchedCargoDiagnosticEvent,
    /// An external process-diagnostic payload omitted its required diagnostic.
    MissingExternalProcessDiagnostic,
    /// An external diagnostic was not already sanitized for durable storage.
    ExternalDiagnosticNotRedacted,
    /// An artifact event did not carry the matching verified publication and hash.
    MismatchedArtifactEvent,
    /// A new request or artifact event omitted its required durable input provenance.
    MissingBuildInputProvenance,
    /// An artifact event carried provenance from a different request.
    MismatchedBuildInputProvenance,
    /// New durable provenance omitted the canonical execution-graph contract.
    MissingBuildGraphContract,
    /// New durable provenance omitted the validated execution-graph manifest.
    MissingBuildGraphManifest,
    /// A durable graph manifest did not produce its retained contract hash.
    MismatchedBuildGraphProvenance,
    /// A durable graph manifest was not in canonical node/root/value order.
    NonCanonicalBuildGraphProvenance,
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
    /// Cargo stderr pipe was unavailable after a successful spawn.
    MissingCargoStderr,
    /// Cargo stdout reader terminated unexpectedly.
    CargoReaderPanicked,
    /// Cargo process could not be waited on.
    CargoWait(String),
    /// Cargo stdout could not be read.
    CargoRead(String),
    /// Cargo emitted a line larger than the declared protocol limit.
    CargoOutputTooLarge(usize),
    /// Cargo exceeded the aggregate output bounds for one operation.
    CargoOutputAggregateTooLarge {
        /// Total raw JSON bytes observed before termination.
        bytes: usize,
        /// Total raw JSON lines observed before termination.
        lines: usize,
    },
    /// Cargo process stderr exceeded the retained diagnostic limit.
    CargoStderrTooLarge(usize),
    /// Cargo output was not valid UTF-8 JSON text.
    CargoOutputNotUtf8,
    /// Cargo process stderr was not valid UTF-8 text.
    CargoStderrNotUtf8,
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
    /// Cargo did not nominate an executable for requested publication.
    CargoArtifactExecutableMissing,
    /// Cargo's executable path was absent from its artifact filename list.
    CargoArtifactExecutableNotListed,
    /// Cargo's executable path was not absolute.
    CargoArtifactExecutableNotAbsolute,
    /// Cargo's executable path escaped the declared output root.
    CargoArtifactExecutableOutsideOutputRoot,
    /// More or fewer than one Cargo executable was eligible for one publication.
    CargoArtifactExecutableCount(usize),
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
    /// Cargo metadata did not provide exactly one resolved package per workspace member.
    CargoMetadataWorkspaceMembershipMismatch,
    /// A Cargo workspace member manifest fell outside the declared workspace root.
    CargoMetadataWorkspaceManifestOutsideRoot,
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
    /// A durable state path was empty or did not name a file.
    InvalidSnapshotPath,
    /// A durable state snapshot was requested before it exists.
    SnapshotMissing,
    /// A durable state path or its direct parent resolves through a symlink.
    SnapshotPathSymlink,
    /// A durable state path exists but is not a regular file.
    SnapshotPathNotRegular,
    /// A durable state parent exists but is not a directory.
    SnapshotParentNotDirectory,
    /// A durable state operation exhausted unique temporary-file attempts.
    SnapshotTemporaryExhausted,
    /// A durable state file was not valid UTF-8 JSON text.
    SnapshotNotUtf8,
    /// A durable state-file operation failed.
    SnapshotIo {
        /// Bounded operation label suitable for user-facing diagnostics.
        operation: &'static str,
        /// Platform error detail, treated as untrusted diagnostic text by callers.
        message: String,
    },
    /// Artifact store root was empty.
    InvalidArtifactRoot,
    /// An artifact path resolved through a symlink.
    ArtifactPathSymlink,
    /// An artifact path expected to be a directory was not a directory.
    ArtifactPathNotDirectory,
    /// An artifact source or reference path was not a regular file.
    ArtifactSourceNotRegular,
    /// An artifact source or existing published object exceeded the bounded limit.
    ArtifactTooLarge(usize),
    /// Artifact temporary-file creation exhausted collision retries.
    ArtifactTemporaryExhausted,
    /// A content hash did not use the required BLAKE3 hexadecimal representation.
    InvalidArtifactHash(String),
    /// An existing content-addressed object did not match its object name.
    ArtifactObjectHashMismatch,
    /// An existing BuildId/node reference named a different artifact.
    ArtifactReferenceConflict,
    /// A requested artifact reference was absent.
    ArtifactReferenceMissing,
    /// Artifact reference serialization failed.
    ArtifactSerialization(String),
    /// Artifact reference exceeded the bounded local state limit.
    ArtifactReferenceTooLarge(usize),
    /// Artifact reference data was malformed or structurally invalid.
    MalformedArtifactReference(String),
    /// Artifact-store filesystem operation failed.
    ArtifactIo {
        /// Bounded operation label suitable for user-facing diagnostics.
        operation: &'static str,
        /// Platform error detail, treated as untrusted diagnostic text by callers.
        message: String,
    },
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
            Self::EmptyBuildGraph => formatter.write_str("build graph needs at least one node"),
            Self::TooManyBuildGraphNodes(count) => {
                write!(formatter, "build graph has too many nodes ({count})")
            }
            Self::DuplicateBuildGraphNode(node) => {
                write!(formatter, "build graph duplicates node {node}")
            }
            Self::UnknownBuildGraphRoot(node) => {
                write!(formatter, "build graph root {node} is not declared")
            }
            Self::UnknownBuildGraphDependency { node, dependency } => {
                write!(
                    formatter,
                    "build node {node} depends on unknown node {dependency}"
                )
            }
            Self::BuildGraphSelfDependency(node) => {
                write!(formatter, "build node {node} cannot depend on itself")
            }
            Self::DuplicateBuildGraphDependency { node, dependency } => {
                write!(
                    formatter,
                    "build node {node} duplicates dependency {dependency}"
                )
            }
            Self::DuplicateBuildGraphInput { node, input } => {
                write!(formatter, "build node {node} duplicates input hash {input}")
            }
            Self::DuplicateBuildGraphEnvironment { node, name } => {
                write!(formatter, "build node {node} duplicates environment {name}")
            }
            Self::CyclicBuildGraphDependency(node) => {
                write!(formatter, "build graph has a dependency cycle at {node}")
            }
            Self::UnreachableBuildGraphNode(node) => {
                write!(
                    formatter,
                    "build graph node {node} is not reachable from a requested root"
                )
            }
            Self::UnknownBuildGraphNode(node) => {
                write!(formatter, "build graph node {node} is unknown")
            }
            Self::InvalidBuildGraphNodeTransition {
                node,
                current,
                next,
            } => write!(
                formatter,
                "invalid build graph transition for {node} from {current:?} to {next:?}"
            ),
            Self::InvalidBuildGraphCompletion(phase) => {
                write!(
                    formatter,
                    "build graph cannot complete a node with {phase:?}"
                )
            }
            Self::BuildGraphIdentityRootsMismatch => {
                formatter.write_str("build graph roots do not match the BuildId identity roots")
            }
            Self::BuildGraphIdentityContractMismatch => formatter.write_str(
                "build graph contract does not match the BuildId identity input",
            ),
            Self::BuildGraphRequestRootMismatch => formatter.write_str(
                "single-root build request does not match the validated graph root",
            ),
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
            Self::InvalidProgress(progress) => {
                write!(formatter, "build progress {progress} is outside 0 through 100")
            }
            Self::OperationIdExhausted => {
                formatter.write_str("cannot allocate another build operation ID")
            }
            Self::CargoMessageOutsideRunning(phase) => {
                write!(
                    formatter,
                    "Cargo message cannot arrive while build is {phase:?}"
                )
            }
            Self::CargoFinishedOutcomeMismatch {
                reported_success,
                phase,
            } => write!(
                formatter,
                "Cargo build-finished success={reported_success} conflicts with terminal {phase:?}"
            ),
            Self::ConflictingCargoFinishedOutcome { previous, received } => write!(
                formatter,
                "Cargo build-finished success={received} conflicts with earlier success={previous}"
            ),
            Self::CargoWorkerCommandMismatch => formatter.write_str(
                "supervised Cargo command does not match the request root node",
            ),
            Self::DuplicateSupervisedCargoOperation(operation_id) => {
                write!(formatter, "Cargo worker already supervises operation {operation_id}")
            }
            Self::CargoWorkerBusy => {
                formatter.write_str("Cargo worker already has an active operation")
            }
            Self::CargoWorkerPoolClosed => {
                formatter.write_str("Cargo worker pool is closed")
            }
            Self::CargoWorkerStart => {
                formatter.write_str("Cargo worker could not start")
            }
            Self::CargoWorkerLost(operation_id) => {
                write!(formatter, "Cargo worker for operation {operation_id} was lost")
            }
            Self::MismatchedEventIdentity => {
                formatter.write_str("event build or trace identity does not match")
            }
            Self::MismatchedNodeId => {
                formatter.write_str("event node ID does not match the request")
            }
            Self::ExternalPayloadOutsideRunning { current, event } => write!(
                formatter,
                "external payload cannot move a build from {current:?} to {event:?}"
            ),
            Self::UnsupportedEventProtocolVersion(version) => write!(
                formatter,
                "event protocol version {version} is unsupported by build service version {BUILD_PROTOCOL_VERSION}"
            ),
            Self::UnexpectedExternalEventDiagnostic => {
                formatter.write_str("external event carries an unexpected diagnostic")
            }
            Self::MismatchedCargoDiagnosticEvent => {
                formatter.write_str("external Cargo diagnostic does not match its event diagnostic")
            }
            Self::MissingExternalProcessDiagnostic => {
                formatter.write_str("external process-diagnostic event lacks a diagnostic")
            }
            Self::ExternalDiagnosticNotRedacted => {
                formatter.write_str("external diagnostic is not sanitized for durable storage")
            }
            Self::MismatchedArtifactEvent => {
                formatter.write_str("artifact event does not match its verified publication")
            }
            Self::MissingBuildInputProvenance => {
                formatter.write_str("build request or artifact event lacks durable input provenance")
            }
            Self::MismatchedBuildInputProvenance => {
                formatter.write_str("artifact provenance does not match the running build request")
            }
            Self::MissingBuildGraphContract => {
                formatter.write_str("new build provenance lacks an execution-graph contract")
            }
            Self::MissingBuildGraphManifest => {
                formatter.write_str("new build provenance lacks an execution-graph manifest")
            }
            Self::MismatchedBuildGraphProvenance => {
                formatter.write_str("build graph manifest does not match its contract hash")
            }
            Self::NonCanonicalBuildGraphProvenance => {
                formatter.write_str("build graph manifest is not canonical")
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
            Self::MissingCargoStderr => formatter.write_str("Cargo stderr pipe was unavailable"),
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
            Self::CargoOutputAggregateTooLarge { bytes, lines } => write!(
                formatter,
                "Cargo JSON output exceeds {MAX_CARGO_JSON_OUTPUT_BYTES} bytes or {MAX_CARGO_JSON_LINES} lines ({bytes} bytes across {lines} lines)"
            ),
            Self::CargoStderrTooLarge(length) => {
                write!(
                    formatter,
                    "Cargo stderr exceeds {MAX_CARGO_STDERR_BYTES} bytes ({length} bytes)"
                )
            }
            Self::CargoOutputNotUtf8 => formatter.write_str("Cargo JSON output is not UTF-8"),
            Self::CargoStderrNotUtf8 => formatter.write_str("Cargo stderr is not UTF-8"),
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
            Self::CargoArtifactExecutableMissing => {
                formatter.write_str("Cargo artifact did not report an executable")
            }
            Self::CargoArtifactExecutableNotListed => {
                formatter.write_str("Cargo executable was not listed among artifact filenames")
            }
            Self::CargoArtifactExecutableNotAbsolute => {
                formatter.write_str("Cargo executable path must be absolute")
            }
            Self::CargoArtifactExecutableOutsideOutputRoot => {
                formatter.write_str("Cargo executable escaped the declared output root")
            }
            Self::CargoArtifactExecutableCount(count) => write!(
                formatter,
                "expected exactly one Cargo executable for publication ({count} found)"
            ),
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
            Self::CargoMetadataWorkspaceMembershipMismatch => formatter.write_str(
                "Cargo metadata did not resolve exactly one package for every workspace member",
            ),
            Self::CargoMetadataWorkspaceManifestOutsideRoot => formatter.write_str(
                "Cargo metadata workspace member manifest falls outside the workspace root",
            ),
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
            Self::InvalidSnapshotPath => {
                formatter.write_str("build-service state path must name a file")
            }
            Self::SnapshotMissing => formatter.write_str("build-service state file is missing"),
            Self::SnapshotPathSymlink => {
                formatter.write_str("build-service state path must not resolve through a symlink")
            }
            Self::SnapshotPathNotRegular => {
                formatter.write_str("build-service state path is not a regular file")
            }
            Self::SnapshotParentNotDirectory => {
                formatter.write_str("build-service state parent is not a directory")
            }
            Self::SnapshotTemporaryExhausted => {
                formatter.write_str("build-service state could not allocate a temporary file")
            }
            Self::SnapshotNotUtf8 => {
                formatter.write_str("build-service state file is not valid UTF-8")
            }
            Self::SnapshotIo { operation, message } | Self::ArtifactIo { operation, message } => {
                write!(formatter, "failed to {operation}: {message}")
            }
            Self::InvalidArtifactRoot => {
                formatter.write_str("artifact-store root directory must not be empty")
            }
            Self::ArtifactPathSymlink => {
                formatter.write_str("artifact path must not resolve through a symlink")
            }
            Self::ArtifactPathNotDirectory => {
                formatter.write_str("artifact path expected to be a directory is not a directory")
            }
            Self::ArtifactSourceNotRegular => {
                formatter.write_str("artifact source or reference path is not a regular file")
            }
            Self::ArtifactTooLarge(length) => {
                write!(
                    formatter,
                    "artifact exceeds the {MAX_CARGO_INPUT_BYTES}-byte first-slice limit ({length} bytes)"
                )
            }
            Self::ArtifactTemporaryExhausted => {
                formatter.write_str("artifact store could not allocate a temporary file")
            }
            Self::InvalidArtifactHash(content_hash) => {
                write!(formatter, "artifact hash {content_hash} is malformed")
            }
            Self::ArtifactObjectHashMismatch => {
                formatter.write_str("existing artifact object does not match its content hash")
            }
            Self::ArtifactReferenceConflict => {
                formatter.write_str("artifact reference already names different content")
            }
            Self::ArtifactReferenceMissing => formatter.write_str("artifact reference is missing"),
            Self::ArtifactSerialization(message) => {
                write!(formatter, "failed to serialize artifact reference: {message}")
            }
            Self::ArtifactReferenceTooLarge(length) => write!(
                formatter,
                "artifact reference exceeds the {MAX_BUILD_SNAPSHOT_BYTES}-byte limit ({length} bytes)"
            ),
            Self::MalformedArtifactReference(message) => {
                write!(formatter, "artifact reference is malformed: {message}")
            }
        }
    }
}

impl Error for BuildError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> BuildIdentityInput {
        let root = BuildNode::cargo_check(
            BuildNodeId::new("cargo-check").expect("node ID"),
            "cargo 1.90",
        )
        .expect("node");
        let graph =
            BuildGraph::new(vec![root.clone()], vec![root.id.clone()]).expect("single-root graph");
        BuildIdentityInput {
            source_checkpoint: "abc123".to_owned(),
            resolved_profile: "debug".to_owned(),
            cargo_metadata_and_lock: "lock-hash".to_owned(),
            build_graph_contract: graph.contract_hash(),
            command_arguments: Vec::new(),
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

    fn external_event(
        request: &BuildRequest,
        sequence: u64,
        payload: BuildEventPayload,
        diagnostic: Option<BuildDiagnostic>,
    ) -> BuildEvent {
        BuildEvent {
            protocol_version: BUILD_PROTOCOL_VERSION,
            build_id: request.build_id.clone(),
            operation_id: request.operation_id,
            node_id: request.root_node.id.clone(),
            sequence,
            phase: BuildPhase::Running,
            progress: 50,
            diagnostic,
            artifact_hash: None,
            trace_id: request.trace_id,
            payload,
        }
    }

    fn assert_external_artifact_provenance_is_rejected(
        service: &mut BuildService,
        event: &BuildEvent,
    ) {
        let mut tampered = event.clone();
        if let BuildEventPayload::Artifact(publication) = &mut tampered.payload {
            let mut different_identity = identity();
            different_identity.source_checkpoint = "external-tampering".to_owned();
            publication.build_input_provenance = Some(Box::new(
                BuildInputProvenance::from_identity(&different_identity)
                    .expect("tampered input provenance"),
            ));
        } else {
            panic!("published event should carry an artifact payload");
        }
        assert!(matches!(
            service.accept_external_event(&tampered),
            Err(BuildError::MismatchedBuildInputProvenance)
        ));
    }

    fn descendant_recovery_warning() -> BuildDiagnostic {
        BuildDiagnostic {
            code: None,
            severity: DiagnosticSeverity::Warning,
            message: "direct Cargo child was terminated".to_owned(),
            rendered: None,
        }
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
    fn identity_changes_when_ordered_command_arguments_change() {
        let mut first = identity();
        first.command_arguments = vec!["-p".to_owned(), "meridian-core".to_owned()];
        let mut second = first.clone();
        second.command_arguments = vec!["-p".to_owned(), "meridian-build".to_owned()];
        assert_ne!(
            BuildId::derive(&first).expect("first"),
            BuildId::derive(&second).expect("second")
        );

        let mut reordered = first.clone();
        reordered.command_arguments.reverse();
        assert_ne!(
            BuildId::derive(&first).expect("first"),
            BuildId::derive(&reordered).expect("reordered")
        );
    }

    #[test]
    fn cargo_environment_explicitly_admits_required_windows_roots() {
        let mut environment = CargoEnvironment::default();
        environment
            .insert("USERPROFILE", r"C:\Users\runner")
            .expect("Windows profile is allowlisted");
        environment
            .insert("SYSTEMROOT", r"C:\Windows")
            .expect("Windows system root is allowlisted");
        assert_eq!(
            environment.identity_values().get("USERPROFILE"),
            Some(&r"C:\Users\runner".to_owned())
        );
        assert!(matches!(
            environment.insert("APPDATA", r"C:\Users\runner\AppData"),
            Err(BuildError::EnvironmentNotAllowlisted(_))
        ));
    }

    #[test]
    fn cargo_environment_admits_explicit_windows_linker_context() {
        let mut environment = CargoEnvironment::default();
        for name in [
            "INCLUDE",
            "LIB",
            "LIBPATH",
            "UniversalCRTSdkDir",
            "UCRTVersion",
            "VCINSTALLDIR",
            "VCToolsInstallDir",
            "VSINSTALLDIR",
            "WindowsSdkDir",
            "WindowsSDKVersion",
            "VSCMD_ARG_HOST_ARCH",
            "VSCMD_ARG_TGT_ARCH",
            "VSCMD_VER",
        ] {
            environment
                .insert(name, format!("test-{name}"))
                .expect("declared Windows toolchain context is allowlisted");
        }
        assert_eq!(environment.identity_values().len(), 13);
    }

    #[test]
    fn cargo_environment_accepts_bounded_toolchain_search_paths() {
        let mut environment = CargoEnvironment::default();
        environment
            .insert("LIB", "C:\\toolchain;".repeat(512))
            .expect("bounded Visual Studio library search path");
        assert!(environment.identity_values().contains_key("LIB"));
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
    fn external_worker_events_revalidate_payloads_before_durable_acceptance() {
        let request = request(1);
        let mut service = BuildService::default();
        service.submit(request.clone()).expect("queued");
        service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(1), BuildPhase::Ready, 15)
            .expect("ready");
        service
            .transition(OperationId::new(1), BuildPhase::Running, 20)
            .expect("running");

        let mut incompatible = external_event(&request, 5, BuildEventPayload::Lifecycle, None);
        incompatible.protocol_version = BUILD_PROTOCOL_VERSION.saturating_add(1);
        assert!(matches!(
            service.accept_external_event(&incompatible),
            Err(BuildError::UnsupportedEventProtocolVersion(_))
        ));

        let oversized_artifact = CargoArtifact {
            package_id: "path+file:///workspace#fixture@0.1.0".to_owned(),
            target_name: "fixture".to_owned(),
            filenames: vec!["/tmp/fixture-output".to_owned(); MAX_FILENAMES + 1],
            executable: None,
        };
        assert!(matches!(
            service.accept_external_event(&external_event(
                &request,
                5,
                BuildEventPayload::Cargo(CargoMessage::Artifact(oversized_artifact)),
                None,
            )),
            Err(BuildError::TooManyArtifactFilenames(_))
        ));

        let raw_diagnostic = BuildDiagnostic {
            code: None,
            severity: DiagnosticSeverity::Error,
            message: "token=untrusted".to_owned(),
            rendered: Some("password=untrusted".to_owned()),
        };
        assert!(matches!(
            service.accept_external_event(&external_event(
                &request,
                5,
                BuildEventPayload::Cargo(CargoMessage::Diagnostic(raw_diagnostic.clone())),
                Some(raw_diagnostic),
            )),
            Err(BuildError::ExternalDiagnosticNotRedacted)
        ));
        assert!(matches!(
            service.accept_external_event(&external_event(
                &request,
                5,
                BuildEventPayload::ProcessDiagnostic,
                None,
            )),
            Err(BuildError::MissingExternalProcessDiagnostic)
        ));

        let redacted_diagnostic = BuildDiagnostic {
            code: Some("E0001".to_owned()),
            severity: DiagnosticSeverity::Error,
            message: "token=[REDACTED]".to_owned(),
            rendered: Some("password=[REDACTED]".to_owned()),
        };
        let mut terminal_diagnostic = external_event(
            &request,
            5,
            BuildEventPayload::Cargo(CargoMessage::Diagnostic(redacted_diagnostic.clone())),
            Some(redacted_diagnostic.clone()),
        );
        terminal_diagnostic.phase = BuildPhase::Succeeded;
        assert!(matches!(
            service.accept_external_event(&terminal_diagnostic),
            Err(BuildError::ExternalPayloadOutsideRunning {
                current: BuildPhase::Running,
                event: BuildPhase::Succeeded,
            })
        ));
        service
            .accept_external_event(&external_event(
                &request,
                5,
                BuildEventPayload::Cargo(CargoMessage::Diagnostic(redacted_diagnostic.clone())),
                Some(redacted_diagnostic),
            ))
            .expect("sanitized external Cargo diagnostic accepts");

        service
            .accept_external_event(&external_event(
                &request,
                6,
                BuildEventPayload::ProcessDiagnostic,
                Some(descendant_recovery_warning()),
            ))
            .expect("sanitized external process diagnostic accepts");
        assert_eq!(
            service.phase(OperationId::new(1)).expect("running phase"),
            BuildPhase::Running
        );
    }

    #[test]
    fn durable_service_preserves_state_when_an_external_event_is_rejected() {
        let directory = TemporaryDirectory::new();
        let request = request(1);
        let state_path = directory.state_path();
        let mut service =
            DurableBuildService::open(BuildServiceStore::new(&state_path).expect("state path"))
                .expect("durable service")
                .service;
        service.submit(request.clone()).expect("queued");
        service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(1), BuildPhase::Ready, 15)
            .expect("ready");
        service
            .transition(OperationId::new(1), BuildPhase::Running, 20)
            .expect("running");
        let invalid = BuildEvent {
            protocol_version: BUILD_PROTOCOL_VERSION,
            build_id: request.build_id.clone(),
            operation_id: request.operation_id,
            node_id: request.root_node.id.clone(),
            sequence: 5,
            phase: BuildPhase::Succeeded,
            progress: 50,
            diagnostic: Some(descendant_recovery_warning()),
            artifact_hash: None,
            trace_id: request.trace_id,
            payload: BuildEventPayload::ProcessDiagnostic,
        };
        assert!(matches!(
            service.accept_external_event(&invalid),
            Err(BuildError::ExternalPayloadOutsideRunning {
                current: BuildPhase::Running,
                event: BuildPhase::Succeeded,
            })
        ));
        drop(service);

        let recovery =
            DurableBuildService::open(BuildServiceStore::new(&state_path).expect("state path"))
                .expect("reopen");
        assert_eq!(recovery.recovery_events.len(), 1);
        assert_eq!(recovery.recovery_events[0].sequence, 5);
        assert_eq!(
            recovery
                .service
                .service()
                .phase(OperationId::new(1))
                .expect("phase"),
            BuildPhase::WorkerLost,
            "reopen must recover only the previously persisted running operation"
        );
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
    fn cargo_finished_result_must_match_terminal_phase_for_local_and_external_events() {
        let first_request = request(1);
        let mut service = BuildService::default();
        service.submit(first_request.clone()).expect("queued");
        service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(1), BuildPhase::Ready, 10)
            .expect("ready");
        service
            .transition(OperationId::new(1), BuildPhase::Running, 20)
            .expect("running");

        service
            .record_cargo_message(
                OperationId::new(1),
                CargoMessage::Finished { success: false },
            )
            .expect("Cargo failure result");
        assert!(matches!(
            service.transition(OperationId::new(1), BuildPhase::Succeeded, 100),
            Err(BuildError::CargoFinishedOutcomeMismatch {
                reported_success: false,
                phase: BuildPhase::Succeeded,
            })
        ));
        service
            .transition(OperationId::new(1), BuildPhase::Failed, 100)
            .expect("matching terminal failure");

        let request = request(2);
        let mut service = BuildService::default();
        service.submit(request.clone()).expect("queued");
        service
            .transition(OperationId::new(2), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(2), BuildPhase::Ready, 10)
            .expect("ready");
        service
            .transition(OperationId::new(2), BuildPhase::Running, 20)
            .expect("running");
        service
            .accept_external_event(&external_event(
                &request,
                5,
                BuildEventPayload::Cargo(CargoMessage::Finished { success: false }),
                None,
            ))
            .expect("external Cargo failure result");
        let mut terminal = external_event(&request, 6, BuildEventPayload::Lifecycle, None);
        terminal.phase = BuildPhase::Succeeded;
        terminal.progress = 100;
        assert!(matches!(
            service.accept_external_event(&terminal),
            Err(BuildError::CargoFinishedOutcomeMismatch {
                reported_success: false,
                phase: BuildPhase::Succeeded,
            })
        ));
        terminal.phase = BuildPhase::Failed;
        service
            .accept_external_event(&terminal)
            .expect("matching external terminal failure");
    }

    #[test]
    fn durable_snapshot_rejects_cargo_finished_terminal_mismatch() {
        let persisted = PersistedBuildService {
            version: BUILD_PROTOCOL_VERSION,
            operations: vec![PersistedBuildOperation {
                request: request(1),
                phase: BuildPhase::Succeeded,
                last_sequence: 5,
                cargo_finished: Some(false),
            }],
        };
        let snapshot = serde_json::to_string(&persisted).expect("snapshot serializes");
        assert!(matches!(
            BuildService::restore_json(&snapshot),
            Err(BuildError::CargoFinishedOutcomeMismatch {
                reported_success: false,
                phase: BuildPhase::Succeeded,
            })
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
    fn cargo_output_budget_rejects_aggregate_bytes_and_lines() {
        let mut bytes = CargoOutputBudget::default();
        bytes
            .accept_line(MAX_CARGO_JSON_OUTPUT_BYTES)
            .expect("exact aggregate byte limit accepts");
        assert!(matches!(
            bytes.accept_line(1),
            Err(BuildError::CargoOutputAggregateTooLarge { .. })
        ));

        let mut lines = CargoOutputBudget::default();
        for _ in 0..MAX_CARGO_JSON_LINES {
            lines.accept_line(0).expect("line within limit accepts");
        }
        assert!(matches!(
            lines.accept_line(0),
            Err(BuildError::CargoOutputAggregateTooLarge { .. })
        ));
    }

    #[test]
    fn progress_is_bounded_for_local_and_external_events() {
        let request = request(1);
        let mut service = BuildService::default();
        service.submit(request.clone()).expect("queued");
        assert!(matches!(
            service.transition(OperationId::new(1), BuildPhase::Resolving, 101),
            Err(BuildError::InvalidProgress(101))
        ));
        assert_eq!(
            service.phase(OperationId::new(1)).expect("queued phase"),
            BuildPhase::Queued
        );

        service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(1), BuildPhase::Ready, 15)
            .expect("ready");
        service
            .transition(OperationId::new(1), BuildPhase::Running, 20)
            .expect("running");
        let mut event = external_event(&request, 5, BuildEventPayload::Lifecycle, None);
        event.progress = 101;
        assert!(matches!(
            service.accept_external_event(&event),
            Err(BuildError::InvalidProgress(101))
        ));
    }

    #[test]
    fn metadata_parser_preserves_workspace_package_and_target_contracts() {
        let metadata = parse_cargo_metadata(
            r#"{"workspace_root":"/repo","workspace_members":["path+file:///repo#crate@0.1.0"],"packages":[{"id":"path+file:///repo#crate@0.1.0","name":"crate","version":"0.1.0","manifest_path":"/repo/Cargo.toml","targets":[{"name":"crate","kind":["lib"]}]}]}"#,
        )
        .expect("metadata");
        assert_eq!(metadata.workspace_root, "/repo");
        assert_eq!(metadata.packages[0].version, "0.1.0");
        assert_eq!(metadata.packages[0].targets[0].kinds, ["lib"]);
    }

    #[test]
    fn workspace_metadata_identity_ignores_checkout_root_but_tracks_contract_changes() {
        let first = parse_cargo_metadata(
            r#"{"workspace_root":"/checkouts/one","workspace_members":["path+file:///checkouts/one#root@0.1.0","path+file:///checkouts/one/editor/tool#tool@0.2.0"],"packages":[{"id":"path+file:///checkouts/one#root@0.1.0","name":"root","version":"0.1.0","manifest_path":"/checkouts/one/Cargo.toml","targets":[{"name":"root","kind":["lib"]}]},{"id":"path+file:///checkouts/one/editor/tool#tool@0.2.0","name":"tool","version":"0.2.0","manifest_path":"/checkouts/one/editor/tool/Cargo.toml","targets":[{"name":"tool","kind":["bin","example"]}]},{"id":"registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0","name":"serde","version":"1.0.0","manifest_path":"/cargo/registry/serde/Cargo.toml","targets":[{"name":"serde","kind":["lib"]}]}]}"#,
        )
        .expect("first metadata");
        let second = parse_cargo_metadata(
            r#"{"workspace_root":"/different/location","workspace_members":["path+file:///different/location#root@0.1.0","path+file:///different/location/editor/tool#tool@0.2.0"],"packages":[{"id":"path+file:///different/location#root@0.1.0","name":"root","version":"0.1.0","manifest_path":"/different/location/Cargo.toml","targets":[{"name":"root","kind":["lib"]}]},{"id":"path+file:///different/location/editor/tool#tool@0.2.0","name":"tool","version":"0.2.0","manifest_path":"/different/location/editor/tool/Cargo.toml","targets":[{"name":"tool","kind":["example","bin"]}]},{"id":"registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0","name":"serde","version":"1.0.0","manifest_path":"/cargo/registry/serde/Cargo.toml","targets":[{"name":"serde","kind":["lib"]}]}]}"#,
        )
        .expect("second metadata");
        assert_eq!(
            first.workspace_identity_hash().expect("first identity"),
            second.workspace_identity_hash().expect("second identity")
        );

        let changed = parse_cargo_metadata(
            r#"{"workspace_root":"/different/location","workspace_members":["path+file:///different/location#root@0.1.0","path+file:///different/location/editor/tool#tool@0.3.0"],"packages":[{"id":"path+file:///different/location#root@0.1.0","name":"root","version":"0.1.0","manifest_path":"/different/location/Cargo.toml","targets":[{"name":"root","kind":["lib"]}]},{"id":"path+file:///different/location/editor/tool#tool@0.3.0","name":"tool","version":"0.3.0","manifest_path":"/different/location/editor/tool/Cargo.toml","targets":[{"name":"tool","kind":["bin","example"]}]}]}"#,
        )
        .expect("changed metadata");
        assert_ne!(
            first.workspace_identity_hash().expect("first identity"),
            changed.workspace_identity_hash().expect("changed identity")
        );
    }

    #[test]
    fn workspace_metadata_identity_rejects_missing_or_outside_members() {
        let missing = parse_cargo_metadata(
            r#"{"workspace_root":"/repo","workspace_members":["path+file:///repo#missing@0.1.0"],"packages":[]}"#,
        )
        .expect("missing-member metadata");
        assert!(matches!(
            missing.workspace_identity_hash(),
            Err(BuildError::CargoMetadataWorkspaceMembershipMismatch)
        ));

        let outside = parse_cargo_metadata(
            r#"{"workspace_root":"/repo","workspace_members":["path+file:///repo#crate@0.1.0"],"packages":[{"id":"path+file:///repo#crate@0.1.0","name":"crate","version":"0.1.0","manifest_path":"/other/Cargo.toml","targets":[{"name":"crate","kind":["lib"]}]}]}"#,
        )
        .expect("outside-member metadata");
        assert!(matches!(
            outside.workspace_identity_hash(),
            Err(BuildError::CargoMetadataWorkspaceManifestOutsideRoot)
        ));
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
        for (command, expected_subcommand, expects_no_run) in [
            (CargoCommand::Check, "check", false),
            (CargoCommand::Build, "build", false),
            (CargoCommand::TestNoRun, "test", true),
        ] {
            let invocation = CargoInvocation::new(
                "/workspace",
                command,
                vec!["-p".to_owned(), "meridian-build".to_owned()],
                CargoEnvironment::default(),
            )
            .expect("invocation");
            let plan = invocation.command_plan();
            assert_eq!(plan.arguments[0], expected_subcommand);
            assert!(plan
                .arguments
                .iter()
                .any(|argument| argument == "--message-format=json"));
            assert_eq!(
                plan.arguments.iter().any(|argument| argument == "--no-run"),
                expects_no_run
            );
        }
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

    #[cfg(unix)]
    #[test]
    fn cancelling_cargo_terminates_an_inherited_build_script_child() {
        let directory = TemporaryDirectory::new();
        fs::create_dir_all(directory.path.join("src")).expect("fixture source directory");
        fs::write(
            directory.path.join("Cargo.toml"),
            "[package]\nname = \"cancellation-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n",
        )
        .expect("fixture manifest");
        fs::write(
            directory.path.join("Cargo.lock"),
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"cancellation-fixture\"\nversion = \"0.1.0\"\n",
        )
        .expect("fixture lockfile");
        fs::write(directory.path.join("src/lib.rs"), "pub fn fixture() {}\n")
            .expect("fixture library");
        fs::write(
            directory.path.join("build.rs"),
            "use std::fs;\nuse std::process::Command;\n\nfn main() {\n    fs::write(\"build-script-started\", \"started\").expect(\"marker\");\n    let _ = Command::new(\"/bin/sleep\").arg(\"60\").status();\n}\n",
        )
        .expect("fixture build script");

        let marker = directory.path.join("build-script-started");
        let cancellation = BuildCancellation::default();
        let cancellation_watcher = cancellation.clone();
        let watcher = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while std::time::Instant::now() < deadline {
                if marker.is_file() {
                    cancellation_watcher.cancel();
                    return true;
                }
                thread::sleep(Duration::from_millis(10));
            }
            false
        });
        let invocation = CargoInvocation::new(
            &directory.path,
            CargoCommand::Check,
            vec![
                "--target-dir".to_owned(),
                directory.path.join("target").display().to_string(),
            ],
            CargoEnvironment::from_host(),
        )
        .expect("fixture invocation");

        let started = std::time::Instant::now();
        let result = run_cargo_json(&invocation, &cancellation).expect("cancelled Cargo outcome");
        assert!(
            watcher.join().expect("cancellation watcher joins"),
            "build script did not start before cancellation timeout: {result:?}"
        );
        assert_eq!(result.status, CargoRunStatus::Cancelled);
        assert!(result.process_diagnostic.is_none());
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "cancelling Cargo must not wait for the build-script child"
        );
    }

    #[test]
    fn cargo_stderr_reuses_the_existing_service_bound() {
        let normal_fresh_check_output = vec![b'x'; 16 * 1_024 + 1];
        assert_eq!(
            read_cargo_stderr(std::io::Cursor::new(normal_fresh_check_output.clone()))
                .expect("normal Cargo status output fits the service bound"),
            String::from_utf8(normal_fresh_check_output).expect("fixture is UTF-8")
        );

        let oversized = vec![b'x'; MAX_CARGO_STDERR_BYTES + 1];
        assert!(matches!(
            read_cargo_stderr(std::io::Cursor::new(oversized)),
            Err(BuildError::CargoStderrTooLarge(length)) if length > MAX_CARGO_STDERR_BYTES
        ));
    }

    #[test]
    fn cargo_process_failure_is_bounded_and_redacted() {
        let invocation = CargoInvocation::new(
            env!("CARGO_MANIFEST_DIR"),
            CargoCommand::Check,
            vec!["-p".to_owned(), "missing-token=abc".to_owned()],
            CargoEnvironment::from_host(),
        )
        .expect("invocation");
        let result = run_cargo_json(&invocation, &BuildCancellation::default())
            .expect("Cargo process failure is a result, not an adapter failure");
        assert!(matches!(result.status, CargoRunStatus::Failed(_)));
        let diagnostic = result.process_diagnostic.expect("Cargo stderr diagnostic");
        let rendered = diagnostic.rendered.expect("rendered diagnostic");
        assert!(rendered.contains("missing-token=[REDACTED]"));
        assert!(!rendered.contains("missing-token=abc"));

        let mut service = BuildService::default();
        service.submit(request(2)).expect("queued");
        service
            .transition(OperationId::new(2), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(2), BuildPhase::Ready, 10)
            .expect("ready");
        service
            .transition(OperationId::new(2), BuildPhase::Running, 20)
            .expect("running");
        let event = service
            .record_process_diagnostic(
                OperationId::new(2),
                BuildDiagnostic {
                    code: None,
                    severity: DiagnosticSeverity::Error,
                    message: "token=abc".to_owned(),
                    rendered: Some("password=hunter2".to_owned()),
                },
            )
            .expect("process diagnostic event");
        assert!(matches!(
            event.payload,
            BuildEventPayload::ProcessDiagnostic
        ));
        assert_eq!(
            event
                .diagnostic
                .as_ref()
                .map(|value| value.message.as_str()),
            Some("token=[REDACTED]")
        );
        assert_eq!(
            event.diagnostic.and_then(|value| value.rendered).as_deref(),
            Some("password=[REDACTED]")
        );
    }

    #[test]
    fn metadata_process_failure_returns_a_typed_diagnostic() {
        let invocation = CargoInvocation::new(
            env!("CARGO_MANIFEST_DIR"),
            CargoCommand::Metadata,
            vec!["--unknown-token=abc".to_owned()],
            CargoEnvironment::from_host(),
        )
        .expect("invocation");
        let result = run_cargo_metadata(&invocation, &BuildCancellation::default())
            .expect("Cargo process failure is a result, not an adapter failure");
        assert!(matches!(result.status, CargoRunStatus::Failed(_)));
        let diagnostic = result
            .process_diagnostic
            .expect("Cargo metadata stderr diagnostic");
        let rendered = diagnostic.rendered.expect("rendered diagnostic");
        assert!(rendered.contains("unexpected argument"));
    }

    fn cargo_graph_nodes() -> (BuildNode, BuildNode) {
        let metadata = BuildNode::cargo_metadata(
            BuildNodeId::new("cargo-metadata").expect("metadata ID"),
            "cargo 1.90",
        )
        .expect("metadata node");
        let mut check = BuildNode::cargo_check(
            BuildNodeId::new("cargo-check").expect("check ID"),
            "cargo 1.90",
        )
        .expect("check node");
        check.dependencies.push(metadata.id.clone());
        (metadata, check)
    }

    #[test]
    fn graph_schedules_dependencies_before_requested_roots() {
        let (metadata, check) = cargo_graph_nodes();
        let graph = BuildGraph::new(
            vec![check.clone(), metadata.clone()],
            vec![check.id.clone()],
        )
        .expect("graph");
        assert_eq!(
            graph.execution_order(),
            vec![metadata.id.clone(), check.id.clone()]
        );
        let mut matching_identity = identity();
        matching_identity.build_graph_contract = graph.contract_hash();
        graph
            .validate_identity(&matching_identity)
            .expect("identity roots");

        let mut schedule = graph.schedule();
        assert_eq!(schedule.ready_nodes(), vec![metadata.id.clone()]);
        assert_eq!(
            schedule.start(&metadata.id).expect("metadata starts").state,
            BuildGraphNodeState::Running
        );
        assert_eq!(
            schedule
                .finish(&metadata.id, BuildPhase::Succeeded)
                .expect("metadata succeeds")
                .state,
            BuildGraphNodeState::Succeeded
        );
        assert_eq!(schedule.ready_nodes(), vec![check.id.clone()]);
        schedule.start(&check.id).expect("check starts");
        schedule
            .finish(&check.id, BuildPhase::Succeeded)
            .expect("check succeeds");
        assert!(schedule.is_complete());
    }

    #[test]
    fn graph_rejects_bad_edges_and_blocks_dependents_after_failure() {
        let (metadata, mut check) = cargo_graph_nodes();
        check
            .dependencies
            .push(BuildNodeId::new("missing").expect("missing ID"));
        assert!(matches!(
            BuildGraph::new(vec![metadata, check.clone()], vec![check.id.clone()]),
            Err(BuildError::UnknownBuildGraphDependency { .. })
        ));

        let (metadata, mut check) = cargo_graph_nodes();
        check.input_hashes = vec!["same-input".to_owned(), "same-input".to_owned()];
        assert!(matches!(
            BuildGraph::new(vec![metadata, check.clone()], vec![check.id.clone()]),
            Err(BuildError::DuplicateBuildGraphInput { .. })
        ));

        let (mut first, second) = cargo_graph_nodes();
        first.dependencies.push(second.id.clone());
        assert!(matches!(
            BuildGraph::new(vec![first.clone(), second.clone()], vec![second.id.clone()]),
            Err(BuildError::CyclicBuildGraphDependency(_))
        ));

        let (metadata, check) = cargo_graph_nodes();
        let orphan =
            BuildNode::cargo_metadata(BuildNodeId::new("orphan").expect("orphan ID"), "cargo 1.90")
                .expect("orphan node");
        assert!(matches!(
            BuildGraph::new(
                vec![metadata.clone(), check.clone(), orphan],
                vec![check.id.clone()]
            ),
            Err(BuildError::UnreachableBuildGraphNode(_))
        ));

        let graph = BuildGraph::new(
            vec![metadata.clone(), check.clone()],
            vec![check.id.clone()],
        )
        .expect("graph");
        let mut mismatched_identity = identity();
        mismatched_identity.root_node_ids = vec!["different-root".to_owned()];
        assert!(matches!(
            graph.validate_identity(&mismatched_identity),
            Err(BuildError::BuildGraphIdentityRootsMismatch)
        ));
        let mut schedule = graph.schedule();
        schedule.start(&metadata.id).expect("metadata starts");
        schedule
            .finish(&metadata.id, BuildPhase::Failed)
            .expect("metadata fails");
        assert_eq!(
            schedule.state(&check.id).expect("check state"),
            BuildGraphNodeState::Blocked
        );
        assert!(schedule.is_complete());
    }

    #[test]
    fn graph_contract_changes_build_identity_and_request_provenance() {
        let (metadata, check) = cargo_graph_nodes();
        let graph = BuildGraph::new(
            vec![metadata.clone(), check.clone()],
            vec![check.id.clone()],
        )
        .expect("first graph");
        let mut first_identity = identity();
        first_identity.build_graph_contract = graph.contract_hash();
        graph
            .validate_identity(&first_identity)
            .expect("first graph identity");
        let first_build_id = BuildId::derive(&first_identity).expect("first build ID");

        let mut changed_metadata = metadata;
        changed_metadata.tool_id_version = "cargo 1.91".to_owned();
        let changed_graph = BuildGraph::new(
            vec![changed_metadata, check.clone()],
            vec![check.id.clone()],
        )
        .expect("changed graph");
        assert!(matches!(
            changed_graph.validate_identity(&first_identity),
            Err(BuildError::BuildGraphIdentityContractMismatch)
        ));

        let mut changed_identity = first_identity.clone();
        changed_identity.build_graph_contract = changed_graph.contract_hash();
        let changed_build_id = BuildId::derive(&changed_identity).expect("changed build ID");
        assert_ne!(first_build_id, changed_build_id);
        let request = BuildRequest::new_with_graph(
            &changed_identity,
            OperationId::new(1),
            TraceId::new(7),
            check.clone(),
            &changed_graph,
        )
        .expect("graph-bound request");
        let changed_contract = changed_graph.contract_hash();
        assert_eq!(
            request
                .input_provenance
                .as_ref()
                .and_then(BuildInputProvenance::build_graph_contract),
            Some(changed_contract.as_str())
        );
        let manifest = request
            .input_provenance
            .as_ref()
            .and_then(BuildInputProvenance::build_graph)
            .expect("graph manifest");
        assert_eq!(manifest.contract_hash(), changed_contract);
        assert_eq!(manifest.requested_roots(), std::slice::from_ref(&check.id));
        assert_eq!(manifest.nodes().len(), 2);
    }

    #[test]
    fn durable_graph_provenance_must_match_the_canonical_contract() {
        let (metadata, check) = cargo_graph_nodes();
        let graph =
            BuildGraph::new(vec![metadata, check.clone()], vec![check.id.clone()]).expect("graph");
        let mut graph_identity = identity();
        graph_identity.build_graph_contract = graph.contract_hash();
        let request = BuildRequest::new_with_graph(
            &graph_identity,
            OperationId::new(1),
            TraceId::new(7),
            check,
            &graph,
        )
        .expect("request");
        let provenance = request.input_provenance.expect("request provenance");
        validate_build_input_provenance(&provenance, true).expect("canonical provenance");

        let mut missing_manifest = provenance.clone();
        missing_manifest.build_graph = None;
        assert!(matches!(
            validate_build_input_provenance(&missing_manifest, true),
            Err(BuildError::MissingBuildGraphManifest)
        ));

        let mut mismatched_manifest = provenance.clone();
        mismatched_manifest
            .build_graph
            .as_mut()
            .expect("manifest")
            .contract_hash = "0".repeat(64);
        assert!(matches!(
            validate_build_input_provenance(&mismatched_manifest, true),
            Err(BuildError::MismatchedBuildGraphProvenance)
        ));

        let mut noncanonical_manifest = provenance;
        noncanonical_manifest
            .build_graph
            .as_mut()
            .expect("manifest")
            .nodes
            .reverse();
        assert!(matches!(
            validate_build_input_provenance(&noncanonical_manifest, true),
            Err(BuildError::NonCanonicalBuildGraphProvenance)
        ));
    }

    struct TemporaryDirectory {
        path: PathBuf,
    }

    impl TemporaryDirectory {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("meridian-build-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).expect("temporary directory creates");
            Self { path }
        }

        fn state_path(&self) -> PathBuf {
            self.path.join("build-service-state.json")
        }

        fn artifact_root(&self) -> PathBuf {
            self.path.join("artifacts")
        }

        fn artifact_source(&self) -> PathBuf {
            self.path.join("artifact-input.bin")
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn running_durable_service(
        directory: &TemporaryDirectory,
        request: BuildRequest,
    ) -> DurableBuildService {
        let mut service = DurableBuildService::open(
            BuildServiceStore::new(directory.state_path()).expect("state"),
        )
        .expect("durable service")
        .service;
        let operation_id = request.operation_id;
        service.submit(request).expect("queued");
        service
            .transition(operation_id, BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(operation_id, BuildPhase::Ready, 15)
            .expect("ready");
        service
            .transition(operation_id, BuildPhase::Running, 20)
            .expect("running");
        service
    }

    #[test]
    fn cargo_worker_supervisor_completes_and_cancels_durable_requests() {
        let directory = TemporaryDirectory::new();
        let initial_request = request(1);
        let mut service = running_durable_service(&directory, initial_request.clone());
        let invocation = CargoInvocation::new(
            env!("CARGO_MANIFEST_DIR"),
            CargoCommand::Check,
            vec!["-p".to_owned(), "meridian-core".to_owned()],
            CargoEnvironment::from_host(),
        )
        .expect("Cargo invocation");
        let mut supervisor = CargoBuildSupervisor::try_new().expect("Cargo worker starts");
        supervisor
            .submit(&service, &initial_request, invocation)
            .expect("supervised Cargo request");
        assert!(matches!(
            supervisor.submit(
                &service,
                &initial_request,
                CargoInvocation::new(
                    env!("CARGO_MANIFEST_DIR"),
                    CargoCommand::Check,
                    vec!["-p".to_owned(), "meridian-core".to_owned()],
                    CargoEnvironment::from_host(),
                )
                .expect("duplicate invocation"),
            ),
            Err(BuildError::DuplicateSupervisedCargoOperation(_))
        ));
        let second_request = request(2);
        service
            .submit(second_request.clone())
            .expect("second request queues");
        service
            .transition(second_request.operation_id, BuildPhase::Resolving, 5)
            .expect("second request resolves");
        service
            .transition(second_request.operation_id, BuildPhase::Ready, 15)
            .expect("second request is ready");
        service
            .transition(second_request.operation_id, BuildPhase::Running, 20)
            .expect("second request runs");
        assert!(matches!(
            supervisor.submit(
                &service,
                &second_request,
                CargoInvocation::new(
                    env!("CARGO_MANIFEST_DIR"),
                    CargoCommand::Check,
                    vec!["-p".to_owned(), "meridian-core".to_owned()],
                    CargoEnvironment::from_host(),
                )
                .expect("second invocation"),
            ),
            Err(BuildError::CargoWorkerBusy)
        ));
        let cancellation = supervisor
            .cancel(&mut service, initial_request.operation_id)
            .expect("cancellation records")
            .expect("pending job cancels");
        assert_eq!(cancellation.phase, BuildPhase::CancelRequested);

        let completion = loop {
            if let Some(completion) = supervisor.poll(&mut service) {
                break completion.expect("supervised completion");
            }
            thread::yield_now();
        };
        assert_eq!(completion.status(), CargoRunStatus::Cancelled);
        assert_eq!(
            completion.events().last().map(|event| event.phase),
            Some(BuildPhase::Cancelled)
        );
        assert_eq!(supervisor.pending_len(), 0);
        assert_eq!(
            service
                .service()
                .phase(initial_request.operation_id)
                .expect("durable phase"),
            BuildPhase::Cancelled
        );
    }

    #[test]
    fn cargo_worker_start_failure_maps_to_a_typed_build_error() {
        assert!(matches!(
            CargoBuildSupervisor::from_task_pool(Err(TaskError::WorkerStart)),
            Err(BuildError::CargoWorkerStart)
        ));
    }

    #[test]
    fn cargo_worker_supervisor_persists_a_successful_cargo_check() {
        let directory = TemporaryDirectory::new();
        let request = request(2);
        let mut service = running_durable_service(&directory, request.clone());
        let invocation = CargoInvocation::new(
            env!("CARGO_MANIFEST_DIR"),
            CargoCommand::Check,
            vec!["-p".to_owned(), "meridian-core".to_owned()],
            CargoEnvironment::from_host(),
        )
        .expect("Cargo invocation");
        let mut supervisor = CargoBuildSupervisor::try_new().expect("Cargo worker starts");
        supervisor
            .submit(&service, &request, invocation)
            .expect("supervised Cargo request");
        let completion = loop {
            if let Some(completion) = supervisor.poll(&mut service) {
                break completion.expect("supervised completion");
            }
            thread::yield_now();
        };
        assert_eq!(completion.status(), CargoRunStatus::Succeeded);
        assert_eq!(completion.operation_id(), request.operation_id);
        assert_eq!(supervisor.worker_count().get(), 1);
        assert_eq!(
            service
                .service()
                .phase(request.operation_id)
                .expect("durable phase"),
            BuildPhase::Succeeded
        );
    }

    #[test]
    fn cargo_worker_supervisor_fails_a_contradictory_cargo_result() {
        let directory = TemporaryDirectory::new();
        let request = request(1);
        let mut service = running_durable_service(&directory, request.clone());
        let outcome = CargoRunOutcome {
            status: CargoRunStatus::Succeeded,
            messages: vec![CargoMessage::Finished { success: false }],
            process_diagnostic: None,
        };

        assert!(matches!(
            complete_cargo_worker_outcome(
                &mut service,
                request.operation_id,
                outcome,
                |_, _, _| { Ok(Vec::new()) }
            ),
            Err(BuildError::CargoFinishedOutcomeMismatch {
                reported_success: false,
                phase: BuildPhase::Succeeded,
            })
        ));
        assert_eq!(
            service
                .service()
                .phase(request.operation_id)
                .expect("durable terminal phase"),
            BuildPhase::Failed
        );
    }

    #[test]
    fn request_bound_artifact_reopens_with_its_canonical_graph_manifest() {
        let directory = TemporaryDirectory::new();
        let source = directory.artifact_source();
        fs::write(&source, b"graph manifest artifact").expect("artifact source writes");
        let request = request(1);
        let store = ArtifactStore::new(directory.artifact_root()).expect("artifact store");
        let published = store
            .publish_file_for_request(&request, "cargo-artifact-v1", &source)
            .expect("request-bound artifact publishes");
        let reopened = store
            .publish_file_for_request(&request, "cargo-artifact-v1", &source)
            .expect("matching reference reopens");
        assert_eq!(reopened, published);
        let manifest = reopened
            .build_input_provenance()
            .and_then(BuildInputProvenance::build_graph)
            .expect("persisted graph manifest");
        assert_eq!(manifest.requested_roots(), [request.root_node.id]);
        assert_eq!(manifest.nodes().len(), 1);
    }

    #[test]
    fn artifact_store_publishes_verified_content_and_rejects_reference_conflicts() {
        let directory = TemporaryDirectory::new();
        let source = directory.artifact_source();
        fs::write(&source, b"first published artifact").expect("artifact source writes");
        let node = BuildNode::cargo_build(
            BuildNodeId::new("cargo-build").expect("node ID"),
            "cargo 1.90",
        )
        .expect("build node");
        let mut artifact_identity = identity();
        artifact_identity.root_node_ids = vec![node.id.as_str().to_owned()];
        let build_id = BuildId::derive(&artifact_identity).expect("build ID");
        let store = ArtifactStore::new(directory.artifact_root()).expect("artifact store");

        let published = store
            .publish_file(&build_id, &node, "cargo-artifact-v1", &source)
            .expect("artifact publishes");
        assert_eq!(published.byte_length(), 24);
        assert_eq!(
            published.content_hash(),
            blake3::hash(b"first published artifact")
                .to_hex()
                .to_string()
        );
        let object_path = store
            .object_path(published.content_hash())
            .expect("object path");
        assert_eq!(
            fs::read(&object_path).expect("published object reads"),
            b"first published artifact"
        );
        assert_eq!(
            store
                .publish_file(&build_id, &node, "cargo-artifact-v1", &source)
                .expect("idempotent publish"),
            published
        );

        fs::write(&source, b"conflicting artifact").expect("changed source writes");
        assert!(matches!(
            store.publish_file(&build_id, &node, "cargo-artifact-v1", &source),
            Err(BuildError::ArtifactReferenceConflict)
        ));
        assert_eq!(
            fs::read(&object_path).expect("original object stays immutable"),
            b"first published artifact"
        );
        assert!(fs::read_dir(directory.artifact_root().join("objects"))
            .expect("objects directory reads")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
    }

    #[test]
    fn artifact_store_detects_a_corrupted_existing_object() {
        let directory = TemporaryDirectory::new();
        let source = directory.artifact_source();
        fs::write(&source, b"artifact subject to corruption").expect("artifact source writes");
        let node = BuildNode::cargo_build(
            BuildNodeId::new("cargo-build").expect("node ID"),
            "cargo 1.90",
        )
        .expect("build node");
        let mut artifact_identity = identity();
        artifact_identity.root_node_ids = vec![node.id.as_str().to_owned()];
        let build_id = BuildId::derive(&artifact_identity).expect("build ID");
        let store = ArtifactStore::new(directory.artifact_root()).expect("artifact store");
        let published = store
            .publish_file(&build_id, &node, "cargo-artifact-v1", &source)
            .expect("artifact publishes");
        let object_path = store
            .object_path(published.content_hash())
            .expect("object path");
        fs::write(&object_path, b"corrupt").expect("object corruption writes");
        assert!(matches!(
            store.publish_file(&build_id, &node, "cargo-artifact-v1", &source),
            Err(BuildError::ArtifactObjectHashMismatch)
        ));
    }

    #[test]
    fn artifact_store_qualifies_one_listed_executable_within_the_output_root() {
        let directory = TemporaryDirectory::new();
        let cargo_output_root = directory.path.join("cargo-target");
        let executable = cargo_output_root.join("debug").join("example-bin");
        fs::create_dir_all(executable.parent().expect("executable parent"))
            .expect("output root creates");
        fs::write(&executable, b"Cargo-reported executable").expect("executable writes");
        let node = BuildNode::cargo_build(
            BuildNodeId::new("cargo-build").expect("node ID"),
            "cargo 1.90",
        )
        .expect("build node");
        let mut artifact_identity = identity();
        artifact_identity.root_node_ids = vec![node.id.as_str().to_owned()];
        let build_id = BuildId::derive(&artifact_identity).expect("build ID");
        let store = ArtifactStore::new(directory.artifact_root()).expect("artifact store");
        let executable_text = executable.to_string_lossy().into_owned();
        let artifact = CargoArtifact {
            package_id: "path+file:///workspace#example@0.1.0".to_owned(),
            target_name: "example-bin".to_owned(),
            filenames: vec![executable_text.clone()],
            executable: Some(executable_text),
        };

        let outside = directory.path.join("outside-bin");
        fs::write(&outside, b"outside executable").expect("outside executable writes");
        let outside_text = outside.to_string_lossy().into_owned();
        assert!(matches!(
            store.publish_cargo_executable(
                &build_id,
                &node,
                &CargoArtifact {
                    package_id: artifact.package_id.clone(),
                    target_name: artifact.target_name.clone(),
                    filenames: vec![outside_text.clone()],
                    executable: Some(outside_text),
                },
                &cargo_output_root,
            ),
            Err(BuildError::CargoArtifactExecutableOutsideOutputRoot)
        ));
        assert!(matches!(
            store.publish_cargo_executable(
                &build_id,
                &node,
                &CargoArtifact {
                    package_id: artifact.package_id.clone(),
                    target_name: artifact.target_name.clone(),
                    filenames: Vec::new(),
                    executable: artifact.executable.clone(),
                },
                &cargo_output_root,
            ),
            Err(BuildError::CargoArtifactExecutableNotListed)
        ));

        let published = store
            .publish_cargo_executable(&build_id, &node, &artifact, &cargo_output_root)
            .expect("listed executable publishes");
        assert_eq!(published.schema(), "cargo-executable-v1");
        let provenance = published.cargo_provenance().expect("Cargo provenance");
        assert_eq!(provenance.package_id(), artifact.package_id);
        assert_eq!(provenance.target_name(), artifact.target_name);
        assert_eq!(
            fs::read(
                store
                    .object_path(published.content_hash())
                    .expect("object path"),
            )
            .expect("published executable reads"),
            b"Cargo-reported executable"
        );
    }

    #[test]
    fn artifact_store_reuses_matching_legacy_reference_without_upgrading_it() {
        let directory = TemporaryDirectory::new();
        let cargo_output_root = directory.path.join("cargo-target");
        let executable = cargo_output_root.join("debug").join("example-bin");
        fs::create_dir_all(executable.parent().expect("executable parent"))
            .expect("output root creates");
        fs::write(&executable, b"legacy Cargo executable").expect("executable writes");
        let node = BuildNode::cargo_build(
            BuildNodeId::new("cargo-build").expect("node ID"),
            "cargo 1.90",
        )
        .expect("build node");
        let mut artifact_identity = identity();
        artifact_identity.root_node_ids = vec![node.id.as_str().to_owned()];
        let build_id = BuildId::derive(&artifact_identity).expect("build ID");
        let store = ArtifactStore::new(directory.artifact_root()).expect("artifact store");
        let legacy = store
            .publish_file(&build_id, &node, "cargo-executable-v1", &executable)
            .expect("legacy reference publishes");
        assert!(legacy.cargo_provenance().is_none());
        let executable_text = executable.to_string_lossy().into_owned();
        let artifact = CargoArtifact {
            package_id: "path+file:///workspace#example@0.1.0".to_owned(),
            target_name: "example-bin".to_owned(),
            filenames: vec![executable_text.clone()],
            executable: Some(executable_text),
        };

        assert_eq!(
            store
                .publish_cargo_executable(&build_id, &node, &artifact, &cargo_output_root)
                .expect("matching legacy reference reuses"),
            legacy
        );
    }

    #[test]
    fn verified_artifact_events_require_the_running_request_identity() {
        let directory = TemporaryDirectory::new();
        let source = directory.artifact_source();
        fs::write(&source, b"verified event artifact").expect("artifact source writes");
        let request = request(1);
        let build_id = request.build_id.clone();
        let node = request.root_node.clone();
        let store = ArtifactStore::new(directory.artifact_root()).expect("artifact store");
        let published = store
            .publish_file_for_request(&request, "cargo-artifact-v1", &source)
            .expect("artifact publishes");

        let mut service = BuildService::default();
        service.submit(request).expect("queued");
        service
            .transition(OperationId::new(1), BuildPhase::Resolving, 5)
            .expect("resolving");
        service
            .transition(OperationId::new(1), BuildPhase::Ready, 10)
            .expect("ready");
        service
            .transition(OperationId::new(1), BuildPhase::Running, 20)
            .expect("running");

        let event = service
            .record_published_artifact(OperationId::new(1), published.clone())
            .expect("publication event");
        assert_eq!(
            event.artifact_hash,
            Some(published.content_hash().to_owned())
        );
        let mut external = event.clone();
        external.sequence = external.sequence.saturating_add(1);
        assert_external_artifact_provenance_is_rejected(&mut service, &external);
        service
            .accept_external_event(&external)
            .expect("matching external publication event");
        let mut tampered = external;
        tampered.sequence = tampered.sequence.saturating_add(1);
        tampered.artifact_hash = Some("0".repeat(64));
        assert!(matches!(
            service.accept_external_event(&tampered),
            Err(BuildError::MismatchedArtifactEvent)
        ));
        assert!(matches!(
            event.payload,
            BuildEventPayload::Artifact(ref event_publication) if event_publication == &published
        ));

        let legacy_store = ArtifactStore::new(directory.path.join("legacy-artifacts"))
            .expect("legacy artifact store");
        let legacy = legacy_store
            .publish_file(&build_id, &node, "cargo-artifact-v1", &source)
            .expect("legacy artifact publishes");
        assert!(matches!(
            service.record_published_artifact(OperationId::new(1), legacy),
            Err(BuildError::MissingBuildInputProvenance)
        ));

        let mut mismatched_provenance = published.clone();
        let mut different_identity = identity();
        different_identity.source_checkpoint = "different-provenance".to_owned();
        mismatched_provenance.build_input_provenance = Some(Box::new(
            BuildInputProvenance::from_identity(&different_identity)
                .expect("different input provenance"),
        ));
        assert!(matches!(
            service.record_published_artifact(OperationId::new(1), mismatched_provenance),
            Err(BuildError::MismatchedBuildInputProvenance)
        ));

        let mut other_identity = identity();
        other_identity.source_checkpoint = "different-checkpoint".to_owned();
        let other_build_id = BuildId::derive(&other_identity).expect("different build ID");
        let wrong_build = store
            .publish_file(&other_build_id, &node, "cargo-artifact-v1", &source)
            .expect("different build publication");
        assert!(matches!(
            service.record_published_artifact(OperationId::new(1), wrong_build),
            Err(BuildError::MismatchedEventIdentity)
        ));

        let other_node = BuildNode::cargo_build(
            BuildNodeId::new("different-node").expect("different node ID"),
            "cargo 1.90",
        )
        .expect("different build node");
        let wrong_node = store
            .publish_file(&build_id, &other_node, "cargo-artifact-v1", &source)
            .expect("different node publication");
        assert!(matches!(
            service.record_published_artifact(OperationId::new(1), wrong_node),
            Err(BuildError::MismatchedNodeId)
        ));
    }

    #[test]
    fn durable_service_persists_recovery_before_exposing_worker_lost() {
        let directory = TemporaryDirectory::new();
        let state_path = directory.state_path();
        let mut service =
            DurableBuildService::open(BuildServiceStore::new(&state_path).expect("state path"))
                .expect("empty durable service")
                .service;
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
        drop(service);

        let recovered =
            DurableBuildService::open(BuildServiceStore::new(&state_path).expect("state path"))
                .expect("recovery");
        assert_eq!(recovered.recovery_events.len(), 1);
        assert_eq!(recovered.recovery_events[0].phase, BuildPhase::WorkerLost);
        assert_eq!(
            recovered
                .service
                .service()
                .phase(OperationId::new(1))
                .expect("recovered phase"),
            BuildPhase::WorkerLost
        );
        drop(recovered);

        let reopened =
            DurableBuildService::open(BuildServiceStore::new(&state_path).expect("state path"))
                .expect("reopen");
        assert!(reopened.recovery_events.is_empty());
        assert!(fs::read_dir(&directory.path)
            .expect("state directory reads")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
    }

    #[test]
    fn durable_store_rejects_missing_malformed_oversized_and_non_regular_state() {
        let directory = TemporaryDirectory::new();
        let state_path = directory.state_path();
        let store = BuildServiceStore::new(&state_path).expect("state path");
        assert!(matches!(store.load(), Err(BuildError::SnapshotMissing)));

        fs::write(&state_path, "not-json").expect("malformed state writes");
        assert!(matches!(
            store.load(),
            Err(BuildError::MalformedSnapshot(_))
        ));

        fs::write(
            &state_path,
            "x".repeat(MAX_BUILD_SNAPSHOT_BYTES.saturating_add(1)),
        )
        .expect("oversized state writes");
        assert!(matches!(store.load(), Err(BuildError::SnapshotTooLarge(_))));

        let directory_path = directory.path.join("state-directory");
        fs::create_dir(&directory_path).expect("state directory creates");
        let directory_store = BuildServiceStore::new(directory_path).expect("directory path");
        assert!(matches!(
            directory_store.exists(),
            Err(BuildError::SnapshotPathNotRegular)
        ));
    }
}
