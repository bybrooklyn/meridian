use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_core::FrameId;
use meridian_platform::{
    run, EventLoopMode, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent,
};
use meridian_renderer::{
    UiDirectFramePlan, UiDirectGpuFrame, UiDirectGpuRenderer, UiDirectImage,
    UiDirectPrepareRequest, UiDirectResourceSet,
};
use meridian_rhi::{
    CaptureOutcome, CaptureRequest, CaptureSource, ClearColor, FrameOutcome, Rhi, RhiConfig,
};
use meridian_ui_core::{UiColor, UiContrast, UiNodeId, UiPoint, UiRect, UiSize};
use meridian_ui_render::{
    DisplayList, DisplayPrimitive, UiBackdropDescriptor, UiClipId, UiCornerRadii,
    UiEffectCapabilities, UiImageHandle, UiLayerId, UiMeshHandle, UiPathCommand, UiStroke,
};

const PRESENTATION_TIMEOUT: Duration = Duration::from_secs(5);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const PRESENT_RETRY_DELAY: Duration = Duration::from_millis(50);

struct DirectUiSmoke {
    failure: Arc<Mutex<Option<String>>>,
    rhi: Option<Rhi>,
    plan: Option<UiDirectFramePlan>,
    gpu: Option<UiDirectGpuFrame>,
    present_attempts: u8,
    visible_outcome: Option<FrameOutcome>,
    presentation_deadline: Option<Instant>,
    capture_deadline: Option<Instant>,
}

