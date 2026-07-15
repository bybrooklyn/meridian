//! Backend-neutral mesh and material resource contracts.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{MaterialHandle, MeshHandle, RenderInstance, RenderSnapshot, TextureHandle};

/// Built mesh metadata required before an instance can be rendered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshResource {
    handle: MeshHandle,
    vertex_count: u32,
    index_count: u32,
    bounds_radius: f32,
}

impl MeshResource {
    #[must_use]
    pub const fn new(
        handle: MeshHandle,
        vertex_count: u32,
        index_count: u32,
        bounds_radius: f32,
    ) -> Self {
        Self {
            handle,
            vertex_count,
            index_count,
            bounds_radius,
        }
    }

    #[must_use]
    pub const fn handle(self) -> MeshHandle {
        self.handle
    }

    #[must_use]
    pub const fn vertex_count(self) -> u32 {
        self.vertex_count
    }

    #[must_use]
    pub const fn index_count(self) -> u32 {
        self.index_count
    }

    #[must_use]
    pub const fn bounds_radius(self) -> f32 {
        self.bounds_radius
    }

    fn is_valid(self) -> bool {
        self.vertex_count > 0
            && self.index_count > 0
            && self.bounds_radius.is_finite()
            && self.bounds_radius >= 0.0
    }
}

/// Built material parameters required before an instance can be rendered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialResource {
    handle: MaterialHandle,
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    base_color_texture: Option<TextureHandle>,
    normal_texture: Option<TextureHandle>,
    metallic_roughness_texture: Option<TextureHandle>,
}

impl MaterialResource {
    #[must_use]
    pub const fn new(
        handle: MaterialHandle,
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
    ) -> Self {
        Self {
            handle,
            base_color,
            metallic,
            roughness,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
        }
    }

    #[must_use]
    pub const fn handle(self) -> MaterialHandle {
        self.handle
    }

    #[must_use]
    pub const fn base_color(self) -> [f32; 4] {
        self.base_color
    }

    #[must_use]
    pub const fn metallic(self) -> f32 {
        self.metallic
    }

    #[must_use]
    pub const fn roughness(self) -> f32 {
        self.roughness
    }

    /// Associates optional built textures with the material's PBR channels.
    #[must_use]
    pub const fn with_textures(
        mut self,
        base_color: Option<TextureHandle>,
        normal: Option<TextureHandle>,
        metallic_roughness: Option<TextureHandle>,
    ) -> Self {
        self.base_color_texture = base_color;
        self.normal_texture = normal;
        self.metallic_roughness_texture = metallic_roughness;
        self
    }

    #[must_use]
    pub const fn base_color_texture(self) -> Option<TextureHandle> {
        self.base_color_texture
    }

    #[must_use]
    pub const fn normal_texture(self) -> Option<TextureHandle> {
        self.normal_texture
    }

    #[must_use]
    pub const fn metallic_roughness_texture(self) -> Option<TextureHandle> {
        self.metallic_roughness_texture
    }

    fn is_valid(self) -> bool {
        self.base_color.iter().all(|value| value.is_finite())
            && self
                .base_color
                .iter()
                .all(|value| (0.0..=1.0).contains(value))
            && self.metallic.is_finite()
            && (0.0..=1.0).contains(&self.metallic)
            && self.roughness.is_finite()
            && (0.0..=1.0).contains(&self.roughness)
    }
}

/// Color encoding expected by a built texture resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureColorSpace {
    Linear,
    Srgb,
}

/// Built texture metadata required before a material can reference a texture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureResource {
    handle: TextureHandle,
    width: u32,
    height: u32,
    mip_levels: u16,
    color_space: TextureColorSpace,
    resident_bytes: u64,
    normal_map: bool,
}

