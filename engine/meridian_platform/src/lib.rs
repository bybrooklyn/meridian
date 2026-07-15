//! Renderer-neutral platform window and application lifecycle.

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

use meridian_input::{winit_adapter, NativeInputEvent};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use winit::window::{Window, WindowId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventLoopMode {
    /// Sleep until the operating system or application requests more work.
    Wait,
    /// Continuously process events. Intended for active game rendering.
    Poll,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformConfig {
    pub title: String,
    pub initial_size: WindowSize,
    pub resizable: bool,
    pub visible: bool,
    pub event_loop_mode: EventLoopMode,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            title: "Meridian Engine".to_owned(),
            initial_size: WindowSize::new(1280, 720),
            resizable: true,
            visible: true,
            event_loop_mode: EventLoopMode::Wait,
        }
    }
}

/// Opaque native window wrapper suitable for later RHI surface creation.
#[derive(Clone)]
pub struct PlatformWindow {
    inner: Arc<Window>,
}

impl PlatformWindow {
    #[must_use]
    pub fn size(&self) -> WindowSize {
        let size = self.inner.inner_size();
        WindowSize::new(size.width, size.height)
    }

    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.inner.scale_factor()
    }

    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    pub fn set_visible(&self, visible: bool) {
        self.inner.set_visible(visible);
    }
}

impl Debug for PlatformWindow {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformWindow")
            .field("size", &self.size())
            .field("scale_factor", &self.scale_factor())
            .finish_non_exhaustive()
    }
}

impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformEvent {
    Resumed,
    Suspended,
    WindowCreated { size: WindowSize, scale_factor: f64 },
    Resized(WindowSize),
    ScaleFactorChanged { scale_factor: f64, size: WindowSize },
    Focused(bool),
    Input(NativeInputEvent),
    RedrawRequested,
    CloseRequested,
    MemoryWarning,
    Exiting,
}

#[derive(Debug, Default)]
struct PlatformControl {
    exit_requested: bool,
    redraw_requested: bool,
}

/// Context supplied with each renderer-neutral platform event.
pub struct PlatformContext<'window> {
    window: Option<&'window PlatformWindow>,
    control: &'window mut PlatformControl,
}

impl PlatformContext<'_> {
    #[must_use]
    pub const fn window(&self) -> Option<&PlatformWindow> {
        self.window
    }

    pub fn request_redraw(&mut self) {
        self.control.redraw_requested = true;
    }

    pub fn exit(&mut self) {
        self.control.exit_requested = true;
    }
}

pub trait PlatformApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>);
}

/// Starts the native event loop and runs `application` until it requests exit.
///
/// # Errors
///
/// Returns [`PlatformError`] if event-loop creation, native-window creation,
/// or event-loop execution fails.
pub fn run<A: PlatformApplication>(
    config: PlatformConfig,
    application: A,
) -> Result<(), PlatformError> {
    let event_loop = EventLoop::new().map_err(|error| {
        PlatformError::new(PlatformErrorKind::EventLoopCreation, error.to_string())
    })?;
    event_loop.set_control_flow(match config.event_loop_mode {
        EventLoopMode::Wait => ControlFlow::Wait,
        EventLoopMode::Poll => ControlFlow::Poll,
    });

    let mut adapter = WinitApplication::new(config, application);
    event_loop
        .run_app(&mut adapter)
        .map_err(|error| PlatformError::new(PlatformErrorKind::EventLoopRun, error.to_string()))?;

    if let Some(error) = adapter.startup_error {
        return Err(error);
    }

    Ok(())
}

struct WinitApplication<A> {
    config: PlatformConfig,
    application: A,
    window: Option<PlatformWindow>,
    native_window_id: Option<WindowId>,
    startup_error: Option<PlatformError>,
}

impl<A: PlatformApplication> WinitApplication<A> {
    fn new(config: PlatformConfig, application: A) -> Self {
        Self {
            config,
            application,
            window: None,
            native_window_id: None,
            startup_error: None,
        }
    }

