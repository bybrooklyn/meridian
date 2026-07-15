//! Deterministic change tracking between immutable snapshots and GPU uploads.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{RenderInstance, RenderInstanceId, RenderSnapshot};

/// One renderer-side instance-buffer change.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderUploadOperation {
    /// Insert a new instance or replace an existing instance's source data.
    Upsert(RenderInstance),
    /// Remove an instance that is absent from the newest snapshot.
    Remove(RenderInstanceId),
}

impl RenderUploadOperation {
    #[must_use]
    pub const fn id(&self) -> RenderInstanceId {
        match self {
            Self::Upsert(instance) => instance.id,
            Self::Remove(id) => *id,
        }
    }
}

/// Changes required to make the renderer's instance data match one snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderUploadBatch {
    frame_id: u64,
    fixed_tick: u64,
    interpolation_alpha: f32,
    operations: Vec<RenderUploadOperation>,
}

impl RenderUploadBatch {
    #[must_use]
    pub const fn frame_id(&self) -> u64 {
        self.frame_id
    }

    #[must_use]
    pub const fn fixed_tick(&self) -> u64 {
        self.fixed_tick
    }

    #[must_use]
    pub const fn interpolation_alpha(&self) -> f32 {
        self.interpolation_alpha
    }

    #[must_use]
    pub fn operations(&self) -> &[RenderUploadOperation] {
        &self.operations
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

/// Tracks the last accepted snapshot and emits only changed instance data.
#[derive(Debug, Default)]
pub struct RenderUploadTracker {
    previous: BTreeMap<RenderInstanceId, RenderInstance>,
    last_frame_id: Option<u64>,
}

impl RenderUploadTracker {
    /// Diffs a newer snapshot and advances the tracker only on success.
    ///
    /// # Errors
    ///
    /// Returns [`RenderUploadError::StaleFrame`] when a snapshot is older than
    /// or equal to the last accepted frame.
    pub fn diff(
        &mut self,
        snapshot: &RenderSnapshot,
    ) -> Result<RenderUploadBatch, RenderUploadError> {
        if let Some(last_frame_id) = self.last_frame_id {
            if snapshot.frame_id() <= last_frame_id {
                return Err(RenderUploadError::StaleFrame {
                    last_frame_id,
                    incoming_frame_id: snapshot.frame_id(),
                });
            }
        }

        let current = snapshot
            .instances()
            .iter()
            .map(|instance| (instance.id, instance.clone()))
            .collect::<BTreeMap<_, _>>();
        let current_ids = current.keys().copied().collect::<BTreeSet<_>>();
        let mut operations = BTreeMap::new();

        for (id, instance) in &current {
            if self.previous.get(id) != Some(instance) {
                operations.insert(*id, RenderUploadOperation::Upsert(instance.clone()));
            }
        }
        for id in self.previous.keys().filter(|id| !current_ids.contains(id)) {
            operations.insert(*id, RenderUploadOperation::Remove(*id));
        }

        self.previous = current;
        self.last_frame_id = Some(snapshot.frame_id());

        Ok(RenderUploadBatch {
            frame_id: snapshot.frame_id(),
            fixed_tick: snapshot.fixed_tick(),
            interpolation_alpha: snapshot.interpolation_alpha(),
            operations: operations.into_values().collect(),
        })
    }

    #[must_use]
    pub const fn last_frame_id(&self) -> Option<u64> {
        self.last_frame_id
    }

    pub fn reset(&mut self) {
        self.previous.clear();
        self.last_frame_id = None;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderUploadError {
    StaleFrame {
        last_frame_id: u64,
        incoming_frame_id: u64,
    },
}

impl Display for RenderUploadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleFrame {
                last_frame_id,
                incoming_frame_id,
            } => write!(
                formatter,
                "stale render frame {incoming_frame_id}; last accepted frame is {last_frame_id}"
            ),
        }
    }
}

impl Error for RenderUploadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaterialHandle, MeshHandle, RenderSnapshotBuilder, Transform};

    fn instance(id: u64, x: f32) -> RenderInstance {
        RenderInstance::new(
            RenderInstanceId::new(id),
            Transform::from_translation([x, 0.0, 0.0]),
            1.0,
            MeshHandle(1),
            MaterialHandle(1),
        )
    }

    fn snapshot(
        frame_id: u64,
        instances: impl IntoIterator<Item = RenderInstance>,
    ) -> RenderSnapshot {
        let mut builder = RenderSnapshotBuilder::new(frame_id, 4, 0.5);
        for instance in instances {
            builder.push(instance).expect("test IDs are unique");
        }
        builder.build()
    }

    #[test]
    fn first_snapshot_uploads_sorted_instances_and_next_unchanged_frame_is_empty() {
        let first = snapshot(1, [instance(9, 9.0), instance(2, 2.0)]);
        let second = snapshot(2, [instance(2, 2.0), instance(9, 9.0)]);
        let mut tracker = RenderUploadTracker::default();

        let initial = tracker.diff(&first).expect("first frame is accepted");
        assert_eq!(
            initial
                .operations()
                .iter()
                .map(RenderUploadOperation::id)
                .collect::<Vec<_>>(),
            [RenderInstanceId::new(2), RenderInstanceId::new(9)]
        );
        assert!(matches!(
            initial.operations()[0],
            RenderUploadOperation::Upsert(_)
        ));

        let unchanged = tracker.diff(&second).expect("newer frame is accepted");
        assert!(unchanged.is_empty());
        assert!((unchanged.interpolation_alpha() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn changed_and_removed_instances_emit_deterministic_operations() {
        let first = snapshot(1, [instance(1, 1.0), instance(2, 2.0)]);
        let second = snapshot(2, [instance(1, 10.0), instance(3, 3.0)]);
        let mut tracker = RenderUploadTracker::default();
        tracker.diff(&first).expect("first frame is accepted");

        let batch = tracker.diff(&second).expect("second frame is accepted");
        assert_eq!(
            batch
                .operations()
                .iter()
                .map(RenderUploadOperation::id)
                .collect::<Vec<_>>(),
            [
                RenderInstanceId::new(1),
                RenderInstanceId::new(2),
                RenderInstanceId::new(3)
            ]
        );
        assert!(matches!(
            batch.operations()[0],
            RenderUploadOperation::Upsert(_)
        ));
        assert_eq!(
            batch.operations()[1],
            RenderUploadOperation::Remove(RenderInstanceId::new(2))
        );
    }

    #[test]
    fn stale_frames_are_rejected_without_mutating_tracker_state() {
        let first = snapshot(5, [instance(1, 1.0)]);
        let stale = snapshot(4, [instance(1, 4.0)]);
        let mut tracker = RenderUploadTracker::default();
        tracker.diff(&first).expect("first frame is accepted");

        assert_eq!(
            tracker.diff(&stale),
            Err(RenderUploadError::StaleFrame {
                last_frame_id: 5,
                incoming_frame_id: 4,
            })
        );
        let next = tracker
            .diff(&snapshot(6, [instance(1, 1.0)]))
            .expect("tracker remains usable");
        assert!(next.is_empty());
    }
}