impl TextureResource {
    #[must_use]
    pub const fn new(
        handle: TextureHandle,
        width: u32,
        height: u32,
        mip_levels: u16,
        color_space: TextureColorSpace,
        resident_bytes: u64,
        normal_map: bool,
    ) -> Self {
        Self {
            handle,
            width,
            height,
            mip_levels,
            color_space,
            resident_bytes,
            normal_map,
        }
    }

    #[must_use]
    pub const fn handle(self) -> TextureHandle {
        self.handle
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    #[must_use]
    pub const fn mip_levels(self) -> u16 {
        self.mip_levels
    }

    #[must_use]
    pub const fn color_space(self) -> TextureColorSpace {
        self.color_space
    }

    #[must_use]
    pub const fn resident_bytes(self) -> u64 {
        self.resident_bytes
    }

    #[must_use]
    pub const fn is_normal_map(self) -> bool {
        self.normal_map
    }

    fn is_valid(self) -> bool {
        self.width > 0
            && self.height > 0
            && self.mip_levels > 0
            && self.resident_bytes > 0
            && (!self.normal_map || self.color_space == TextureColorSpace::Linear)
    }
}

/// Resource lookup or validation failure at the renderer boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderResourceError {
    DuplicateMesh(MeshHandle),
    DuplicateMaterial(MaterialHandle),
    DuplicateTexture(TextureHandle),
    InvalidMesh(MeshHandle),
    InvalidMaterial(MaterialHandle),
    InvalidTexture(TextureHandle),
    MissingMesh(MeshHandle),
    MissingMaterial(MaterialHandle),
    MissingTexture(TextureHandle),
}

impl Display for RenderResourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMesh(handle) => {
                write!(formatter, "mesh resource already registered: {}", handle.0)
            }
            Self::DuplicateMaterial(handle) => write!(
                formatter,
                "material resource already registered: {}",
                handle.0
            ),
            Self::DuplicateTexture(handle) => {
                write!(
                    formatter,
                    "texture resource already registered: {}",
                    handle.0
                )
            }
            Self::InvalidMesh(handle) => {
                write!(formatter, "mesh resource is invalid: {}", handle.0)
            }
            Self::InvalidMaterial(handle) => {
                write!(formatter, "material resource is invalid: {}", handle.0)
            }
            Self::InvalidTexture(handle) => {
                write!(formatter, "texture resource is invalid: {}", handle.0)
            }
            Self::MissingMesh(handle) => {
                write!(formatter, "mesh resource is missing: {}", handle.0)
            }
            Self::MissingMaterial(handle) => {
                write!(formatter, "material resource is missing: {}", handle.0)
            }
            Self::MissingTexture(handle) => {
                write!(formatter, "texture resource is missing: {}", handle.0)
            }
        }
    }
}

impl Error for RenderResourceError {}

/// Deterministic registry for built mesh and material handles.
#[derive(Debug, Default)]
pub struct RenderResourceRegistry {
    meshes: BTreeMap<MeshHandle, MeshResource>,
    materials: BTreeMap<MaterialHandle, MaterialResource>,
    textures: BTreeMap<TextureHandle, TextureResource>,
}

impl RenderResourceRegistry {
    /// Registers a built mesh after validating its drawable metadata.
    ///
    /// # Errors
    ///
    /// Returns [`RenderResourceError::InvalidMesh`] for unusable geometry or
    /// [`RenderResourceError::DuplicateMesh`] for an existing handle.
    pub fn register_mesh(&mut self, resource: MeshResource) -> Result<(), RenderResourceError> {
        if !resource.is_valid() {
            return Err(RenderResourceError::InvalidMesh(resource.handle));
        }
        if self.meshes.insert(resource.handle, resource).is_some() {
            return Err(RenderResourceError::DuplicateMesh(resource.handle));
        }
        Ok(())
    }

