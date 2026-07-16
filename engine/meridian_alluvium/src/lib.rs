//! Text-authoritative, deterministic Alluvium recipe foundation.
//!
//! This crate owns source recipe validation, scalar reference evaluation,
//! generated identity, cache integrity, override reconciliation, provenance,
//! and licensing. It deliberately does not depend on runtime, renderer, UI,
//! platform, or private-game crates.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_assets::ArtifactHash;
use meridian_core::StableId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The only source schema accepted for new Alluvium recipes in this package.
pub const RECIPE_SCHEMA: &str = "meridian.procedural-recipe/v1";
/// Current textual source schema version.
pub const RECIPE_SCHEMA_VERSION: u32 = 1;

/// Parses the canonical persistent ID spelling used by `.mproc` and command
/// arguments.
///
/// # Errors
///
/// Returns an error unless `value` is exactly 32 hexadecimal characters.
pub fn parse_stable_id(value: &str) -> Result<StableId, AlluviumError> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AlluviumError::InvalidRecipe(
            "stable IDs must be 32 hexadecimal characters".to_owned(),
        ));
    }
    u128::from_str_radix(value, 16)
        .map(StableId::new)
        .map_err(|error| AlluviumError::InvalidRecipe(error.to_string()))
}

/// A checked hexadecimal persistent identifier in recipe JSON.
mod stable_id_hex {
    use super::StableId;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(id: &StableId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&id.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StableId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(serde::de::Error::custom(
                "stable IDs must be 32 hexadecimal characters",
            ));
        }
        u128::from_str_radix(&value, 16)
            .map(StableId::new)
            .map_err(serde::de::Error::custom)
    }
}

/// A human-readable, versioned, source-authoritative recipe.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProceduralRecipe {
    pub schema: String,
    pub schema_version: u32,
    pub recipe_version: u32,
    #[serde(with = "stable_id_hex")]
    pub id: StableId,
    pub label: String,
    pub default_seed: u64,
    pub determinism: DeterminismLevel,
    pub operation: ScalarGridPlacement,
    #[serde(default)]
    pub dependencies: Vec<RecipeDependency>,
    pub provenance: ProvenanceManifest,
    pub license_policy: LicensePolicy,
    #[serde(default)]
    pub overrides: Vec<GeneratedOverride>,
    #[serde(default)]
    pub migration_history: Vec<String>,
    /// Optional future data survives read/canonical-write without becoming
    /// executable semantics in this version.
    #[serde(flatten, default)]
    pub extensions: BTreeMap<String, Value>,
}

impl ProceduralRecipe {
    /// Parses and validates recipe source before any allocation proportional to
    /// its requested generated output count.
    ///
    /// # Errors
    ///
    /// Returns an error when JSON, schema, source identity, provenance, or
    /// bounded scalar-operation validation fails.
    pub fn from_json(source: &str) -> Result<Self, AlluviumError> {
        let recipe: Self = serde_json::from_str(source).map_err(AlluviumError::Json)?;
        recipe.validate()?;
        Ok(recipe)
    }