impl DirectUiSmoke {
    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(message.into());
        }
        context.exit();
    }

    #[allow(clippy::cast_precision_loss)]
    fn prepare(rhi: &mut Rhi) -> Result<(UiDirectFramePlan, UiDirectGpuFrame), Box<dyn Error>> {
        let identity = rhi.render_identity();
        let viewport = UiSize::new(
            identity.surface_size.width as f32,
            identity.surface_size.height as f32,
        );
        let image = UiImageHandle(1);
        let mesh = UiMeshHandle(2);
        let resources = UiDirectResourceSet::new(1, 1)
            .with_image_descriptor(UiDirectImage::try_solid(image, 4, 4, [192, 150, 78, 255])?)
            .with_mesh(mesh);
        let list = smoke_display_list(viewport, image, mesh);
        let mut renderer = UiDirectGpuRenderer::new(identity);
        let plan = renderer.prepare_frame(UiDirectPrepareRequest {
            display_revision: 1,
            display_list: &list,
            viewport,
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            effects: UiEffectCapabilities {
                backdrop_filtering: true,
            },
            resources: &resources,
        })?;
        let gpu = plan.upload_gpu_frame(rhi)?;
        Ok((plan, gpu))
    }

    #[allow(clippy::cast_precision_loss)]
    fn validate_clear_only_targets(rhi: &mut Rhi) -> Result<(), Box<dyn Error>> {
        let identity = rhi.render_identity();
        let viewport = UiSize::new(
            identity.surface_size.width as f32,
            identity.surface_size.height as f32,
        );
        let bounds = UiRect::new(UiPoint { x: 0.0, y: 0.0 }, viewport);
        let backdrop = DisplayPrimitive::Backdrop {
            node: UiNodeId::new(1),
            descriptor: effect_descriptor(bounds),
        };
        let cases = [
            DisplayList {
                primitives: vec![
                    DisplayPrimitive::BeginLayer {
                        id: UiLayerId(10),
                        opacity: 1.0,
                    },
                    DisplayPrimitive::EndLayer { id: UiLayerId(10) },
                ],
            },
            DisplayList {
                primitives: vec![backdrop.clone()],
            },
            DisplayList {
                primitives: vec![
                    DisplayPrimitive::BeginLayer {
                        id: UiLayerId(11),
                        opacity: 1.0,
                    },
                    backdrop,
                    DisplayPrimitive::EndLayer { id: UiLayerId(11) },
                ],
            },
        ];
        for (index, list) in cases.iter().enumerate() {
            let plan = UiDirectGpuRenderer::new(identity.clone()).prepare_frame(
                UiDirectPrepareRequest {
                    display_revision: u64::try_from(index + 1)?,
                    display_list: list,
                    viewport,
                    scale_factor: 1.0,
                    contrast: UiContrast::Standard,
                    effects: UiEffectCapabilities {
                        backdrop_filtering: true,
                    },
                    resources: &UiDirectResourceSet::default(),
                },
            )?;
            let gpu = plan.upload_gpu_frame(rhi)?;
            gpu.submit_structural_validation(rhi, &plan, ClearColor::default())?;
        }
        Ok(())
    }

    fn initialize(
        &mut self,
        window: meridian_platform::PlatformWindow,
    ) -> Result<(), Box<dyn Error>> {
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        Self::validate_clear_only_targets(&mut rhi)?;
        let (plan, gpu) = Self::prepare(&mut rhi)?;
        rhi.request_capture(CaptureRequest::new(
            FrameId::new(1),
            4096,
            4096,
            64 * 1024 * 1024,
        ))?;
        self.rhi = Some(rhi);
        self.plan = Some(plan);
        self.gpu = Some(gpu);
        self.present_attempts = 0;
        self.visible_outcome = None;
        self.presentation_deadline = Some(Instant::now() + PRESENTATION_TIMEOUT);
        self.capture_deadline = None;
        Ok(())
    }

    fn rebuild(&mut self) -> Result<(), Box<dyn Error>> {
        let rhi = self.rhi.as_mut().ok_or("direct UI RHI is unavailable")?;
        let (plan, gpu) = Self::prepare(rhi)?;
        self.plan = Some(plan);
        self.gpu = Some(gpu);
        Ok(())
    }

    fn report(&self, outcome: FrameOutcome) -> String {
        let plan = self.plan.as_ref().expect("prepared direct UI plan exists");
        format!(
            "outcome {outcome:?}, visible surface presentation, {} batches, {} layers, {} backdrop filters/{} target bytes, {} vertices, {} indices, atlas {}x{}",
            plan.diagnostics().batch_count,
            plan.diagnostics().layer_count,
            plan.diagnostics().backdrop_effect_count,
            plan.diagnostics().layer_target_bytes,
            plan.diagnostics().vertex_count,
            plan.diagnostics().index_count,
            plan.atlas().width,
            plan.atlas().height,
        )
    }

    fn redraw(&mut self, context: &mut PlatformContext<'_>) -> Result<(), Box<dyn Error>> {
        if let Some(outcome) = self.visible_outcome {
            return self.finish_capture(outcome, context);
        }
        let outcome = self
            .gpu
            .as_ref()
            .ok_or("direct UI GPU frame is unavailable")?
            .present(
                self.rhi.as_mut().ok_or("direct UI RHI is unavailable")?,
                self.plan
                    .as_ref()
                    .ok_or("direct UI frame plan is unavailable")?,
                ClearColor {
                    red: 0.002_731_743,
                    green: 0.003_346_536,
                    blue: 0.003_346_536,
                    alpha: 1.0,
                },
            )?;
        self.present_attempts = self.present_attempts.saturating_add(1);
        if outcome.visible() {
            self.visible_outcome = Some(outcome);
            self.presentation_deadline = None;
            self.capture_deadline = Some(Instant::now() + CAPTURE_TIMEOUT);
            return self.finish_capture(outcome, context);
        }
        if matches!(
            outcome,
            FrameOutcome::DeviceLost | FrameOutcome::UnsupportedSurface
        ) {
            return Err(format!("direct UI surface is unavailable: {outcome:?}").into());
        }
        if self
            .presentation_deadline
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            context.request_redraw_after(PRESENT_RETRY_DELAY);
            return Ok(());
        }
        Err(format!(
            "direct UI presentation remained unavailable before its deadline after {} attempts: {outcome:?}",
            self.present_attempts
        )
        .into())
    }

    fn finish_capture(
        &mut self,
        outcome: FrameOutcome,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let capture = self
            .rhi
            .as_mut()
            .ok_or("direct UI RHI is unavailable")?
            .take_capture();
        let Some(capture) = capture else {
            if self
                .capture_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return Err("direct UI pixel readback timed out".into());
            }
            context.request_redraw();
            return Ok(());
        };
        let CaptureOutcome::Captured(frame) = capture else {
            return Err(format!("direct UI pixel readback was not captured: {capture:?}").into());
        };
        let size = self
            .rhi
            .as_ref()
            .ok_or("direct UI RHI is unavailable")?
            .size();
        let expected_bytes = usize::try_from(
            u64::from(size.width)
                .saturating_mul(u64::from(size.height))
                .saturating_mul(4),
        )?;
        if frame.frame_id != FrameId::new(1)
            || frame.width != size.width
            || frame.height != size.height
            || frame.source != CaptureSource::PresentedSurface
            || frame.surface_outcome != Some(outcome)
            || frame.pixels.len() != expected_bytes
        {
            return Err(format!("direct UI pixel readback metadata is invalid: {frame:?}").into());
        }
        let first = frame
            .pixels
            .get(..4)
            .ok_or("direct UI pixel readback has no first pixel")?;
        let varied = frame.pixels.chunks_exact(4).any(|pixel| pixel != first);
        if !varied {
            return Err("direct UI pixel readback is unexpectedly uniform".into());
        }
        println!(
            "Meridian direct UI smoke: {}, captured {}x{} non-uniform RGBA8 sRGB pixels",
            self.report(outcome),
            frame.width,
            frame.height,
        );
        context.exit();
        Ok(())
    }
}

