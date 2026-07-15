//! Provisional JSON source-world compiler for MS-01.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_asset_tools::AssetDatabaseSnapshot;
use meridian_assets::SourceId;
use meridian_core::StableId;
use meridian_world::{CompiledEntity, CompiledWorldCell, WorldCell, WorldPosition};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const WORLD_CELL_SOURCE_SCHEMA: &str = "meridian.world-cell-source/v1";
pub const DEFAULT_MAX_WORLD_SOURCE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_WORLD_SOURCE_ENTITIES: usize = 65_536;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorldEntitySource {
    pub stable_id: String,
    pub visual_source: String,
    pub position: [f64; 3],
    #[serde(default = "unit_scale")]
    pub scale: [f32; 3],
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorldCellSource {
    pub schema: String,
    pub version: u32,
    pub cell: [i64; 3],
    pub entities: Vec<WorldEntitySource>,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

const fn unit_scale() -> [f32; 3] {
    [1.0; 3]
}

/// Compiles editor JSON into deterministic runtime-only bytes.
///
/// # Errors
///
/// Rejects unsupported schemas, duplicates, missing visual sources, invalid IDs,
/// invalid transforms, and bounded-size violations.
pub fn compile_world_source(
    bytes: &[u8],
    assets: &AssetDatabaseSnapshot,
) -> Result<CompiledWorldCell, WorldCompileError> {
    if bytes.len() > DEFAULT_MAX_WORLD_SOURCE_BYTES {
        return Err(WorldCompileError::SourceTooLarge {
            size: bytes.len(),
            max: DEFAULT_MAX_WORLD_SOURCE_BYTES,
        });
    }
    let source: WorldCellSource = serde_json::from_slice(bytes)
        .map_err(|error| WorldCompileError::InvalidJson(error.to_string()))?;
    if source.schema != WORLD_CELL_SOURCE_SCHEMA || source.version != 1 {
        return Err(WorldCompileError::UnsupportedSchema {
            schema: source.schema,
            version: source.version,
        });
    }
    if source.entities.len() > MAX_WORLD_SOURCE_ENTITIES {
        return Err(WorldCompileError::EntityCountExceeded {
            count: source.entities.len(),
            max: MAX_WORLD_SOURCE_ENTITIES,
        });
    }
    let mut stable_ids = BTreeSet::new();
    let mut entities = Vec::with_capacity(source.entities.len());
    for entity in source.entities {
        let stable_id = parse_stable_id(&entity.stable_id)?;
        if !stable_ids.insert(stable_id) {
            return Err(WorldCompileError::DuplicateStableId(stable_id));
        }
        let visual_source = SourceId::from_canonical_name(entity.visual_source.trim());
        if !assets.meshes.contains_key(&visual_source) {
            return Err(WorldCompileError::MissingVisualSource(visual_source));
        }
        if entity.position.iter().any(|value| !value.is_finite())
            || entity.scale.iter().any(|value| !value.is_finite())
        {
            return Err(WorldCompileError::InvalidTransform(stable_id));
        }
        entities.push(CompiledEntity {
            stable_id,
            visual_source,
            position: WorldPosition::new(
                entity.position[0],
                entity.position[1],
                entity.position[2],
            ),
            scale: entity.scale,
        });
    }
    entities.sort_unstable_by_key(|entity| entity.stable_id);
    Ok(CompiledWorldCell::new(
        WorldCell::new(source.cell[0], source.cell[1], source.cell[2]),
        entities,
    ))
}

fn parse_stable_id(value: &str) -> Result<StableId, WorldCompileError> {
    let trimmed = value.trim();
    if trimmed.len() != 32 {
        return Err(WorldCompileError::InvalidStableId(value.to_owned()));
    }
    u128::from_str_radix(trimmed, 16)
        .map(StableId::new)
        .map_err(|_| WorldCompileError::InvalidStableId(value.to_owned()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldCompileError {
    InvalidJson(String),
    UnsupportedSchema { schema: String, version: u32 },
    SourceTooLarge { size: usize, max: usize },
    EntityCountExceeded { count: usize, max: usize },
    InvalidStableId(String),
    DuplicateStableId(StableId),
    MissingVisualSource(SourceId),
    InvalidTransform(StableId),
}

impl Display for WorldCompileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(formatter, "invalid world source JSON: {message}"),
            Self::UnsupportedSchema { schema, version } => {
                write!(
                    formatter,
                    "unsupported world source schema {schema} version {version}"
                )
            }
            Self::SourceTooLarge { size, max } => {
                write!(formatter, "world source is {size} bytes; maximum is {max}")
            }
            Self::EntityCountExceeded { count, max } => {
                write!(
                    formatter,
                    "world source has {count} entities; maximum is {max}"
                )
            }
            Self::InvalidStableId(value) => write!(formatter, "invalid stable entity ID: {value}"),
            Self::DuplicateStableId(id) => write!(formatter, "duplicate stable entity ID: {id}"),
            Self::MissingVisualSource(id) => write!(
                formatter,
                "world entity references missing visual source {id}"
            ),
            Self::InvalidTransform(id) => {
                write!(formatter, "world entity {id} has an invalid transform")
            }
        }
    }
}

impl Error for WorldCompileError {}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_asset_tools::import_fixture_mesh;
    use meridian_assets::CancellationToken;

    fn assets() -> AssetDatabaseSnapshot {
        let bytes = br#"{
          "schema":"meridian.fixture-mesh/v1","version":1,
          "source_id":"fixtures/ms01/triangle","authority":"engine_fixture",
          "provenance":{"origin":"test","license":"CC0-1.0"},"dependencies":[],
          "vertices":[
            {"position":[0,0,0],"normal":[0,0,1],"color":[1,0,0,1],"uv":[0,0]},
            {"position":[1,0,0],"normal":[0,0,1],"color":[0,1,0,1],"uv":[1,0]},
            {"position":[0,1,0],"normal":[0,0,1],"color":[0,0,1,1],"uv":[0,1]}
          ],"indices":[0,1,2]
        }"#;
        let mesh = import_fixture_mesh(bytes, &CancellationToken::new()).expect("mesh imports");
        AssetDatabaseSnapshot {
            generation: 1,
            meshes: [(mesh.metadata.source_id, mesh)].into_iter().collect(),
        }
    }

    fn world_source(stable_id: &str) -> Vec<u8> {
        format!(
            r#"{{"schema":"meridian.world-cell-source/v1","version":1,"cell":[0,0,0],
            "future":"preserved-at-source-boundary","entities":[{{"stable_id":"{stable_id}",
            "visual_source":"fixtures/ms01/triangle","position":[0.0,0.0,0.0],"scale":[1.0,1.0,1.0]}}]}}"#
        )
        .into_bytes()
    }

    #[test]
    fn source_compiles_deterministically_to_runtime_cell() {
        let assets = assets();
        let bytes = world_source("00000000000000000000000000000001");
        let first = compile_world_source(&bytes, &assets).expect("world compiles");
        let second = compile_world_source(&bytes, &assets).expect("world compiles");

        assert_eq!(first, second);
        assert_eq!(first.entities.len(), 1);
        assert_eq!(
            CompiledWorldCell::decode(&first.encode()).expect("runtime decodes"),
            first
        );
    }

    #[test]
    fn missing_visual_and_duplicate_stable_ids_are_rejected() {
        let empty_assets = AssetDatabaseSnapshot::default();
        assert!(matches!(
            compile_world_source(
                &world_source("00000000000000000000000000000001"),
                &empty_assets
            ),
            Err(WorldCompileError::MissingVisualSource(_))
        ));

        let assets = assets();
        let mut duplicated: WorldCellSource =
            serde_json::from_slice(&world_source("00000000000000000000000000000001"))
                .expect("source parses");
        duplicated.entities.push(duplicated.entities[0].clone());
        let duplicated = serde_json::to_vec(&duplicated).expect("source serializes");
        assert!(matches!(
            compile_world_source(&duplicated, &assets),
            Err(WorldCompileError::DuplicateStableId(_))
        ));
    }
}
