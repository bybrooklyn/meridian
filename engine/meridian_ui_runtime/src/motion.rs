//! Bounded presentation motion that never owns layout or interaction authority.

use std::collections::BTreeMap;
use std::error::Error;
use std::f32::consts::TAU;
use std::fmt::{self, Display, Formatter};

use meridian_ui_core::{
    MotionPreference, UiColor, UiMotionTokens, UiNodeId, UiPoint, UiRect, UiSize,
    UiSpatialMotionKind, MAX_RETAINED_NODES,
};

const SETTLED_POSITION_EPSILON: f32 = 0.01;
const SETTLED_VELOCITY_EPSILON: f32 = 0.01;
const LOCKED_STATE_TRANSITION_MIN_MS: u16 = 100;
const LOCKED_STATE_TRANSITION_MAX_MS: u16 = 160;

/// Independent presentation channels supported by Meridian UI.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UiMotionChannel {
    Spatial,
    Opacity,
    Color,
}

/// Presentation value returned without changing authoritative geometry or state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiPresentationValue {
    Rect(UiRect),
    Opacity(f32),
    Color(UiColor),
}

/// Observable state for one bounded presentation track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiMotionSnapshot {
    pub node: UiNodeId,
    pub channel: UiMotionChannel,
    pub spatial_kind: Option<UiSpatialMotionKind>,
    pub current: UiPresentationValue,
    pub target: UiPresentationValue,
    pub active: bool,
}

#[derive(Clone, Copy, Debug)]
struct SpatialTrack {
    kind: UiSpatialMotionKind,
    current: [f32; 4],
    target: [f32; 4],
    velocity: [f32; 4],
    response_ms: u16,
    active: bool,
}

#[derive(Clone, Copy, Debug)]
struct StateTrack {
    start: [f32; 4],
    current: [f32; 4],
    target: [f32; 4],
    components: usize,
    elapsed_ms: u32,
    duration_ms: u16,
    active: bool,
}

#[derive(Clone, Copy, Debug)]
enum UiMotionTrack {
    Spatial(SpatialTrack),
    Opacity(StateTrack),
    Color(StateTrack),
}

/// Presentation-only motion registry bounded by the retained-node limit.
#[derive(Clone, Debug, Default)]
pub struct UiMotionSystem {
    tracks: BTreeMap<(UiNodeId, UiMotionChannel), UiMotionTrack>,
}

impl UiMotionSystem {
    /// Retargets a physical panel or shared element from its current presentation.
    ///
    /// Authoritative hit testing uses `target`, available immediately through
    /// [`Self::authoritative_rect`]. Reduced Motion snaps presentation as well.
    ///
    /// # Errors
    ///
    /// Rejects malformed geometry, invalid tokens, or aggregate overflow without
    /// changing an accepted track.
    pub fn retarget_spatial(
        &mut self,
        node: UiNodeId,
        initial: UiRect,
        target: UiRect,
        kind: UiSpatialMotionKind,
        preference: MotionPreference,
        tokens: UiMotionTokens,
    ) -> Result<(), UiMotionError> {
        validate_rect(initial)?;
        validate_rect(target)?;
        validate_tokens(tokens)?;
        self.reserve_track(node, UiMotionChannel::Spatial)?;
        let key = (node, UiMotionChannel::Spatial);
        let (current, velocity) = self
            .tracks
            .get(&key)
            .and_then(|track| match track {
                UiMotionTrack::Spatial(track) => Some((track.current, track.velocity)),
                UiMotionTrack::Opacity(_) | UiMotionTrack::Color(_) => None,
            })
            .unwrap_or_else(|| (rect_components(initial), [0.0; 4]));
        let target = rect_components(target);
        let reduced = preference == MotionPreference::Reduced;
        self.tracks.insert(
            key,
            UiMotionTrack::Spatial(SpatialTrack {
                kind,
                current: if reduced { target } else { current },
                target,
                velocity: if reduced { [0.0; 4] } else { velocity },
                response_ms: tokens.state_transition_max_ms,
                active: !reduced && components_differ(current, target),
            }),
        );
        Ok(())
    }

