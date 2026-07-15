//! Fixed-stride, backend-neutral instance-buffer upload planning.
//!
//! The payload uses little-endian scalar encoding so the contract is stable
//! across CPU architectures.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_rhi::{BufferUsage, GpuBuffer, Rhi, RhiError};

use crate::{RenderInstance, RenderInstanceId, RenderUploadBatch, RenderUploadOperation};

/// The byte stride reserved for one render instance in the GPU instance buffer.
pub const INSTANCE_STRIDE_BYTES: usize = 64;
const INSTANCE_STRIDE_BYTES_U64: u64 = 64;

/// One deterministic write into an RHI-owned instance buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstanceBufferWrite {
    instance_id: RenderInstanceId,
    slot: u32,
    offset_bytes: u64,
    clear: bool,
    data: [u8; INSTANCE_STRIDE_BYTES],
}

impl InstanceBufferWrite {
    #[must_use]
    pub const fn instance_id(&self) -> RenderInstanceId {
        self.instance_id
    }

    #[must_use]
    pub const fn slot(&self) -> u32 {
        self.slot
    }

    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        self.offset_bytes
    }

    #[must_use]
    pub const fn is_clear(&self) -> bool {
        self.clear
    }

    #[must_use]
    pub const fn data(&self) -> &[u8; INSTANCE_STRIDE_BYTES] {
        &self.data
    }
}

/// The writes and frame metadata produced by one accepted render batch.
#[derive(Clone, Debug, PartialEq)]
pub struct InstanceUploadPlan {
    frame_id: u64,
    fixed_tick: u64,
    interpolation_alpha: f32,
    writes: Vec<InstanceBufferWrite>,
}

impl InstanceUploadPlan {
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
    pub fn writes(&self) -> &[InstanceBufferWrite] {
        &self.writes
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}

/// Stable slot allocation for renderer instances.
///
/// New instances take the lowest available slot. Removing an instance returns
/// its slot to the pool, so a later insertion reuses it deterministically.
#[derive(Clone, Debug)]
pub struct RenderInstanceBuffer {
    capacity: u32,
    slots: BTreeMap<RenderInstanceId, u32>,
    free_slots: BTreeSet<u32>,
}

impl RenderInstanceBuffer {
    /// Creates a bounded slot table for `capacity` instances.
    ///
    /// # Errors
    ///
    /// Returns [`InstanceBufferError::InvalidCapacity`] for zero capacity.
    pub fn new(capacity: u32) -> Result<Self, InstanceBufferError> {
        if capacity == 0 {
            return Err(InstanceBufferError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            slots: BTreeMap::new(),
            free_slots: (0..capacity).collect(),
        })
    }

    #[must_use]
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    #[must_use]
    pub fn slot_for(&self, id: RenderInstanceId) -> Option<u32> {
        self.slots.get(&id).copied()
    }

    /// Applies one renderer batch and returns fixed-stride writes.
    ///
    /// Allocation is transactional: if the batch exceeds capacity, neither
    /// the slot table nor the free-slot pool is changed.
    ///
    /// # Errors
    ///
    /// Returns [`InstanceBufferError::CapacityExceeded`] when a new instance
    /// cannot be assigned a slot.
    pub fn apply(
        &mut self,
        batch: &RenderUploadBatch,
    ) -> Result<InstanceUploadPlan, InstanceBufferError> {
        let mut slots = self.slots.clone();
        let mut free_slots = self.free_slots.clone();
        let mut writes = Vec::with_capacity(batch.operations().len());

        for operation in batch.operations() {
            match operation {
                RenderUploadOperation::Upsert(instance) => {
                    let slot = if let Some(slot) = slots.get(&instance.id).copied() {
                        slot
                    } else {
                        let slot = free_slots.iter().next().copied().ok_or(
                            InstanceBufferError::CapacityExceeded {
                                capacity: self.capacity,
                                instance_id: instance.id,
                            },
                        )?;
                        free_slots.remove(&slot);
                        slots.insert(instance.id, slot);
                        slot
                    };
                    writes.push(InstanceBufferWrite {
                        instance_id: instance.id,
                        slot,
                        offset_bytes: u64::from(slot) * INSTANCE_STRIDE_BYTES_U64,
                        clear: false,
                        data: encode_instance(instance, batch.interpolation_alpha()),
                    });
                }
                RenderUploadOperation::Remove(id) => {
                    if let Some(slot) = slots.remove(id) {
                        free_slots.insert(slot);
                        writes.push(InstanceBufferWrite {
                            instance_id: *id,
                            slot,
                            offset_bytes: u64::from(slot) * INSTANCE_STRIDE_BYTES_U64,
                            clear: true,
                            data: [0; INSTANCE_STRIDE_BYTES],
                        });
                    }
                }
            }
        }

        self.slots = slots;
        self.free_slots = free_slots;
        Ok(InstanceUploadPlan {
            frame_id: batch.frame_id(),
            fixed_tick: batch.fixed_tick(),
            interpolation_alpha: batch.interpolation_alpha(),
            writes,
        })
    }
}

/// Failure while maintaining the bounded instance-buffer slot table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceBufferError {
    InvalidCapacity,
    CapacityExceeded {
        capacity: u32,
        instance_id: RenderInstanceId,
    },
}

