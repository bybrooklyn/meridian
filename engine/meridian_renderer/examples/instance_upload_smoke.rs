use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_diagnostics::FrameSample;
use meridian_platform::{
    run, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent, WindowSize,
};
use meridian_renderer::{
    EnvironmentLight, GpuInstanceBuffer, GpuMesh, InstanceUploadPlan, MaterialHandle,
    MaterialResource, MeshHandle, MeshResource, RenderInstance, RenderInstanceId,
    RenderSnapshotBuilder, RenderUploadTracker, SunLight, Transform,
};
use meridian_rhi::{
    BufferUsage, ClearColor, FrameOutcome, GpuBuffer, GpuEnvironmentMap, GpuMaterialBindings,
    GpuMaterialTextureBindGroup, GpuRenderPipeline, GpuShadowMap, GpuTexture, GpuUniformBindGroup,
    PbrLightingResources, Rhi, RhiConfig, ShadowDepthDraw, TextureFormat, VertexAttribute,
    VertexFormat, VertexLayout,
};

struct InstanceUploadApplication {
    rhi: Option<Rhi>,
    instance_buffer: Option<GpuInstanceBuffer>,
    mesh: Option<GpuMesh>,
    pipeline: Option<GpuRenderPipeline>,
    shadow_map: Option<GpuShadowMap>,
    shadow_parameter_buffer: Option<GpuBuffer>,
    environment_map: Option<GpuEnvironmentMap>,
    environment_parameter_buffer: Option<GpuBuffer>,
    texture: Option<GpuTexture>,
    normal_texture: Option<GpuTexture>,
    metallic_roughness_texture: Option<GpuTexture>,
    material_bindings: Option<GpuMaterialBindings>,
    camera_buffer: Option<GpuBuffer>,
    object_buffer: Option<GpuBuffer>,
    material_buffer: Option<GpuBuffer>,
    lighting_buffer: Option<GpuBuffer>,
    failure: Arc<Mutex<Option<String>>>,
}

struct BootstrapResources {
    rhi: Rhi,
    instance_buffer: GpuInstanceBuffer,
    mesh: GpuMesh,
    pipeline: GpuRenderPipeline,
    shadow_map: GpuShadowMap,
    shadow_parameter_buffer: GpuBuffer,
    environment_map: GpuEnvironmentMap,
    environment_parameter_buffer: GpuBuffer,
    shadow_resolution: u32,
    shadow_cascade_count: u8,
    texture: GpuTexture,
    normal_texture: GpuTexture,
    metallic_roughness_texture: GpuTexture,
    material_bindings: GpuMaterialBindings,
    camera_buffer: GpuBuffer,
    object_buffer: GpuBuffer,
    material_buffer: GpuBuffer,
    lighting_buffer: GpuBuffer,
    plan: InstanceUploadPlan,
    frame: FrameOutcome,
    gpu_duration: Option<Duration>,
    frame_sample: FrameSample,
}

