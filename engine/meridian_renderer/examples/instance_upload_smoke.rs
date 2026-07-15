use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_core::FrameId;
use meridian_diagnostics::{FrameSample, GpuTimingStatus};
use meridian_platform::{
    run, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent, WindowSize,
};
use meridian_renderer::{
    FoundationMeshDescriptor, GpuInstanceBuffer, InstanceUploadPlan, MaterialHandle, MeshHandle,
    PenumbraFoundationRenderer, RenderInstance, RenderInstanceId, RenderSnapshotBuilder,
    RenderUploadTracker, Transform,
};
use meridian_rhi::{
    CaptureOutcome, CaptureRequest, ClearColor, FrameOutcome, GpuTimingOutcome, PassTimingSample,
    Rhi, RhiConfig, TimingFrameId,
};

struct InstanceUploadApplication {
    rhi: Option<Rhi>,
    foundation: Option<PenumbraFoundationRenderer>,
    instance_buffer: Option<GpuInstanceBuffer>,
    timing_frame_id: Option<TimingFrameId>,
    timing_samples: Vec<PassTimingSample>,
    capture: Option<CaptureOutcome>,
    frame_time: Option<Duration>,
    deadline: Option<Instant>,
    failure: Arc<Mutex<Option<String>>>,
}

struct BootstrapResources {
    rhi: Rhi,
    foundation: PenumbraFoundationRenderer,
    instance_buffer: GpuInstanceBuffer,
    plan: InstanceUploadPlan,
    frame: FrameOutcome,
    timing_frame_id: TimingFrameId,
    frame_time: Duration,
}

