//! Exercises the verified artifact-event boundary without invoking Cargo.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use meridian_build::{
    ArtifactStore, BuildEventPayload, BuildIdentityInput, BuildNode, BuildNodeId, BuildPhase,
    BuildRequest, BuildService,
};
use meridian_core::{OperationId, TraceId};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = TemporaryDirectory::new()?;
    let source = directory.path().join("artifact-input.bin");
    fs::write(&source, b"Meridian verified artifact event smoke")?;

    let node = BuildNode::cargo_build(BuildNodeId::new("cargo-build")?, "cargo-example")?;
    let identity = BuildIdentityInput {
        source_checkpoint: "artifact-event-smoke".to_owned(),
        resolved_profile: "cargo-build".to_owned(),
        cargo_metadata_and_lock: "smoke-metadata-and-lock".to_owned(),
        toolchain_version: "cargo-example".to_owned(),
        target_and_capabilities: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        environment_allowlist: BTreeMap::new(),
        root_node_ids: vec![node.id.as_str().to_owned()],
    };
    let request = BuildRequest::new(&identity, OperationId::new(1), TraceId::new(1), node)?;
    let store = ArtifactStore::new(directory.path().join("artifacts"))?;
    let publication = store.publish_file(
        &request.build_id,
        &request.root_node,
        "meridian-artifact-event-smoke-v1",
        &source,
    )?;

    let mut service = BuildService::default();
    service.submit(request)?;
    service.transition(OperationId::new(1), BuildPhase::Resolving, 5)?;
    service.transition(OperationId::new(1), BuildPhase::Ready, 15)?;
    service.transition(OperationId::new(1), BuildPhase::Running, 20)?;
    let event = service.record_published_artifact(OperationId::new(1), publication.clone())?;

    if event.artifact_hash.as_deref() != Some(publication.content_hash())
        || !matches!(&event.payload, BuildEventPayload::Artifact(value) if value == &publication)
    {
        return Err("verified artifact event did not preserve its publication".into());
    }
    println!(
        "Meridian verified artifact event smoke passed: {} {}",
        event.build_id,
        publication.content_hash()
    );
    Ok(())
}

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn new() -> Result<Self, Box<dyn Error>> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "meridian-artifact-event-smoke-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