    fn dispatch(&mut self, event: PlatformEvent, event_loop: &ActiveEventLoop) {
        let mut control = PlatformControl::default();
        let mut context = PlatformContext {
            window: self.window.as_ref(),
            control: &mut control,
        };
        self.application.on_event(event, &mut context);

        if control.redraw_requested {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        if control.exit_requested {
            event_loop.exit();
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(
                f64::from(self.config.initial_size.width),
                f64::from(self.config.initial_size.height),
            ))
            .with_resizable(self.config.resizable)
            .with_visible(self.config.visible);

        match event_loop.create_window(attributes) {
            Ok(window) => {
                self.native_window_id = Some(window.id());
                self.window = Some(PlatformWindow {
                    inner: Arc::new(window),
                });
                let window = self.window.as_ref().expect("window was just stored");
                self.dispatch(
                    PlatformEvent::WindowCreated {
                        size: window.size(),
                        scale_factor: window.scale_factor(),
                    },
                    event_loop,
                );
            }
            Err(error) => {
                self.startup_error = Some(PlatformError::new(
                    PlatformErrorKind::WindowCreation,
                    error.to_string(),
                ));
                event_loop.exit();
            }
        }
    }
}

impl<A: PlatformApplication> ApplicationHandler for WinitApplication<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(PlatformEvent::Resumed, event_loop);
        if self.window.is_none() && self.startup_error.is_none() {
            self.create_window(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.native_window_id != Some(window_id) {
            return;
        }

        let native_input = winit_adapter::translate_window_event(&event);
        let event = match event {
            WindowEvent::CloseRequested => Some(PlatformEvent::CloseRequested),
            WindowEvent::Resized(size) => Some(PlatformEvent::Resized(WindowSize::new(
                size.width,
                size.height,
            ))),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.window
                    .as_ref()
                    .map(|window| PlatformEvent::ScaleFactorChanged {
                        scale_factor,
                        size: window.size(),
                    })
            }
            WindowEvent::Focused(focused) => Some(PlatformEvent::Focused(focused)),
            WindowEvent::RedrawRequested => Some(PlatformEvent::RedrawRequested),
            _ => native_input.map(PlatformEvent::Input),
        };

        if let Some(event) = event {
            self.dispatch(event, event_loop);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(event) = winit_adapter::translate_device_event(&event) {
            self.dispatch(PlatformEvent::Input(event), event_loop);
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(PlatformEvent::Suspended, event_loop);
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(PlatformEvent::MemoryWarning, event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(PlatformEvent::Exiting, event_loop);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformErrorKind {
    EventLoopCreation,
    WindowCreation,
    EventLoopRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformError {
    kind: PlatformErrorKind,
    message: String,
}

impl PlatformError {
    fn new(kind: PlatformErrorKind, message: String) -> Self {
        Self { kind, message }
    }

    #[must_use]
    pub const fn kind(&self) -> PlatformErrorKind {
        self.kind
    }
}

impl Display for PlatformError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for PlatformError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_a_visible_resizable_waiting_window() {
        let config = PlatformConfig::default();

        assert_eq!(config.title, "Meridian Engine");
        assert_eq!(config.initial_size, WindowSize::new(1280, 720));
        assert!(config.resizable);
        assert!(config.visible);
        assert_eq!(config.event_loop_mode, EventLoopMode::Wait);
    }

    #[test]
    fn zero_size_is_detected_for_minimized_surface_handling() {
        assert!(WindowSize::new(0, 720).is_zero());
        assert!(WindowSize::new(1280, 0).is_zero());
        assert!(!WindowSize::new(1280, 720).is_zero());
    }

    #[test]
    fn platform_context_records_redraw_and_exit_requests() {
        let mut control = PlatformControl::default();
        let mut context = PlatformContext {
            window: None,
            control: &mut control,
        };

        context.request_redraw();
        context.exit();

        assert!(control.redraw_requested);
        assert!(control.exit_requested);
    }
}
