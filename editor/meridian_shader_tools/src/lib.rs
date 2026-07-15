//! Offline shader manifests, WGSL validation, and binding reflection.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShaderId(u64);

impl ShaderId {
    /// Creates a stable FNV-1a identifier from a manifest name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in name.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        Self(hash)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShaderStage {
    Vertex,
    Task,
    Mesh,
    Fragment,
    Compute,
    RayGeneration,
    AnyHit,
    ClosestHit,
    Miss,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderEntryPoint {
    pub name: String,
    pub stage: ShaderStage,
}

impl ShaderEntryPoint {
    #[must_use]
    pub fn new(name: impl Into<String>, stage: ShaderStage) -> Self {
        Self {
            name: name.into(),
            stage,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderVariant {
    pub name: String,
    pub defines: Vec<(String, String)>,
}

impl ShaderVariant {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            defines: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_define(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defines.push((key.into(), value.into()));
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderBinding {
    pub group: u32,
    pub binding: u32,
    pub name: String,
}

impl ShaderBinding {
    #[must_use]
    pub fn new(group: u32, binding: u32, name: impl Into<String>) -> Self {
        Self {
            group,
            binding,
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderManifest {
    pub id: ShaderId,
    pub source_name: String,
    pub source: String,
    pub entry_points: Vec<ShaderEntryPoint>,
    pub variants: Vec<ShaderVariant>,
    pub expected_bindings: Vec<ShaderBinding>,
}

impl ShaderManifest {
    #[must_use]
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        let source_name = name.into();
        let id = ShaderId::from_name(&source_name);
        Self {
            id,
            source_name,
            source: source.into(),
            entry_points: Vec::new(),
            variants: Vec::new(),
            expected_bindings: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_entry_point(mut self, entry_point: ShaderEntryPoint) -> Self {
        self.entry_points.push(entry_point);
        self
    }

    #[must_use]
    pub fn with_variant(mut self, variant: ShaderVariant) -> Self {
        self.variants.push(variant);
        self
    }

    #[must_use]
    pub fn with_binding(mut self, binding: ShaderBinding) -> Self {
        self.expected_bindings.push(binding);
        self
    }

    /// Parses and validates WGSL, then checks manifest declarations against reflection.
    ///
    /// # Errors
    ///
    /// Returns [`ShaderError`] when WGSL is invalid, an entry point is missing,
    /// a manifest declaration is duplicated, or an expected resource binding
    /// does not exist in the reflected module.
    pub fn validate(&self) -> Result<ShaderReflection, ShaderError> {
        validate_manifest_declarations(self)?;
        let module =
            naga::front::wgsl::parse_str(&self.source).map_err(|error| ShaderError::Parse {
                source_name: self.source_name.clone(),
                message: error.to_string(),
            })?;
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .map_err(|error| ShaderError::Validation {
            source_name: self.source_name.clone(),
            message: error.to_string(),
        })?;

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry_point| ReflectedEntryPoint {
                name: entry_point.name.clone(),
                stage: shader_stage(entry_point.stage),
            })
            .collect::<Vec<_>>();
        for entry_point in &self.entry_points {
            if !entry_points.iter().any(|reflected| {
                reflected.name == entry_point.name && reflected.stage == entry_point.stage
            }) {
                return Err(ShaderError::MissingEntryPoint {
                    source_name: self.source_name.clone(),
                    name: entry_point.name.clone(),
                    stage: entry_point.stage,
                });
            }
        }

        let mut bindings = module
            .global_variables
            .iter()
            .filter_map(|(_, variable)| {
                variable.binding.map(|binding| ReflectedBinding {
                    group: binding.group,
                    binding: binding.binding,
                    name: variable.name.clone().unwrap_or_else(|| {
                        format!("binding_{}_{}", binding.group, binding.binding)
                    }),
                })
            })
            .collect::<Vec<_>>();
        bindings.sort_by_key(|binding| (binding.group, binding.binding));

        for expected in &self.expected_bindings {
            if !bindings.iter().any(|reflected| {
                reflected.group == expected.group && reflected.binding == expected.binding
            }) {
                return Err(ShaderError::MissingBinding {
                    source_name: self.source_name.clone(),
                    group: expected.group,
                    binding: expected.binding,
                });
            }
        }

        Ok(ShaderReflection {
            entry_points,
            bindings,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflectedEntryPoint {
    pub name: String,
    pub stage: ShaderStage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflectedBinding {
    pub group: u32,
    pub binding: u32,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderReflection {
    pub entry_points: Vec<ReflectedEntryPoint>,
    pub bindings: Vec<ReflectedBinding>,
}

fn validate_manifest_declarations(manifest: &ShaderManifest) -> Result<(), ShaderError> {
    for (index, entry_point) in manifest.entry_points.iter().enumerate() {
        if manifest.entry_points[..index].iter().any(|previous| {
            previous.name == entry_point.name && previous.stage == entry_point.stage
        }) {
            return Err(ShaderError::DuplicateEntryPoint {
                source_name: manifest.source_name.clone(),
                name: entry_point.name.clone(),
                stage: entry_point.stage,
            });
        }
    }

    for (index, binding) in manifest.expected_bindings.iter().enumerate() {
        if manifest.expected_bindings[..index]
            .iter()
            .any(|previous| previous.group == binding.group && previous.binding == binding.binding)
        {
            return Err(ShaderError::DuplicateBinding {
                source_name: manifest.source_name.clone(),
                group: binding.group,
                binding: binding.binding,
            });
        }
    }

    Ok(())
}

const fn shader_stage(stage: naga::ShaderStage) -> ShaderStage {
    match stage {
        naga::ShaderStage::Vertex => ShaderStage::Vertex,
        naga::ShaderStage::Task => ShaderStage::Task,
        naga::ShaderStage::Mesh => ShaderStage::Mesh,
        naga::ShaderStage::Fragment => ShaderStage::Fragment,
        naga::ShaderStage::Compute => ShaderStage::Compute,
        naga::ShaderStage::RayGeneration => ShaderStage::RayGeneration,
        naga::ShaderStage::AnyHit => ShaderStage::AnyHit,
        naga::ShaderStage::ClosestHit => ShaderStage::ClosestHit,
        naga::ShaderStage::Miss => ShaderStage::Miss,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShaderError {
    Parse {
        source_name: String,
        message: String,
    },
    Validation {
        source_name: String,
        message: String,
    },
    MissingEntryPoint {
        source_name: String,
        name: String,
        stage: ShaderStage,
    },
    DuplicateEntryPoint {
        source_name: String,
        name: String,
        stage: ShaderStage,
    },
    MissingBinding {
        source_name: String,
        group: u32,
        binding: u32,
    },
    DuplicateBinding {
        source_name: String,
        group: u32,
        binding: u32,
    },
}

impl Display for ShaderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                source_name,
                message,
            } => write!(formatter, "WGSL parse failed for {source_name}: {message}"),
            Self::Validation {
                source_name,
                message,
            } => write!(
                formatter,
                "WGSL validation failed for {source_name}: {message}"
            ),
            Self::MissingEntryPoint {
                source_name,
                name,
                stage,
            } => write!(
                formatter,
                "{source_name} is missing {stage:?} entry point {name}"
            ),
            Self::DuplicateEntryPoint {
                source_name,
                name,
                stage,
            } => write!(
                formatter,
                "{source_name} declares {stage:?} entry point {name} twice"
            ),
            Self::MissingBinding {
                source_name,
                group,
                binding,
            } => write!(
                formatter,
                "{source_name} is missing expected binding group {group}, binding {binding}"
            ),
            Self::DuplicateBinding {
                source_name,
                group,
                binding,
            } => write!(
                formatter,
                "{source_name} declares binding group {group}, binding {binding} twice"
            ),
        }
    }
}

impl Error for ShaderError {}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SHADER: &str = r"
        struct VertexOutput {
            @builtin(position) position: vec4<f32>,
        };

        @group(0) @binding(0)
        var<uniform> camera: mat4x4<f32>;

        @vertex
        fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
            var output: VertexOutput;
            output.position = camera * vec4<f32>(f32(vertex_index), 0.0, 0.0, 1.0);
            return output;
        }

        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0);
        }
    ";

    #[test]
    fn shader_ids_are_stable() {
        assert_eq!(
            ShaderId::from_name("clear.wgsl"),
            ShaderId::from_name("clear.wgsl")
        );
        assert_ne!(
            ShaderId::from_name("clear.wgsl"),
            ShaderId::from_name("pbr.wgsl")
        );
    }

    #[test]
    fn valid_manifest_returns_entry_points_and_bindings() {
        let manifest = ShaderManifest::new("clear.wgsl", VALID_SHADER)
            .with_entry_point(ShaderEntryPoint::new("vs_main", ShaderStage::Vertex))
            .with_entry_point(ShaderEntryPoint::new("fs_main", ShaderStage::Fragment))
            .with_binding(ShaderBinding::new(0, 0, "camera"))
            .with_variant(ShaderVariant::new("default").with_define("DEBUG", "0"));

        let reflection = manifest.validate().expect("WGSL should validate");

        assert_eq!(reflection.entry_points.len(), 2);
        assert_eq!(
            reflection.bindings,
            [ReflectedBinding {
                group: 0,
                binding: 0,
                name: "camera".to_owned(),
            }]
        );
    }

    #[test]
    fn invalid_wgsl_is_rejected_before_reflection() {
        let manifest = ShaderManifest::new("broken.wgsl", "@vertex fn broken(");

        assert!(matches!(
            manifest.validate(),
            Err(ShaderError::Parse { .. })
        ));
    }

    #[test]
    fn missing_manifest_entry_point_is_actionable() {
        let manifest = ShaderManifest::new("clear.wgsl", VALID_SHADER)
            .with_entry_point(ShaderEntryPoint::new("missing", ShaderStage::Vertex));

        assert!(matches!(
            manifest.validate(),
            Err(ShaderError::MissingEntryPoint { name, .. }) if name == "missing"
        ));
    }
}