    /// Writes a stable, pretty JSON source representation.
    ///
    /// # Errors
    ///
    /// Returns an error when the recipe is invalid or serialization fails.
    pub fn canonical_json(&self) -> Result<String, AlluviumError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(AlluviumError::Json)
    }

    /// Ensures this source is safe to inspect or evaluate through the bounded
    /// scalar reference path.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported schema, invalid source semantics, or
    /// incomplete provenance/license declarations.
    pub fn validate(&self) -> Result<(), AlluviumError> {
        if self.schema != RECIPE_SCHEMA || self.schema_version != RECIPE_SCHEMA_VERSION {
            return Err(AlluviumError::UnsupportedSchema {
                schema: self.schema.clone(),
                version: self.schema_version,
            });
        }
        if self.recipe_version == 0 || self.label.trim().is_empty() {
            return Err(AlluviumError::InvalidRecipe(
                "recipe version and label must be nonzero/nonempty".to_owned(),
            ));
        }
        self.operation.validate()?;
        self.provenance.validate()?;
        self.license_policy.validate()?;
        let mut dependency_ids = BTreeSet::new();
        for dependency in &self.dependencies {
            if dependency.name.trim().is_empty() || !dependency_ids.insert(dependency.name.as_str())
            {
                return Err(AlluviumError::InvalidRecipe(
                    "dependency names must be nonempty and unique".to_owned(),
                ));
            }
            dependency.provenance.validate()?;
        }
        let mut targets = BTreeSet::new();
        for override_entry in &self.overrides {
            if !targets.insert(override_entry.target) {
                return Err(AlluviumError::InvalidRecipe(
                    "an output may have only one override entry".to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// Migrates an inspectable v0 compatibility document one explicit step to
    /// the current schema. No mutation is implicit.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported migration source or invalid result.
    pub fn migrate_one_step(mut self) -> Result<Self, AlluviumError> {
        if self.schema == RECIPE_SCHEMA && self.schema_version == RECIPE_SCHEMA_VERSION {
            return Ok(self);
        }
        if self.schema == "meridian.procedural-recipe/v0" && self.schema_version == 0 {
            RECIPE_SCHEMA.clone_into(&mut self.schema);
            self.schema_version = RECIPE_SCHEMA_VERSION;
            self.migration_history
                .push("meridian.procedural-recipe/v0 -> meridian.procedural-recipe/v1".to_owned());
            self.validate()?;
            return Ok(self);
        }
        Err(AlluviumError::UnsupportedSchema {
            schema: self.schema,
            version: self.schema_version,
        })
    }
}

/// Determinism promise for an evaluated source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismLevel {
    Strict,
    Stable,
    Opportunistic,
}

/// The initial bounded scalar operation. Its units are explicit millimetres;
/// later graph/field forms do not alter this source semantics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScalarGridPlacement {
    #[serde(with = "stable_id_hex")]
    pub source_id: StableId,
    pub count: u32,
    pub spacing_mm: i64,
    pub origin_mm: Millimetres3,
}

impl ScalarGridPlacement {
    fn validate(&self) -> Result<(), AlluviumError> {
        if self.count == 0 || self.spacing_mm <= 0 {
            return Err(AlluviumError::InvalidRecipe(
                "scalar grid count and spacing must be positive".to_owned(),
            ));
        }
        Ok(())
    }
}

/// A typed coordinate-space value used by the scalar reference evaluator.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Millimetres3 {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// A declared recipe input with its propagated provenance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeDependency {
    pub name: String,
    pub content_hash: String,
    pub provenance: ProvenanceManifest,
}

/// Provenance that remains with source and generated results.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProvenanceManifest {
    pub origin: String,
    pub license: String,
    #[serde(default)]
    pub attribution: Option<String>,
}

impl ProvenanceManifest {
    fn validate(&self) -> Result<(), AlluviumError> {
        if self.origin.trim().is_empty() || self.license.trim().is_empty() {
            return Err(AlluviumError::InvalidRecipe(
                "provenance origin and license must be nonempty".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Target-distribution policy evaluated by `license-audit` and authoritative
/// bakes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LicensePolicy {
    pub shipping_allowed: bool,
    #[serde(default)]
    pub allowed_licenses: BTreeSet<String>,
}

impl LicensePolicy {
    fn validate(&self) -> Result<(), AlluviumError> {
        if self
            .allowed_licenses
            .iter()
            .any(|license| license.trim().is_empty())
        {
            return Err(AlluviumError::InvalidRecipe(
                "allowed licenses must be nonempty".to_owned(),
            ));
        }
        Ok(())
    }
}

/// A retained, non-destructive user edit to a generated object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeneratedOverride {
    #[serde(with = "stable_id_hex")]
    pub target: StableId,
    #[serde(default, with = "stable_id_hex_option")]
    pub expected_source: Option<StableId>,
    pub action: OverrideAction,
}

mod stable_id_hex_option {
    use super::{stable_id_hex, StableId};
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(clippy::ref_option)] // serde `with` requires a reference to the field type.
    pub fn serialize<S>(value: &Option<StableId>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => stable_id_hex::serialize(value, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<StableId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        value
            .map(|value| {
                if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(serde::de::Error::custom(
                        "stable IDs must be 32 hexadecimal characters",
                    ));
                }
                u128::from_str_radix(&value, 16)
                    .map(StableId::new)
                    .map_err(serde::de::Error::custom)
            })
            .transpose()
    }
}

/// Explicit curation forms accepted by the initial evaluator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OverrideAction {
    Suppress,
    Translate {
        by_mm: Millimetres3,
    },
    ReplaceSource {
        #[serde(with = "stable_id_hex")]
        source_id: StableId,
    },
    AttachMetadata {
        key: String,
        value: String,
    },
}

/// Whether an override was retained, migrated, or needs a recovery decision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideStatus {
    Applied,
    Migrated,
    Conflicted,
    Orphaned,
    Invalid,
}

