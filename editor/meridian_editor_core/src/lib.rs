//! Source-authoritative Creator Editor sessions, commands, history, and recovery.
//!
//! This crate deliberately contains no UI, platform, renderer, Cargo, or game
//! types. Its deterministic command stream is the only mutation boundary for
//! Creator Alpha project documents.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use meridian_core::StableId;
use meridian_save::{SaveConfig, SaveError, SaveStore};
use serde::{Deserialize, Serialize};

/// Creator Alpha's public project-source schema.
pub const PROJECT_SCHEMA: &str = "meridian.creator-project/v1";
const RECOVERY_SCHEMA: &str = "meridian.editor-recovery/v1";
const CHECKPOINT_INTERVAL: usize = 8;
static SOURCE_TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A deterministic world-space translation in millimetres.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Translation {
    /// East/west displacement in millimetres.
    pub x_mm: i64,
    /// Vertical displacement in millimetres.
    pub y_mm: i64,
    /// North/south displacement in millimetres.
    pub z_mm: i64,
}

/// A registered source imported by an authoritative DAT boundary.
///
/// The editor retains source identity and provenance only; it never replaces the
/// original source path with a render preview or opaque derived asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedSource {
    /// Persistent identity supplied by the importing source authority.
    pub id: StableId,
    /// Human-readable generic name for the source.
    pub label: String,
    /// Project-relative public source path.
    pub source_path: String,
    /// Declared immutable content identity supplied by import.
    pub source_hash: String,
}

/// One editable object placement in the public generic Creator Alpha world.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPlacement {
    /// Persistent object identity.
    pub id: StableId,
    /// Imported source identity. It must exist in [`ProjectDocument::sources`].
    pub source_id: StableId,
    /// User-facing object label.
    pub label: String,
    /// Editable source transform, never a renderer-owned transform handle.
    pub translation: Translation,
}

/// Versioned, human-readable source owned by the Creator Editor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDocument {
    /// Exact source schema identifier.
    pub schema: String,
    /// Stable project identity.
    pub id: StableId,
    /// Incremented for every committed source mutation.
    pub generation: u64,
    /// Source imports keyed by stable ID.
    pub sources: BTreeMap<StableId, ImportedSource>,
    /// Editable world placements keyed by stable ID.
    pub placements: BTreeMap<StableId, WorldPlacement>,
}

impl ProjectDocument {
    /// Creates an empty, valid project document.
    #[must_use]
    pub fn new(id: StableId) -> Self {
        Self {
            schema: PROJECT_SCHEMA.to_owned(),
            id,
            generation: 0,
            sources: BTreeMap::new(),
            placements: BTreeMap::new(),
        }
    }

    /// Verifies schema, stable identities, and source ownership references.
    ///
    /// # Errors
    ///
    /// Returns a typed validation failure without mutating the document.
    pub fn validate(&self) -> Result<(), EditorError> {
        if self.schema != PROJECT_SCHEMA {
            return Err(EditorError::UnsupportedProjectSchema(self.schema.clone()));
        }
        for (id, source) in &self.sources {
            if *id != source.id {
                return Err(EditorError::SourceKeyMismatch(*id));
            }
            if self.placements.contains_key(id) {
                return Err(EditorError::StableIdentityCollision(*id));
            }
            if source.label.trim().is_empty()
                || source.source_path.trim().is_empty()
                || source.source_hash.trim().is_empty()
            {
                return Err(EditorError::InvalidImportedSource(*id));
            }
        }
        for (id, placement) in &self.placements {
            if *id != placement.id {
                return Err(EditorError::PlacementKeyMismatch(*id));
            }
            if placement.label.trim().is_empty() {
                return Err(EditorError::InvalidPlacement(*id));
            }
            if !self.sources.contains_key(&placement.source_id) {
                return Err(EditorError::UnknownSource(placement.source_id));
            }
        }
        Ok(())
    }

    /// Reads one regular, bounded Creator project source file.
    ///
    /// # Errors
    ///
    /// Rejects symlinks, directories, oversized inputs, malformed JSON, and
    /// invalid source before a session can observe it.
    pub fn read_source(path: impl AsRef<Path>) -> Result<Self, EditorError> {
        let path = path.as_ref();
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| EditorError::SourceRead(format!("{}: {error}", path.display())))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(EditorError::InvalidSourcePath(path.display().to_string()));
        }
        let maximum = SaveConfig::default().max_payload_bytes;
        let size = usize::try_from(metadata.len()).map_err(|_| EditorError::SourceTooLarge {
            size: usize::MAX,
            maximum,
        })?;
        if size > maximum {
            return Err(EditorError::SourceTooLarge { size, maximum });
        }
        let bytes = fs::read(path)
            .map_err(|error| EditorError::SourceRead(format!("{}: {error}", path.display())))?;
        let document: Self = serde_json::from_slice(&bytes)
            .map_err(|error| EditorError::SourceDecoding(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }

    /// Emits canonical, pretty Creator project source JSON.
    ///
    /// # Errors
    ///
    /// Returns validation or encoding errors without changing source.
    pub fn canonical_json(&self) -> Result<String, EditorError> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| EditorError::SourceEncoding(error.to_string()))
    }
}

/// A generation-checked selection stored independently from project source.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// Document generation at which this selection was made.
    pub generation: u64,
    /// Selected source or placement stable IDs.
    pub ids: BTreeSet<StableId>,
}

impl Selection {
    /// Replaces the selection after confirming every identity is selectable.
    ///
    /// # Errors
    ///
    /// Returns an error for IDs absent from the source document.
    pub fn replace(
        &mut self,
        document: &ProjectDocument,
        ids: impl IntoIterator<Item = StableId>,
    ) -> Result<(), EditorError> {
        let ids = ids.into_iter().collect::<BTreeSet<_>>();
        for id in &ids {
            if !document.sources.contains_key(id) && !document.placements.contains_key(id) {
                return Err(EditorError::UnknownSelection(*id));
            }
        }
        self.generation = document.generation;
        self.ids = ids;
        Ok(())
    }

    /// Rejects a selection created against an older document generation.
    ///
    /// # Errors
    ///
    /// Returns [`EditorError::StaleSelection`] before stale IDs can be used.
    pub fn validate(&self, document: &ProjectDocument) -> Result<(), EditorError> {
        if self.generation != document.generation {
            return Err(EditorError::StaleSelection {
                selection_generation: self.generation,
                document_generation: document.generation,
            });
        }
        Ok(())
    }
}

