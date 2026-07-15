//! ECS-facing render extraction for the immutable renderer snapshot.

use bevy_ecs::prelude::{Component, Query, Res, ResMut, Resource};

use crate::{
    MaterialHandle, MeshHandle, RenderFlags, RenderInstance, RenderInstanceId, RenderSnapshot,
    RenderSnapshotBuilder, SnapshotError, Transform,
};

/// Render metadata supplied by the engine immediately before extraction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Resource)]
pub struct RenderExtractionFrame {
    frame_id: u64,
    fixed_tick: u64,
    interpolation_alpha: f32,
}

impl RenderExtractionFrame {
    #[must_use]
    pub fn new(frame_id: u64, fixed_tick: u64, interpolation_alpha: f32) -> Self {
        Self {
            frame_id,
            fixed_tick,
            interpolation_alpha: interpolation_alpha.clamp(0.0, 1.0),
        }
    }

    #[must_use]
    pub const fn frame_id(self) -> u64 {
        self.frame_id
    }

    #[must_use]
    pub const fn fixed_tick(self) -> u64 {
        self.fixed_tick
    }

    #[must_use]
    pub const fn interpolation_alpha(self) -> f32 {
        self.interpolation_alpha
    }
}

/// ECS component containing the renderer-neutral data needed for one instance.
#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub struct RenderInstanceSource {
    pub id: RenderInstanceId,
    pub previous_transform: Transform,
    pub transform: Transform,
    pub bounds_radius: f32,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub flags: RenderFlags,
}

impl RenderInstanceSource {
    #[must_use]
    pub fn new(
        id: RenderInstanceId,
        transform: Transform,
        bounds_radius: f32,
        mesh: MeshHandle,
        material: MaterialHandle,
    ) -> Self {
        let instance = RenderInstance::new(id, transform, bounds_radius, mesh, material);
        Self {
            id: instance.id,
            previous_transform: instance.previous_transform,
            transform: instance.transform,
            bounds_radius: instance.bounds_radius,
            mesh: instance.mesh,
            material: instance.material,
            flags: instance.flags,
        }
    }

    #[must_use]
    pub const fn to_render_instance(self) -> RenderInstance {
        RenderInstance {
            id: self.id,
            previous_transform: self.previous_transform,
            transform: self.transform,
            bounds_radius: self.bounds_radius,
            mesh: self.mesh,
            material: self.material,
            flags: self.flags,
        }
    }
}

/// Result of the most recent built-in extraction pass.
#[derive(Debug, Default, Resource)]
pub struct RenderExtractionOutput {
    snapshot: Option<RenderSnapshot>,
    error: Option<SnapshotError>,
}

impl RenderExtractionOutput {
    #[must_use]
    pub fn snapshot(&self) -> Option<&RenderSnapshot> {
        self.snapshot.as_ref()
    }

    #[must_use]
    pub const fn error(&self) -> Option<SnapshotError> {
        self.error
    }

    fn replace_snapshot(&mut self, snapshot: Result<RenderSnapshot, SnapshotError>) {
        match snapshot {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.error = None;
            }
            Err(error) => {
                self.snapshot = None;
                self.error = Some(error);
            }
        }
    }
}

/// Extracts all [`RenderInstanceSource`] components into a deterministic snapshot.
pub fn extract_render_instances(
    frame: Res<RenderExtractionFrame>,
    query: Query<&RenderInstanceSource>,
    mut output: ResMut<RenderExtractionOutput>,
) {
    let frame = frame.into_inner();
    let mut builder = RenderSnapshotBuilder::new(
        frame.frame_id(),
        frame.fixed_tick(),
        frame.interpolation_alpha(),
    );

    for source in &query {
        if let Err(error) = builder.push(source.to_render_instance()) {
            output.replace_snapshot(Err(error));
            return;
        }
    }

    output.replace_snapshot(Ok(builder.build()));
}