    /// Registers a built material after validating its PBR parameter ranges.
    ///
    /// # Errors
    ///
    /// Returns [`RenderResourceError::InvalidMaterial`] for non-finite or
    /// out-of-range parameters, or [`RenderResourceError::DuplicateMaterial`]
    /// for an existing handle.
    pub fn register_material(
        &mut self,
        resource: MaterialResource,
    ) -> Result<(), RenderResourceError> {
        if !resource.is_valid() {
            return Err(RenderResourceError::InvalidMaterial(resource.handle));
        }
        if self.materials.insert(resource.handle, resource).is_some() {
            return Err(RenderResourceError::DuplicateMaterial(resource.handle));
        }
        Ok(())
    }

    /// Registers built texture metadata after validating dimensions, mip data,
    /// color space, and the normal-map contract.
    ///
    /// # Errors
    ///
    /// Returns [`RenderResourceError::InvalidTexture`] for unusable metadata
    /// or [`RenderResourceError::DuplicateTexture`] for an existing handle.
    pub fn register_texture(
        &mut self,
        resource: TextureResource,
    ) -> Result<(), RenderResourceError> {
        if !resource.is_valid() {
            return Err(RenderResourceError::InvalidTexture(resource.handle));
        }
        if self.textures.insert(resource.handle, resource).is_some() {
            return Err(RenderResourceError::DuplicateTexture(resource.handle));
        }
        Ok(())
    }

    #[must_use]
    pub fn mesh(&self, handle: MeshHandle) -> Option<MeshResource> {
        self.meshes.get(&handle).copied()
    }

    #[must_use]
    pub fn material(&self, handle: MaterialHandle) -> Option<MaterialResource> {
        self.materials.get(&handle).copied()
    }

    #[must_use]
    pub fn texture(&self, handle: TextureHandle) -> Option<TextureResource> {
        self.textures.get(&handle).copied()
    }

    /// Validates that every instance in a snapshot resolves to built resources.
    ///
    /// # Errors
    ///
    /// Returns the first missing mesh or material in deterministic snapshot
    /// order.
    pub fn validate_snapshot(&self, snapshot: &RenderSnapshot) -> Result<(), RenderResourceError> {
        for instance in snapshot.instances() {
            self.validate_instance(instance)?;
        }
        Ok(())
    }