/// Typed source mutations available in the MS-03 Editor Alpha boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EditorCommand {
    /// Registers a source already accepted by the import authority.
    RegisterImportedSource(ImportedSource),
    /// Replaces the retained metadata for an already registered source after a
    /// fresh DAT-owned import. The stable source identity cannot change.
    UpdateImportedSource(ImportedSource),
    /// Removes a source which has no editable placement references.
    RemoveImportedSource {
        /// Source to remove.
        source_id: StableId,
    },
    /// Adds an editable world object referencing a registered source.
    PlaceObject(WorldPlacement),
    /// Changes a source-owned world placement transform.
    SetPlacementTranslation {
        /// Placement to edit.
        placement_id: StableId,
        /// New source translation.
        translation: Translation,
    },
    /// Removes an editable world placement.
    RemovePlacement {
        /// Placement to remove.
        placement_id: StableId,
    },
}

/// Required audit metadata carried with every committed command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandMetadata {
    /// Human-readable action label for history and assistive UI.
    pub label: String,
    /// Stable identities affected by this mutation.
    pub affected_ids: BTreeSet<StableId>,
    /// Caller-provided, non-secret audit actor label.
    pub actor: String,
}

impl CommandMetadata {
    /// Builds metadata for a bounded local Creator Alpha command.
    #[must_use]
    pub fn local(
        label: impl Into<String>,
        affected_ids: impl IntoIterator<Item = StableId>,
    ) -> Self {
        Self {
            label: label.into(),
            affected_ids: affected_ids.into_iter().collect(),
            actor: "local-creator".to_owned(),
        }
    }
}

/// A validated mutation request which can be previewed before commitment.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorTransaction {
    /// Typed source mutation.
    pub command: EditorCommand,
    /// User-visible audit and affected-ID data.
    pub metadata: CommandMetadata,
}

/// A no-mutation preview result for UI, CLI, and agent callers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandPreview {
    /// IDs that will be changed if the transaction commits.
    pub affected_ids: BTreeSet<StableId>,
    /// Validated action label.
    pub label: String,
}

/// An immutable source checkpoint retained at bounded history intervals.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    /// Document generation after the checkpointed transaction.
    pub generation: u64,
    /// Complete source snapshot for deterministic recovery/rebuild.
    pub document: ProjectDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HistoryEntry {
    transaction: EditorTransaction,
    inverse: EditorCommand,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PlaySession {
    base_document: ProjectDocument,
    runtime_document: ProjectDocument,
}

/// An explicit source difference emitted before a Play session can apply back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayChange {
    /// Placement whose source translation would change.
    pub placement_id: StableId,
    /// Translation before Play.
    pub before: Translation,
    /// Translation produced by the isolated Play fork.
    pub after: Translation,
}

/// Editor-owned session state, history, checkpoints, and optional Play fork.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSession {
    document: ProjectDocument,
    selection: Selection,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    checkpoints: Vec<Checkpoint>,
    play: Option<PlaySession>,
}

impl EditorSession {
    /// Opens a new session over a valid source document.
    ///
    /// # Errors
    ///
    /// Returns validation errors without creating a partially valid session.
    pub fn open(document: ProjectDocument) -> Result<Self, EditorError> {
        document.validate()?;
        Ok(Self {
            selection: Selection {
                generation: document.generation,
                ids: BTreeSet::new(),
            },
            document,
            undo: Vec::new(),
            redo: Vec::new(),
            checkpoints: Vec::new(),
            play: None,
        })
    }

    /// Returns the source-authoritative project document.
    #[must_use]
    pub fn document(&self) -> &ProjectDocument {
        &self.document
    }

    /// Returns the independent, generation-checked local selection.
    #[must_use]
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Returns retained immutable history checkpoints.
    #[must_use]
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Returns the number of committed source transactions available to undo.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// Returns the number of previously undone transactions available to redo.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    /// Returns whether an isolated Play fork is currently active.
    #[must_use]
    pub fn play_active(&self) -> bool {
        self.play.is_some()
    }

    /// Returns the number of explicit source changes currently pending in Play.
    ///
    /// A missing Play fork has no pending changes; this query never mutates the
    /// source-authoritative session.
    #[must_use]
    pub fn pending_play_change_count(&self) -> usize {
        self.play_changes().map_or(0, |changes| changes.len())
    }

    /// Validates a transaction without changing source or history.
    ///
    /// # Errors
    ///
    /// Returns the same typed validation failure that `commit` would return.
    pub fn preview(&self, transaction: &EditorTransaction) -> Result<CommandPreview, EditorError> {
        validate_transaction(&self.document, transaction)?;
        Ok(CommandPreview {
            affected_ids: transaction.metadata.affected_ids.clone(),
            label: transaction.metadata.label.clone(),
        })
    }