    /// Retargets opacity over the locked 100–160ms state-transition interval.
    ///
    /// # Errors
    ///
    /// Rejects invalid opacity, duration, tokens, or aggregate overflow.
    pub fn retarget_opacity(
        &mut self,
        node: UiNodeId,
        initial: f32,
        target: f32,
        duration_ms: u16,
        preference: MotionPreference,
        tokens: UiMotionTokens,
    ) -> Result<(), UiMotionError> {
        validate_unit(initial, UiMotionChannel::Opacity)?;
        validate_unit(target, UiMotionChannel::Opacity)?;
        self.retarget_state(
            node,
            UiMotionChannel::Opacity,
            [initial, 0.0, 0.0, 0.0],
            [target, 0.0, 0.0, 0.0],
            1,
            duration_ms,
            preference,
            tokens,
        )
    }

    /// Retargets a colour over the locked state-transition interval.
    ///
    /// # Errors
    ///
    /// Rejects invalid colour channels, duration, tokens, or aggregate overflow.
    pub fn retarget_color(
        &mut self,
        node: UiNodeId,
        initial: UiColor,
        target: UiColor,
        duration_ms: u16,
        preference: MotionPreference,
        tokens: UiMotionTokens,
    ) -> Result<(), UiMotionError> {
        validate_color(initial)?;
        validate_color(target)?;
        self.retarget_state(
            node,
            UiMotionChannel::Color,
            color_components(initial),
            color_components(target),
            4,
            duration_ms,
            preference,
            tokens,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn retarget_state(
        &mut self,
        node: UiNodeId,
        channel: UiMotionChannel,
        initial: [f32; 4],
        target: [f32; 4],
        components: usize,
        duration_ms: u16,
        preference: MotionPreference,
        tokens: UiMotionTokens,
    ) -> Result<(), UiMotionError> {
        validate_tokens(tokens)?;
        if !(tokens.state_transition_min_ms..=tokens.state_transition_max_ms).contains(&duration_ms)
        {
            return Err(UiMotionError::InvalidDuration {
                duration_ms,
                minimum_ms: tokens.state_transition_min_ms,
                maximum_ms: tokens.state_transition_max_ms,
            });
        }
        self.reserve_track(node, channel)?;
        let key = (node, channel);
        if let Some(existing) = self.tracks.get(&key) {
            let existing_target = match (channel, existing) {
                (UiMotionChannel::Opacity, UiMotionTrack::Opacity(track))
                | (UiMotionChannel::Color, UiMotionTrack::Color(track)) => Some(track.target),
                _ => None,
            };
            if existing_target
                .is_some_and(|existing_target| !components_differ(existing_target, target))
                && preference == MotionPreference::Full
            {
                return Ok(());
            }
        }
        let current = self
            .tracks
            .get(&key)
            .and_then(|track| match (channel, track) {
                (UiMotionChannel::Opacity, UiMotionTrack::Opacity(track))
                | (UiMotionChannel::Color, UiMotionTrack::Color(track)) => Some(track.current),
                _ => None,
            })
            .unwrap_or(initial);
        let reduced = preference == MotionPreference::Reduced;
        let track = StateTrack {
            start: current,
            current: if reduced { target } else { current },
            target,
            components,
            elapsed_ms: 0,
            duration_ms,
            active: !reduced && components_differ(current, target),
        };
        self.tracks.insert(
            key,
            match channel {
                UiMotionChannel::Opacity => UiMotionTrack::Opacity(track),
                UiMotionChannel::Color => UiMotionTrack::Color(track),
                UiMotionChannel::Spatial => return Err(UiMotionError::ChannelMismatch),
            },
        );
        Ok(())
    }

    fn reserve_track(&self, node: UiNodeId, channel: UiMotionChannel) -> Result<(), UiMotionError> {
        if self.tracks.contains_key(&(node, channel)) || self.tracks.len() < MAX_RETAINED_NODES {
            Ok(())
        } else {
            Err(UiMotionError::TooManyTracks {
                count: self.tracks.len().saturating_add(1),
                maximum: MAX_RETAINED_NODES,
            })
        }
    }

    /// Advances all presentation tracks by a caller-owned monotonic interval.
    pub fn advance(&mut self, delta_ms: u32) {
        if delta_ms == 0 {
            return;
        }
        for track in self.tracks.values_mut() {
            match track {
                UiMotionTrack::Spatial(track) => advance_spatial(track, delta_ms),
                UiMotionTrack::Opacity(track) | UiMotionTrack::Color(track) => {
                    advance_state(track, delta_ms);
                }
            }
        }
    }

    /// Applies a frame preference to already-retained tracks.
    ///
    /// Reduced Motion immediately settles every presentation channel; returning
    /// to full motion does not replay movement that the user asked to skip.
    pub fn apply_preference(&mut self, preference: MotionPreference) {
        if preference != MotionPreference::Reduced {
            return;
        }
        for track in self.tracks.values_mut() {
            match track {
                UiMotionTrack::Spatial(track) => {
                    track.current = track.target;
                    track.velocity = [0.0; 4];
                    track.active = false;
                }
                UiMotionTrack::Opacity(track) | UiMotionTrack::Color(track) => {
                    track.start = track.target;
                    track.current = track.target;
                    track.elapsed_ms = u32::from(track.duration_ms);
                    track.active = false;
                }
            }
        }
    }

    /// Returns one presentation snapshot when the channel exists.
    #[must_use]
    pub fn snapshot(&self, node: UiNodeId, channel: UiMotionChannel) -> Option<UiMotionSnapshot> {
        self.tracks.get(&(node, channel)).map(|track| match track {
            UiMotionTrack::Spatial(track) => UiMotionSnapshot {
                node,
                channel,
                spatial_kind: Some(track.kind),
                current: UiPresentationValue::Rect(components_rect(track.current)),
                target: UiPresentationValue::Rect(components_rect(track.target)),
                active: track.active,
            },
            UiMotionTrack::Opacity(track) => UiMotionSnapshot {
                node,
                channel,
                spatial_kind: None,
                current: UiPresentationValue::Opacity(track.current[0]),
                target: UiPresentationValue::Opacity(track.target[0]),
                active: track.active,
            },
            UiMotionTrack::Color(track) => UiMotionSnapshot {
                node,
                channel,
                spatial_kind: None,
                current: UiPresentationValue::Color(components_color(track.current)),
                target: UiPresentationValue::Color(components_color(track.target)),
                active: track.active,
            },
        })
    }

    /// Returns every retained track in stable node/channel order for an
    /// immutable runtime frame. The registry's retained-node cap bounds this
    /// allocation and no adapter state crosses this boundary.
    #[must_use]
    pub fn snapshots(&self) -> Vec<UiMotionSnapshot> {
        self.tracks
            .iter()
            .filter_map(|((node, channel), _)| self.snapshot(*node, *channel))
            .collect()
    }

    /// Returns authoritative spatial geometry, never lagging presentation motion.
    #[must_use]
    pub fn authoritative_rect(&self, node: UiNodeId) -> Option<UiRect> {
        self.tracks
            .get(&(node, UiMotionChannel::Spatial))
            .and_then(|track| match track {
                UiMotionTrack::Spatial(track) => Some(components_rect(track.target)),
                UiMotionTrack::Opacity(_) | UiMotionTrack::Color(_) => None,
            })
    }

    /// Removes every presentation channel for one no-longer-retained node.
    pub fn remove_node(&mut self, node: UiNodeId) {
        self.tracks.retain(|(track_node, _), _| *track_node != node);
    }

    /// Removes one presentation channel without disturbing other node motion.
    pub fn remove_channel(&mut self, node: UiNodeId, channel: UiMotionChannel) {
        self.tracks.remove(&(node, channel));
    }

    /// Reports tracks that are actively changing this frame, excluding settled history.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.tracks
            .values()
            .filter(|track| match track {
                UiMotionTrack::Spatial(track) => track.active,
                UiMotionTrack::Opacity(track) | UiMotionTrack::Color(track) => track.active,
            })
            .count()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

fn advance_spatial(track: &mut SpatialTrack, delta_ms: u32) {
    if !track.active {
        return;
    }
    let bounded_delta_ms = u16::try_from(delta_ms).unwrap_or(u16::MAX);
    let elapsed_seconds = f32::from(bounded_delta_ms) / 1_000.0;
    let response_seconds = f32::from(track.response_ms) / 1_000.0;
    let angular_frequency = TAU / response_seconds;
    for component in 0..4 {
        let displacement = track.current[component] - track.target[component];
        let velocity = track.velocity[component];
        let decay = (-angular_frequency * elapsed_seconds).exp();
        let coefficient = velocity + angular_frequency * displacement;
        track.current[component] =
            track.target[component] + (displacement + coefficient * elapsed_seconds) * decay;
        track.velocity[component] =
            (velocity - angular_frequency * coefficient * elapsed_seconds) * decay;
    }
    if (0..4).all(|component| {
        (track.current[component] - track.target[component]).abs() <= SETTLED_POSITION_EPSILON
            && track.velocity[component].abs() <= SETTLED_VELOCITY_EPSILON
    }) {
        track.current = track.target;
        track.velocity = [0.0; 4];
        track.active = false;
    }
}

fn advance_state(track: &mut StateTrack, delta_ms: u32) {
    if !track.active {
        return;
    }
    track.elapsed_ms = track.elapsed_ms.saturating_add(delta_ms);
    let elapsed = u16::try_from(track.elapsed_ms).unwrap_or(u16::MAX);
    let progress = (f32::from(elapsed) / f32::from(track.duration_ms)).clamp(0.0, 1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    for component in 0..track.components {
        track.current[component] =
            track.start[component] + (track.target[component] - track.start[component]) * eased;
    }
    if progress >= 1.0 {
        track.current = track.target;
        track.active = false;
    }
}

fn validate_tokens(tokens: UiMotionTokens) -> Result<(), UiMotionError> {
    if tokens.state_transition_min_ms != LOCKED_STATE_TRANSITION_MIN_MS
        || tokens.state_transition_max_ms != LOCKED_STATE_TRANSITION_MAX_MS
    {
        Err(UiMotionError::InvalidMotionTokens)
    } else {
        Ok(())
    }
}

fn validate_rect(rect: UiRect) -> Result<(), UiMotionError> {
    let values = [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ];
    if values.iter().all(|value| value.is_finite())
        && rect.size.width >= 0.0
        && rect.size.height >= 0.0
    {
        Ok(())
    } else {
        Err(UiMotionError::InvalidSpatialGeometry)
    }
}

fn validate_color(color: UiColor) -> Result<(), UiMotionError> {
    for value in color_components(color) {
        validate_unit(value, UiMotionChannel::Color)?;
    }
    Ok(())
}

fn validate_unit(value: f32, channel: UiMotionChannel) -> Result<(), UiMotionError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(UiMotionError::InvalidStateValue { channel })
    }
}

fn components_differ(left: [f32; 4], right: [f32; 4]) -> bool {
    left.into_iter()
        .zip(right)
        .any(|(left, right)| (left - right).abs() > f32::EPSILON)
}

const fn rect_components(rect: UiRect) -> [f32; 4] {
    [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ]
}

const fn components_rect(value: [f32; 4]) -> UiRect {
    UiRect::new(
        UiPoint {
            x: value[0],
            y: value[1],
        },
        UiSize::new(value[2], value[3]),
    )
}

const fn color_components(color: UiColor) -> [f32; 4] {
    [color.red, color.green, color.blue, color.alpha]
}

const fn components_color(value: [f32; 4]) -> UiColor {
    UiColor::rgba(value[0], value[1], value[2], value[3])
}

/// Typed rejection from the presentation-only motion boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiMotionError {
    TooManyTracks {
        count: usize,
        maximum: usize,
    },
    InvalidMotionTokens,
    InvalidDuration {
        duration_ms: u16,
        minimum_ms: u16,
        maximum_ms: u16,
    },
    InvalidSpatialGeometry,
    InvalidStateValue {
        channel: UiMotionChannel,
    },
    ChannelMismatch,
}

impl Display for UiMotionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid UI motion: {self:?}")
    }
}

