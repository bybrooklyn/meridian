//! Backend-neutral directional-light and cascaded-shadow contracts.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Validated settings for a sun's cascaded raster shadow map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowSettings {
    cascade_count: u8,
    max_distance: f32,
    depth_bias: f32,
    normal_bias: f32,
    texel_world_size: f32,
    stable_texel_snap: bool,
}

impl ShadowSettings {
    /// Creates shadow settings with the distance and bias values used by the
    /// renderer's backend-neutral contract.
    ///
    /// `texel_world_size` controls the world-space snapping grid used to keep
    /// cascades stable while the camera moves.
    ///
    /// # Errors
    ///
    /// Returns [`LightingError`] when a cascade count, distance, bias, or
    /// snapping grid is invalid.
    pub fn new(
        cascade_count: u8,
        max_distance: f32,
        depth_bias: f32,
        normal_bias: f32,
        texel_world_size: f32,
        stable_texel_snap: bool,
    ) -> Result<Self, LightingError> {
        if !(1..=8).contains(&cascade_count) {
            return Err(LightingError::InvalidCascadeCount(cascade_count));
        }
        if !max_distance.is_finite() || max_distance <= 0.0 {
            return Err(LightingError::InvalidShadowDistance);
        }
        if !depth_bias.is_finite() || depth_bias < 0.0 {
            return Err(LightingError::InvalidDepthBias);
        }
        if !normal_bias.is_finite() || normal_bias < 0.0 {
            return Err(LightingError::InvalidNormalBias);
        }
        if !texel_world_size.is_finite() || texel_world_size <= 0.0 {
            return Err(LightingError::InvalidTexelWorldSize);
        }
        Ok(Self {
            cascade_count,
            max_distance,
            depth_bias,
            normal_bias,
            texel_world_size,
            stable_texel_snap,
        })
    }

    #[must_use]
    pub const fn cascade_count(self) -> u8 {
        self.cascade_count
    }

    #[must_use]
    pub const fn max_distance(self) -> f32 {
        self.max_distance
    }

    #[must_use]
    pub const fn depth_bias(self) -> f32 {
        self.depth_bias
    }

    #[must_use]
    pub const fn normal_bias(self) -> f32 {
        self.normal_bias
    }

    #[must_use]
    pub const fn texel_world_size(self) -> f32 {
        self.texel_world_size
    }

    #[must_use]
    pub const fn stable_texel_snap(self) -> bool {
        self.stable_texel_snap
    }

    /// Builds practical-split cascade ranges for a camera near plane.
    ///
    /// The split scheme blends logarithmic and linear partitions so the
    /// limited shadow resolution is concentrated near the camera while still
    /// covering the configured maximum distance.
    ///
    /// # Errors
    ///
    /// Returns [`LightingError::InvalidShadowNearPlane`] when the camera near
    /// plane is non-finite, non-positive, or not below the shadow distance.
    pub fn cascade_layout(self, camera_near: f32) -> Result<CascadedShadowLayout, LightingError> {
        CascadedShadowLayout::new(self, camera_near)
    }
}

/// One depth range in a cascaded directional shadow layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowCascade {
    index: u8,
    near_distance: f32,
    far_distance: f32,
    texel_world_size: f32,
}

impl ShadowCascade {
    #[must_use]
    pub const fn index(self) -> u8 {
        self.index
    }

    #[must_use]
    pub const fn near_distance(self) -> f32 {
        self.near_distance
    }

    #[must_use]
    pub const fn far_distance(self) -> f32 {
        self.far_distance
    }

    #[must_use]
    pub const fn texel_world_size(self) -> f32 {
        self.texel_world_size
    }

    #[must_use]
    pub fn contains(self, camera_depth: f32) -> bool {
        camera_depth.is_finite()
            && camera_depth >= self.near_distance
            && camera_depth <= self.far_distance
    }
}

/// Deterministic, practical-split ranges for a sun's cascaded shadow map.
#[derive(Clone, Debug, PartialEq)]
pub struct CascadedShadowLayout {
    cascades: Vec<ShadowCascade>,
}

impl CascadedShadowLayout {
    fn new(settings: ShadowSettings, camera_near: f32) -> Result<Self, LightingError> {
        if !camera_near.is_finite() || camera_near <= 0.0 || camera_near >= settings.max_distance {
            return Err(LightingError::InvalidShadowNearPlane);
        }

        let mut cascades = Vec::with_capacity(usize::from(settings.cascade_count));
        let mut near_distance = camera_near;
        for index in 0..settings.cascade_count {
            let ratio = f32::from(index + 1) / f32::from(settings.cascade_count);
            let logarithmic = camera_near * (settings.max_distance / camera_near).powf(ratio);
            let linear = camera_near + (settings.max_distance - camera_near) * ratio;
            let far_distance = logarithmic.mul_add(0.5, linear * 0.5);
            cascades.push(ShadowCascade {
                index,
                near_distance,
                far_distance,
                texel_world_size: settings.texel_world_size,
            });
            near_distance = far_distance;
        }
        Ok(Self { cascades })
    }

