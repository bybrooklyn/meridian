use std::error::Error;
use std::sync::{Arc, Mutex};

use meridian_platform::{run, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent};
use meridian_rhi::{ClearColor, FrameOutcome, Rhi, RhiConfig, RhiError};

struct ClearFrameApplication {
    rhi: Option<Rhi>,
    failure: Arc<Mutex<Option<RhiError>>>,
}

impl ClearFrameApplication {
    fn fail(&mut self, error: RhiError, context: &mut PlatformContext<'_>) {
        *self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error);
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
                    Err(error) => self.fail(error, context),
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
                match rhi.clear_and_present(ClearColor::default()) {
                    Ok(FrameOutcome::Presented | FrameOutcome::PresentedSuboptimal) => {
                        match rhi.take_last_gpu_duration() {
                            Ok(duration) => {
                                println!(
                                    "Meridian RHI clear frame presented, GPU time {duration:?}"
                                );
                                context.exit();
                            }
                            Err(error) => self.fail(error, context),
                        }
                    }
                    Ok(
                        FrameOutcome::SkippedZeroSize
                        | FrameOutcome::SkippedTimeout
                        | FrameOutcome::SkippedOccluded
                        | FrameOutcome::ReconfiguredOutdated
                        | FrameOutcome::RecreatedLostSurface,
                    ) => context.request_redraw(),
                    Err(error) => self.fail(error, context),
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
            failure: Arc::clone(&failure),
        },
    )?;

    if let Some(error) = failure
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
    {
        return Err(Box::new(error));
    }
    Ok(())
}
