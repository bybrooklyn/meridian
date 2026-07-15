//! Backend-neutral perspective camera and frustum math.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Row-major 4x4 matrix used at the renderer boundary.
pub type Matrix4 = [[f32; 4]; 4];

/// Perspective projection parameters using a zero-to-one depth range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerspectiveProjection {
    vertical_fov_radians: f32,
    aspect_ratio: f32,
    near_plane: f32,
    far_plane: f32,
}

impl PerspectiveProjection {
    /// Creates a perspective projection with validated depth bounds.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::InvalidProjection`] when the field of view,
    /// aspect ratio, or near/far planes are non-finite or out of range.
    pub fn new(
        vertical_fov_radians: f32,
        aspect_ratio: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Result<Self, CameraError> {
        if !vertical_fov_radians.is_finite()
            || vertical_fov_radians <= 0.0
            || vertical_fov_radians >= std::f32::consts::PI
        {
            return Err(CameraError::InvalidProjection("vertical field of view"));
        }
        if !aspect_ratio.is_finite() || aspect_ratio <= 0.0 {
            return Err(CameraError::InvalidProjection("aspect ratio"));
        }
        if !near_plane.is_finite() || near_plane <= 0.0 {
            return Err(CameraError::InvalidProjection("near plane"));
        }
        if !far_plane.is_finite() || far_plane <= near_plane {
            return Err(CameraError::InvalidProjection("far plane"));
        }
        Ok(Self {
            vertical_fov_radians,
            aspect_ratio,
            near_plane,
            far_plane,
        })
    }

    #[must_use]
    pub const fn vertical_fov_radians(self) -> f32 {
        self.vertical_fov_radians
    }

    #[must_use]
    pub const fn aspect_ratio(self) -> f32 {
        self.aspect_ratio
    }

    #[must_use]
    pub const fn near_plane(self) -> f32 {
        self.near_plane
    }

    #[must_use]
    pub const fn far_plane(self) -> f32 {
        self.far_plane
    }

    fn vertical_tangent(self) -> f32 {
        (self.vertical_fov_radians * 0.5).tan()
    }
}

/// Renderer-independent camera pose and perspective/frustum operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    projection: PerspectiveProjection,
    position: [f32; 3],
    forward: [f32; 3],
    up: [f32; 3],
}

impl Camera {
    /// Creates a camera and orthonormalizes its forward/up basis.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::InvalidPose`] when the position is non-finite,
    /// the forward vector is degenerate, or forward and up are parallel.
    pub fn new(
        projection: PerspectiveProjection,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
    ) -> Result<Self, CameraError> {
        let mut camera = Self {
            projection,
            position: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
        };
        camera.set_pose(position, forward, up)?;
        Ok(camera)
    }

    #[must_use]
    pub const fn projection(self) -> PerspectiveProjection {
        self.projection
    }

    #[must_use]
    pub const fn position(self) -> [f32; 3] {
        self.position
    }

    #[must_use]
    pub const fn forward(self) -> [f32; 3] {
        self.forward
    }

    #[must_use]
    pub const fn up(self) -> [f32; 3] {
        self.up
    }

    /// Updates the pose while preserving an orthonormal camera basis.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::InvalidPose`] for non-finite or degenerate
    /// vectors.
    pub fn set_pose(
        &mut self,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
    ) -> Result<(), CameraError> {
        if !position.iter().all(|value| value.is_finite()) {
            return Err(CameraError::InvalidPose("position"));
        }
        let forward = normalize(forward).ok_or(CameraError::InvalidPose("forward"))?;
        let right = normalize(cross(forward, up)).ok_or(CameraError::InvalidPose(
            "forward and up must not be parallel",
        ))?;
        let up = normalize(cross(right, forward)).ok_or(CameraError::InvalidPose("up"))?;
        self.position = position;
        self.forward = forward;
        self.up = up;
        Ok(())
    }

