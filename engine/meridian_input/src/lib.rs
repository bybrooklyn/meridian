//! Backend-neutral action mapping and per-frame input state.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Action {
    Move,
    Look,
    Crouch,
    Interact,
    Flashlight,
    Pause,
    PhotoMode,
    DocumentBack,
    MenuNavigate,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum KeyCode {
    W,
    A,
    S,
    D,
    E,
    F,
    P,
    Tab,
    Escape,
    Backspace,
    LeftShift,
    LeftControl,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    Start,
    Back,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ButtonControl {
    Key(KeyCode),
    Mouse(MouseButton),
    Gamepad(GamepadButton),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AxisControl {
    MouseX,
    MouseY,
    Gamepad(GamepadAxis),
}

/// Renderer-neutral event emitted by a native input adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NativeInputEvent {
    Button { control: ButtonControl, down: bool },
    MouseMotion { x: f32, y: f32 },
    FocusLost,
}

/// Backend-neutral event normalized from one active gamepad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GamepadInputEvent {
    Button { button: GamepadButton, down: bool },
    Axis { axis: GamepadAxis, value: f32 },
    Disconnected,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis2 {
    pub x: f32,
    pub y: f32,
}

impl Axis2 {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    #[must_use]
    pub const fn clamped(self) -> Self {
        Self {
            x: self.x.clamp(-1.0, 1.0),
            y: self.y.clamp(-1.0, 1.0),
        }
    }
}

impl Default for Axis2 {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputBinding {
    Button {
        control: ButtonControl,
        contribution: Axis2,
    },
    Axis {
        control: AxisControl,
        contribution: Axis2,
    },
}

impl InputBinding {
    #[must_use]
    pub const fn button(control: ButtonControl, contribution: Axis2) -> Self {
        Self::Button {
            control,
            contribution,
        }
    }

    #[must_use]
    pub const fn axis(control: AxisControl, contribution: Axis2) -> Self {
        Self::Axis {
            control,
            contribution,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InputActionMap {
    bindings: BTreeMap<Action, Vec<InputBinding>>,
    dead_zone: f32,
}

impl InputActionMap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
            dead_zone: 0.15,
        }
    }

    #[must_use]
    pub fn with_dead_zone(mut self, dead_zone: f32) -> Self {
        self.dead_zone = dead_zone.clamp(0.0, 1.0);
        self
    }

    pub fn bind(&mut self, action: Action, binding: InputBinding) {
        self.bindings.entry(action).or_default().push(binding);
    }

    #[must_use]
    pub fn with_binding(mut self, action: Action, binding: InputBinding) -> Self {
        self.bind(action, binding);
        self
    }

    /// Returns the engine's initial keyboard/mouse/gamepad action map.
    #[must_use]
    pub fn default_gameplay() -> Self {
        let mut map = Self::new();
        let positive_x = Axis2::new(1.0, 0.0);
        let negative_x = Axis2::new(-1.0, 0.0);
        let positive_y = Axis2::new(0.0, 1.0);
        let negative_y = Axis2::new(0.0, -1.0);
        map.bind(
            Action::Move,
            InputBinding::button(ButtonControl::Key(KeyCode::W), positive_y),
        );
        map.bind(
            Action::Move,
            InputBinding::button(ButtonControl::Key(KeyCode::S), negative_y),
        );
        map.bind(
            Action::Move,
            InputBinding::button(ButtonControl::Key(KeyCode::A), negative_x),
        );
        map.bind(
            Action::Move,
            InputBinding::button(ButtonControl::Key(KeyCode::D), positive_x),
        );
        map.bind(
            Action::Move,
            InputBinding::axis(AxisControl::Gamepad(GamepadAxis::LeftStickX), positive_x),
        );
        map.bind(
            Action::Move,
            InputBinding::axis(AxisControl::Gamepad(GamepadAxis::LeftStickY), positive_y),
        );
        map.bind(
            Action::Look,
            InputBinding::axis(AxisControl::MouseX, positive_x),
        );
        map.bind(
            Action::Look,
            InputBinding::axis(AxisControl::MouseY, positive_y),
        );
        map.bind(
            Action::Look,
            InputBinding::axis(AxisControl::Gamepad(GamepadAxis::RightStickX), positive_x),
        );
        map.bind(
            Action::Look,
            InputBinding::axis(AxisControl::Gamepad(GamepadAxis::RightStickY), positive_y),
        );
        map.bind(
            Action::Crouch,
            InputBinding::button(ButtonControl::Key(KeyCode::LeftControl), positive_x),
        );
        map.bind(
            Action::Interact,
            InputBinding::button(ButtonControl::Key(KeyCode::E), positive_x),
        );
        map.bind(
            Action::Flashlight,
            InputBinding::button(ButtonControl::Key(KeyCode::F), positive_x),
        );
        map.bind(
            Action::Pause,
            InputBinding::button(ButtonControl::Key(KeyCode::Escape), positive_x),
        );
        map.bind(
            Action::PhotoMode,
            InputBinding::button(ButtonControl::Key(KeyCode::P), positive_x),
        );
        map.bind(
            Action::DocumentBack,
            InputBinding::button(ButtonControl::Key(KeyCode::Backspace), positive_x),
        );
        map.bind(
            Action::MenuNavigate,
            InputBinding::button(ButtonControl::Key(KeyCode::Tab), positive_x),
        );
        map
    }
}

impl Default for InputActionMap {
    fn default() -> Self {
        Self::default_gameplay()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionState {
    pub axis: Axis2,
    pub down: bool,
    pub pressed: bool,
    pub released: bool,
}

impl ActionState {
    #[must_use]
    pub const fn idle() -> Self {
        Self {
            axis: Axis2::new(0.0, 0.0),
            down: false,
            pressed: false,
            released: false,
        }
    }
}

pub struct InputState {
    map: InputActionMap,
    current_buttons: BTreeMap<ButtonControl, bool>,
    previous_buttons: BTreeMap<ButtonControl, bool>,
    current_axes: BTreeMap<AxisControl, f32>,
    previous_axes: BTreeMap<AxisControl, f32>,
}

impl InputState {
    #[must_use]
    pub fn new(map: InputActionMap) -> Self {
        Self {
            map,
            current_buttons: BTreeMap::new(),
            previous_buttons: BTreeMap::new(),
            current_axes: BTreeMap::new(),
            previous_axes: BTreeMap::new(),
        }
    }

    /// Begins an input frame, preserving the previous raw state for edge detection.
    pub fn begin_frame(&mut self) {
        self.previous_buttons = self.current_buttons.clone();
        self.previous_axes = self.current_axes.clone();
        self.current_axes.insert(AxisControl::MouseX, 0.0);
        self.current_axes.insert(AxisControl::MouseY, 0.0);
    }

    pub fn set_button(&mut self, control: ButtonControl, down: bool) {
        self.current_buttons.insert(control, down);
    }

    pub fn set_axis(&mut self, control: AxisControl, value: f32) {
        self.current_axes.insert(control, value.clamp(-1.0, 1.0));
    }

    /// Applies one renderer-neutral native event to the current frame.
    pub fn apply_native_event(&mut self, event: NativeInputEvent) {
        match event {
            NativeInputEvent::Button { control, down } => self.set_button(control, down),
            NativeInputEvent::MouseMotion { x, y } => {
                self.add_axis(AxisControl::MouseX, x);
                self.add_axis(AxisControl::MouseY, y);
            }
            NativeInputEvent::FocusLost => self.clear_all(),
        }
    }

    /// Applies one normalized gamepad event to the current frame.
    pub fn apply_gamepad_event(&mut self, event: GamepadInputEvent) {
        match event {
            GamepadInputEvent::Button { button, down } => {
                self.set_button(ButtonControl::Gamepad(button), down);
            }
            GamepadInputEvent::Axis { axis, value } => {
                self.set_axis(AxisControl::Gamepad(axis), value);
            }
            GamepadInputEvent::Disconnected => self.clear_gamepad(),
        }
    }

    pub fn clear_all(&mut self) {
        self.current_buttons.clear();
        self.current_axes.clear();
    }

    fn clear_gamepad(&mut self) {
        self.current_buttons
            .retain(|control, _| !matches!(control, ButtonControl::Gamepad(_)));
        self.current_axes
            .retain(|control, _| !matches!(control, AxisControl::Gamepad(_)));
    }

    #[must_use]
    pub fn action_state(&self, action: Action) -> ActionState {
        let current = self.evaluate(action, &self.current_buttons, &self.current_axes);
        let previous = self.evaluate(action, &self.previous_buttons, &self.previous_axes);
        ActionState {
            axis: current.axis,
            down: current.down,
            pressed: current.down && !previous.down,
            released: !current.down && previous.down,
        }
    }

    fn evaluate(
        &self,
        action: Action,
        buttons: &BTreeMap<ButtonControl, bool>,
        axes: &BTreeMap<AxisControl, f32>,
    ) -> EvaluatedAction {
        let mut evaluated = EvaluatedAction::default();
        if let Some(bindings) = self.map.bindings.get(&action) {
            for binding in bindings {
                let (value, active) = match binding {
                    InputBinding::Button { control, .. } => {
                        let down = buttons.get(control).copied().unwrap_or(false);
                        (f32::from(down), down)
                    }
                    InputBinding::Axis { control, .. } => {
                        let value = axes.get(control).copied().unwrap_or_default();
                        (value, value.abs() > self.map.dead_zone)
                    }
                };
                let contribution = match binding {
                    InputBinding::Button { contribution, .. }
                    | InputBinding::Axis { contribution, .. } => *contribution,
                };
                if active {
                    evaluated.down = true;
                }
                evaluated.axis.x += value * contribution.x;
                evaluated.axis.y += value * contribution.y;
            }
        }
        evaluated.axis = evaluated.axis.clamped();
        evaluated
    }

    fn add_axis(&mut self, control: AxisControl, value: f32) {
        let current = self.current_axes.get(&control).copied().unwrap_or_default();
        self.set_axis(control, current + value);
    }
}

/// Native `winit` translation kept in an integration module.
pub mod winit_adapter {
    use super::{ButtonControl, KeyCode, MouseButton, NativeInputEvent};
    use winit::event::{DeviceEvent, ElementState, MouseButton as WinitMouseButton, WindowEvent};
    use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

    /// Translates keyboard, mouse-button, and focus events into engine input.
    #[must_use]
    pub fn translate_window_event(event: &WindowEvent) -> Option<NativeInputEvent> {
        match event {
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(key) => map_key_code(key).map(|key| NativeInputEvent::Button {
                    control: ButtonControl::Key(key),
                    down: event.state == ElementState::Pressed,
                }),
                PhysicalKey::Unidentified(_) => None,
            },
            WindowEvent::MouseInput { state, button, .. } => {
                map_mouse_button(*button).map(|button| NativeInputEvent::Button {
                    control: ButtonControl::Mouse(button),
                    down: *state == ElementState::Pressed,
                })
            }
            WindowEvent::Focused(false) => Some(NativeInputEvent::FocusLost),
            _ => None,
        }
    }

    /// Translates raw mouse motion, which is delivered as a device event.
    #[must_use]
    pub fn translate_device_event(event: &DeviceEvent) -> Option<NativeInputEvent> {
        match event {
            DeviceEvent::MouseMotion { delta } => Some(NativeInputEvent::MouseMotion {
                x: f64_to_f32(delta.0),
                y: f64_to_f32(delta.1),
            }),
            _ => None,
        }
    }

    #[must_use]
    pub fn map_key_code(key: WinitKeyCode) -> Option<KeyCode> {
        Some(match key {
            WinitKeyCode::KeyW => KeyCode::W,
            WinitKeyCode::KeyA => KeyCode::A,
            WinitKeyCode::KeyS => KeyCode::S,
            WinitKeyCode::KeyD => KeyCode::D,
            WinitKeyCode::KeyE => KeyCode::E,
            WinitKeyCode::KeyF => KeyCode::F,
            WinitKeyCode::KeyP => KeyCode::P,
            WinitKeyCode::Tab => KeyCode::Tab,
            WinitKeyCode::Escape => KeyCode::Escape,
            WinitKeyCode::Backspace => KeyCode::Backspace,
            WinitKeyCode::ShiftLeft => KeyCode::LeftShift,
            WinitKeyCode::ControlLeft => KeyCode::LeftControl,
            WinitKeyCode::ArrowUp => KeyCode::Up,
            WinitKeyCode::ArrowDown => KeyCode::Down,
            WinitKeyCode::ArrowLeft => KeyCode::Left,
            WinitKeyCode::ArrowRight => KeyCode::Right,
            _ => return None,
        })
    }

    #[must_use]
    pub fn map_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
        match button {
            WinitMouseButton::Left => Some(MouseButton::Left),
            WinitMouseButton::Right => Some(MouseButton::Right),
            WinitMouseButton::Middle => Some(MouseButton::Middle),
            WinitMouseButton::Other(button) => u8::try_from(button).ok().map(MouseButton::Other),
            WinitMouseButton::Back | WinitMouseButton::Forward => None,
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct EvaluatedAction {
    axis: Axis2,
    down: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_action_edges_are_stable_across_frames() {
        let mut input = InputState::new(InputActionMap::default_gameplay());
        let w = ButtonControl::Key(KeyCode::W);

        input.begin_frame();
        input.set_button(w, true);
        let pressed = input.action_state(Action::Move);
        assert_eq!(pressed.axis, Axis2::new(0.0, 1.0));
        assert!(pressed.down);
        assert!(pressed.pressed);
        assert!(!pressed.released);

        input.begin_frame();
        input.set_button(w, true);
        let held = input.action_state(Action::Move);
        assert!(held.down);
        assert!(!held.pressed);
        assert!(!held.released);

        input.begin_frame();
        input.set_button(w, false);
        let released = input.action_state(Action::Move);
        assert!(!released.down);
        assert!(!released.pressed);
        assert!(released.released);
    }

    #[test]
    fn remapping_changes_action_without_hardware_dependencies() {
        let mut map = InputActionMap::new();
        map.bind(
            Action::Interact,
            InputBinding::button(ButtonControl::Key(KeyCode::P), Axis2::new(1.0, 0.0)),
        );
        let mut input = InputState::new(map);
        input.begin_frame();
        input.set_button(ButtonControl::Key(KeyCode::P), true);

        let state = input.action_state(Action::Interact);
        assert!(state.down);
        assert!(state.pressed);
        assert!((state.axis.x - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn analog_dead_zone_and_clamping_are_applied() {
        let map = InputActionMap::new().with_dead_zone(0.2).with_binding(
            Action::Look,
            InputBinding::axis(
                AxisControl::Gamepad(GamepadAxis::RightStickX),
                Axis2::new(2.0, 0.0),
            ),
        );
        let mut input = InputState::new(map);
        let axis = AxisControl::Gamepad(GamepadAxis::RightStickX);

        input.begin_frame();
        input.set_axis(axis, 0.1);
        assert!(!input.action_state(Action::Look).down);

        input.begin_frame();
        input.set_axis(axis, 0.8);
        let state = input.action_state(Action::Look);
        assert!(state.down);
        assert_eq!(state.axis, Axis2::new(1.0, 0.0));
    }

    #[test]
    fn opposing_bindings_cancel_axis_but_remain_active() {
        let mut input = InputState::new(InputActionMap::default_gameplay());
        input.begin_frame();
        input.set_button(ButtonControl::Key(KeyCode::W), true);
        input.set_button(ButtonControl::Key(KeyCode::S), true);

        let state = input.action_state(Action::Move);
        assert!(state.down);
        assert_eq!(state.axis, Axis2::default());
    }

    #[test]
    fn native_events_update_actions_and_focus_loss_releases_buttons() {
        let mut input = InputState::new(InputActionMap::default_gameplay());
        input.begin_frame();
        input.apply_native_event(NativeInputEvent::Button {
            control: ButtonControl::Key(KeyCode::W),
            down: true,
        });
        input.apply_native_event(NativeInputEvent::MouseMotion { x: 2.0, y: -0.5 });

        assert!(input.action_state(Action::Move).down);
        assert_eq!(input.action_state(Action::Look).axis, Axis2::new(1.0, -0.5));

        input.begin_frame();
        input.apply_native_event(NativeInputEvent::FocusLost);
        assert!(input.action_state(Action::Move).released);
    }

    #[test]
    fn winit_adapter_maps_keyboard_mouse_and_raw_motion() {
        use winit::event::{DeviceEvent, MouseButton as WinitMouseButton};
        use winit::keyboard::KeyCode as WinitKeyCode;

        assert_eq!(
            winit_adapter::map_key_code(WinitKeyCode::KeyW),
            Some(KeyCode::W)
        );
        assert_eq!(winit_adapter::map_key_code(WinitKeyCode::Numpad0), None);
        assert_eq!(
            winit_adapter::map_mouse_button(WinitMouseButton::Other(7)),
            Some(MouseButton::Other(7))
        );
        let motion = DeviceEvent::MouseMotion { delta: (2.0, -3.0) };
        assert_eq!(
            winit_adapter::translate_device_event(&motion),
            Some(NativeInputEvent::MouseMotion { x: 2.0, y: -3.0 })
        );
    }

    #[test]
    fn gamepad_events_drive_bindings_and_disconnect_clears_state() {
        let mut map = InputActionMap::new();
        map.bind(
            Action::Interact,
            InputBinding::button(
                ButtonControl::Gamepad(GamepadButton::South),
                Axis2::new(1.0, 0.0),
            ),
        );
        map.bind(
            Action::Look,
            InputBinding::axis(
                AxisControl::Gamepad(GamepadAxis::RightStickX),
                Axis2::new(1.0, 0.0),
            ),
        );
        let mut input = InputState::new(map);
        input.begin_frame();
        input.apply_gamepad_event(GamepadInputEvent::Button {
            button: GamepadButton::South,
            down: true,
        });
        input.apply_gamepad_event(GamepadInputEvent::Axis {
            axis: GamepadAxis::RightStickX,
            value: 0.8,
        });

        assert!(input.action_state(Action::Interact).pressed);
        assert!((input.action_state(Action::Look).axis.x - 0.8).abs() < f32::EPSILON);

        input.begin_frame();
        input.apply_gamepad_event(GamepadInputEvent::Disconnected);
        assert!(input.action_state(Action::Interact).released);
        assert!(!input.action_state(Action::Look).down);
    }
}
