use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_platform::WindowSize;
use meridian_rhi::{
    BufferUsage, ClearColor, FrameOutcome, GpuBuffer, GpuEnvironmentMap, GpuMaterialBindings,
    GpuMaterialTextureBindGroup, GpuRenderPipeline, GpuShadowMap, GpuTexture, GpuUniformBindGroup,
    OffscreenIndexedCaptureDraw, PbrLightingResources, Rhi, RhiError, ShadowDepthDraw,
    TextureFormat, VertexAttribute, VertexFormat, VertexLayout, VertexLayoutError,
};

use crate::{
    EnvironmentLight, GpuMesh, GpuMeshError, LightingError, MaterialHandle, MaterialResource,
    MeshHandle, MeshResource, SunLight,
};

#[derive(Clone, Copy)]
pub struct FoundationMeshDescriptor<'a> {
    pub label: &'a str,
    pub vertex_data: &'a [u8],
    pub indices: &'a [u32],
    pub bounds_radius: f32,
}

/// Reusable direct-PBR/shadow/diffuse-IBL foundation. Not production Forward+.
pub struct PenumbraFoundationRenderer {
    mesh: GpuMesh,
    pipeline: GpuRenderPipeline,
    shadow_map: GpuShadowMap,
    shadow_parameter_buffer: GpuBuffer,
    environment_map: GpuEnvironmentMap,
    environment_parameter_buffer: GpuBuffer,
    material_bindings: GpuMaterialBindings,
    _base_color_texture: GpuTexture,
    _normal_texture: GpuTexture,
    _metallic_roughness_texture: GpuTexture,
    _camera_buffer: GpuBuffer,
    _object_buffer: GpuBuffer,
    _material_buffer: GpuBuffer,
    _lighting_buffer: GpuBuffer,
}

impl PenumbraFoundationRenderer {
    /// Creates reusable foundation resources and submits one shadow-depth pass.
    ///
    /// # Errors
    ///
    /// Returns an error when mesh validation, GPU resource creation, shader/pipeline
    /// construction, uploads, or foundation bind-group creation fails.
    pub fn new(
        rhi: &mut Rhi,
        descriptor: FoundationMeshDescriptor<'_>,
    ) -> Result<Self, FoundationRendererError> {
        let vertex_count = u32::try_from(descriptor.vertex_data.len() / 48)
            .map_err(|_| FoundationRendererError::new("vertex count overflow"))?;
        let index_count = u32::try_from(descriptor.indices.len())
            .map_err(|_| FoundationRendererError::new("index count overflow"))?;
        let mesh = GpuMesh::new(
            rhi,
            descriptor.label,
            MeshResource::new(
                MeshHandle(1),
                vertex_count,
                index_count,
                descriptor.bounds_radius,
            ),
            48,
            descriptor.vertex_data,
            descriptor.indices,
        )?;
        let vertex_layout = VertexLayout::new(
            48,
            [
                VertexAttribute::new(VertexFormat::Float32x3, 0, 0),
                VertexAttribute::new(VertexFormat::Float32x3, 12, 1),
                VertexAttribute::new(VertexFormat::Float32x4, 24, 2),
                VertexAttribute::new(VertexFormat::Float32x2, 40, 3),
            ],
        )?;
        let pipeline = rhi.create_render_pipeline_with_layout(
            "Meridian Penumbra foundation PBR pipeline",
            include_str!("../../../shaders/textured_material_triangle.wgsl"),
            "vs_main",
            "fs_main",
            Some(&vertex_layout),
        )?;
        let (base_color_texture, normal_texture, metallic_roughness_texture, textures) =
            make_material_textures(rhi, &pipeline)?;
        let camera_buffer = rhi.create_buffer(
            "Meridian foundation camera uniform",
            96,
            BufferUsage::Uniform,
        )?;
        let object_buffer = rhi.create_buffer(
            "Meridian foundation object uniform",
            64,
            BufferUsage::Uniform,
        )?;
        rhi.write_buffer(&camera_buffer, 0, &camera_parameter_bytes())?;
        rhi.write_buffer(&object_buffer, 0, &identity_matrix_bytes())?;
        let uniforms = rhi.create_uniform_bind_group(
            "Meridian foundation camera/object bindings",
            &pipeline,
            &camera_buffer,
            &object_buffer,
        )?;
        let shadow_map = submit_shadow_depth(rhi, &mesh, &camera_buffer, &object_buffer)?;
        let (environment_map, environment_parameter_buffer) = make_environment_map(rhi)?;
        let (shadow_parameter_buffer, material_buffer, lighting_buffer, material_bindings) =
            make_material_bindings(
                rhi,
                &pipeline,
                &shadow_map,
                &environment_map,
                &environment_parameter_buffer,
                textures,
                uniforms,
            )?;
        Ok(Self {
            mesh,
            pipeline,
            shadow_map,
            shadow_parameter_buffer,
            environment_map,
            environment_parameter_buffer,
            material_bindings,
            _base_color_texture: base_color_texture,
            _normal_texture: normal_texture,
            _metallic_roughness_texture: metallic_roughness_texture,
            _camera_buffer: camera_buffer,
            _object_buffer: object_buffer,
            _material_buffer: material_buffer,
            _lighting_buffer: lighting_buffer,
        })
    }