    #[must_use]
    pub fn cascades(&self) -> &[ShadowCascade] {
        &self.cascades
    }

    #[must_use]
    pub fn select(&self, camera_depth: f32) -> Option<ShadowCascade> {
        if !camera_depth.is_finite() {
            return None;
        }
        self.cascades
            .iter()
            .find(|cascade| camera_depth <= cascade.far_distance)
            .copied()
            .or_else(|| self.cascades.last().copied())
    }
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            cascade_count: 4,
            max_distance: 250.0,
            depth_bias: 0.001,
            normal_bias: 0.02,
            texel_world_size: 0.25,
            stable_texel_snap: true,
        }
    }
}

/// Validated dynamic directional sunlight description.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunLight {
    direction_to_light: [f32; 3],
    color: [f32; 3],
    illuminance_lux: f32,
    shadows: Option<ShadowSettings>,
}

impl SunLight {
    /// Creates a sun whose direction points from the shaded world toward the
    /// light source. The direction is normalized at the renderer boundary.
    ///
    /// # Errors
    ///
    /// Returns [`LightingError`] for a degenerate direction, invalid color,
    /// or non-finite/negative illuminance.
    pub fn new(
        direction_to_light: [f32; 3],
        color: [f32; 3],
        illuminance_lux: f32,
    ) -> Result<Self, LightingError> {
        let direction_to_light =
            normalize(direction_to_light).ok_or(LightingError::InvalidDirection)?;
        if !color
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        {
            return Err(LightingError::InvalidColor);
        }
        if !illuminance_lux.is_finite() || illuminance_lux < 0.0 {
            return Err(LightingError::InvalidIlluminance);
        }
        Ok(Self {
            direction_to_light,
            color,
            illuminance_lux,
            shadows: Some(ShadowSettings::default()),
        })
    }

    #[must_use]
    pub const fn direction_to_light(self) -> [f32; 3] {
        self.direction_to_light
    }

    #[must_use]
    pub const fn color(self) -> [f32; 3] {
        self.color
    }

    #[must_use]
    pub const fn illuminance_lux(self) -> f32 {
        self.illuminance_lux
    }

    #[must_use]
    pub const fn shadows(self) -> Option<ShadowSettings> {
        self.shadows
    }

    /// Enables or disables sun shadows without changing light intensity.
    #[must_use]
    pub const fn with_shadows(mut self, shadows: Option<ShadowSettings>) -> Self {
        self.shadows = shadows;
        self
    }
}

/// Intensity control for a pre-convolved diffuse irradiance environment map.
///
/// The environment texture itself is an RHI-owned cube resource; this
/// backend-neutral value controls only its contribution to the PBR ambient
/// term and deliberately excludes specular prefiltering for now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentLight {
    diffuse_intensity: f32,
}

impl EnvironmentLight {
    /// Creates a diffuse environment-light contribution.
    ///
    /// # Errors
    ///
    /// Returns [`LightingError::InvalidEnvironmentIntensity`] when intensity
    /// is negative or non-finite.
    pub fn new(diffuse_intensity: f32) -> Result<Self, LightingError> {
        if !diffuse_intensity.is_finite() || diffuse_intensity < 0.0 {
            return Err(LightingError::InvalidEnvironmentIntensity);
        }
        Ok(Self { diffuse_intensity })
    }

    #[must_use]
    pub const fn diffuse_intensity(self) -> f32 {
        self.diffuse_intensity
    }
}

impl Default for EnvironmentLight {
    fn default() -> Self {
        Self {
            diffuse_intensity: 1.0,
        }
    }
}

/// Invalid directional-light or shadow configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LightingError {
    InvalidDirection,
    InvalidColor,
    InvalidIlluminance,
    InvalidEnvironmentIntensity,
    InvalidCascadeCount(u8),
    InvalidShadowDistance,
    InvalidDepthBias,
    InvalidNormalBias,
    InvalidTexelWorldSize,
    InvalidShadowNearPlane,
}

impl Display for LightingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirection => {
                write!(formatter, "sun direction must be finite and non-zero")
            }
            Self::InvalidColor => write!(formatter, "sun color must be finite and within 0..=1"),
            Self::InvalidIlluminance => {
                write!(formatter, "sun illuminance must be finite and non-negative")
            }
            Self::InvalidEnvironmentIntensity => write!(
                formatter,
                "environment diffuse intensity must be finite and non-negative"
            ),
            Self::InvalidCascadeCount(count) => {
                write!(formatter, "shadow cascade count {count} is outside 1..=8")
            }
            Self::InvalidShadowDistance => {
                write!(formatter, "shadow distance must be finite and positive")
            }
            Self::InvalidDepthBias => write!(
                formatter,
                "shadow depth bias must be finite and non-negative"
            ),
            Self::InvalidNormalBias => {
                write!(
                    formatter,
                    "shadow normal bias must be finite and non-negative"
                )
            }
            Self::InvalidTexelWorldSize => {
                write!(
                    formatter,
                    "shadow texel world size must be finite and positive"
                )
            }
            Self::InvalidShadowNearPlane => {
                write!(formatter, "shadow camera near plane must be finite, positive, and below the shadow distance")
            }
        }
    }
}