/// A recoverable outcome for one non-destructive override.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OverrideReconciliation {
    #[serde(with = "stable_id_hex")]
    pub target: StableId,
    pub status: OverrideStatus,
    pub detail: String,
}

/// Explicit evaluation mode. The first implementation has one scalar kernel;
/// it remains visibly distinct between preview and authoritative bake.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationMode {
    Preview,
    Bake,
}

/// A caller-selected bound prevents recipe source from allocating unbounded
/// output. No global capacity budget is invented by this package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationBudget {
    pub max_objects: usize,
}

/// Input control for one evaluation, including cooperative cancellation before
/// source work begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationRequest {
    pub mode: EvaluationMode,
    pub budget: EvaluationBudget,
    pub cancelled: bool,
}

/// A generated object is derived, never source authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeneratedPlacement {
    #[serde(with = "stable_id_hex")]
    pub id: StableId,
    #[serde(with = "stable_id_hex")]
    pub source_id: StableId,
    pub position_mm: Millimetres3,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Typed scalar output so callers cannot confuse millimetre coordinates with a
/// renderer or runtime transform.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScalarField {
    pub value_kind: String,
    pub unit: String,
    pub samples: Vec<GeneratedPlacement>,
}

/// Cache observability result for an evaluation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheDisposition {
    Miss,
    Hit,
    RecoveredCorruption,
}

/// Outcome of importing a host-persisted derived cache record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheRestoreDisposition {
    Restored,
    DiscardedCorruption,
}

/// Immutable result of the scalar reference evaluator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvaluationResult {
    pub mode: EvaluationMode,
    pub field: ScalarField,
    pub cache_key: String,
    pub cache: CacheDisposition,
    pub overrides: Vec<OverrideReconciliation>,
    pub provenance: ProvenanceManifest,
    pub determinism: DeterminismLevel,
}

/// A precise source-diff report used for dirty rebuild selection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirtyReport {
    pub dirty: bool,
    pub reasons: Vec<DirtyReason>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirtyReason {
    RecipeVersion,
    Operation,
    Dependency(String),
    Provenance,
    LicensePolicy,
    Override,
}

/// Machine-readable provenance and licensing audit result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LicenseAudit {
    pub target: String,
    pub accepted: bool,
    pub records: Vec<ProvenanceManifest>,
    pub rejected_licenses: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CacheRecord {
    checksum: String,
    result: EvaluationResult,
}

/// In-memory cache with an explicit corruption-recovery path. Hosts may persist
/// records through their own atomic data boundary without making the cache
/// source authority.
#[derive(Default)]
pub struct AlluviumEngine {
    cache: BTreeMap<String, CacheRecord>,
    recovered_corruption: BTreeSet<String>,
}

impl AlluviumEngine {
    /// Evaluates a validated immutable recipe with bounded source-derived work.
    ///
    /// # Errors
    ///
    /// Returns a typed validation, cancellation, or caller-budget failure.
    pub fn evaluate(
        &mut self,
        recipe: &ProceduralRecipe,
        request: EvaluationRequest,
    ) -> Result<EvaluationResult, AlluviumError> {
        recipe.validate()?;
        if request.cancelled {
            return Err(AlluviumError::Cancelled);
        }
        let requested = usize::try_from(recipe.operation.count)
            .map_err(|_| AlluviumError::InvalidRecipe("count cannot fit host size".to_owned()))?;
        if requested > request.budget.max_objects {
            return Err(AlluviumError::BudgetExceeded {
                requested,
                allowed: request.budget.max_objects,
            });
        }
        let key = cache_key(recipe, request.mode)?;
        if let Some(record) = self.cache.get(&key) {
            if cache_record_checksum(&record.result)? == record.checksum {
                let mut cached = record.result.clone();
                cached.cache = CacheDisposition::Hit;
                return Ok(cached);
            }
            self.cache.remove(&key);
            let rebuilt = Self::scalar_reference(
                recipe,
                request.mode,
                key,
                CacheDisposition::RecoveredCorruption,
            )?;
            self.store(rebuilt.clone())?;
            return Ok(rebuilt);
        }
        let disposition = if self.recovered_corruption.remove(&key) {
            CacheDisposition::RecoveredCorruption
        } else {
            CacheDisposition::Miss
        };
        let result = Self::scalar_reference(recipe, request.mode, key, disposition)?;
        self.store(result.clone())?;
        Ok(result)
    }

