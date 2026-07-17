//! Renderer-neutral platform window and application lifecycle.

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;
use std::time::Instant;

use meridian_core::{MonotonicNs, RuntimeEpoch};
use meridian_input::{winit_adapter, NativeInputEvent};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, Ime, WindowEvent};
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

/// Keyboard modifiers normalized for Meridian application shortcuts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent platform modifier keys are not a state machine.
pub struct PlatformModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
}

impl PlatformModifiers {
    /// Returns whether the platform's primary command modifier is held.
    #[must_use]
    pub const fn primary_command(self) -> bool {
        self.control || self.command
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformEvent {
    Resumed,
    Suspended,
    WindowCreated {
        size: WindowSize,
        scale_factor: f64,
    },
    Resized(WindowSize),
    ScaleFactorChanged {
        scale_factor: f64,
        size: WindowSize,
    },
    Focused(bool),
    ModifiersChanged(PlatformModifiers),
    Input(NativeInputEvent),
    PointerMoved {
        x: f32,
        y: f32,
    },
    TextCommit(String),
    ImePreedit {
        text: String,
        /// Half-open UTF-8 byte range within `text`; `None` hides the cursor.
        cursor: Option<(usize, usize)>,
    },
    ImeCancelled,
    RedrawRequested,
    CloseRequested,
    MemoryWarning,
    Exiting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlatformEventMetadata {
    pub sequence: u64,
    pub monotonic_ns: MonotonicNs,
    pub runtime_epoch: RuntimeEpoch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformEventEnvelope {
    pub metadata: PlatformEventMetadata,
    pub event: PlatformEvent,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LifecycleState {
    #[default]
    Suspended,
    Ready,
    ZeroExtent,
    Occluded,
    RecoveringSurface,
    DeviceLost,
    Exiting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeRebuildAction {
    None,
    ReconfigureSurface,
    RecreateSurface,
    RebuildDevice,
    Exit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceSignal {
    Presented,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    DeviceLost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecycleTransition {
    pub previous: LifecycleState,
    pub current: LifecycleState,
    pub epoch: RuntimeEpoch,
    pub rebuild: RuntimeRebuildAction,
}

/// Renderer-neutral lifecycle owner. Epoch changes invalidate queued domain work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLifecycle {
    state: LifecycleState,
    epoch: RuntimeEpoch,
    last_non_zero_size: Option<WindowSize>,
}

impl RuntimeLifecycle {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: LifecycleState::Suspended,
            epoch: RuntimeEpoch::new(1),
            last_non_zero_size: None,
        }
    }

    #[must_use]
    pub const fn state(self) -> LifecycleState {
        self.state
    }

    #[must_use]
    pub const fn epoch(self) -> RuntimeEpoch {
        self.epoch
    }

    #[must_use]
    pub const fn last_non_zero_size(self) -> Option<WindowSize> {
        self.last_non_zero_size
    }

    pub fn observe_platform(&mut self, event: &PlatformEvent) -> LifecycleTransition {
        let (next, rebuild, invalidates) = match event {
            PlatformEvent::Resumed => (LifecycleState::Ready, RuntimeRebuildAction::None, false),
            PlatformEvent::WindowCreated { size, .. }
            | PlatformEvent::Resized(size)
            | PlatformEvent::ScaleFactorChanged { size, .. } => {
                if size.is_zero() {
                    (LifecycleState::ZeroExtent, RuntimeRebuildAction::None, true)
                } else {
                    self.last_non_zero_size = Some(*size);
                    (
                        LifecycleState::Ready,
                        RuntimeRebuildAction::ReconfigureSurface,
                        self.state == LifecycleState::ZeroExtent,
                    )
                }
            }
            PlatformEvent::Suspended => {
                (LifecycleState::Suspended, RuntimeRebuildAction::None, true)
            }
            PlatformEvent::CloseRequested | PlatformEvent::Exiting => {
                (LifecycleState::Exiting, RuntimeRebuildAction::Exit, true)
            }
            PlatformEvent::Focused(_)
            | PlatformEvent::ModifiersChanged(_)
            | PlatformEvent::Input(_)
            | PlatformEvent::PointerMoved { .. }
            | PlatformEvent::TextCommit(_)
            | PlatformEvent::ImePreedit { .. }
            | PlatformEvent::ImeCancelled
            | PlatformEvent::RedrawRequested
            | PlatformEvent::MemoryWarning => {
                return self.transition(self.state, RuntimeRebuildAction::None, false);
            }
        };
        self.transition(next, rebuild, invalidates)
    }

    pub fn observe_surface(&mut self, signal: SurfaceSignal) -> LifecycleTransition {
        let (next, rebuild, invalidates) = match signal {
            SurfaceSignal::Presented => (LifecycleState::Ready, RuntimeRebuildAction::None, false),
            SurfaceSignal::Timeout => (self.state, RuntimeRebuildAction::None, false),
            SurfaceSignal::Occluded => {
                (LifecycleState::Occluded, RuntimeRebuildAction::None, false)
            }
            SurfaceSignal::Outdated => (
                LifecycleState::RecoveringSurface,
                RuntimeRebuildAction::ReconfigureSurface,
                true,
            ),
            SurfaceSignal::Lost => (
                LifecycleState::RecoveringSurface,
                RuntimeRebuildAction::RecreateSurface,
                true,
            ),
            SurfaceSignal::DeviceLost => (
                LifecycleState::DeviceLost,
                RuntimeRebuildAction::RebuildDevice,
                true,
            ),
        };
        self.transition(next, rebuild, invalidates)
    }

    fn transition(
        &mut self,
        next: LifecycleState,
        rebuild: RuntimeRebuildAction,
        invalidates: bool,
    ) -> LifecycleTransition {
        let previous = self.state;
        self.state = next;
        if invalidates {
            self.epoch = self.epoch.next();
        }
        LifecycleTransition {
            previous,
            current: next,
            epoch: self.epoch,
            rebuild,
        }
    }
}

impl Default for RuntimeLifecycle {
    fn default() -> Self {
        Self::new()
    }
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

    fn on_event_envelope(
        &mut self,
        envelope: PlatformEventEnvelope,
        context: &mut PlatformContext<'_>,
    ) {
        self.on_event(envelope.event, context);
    }

    /// Returns an application-owned terminal failure observed while handling a
    /// platform event.
    ///
    /// Native event loops do not transport application errors themselves. An
    /// application that must exit after an unrecoverable renderer, UI, or
    /// lifecycle error records it here so [`run`] can preserve a failing
    /// process result instead of reporting a clean window close.
    fn terminal_error(&self) -> Option<PlatformError> {
        None
    }
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

    if let Some(error) = adapter.completion_error() {
        return Err(error);
    }
    if let Some(error) = adapter.application.terminal_error() {
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
    started_at: Instant,
    next_event_sequence: u64,
    lifecycle: RuntimeLifecycle,
    initial_redraw_pending: bool,
}

impl<A: PlatformApplication> WinitApplication<A> {
    fn new(config: PlatformConfig, application: A) -> Self {
        Self {
            config,
            application,
            window: None,
            native_window_id: None,
            startup_error: None,
            started_at: Instant::now(),
            next_event_sequence: 1,
            lifecycle: RuntimeLifecycle::new(),
            initial_redraw_pending: false,
        }
    }

    fn completion_error(&self) -> Option<PlatformError> {
        self.startup_error
            .clone()
            .or_else(|| self.application.terminal_error())
    }

    fn dispatch(&mut self, event: PlatformEvent, event_loop: &ActiveEventLoop) {
        let transition = self.lifecycle.observe_platform(&event);
        let sequence = self.next_event_sequence;
        self.next_event_sequence = self.next_event_sequence.wrapping_add(1).max(1);
        let monotonic_ns = u64::try_from(self.started_at.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let mut control = PlatformControl::default();
        let mut context = PlatformContext {
            window: self.window.as_ref(),
            control: &mut control,
        };
        self.application.on_event_envelope(
            PlatformEventEnvelope {
                metadata: PlatformEventMetadata {
                    sequence,
                    monotonic_ns: MonotonicNs::new(monotonic_ns),
                    runtime_epoch: transition.epoch,
                },
                event,
            },
            &mut context,
        );

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
                window.set_ime_allowed(true);
                self.native_window_id = Some(window.id());
                self.window = Some(PlatformWindow {
                    inner: Arc::new(window),
                });
                self.initial_redraw_pending = self.config.visible;
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
            WindowEvent::ModifiersChanged(modifiers) => {
                let modifiers = modifiers.state();
                Some(PlatformEvent::ModifiersChanged(PlatformModifiers {
                    shift: modifiers.shift_key(),
                    control: modifiers.control_key(),
                    alt: modifiers.alt_key(),
                    command: modifiers.super_key(),
                }))
            }
            WindowEvent::CursorMoved { position, .. } => Some(PlatformEvent::PointerMoved {
                x: f64_to_f32(position.x),
                y: f64_to_f32(position.y),
            }),
            WindowEvent::Ime(Ime::Commit(text)) => Some(PlatformEvent::TextCommit(text)),
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                Some(PlatformEvent::ImePreedit { text, cursor })
            }
            WindowEvent::Ime(Ime::Disabled) => Some(PlatformEvent::ImeCancelled),
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.initial_redraw_pending {
            if let Some(window) = &self.window {
                window.request_redraw();
                self.initial_redraw_pending = false;
            }
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(PlatformEvent::Exiting, event_loop);
    }
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    if value.is_finite() {
        value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformErrorKind {
    EventLoopCreation,
    WindowCreation,
    EventLoopRun,
    Application,
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

    /// Creates a typed terminal failure reported by a Meridian-owned native
    /// application adapter.
    #[must_use]
    pub fn application(message: impl Into<String>) -> Self {
        Self::new(PlatformErrorKind::Application, message.into())
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

    struct FailedApplication;

    impl PlatformApplication for FailedApplication {
        fn on_event(&mut self, _event: PlatformEvent, _context: &mut PlatformContext<'_>) {}

        fn terminal_error(&self) -> Option<PlatformError> {
            Some(PlatformError::application("synthetic application failure"))
        }
    }

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

    #[test]
    fn application_failures_remain_typed_platform_errors() {
        let error = PlatformError::application("renderer initialization failed");

        assert_eq!(error.kind(), PlatformErrorKind::Application);
        assert_eq!(
            error.to_string(),
            "Application: renderer initialization failed"
        );
    }

    #[test]
    fn application_terminal_failure_is_returned_after_the_event_loop() {
        let adapter = WinitApplication::new(PlatformConfig::default(), FailedApplication);

        assert_eq!(
            adapter
                .completion_error()
                .expect("failure is retained")
                .kind(),
            PlatformErrorKind::Application
        );
    }

    #[test]
    fn native_adapter_starts_without_a_redraw_before_window_creation() {
        let adapter = WinitApplication::new(PlatformConfig::default(), FailedApplication);

        assert!(!adapter.initial_redraw_pending);
    }

    #[test]
    fn lifecycle_covers_resize_suspend_surface_recovery_and_device_loss() {
        let mut lifecycle = RuntimeLifecycle::new();
        let initial_epoch = lifecycle.epoch();
        lifecycle.observe_platform(&PlatformEvent::Resumed);
        lifecycle.observe_platform(&PlatformEvent::Resized(WindowSize::new(1280, 720)));
        assert_eq!(lifecycle.state(), LifecycleState::Ready);
        assert_eq!(
            lifecycle.last_non_zero_size(),
            Some(WindowSize::new(1280, 720))
        );
        let focused_epoch = lifecycle.epoch();
        lifecycle.observe_platform(&PlatformEvent::Focused(false));
        assert_eq!(lifecycle.state(), LifecycleState::Ready);
        assert_eq!(lifecycle.epoch(), focused_epoch);

        let timeout = lifecycle.observe_surface(SurfaceSignal::Timeout);
        assert_eq!(timeout.current, LifecycleState::Ready);
        assert_eq!(timeout.epoch, focused_epoch);
        assert_eq!(
            lifecycle.observe_surface(SurfaceSignal::Occluded).current,
            LifecycleState::Occluded
        );
        lifecycle.observe_platform(&PlatformEvent::Suspended);
        assert_eq!(lifecycle.state(), LifecycleState::Suspended);
        lifecycle.observe_platform(&PlatformEvent::Resumed);
        assert_eq!(lifecycle.state(), LifecycleState::Ready);

        let minimized = lifecycle.observe_platform(&PlatformEvent::Resized(WindowSize::new(0, 0)));
        assert_eq!(minimized.current, LifecycleState::ZeroExtent);
        assert!(minimized.epoch > initial_epoch);
        let restored =
            lifecycle.observe_platform(&PlatformEvent::Resized(WindowSize::new(800, 600)));
        assert_eq!(restored.rebuild, RuntimeRebuildAction::ReconfigureSurface);
        assert_eq!(restored.current, LifecycleState::Ready);

        assert_eq!(
            lifecycle.observe_surface(SurfaceSignal::Outdated).rebuild,
            RuntimeRebuildAction::ReconfigureSurface
        );
        assert_eq!(
            lifecycle.observe_surface(SurfaceSignal::Lost).rebuild,
            RuntimeRebuildAction::RecreateSurface
        );
        assert_eq!(
            lifecycle.observe_surface(SurfaceSignal::DeviceLost).rebuild,
            RuntimeRebuildAction::RebuildDevice
        );
        assert_eq!(lifecycle.state(), LifecycleState::DeviceLost);

        let exiting = lifecycle.observe_platform(&PlatformEvent::Exiting);
        assert_eq!(exiting.current, LifecycleState::Exiting);
        assert_eq!(exiting.rebuild, RuntimeRebuildAction::Exit);
    }
}