impl Error for UiMotionError {}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKENS: UiMotionTokens = UiMotionTokens {
        state_transition_min_ms: 100,
        state_transition_max_ms: 160,
    };

    fn rect(x: f32) -> UiRect {
        UiRect::new(UiPoint { x, y: 0.0 }, UiSize::new(100.0, 44.0))
    }

    #[test]
    fn spatial_retarget_uses_current_presentation_and_immediate_hit_geometry() {
        let node = UiNodeId::new(1);
        let mut motion = UiMotionSystem::default();
        motion
            .retarget_spatial(
                node,
                rect(0.0),
                rect(200.0),
                UiSpatialMotionKind::PhysicalPanel,
                MotionPreference::Full,
                TOKENS,
            )
            .expect("physical panel track");
        assert_eq!(motion.authoritative_rect(node), Some(rect(200.0)));
        motion.advance(32);
        let before = motion
            .snapshot(node, UiMotionChannel::Spatial)
            .expect("spatial snapshot");
        motion
            .retarget_spatial(
                node,
                rect(0.0),
                rect(40.0),
                UiSpatialMotionKind::PhysicalPanel,
                MotionPreference::Full,
                TOKENS,
            )
            .expect("interrupted track retargets");
        assert_eq!(
            motion
                .snapshot(node, UiMotionChannel::Spatial)
                .expect("retargeted snapshot")
                .current,
            before.current
        );
        assert_eq!(motion.authoritative_rect(node), Some(rect(40.0)));
        motion.advance(2_000);
        assert!(
            !motion
                .snapshot(node, UiMotionChannel::Spatial)
                .expect("settled snapshot")
                .active
        );
    }

    #[test]
    fn reduced_motion_snaps_spatial_and_state_presentation() {
        let node = UiNodeId::new(2);
        let mut motion = UiMotionSystem::default();
        motion
            .retarget_spatial(
                node,
                rect(0.0),
                rect(80.0),
                UiSpatialMotionKind::SharedElement,
                MotionPreference::Reduced,
                TOKENS,
            )
            .expect("reduced spatial state");
        motion
            .retarget_opacity(node, 0.0, 1.0, 100, MotionPreference::Reduced, TOKENS)
            .expect("reduced opacity state");
        assert_eq!(
            motion
                .snapshot(node, UiMotionChannel::Spatial)
                .expect("spatial snapshot")
                .current,
            UiPresentationValue::Rect(rect(80.0))
        );
        assert_eq!(
            motion
                .snapshot(node, UiMotionChannel::Opacity)
                .expect("opacity snapshot")
                .current,
            UiPresentationValue::Opacity(1.0)
        );
    }

    #[test]
    fn state_reversal_starts_current_and_duration_uses_locked_tokens() {
        let node = UiNodeId::new(3);
        let mut motion = UiMotionSystem::default();
        motion
            .retarget_color(
                node,
                UiColor::background(),
                UiColor::amber(),
                160,
                MotionPreference::Full,
                TOKENS,
            )
            .expect("colour transition");
        motion.advance(80);
        let midpoint = motion
            .snapshot(node, UiMotionChannel::Color)
            .expect("colour midpoint")
            .current;
        motion
            .retarget_color(
                node,
                UiColor::background(),
                UiColor::surface(),
                100,
                MotionPreference::Full,
                TOKENS,
            )
            .expect("colour reversal");
        assert_eq!(
            motion
                .snapshot(node, UiMotionChannel::Color)
                .expect("reversed colour")
                .current,
            midpoint
        );
        assert!(matches!(
            motion.retarget_opacity(node, 0.0, 1.0, 99, MotionPreference::Full, TOKENS),
            Err(UiMotionError::InvalidDuration { .. })
        ));
        assert_eq!(
            motion.retarget_opacity(
                node,
                0.0,
                1.0,
                100,
                MotionPreference::Full,
                UiMotionTokens {
                    state_transition_min_ms: 1,
                    state_transition_max_ms: 1_000,
                },
            ),
            Err(UiMotionError::InvalidMotionTokens)
        );
    }
}