    /// Restores an externally retained derived cache record only if its checksum
    /// agrees with its result. Corrupt inputs are discarded and the next
    /// matching evaluation reports recovery; they are never published.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed cache data. Integrity-invalid cache data
    /// returns [`CacheRestoreDisposition::DiscardedCorruption`].
    pub fn restore_cache_record(
        &mut self,
        key: String,
        record_json: &str,
    ) -> Result<CacheRestoreDisposition, AlluviumError> {
        let record: CacheRecord = serde_json::from_str(record_json).map_err(AlluviumError::Json)?;
        if cache_record_checksum(&record.result)? != record.checksum {
            self.cache.remove(&key);
            self.recovered_corruption.insert(key);
            return Ok(CacheRestoreDisposition::DiscardedCorruption);
        }
        self.cache.insert(key, record);
        Ok(CacheRestoreDisposition::Restored)
    }

    /// Exports one derived cache record for a host-owned persistence boundary.
    ///
    /// # Errors
    ///
    /// Returns an error only when the retained record cannot be serialized.
    pub fn cache_record_json(&self, key: &str) -> Result<Option<String>, AlluviumError> {
        self.cache
            .get(key)
            .map(|record| serde_json::to_string(record).map_err(AlluviumError::Json))
            .transpose()
    }

    fn store(&mut self, result: EvaluationResult) -> Result<(), AlluviumError> {
        let checksum = cache_record_checksum(&result)?;
        self.cache
            .insert(result.cache_key.clone(), CacheRecord { checksum, result });
        Ok(())
    }

    fn scalar_reference(
        recipe: &ProceduralRecipe,
        mode: EvaluationMode,
        cache_key: String,
        cache: CacheDisposition,
    ) -> Result<EvaluationResult, AlluviumError> {
        let mut placements =
            Vec::with_capacity(usize::try_from(recipe.operation.count).unwrap_or(0));
        for candidate in 0..recipe.operation.count {
            let offset = i64::from(candidate)
                .checked_mul(recipe.operation.spacing_mm)
                .ok_or_else(|| {
                    AlluviumError::InvalidRecipe("placement coordinate overflow".to_owned())
                })?;
            let x = recipe
                .operation
                .origin_mm
                .x
                .checked_add(offset)
                .ok_or_else(|| {
                    AlluviumError::InvalidRecipe("placement coordinate overflow".to_owned())
                })?;
            let position = Millimetres3 {
                x,
                ..recipe.operation.origin_mm
            };
            placements.push(GeneratedPlacement {
                id: generated_id(recipe, position),
                source_id: recipe.operation.source_id,
                position_mm: position,
                metadata: BTreeMap::new(),
            });
        }
        let overrides = apply_overrides(recipe, &mut placements);
        Ok(EvaluationResult {
            mode,
            field: ScalarField {
                value_kind: "placement".to_owned(),
                unit: "millimetres".to_owned(),
                samples: placements,
            },
            cache_key,
            cache,
            overrides,
            provenance: recipe.provenance.clone(),
            determinism: recipe.determinism,
        })
    }
}

/// Computes the smallest source change class without broad invalidation.
#[must_use]
pub fn dirty_report(previous: &ProceduralRecipe, current: &ProceduralRecipe) -> DirtyReport {
    let mut reasons = Vec::new();
    if previous.recipe_version != current.recipe_version
        || previous.default_seed != current.default_seed
    {
        reasons.push(DirtyReason::RecipeVersion);
    }
    if previous.operation != current.operation || previous.determinism != current.determinism {
        reasons.push(DirtyReason::Operation);
    }
    let previous_dependencies: BTreeMap<_, _> = previous
        .dependencies
        .iter()
        .map(|entry| (&entry.name, &entry.content_hash))
        .collect();
    let current_dependencies: BTreeMap<_, _> = current
        .dependencies
        .iter()
        .map(|entry| (&entry.name, &entry.content_hash))
        .collect();
    for name in previous_dependencies
        .keys()
        .chain(current_dependencies.keys())
    {
        if previous_dependencies.get(name) != current_dependencies.get(name) {
            reasons.push(DirtyReason::Dependency((**name).clone()));
        }
    }
    if previous.provenance != current.provenance {
        reasons.push(DirtyReason::Provenance);
    }
    if previous.license_policy != current.license_policy {
        reasons.push(DirtyReason::LicensePolicy);
    }
    if previous.overrides != current.overrides {
        reasons.push(DirtyReason::Override);
    }
    reasons.sort_by_key(|reason| format!("{reason:?}"));
    reasons.dedup();
    DirtyReport {
        dirty: !reasons.is_empty(),
        reasons,
    }
}