    /// Returns a right-handed view matrix for the current pose.
    #[must_use]
    pub fn view_matrix(self) -> Matrix4 {
        let right = cross(self.forward, self.up);
        [
            [right[0], right[1], right[2], -dot(right, self.position)],
            [
                self.up[0],
                self.up[1],
                self.up[2],
                -dot(self.up, self.position),
            ],
            [
                -self.forward[0],
                -self.forward[1],
                -self.forward[2],
                dot(self.forward, self.position),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    /// Returns a zero-to-one depth-range perspective projection matrix.
    #[must_use]
    pub fn projection_matrix(self) -> Matrix4 {
        let focal_length = 1.0 / self.projection.vertical_tangent();
        let depth = self.projection.far_plane - self.projection.near_plane;
        [
            [focal_length / self.projection.aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, focal_length, 0.0, 0.0],
            [
                0.0,
                0.0,
                -self.projection.far_plane / depth,
                -(self.projection.near_plane * self.projection.far_plane) / depth,
            ],
            [0.0, 0.0, -1.0, 0.0],
        ]
    }

    /// Returns projection multiplied by view in the camera's row-major form.
    #[must_use]
    pub fn view_projection_matrix(self) -> Matrix4 {
        multiply(self.projection_matrix(), self.view_matrix())
    }

    /// Tests a world-space bounding sphere against the camera frustum.
    #[must_use]
    pub fn sphere_visible(self, center: [f32; 3], radius: f32) -> bool {
        if !center.iter().all(|value| value.is_finite()) || !radius.is_finite() {
            return false;
        }
        let radius = radius.max(0.0);
        let relative = subtract(center, self.position);
        let depth = dot(relative, self.forward);
        if depth + radius < self.projection.near_plane || depth - radius > self.projection.far_plane
        {
            return false;
        }
        // A sphere crossing the eye is conservatively retained for culling.
        if depth <= 0.0 {
            return true;
        }
        let right = cross(self.forward, self.up);
        let horizontal = dot(relative, right).abs();
        let vertical = dot(relative, self.up).abs();
        let vertical_tangent = self.projection.vertical_tangent();
        let horizontal_tangent = vertical_tangent * self.projection.aspect_ratio;
        let horizontal_margin = radius * (1.0 + horizontal_tangent * horizontal_tangent).sqrt();
        let vertical_margin = radius * (1.0 + vertical_tangent * vertical_tangent).sqrt();
        horizontal <= depth * horizontal_tangent + horizontal_margin
            && vertical <= depth * vertical_tangent + vertical_margin
    }
}

/// Camera construction or pose validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraError {
    InvalidProjection(&'static str),
    InvalidPose(&'static str),
}

impl Display for CameraError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProjection(field) => {
                write!(formatter, "invalid camera projection field: {field}")
            }
            Self::InvalidPose(field) => write!(formatter, "invalid camera pose field: {field}"),
        }
    }
}

impl Error for CameraError {}

fn normalize(value: [f32; 3]) -> Option<[f32; 3]> {
    if !value.iter().all(|component| component.is_finite()) {
        return None;
    }
    let length_squared = dot(value, value);
    if length_squared <= f32::EPSILON {
        return None;
    }
    let inverse_length = length_squared.sqrt().recip();
    Some([
        value[0] * inverse_length,
        value[1] * inverse_length,
        value[2] * inverse_length,
    ])
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn subtract(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn multiply(left: Matrix4, right: Matrix4) -> Matrix4 {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..4)
                .map(|index| left[row][index] * right[index][column])
                .sum()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection() -> PerspectiveProjection {
        PerspectiveProjection::new(90.0_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0)
            .expect("projection is valid")
    }

    fn camera() -> Camera {
        Camera::new(
            projection(),
            [0.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
        )
        .expect("camera pose is valid")
    }

    #[test]
    fn projection_rejects_invalid_ranges_and_pose_is_orthonormalized() {
        assert!(matches!(
            PerspectiveProjection::new(0.0, 1.0, 0.1, 10.0),
            Err(CameraError::InvalidProjection("vertical field of view"))
        ));
        assert!(matches!(
            PerspectiveProjection::new(1.0, 0.0, 0.1, 10.0),
            Err(CameraError::InvalidProjection("aspect ratio"))
        ));
        let camera = Camera::new(
            projection(),
            [1.0, 2.0, 3.0],
            [0.0, 0.0, -4.0],
            [0.0, 3.0, 0.0],
        )
        .expect("non-unit basis is normalized");
        assert!((dot(camera.forward(), camera.forward()) - 1.0).abs() < 1.0e-5);
        assert!((dot(camera.up(), camera.up()) - 1.0).abs() < 1.0e-5);
        assert!(dot(camera.forward(), camera.up()).abs() < 1.0e-5);
    }

    #[test]
    fn frustum_culling_keeps_forward_spheres_and_rejects_outside_or_behind() {
        let camera = camera();
        assert!(camera.sphere_visible([0.0, 0.0, -5.0], 0.5));
        assert!(camera.sphere_visible([0.0, 0.0, -0.05], 0.1));
        assert!(!camera.sphere_visible([0.0, 0.0, 5.0], 0.5));
        assert!(!camera.sphere_visible([20.0, 0.0, -5.0], 0.5));
        assert!(!camera.sphere_visible([0.0, 0.0, -101.0], 0.5));
    }

    #[test]
    fn view_projection_matrix_is_finite_and_tracks_camera_translation() {
        let origin = camera().view_projection_matrix();
        let mut translated = camera();
        translated
            .set_pose([3.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0])
            .expect("translated pose is valid");
        assert!(origin.iter().flatten().all(|value| value.is_finite()));
        assert!(translated
            .view_projection_matrix()
            .iter()
            .flatten()
            .all(|value| value.is_finite()));
        assert_ne!(origin, translated.view_projection_matrix());
    }
}