impl Display for InstanceBufferError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => write!(formatter, "instance buffer capacity must be non-zero"),
            Self::CapacityExceeded {
                capacity,
                instance_id,
            } => write!(
                formatter,
                "instance buffer capacity {capacity} exceeded by instance {}",
                instance_id.value()
            ),
        }
    }
}

impl Error for InstanceBufferError {}

/// RHI-backed instance storage that submits deterministic renderer writes.
pub struct GpuInstanceBuffer {
    buffer: GpuBuffer,
    slots: RenderInstanceBuffer,
}

impl GpuInstanceBuffer {
    /// Allocates a storage buffer sized for `capacity` instances.
    ///
    /// # Errors
    ///
    /// Returns [`GpuInstanceBufferError::Instance`] for an invalid capacity,
    /// [`GpuInstanceBufferError::SizeOverflow`] when the allocation size cannot
    /// be represented, or [`GpuInstanceBufferError::Rhi`] for device errors.
    pub fn new(rhi: &Rhi, label: &str, capacity: u32) -> Result<Self, GpuInstanceBufferError> {
        let slots =
            RenderInstanceBuffer::new(capacity).map_err(GpuInstanceBufferError::Instance)?;
        let size = u64::from(capacity)
            .checked_mul(INSTANCE_STRIDE_BYTES_U64)
            .ok_or(GpuInstanceBufferError::SizeOverflow)?;
        let buffer = rhi
            .create_buffer(label, size, BufferUsage::Storage)
            .map_err(GpuInstanceBufferError::Rhi)?;
        Ok(Self { buffer, slots })
    }

    #[must_use]
    pub const fn size(&self) -> u64 {
        self.buffer.size()
    }

    #[must_use]
    pub const fn capacity(&self) -> u32 {
        self.slots.capacity()
    }

    #[must_use]
    pub fn slot_for(&self, id: RenderInstanceId) -> Option<u32> {
        self.slots.slot_for(id)
    }

    /// Submits a batch through the RHI and commits slot state after success.
    ///
    /// The CPU-side slot table is prepared on a clone, so capacity failures and
    /// RHI validation/device errors do not advance renderer bookkeeping.
    ///
    /// # Errors
    ///
    /// Returns [`GpuInstanceBufferError::Instance`] for slot allocation errors
    /// or [`GpuInstanceBufferError::Rhi`] for a failed queue write.
    pub fn apply(
        &mut self,
        rhi: &Rhi,
        batch: &RenderUploadBatch,
    ) -> Result<InstanceUploadPlan, GpuInstanceBufferError> {
        let mut next_slots = self.slots.clone();
        let plan = next_slots
            .apply(batch)
            .map_err(GpuInstanceBufferError::Instance)?;
        for write in plan.writes() {
            rhi.write_buffer(&self.buffer, write.offset_bytes(), write.data())
                .map_err(GpuInstanceBufferError::Rhi)?;
        }
        self.slots = next_slots;
        Ok(plan)
    }
}

/// Failure while allocating or submitting an RHI-backed instance buffer.
#[derive(Debug)]
pub enum GpuInstanceBufferError {
    Instance(InstanceBufferError),
    SizeOverflow,
    Rhi(RhiError),
}

impl Display for GpuInstanceBufferError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instance(error) => Display::fmt(error, formatter),
            Self::SizeOverflow => write!(formatter, "instance buffer size overflowed"),
            Self::Rhi(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for GpuInstanceBufferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Instance(error) => Some(error),
            Self::SizeOverflow => None,
            Self::Rhi(error) => Some(error),
        }
    }
}