/// Audits source and declared dependencies for a requested distribution target.
///
/// # Errors
///
/// Returns an error if recipe source is invalid or the target is blank.
pub fn license_audit(
    recipe: &ProceduralRecipe,
    target: &str,
) -> Result<LicenseAudit, AlluviumError> {
    recipe.validate()?;
    if target.trim().is_empty() {
        return Err(AlluviumError::InvalidTarget);
    }
    let mut records = vec![recipe.provenance.clone()];
    records.extend(
        recipe
            .dependencies
            .iter()
            .map(|dependency| dependency.provenance.clone()),
    );
    let rejected_licenses: Vec<String> = records
        .iter()
        .filter(|record| {
            !recipe
                .license_policy
                .allowed_licenses
                .contains(&record.license)
        })
        .map(|record| record.license.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(LicenseAudit {
        target: target.to_owned(),
        accepted: recipe.license_policy.shipping_allowed && rejected_licenses.is_empty(),
        records,
        rejected_licenses,
    })
}

/// Finds a generated output without conflating a missing result with an error.
///
/// # Errors
///
/// Returns an error if the recipe cannot be validated or evaluated.
pub fn explain(
    recipe: &ProceduralRecipe,
    id: StableId,
) -> Result<Option<GeneratedPlacement>, AlluviumError> {
    let mut engine = AlluviumEngine::default();
    let result = engine.evaluate(
        recipe,
        EvaluationRequest {
            mode: EvaluationMode::Preview,
            budget: EvaluationBudget {
                max_objects: usize::try_from(recipe.operation.count).unwrap_or(0),
            },
            cancelled: false,
        },
    )?;
    Ok(result
        .field
        .samples
        .into_iter()
        .find(|placement| placement.id == id))
}

fn cache_key(recipe: &ProceduralRecipe, mode: EvaluationMode) -> Result<String, AlluviumError> {
    let mut source = recipe.canonical_json()?;
    source.push_str(match mode {
        EvaluationMode::Preview => "\npreview",
        EvaluationMode::Bake => "\nbake",
    });
    Ok(ArtifactHash::digest(source.as_bytes()).to_string())
}

fn cache_record_checksum(result: &EvaluationResult) -> Result<String, AlluviumError> {
    let bytes = serde_json::to_vec(result).map_err(AlluviumError::Json)?;
    Ok(ArtifactHash::digest(&bytes).to_string())
}

fn generated_id(recipe: &ProceduralRecipe, position: Millimetres3) -> StableId {
    let semantic = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        recipe.id,
        recipe.recipe_version,
        recipe.default_seed,
        recipe.operation.source_id,
        position.x,
        position.y,
        position.z
    );
    let hash = ArtifactHash::digest(semantic.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash.as_bytes()[..16]);
    StableId::new(u128::from_le_bytes(bytes))
}

