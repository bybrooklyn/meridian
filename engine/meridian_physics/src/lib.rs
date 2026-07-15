//! Backend-neutral physics and grounded-controller contracts.
//!
//! Rapier integration will implement collision queries and rigid bodies
//! behind these boundaries. The custom controller remains independent of the
//! physics backend so fixed-step movement and save/restore behavior are stable.

use rapier3d::control::KinematicCharacterController;
use rapier3d::prelude::{
    Collider, ColliderBuilder, ColliderHandle, PhysicsWorld, Pose, QueryFilter, Vector,
};

/// Default simulation step used by the engine's fixed update schedule.
pub const DEFAULT_FIXED_TIMESTEP: f32 = 1.0 / 60.0;
const GROUNDED_PROBE_DISTANCE: f32 = 0.05;
const GROUNDED_CONTACT_EPSILON: f32 = 0.001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerConfig {
    pub forward_speed: f32,
    pub backward_speed: f32,
    pub strafe_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub gravity: f32,
    pub max_slope_degrees: f32,
    pub standing_height: f32,
    pub crouching_height: f32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            forward_speed: 1.8,
            backward_speed: 1.4,
            strafe_speed: 1.6,
            acceleration: 8.0,
            deceleration: 10.0,
            gravity: 18.0,
            max_slope_degrees: 42.0,
            standing_height: 1.7,
            crouching_height: 1.1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControllerInput {
    /// Horizontal movement as `[strafe, forward]`, clamped to a unit circle.
    pub move_axis: [f32; 2],
    pub crouch: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundContact {
    pub grounded: bool,
    pub slope_degrees: f32,
}

impl GroundContact {
    #[must_use]
    pub const fn flat() -> Self {
        Self {
            grounded: true,
            slope_degrees: 0.0,
        }
    }

    #[must_use]
    pub const fn airborne() -> Self {
        Self {
            grounded: false,
            slope_degrees: 0.0,
        }
    }

    #[must_use]
    pub fn is_walkable(self, max_slope_degrees: f32) -> bool {
        self.grounded && self.slope_degrees.is_finite() && self.slope_degrees <= max_slope_degrees
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerState {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub grounded: bool,
    pub crouched: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerOutput {
    pub displacement: [f32; 3],
    pub velocity: [f32; 3],
    pub grounded: bool,
    pub crouched: bool,
    pub camera_height: f32,
}

/// Deterministic fixed-step controller for a heavy, grounded first-person
/// character.
pub struct GroundedController {
    config: ControllerConfig,
    state: ControllerState,
}

impl GroundedController {
    #[must_use]
    pub fn new(config: ControllerConfig, position: [f32; 3]) -> Self {
        Self {
            config,
            state: ControllerState {
                position,
                velocity: [0.0; 3],
                grounded: false,
                crouched: false,
            },
        }
    }

    #[must_use]
    pub const fn state(&self) -> ControllerState {
        self.state
    }

    #[must_use]
    pub const fn config(&self) -> ControllerConfig {
        self.config
    }

    /// Applies an authoritative collision-corrected position from the physics
    /// backend without changing controller velocity.
    pub fn correct_position(&mut self, position: [f32; 3]) {
        self.state.position = position;
    }

    /// Applies the backend's authoritative walkable-ground result.
    pub fn correct_grounded(&mut self, grounded: bool) {
        self.state.grounded = grounded;
        if grounded {
            self.state.velocity[1] = 0.0;
        }
    }

    /// Resets position and velocity for deterministic save/restore.
    pub fn reset(&mut self, position: [f32; 3]) {
        self.state.position = position;
        self.state.velocity = [0.0; 3];
        self.state.grounded = false;
        self.state.crouched = false;
    }

    /// Advances one controller step using contact data from the physics backend.
    ///
    /// The controller does not perform collision detection. It predicts a
    /// displacement, which the backend may correct before the next step.
    /// There is intentionally no jump or sprint input in this contract.
    ///
    /// # Errors
    ///
    /// Returns [`ControllerError::InvalidDeltaTime`] for a non-positive or
    /// non-finite timestep.
    pub fn step(
        &mut self,
        input: ControllerInput,
        contact: GroundContact,
        delta_seconds: f32,
    ) -> Result<ControllerOutput, ControllerError> {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return Err(ControllerError::InvalidDeltaTime);
        }

        let axis = normalized_axis(input.move_axis);
        let target_velocity = [
            axis[0] * self.config.strafe_speed,
            0.0,
            if axis[1] >= 0.0 {
                axis[1] * self.config.forward_speed
            } else {
                axis[1] * self.config.backward_speed
            },
        ];
        let acceleration = if target_velocity[0].abs() + target_velocity[2].abs() > 0.0 {
            self.config.acceleration
        } else {
            self.config.deceleration
        };
        self.state.velocity[0] = approach(
            self.state.velocity[0],
            target_velocity[0],
            acceleration * delta_seconds,
        );
        self.state.velocity[2] = approach(
            self.state.velocity[2],
            target_velocity[2],
            acceleration * delta_seconds,
        );

        self.state.grounded = contact.is_walkable(self.config.max_slope_degrees);
        if self.state.grounded {
            self.state.velocity[1] = 0.0;
        } else {
            self.state.velocity[1] -= self.config.gravity * delta_seconds;
        }
        self.state.crouched = input.crouch;

        let displacement = [
            self.state.velocity[0] * delta_seconds,
            self.state.velocity[1] * delta_seconds,
            self.state.velocity[2] * delta_seconds,
        ];
        self.state.position = add(self.state.position, displacement);
        Ok(ControllerOutput {
            displacement,
            velocity: self.state.velocity,
            grounded: self.state.grounded,
            crouched: self.state.crouched,
            camera_height: if self.state.crouched {
                self.config.crouching_height
            } else {
                self.config.standing_height
            },
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerError {
    InvalidDeltaTime,
}

impl std::fmt::Display for ControllerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeltaTime => {
                formatter.write_str("controller delta time must be finite and positive")
            }
        }
    }
}

impl std::error::Error for ControllerError {}

fn normalized_axis(axis: [f32; 2]) -> [f32; 2] {
    let x = finite_or_zero(axis[0]).clamp(-1.0, 1.0);
    let y = finite_or_zero(axis[1]).clamp(-1.0, 1.0);
    let magnitude_squared = x * x + y * y;
    if magnitude_squared > 1.0 {
        let scale = magnitude_squared.sqrt().recip();
        [x * scale, y * scale]
    } else {
        [x, y]
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

/// Opaque identifier for a collider owned by [`RapierWorld`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PhysicsColliderId(ColliderHandle);

/// Result of moving a kinematic character through the Rapier scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMoveResult {
    pub translation: [f32; 3],
    pub grounded: bool,
    pub sliding_down_slope: bool,
}

/// Small backend-neutral wrapper around the Rapier physics world.
pub struct RapierWorld {
    world: PhysicsWorld,
}

impl RapierWorld {
    #[must_use]
    pub fn new(gravity: [f32; 3]) -> Self {
        let world = PhysicsWorld {
            gravity: Vector::new(gravity[0], gravity[1], gravity[2]),
            ..PhysicsWorld::default()
        };
        Self { world }
    }

    /// Inserts a fixed cuboid collider and returns an opaque engine handle.
    pub fn insert_static_box(
        &mut self,
        center: [f32; 3],
        half_extents: [f32; 3],
    ) -> PhysicsColliderId {
        let collider = ColliderBuilder::cuboid(
            half_extents[0].max(0.001),
            half_extents[1].max(0.001),
            half_extents[2].max(0.001),
        )
        .translation(Vector::new(center[0], center[1], center[2]))
        .build();
        PhysicsColliderId(self.world.insert_collider(collider, None))
    }

    #[must_use]
    pub fn contains_collider(&self, collider: PhysicsColliderId) -> bool {
        self.world.colliders.contains(collider.0)
    }

    pub fn remove_collider(&mut self, collider: PhysicsColliderId) -> bool {
        self.world.remove_collider(collider.0).is_some()
    }

    /// Advances Rapier by one fixed simulation step.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsBackendError::InvalidDeltaTime`] for a non-finite or
    /// non-positive timestep.
    pub fn step(&mut self, delta_seconds: f32) -> Result<(), PhysicsBackendError> {
        validate_delta_time(delta_seconds)?;
        self.world.integration_parameters.dt = delta_seconds;
        self.world.step();
        Ok(())
    }

    /// Computes collision-corrected movement for a kinematic character.
    /// The world is not mutated; the caller owns the resulting position.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsBackendError::InvalidDeltaTime`] for a non-finite or
    /// non-positive timestep.
    pub fn move_character(
        &self,
        character: &RapierCharacter,
        position: [f32; 3],
        desired_translation: [f32; 3],
        delta_seconds: f32,
    ) -> Result<PhysicsMoveResult, PhysicsBackendError> {
        validate_delta_time(delta_seconds)?;
        let query_pipeline = self.world.broad_phase.as_query_pipeline(
            self.world.narrow_phase.query_dispatcher(),
            &self.world.bodies,
            &self.world.colliders,
            QueryFilter::default(),
        );
        let effective = character.controller.move_shape(
            delta_seconds,
            &query_pipeline,
            character.shape.shape(),
            &Pose::from_translation(Vector::new(position[0], position[1], position[2])),
            Vector::new(
                desired_translation[0],
                desired_translation[1],
                desired_translation[2],
            ),
            |_| {},
        );
        Ok(PhysicsMoveResult {
            translation: [
                effective.translation.x,
                effective.translation.y,
                effective.translation.z,
            ],
            grounded: effective.grounded,
            sliding_down_slope: effective.is_sliding_down_slope,
        })
    }

    /// Runs the engine controller prediction through Rapier and applies the
    /// collision-corrected position back to the controller.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsBackendError::InvalidDeltaTime`] for a non-finite or
    /// non-positive timestep.
    pub fn step_grounded_controller(
        &self,
        controller: &mut GroundedController,
        character: &RapierCharacter,
        input: ControllerInput,
        delta_seconds: f32,
    ) -> Result<ControllerOutput, PhysicsBackendError> {
        validate_delta_time(delta_seconds)?;
        let before = controller.state();
        let contact = if before.grounded {
            GroundContact::flat()
        } else {
            GroundContact::airborne()
        };
        let predicted = controller
            .step(input, contact, delta_seconds)
            .map_err(|_| PhysicsBackendError::InvalidDeltaTime)?;
        let effective = self.move_character(
            character,
            before.position,
            predicted.displacement,
            delta_seconds,
        )?;
        let mut corrected_translation = effective.translation;
        let blocked_downward_motion = predicted.displacement[1] < 0.0
            && effective.translation[1] > predicted.displacement[1] + GROUNDED_CONTACT_EPSILON;
        let mut walkable_grounded =
            (effective.grounded || blocked_downward_motion) && !is_downward_slide(effective);
        if before.grounded
            && !walkable_grounded
            && !is_downward_slide(effective)
            && predicted.displacement[1] >= 0.0
        {
            let probe = self.move_character(
                character,
                add(before.position, effective.translation),
                [0.0, -GROUNDED_PROBE_DISTANCE, 0.0],
                delta_seconds,
            )?;
            corrected_translation = add(corrected_translation, probe.translation);
            let probe_blocked = probe.translation[1] >= -GROUNDED_CONTACT_EPSILON;
            walkable_grounded = probe.grounded || probe_blocked;
        }
        controller.correct_position(add(before.position, corrected_translation));
        controller.correct_grounded(walkable_grounded);

        let mut velocity = predicted.velocity;
        if walkable_grounded {
            velocity[1] = 0.0;
        }
        Ok(ControllerOutput {
            displacement: corrected_translation,
            velocity,
            grounded: walkable_grounded,
            crouched: predicted.crouched,
            camera_height: predicted.camera_height,
        })
    }
}

impl Default for RapierWorld {
    fn default() -> Self {
        Self::new([0.0, -9.81, 0.0])
    }
}

/// Rapier-backed kinematic capsule configured from the engine controller contract.
pub struct RapierCharacter {
    shape: Collider,
    controller: KinematicCharacterController,
}

impl RapierCharacter {
    #[must_use]
    pub fn new(config: ControllerConfig, radius: f32) -> Self {
        let radius = radius.max(0.01);
        let half_height = (config.standing_height * 0.5 - radius).max(0.01);
        Self {
            shape: ColliderBuilder::capsule_y(half_height, radius).build(),
            controller: KinematicCharacterController {
                max_slope_climb_angle: config.max_slope_degrees.to_radians(),
                min_slope_slide_angle: std::f32::consts::FRAC_PI_2,
                ..KinematicCharacterController::default()
            },
        }
    }
}

/// Engine-owned floor-and-controller harness for deterministic service tests.
///
/// This scene contains no consumer-game content. It exists to exercise the
/// fixed-step controller, collision correction, and reset boundary together.
pub struct GroundedControllerTestScene {
    world: RapierWorld,
    character: RapierCharacter,
    controller: GroundedController,
    ground: PhysicsColliderId,
}

impl GroundedControllerTestScene {
    /// Creates a large flat floor and places the controller just above it.
    ///
    /// # Panics
    ///
    /// Panics only if the fixed, valid initialization timestep is rejected by
    /// the backend.
    #[must_use]
    pub fn new(config: ControllerConfig) -> Self {
        let mut world = RapierWorld::default();
        let ground = world.insert_static_box([0.0, -0.5, 0.0], [50.0, 0.5, 50.0]);
        world
            .step(DEFAULT_FIXED_TIMESTEP)
            .expect("test scene floor broad phase initializes");
        let mut controller = GroundedController::new(config, [0.0, 0.86, 0.0]);
        controller.correct_grounded(true);
        Self {
            world,
            character: RapierCharacter::new(config, 0.3),
            controller,
            ground,
        }
    }

    #[must_use]
    pub const fn ground_collider(&self) -> PhysicsColliderId {
        self.ground
    }

    #[must_use]
    pub const fn state(&self) -> ControllerState {
        self.controller.state()
    }

    /// Advances the scene through one fixed controller/physics step.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsBackendError::InvalidDeltaTime`] when the timestep is
    /// not finite and positive.
    pub fn step(
        &mut self,
        input: ControllerInput,
        delta_seconds: f32,
    ) -> Result<ControllerOutput, PhysicsBackendError> {
        self.world.step_grounded_controller(
            &mut self.controller,
            &self.character,
            input,
            delta_seconds,
        )
    }

    /// Resets the controller while retaining the test floor and character.
    pub fn reset(&mut self, position: [f32; 3]) {
        self.controller.reset(position);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicsBackendError {
    InvalidDeltaTime,
}

impl std::fmt::Display for PhysicsBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeltaTime => {
                formatter.write_str("physics delta time must be finite and positive")
            }
        }
    }
}

impl std::error::Error for PhysicsBackendError {}

fn validate_delta_time(delta_seconds: f32) -> Result<(), PhysicsBackendError> {
    if delta_seconds.is_finite() && delta_seconds > 0.0 {
        Ok(())
    } else {
        Err(PhysicsBackendError::InvalidDeltaTime)
    }
}

fn is_downward_slide(movement: PhysicsMoveResult) -> bool {
    movement.sliding_down_slope && movement.translation[1] < -GROUNDED_CONTACT_EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> GroundedController {
        GroundedController::new(ControllerConfig::default(), [0.0, 0.0, 0.0])
    }

    #[test]
    fn fixed_step_movement_accelerates_and_crouch_changes_camera_height() {
        let mut controller = controller();
        let output = controller
            .step(
                ControllerInput {
                    move_axis: [0.0, 1.0],
                    crouch: true,
                },
                GroundContact::flat(),
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid fixed timestep");

        assert!(output.displacement[2] > 0.0);
        assert!((output.displacement[1]).abs() < f32::EPSILON);
        assert!(output.grounded);
        assert!(output.crouched);
        assert!((output.camera_height - 1.1).abs() < f32::EPSILON);
    }

    #[test]
    fn diagonal_input_is_clamped_and_steep_ground_is_not_walkable() {
        let mut controller = controller();
        let output = controller
            .step(
                ControllerInput {
                    move_axis: [1.0, 1.0],
                    crouch: false,
                },
                GroundContact {
                    grounded: true,
                    slope_degrees: 60.0,
                },
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid fixed timestep");
        let horizontal_speed = (output.velocity[0] * output.velocity[0]
            + output.velocity[2] * output.velocity[2])
            .sqrt();

        assert!(!output.grounded);
        assert!(horizontal_speed < ControllerConfig::default().forward_speed);
    }

    #[test]
    fn airborne_gravity_is_deterministic_and_landing_clears_vertical_velocity() {
        let mut controller = controller();
        let first = controller
            .step(
                ControllerInput::default(),
                GroundContact::airborne(),
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid fixed timestep");
        let second = controller
            .step(
                ControllerInput::default(),
                GroundContact::airborne(),
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid fixed timestep");
        assert!(second.velocity[1] < first.velocity[1]);

        let landed = controller
            .step(
                ControllerInput::default(),
                GroundContact::flat(),
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid fixed timestep");
        assert!(landed.grounded);
        assert!((landed.velocity[1]).abs() < f32::EPSILON);
    }

    #[test]
    fn invalid_delta_time_is_rejected() {
        let mut controller = controller();
        assert_eq!(
            controller.step(ControllerInput::default(), GroundContact::flat(), 0.0),
            Err(ControllerError::InvalidDeltaTime)
        );
        assert_eq!(
            controller.step(ControllerInput::default(), GroundContact::flat(), f32::NAN),
            Err(ControllerError::InvalidDeltaTime)
        );
    }

    #[test]
    fn rapier_character_stops_on_ground_and_collider_handles_are_opaque() {
        let mut world = RapierWorld::default();
        let ground = world.insert_static_box([0.0, -0.5, 0.0], [5.0, 0.5, 5.0]);
        let character = RapierCharacter::new(ControllerConfig::default(), 0.3);
        world
            .step(DEFAULT_FIXED_TIMESTEP)
            .expect("scene broad phase updates");

        let movement = world
            .move_character(
                &character,
                [0.0, 2.0, 0.0],
                [0.0, -3.0, 0.0],
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid character move");

        assert!(movement.grounded);
        assert!(movement.translation[1] < 0.0);
        assert!(movement.translation[1] > -3.0);
        assert!(world.contains_collider(ground));
        assert!(world.remove_collider(ground));
        assert!(!world.contains_collider(ground));
    }

    #[test]
    fn rapier_world_rejects_invalid_step_time() {
        let mut world = RapierWorld::default();
        assert_eq!(world.step(0.0), Err(PhysicsBackendError::InvalidDeltaTime));
    }

    #[test]
    fn rapier_world_applies_grounded_controller_correction() {
        let mut world = RapierWorld::default();
        world.insert_static_box([0.0, -0.5, 0.0], [50.0, 0.5, 50.0]);
        world
            .step(DEFAULT_FIXED_TIMESTEP)
            .expect("scene broad phase updates");
        let config = ControllerConfig::default();
        let character = RapierCharacter::new(config, 0.3);
        let mut controller = GroundedController::new(config, [0.0, 0.86, 0.0]);
        controller.correct_grounded(true);
        let output = world
            .step_grounded_controller(
                &mut controller,
                &character,
                ControllerInput {
                    move_axis: [0.0, 1.0],
                    crouch: false,
                },
                DEFAULT_FIXED_TIMESTEP,
            )
            .expect("valid controller step");

        assert!(output.grounded);
        assert!(controller.state().position[1] > 0.7);
        assert!(controller.state().position[2] > 0.0);
        assert!(controller.state().velocity[1].abs() < f32::EPSILON);
    }

    #[test]
    fn controller_test_scene_moves_for_many_fixed_steps_without_penetrating_floor() {
        let mut scene = GroundedControllerTestScene::new(ControllerConfig::default());
        for _ in 0..120 {
            let output = scene
                .step(
                    ControllerInput {
                        move_axis: [0.0, 1.0],
                        crouch: false,
                    },
                    DEFAULT_FIXED_TIMESTEP,
                )
                .expect("test scene step succeeds");
            assert!(output.grounded);
        }

        let state = scene.state();
        assert!(state.position[2] > 2.0);
        assert!(state.position[1] > 0.7);
        assert!(state.grounded);
        assert!(scene.world.contains_collider(scene.ground_collider()));

        scene.reset([0.0, 2.0, 0.0]);
        let reset_position = scene.state().position;
        assert!(reset_position
            .iter()
            .zip([0.0, 2.0, 0.0])
            .all(|(actual, expected)| (*actual - expected).abs() < f32::EPSILON));
        let airborne = scene
            .step(ControllerInput::default(), DEFAULT_FIXED_TIMESTEP)
            .expect("airborne test step succeeds");
        assert!(!airborne.grounded);
        assert!(scene.state().position[1] < 2.0);
    }
}