fn encode_instance(instance: &RenderInstance, interpolation_alpha: f32) -> [u8; 64] {
    let transform = instance.interpolated_transform(interpolation_alpha);
    let floats = [
        transform.translation[0],
        transform.translation[1],
        transform.translation[2],
        1.0,
        transform.rotation[0],
        transform.rotation[1],
        transform.rotation[2],
        transform.rotation[3],
        transform.scale[0],
        transform.scale[1],
        transform.scale[2],
        0.0,
    ];
    let words = [
        instance.mesh.0,
        instance.material.0,
        instance.flags.bits(),
        0,
    ];
    let mut data = [0; 64];
    for (index, value) in floats.into_iter().enumerate() {
        let start = index * 4;
        data[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (index, value) in words.into_iter().enumerate() {
        let start = 48 + index * 4;
        data[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaterialHandle, MeshHandle, RenderSnapshotBuilder, Transform};

    fn instance(id: u64, x: f32) -> RenderInstance {
        let mut instance = RenderInstance::new(
            RenderInstanceId::new(id),
            Transform::from_translation([x, 0.0, 0.0]),
            1.0,
            MeshHandle(7),
            MaterialHandle(9),
        );
        instance.previous_transform = Transform::from_translation([0.0, 0.0, 0.0]);
        instance
    }

    fn batch(
        frame_id: u64,
        instances: impl IntoIterator<Item = RenderInstance>,
    ) -> RenderUploadBatch {
        let mut snapshot = RenderSnapshotBuilder::new(frame_id, 12, 0.5);
        for instance in instances {
            snapshot.push(instance).expect("test IDs are unique");
        }
        let mut tracker = crate::RenderUploadTracker::default();
        tracker
            .diff(&snapshot.build())
            .expect("first frame is accepted")
    }

    fn read_f32(data: &[u8; 64], offset: usize) -> f32 {
        let bytes = data[offset..offset + 4]
            .try_into()
            .expect("test offset is four-byte aligned");
        f32::from_le_bytes(bytes)
    }

    #[test]
    fn encodes_interpolated_transform_and_material_metadata() {
        let mut buffer = RenderInstanceBuffer::new(2).expect("capacity is valid");
        let plan = buffer
            .apply(&batch(1, [instance(4, 10.0)]))
            .expect("one instance fits");
        let write = &plan.writes()[0];

        assert_eq!(write.slot(), 0);
        assert_eq!(write.offset_bytes(), 0);
        assert!((read_f32(write.data(), 0) - 5.0).abs() < f32::EPSILON);
        assert_eq!(
            u32::from_le_bytes(write.data()[48..52].try_into().unwrap()),
            7
        );
        assert_eq!(
            u32::from_le_bytes(write.data()[52..56].try_into().unwrap()),
            9
        );
        assert!(!write.is_clear());
    }

    #[test]
    fn slots_are_stable_and_reused_in_lowest_first_order() {
        let mut buffer = RenderInstanceBuffer::new(3).expect("capacity is valid");
        let first = buffer
            .apply(&batch(1, [instance(9, 0.0), instance(2, 0.0)]))
            .expect("two instances fit");
        assert_eq!(
            first
                .writes()
                .iter()
                .map(InstanceBufferWrite::slot)
                .collect::<Vec<_>>(),
            [0, 1]
        );

        let second = buffer
            .apply(&batch(2, [instance(9, 1.0)]))
            .expect("existing instance keeps its slot");
        assert_eq!(second.writes()[0].slot(), 1);
    }

    #[test]
    fn removal_clears_a_slot_and_next_insert_reuses_it() {
        let mut buffer = RenderInstanceBuffer::new(2).expect("capacity is valid");
        let mut tracker = crate::RenderUploadTracker::default();
        let mut initial = RenderSnapshotBuilder::new(1, 12, 0.5);
        initial.push(instance(1, 0.0)).expect("test ID is unique");
        initial.push(instance(2, 0.0)).expect("test ID is unique");
        let initial = initial.build();
        let initial_batch = tracker.diff(&initial).expect("initial frame is accepted");
        buffer.apply(&initial_batch).expect("initial instances fit");

        let mut next = RenderSnapshotBuilder::new(2, 12, 0.5);
        next.push(instance(2, 0.0)).expect("test ID is unique");
        let removal = tracker
            .diff(&next.build())
            .expect("newer frame is accepted");
        let plan = buffer.apply(&removal).expect("removal is valid");
        assert!(plan.writes().iter().any(InstanceBufferWrite::is_clear));
        assert_eq!(buffer.len(), 1);

        let mut final_snapshot = RenderSnapshotBuilder::new(3, 12, 0.5);
        final_snapshot
            .push(instance(2, 0.0))
            .expect("test ID is unique");
        final_snapshot
            .push(instance(3, 0.0))
            .expect("test ID is unique");
        let insertion = tracker
            .diff(&final_snapshot.build())
            .expect("newer frame is accepted");
        buffer.apply(&insertion).expect("freed slot is reused");
        assert_eq!(buffer.slot_for(RenderInstanceId::new(3)), Some(0));
    }

    #[test]
    fn capacity_failure_does_not_mutate_existing_slots() {
        let mut buffer = RenderInstanceBuffer::new(1).expect("capacity is valid");
        buffer
            .apply(&batch(1, [instance(1, 0.0)]))
            .expect("initial instance fits");
        let result = buffer.apply(&batch(2, [instance(2, 0.0)]));
        assert!(matches!(
            result,
            Err(InstanceBufferError::CapacityExceeded { capacity: 1, .. })
        ));
        assert_eq!(buffer.slot_for(RenderInstanceId::new(1)), Some(0));
        assert_eq!(buffer.len(), 1);
    }
}