fn apply_overrides(
    recipe: &ProceduralRecipe,
    placements: &mut Vec<GeneratedPlacement>,
) -> Vec<OverrideReconciliation> {
    let mut outcomes = Vec::with_capacity(recipe.overrides.len());
    for override_entry in &recipe.overrides {
        let Some(index) = placements
            .iter()
            .position(|placement| placement.id == override_entry.target)
        else {
            outcomes.push(OverrideReconciliation {
                target: override_entry.target,
                status: OverrideStatus::Orphaned,
                detail: "generated target no longer exists; retained for recovery".to_owned(),
            });
            continue;
        };
        if let Some(expected_source) = override_entry.expected_source {
            if placements[index].source_id != expected_source {
                outcomes.push(OverrideReconciliation { target: override_entry.target, status: OverrideStatus::Conflicted, detail: "generated source changed; explicit retarget or preserve-as-authored decision required".to_owned() });
                continue;
            }
        }
        match &override_entry.action {
            OverrideAction::Suppress => {
                placements.remove(index);
            }
            OverrideAction::Translate { by_mm } => {
                let placement = &mut placements[index];
                let Some(x) = placement.position_mm.x.checked_add(by_mm.x) else {
                    outcomes.push(OverrideReconciliation {
                        target: override_entry.target,
                        status: OverrideStatus::Invalid,
                        detail: "translation overflows millimetre coordinate range".to_owned(),
                    });
                    continue;
                };
                let Some(y) = placement.position_mm.y.checked_add(by_mm.y) else {
                    outcomes.push(OverrideReconciliation {
                        target: override_entry.target,
                        status: OverrideStatus::Invalid,
                        detail: "translation overflows millimetre coordinate range".to_owned(),
                    });
                    continue;
                };
                let Some(z) = placement.position_mm.z.checked_add(by_mm.z) else {
                    outcomes.push(OverrideReconciliation {
                        target: override_entry.target,
                        status: OverrideStatus::Invalid,
                        detail: "translation overflows millimetre coordinate range".to_owned(),
                    });
                    continue;
                };
                placement.position_mm = Millimetres3 { x, y, z };
            }
            OverrideAction::ReplaceSource { source_id } => placements[index].source_id = *source_id,
            OverrideAction::AttachMetadata { key, value } if key.trim().is_empty() => {
                outcomes.push(OverrideReconciliation {
                    target: override_entry.target,
                    status: OverrideStatus::Invalid,
                    detail: "metadata key must be nonempty".to_owned(),
                });
                continue;
            }
            OverrideAction::AttachMetadata { key, value } => {
                placements[index]
                    .metadata
                    .insert(key.clone(), value.clone());
            }
        }
        outcomes.push(OverrideReconciliation {
            target: override_entry.target,
            status: OverrideStatus::Applied,
            detail: "override retained and applied to derived output".to_owned(),
        });
    }
    outcomes
}

/// Typed failure outcomes for callers and command adapters.
#[derive(Debug)]
pub enum AlluviumError {
    Json(serde_json::Error),
    UnsupportedSchema { schema: String, version: u32 },
    InvalidRecipe(String),
    BudgetExceeded { requested: usize, allowed: usize },
    Cancelled,
    InvalidTarget,
}

impl Display for AlluviumError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "recipe JSON error: {error}"),
            Self::UnsupportedSchema { schema, version } => write!(
                formatter,
                "unsupported recipe schema {schema} version {version}"
            ),
            Self::InvalidRecipe(detail) => write!(formatter, "invalid recipe: {detail}"),
            Self::BudgetExceeded { requested, allowed } => write!(
                formatter,
                "recipe requested {requested} outputs but the caller allowed {allowed}"
            ),
            Self::Cancelled => {
                formatter.write_str("recipe evaluation was cancelled before execution")
            }
            Self::InvalidTarget => formatter.write_str("license-audit target must be nonempty"),
        }
    }
}