impl InstanceUploadApplication {
    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        *self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(message.into());
        context.exit();
    }

    fn make_upload_batch() -> Result<meridian_renderer::RenderUploadBatch, Box<dyn Error>> {
        let mut builder = RenderSnapshotBuilder::new(1, 1, 0.5);
        builder.push(RenderInstance::new(
            RenderInstanceId::new(1),
            Transform::from_translation([0.0, 0.0, -2.0]),
            1.0,
            MeshHandle(1),
            MaterialHandle(1),
        ))?;
        let snapshot = builder.build();
        let mut tracker = RenderUploadTracker::default();
        Ok(tracker.diff(&snapshot)?)
    }

    fn make_bootstrap_mesh(rhi: &Rhi) -> Result<GpuMesh, Box<dyn Error>> {
        let vertex_data = [
            -0.5_f32, -0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.5, -0.5, 0.0, 0.0,
            0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0,
            1.0, 0.5, 0.0,
        ]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect::<Vec<_>>();
        Ok(GpuMesh::new(
            rhi,
            "Meridian bootstrap mesh",
            MeshResource::new(MeshHandle(1), 3, 3, 1.0),
            48,
            &vertex_data,
            &[0, 1, 2],
        )?)
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
        bytes[..64].copy_from_slice(&Self::identity_matrix_bytes());
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
        let mut bytes = [0; 32];
        for (index, value) in values.into_iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn lighting_parameter_bytes(sun: SunLight) -> [u8; 32] {
        let direction = sun.direction_to_light();
        let color = sun.color();
        let values = [
            direction[0],
            direction[1],
            direction[2],
            0.0,
            color[0],
            color[1],
            color[2],
            (sun.illuminance_lux() / 100_000.0).clamp(0.0, 4.0),
        ];
        let mut bytes = [0; 32];
        for (index, value) in values.into_iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn shadow_parameter_bytes() -> [u8; 560] {
        let mut bytes = [0; 560];
        let identity = Self::identity_matrix_bytes();
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

    fn environment_parameter_bytes(environment: EnvironmentLight) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[..4].copy_from_slice(&environment.diffuse_intensity().to_le_bytes());
        bytes
    }

    fn make_environment_map(rhi: &Rhi) -> Result<(GpuEnvironmentMap, GpuBuffer), Box<dyn Error>> {
        let environment_map = rhi.create_environment_map(
            "Meridian bootstrap diffuse irradiance",
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
            rhi.write_environment_face(&environment_map, u8::try_from(face_index)?, 0, face, 4)?;
        }
        let environment_parameter_buffer =
            rhi.create_buffer("Meridian environment lighting", 16, BufferUsage::Uniform)?;
        let environment = EnvironmentLight::new(0.75)?;
        rhi.write_buffer(
            &environment_parameter_buffer,
            0,
            &Self::environment_parameter_bytes(environment),
        )?;
        Ok((environment_map, environment_parameter_buffer))
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
        Box<dyn Error>,
    > {
        let texture = rhi.create_texture(
            "Meridian bootstrap texture",
            WindowSize::new(1, 1),
            1,
            TextureFormat::Rgba8UnormSrgb,
        )?;
        rhi.write_texture(&texture, 0, &[255, 255, 255, 255], 4)?;
        let normal_texture = rhi.create_texture(
            "Meridian bootstrap normal texture",
            WindowSize::new(1, 1),
            1,
            TextureFormat::Rgba8Unorm,
        )?;
        rhi.write_texture(&normal_texture, 0, &[128, 128, 255, 255], 4)?;
        let metallic_roughness_texture = rhi.create_texture(
            "Meridian bootstrap metallic roughness texture",
            WindowSize::new(1, 1),
            1,
            TextureFormat::Rgba8Unorm,
        )?;
        rhi.write_texture(&metallic_roughness_texture, 0, &[0, 153, 25, 255], 4)?;
        let bind_group = rhi.create_material_texture_bind_group(
            "Meridian material texture channels",
            pipeline,
            &texture,
            &normal_texture,
            &metallic_roughness_texture,
        )?;
        Ok((
            texture,
            normal_texture,
            metallic_roughness_texture,
            bind_group,
        ))
    }

    fn submit_shadow_depth(
        rhi: &mut Rhi,
        mesh: &GpuMesh,
        camera_buffer: &GpuBuffer,
        object_buffer: &GpuBuffer,
    ) -> Result<GpuShadowMap, Box<dyn Error>> {
        let shadow_vertex_layout =
            VertexLayout::new(48, [VertexAttribute::new(VertexFormat::Float32x3, 0, 0)])?;
        let shadow_pipeline = rhi.create_shadow_depth_pipeline_with_layout(
            "Meridian cascaded shadow depth pipeline",
            include_str!("../../../shaders/shadow_depth.wgsl"),
            "vs_main",
            Some(&shadow_vertex_layout),
        )?;
        let shadow_map = rhi.create_shadow_map("Meridian cascaded shadow map", 1024, 4)?;
        let shadow_uniform_bind_group = rhi.create_uniform_bind_group(
            "Meridian shadow camera/object bindings",
            &shadow_pipeline,
            camera_buffer,
            object_buffer,
        )?;
        let shadow_draw = ShadowDepthDraw {
            pipeline: &shadow_pipeline,
            shadow_map: &shadow_map,
            cascade_index: 0,
            vertex_buffer: mesh.vertex_buffer(),
            index_buffer: mesh.index_buffer(),
            index_count: mesh.index_count(),
            uniform_bind_group: &shadow_uniform_bind_group,
        };
        rhi.render_shadow_depth(&shadow_draw)?;
        Ok(shadow_map)
    }

    fn make_material_bindings(
        rhi: &Rhi,
        pipeline: &GpuRenderPipeline,
        shadow_map: &GpuShadowMap,
        environment_map: &GpuEnvironmentMap,
        environment_parameter_buffer: &GpuBuffer,
        material_texture_bind_group: meridian_rhi::GpuMaterialTextureBindGroup,
        uniform_bind_group: GpuUniformBindGroup,
    ) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer, GpuMaterialBindings), Box<dyn Error>> {
        let shadow_parameter_buffer =
            rhi.create_buffer("Meridian shadow parameters", 560, BufferUsage::Uniform)?;
        rhi.write_buffer(&shadow_parameter_buffer, 0, &Self::shadow_parameter_bytes())?;
        let material_buffer =
            rhi.create_buffer("Meridian material parameters", 32, BufferUsage::Uniform)?;
        let material = MaterialResource::new(MaterialHandle(1), [0.8, 0.7, 0.6, 1.0], 0.1, 0.6);
        rhi.write_buffer(
            &material_buffer,
            0,
            &Self::material_parameter_bytes(material),
        )?;
        let material_parameter_bind_group = rhi.create_material_parameter_bind_group(
            "Meridian material parameter binding",
            pipeline,
            &material_buffer,
        )?;
        let lighting_buffer =
            rhi.create_buffer("Meridian sun lighting", 32, BufferUsage::Uniform)?;
        let sun = SunLight::new([0.0, 0.0, 1.0], [1.0, 0.9, 0.8], 100_000.0)?;
        rhi.write_buffer(&lighting_buffer, 0, &Self::lighting_parameter_bytes(sun))?;
        let lighting_resources = PbrLightingResources::new(
            &lighting_buffer,
            shadow_map,
            &shadow_parameter_buffer,
            environment_map,
            environment_parameter_buffer,
        );
        let lighting_bind_group = rhi.create_lighting_shadow_environment_bind_group(
            "Meridian sun, shadow, and environment binding",
            pipeline,
            &lighting_resources,
        )?;
        let bindings = GpuMaterialBindings::new(
            material_texture_bind_group,
            uniform_bind_group,
            material_parameter_bind_group,
            lighting_bind_group,
        );
        Ok((
            shadow_parameter_buffer,
            material_buffer,
            lighting_buffer,
            bindings,
        ))
    }

    fn initialize(
        window: meridian_platform::PlatformWindow,
    ) -> Result<BootstrapResources, Box<dyn Error>> {
        let sample_start = Instant::now();
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        let mut instance_buffer =
            GpuInstanceBuffer::new(&rhi, "Meridian instance upload smoke", 4)?;
        let plan = instance_buffer.apply(&rhi, &Self::make_upload_batch()?)?;
        let mesh = Self::make_bootstrap_mesh(&rhi)?;
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
            "Meridian textured material triangle pipeline",
            include_str!("../../../shaders/textured_material_triangle.wgsl"),
            "vs_main",
            "fs_main",
            Some(&vertex_layout),
        )?;
        let (texture, normal_texture, metallic_roughness_texture, material_texture_bind_group) =
            Self::make_material_textures(&rhi, &pipeline)?;
        let camera_buffer =
            rhi.create_buffer("Meridian camera uniform", 96, BufferUsage::Uniform)?;
        let object_buffer =
            rhi.create_buffer("Meridian object uniform", 64, BufferUsage::Uniform)?;
        rhi.write_buffer(&camera_buffer, 0, &Self::camera_parameter_bytes())?;
        rhi.write_buffer(&object_buffer, 0, &Self::identity_matrix_bytes())?;
        let uniform_bind_group = rhi.create_uniform_bind_group(
            "Meridian camera/object bindings",
            &pipeline,
            &camera_buffer,
            &object_buffer,
        )?;
        let shadow_map =
            Self::submit_shadow_depth(&mut rhi, &mesh, &camera_buffer, &object_buffer)?;
        let shadow_resolution = shadow_map.resolution();
        let shadow_cascade_count = shadow_map.cascade_count();
        let (environment_map, environment_parameter_buffer) = Self::make_environment_map(&rhi)?;
        let (shadow_parameter_buffer, material_buffer, lighting_buffer, material_bindings) =
            Self::make_material_bindings(
                &rhi,
                &pipeline,
                &shadow_map,
                &environment_map,
                &environment_parameter_buffer,
                material_texture_bind_group,
                uniform_bind_group,
            )?;
        let frame = rhi.render_indexed_mesh_with_material_bindings_and_present(
            &pipeline,
            mesh.vertex_buffer(),
            mesh.index_buffer(),
            mesh.index_count(),
            &material_bindings,
            ClearColor::default(),
        )?;
        let gpu_duration = rhi.take_last_gpu_duration()?;
        let frame_time = sample_start.elapsed();
        let frame_sample = FrameSample::new(frame_time, frame_time).with_gpu_time(gpu_duration);
        Ok(BootstrapResources {
            rhi,
            instance_buffer,
            mesh,
            pipeline,
            shadow_map,
            shadow_parameter_buffer,
            environment_map,
            environment_parameter_buffer,
            shadow_resolution,
            shadow_cascade_count,
            texture,
            normal_texture,
            metallic_roughness_texture,
            material_bindings,
            camera_buffer,
            object_buffer,
            material_buffer,
            lighting_buffer,
            plan,
            frame,
            gpu_duration,
            frame_sample,
        })
    }
}