    /// Draws and presents the foundation mesh.
    ///
    /// # Errors
    ///
    /// Returns a typed RHI draw, surface, or device error.
    pub fn render_frame(&self, rhi: &mut Rhi, clear: ClearColor) -> Result<FrameOutcome, RhiError> {
        rhi.render_indexed_mesh_with_material_bindings_and_present(
            &self.pipeline,
            self.mesh.vertex_buffer(),
            self.mesh.index_buffer(),
            self.mesh.index_count(),
            &self.material_bindings,
            clear,
        )
    }

    /// Submits the production foundation draw path to a non-readable target.
    ///
    /// # Errors
    ///
    /// Returns a typed RHI draw or device error.
    pub fn submit_structural_validation(
        &self,
        rhi: &mut Rhi,
        clear: ClearColor,
    ) -> Result<(), RhiError> {
        rhi.submit_indexed_mesh_structural_validation(
            &self.pipeline,
            self.mesh.vertex_buffer(),
            self.mesh.index_buffer(),
            self.mesh.index_count(),
            &self.material_bindings,
            clear,
        )
    }

    /// Submits the production foundation draw path to a capturable offscreen target.
    ///
    /// # Errors
    ///
    /// Returns a typed RHI draw, dimension, capture, or device error.
    pub fn submit_offscreen_capture(
        &self,
        rhi: &mut Rhi,
        clear: ClearColor,
        size: WindowSize,
    ) -> Result<(), RhiError> {
        rhi.submit_indexed_mesh_offscreen_capture(&OffscreenIndexedCaptureDraw {
            pipeline: &self.pipeline,
            vertex_buffer: self.mesh.vertex_buffer(),
            index_buffer: self.mesh.index_buffer(),
            index_count: self.mesh.index_count(),
            material_bindings: &self.material_bindings,
            color: clear,
            size,
        })
    }

    #[must_use]
    pub const fn mesh(&self) -> &GpuMesh {
        &self.mesh
    }

    #[must_use]
    pub const fn shadow_map(&self) -> &GpuShadowMap {
        &self.shadow_map
    }

    #[must_use]
    pub const fn shadow_parameter_bytes(&self) -> u64 {
        self.shadow_parameter_buffer.size()
    }

    #[must_use]
    pub const fn environment_map(&self) -> &GpuEnvironmentMap {
        &self.environment_map
    }

    #[must_use]
    pub const fn environment_parameter_bytes(&self) -> u64 {
        self.environment_parameter_buffer.size()
    }
}