impl Error for AlluviumError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> ProceduralRecipe {
        ProceduralRecipe {
            schema: RECIPE_SCHEMA.to_owned(),
            schema_version: 1,
            recipe_version: 1,
            id: StableId::new(1),
            label: "Public placements".to_owned(),
            default_seed: 7,
            determinism: DeterminismLevel::Strict,
            operation: ScalarGridPlacement {
                source_id: StableId::new(2),
                count: 3,
                spacing_mm: 2000,
                origin_mm: Millimetres3 { x: 0, y: 0, z: 0 },
            },
            dependencies: vec![RecipeDependency {
                name: "public-fixture".to_owned(),
                content_hash: "fixture-v1".to_owned(),
                provenance: ProvenanceManifest {
                    origin: "Meridian public fixture".to_owned(),
                    license: "MIT OR Apache-2.0".to_owned(),
                    attribution: None,
                },
            }],
            provenance: ProvenanceManifest {
                origin: "Meridian public Creator Alpha".to_owned(),
                license: "MIT OR Apache-2.0".to_owned(),
                attribution: None,
            },
            license_policy: LicensePolicy {
                shipping_allowed: true,
                allowed_licenses: ["MIT OR Apache-2.0".to_owned()].into_iter().collect(),
            },
            overrides: vec![],
            migration_history: vec![],
            extensions: BTreeMap::new(),
        }
    }

    fn request() -> EvaluationRequest {
        EvaluationRequest {
            mode: EvaluationMode::Bake,
            budget: EvaluationBudget { max_objects: 3 },
            cancelled: false,
        }
    }

    #[test]
    fn canonical_json_round_trips_and_preserves_extensions() {
        let mut input = recipe();
        input
            .extensions
            .insert("future_optional".to_owned(), Value::Bool(true));
        let canonical = input.canonical_json().expect("canonical source");
        assert_eq!(
            ProceduralRecipe::from_json(&canonical).expect("parse"),
            input
        );
    }

    #[test]
    fn scalar_evaluation_is_deterministic_and_cacheable() {
        let mut engine = AlluviumEngine::default();
        let input = recipe();
        let first = engine
            .evaluate(&input, request())
            .expect("first evaluation");
        let second = engine
            .evaluate(&input, request())
            .expect("cached evaluation");
        assert_eq!(first.field.samples, second.field.samples);
        assert_eq!(second.cache, CacheDisposition::Hit);
        assert_ne!(first.field.samples[0].id, first.field.samples[1].id);
    }

    #[test]
    fn budget_and_cancellation_are_typed_failures() {
        let input = recipe();
        let mut engine = AlluviumEngine::default();
        assert!(matches!(
            engine.evaluate(
                &input,
                EvaluationRequest {
                    budget: EvaluationBudget { max_objects: 2 },
                    ..request()
                }
            ),
            Err(AlluviumError::BudgetExceeded { .. })
        ));
        assert!(matches!(
            engine.evaluate(
                &input,
                EvaluationRequest {
                    cancelled: true,
                    ..request()
                }
            ),
            Err(AlluviumError::Cancelled)
        ));
    }

    #[test]
    fn dirty_report_is_precise() {
        let before = recipe();
        let mut after = before.clone();
        after.operation.spacing_mm = 3_000;
        let report = dirty_report(&before, &after);
        assert_eq!(report.reasons, vec![DirtyReason::Operation]);
    }

    #[test]
    fn corrupted_cache_is_rejected_and_never_restored() {
        let input = recipe();
        let mut engine = AlluviumEngine::default();
        let result = engine.evaluate(&input, request()).expect("evaluation");
        let record = engine
            .cache_record_json(&result.cache_key)
            .expect("record")
            .expect("present");
        let corrupt = record.replacen("placement", "corruption", 1);
        let mut recovered = AlluviumEngine::default();
        assert_eq!(
            recovered
                .restore_cache_record(result.cache_key.clone(), &corrupt)
                .expect("corruption is discarded"),
            CacheRestoreDisposition::DiscardedCorruption
        );
        assert_eq!(
            recovered
                .evaluate(&input, request())
                .expect("rebuild after corruption")
                .cache,
            CacheDisposition::RecoveredCorruption
        );
    }

    #[test]
    fn overrides_report_applied_conflicted_and_orphaned_without_deletion() {
        let base = recipe();
        let mut engine = AlluviumEngine::default();
        let output = engine.evaluate(&base, request()).expect("base");
        let mut changed = base.clone();
        changed.overrides = vec![
            GeneratedOverride {
                target: output.field.samples[0].id,
                expected_source: Some(StableId::new(2)),
                action: OverrideAction::Suppress,
            },
            GeneratedOverride {
                target: output.field.samples[1].id,
                expected_source: Some(StableId::new(9)),
                action: OverrideAction::Suppress,
            },
            GeneratedOverride {
                target: StableId::new(99),
                expected_source: None,
                action: OverrideAction::Suppress,
            },
        ];
        let result = AlluviumEngine::default()
            .evaluate(&changed, request())
            .expect("override result");
        assert!(result
            .overrides
            .iter()
            .any(|outcome| outcome.status == OverrideStatus::Applied));
        assert!(result
            .overrides
            .iter()
            .any(|outcome| outcome.status == OverrideStatus::Conflicted));
        assert!(result
            .overrides
            .iter()
            .any(|outcome| outcome.status == OverrideStatus::Orphaned));
    }

    #[test]
    fn migration_is_explicit_and_license_audit_fails_closed() {
        let mut old = recipe();
        old.schema = "meridian.procedural-recipe/v0".to_owned();
        old.schema_version = 0;
        let migrated = old.migrate_one_step().expect("one step migration");
        assert_eq!(migrated.schema, RECIPE_SCHEMA);
        let mut rejected = migrated.clone();
        rejected.license_policy.allowed_licenses.clear();
        assert!(
            !license_audit(&rejected, "shipping")
                .expect("audit")
                .accepted
        );
    }
}
