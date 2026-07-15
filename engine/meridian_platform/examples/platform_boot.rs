use meridian_platform::{
    run, PlatformApplication, PlatformConfig, PlatformContext, PlatformError, PlatformEvent,
};

struct SmokeApplication;

impl PlatformApplication for SmokeApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { size, scale_factor } => {
                println!(
                    "Meridian platform window: {}x{} at {scale_factor:.2}x",
                    size.width, size.height
                );
                context.request_redraw();
            }
            PlatformEvent::RedrawRequested | PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn main() -> Result<(), PlatformError> {
    run(PlatformConfig::default(), SmokeApplication)
}