    /// Validates the mesh and material referenced by one instance.
    ///
    /// # Errors
    ///
    /// Returns [`RenderResourceError::MissingMesh`] or
    /// [`RenderResourceError::MissingMaterial`] when a handle is unknown.
    pub fn validate_instance(&self, instance: &RenderInstance) -> Result<(), RenderResourceError> {
        if !self.meshes.contains_key(&instance.mesh) {
            return Err(RenderResourceError::MissingMesh(instance.mesh));
        }
        if !self.materials.contains_key(&instance.material) {
            return Err(RenderResourceError::MissingMaterial(instance.material));
        }
        let material = self.materials[&instance.material];
        for texture in [
            material.base_color_texture,
            material.normal_texture,
            material.metallic_roughness_texture,
        ]
        .into_iter()
        .flatten()
        {
            let resource = self
                .textures
                .get(&texture)
                .ok_or(RenderResourceError::MissingTexture(texture))?;
            if material.normal_texture == Some(texture)
                && (!resource.normal_map || resource.color_space != TextureColorSpace::Linear)
            {
                return Err(RenderResourceError::InvalidMaterial(instance.material));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RenderInstanceId, RenderSnapshotBuilder, TextureHandle, Transform};

    fn mesh(handle: u32) -> MeshResource {
        MeshResource::new(MeshHandle(handle), 3, 3, 1.0)
    }

    fn material(handle: u32) -> MaterialResource {
        MaterialResource::new(MaterialHandle(handle), [0.8, 0.7, 0.6, 1.0], 0.1, 0.6)
    }

    fn texture(handle: u32, normal_map: bool) -> TextureResource {
        TextureResource::new(
            TextureHandle(handle),
            512,
            512,
            10,
            if normal_map {
                TextureColorSpace::Linear
            } else {
                TextureColorSpace::Srgb
            },
            1_048_576,
            normal_map,
        )
    }

    fn snapshot(mesh: MeshHandle, material: MaterialHandle) -> RenderSnapshot {
        let mut builder = RenderSnapshotBuilder::new(1, 1, 0.0);
        builder
            .push(RenderInstance::new(
                RenderInstanceId::new(1),
                Transform::IDENTITY,
                1.0,
                mesh,
                material,
            ))
            .expect("test instance is unique");
        builder.build()
    }

    #[test]
    fn registry_rejects_invalid_and_duplicate_resources() {
        let mut registry = RenderResourceRegistry::default();
        assert_eq!(
            registry.register_mesh(MeshResource::new(MeshHandle(1), 0, 3, 1.0)),
            Err(RenderResourceError::InvalidMesh(MeshHandle(1)))
        );
        registry.register_mesh(mesh(1)).expect("mesh is valid");
        assert_eq!(
            registry.register_mesh(mesh(1)),
            Err(RenderResourceError::DuplicateMesh(MeshHandle(1)))
        );
        registry
            .register_material(material(2))
            .expect("material is valid");
        assert_eq!(
            registry.register_material(MaterialResource::new(
                MaterialHandle(3),
                [1.0, 0.0, 0.0, 1.0],
                1.2,
                0.5,
            )),
            Err(RenderResourceError::InvalidMaterial(MaterialHandle(3)))
        );
        registry
            .register_texture(texture(4, false))
            .expect("texture is valid");
        assert_eq!(
            registry.register_texture(texture(4, false)),
            Err(RenderResourceError::DuplicateTexture(TextureHandle(4)))
        );
        assert_eq!(
            registry.register_texture(TextureResource::new(
                TextureHandle(5),
                0,
                512,
                1,
                TextureColorSpace::Srgb,
                1,
                false,
            )),
            Err(RenderResourceError::InvalidTexture(TextureHandle(5)))
        );
    }

    #[test]
    fn snapshot_validation_requires_built_mesh_and_material_handles() {
        let mut registry = RenderResourceRegistry::default();
        registry.register_mesh(mesh(1)).expect("mesh is valid");
        registry
            .register_material(material(2))
            .expect("material is valid");
        assert!(registry
            .validate_snapshot(&snapshot(MeshHandle(1), MaterialHandle(2)))
            .is_ok());
        assert_eq!(
            registry.validate_snapshot(&snapshot(MeshHandle(9), MaterialHandle(2))),
            Err(RenderResourceError::MissingMesh(MeshHandle(9)))
        );
        assert_eq!(
            registry.validate_snapshot(&snapshot(MeshHandle(1), MaterialHandle(9))),
            Err(RenderResourceError::MissingMaterial(MaterialHandle(9)))
        );
    }

    #[test]
    fn material_texture_references_must_resolve_to_built_metadata() {
        let mut registry = RenderResourceRegistry::default();
        registry.register_mesh(mesh(1)).expect("mesh is valid");
        registry
            .register_texture(texture(4, false))
            .expect("base color texture is valid");
        registry
            .register_texture(texture(5, true))
            .expect("normal texture is valid");
        let textured_material =
            material(2).with_textures(Some(TextureHandle(4)), Some(TextureHandle(5)), None);
        registry
            .register_material(textured_material)
            .expect("material is valid");
        assert!(registry
            .validate_snapshot(&snapshot(MeshHandle(1), MaterialHandle(2)))
            .is_ok());

        let missing = material(3).with_textures(Some(TextureHandle(99)), None, None);
        registry
            .register_material(missing)
            .expect("handles are resolved during snapshot validation");
        assert_eq!(
            registry.validate_snapshot(&snapshot(MeshHandle(1), MaterialHandle(3))),
            Err(RenderResourceError::MissingTexture(TextureHandle(99)))
        );
    }
}