impl PlatformApplication for InstanceUploadApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.fail("window-created event had no window", context);
                    return;
                };
                let resources = match Self::initialize(window) {
                    Ok(resources) => resources,
                    Err(error) => {
                        self.fail(error.to_string(), context);
                        return;
                    }
                };
                println!(
                    "Meridian renderer smoke: {} writes, {} bytes, slot {:?}, mesh {}v/{}i uploaded, shadow cascade 0/{} submitted at {}x{} with {}B parameters, diffuse irradiance cube {}x{}x6 with {}B parameters, base-color/normal/metallic-roughness channels plus camera/object/material/sun uniforms constructed, surface outcome {:?}, GPU duration {:?}, diagnostic GPU {:?}",
                    resources.plan.writes().len(),
                    resources.instance_buffer.size(),
                    resources.instance_buffer.slot_for(RenderInstanceId::new(1)),
                    resources.mesh.vertex_count(),
                    resources.mesh.index_count(),
                    resources.shadow_cascade_count,
                    resources.shadow_resolution,
                    resources.shadow_resolution,
                    resources.shadow_parameter_buffer.size(),
                    resources.environment_map.face_size(),
                    resources.environment_map.face_size(),
                    resources.environment_parameter_buffer.size(),
                    resources.frame,
                    resources.gpu_duration,
                    resources.frame_sample.gpu_time
                );
                self.rhi = Some(resources.rhi);
                self.instance_buffer = Some(resources.instance_buffer);
                self.mesh = Some(resources.mesh);
                self.pipeline = Some(resources.pipeline);
                self.shadow_map = Some(resources.shadow_map);
                self.shadow_parameter_buffer = Some(resources.shadow_parameter_buffer);
                self.environment_map = Some(resources.environment_map);
                self.environment_parameter_buffer = Some(resources.environment_parameter_buffer);
                self.texture = Some(resources.texture);
                self.normal_texture = Some(resources.normal_texture);
                self.metallic_roughness_texture = Some(resources.metallic_roughness_texture);
                self.material_bindings = Some(resources.material_bindings);
                self.camera_buffer = Some(resources.camera_buffer);
                self.object_buffer = Some(resources.object_buffer);
                self.material_buffer = Some(resources.material_buffer);
                self.lighting_buffer = Some(resources.lighting_buffer);
                context.exit();
            }
            PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let failure = Arc::new(Mutex::new(None));
    run(
        PlatformConfig {
            title: "Meridian Instance Upload Smoke".to_owned(),
            initial_size: WindowSize::new(1280, 720),
            ..PlatformConfig::default()
        },
        InstanceUploadApplication {
            rhi: None,
            instance_buffer: None,
            mesh: None,
            pipeline: None,
            shadow_map: None,
            shadow_parameter_buffer: None,
            environment_map: None,
            environment_parameter_buffer: None,
            texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            material_bindings: None,
            camera_buffer: None,
            object_buffer: None,
            material_buffer: None,
            lighting_buffer: None,
            failure: Arc::clone(&failure),
        },
    )?;

    if let Some(message) = failure
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
    {
        return Err(io::Error::other(message).into());
    }
    Ok(())
}
