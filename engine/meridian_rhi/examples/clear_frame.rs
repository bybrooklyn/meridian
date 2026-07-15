use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_platform::{run, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent};
use meridian_rhi::{ClearColor, FrameOutcome, GpuTimingOutcome, Rhi, RhiConfig};

struct ClearFrameApplication {
    rhi: Option<Rhi>,
    frame_submitted: bool,
    timing_deadline: Option<Instant>,
    failure: Arc<Mutex<Option<String>>>,
}

impl ClearFrameApplication {
    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        *self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(message.into());
        context.exit();
    }
}

impl PlatformApplication for ClearFrameApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    return;
                };
                match Rhi::new(window, RhiConfig::default()) {
                    Ok(rhi) => {
                        println!(
                            "Meridian RHI: {} via {:?}, {:?}, timestamps {:?}",
                            rhi.capabilities().adapter_name,
                            rhi.capabilities().backend,
                            rhi.surface_format(),
                            rhi.capabilities().timestamp_queries
                        );
                        self.rhi = Some(rhi);
                        context.request_redraw();
                    }
                    Err(error) => self.fail(error.to_string(), context),
                }
            }
            PlatformEvent::Resized(size) | PlatformEvent::ScaleFactorChanged { size, .. } => {
                if let Some(rhi) = &mut self.rhi {
                    rhi.resize(size);
                }
            }
            PlatformEvent::RedrawRequested => {
                let Some(rhi) = &mut self.rhi else {
                    return;
                };
                if self.frame_submitted {
                    if let Some(sample) = rhi.take_pass_timing() {
                        if sample.gpu == GpuTimingOutcome::Measured(Duration::ZERO) {
                            self.fail("zero GPU duration is not trustworthy", context);
                            return;
                        }
                        println!(
                            "Meridian RHI clear frame timing: frame {}, submission {}, pass {}, CPU {:?}, GPU {:?}, diagnostics {:?}",
                            sample.frame_id.get(),
                            sample.submission_id,
                            sample.pass.as_str(),
                            sample.cpu_encode_time,
                            sample.gpu,
                            rhi.timing_diagnostics()
                        );
                        context.exit();
                        return;
                    }
                    if self
                        .timing_deadline
                        .is_some_and(|deadline| Instant::now() >= deadline)
                    {
                        self.fail(
                            "timing readback did not finish within five seconds",
                            context,
                        );
                        return;
                    }
                    context.request_redraw();
                    return;
                }
                match rhi.clear_and_present(ClearColor::default()) {
                    Ok(FrameOutcome::Presented | FrameOutcome::PresentedSuboptimal) => {
                        self.frame_submitted = true;
                        self.timing_deadline = Some(Instant::now() + Duration::from_secs(5));
                        context.request_redraw();
                    }
                    Ok(
                        outcome @ (FrameOutcome::SkippedZeroSize
                        | FrameOutcome::SkippedTimeout
                        | FrameOutcome::SkippedOccluded
                        | FrameOutcome::ReconfiguredOutdated
                        | FrameOutcome::RecreatedLostSurface),
                    ) => {
                        if let Err(error) =
                            rhi.submit_clear_structural_validation(ClearColor::default())
                        {
                            self.fail(error.to_string(), context);
                            return;
                        }
                        println!(
                            "Meridian RHI clear surface outcome {outcome:?}; submitted offscreen structural validation"
                        );
                        self.frame_submitted = true;
                        self.timing_deadline = Some(Instant::now() + Duration::from_secs(5));
                        context.request_redraw();
                    }
                    Ok(FrameOutcome::DeviceLost | FrameOutcome::UnsupportedSurface) => {
                        self.fail("RHI reported an unavailable surface outcome", context);
                    }
                    Err(error) => self.fail(error.to_string(), context),
                }
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
            title: "Meridian RHI Clear Frame".to_owned(),
            ..PlatformConfig::default()
        },
        ClearFrameApplication {
            rhi: None,
            frame_submitted: false,
            timing_deadline: None,
            failure: Arc::clone(&failure),
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