impl InstanceUploadApplication {
    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        *self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(message.into());
        context.exit();
    }

    const fn diagnostic_gpu_timing(outcome: GpuTimingOutcome) -> GpuTimingStatus {
        match outcome {
            GpuTimingOutcome::Measured(duration) => GpuTimingStatus::Measured(duration),
            GpuTimingOutcome::NotRequested => GpuTimingStatus::NotRequested,
            GpuTimingOutcome::UnsupportedCapability => GpuTimingStatus::UnsupportedCapability,
            GpuTimingOutcome::UnsupportedPlatform(_) => GpuTimingStatus::UnsupportedPlatform,
            GpuTimingOutcome::Inconclusive(_) => GpuTimingStatus::Inconclusive,
        }
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
        Ok(RenderUploadTracker::default().diff(&snapshot)?)
    }

    fn bootstrap_mesh_bytes() -> Vec<u8> {
        [
            -0.5_f32, -0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.1, 0.1, 1.0, 0.0, 1.0, 0.5, -0.5, 0.0, 0.0,
            0.0, 1.0, 0.1, 1.0, 0.1, 1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 0.1, 0.1, 1.0,
            1.0, 0.5, 0.0,
        ]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect()
    }

    fn initialize(
        window: meridian_platform::PlatformWindow,
    ) -> Result<BootstrapResources, Box<dyn Error>> {
        let sample_start = Instant::now();
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        let mut instance_buffer =
            GpuInstanceBuffer::new(&rhi, "Meridian instance upload smoke", 4)?;
        let plan = instance_buffer.apply(&rhi, &Self::make_upload_batch()?)?;
        let timing_frame_id = rhi.begin_timing_frame_for(FrameId::new(1))?;
        let vertices = Self::bootstrap_mesh_bytes();
        let foundation = PenumbraFoundationRenderer::new(
            &mut rhi,
            FoundationMeshDescriptor {
                label: "Meridian source-shaped smoke mesh",
                vertex_data: &vertices,
                indices: &[0, 1, 2],
                bounds_radius: 1.0,
            },
        )?;
        rhi.request_capture(CaptureRequest::new(
            FrameId::new(1),
            1920,
            1080,
            16 * 1024 * 1024,
        ))?;
        let frame = foundation.render_frame(&mut rhi, ClearColor::default())?;
        if !frame.visible() {
            foundation.submit_offscreen_capture(
                &mut rhi,
                ClearColor::default(),
                WindowSize::new(64, 64),
            )?;
        }
        rhi.end_timing_frame(timing_frame_id)?;
        Ok(BootstrapResources {
            rhi,
            foundation,
            instance_buffer,
            plan,
            frame,
            timing_frame_id,
            frame_time: sample_start.elapsed(),
        })
    }

    fn handle_window_created(&mut self, context: &mut PlatformContext<'_>) {
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
            "Meridian renderer smoke: {} writes, {} bytes, slot {:?}, mesh {}v/{}i uploaded, shadow cascade 0/{} submitted at {}x{} with {}B parameters, diffuse irradiance cube {}x{}x6 with {}B parameters, PBR pipeline and bind groups 0-3 constructed for base-color/normal/metallic-roughness, camera/object, material, sun/shadow/environment resources, surface outcome {:?}",
            resources.plan.writes().len(),
            resources.instance_buffer.size(),
            resources.instance_buffer.slot_for(RenderInstanceId::new(1)),
            resources.foundation.mesh().vertex_count(),
            resources.foundation.mesh().index_count(),
            resources.foundation.shadow_map().cascade_count(),
            resources.foundation.shadow_map().resolution(),
            resources.foundation.shadow_map().resolution(),
            resources.foundation.shadow_parameter_bytes(),
            resources.foundation.environment_map().face_size(),
            resources.foundation.environment_map().face_size(),
            resources.foundation.environment_parameter_bytes(),
            resources.frame
        );
        self.timing_frame_id = Some(resources.timing_frame_id);
        self.frame_time = Some(resources.frame_time);
        self.deadline = Some(Instant::now() + Duration::from_secs(5));
        self.rhi = Some(resources.rhi);
        self.foundation = Some(resources.foundation);
        self.instance_buffer = Some(resources.instance_buffer);
        context.request_redraw();
    }

    fn finish_if_ready(&mut self, context: &mut PlatformContext<'_>) -> bool {
        let Some(rhi) = self.rhi.as_mut() else {
            return false;
        };
        while let Some(sample) = rhi.take_pass_timing() {
            self.timing_samples.push(sample);
        }
        if self.capture.is_none() {
            self.capture = rhi.take_capture();
        }
        if self.timing_samples.len() < 2 || self.capture.is_none() {
            return false;
        }
        let expected_frame = self.timing_frame_id.expect("timing frame exists");
        if let Some(sample) = self.timing_samples.iter().find(|sample| {
            sample.frame_id != expected_frame
                || sample.runtime_frame_id != Some(FrameId::new(1))
                || sample.gpu == GpuTimingOutcome::Measured(Duration::ZERO)
        }) {
            self.fail(format!("invalid timing sample: {sample:?}"), context);
            return true;
        }
        let shadow = self
            .timing_samples
            .iter()
            .find(|sample| sample.pass.as_str() == "shadow_depth");
        let indexed = self
            .timing_samples
            .iter()
            .find(|sample| sample.pass.as_str() == "indexed_mesh");
        let (Some(shadow), Some(indexed)) = (shadow, indexed) else {
            self.fail("missing correlated shadow/indexed timing", context);
            return true;
        };
        let capture = self.capture.as_ref().expect("capture ready");
        if let CaptureOutcome::Captured(frame) = capture {
            if frame.pixels.len()
                != usize::try_from(u64::from(frame.width) * u64::from(frame.height) * 4)
                    .expect("smoke capture fits")
            {
                self.fail("capture pixel length is invalid", context);
                return true;
            }
        }
        let frame_time = self.frame_time.expect("frame time exists");
        let frame_sample = FrameSample::new(frame_time, frame_time)
            .with_gpu_timing(Self::diagnostic_gpu_timing(indexed.gpu));
        let capture_summary = match capture {
            CaptureOutcome::Captured(frame) => format!(
                "Captured({}x{}, {:?}, {} bytes)",
                frame.width,
                frame.height,
                frame.source,
                frame.pixels.len()
            ),
            CaptureOutcome::UnsupportedCapability { failure, .. } => {
                format!("UnsupportedCapability({failure:?})")
            }
            CaptureOutcome::Inconclusive { failure, .. } => {
                format!("Inconclusive({failure:?})")
            }
        };
        println!(
            "Meridian renderer timings: frame {}, shadow CPU {:?} GPU {:?}, indexed CPU {:?} GPU {:?}, diagnostic GPU {:?}, capture {}, timing diagnostics {:?}, capture diagnostics {:?}",
            expected_frame.get(),
            shadow.cpu_encode_time,
            shadow.gpu,
            indexed.cpu_encode_time,
            indexed.gpu,
            frame_sample.gpu_timing,
            capture_summary,
            rhi.timing_diagnostics(),
            rhi.capture_diagnostics()
        );
        context.exit();
        true
    }

    fn handle_redraw(&mut self, context: &mut PlatformContext<'_>) {
        if self.finish_if_ready(context) {
            return;
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.fail("renderer async evidence timed out", context);
            return;
        }
        context.request_redraw();
    }
}

impl PlatformApplication for InstanceUploadApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => self.handle_window_created(context),
            PlatformEvent::Resized(size) | PlatformEvent::ScaleFactorChanged { size, .. } => {
                if let Some(rhi) = &mut self.rhi {
                    rhi.resize(size);
                }
            }
            PlatformEvent::RedrawRequested => self.handle_redraw(context),
            PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let failure = Arc::new(Mutex::new(None));
    run(
        PlatformConfig {
            title: "Meridian Penumbra Foundation Smoke".to_owned(),
            initial_size: WindowSize::new(1280, 720),
            ..PlatformConfig::default()
        },
        InstanceUploadApplication {
            rhi: None,
            foundation: None,
            instance_buffer: None,
            timing_frame_id: None,
            timing_samples: Vec::new(),
            capture: None,
            frame_time: None,
            deadline: None,
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