fn identity_matrix_bytes() -> [u8; 64] {
    let values = [
        1.0_f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let mut bytes = [0; 64];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn camera_parameter_bytes() -> [u8; 96] {
    let mut bytes = [0; 96];
    bytes[..64].copy_from_slice(&identity_matrix_bytes());
    for (index, value) in [0.0_f32, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0, 0.0]
        .into_iter()
        .enumerate()
    {
        bytes[64 + index * 4..68 + index * 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn material_parameter_bytes(material: MaterialResource) -> [u8; 32] {
    let values = [
        material.base_color()[0],
        material.base_color()[1],
        material.base_color()[2],
        material.base_color()[3],
        material.metallic(),
        material.roughness(),
        0.0,
        0.0,
    ];
    f32_bytes(values)
}

fn lighting_parameter_bytes(sun: SunLight) -> [u8; 32] {
    let direction = sun.direction_to_light();
    let color = sun.color();
    f32_bytes([
        direction[0],
        direction[1],
        direction[2],
        0.0,
        color[0],
        color[1],
        color[2],
        (sun.illuminance_lux() / 100_000.0).clamp(0.0, 4.0),
    ])
}

fn f32_bytes<const N: usize, const B: usize>(values: [f32; N]) -> [u8; B] {
    let mut bytes = [0; B];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn shadow_parameter_bytes() -> [u8; 560] {
    let mut bytes = [0; 560];
    let identity = identity_matrix_bytes();
    for matrix_index in 0..8 {
        let start = matrix_index * identity.len();
        bytes[start..start + identity.len()].copy_from_slice(&identity);
    }
    for (index, value) in [50.0_f32, 100.0, 150.0, 200.0].into_iter().enumerate() {
        let start = 512 + index * 4;
        bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (index, value) in [1024.0_f32, 0.001, 0.002, 4.0].into_iter().enumerate() {
        let start = 544 + index * 4;
        bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn make_environment_map(
    rhi: &Rhi,
) -> Result<(GpuEnvironmentMap, GpuBuffer), FoundationRendererError> {
    let environment_map = rhi.create_environment_map(
        "Meridian foundation diffuse irradiance",
        1,
        1,
        TextureFormat::Rgba8Unorm,
    )?;
    let faces = [
        [48, 42, 36, 255],
        [34, 40, 50, 255],
        [58, 62, 70, 255],
        [20, 24, 30, 255],
        [40, 46, 58, 255],
        [30, 34, 42, 255],
    ];
    for (face_index, face) in faces.iter().enumerate() {
        rhi.write_environment_face(
            &environment_map,
            u8::try_from(face_index)
                .map_err(|_| FoundationRendererError::new("cube face index overflow"))?,
            0,
            face,
            4,
        )?;
    }
    let parameters = rhi.create_buffer(
        "Meridian foundation environment lighting",
        16,
        BufferUsage::Uniform,
    )?;
    let environment = EnvironmentLight::new(0.75)?;
    let mut bytes = [0_u8; 16];
    bytes[..4].copy_from_slice(&environment.diffuse_intensity().to_le_bytes());
    rhi.write_buffer(&parameters, 0, &bytes)?;
    Ok((environment_map, parameters))
}

fn make_material_textures(
    rhi: &Rhi,
    pipeline: &GpuRenderPipeline,
) -> Result<
    (
        GpuTexture,
        GpuTexture,
        GpuTexture,
        GpuMaterialTextureBindGroup,
    ),
    FoundationRendererError,
> {
    let base = rhi.create_texture(
        "Meridian foundation base color",
        WindowSize::new(1, 1),
        1,
        TextureFormat::Rgba8UnormSrgb,
    )?;
    rhi.write_texture(&base, 0, &[255, 255, 255, 255], 4)?;
    let normal = rhi.create_texture(
        "Meridian foundation normal",
        WindowSize::new(1, 1),
        1,
        TextureFormat::Rgba8Unorm,
    )?;
    rhi.write_texture(&normal, 0, &[128, 128, 255, 255], 4)?;
    let material = rhi.create_texture(
        "Meridian foundation metallic roughness",
        WindowSize::new(1, 1),
        1,
        TextureFormat::Rgba8Unorm,
    )?;
    rhi.write_texture(&material, 0, &[0, 153, 25, 255], 4)?;
    let bindings = rhi.create_material_texture_bind_group(
        "Meridian foundation material textures",
        pipeline,
        &base,
        &normal,
        &material,
    )?;
    Ok((base, normal, material, bindings))
}

fn submit_shadow_depth(
    rhi: &mut Rhi,
    mesh: &GpuMesh,
    camera: &GpuBuffer,
    object: &GpuBuffer,
) -> Result<GpuShadowMap, FoundationRendererError> {
    let layout = VertexLayout::new(48, [VertexAttribute::new(VertexFormat::Float32x3, 0, 0)])?;
    let pipeline = rhi.create_shadow_depth_pipeline_with_layout(
        "Meridian foundation shadow pipeline",
        include_str!("../../../shaders/shadow_depth.wgsl"),
        "vs_main",
        Some(&layout),
    )?;
    let shadow_map = rhi.create_shadow_map("Meridian foundation shadow map", 1024, 4)?;
    let uniforms = rhi.create_uniform_bind_group(
        "Meridian foundation shadow uniforms",
        &pipeline,
        camera,
        object,
    )?;
    rhi.render_shadow_depth(&ShadowDepthDraw {
        pipeline: &pipeline,
        shadow_map: &shadow_map,
        cascade_index: 0,
        vertex_buffer: mesh.vertex_buffer(),
        index_buffer: mesh.index_buffer(),
        index_count: mesh.index_count(),
        uniform_bind_group: &uniforms,
    })?;
    Ok(shadow_map)
}

fn make_material_bindings(
    rhi: &Rhi,
    pipeline: &GpuRenderPipeline,
    shadow_map: &GpuShadowMap,
    environment_map: &GpuEnvironmentMap,
    environment_parameters: &GpuBuffer,
    textures: GpuMaterialTextureBindGroup,
    uniforms: GpuUniformBindGroup,
) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer, GpuMaterialBindings), FoundationRendererError> {
    let shadow_parameters = rhi.create_buffer(
        "Meridian foundation shadow parameters",
        560,
        BufferUsage::Uniform,
    )?;
    rhi.write_buffer(&shadow_parameters, 0, &shadow_parameter_bytes())?;
    let material_buffer = rhi.create_buffer(
        "Meridian foundation material parameters",
        32,
        BufferUsage::Uniform,
    )?;
    let material = MaterialResource::new(MaterialHandle(1), [0.8, 0.7, 0.6, 1.0], 0.1, 0.6);
    rhi.write_buffer(&material_buffer, 0, &material_parameter_bytes(material))?;
    let material_parameters = rhi.create_material_parameter_bind_group(
        "Meridian foundation material binding",
        pipeline,
        &material_buffer,
    )?;
    let lighting_buffer =
        rhi.create_buffer("Meridian foundation sun lighting", 32, BufferUsage::Uniform)?;
    let sun = SunLight::new([0.0, 0.0, 1.0], [1.0, 0.9, 0.8], 100_000.0)?;
    rhi.write_buffer(&lighting_buffer, 0, &lighting_parameter_bytes(sun))?;
    let lighting = rhi.create_lighting_shadow_environment_bind_group(
        "Meridian foundation lighting binding",
        pipeline,
        &PbrLightingResources::new(
            &lighting_buffer,
            shadow_map,
            &shadow_parameters,
            environment_map,
            environment_parameters,
        ),
    )?;
    Ok((
        shadow_parameters,
        material_buffer,
        lighting_buffer,
        GpuMaterialBindings::new(textures, uniforms, material_parameters, lighting),
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FoundationRendererError {
    message: String,
}

impl FoundationRendererError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for FoundationRendererError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for FoundationRendererError {}

impl From<RhiError> for FoundationRendererError {
    fn from(error: RhiError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<GpuMeshError> for FoundationRendererError {
    fn from(error: GpuMeshError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<VertexLayoutError> for FoundationRendererError {
    fn from(error: VertexLayoutError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<LightingError> for FoundationRendererError {
    fn from(error: LightingError) -> Self {
        Self::new(error.to_string())
    }
}