    /// Commits one validated transaction and retains its inverse for undo.
    ///
    /// # Errors
    ///
    /// Returns validation errors before source, history, or checkpoints mutate.
    pub fn commit(&mut self, transaction: EditorTransaction) -> Result<(), EditorError> {
        let before = self.clone();
        let result = (|| {
            if self.play.is_some() {
                return Err(EditorError::PlaySessionActive);
            }
            validate_transaction(&self.document, &transaction)?;
            let generation = self
                .document
                .generation
                .checked_add(1)
                .ok_or(EditorError::GenerationExhausted)?;
            let inverse = apply_command(&mut self.document, &transaction.command)?;
            self.document.generation = generation;
            self.undo.push(HistoryEntry {
                transaction,
                inverse,
            });
            self.redo.clear();
            self.record_checkpoint();
            Ok(())
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Undoes the latest committed transaction through its typed inverse.
    ///
    /// # Errors
    ///
    /// Returns [`EditorError::NothingToUndo`] when history is empty.
    pub fn undo(&mut self) -> Result<(), EditorError> {
        let before = self.clone();
        let result = (|| {
            if self.play.is_some() {
                return Err(EditorError::PlaySessionActive);
            }
            let generation = self
                .document
                .generation
                .checked_add(1)
                .ok_or(EditorError::GenerationExhausted)?;
            let entry = self.undo.pop().ok_or(EditorError::NothingToUndo)?;
            apply_command(&mut self.document, &entry.inverse)?;
            self.document.generation = generation;
            self.redo.push(entry);
            self.record_checkpoint();
            Ok(())
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Replays the latest undone transaction after revalidating it.
    ///
    /// # Errors
    ///
    /// Returns [`EditorError::NothingToRedo`] when redo history is empty.
    pub fn redo(&mut self) -> Result<(), EditorError> {
        let before = self.clone();
        let result = (|| {
            if self.play.is_some() {
                return Err(EditorError::PlaySessionActive);
            }
            let generation = self
                .document
                .generation
                .checked_add(1)
                .ok_or(EditorError::GenerationExhausted)?;
            let entry = self.redo.pop().ok_or(EditorError::NothingToRedo)?;
            validate_transaction(&self.document, &entry.transaction)?;
            apply_command(&mut self.document, &entry.transaction.command)?;
            self.document.generation = generation;
            self.undo.push(entry);
            self.record_checkpoint();
            Ok(())
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Starts an isolated runtime fork. Source stays immutable until apply-back.
    ///
    /// # Errors
    ///
    /// Returns an error if a Play session is already active.
    pub fn start_play(&mut self) -> Result<(), EditorError> {
        if self.play.is_some() {
            return Err(EditorError::PlaySessionActive);
        }
        self.play = Some(PlaySession {
            base_document: self.document.clone(),
            runtime_document: self.document.clone(),
        });
        Ok(())
    }

    /// Changes one placement inside the runtime fork only.
    ///
    /// # Errors
    ///
    /// Returns an error without changing source when no matching placement exists.
    pub fn set_play_translation(
        &mut self,
        placement_id: StableId,
        translation: Translation,
    ) -> Result<(), EditorError> {
        let play = self.play.as_mut().ok_or(EditorError::NoPlaySession)?;
        let placement = play
            .runtime_document
            .placements
            .get_mut(&placement_id)
            .ok_or(EditorError::UnknownPlacement(placement_id))?;
        placement.translation = translation;
        Ok(())
    }

    /// Returns the explicit, stable-ID Play-to-source change set.
    ///
    /// # Errors
    ///
    /// Returns an error if Play is inactive.
    pub fn play_changes(&self) -> Result<Vec<PlayChange>, EditorError> {
        let play = self.play.as_ref().ok_or(EditorError::NoPlaySession)?;
        Ok(play
            .runtime_document
            .placements
            .iter()
            .filter_map(|(id, runtime)| {
                let source = play.base_document.placements.get(id)?;
                (source.translation != runtime.translation).then_some(PlayChange {
                    placement_id: *id,
                    before: source.translation,
                    after: runtime.translation,
                })
            })
            .collect())
    }

    /// Applies the explicit Play diff through normal source transactions.
    ///
    /// # Errors
    ///
    /// Returns transaction errors before emitting a partial apply-back.
    pub fn apply_play(&mut self) -> Result<Vec<PlayChange>, EditorError> {
        let changes = self.play_changes()?;
        let before_apply = self.clone();
        self.play = None;
        for change in &changes {
            let result = self.commit(EditorTransaction {
                command: EditorCommand::SetPlacementTranslation {
                    placement_id: change.placement_id,
                    translation: change.after,
                },
                metadata: CommandMetadata::local("Apply Play change", [change.placement_id]),
            });
            if let Err(error) = result {
                *self = before_apply;
                return Err(error);
            }
        }
        Ok(changes)
    }

    /// Stops Play and discards the isolated runtime fork.
    ///
    /// # Errors
    ///
    /// Returns an error if Play is inactive.
    pub fn discard_play(&mut self) -> Result<(), EditorError> {
        self.play.take().ok_or(EditorError::NoPlaySession)?;
        Ok(())
    }

    /// Selects source or placement IDs against the current document generation.
    ///
    /// # Errors
    ///
    /// Returns an error without changing selection if an ID is unavailable.
    pub fn select(&mut self, ids: impl IntoIterator<Item = StableId>) -> Result<(), EditorError> {
        self.selection.replace(&self.document, ids)
    }

    fn record_checkpoint(&mut self) {
        if self.undo.len().is_multiple_of(CHECKPOINT_INTERVAL) {
            self.checkpoints.push(Checkpoint {
                generation: self.document.generation,
                document: self.document.clone(),
            });
        }
    }
}

/// Durable source and recovery coordinator for one Creator project.
///
/// The project JSON is authoritative. Recovery may restore only safe local
/// state after its recovered source exactly matches the accepted project JSON.
pub struct ProjectStore {
    source_path: PathBuf,
    recovery: EditorRecoveryStore,
}

/// How a source-authoritative project open treated local recovery state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectRecoveryStatus {
    /// No recovery sidecar was present.
    None,
    /// Recovery matched the accepted source document and resumed a fresh source
    /// session. Unproven sidecar history is deliberately not restored.
    Restored,
    /// Recovery was invalid or disagreed with source and was safely ignored.
    Ignored,
}

/// A source-authoritative project session returned by [`ProjectStore::open`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenProject {
    /// The validated source session, optionally with matching safe recovery
    /// state. Sidecar undo/redo history never becomes source authority.
    pub session: EditorSession,
    /// Recovery disposition that callers should expose through diagnostics.
    pub recovery: ProjectRecoveryStatus,
}

impl ProjectStore {
    /// Binds one project source path and one separate recovery sidecar.
    #[must_use]
    pub fn new(source_path: impl Into<PathBuf>, recovery_path: impl AsRef<Path>) -> Self {
        Self {
            source_path: source_path.into(),
            recovery: EditorRecoveryStore::new(recovery_path),
        }
    }

    /// Returns the authoritative project source path.
    #[must_use]
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// Returns the local recovery sidecar path.
    #[must_use]
    pub fn recovery_path(&self) -> &Path {
        self.recovery.path()
    }

    /// Writes a newly created source document and its initial recovery sidecar.
    ///
    /// # Errors
    ///
    /// Leaves no partially accepted in-memory project state when source or
    /// recovery persistence fails.
    pub fn create(&self, document: ProjectDocument) -> Result<EditorSession, EditorError> {
        if self.source_path.exists() {
            return Err(EditorError::SourceWrite(
                "refusing to replace an existing project source during creation".to_owned(),
            ));
        }
        let session = EditorSession::open(document)?;
        self.write_document(session.document())?;
        if let Err(error) = self.recovery.save(&session) {
            if let Err(rollback) = remove_source_file(&self.source_path) {
                return Err(EditorError::SourceRollbackFailed(rollback));
            }
            return Err(error);
        }
        Ok(session)
    }

    /// Opens accepted source and restores only matching safe recovery state.
    ///
    /// # Errors
    ///
    /// Source errors are returned; recovery errors are isolated as an ignored
    /// sidecar so they cannot make authoritative source unavailable.
    pub fn open(&self) -> Result<OpenProject, EditorError> {
        let document = ProjectDocument::read_source(&self.source_path)?;
        let session = EditorSession::open(document.clone())?;
        if !self.recovery.path().exists() {
            return Ok(OpenProject {
                session,
                recovery: ProjectRecoveryStatus::None,
            });
        }
        match self.recovery.load() {
            Ok(recovered) if recovered.document() == &document => Ok(OpenProject {
                session: recovered,
                recovery: ProjectRecoveryStatus::Restored,
            }),
            Ok(_) | Err(_) => Ok(OpenProject {
                session,
                recovery: ProjectRecoveryStatus::Ignored,
            }),
        }
    }

    /// Runs one typed session mutation, then atomically persists source and
    /// recovery. Any write failure restores the accepted in-memory session; a
    /// recovery failure also restores the prior source document before return.
    ///
    /// # Errors
    ///
    /// Mutation, source-write, recovery-write, and rollback failures remain
    /// typed and never publish a partial accepted session.
    pub fn mutate<T, F>(&self, session: &mut EditorSession, mutation: F) -> Result<T, EditorError>
    where
        F: FnOnce(&mut EditorSession) -> Result<T, EditorError>,
    {
        let before = session.clone();
        let outcome = match mutation(session) {
            Ok(outcome) => outcome,
            Err(error) => {
                *session = before;
                return Err(error);
            }
        };
        if let Err(error) = self.write_document(session.document()) {
            *session = before;
            return Err(error);
        }
        if let Err(error) = self.recovery.save(session) {
            let rollback = self.write_document(before.document());
            *session = before;
            if let Err(rollback) = rollback {
                return Err(EditorError::SourceRollbackFailed(rollback.to_string()));
            }
            return Err(error);
        }
        Ok(outcome)
    }

    /// Persists a Play-only mutation in recovery without changing source.
    ///
    /// # Errors
    ///
    /// Restores the in-memory Play state if recovery cannot be updated.
    pub fn mutate_play<T, F>(
        &self,
        session: &mut EditorSession,
        mutation: F,
    ) -> Result<T, EditorError>
    where
        F: FnOnce(&mut EditorSession) -> Result<T, EditorError>,
    {
        let before = session.clone();
        let outcome = match mutation(session) {
            Ok(outcome) => outcome,
            Err(error) => {
                *session = before;
                return Err(error);
            }
        };
        if let Err(error) = self.recovery.save(session) {
            *session = before;
            return Err(error);
        }
        Ok(outcome)
    }

    fn write_document(&self, document: &ProjectDocument) -> Result<(), EditorError> {
        write_atomic_source(&self.source_path, document.canonical_json()?.as_bytes())
    }
}

fn write_atomic_source(path: &Path, bytes: &[u8]) -> Result<(), EditorError> {
    let maximum = SaveConfig::default().max_payload_bytes;
    if bytes.len() > maximum {
        return Err(EditorError::SourceTooLarge {
            size: bytes.len(),
            maximum,
        });
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            EditorError::SourceWrite("project source has no parent directory".to_owned())
        })?;
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|error| EditorError::SourceWrite(format!("{}: {error}", parent.display())))?;
    if !parent_metadata.file_type().is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(EditorError::SourceWrite(
            "project source parent must be a real directory".to_owned(),
        ));
    }
    if path.exists() {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| EditorError::SourceWrite(format!("{}: {error}", path.display())))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(EditorError::InvalidSourcePath(path.display().to_string()));
        }
    }
    let temporary = source_temporary_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| EditorError::SourceWrite(format!("{}: {error}", temporary.display())))?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|error| EditorError::SourceWrite(error.to_string()))?;
        file.sync_all()
            .map_err(|error| EditorError::SourceWrite(error.to_string()))?;
        fs::rename(&temporary, path).map_err(|error| EditorError::SourceWrite(error.to_string()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn source_temporary_path(path: &Path) -> PathBuf {
    let sequence = SOURCE_TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temporary = OsString::from(path.as_os_str());
    temporary.push(format!(".{}.{}.tmp", std::process::id(), sequence));
    PathBuf::from(temporary)
}

fn remove_source_file(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// Durable file-backed crash-recovery boundary for an [`EditorSession`].
pub struct EditorRecoveryStore {
    store: SaveStore,
}

impl EditorRecoveryStore {
    /// Creates a recovery store rooted at one caller-owned path.
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            store: SaveStore::new(path.as_ref(), SaveConfig::default()),
        }
    }

    /// Returns the primary recovery path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.store.path()
    }

    /// Atomically persists a serializable source/history snapshot.
    ///
    /// # Errors
    ///
    /// Returns serialization or durable-save failures without claiming recovery.
    pub fn save(&self, session: &EditorSession) -> Result<(), EditorError> {
        let snapshot = RecoverySnapshot {
            schema: RECOVERY_SCHEMA.to_owned(),
            session: session.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| EditorError::RecoveryEncoding(error.to_string()))?;
        self.store.save(bytes).map_err(EditorError::RecoverySave)
    }

    /// Recovers the latest durable source snapshot or its previous backup.
    ///
    /// # Errors
    ///
    /// Returns an error if both durable copies are invalid or violate source
    /// rules. Recovery history cannot establish an authoritative provenance
    /// from a caller-owned sidecar, so this method always rebuilds a fresh
    /// session from the validated snapshot document. A persisted selection is
    /// retained only when it is current and contains selectable identities.
    pub fn load(&self) -> Result<EditorSession, EditorError> {
        let bytes = self.store.load().map_err(EditorError::RecoverySave)?;
        let snapshot: RecoverySnapshot = serde_json::from_slice(&bytes)
            .map_err(|error| EditorError::RecoveryDecoding(error.to_string()))?;
        if snapshot.schema != RECOVERY_SCHEMA {
            return Err(EditorError::UnsupportedRecoverySchema(snapshot.schema));
        }
        let selection = snapshot.session.selection;
        let mut session = EditorSession::open(snapshot.session.document)?;
        if selection.generation == session.document.generation {
            let _ = session.select(selection.ids);
        }
        Ok(session)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RecoverySnapshot {
    schema: String,
    session: EditorSession,
}

fn validate_transaction(
    document: &ProjectDocument,
    transaction: &EditorTransaction,
) -> Result<(), EditorError> {
    document.validate()?;
    if transaction.metadata.label.trim().is_empty() || transaction.metadata.actor.trim().is_empty()
    {
        return Err(EditorError::InvalidCommandMetadata);
    }
    let affected = command_affected_ids(&transaction.command);
    if affected != transaction.metadata.affected_ids {
        return Err(EditorError::AffectedIdsMismatch);
    }
    match &transaction.command {
        EditorCommand::RegisterImportedSource(source) => {
            if document.sources.contains_key(&source.id) {
                return Err(EditorError::DuplicateSource(source.id));
            }
            if document.placements.contains_key(&source.id) {
                return Err(EditorError::StableIdentityCollision(source.id));
            }
            if source.label.trim().is_empty()
                || source.source_path.trim().is_empty()
                || source.source_hash.trim().is_empty()
            {
                return Err(EditorError::InvalidImportedSource(source.id));
            }
        }
        EditorCommand::UpdateImportedSource(source) => {
            if !document.sources.contains_key(&source.id) {
                return Err(EditorError::UnknownSource(source.id));
            }
            if source.label.trim().is_empty()
                || source.source_path.trim().is_empty()
                || source.source_hash.trim().is_empty()
            {
                return Err(EditorError::InvalidImportedSource(source.id));
            }
        }
        EditorCommand::PlaceObject(placement) => {
            if document.placements.contains_key(&placement.id) {
                return Err(EditorError::DuplicatePlacement(placement.id));
            }
            if document.sources.contains_key(&placement.id) {
                return Err(EditorError::StableIdentityCollision(placement.id));
            }
            if placement.label.trim().is_empty() {
                return Err(EditorError::InvalidPlacement(placement.id));
            }
            if !document.sources.contains_key(&placement.source_id) {
                return Err(EditorError::UnknownSource(placement.source_id));
            }
        }
        EditorCommand::RemoveImportedSource { source_id } => {
            if !document.sources.contains_key(source_id) {
                return Err(EditorError::UnknownSource(*source_id));
            }
            if document
                .placements
                .values()
                .any(|placement| placement.source_id == *source_id)
            {
                return Err(EditorError::SourceStillReferenced(*source_id));
            }
        }
        EditorCommand::SetPlacementTranslation { placement_id, .. }
        | EditorCommand::RemovePlacement { placement_id } => {
            if !document.placements.contains_key(placement_id) {
                return Err(EditorError::UnknownPlacement(*placement_id));
            }
        }
    }
    Ok(())
}

fn command_affected_ids(command: &EditorCommand) -> BTreeSet<StableId> {
    let mut ids = BTreeSet::new();
    match command {
        EditorCommand::RegisterImportedSource(source)
        | EditorCommand::UpdateImportedSource(source) => {
            ids.insert(source.id);
        }
        EditorCommand::RemoveImportedSource { source_id } => {
            ids.insert(*source_id);
        }
        EditorCommand::PlaceObject(placement) => {
            ids.insert(placement.id);
            ids.insert(placement.source_id);
        }
        EditorCommand::SetPlacementTranslation { placement_id, .. }
        | EditorCommand::RemovePlacement { placement_id } => {
            ids.insert(*placement_id);
        }
    }
    ids
}

fn apply_command(
    document: &mut ProjectDocument,
    command: &EditorCommand,
) -> Result<EditorCommand, EditorError> {
    match command {
        EditorCommand::RegisterImportedSource(source) => {
            document.sources.insert(source.id, source.clone());
            Ok(EditorCommand::RemoveImportedSource {
                source_id: source.id,
            })
        }
        EditorCommand::UpdateImportedSource(source) => {
            let previous = document
                .sources
                .insert(source.id, source.clone())
                .ok_or(EditorError::UnknownSource(source.id))?;
            Ok(EditorCommand::UpdateImportedSource(previous))
        }
        EditorCommand::RemoveImportedSource { source_id } => {
            let source = document
                .sources
                .remove(source_id)
                .ok_or(EditorError::UnknownSource(*source_id))?;
            Ok(EditorCommand::RegisterImportedSource(source))
        }
        EditorCommand::PlaceObject(placement) => {
            document.placements.insert(placement.id, placement.clone());
            Ok(EditorCommand::RemovePlacement {
                placement_id: placement.id,
            })
        }
        EditorCommand::SetPlacementTranslation {
            placement_id,
            translation,
        } => {
            let placement = document
                .placements
                .get_mut(placement_id)
                .ok_or(EditorError::UnknownPlacement(*placement_id))?;
            let before = placement.translation;
            placement.translation = *translation;
            Ok(EditorCommand::SetPlacementTranslation {
                placement_id: *placement_id,
                translation: before,
            })
        }
        EditorCommand::RemovePlacement { placement_id } => {
            let placement = document
                .placements
                .remove(placement_id)
                .ok_or(EditorError::UnknownPlacement(*placement_id))?;
            Ok(EditorCommand::PlaceObject(placement))
        }
    }
}

/// Typed errors for source validation, session history, Play, and recovery.
#[derive(Debug)]
pub enum EditorError {
    /// Project source path is not a regular non-symlink file.
    InvalidSourcePath(String),
    /// Project source could not be read.
    SourceRead(String),
    /// Project source exceeded the durable source bound.
    SourceTooLarge { size: usize, maximum: usize },
    /// Project source JSON could not be decoded.
    SourceDecoding(String),
    /// Project source JSON could not be encoded canonically.
    SourceEncoding(String),
    /// Atomic project-source persistence failed.
    SourceWrite(String),
    /// Recovery persistence failed after source changed and source rollback failed.
    SourceRollbackFailed(String),
    /// Project source schema is not supported by this Creator Alpha boundary.
    UnsupportedProjectSchema(String),
    /// Recovery payload schema is not supported.
    UnsupportedRecoverySchema(String),
    /// Source map key does not match embedded stable ID.
    SourceKeyMismatch(StableId),
    /// Placement map key does not match embedded stable ID.
    PlacementKeyMismatch(StableId),
    /// One stable ID was assigned to both a source and a placement.
    StableIdentityCollision(StableId),
    /// Imported source is incomplete.
    InvalidImportedSource(StableId),
    /// Placement is incomplete.
    InvalidPlacement(StableId),
    /// Placement references a missing imported source.
    UnknownSource(StableId),
    /// Source register command would overwrite an existing source.
    DuplicateSource(StableId),
    /// A source cannot be removed while placements still reference it.
    SourceStillReferenced(StableId),
    /// Placement command would overwrite an existing object.
    DuplicatePlacement(StableId),
    /// Placement identity is unavailable.
    UnknownPlacement(StableId),
    /// Selection identity is unavailable.
    UnknownSelection(StableId),
    /// Selection refers to a different document revision.
    StaleSelection {
        /// Selection's recorded document generation.
        selection_generation: u64,
        /// Current document generation.
        document_generation: u64,
    },
    /// Command metadata lacks its required label or actor.
    InvalidCommandMetadata,
    /// Metadata affected IDs do not match the typed command.
    AffectedIdsMismatch,
    /// Source generation cannot increment further.
    GenerationExhausted,
    /// No undo entry exists.
    NothingToUndo,
    /// No redo entry exists.
    NothingToRedo,
    /// A source edit was attempted while the runtime fork is active.
    PlaySessionActive,
    /// A Play-only operation was requested while Play is inactive.
    NoPlaySession,
    /// Snapshot encoding failed.
    RecoveryEncoding(String),
    /// Snapshot decoding failed.
    RecoveryDecoding(String),
    /// A recovered history entry or checkpoint is structurally invalid.
    InvalidRecoveryHistory,
    /// Durable recovery storage failed.
    RecoverySave(SaveError),
}

impl Display for EditorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourcePath(path) => write!(formatter, "project source path is not a regular file: {path}"),
            Self::SourceRead(message) => write!(formatter, "project source read failed: {message}"),
            Self::SourceTooLarge { size, maximum } => write!(formatter, "project source has {size} bytes; maximum is {maximum}"),
            Self::SourceDecoding(message) => write!(formatter, "project source decoding failed: {message}"),
            Self::SourceEncoding(message) => write!(formatter, "project source encoding failed: {message}"),
            Self::SourceWrite(message) => write!(formatter, "project source persistence failed: {message}"),
            Self::SourceRollbackFailed(message) => write!(formatter, "project source rollback failed: {message}"),
            Self::UnsupportedProjectSchema(schema) => {
                write!(formatter, "unsupported project schema: {schema}")
            }
            Self::UnsupportedRecoverySchema(schema) => {
                write!(formatter, "unsupported recovery schema: {schema}")
            }
            Self::SourceKeyMismatch(id) => write!(formatter, "source key does not match ID: {id}"),
            Self::PlacementKeyMismatch(id) => {
                write!(formatter, "placement key does not match ID: {id}")
            }
            Self::StableIdentityCollision(id) => {
                write!(formatter, "stable ID is used by multiple source objects: {id}")
            }
            Self::InvalidImportedSource(id) => write!(formatter, "invalid imported source: {id}"),
            Self::InvalidPlacement(id) => write!(formatter, "invalid world placement: {id}"),
            Self::UnknownSource(id) => write!(formatter, "unknown imported source: {id}"),
            Self::DuplicateSource(id) => write!(formatter, "duplicate imported source: {id}"),
            Self::SourceStillReferenced(id) => {
                write!(formatter, "imported source remains referenced: {id}")
            }
            Self::DuplicatePlacement(id) => write!(formatter, "duplicate world placement: {id}"),
            Self::UnknownPlacement(id) => write!(formatter, "unknown world placement: {id}"),
            Self::UnknownSelection(id) => write!(formatter, "unknown selectable ID: {id}"),
            Self::StaleSelection {
                selection_generation,
                document_generation,
            } => write!(
                formatter,
                "stale selection generation {selection_generation}; document generation is {document_generation}"
            ),
            Self::InvalidCommandMetadata => formatter.write_str("invalid command metadata"),
            Self::AffectedIdsMismatch => formatter.write_str("command affected IDs do not match metadata"),
            Self::GenerationExhausted => formatter.write_str("project document generation is exhausted"),
            Self::NothingToUndo => formatter.write_str("nothing to undo"),
            Self::NothingToRedo => formatter.write_str("nothing to redo"),
            Self::PlaySessionActive => formatter.write_str("a Play session is already active"),
            Self::NoPlaySession => formatter.write_str("no Play session is active"),
            Self::RecoveryEncoding(message) => write!(formatter, "recovery encoding failed: {message}"),
            Self::RecoveryDecoding(message) => write!(formatter, "recovery decoding failed: {message}"),
            Self::InvalidRecoveryHistory => formatter.write_str("recovery history is invalid"),
            Self::RecoverySave(error) => write!(formatter, "recovery storage failed: {error}"),
        }
    }
}