impl Error for LightingError {}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    if !vector.iter().all(|value| value.is_finite()) {
        return None;
    }
    let length_squared = vector.iter().map(|value| value * value).sum::<f32>();
    if length_squared <= f32::EPSILON {
        return None;
    }
    let inverse_length = length_squared.sqrt().recip();
    Some(std::array::from_fn(|index| vector[index] * inverse_length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_normalizes_direction_and_defaults_to_stable_cascades() {
        let sun = SunLight::new([0.0, 2.0, 0.0], [1.0, 0.9, 0.8], 100_000.0)
            .expect("sun configuration is valid");
        let direction = sun.direction_to_light();
        assert!(direction[0].abs() < f32::EPSILON);
        assert!((direction[1] - 1.0).abs() < f32::EPSILON);
        assert!(direction[2].abs() < f32::EPSILON);
        assert_eq!(
            sun.shadows().expect("shadows are enabled").cascade_count(),
            4
        );
        assert!(sun
            .shadows()
            .expect("shadows are enabled")
            .stable_texel_snap());
    }

    #[test]
    fn invalid_sun_and_shadow_values_are_rejected() {
        assert_eq!(
            SunLight::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 1.0),
            Err(LightingError::InvalidDirection)
        );
        assert_eq!(
            SunLight::new([0.0, 1.0, 0.0], [1.1, 1.0, 1.0], 1.0),
            Err(LightingError::InvalidColor)
        );
        assert_eq!(
            ShadowSettings::new(0, 100.0, 0.0, 0.0, 1.0, true),
            Err(LightingError::InvalidCascadeCount(0))
        );
        assert_eq!(
            ShadowSettings::new(4, 100.0, -1.0, 0.0, 1.0, true),
            Err(LightingError::InvalidDepthBias)
        );
    }

    #[test]
    fn shadows_can_be_disabled_without_disabling_the_sun() {
        let sun = SunLight::new([1.0, 1.0, 0.0], [1.0, 1.0, 1.0], 50_000.0)
            .expect("sun configuration is valid")
            .with_shadows(None);
        assert_eq!(sun.shadows(), None);
        assert!(sun.illuminance_lux() > 0.0);
    }

    #[test]
    fn environment_light_validates_diffuse_intensity() {
        let environment = EnvironmentLight::new(0.75).expect("environment intensity is valid");
        assert!((environment.diffuse_intensity() - 0.75).abs() < f32::EPSILON);
        assert_eq!(
            EnvironmentLight::new(-0.1),
            Err(LightingError::InvalidEnvironmentIntensity)
        );
        assert_eq!(
            EnvironmentLight::new(f32::NAN),
            Err(LightingError::InvalidEnvironmentIntensity)
        );
    }

    #[test]
    fn cascade_layout_uses_monotonic_practical_splits_and_selects_ranges() {
        let settings = ShadowSettings::new(4, 200.0, 0.001, 0.02, 0.25, true)
            .expect("shadow settings are valid");
        let layout = settings
            .cascade_layout(0.1)
            .expect("near plane is inside the shadow range");

        assert_eq!(layout.cascades().len(), 4);
        assert!((layout.cascades()[0].near_distance() - 0.1).abs() < 0.000_001);
        assert!(
            (layout
                .cascades()
                .last()
                .expect("last cascade")
                .far_distance()
                - 200.0)
                .abs()
                < 0.0001
        );
        for pair in layout.cascades().windows(2) {
            assert!(pair[0].far_distance() < pair[1].far_distance());
            assert!((pair[1].near_distance() - pair[0].far_distance()).abs() < 0.0001);
        }
        assert_eq!(layout.select(0.05).expect("near range").index(), 0);
        assert_eq!(layout.select(199.0).expect("far range").index(), 3);
        assert_eq!(layout.select(f32::NAN), None);
    }

    #[test]
    fn cascade_layout_rejects_near_planes_outside_shadow_distance() {
        let settings = ShadowSettings::default();

        assert_eq!(
            settings.cascade_layout(0.0),
            Err(LightingError::InvalidShadowNearPlane)
        );
        assert_eq!(
            settings.cascade_layout(settings.max_distance()),
            Err(LightingError::InvalidShadowNearPlane)
        );
    }
}