impl PlatformApplication for DirectUiSmoke {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.fail("direct UI smoke window was not available", context);
                    return;
                };
                match self.initialize(window) {
                    Ok(()) => context.request_redraw(),
                    Err(error) => self.fail(error.to_string(), context),
                }
            }
            PlatformEvent::Resized(size) | PlatformEvent::ScaleFactorChanged { size, .. } => {
                if let Some(rhi) = &mut self.rhi {
                    rhi.resize(size);
                    match self.rebuild() {
                        Ok(()) => {
                            self.present_attempts = 0;
                            self.visible_outcome = None;
                            self.presentation_deadline =
                                Some(Instant::now() + PRESENTATION_TIMEOUT);
                            self.capture_deadline = None;
                            context.request_redraw();
                        }
                        Err(error) => self.fail(error.to_string(), context),
                    }
                }
            }
            PlatformEvent::RedrawRequested => {
                if let Err(error) = self.redraw(context) {
                    self.fail(error.to_string(), context);
                }
            }
            PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn smoke_display_list(viewport: UiSize, image: UiImageHandle, mesh: UiMeshHandle) -> DisplayList {
    let margin = viewport
        .width
        .min(viewport.height)
        .mul_add(0.05, 0.0)
        .min(24.0);
    let panel = UiRect::new(
        UiPoint {
            x: margin,
            y: margin,
        },
        UiSize::new(
            (viewport.width - margin * 2.0).max(1.0),
            (viewport.height - margin * 2.0).max(1.0),
        ),
    );
    let full = UiRect::new(UiPoint { x: 0.0, y: 0.0 }, viewport);
    let clip = UiClipId(1);
    let layer_clip = UiClipId(2);
    let outer_layer = UiLayerId(1);
    let inner_layer = UiLayerId(2);
    DisplayList {
        primitives: vec![
            DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: full,
                color: UiColor::background(),
            },
            DisplayPrimitive::Shadow {
                node: UiNodeId::new(2),
                bounds: panel,
                radii: UiCornerRadii::uniform(14.0),
                offset: UiPoint { x: 0.0, y: 8.0 },
                spread: 2.0,
                color: UiColor::rgba(0.0, 0.0, 0.0, 0.45),
            },
            DisplayPrimitive::RoundedRect {
                node: UiNodeId::new(3),
                bounds: panel,
                radii: UiCornerRadii::uniform(14.0),
                color: UiColor::surface(),
            },
            DisplayPrimitive::PushClip {
                id: clip,
                bounds: panel,
                radii: UiCornerRadii::uniform(14.0),
            },
            DisplayPrimitive::BeginLayer {
                id: outer_layer,
                opacity: 0.92,
            },
            DisplayPrimitive::Backdrop {
                node: UiNodeId::new(7),
                descriptor: effect_descriptor(panel),
            },
            DisplayPrimitive::Image {
                node: UiNodeId::new(4),
                bounds: UiRect::new(
                    UiPoint {
                        x: panel.origin.x + 24.0,
                        y: panel.origin.y + 24.0,
                    },
                    UiSize::new(72.0, 72.0),
                ),
                image,
                opacity: 1.0,
            },
            DisplayPrimitive::PushClip {
                id: layer_clip,
                bounds: panel,
                radii: UiCornerRadii::uniform(10.0),
            },
            DisplayPrimitive::BeginLayer {
                id: inner_layer,
                opacity: 0.72,
            },
            DisplayPrimitive::Mesh {
                node: UiNodeId::new(5),
                bounds: UiRect::new(
                    UiPoint {
                        x: panel.origin.x + 112.0,
                        y: panel.origin.y + 24.0,
                    },
                    UiSize::new(72.0, 72.0),
                ),
                mesh,
                tint: UiColor::grass(),
            },
            DisplayPrimitive::EndLayer { id: inner_layer },
            curve_primitive(panel),
            DisplayPrimitive::PopClip { id: layer_clip },
            DisplayPrimitive::EndLayer { id: outer_layer },
            DisplayPrimitive::PopClip { id: clip },
        ],
    }
}