impl Error for EditorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RecoverySave(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    fn id(value: u128) -> StableId {
        StableId::new(value)
    }

    fn source() -> ImportedSource {
        ImportedSource {
            id: id(2),
            label: "Public triangle".to_owned(),
            source_path: "assets/public-triangle.mesh.json".to_owned(),
            source_hash: "public-triangle-v1".to_owned(),
        }
    }

    fn placement() -> WorldPlacement {
        WorldPlacement {
            id: id(3),
            source_id: id(2),
            label: "Triangle placement".to_owned(),
            translation: Translation::default(),
        }
    }

    fn transaction(command: EditorCommand, label: &str) -> EditorTransaction {
        EditorTransaction {
            metadata: CommandMetadata::local(label, command_affected_ids(&command)),
            command,
        }
    }

    fn seeded_session() -> EditorSession {
        let mut session = EditorSession::open(ProjectDocument::new(id(1))).expect("valid session");
        session
            .commit(transaction(
                EditorCommand::RegisterImportedSource(source()),
                "Register imported source",
            ))
            .expect("source commit");
        session
            .commit(transaction(
                EditorCommand::PlaceObject(placement()),
                "Place object",
            ))
            .expect("placement commit");
        session
    }

    #[test]
    fn source_schema_rejects_unknown_fields_recursively_without_rewrite() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "meridian-editor-core-strict-source-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("project directory");
        let path = root.join("project.meridian.json");
        let canonical = seeded_session()
            .document()
            .canonical_json()
            .expect("canonical source");
        let value: serde_json::Value =
            serde_json::from_str(&canonical).expect("canonical source parses as JSON");
        let mut fixtures = Vec::new();

        let mut root_unknown = value.clone();
        root_unknown
            .as_object_mut()
            .expect("root object")
            .insert("unexpected_root".to_owned(), serde_json::Value::Bool(true));
        fixtures.push(root_unknown);

        let mut imported_source_unknown = value.clone();
        imported_source_unknown
            .get_mut("sources")
            .and_then(serde_json::Value::as_object_mut)
            .expect("source map")
            .values_mut()
            .next()
            .and_then(serde_json::Value::as_object_mut)
            .expect("imported source")
            .insert(
                "unexpected_source".to_owned(),
                serde_json::Value::Bool(true),
            );
        fixtures.push(imported_source_unknown);

        let mut placement_unknown = value.clone();
        placement_unknown
            .get_mut("placements")
            .and_then(serde_json::Value::as_object_mut)
            .expect("placement map")
            .values_mut()
            .next()
            .and_then(serde_json::Value::as_object_mut)
            .expect("placement")
            .insert(
                "unexpected_placement".to_owned(),
                serde_json::Value::Bool(true),
            );
        fixtures.push(placement_unknown);

        let mut translation_unknown = value;
        let placements = translation_unknown
            .get_mut("placements")
            .and_then(serde_json::Value::as_object_mut)
            .expect("placement map");
        let placement = placements
            .values_mut()
            .next()
            .and_then(serde_json::Value::as_object_mut)
            .expect("placement");
        placement
            .get_mut("translation")
            .and_then(serde_json::Value::as_object_mut)
            .expect("translation")
            .insert(
                "unexpected_translation".to_owned(),
                serde_json::Value::Bool(true),
            );
        fixtures.push(translation_unknown);

        for fixture in fixtures {
            let bytes = serde_json::to_vec_pretty(&fixture).expect("fixture encodes");
            fs::write(&path, &bytes).expect("fixture writes");
            assert!(matches!(
                ProjectDocument::read_source(&path),
                Err(EditorError::SourceDecoding(_))
            ));
            assert_eq!(fs::read(&path).expect("fixture remains"), bytes);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn transactions_preview_commit_and_inverse_round_trip() {
        let mut session = seeded_session();
        let command = EditorCommand::SetPlacementTranslation {
            placement_id: id(3),
            translation: Translation {
                x_mm: 1250,
                y_mm: 0,
                z_mm: -500,
            },
        };
        let transaction = transaction(command, "Move placement");
        let preview = session.preview(&transaction).expect("preview");
        assert_eq!(preview.affected_ids, [id(3)].into_iter().collect());
        session.commit(transaction).expect("commit");
        assert_eq!(session.document().placements[&id(3)].translation.x_mm, 1250);
        session.undo().expect("undo");
        assert_eq!(
            session.document().placements[&id(3)].translation,
            Translation::default()
        );
        session.redo().expect("redo");
        assert_eq!(session.document().placements[&id(3)].translation.z_mm, -500);
    }

    #[test]
    fn invalid_metadata_preserves_source_and_history() {
        let mut session = seeded_session();
        let before = session.clone();
        let command = EditorCommand::SetPlacementTranslation {
            placement_id: id(3),
            translation: Translation {
                x_mm: 1,
                ..Translation::default()
            },
        };
        let error = session
            .commit(EditorTransaction {
                command,
                metadata: CommandMetadata::local("bad", []),
            })
            .expect_err("affected ID mismatch rejected");
        assert!(matches!(error, EditorError::AffectedIdsMismatch));
        assert_eq!(session, before);
    }

    #[test]
    fn generation_exhaustion_rolls_back_commit_undo_and_redo() {
        let move_placement = || {
            transaction(
                EditorCommand::SetPlacementTranslation {
                    placement_id: id(3),
                    translation: Translation {
                        x_mm: 1,
                        ..Translation::default()
                    },
                },
                "Move placement",
            )
        };

        let mut commit_session = seeded_session();
        commit_session.document.generation = u64::MAX;
        let before_commit = commit_session.clone();
        assert!(matches!(
            commit_session.commit(move_placement()),
            Err(EditorError::GenerationExhausted)
        ));
        assert_eq!(commit_session, before_commit);

        let mut undo_session = seeded_session();
        undo_session.document.generation = u64::MAX;
        let before_undo = undo_session.clone();
        assert!(matches!(
            undo_session.undo(),
            Err(EditorError::GenerationExhausted)
        ));
        assert_eq!(undo_session, before_undo);

        let mut redo_session = seeded_session();
        redo_session.undo().expect("undo creates redo history");
        redo_session.document.generation = u64::MAX;
        let before_redo = redo_session.clone();
        assert!(matches!(
            redo_session.redo(),
            Err(EditorError::GenerationExhausted)
        ));
        assert_eq!(redo_session, before_redo);
    }

    #[test]
    fn stable_identity_collision_is_rejected_before_mutation() {
        let mut session = seeded_session();
        let before = session.clone();
        let colliding = WorldPlacement {
            id: id(2),
            source_id: id(2),
            label: "Colliding placement".to_owned(),
            translation: Translation::default(),
        };
        let error = session
            .commit(transaction(
                EditorCommand::PlaceObject(colliding),
                "Place colliding object",
            ))
            .expect_err("global stable IDs must not collide");
        assert!(
            matches!(error, EditorError::StableIdentityCollision(id) if id == StableId::new(2))
        );
        assert_eq!(session, before);
    }

    #[test]
    fn checkpoint_is_retained_at_bounded_interval() {
        let mut session = seeded_session();
        for x_mm in 0..6 {
            session
                .commit(transaction(
                    EditorCommand::SetPlacementTranslation {
                        placement_id: id(3),
                        translation: Translation {
                            x_mm,
                            ..Translation::default()
                        },
                    },
                    "Move placement",
                ))
                .expect("commit");
        }
        assert_eq!(session.checkpoints().len(), 1);
        assert_eq!(session.checkpoints()[0].generation, 8);
    }

    #[test]
    fn selection_is_generation_checked() {
        let mut session = seeded_session();
        session.select([id(3)]).expect("select");
        session
            .commit(transaction(
                EditorCommand::SetPlacementTranslation {
                    placement_id: id(3),
                    translation: Translation {
                        x_mm: 1,
                        ..Translation::default()
                    },
                },
                "Move placement",
            ))
            .expect("commit");
        assert!(matches!(
            session.selection().validate(session.document()),
            Err(EditorError::StaleSelection { .. })
        ));
    }

    #[test]
    fn play_isolated_then_explicitly_applies_or_discards() {
        let mut session = seeded_session();
        session.start_play().expect("play starts");
        session
            .set_play_translation(
                id(3),
                Translation {
                    x_mm: 77,
                    ..Translation::default()
                },
            )
            .expect("runtime edit");
        assert_eq!(
            session.document().placements[&id(3)].translation,
            Translation::default()
        );
        assert_eq!(session.play_changes().expect("diff").len(), 1);
        session.discard_play().expect("discard");
        assert_eq!(
            session.document().placements[&id(3)].translation,
            Translation::default()
        );

        session.start_play().expect("play starts");
        session
            .set_play_translation(
                id(3),
                Translation {
                    z_mm: 88,
                    ..Translation::default()
                },
            )
            .expect("runtime edit");
        let changes = session.apply_play().expect("apply explicit diff");
        assert_eq!(changes.len(), 1);
        assert_eq!(session.document().placements[&id(3)].translation.z_mm, 88);
    }

    #[test]
    fn recovery_round_trip_rebuilds_source_without_history() {
        let session = seeded_session();
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "meridian-editor-core-recovery-{}-{unique}.state",
            std::process::id()
        ));
        let store = EditorRecoveryStore::new(&path);
        store.save(&session).expect("save recovery");
        let mut recovered = store.load().expect("recover");
        assert_eq!(recovered.document(), session.document());
        assert_eq!(recovered.selection().ids.len(), 0);
        assert_eq!(
            recovered.selection().generation,
            recovered.document().generation
        );
        assert!(matches!(recovered.undo(), Err(EditorError::NothingToUndo)));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("state.bak"));
    }

    #[test]
    fn recovery_discards_an_uncommitted_play_fork() {
        let mut session = seeded_session();
        session.start_play().expect("Play starts");
        session
            .set_play_translation(
                id(3),
                Translation {
                    x_mm: 50,
                    ..Translation::default()
                },
            )
            .expect("runtime edit");
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "meridian-editor-core-play-recovery-{}-{unique}.state",
            std::process::id()
        ));
        let store = EditorRecoveryStore::new(&path);
        store.save(&session).expect("save recovery");
        let mut recovered = store.load().expect("recover");
        recovered
            .commit(transaction(
                EditorCommand::SetPlacementTranslation {
                    placement_id: id(3),
                    translation: Translation {
                        y_mm: 25,
                        ..Translation::default()
                    },
                },
                "Edit recovered source",
            ))
            .expect("recovered source is editable");
        assert_eq!(recovered.document().placements[&id(3)].translation.y_mm, 25);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("state.bak"));
    }

    #[test]
    fn project_store_writes_canonical_source_and_resumes_matching_source() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "meridian-editor-core-project-store-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("project directory");
        let store = ProjectStore::new(
            root.join("project.meridian.json"),
            root.join("editor-recovery.state"),
        );
        let session = store
            .create(ProjectDocument::new(id(1)))
            .expect("initial source persists");
        let source = fs::read_to_string(store.source_path()).expect("source reads");
        let parsed =
            ProjectDocument::read_source(store.source_path()).expect("canonical source parses");
        assert_eq!(parsed, *session.document());
        assert!(source.contains(PROJECT_SCHEMA));

        let opened = store.open().expect("matching recovery opens");
        assert_eq!(opened.recovery, ProjectRecoveryStatus::Restored);
        assert_eq!(opened.session.document(), session.document());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_store_ignores_matching_forged_recovery_history() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "meridian-editor-core-forged-history-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("project directory");
        let store = ProjectStore::new(
            root.join("project.meridian.json"),
            root.join("editor-recovery.state"),
        );
        let source_session = store
            .create(seeded_session().document().clone())
            .expect("authoritative source persists");
        let authoritative = source_session.document().clone();
        let mut forged = seeded_session();
        assert_eq!(forged.document(), &authoritative);
        forged.undo = vec![HistoryEntry {
            transaction: transaction(
                EditorCommand::SetPlacementTranslation {
                    placement_id: id(3),
                    translation: Translation::default(),
                },
                "Forged history",
            ),
            inverse: EditorCommand::SetPlacementTranslation {
                placement_id: id(3),
                translation: Translation {
                    x_mm: 999,
                    ..Translation::default()
                },
            },
        }];
        let snapshot = RecoverySnapshot {
            schema: RECOVERY_SCHEMA.to_owned(),
            session: forged,
        };
        let bytes = serde_json::to_vec_pretty(&snapshot).expect("forged sidecar encodes");
        store
            .recovery
            .store
            .save(bytes)
            .expect("forged sidecar saves");

        let mut opened = store.open().expect("authoritative source reopens");
        assert_eq!(opened.recovery, ProjectRecoveryStatus::Restored);
        assert_eq!(opened.session.document(), &authoritative);
        assert!(matches!(
            opened.session.undo(),
            Err(EditorError::NothingToUndo)
        ));
        assert_eq!(opened.session.document(), &authoritative);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_store_create_does_not_replace_existing_authoritative_source() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "meridian-editor-core-existing-project-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("project directory");
        let store = ProjectStore::new(
            root.join("project.meridian.json"),
            root.join("editor-recovery.state"),
        );
        let existing = b"existing-authoritative-source";
        fs::write(store.source_path(), existing).expect("existing source fixture");

        let error = store
            .create(ProjectDocument::new(id(1)))
            .expect_err("creation must not replace existing source");

        assert!(matches!(error, EditorError::SourceWrite(_)));
        assert_eq!(
            fs::read(store.source_path()).expect("existing source survives"),
            existing
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_store_rolls_back_in_memory_mutation_when_source_write_fails() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let parent = env::temp_dir().join(format!(
            "meridian-editor-core-invalid-parent-{}-{unique}",
            std::process::id()
        ));
        fs::write(&parent, b"not a directory").expect("invalid parent fixture");
        let store = ProjectStore::new(
            parent.join("project.meridian.json"),
            parent.join("recovery"),
        );
        let mut session = seeded_session();
        let before = session.clone();
        let error = store
            .mutate(&mut session, |session| {
                session.commit(transaction(
                    EditorCommand::SetPlacementTranslation {
                        placement_id: id(3),
                        translation: Translation {
                            x_mm: 99,
                            ..Translation::default()
                        },
                    },
                    "Move placement",
                ))
            })
            .expect_err("source path rejects persistence");
        assert!(matches!(error, EditorError::SourceWrite(_)));
        assert_eq!(session, before);
        let _ = fs::remove_file(parent);
    }

    #[test]
    fn project_store_rolls_back_session_when_mutation_closure_errors() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "meridian-editor-core-closure-rollback-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("project directory");
        let store = ProjectStore::new(
            root.join("project.meridian.json"),
            root.join("editor-recovery.state"),
        );
        let mut session = store
            .create(seeded_session().document().clone())
            .expect("initial project persists");
        let source_before = fs::read(store.source_path()).expect("source before mutation");
        let before_mutation = session.clone();
        let error = store
            .mutate(&mut session, |session| {
                session.document.generation = 777;
                Err::<(), _>(EditorError::InvalidCommandMetadata)
            })
            .expect_err("failing closure rolls back session");
        assert!(matches!(error, EditorError::InvalidCommandMetadata));
        assert_eq!(session, before_mutation);
        assert_eq!(
            fs::read(store.source_path()).expect("source remains authoritative"),
            source_before
        );

        session.start_play().expect("play starts");
        let before_play_mutation = session.clone();
        let error = store
            .mutate_play(&mut session, |session| {
                session
                    .set_play_translation(
                        id(3),
                        Translation {
                            x_mm: 11,
                            ..Translation::default()
                        },
                    )
                    .expect("play mutation succeeds before explicit failure");
                Err::<(), _>(EditorError::InvalidCommandMetadata)
            })
            .expect_err("failing play closure rolls back session");
        assert!(matches!(error, EditorError::InvalidCommandMetadata));
        assert_eq!(session, before_play_mutation);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_store_restores_source_and_session_when_recovery_write_fails() {
        let unique = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "meridian-editor-core-recovery-rollback-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("project directory");
        let store = ProjectStore::new(
            root.join("project.meridian.json"),
            root.join("editor-recovery.state"),
        );
        let mut session = store
            .create(seeded_session().document().clone())
            .expect("initial project persists");
        let source_before = fs::read(store.source_path()).expect("source before mutation");
        let session_before = session.clone();
        let recovery_temporary = PathBuf::from(format!("{}.tmp", store.recovery_path().display()));
        fs::create_dir(&recovery_temporary).expect("recovery temporary obstruction");

        let error = store
            .mutate(&mut session, |session| {
                session.commit(transaction(
                    EditorCommand::SetPlacementTranslation {
                        placement_id: id(3),
                        translation: Translation {
                            x_mm: 71,
                            ..Translation::default()
                        },
                    },
                    "Move placement",
                ))
            })
            .expect_err("recovery write must fail");

        assert!(matches!(error, EditorError::RecoverySave(_)));
        assert_eq!(session, session_before);
        assert_eq!(
            fs::read(store.source_path()).expect("source after rollback"),
            source_before
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reimport_is_typed_and_undoable_without_changing_source_identity() {
        let mut session = seeded_session();
        let mut reimported = source();
        reimported.source_hash = "public-triangle-v2".to_owned();
        session
            .commit(transaction(
                EditorCommand::UpdateImportedSource(reimported.clone()),
                "Reimport source",
            ))
            .expect("reimport commits");
        assert_eq!(session.document().sources[&id(2)], reimported);
        session.undo().expect("reimport undo");
        assert_eq!(session.document().sources[&id(2)], source());
    }
}
