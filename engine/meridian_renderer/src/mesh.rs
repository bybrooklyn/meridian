//! Validated built-mesh realization into RHI-owned vertex and index buffers.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_rhi::{BufferUsage, GpuBuffer, Rhi, RhiError};

use crate::MeshResource;

/// GPU-resident geometry buffers for one built mesh.
///
/// The backend handles remain private inside [`GpuBuffer`]. This type only
/// exposes the metadata needed by a later renderer draw submission.
pub struct GpuMesh {
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    vertex_count: u32,
    index_count: u32,
    vertex_stride: u32,
}

impl GpuMesh {
    /// Allocates and uploads one built mesh's vertex and `u32` index data.
    ///
    /// The upload is transactional from the caller's perspective: validation
    /// completes before either GPU buffer is created, and queue writes happen
    /// only after both allocations succeed.
    ///
    /// # Errors
    ///
    /// Returns [`GpuMeshError`] when metadata and payload sizes disagree, an
    /// index is outside the vertex range, or the RHI rejects an allocation or
    /// upload.
    pub fn new(
        rhi: &Rhi,
        label: &str,
        resource: MeshResource,
        vertex_stride: u32,
        vertex_data: &[u8],
        indices: &[u32],
    ) -> Result<Self, GpuMeshError> {
        validate_mesh_upload(
            resource.vertex_count(),
            resource.index_count(),
            vertex_stride,
            vertex_data,
            indices,
        )?;

        let index_data = indices
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect::<Vec<_>>();
        let vertex_buffer = rhi
            .create_buffer(
                &format!("{label} vertices"),
                u64::try_from(vertex_data.len()).map_err(|_| GpuMeshError::SizeOverflow)?,
                BufferUsage::Vertex,
            )
            .map_err(GpuMeshError::Rhi)?;
        let index_buffer = rhi
            .create_buffer(
                &format!("{label} indices"),
                u64::try_from(index_data.len()).map_err(|_| GpuMeshError::SizeOverflow)?,
                BufferUsage::Index,
            )
            .map_err(GpuMeshError::Rhi)?;
        rhi.write_buffer(&vertex_buffer, 0, vertex_data)
            .map_err(GpuMeshError::Rhi)?;
        rhi.write_buffer(&index_buffer, 0, &index_data)
            .map_err(GpuMeshError::Rhi)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            vertex_count: resource.vertex_count(),
            index_count: resource.index_count(),
            vertex_stride,
        })
    }

    #[must_use]
    pub const fn vertex_buffer(&self) -> &GpuBuffer {
        &self.vertex_buffer
    }

    #[must_use]
    pub const fn index_buffer(&self) -> &GpuBuffer {
        &self.index_buffer
    }

    #[must_use]
    pub const fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    #[must_use]
    pub const fn index_count(&self) -> u32 {
        self.index_count
    }

    #[must_use]
    pub const fn vertex_stride(&self) -> u32 {
        self.vertex_stride
    }
}

/// Failure while validating or realizing built geometry.
#[derive(Debug)]
pub enum GpuMeshError {
    InvalidVertexStride(u32),
    VertexDataLength {
        expected_count: u32,
        actual_bytes: usize,
        stride: u32,
    },
    IndexCount {
        expected: u32,
        actual: usize,
    },
    IndexOutOfBounds {
        index: u32,
        vertex_count: u32,
    },
    SizeOverflow,
    Rhi(RhiError),
}

impl Display for GpuMeshError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVertexStride(stride) => {
                write!(formatter, "vertex stride must be non-zero and four-byte aligned: {stride}")
            }
            Self::VertexDataLength {
                expected_count,
                actual_bytes,
                stride,
            } => write!(
                formatter,
                "vertex payload has {actual_bytes} bytes; expected {expected_count} vertices at stride {stride}"
            ),
            Self::IndexCount { expected, actual } => {
                write!(formatter, "index payload has {actual} entries; expected {expected}")
            }
            Self::IndexOutOfBounds {
                index,
                vertex_count,
            } => write!(
                formatter,
                "mesh index {index} is outside vertex range 0..{vertex_count}"
            ),
            Self::SizeOverflow => write!(formatter, "mesh buffer size overflowed"),
            Self::Rhi(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for GpuMeshError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Rhi(error) => Some(error),
            _ => None,
        }
    }
}

fn validate_mesh_upload(
    vertex_count: u32,
    index_count: u32,
    vertex_stride: u32,
    vertex_data: &[u8],
    indices: &[u32],
) -> Result<(), GpuMeshError> {
    if vertex_stride == 0 || !vertex_stride.is_multiple_of(4) {
        return Err(GpuMeshError::InvalidVertexStride(vertex_stride));
    }
    let expected_bytes = usize::try_from(vertex_count)
        .ok()
        .and_then(|count| count.checked_mul(vertex_stride as usize));
    if expected_bytes != Some(vertex_data.len()) {
        return Err(GpuMeshError::VertexDataLength {
            expected_count: vertex_count,
            actual_bytes: vertex_data.len(),
            stride: vertex_stride,
        });
    }
    if indices.len() != usize::try_from(index_count).unwrap_or(usize::MAX) {
        return Err(GpuMeshError::IndexCount {
            expected: index_count,
            actual: indices.len(),
        });
    }
    if let Some(&index) = indices.iter().find(|&&index| index >= vertex_count) {
        return Err(GpuMeshError::IndexOutOfBounds {
            index,
            vertex_count,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_upload_validation_accepts_a_tightly_packed_triangle() {
        let vertices = [0_u8; 36];
        assert!(validate_mesh_upload(3, 3, 12, &vertices, &[0, 1, 2]).is_ok());
    }

    #[test]
    fn mesh_upload_validation_rejects_mismatched_payloads_and_indices() {
        assert!(matches!(
            validate_mesh_upload(3, 3, 12, &[0; 24], &[0, 1, 2]),
            Err(GpuMeshError::VertexDataLength { .. })
        ));
        assert!(matches!(
            validate_mesh_upload(3, 2, 12, &[0; 36], &[0, 1, 2]),
            Err(GpuMeshError::IndexCount { .. })
        ));
        assert!(matches!(
            validate_mesh_upload(3, 3, 12, &[0; 36], &[0, 1, 3]),
            Err(GpuMeshError::IndexOutOfBounds { .. })
        ));
        assert!(matches!(
            validate_mesh_upload(3, 3, 10, &[0; 30], &[0, 1, 2]),
            Err(GpuMeshError::InvalidVertexStride(10))
        ));
    }
}
