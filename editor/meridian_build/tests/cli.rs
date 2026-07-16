use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use meridian_build::{
    BuildGraph, BuildIdentityInput, BuildNode, BuildNodeId, BuildPhase, BuildRequest,
    BuildServiceStore, DurableBuildService,
};
use meridian_core::{OperationId, TraceId};

static NEXT_TEMPORARY_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn default_state_is_announced_before_work_and_removed_after_success() {
    let directory = TemporaryDirectory::new();
    let output = run_build(&directory, "default-state-cli", None);

    assert!(output.status.success(), "{}", render_output(&output));
    let stdout = String::from_utf8(output.stdout).expect("CLI stdout is UTF-8");
    let state_path = announced_state_path(&stdout);
    assert!(
        stdout
            .find("Meridian build state:")
            .expect("state announcement")
            < stdout.find("phase Queued").expect("first build event"),
        "state path must be printed before the first build event: {stdout}"
    );
    assert!(
        !state_path.exists(),
        "successful default-owned state must be removed: {}",
        state_path.display()
    );
}

#[test]
fn explicit_state_reports_worker_loss_uses_a_fresh_operation_and_is_retained() {
    let directory = TemporaryDirectory::new();
    let state_path = directory.path.join("caller-owned-state.json");
    persist_interrupted_operation(&state_path);

    let output = run_build(&directory, "explicit-state-cli", Some(&state_path));

    assert!(output.status.success(), "{}", render_output(&output));
    let stdout = String::from_utf8(output.stdout).expect("CLI stdout is UTF-8");
    assert!(
        stdout.contains("operation 1") && stdout.contains("phase WorkerLost"),
        "recovered WorkerLost event was not reported: {stdout}"
    );
    assert!(
        stdout.contains("operation 2") && stdout.contains("phase Queued"),
        "retry did not receive a fresh operation ID: {stdout}"
    );
    assert_eq!(announced_state_path(&stdout), state_path);
    assert!(
        state_path.exists(),
        "successful explicit caller-owned state must be retained"
    );
}

fn run_build(
    directory: &TemporaryDirectory,
    source_checkpoint: &str,
    state_path: Option<&Path>,
) -> std::process::Output {
    let workspace = workspace_root();
    let mut command = Command::new(env!("CARGO_BIN_EXE_meridian-build"));
    command
        .args(["--cargo-check", "--workspace"])
        .arg(&workspace)
        .args(["--source-checkpoint", source_checkpoint])
        .arg("--toolchain")
        .arg("test")
        .arg("--target")
        .arg("host");
    if let Some(state_path) = state_path {
        command.arg("--state").arg(state_path);
    }
    command
        .arg("--")
        .args(["-p", "meridian-core", "--target-dir"])
        .arg(directory.path.join("cargo-target"))
        .output()
        .expect("run meridian-build CLI")
}

fn persist_interrupted_operation(state_path: &Path) {
    let root = BuildNode::cargo_check(BuildNodeId::new("cargo-check").expect("node ID"), "test")
        .expect("node");
    let graph = BuildGraph::new(vec![root.clone()], vec![root.id.clone()]).expect("graph");
    let identity = BuildIdentityInput {
        source_checkpoint: "interrupted".to_owned(),
        resolved_profile: "cargo-check".to_owned(),
        cargo_metadata_and_lock: "interrupted-metadata-and-lock".to_owned(),
        build_graph_contract: graph.contract_hash(),
        command_arguments: Vec::new(),
        toolchain_version: "test".to_owned(),
        target_and_capabilities: "host".to_owned(),
        environment_allowlist: BTreeMap::new(),
        root_node_ids: vec![root.id.as_str().to_owned()],
    };
    let request = BuildRequest::new_with_graph(
        &identity,
        OperationId::new(1),
        TraceId::new(1),
        root,
        &graph,
    )
    .expect("request");
    let mut service = DurableBuildService::open(
        BuildServiceStore::new(state_path.to_path_buf()).expect("state store"),
    )
    .expect("open durable state")
    .service;
    service.submit(request).expect("queue operation");
    service
        .transition(OperationId::new(1), BuildPhase::Resolving, 5)
        .expect("resolve operation");
    service
        .transition(OperationId::new(1), BuildPhase::Ready, 15)
        .expect("ready operation");
    service
        .transition(OperationId::new(1), BuildPhase::Running, 20)
        .expect("run operation");
}

fn announced_state_path(stdout: &str) -> PathBuf {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("Meridian build state: "))
        .map(PathBuf::from)
        .expect("CLI announced a state path")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn render_output(output: &std::process::Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn new() -> Self {
        let identifier = NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "meridian-build-cli-test-{}-{identifier}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temporary directory");
        Self { path }
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