fn effect_descriptor(panel: UiRect) -> UiBackdropDescriptor {
    let bounds = UiRect::new(
        UiPoint {
            x: panel.origin.x + 16.0,
            y: panel.origin.y + 16.0,
        },
        UiSize::new(
            (panel.size.width - 32.0).max(1.0),
            (panel.size.height - 32.0).max(1.0),
        ),
    );
    UiBackdropDescriptor {
        bounds,
        sample_bounds: UiRect::new(
            UiPoint {
                x: bounds.origin.x - 1.0,
                y: bounds.origin.y - 1.0,
            },
            UiSize::new(bounds.size.width + 2.0, bounds.size.height + 2.0),
        ),
        tint: UiColor::rgba(18.0 / 255.0, 21.0 / 255.0, 21.0 / 255.0, 0.72),
        opaque_fallback: UiColor::surface(),
    }
}

fn curve_primitive(panel: UiRect) -> DisplayPrimitive {
    DisplayPrimitive::Path {
        node: UiNodeId::new(6),
        commands: vec![
            UiPathCommand::MoveTo(UiPoint {
                x: panel.origin.x + 24.0,
                y: panel.origin.y + 128.0,
            }),
            UiPathCommand::CubicTo {
                control_a: UiPoint {
                    x: panel.origin.x + 96.0,
                    y: panel.origin.y + 88.0,
                },
                control_b: UiPoint {
                    x: panel.origin.x + 144.0,
                    y: panel.origin.y + 168.0,
                },
                end: UiPoint {
                    x: panel.origin.x + 216.0,
                    y: panel.origin.y + 128.0,
                },
            },
        ],
        fill: None,
        stroke: Some(UiStroke::new(UiColor::amber(), 3.0)),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let failure = Arc::new(Mutex::new(None));
    run(
        PlatformConfig {
            title: "Meridian Direct UI Smoke".to_owned(),
            // A visible native smoke must give the compositor a chance to map
            // the window between bounded redraw attempts. Interactive Meridian
            // remains event-driven; this polling mode belongs only to the
            // short-lived qualification runner.
            event_loop_mode: EventLoopMode::Poll,
            ..PlatformConfig::default()
        },
        DirectUiSmoke {
            failure: Arc::clone(&failure),
            rhi: None,
            plan: None,
            gpu: None,
            present_attempts: 0,
            visible_outcome: None,
            presentation_deadline: None,
            capture_deadline: None,
        },
    )?;
    if let Some(message) = failure
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
    {
        return Err(message.into());
    }
    Ok(())
}
