//! Evidence artifact writers. PNG encoding stays outside runtime/RHI crates.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use meridian_assets::ArtifactHash;
use meridian_rhi::{CaptureSource, CapturedFrame, CapturedPixelFormat, FrameOutcome};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CaptureMetadata {
    pub schema: &'static str,
    pub width: u32,
    pub height: u32,
    pub format: &'static str,
    pub source: &'static str,
    pub frame_id: u64,
    pub capture_id: u64,
    pub pixel_hash: String,
    pub png_hash: String,
    pub surface_outcome: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureArtifact {
    pub png_path: PathBuf,
    pub metadata_path: PathBuf,
    pub metadata: CaptureMetadata,
}

/// Raw tightly-packed RGBA capture artifact used for exact pixel comparison.
///
/// PNG files remain the portable review artifact. This binary form deliberately
/// avoids encoder and metadata variability when a calibrated renderer profile
/// has an approved pixel fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawCaptureArtifact {
    pub rgba_path: PathBuf,
    pub pixel_hash: String,
}

/// Encodes one tightly packed RGBA8 sRGB frame and writes adjacent JSON metadata.
///
/// # Errors
///
/// Rejects invalid pixel lengths, unsupported formats, PNG failures, or IO failures.
pub fn write_capture_png(
    path: impl AsRef<Path>,
    frame: &CapturedFrame,
) -> Result<CaptureArtifact, CaptureWriteError> {
    validate_capture_pixels(frame)?;
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| CaptureWriteError::io("create parent", &error))?;
        }
    }
    let mut encoded = Vec::new();
    {
        let mut png_encoder = png::Encoder::new(&mut encoded, frame.width, frame.height);
        png_encoder.set_color(png::ColorType::Rgba);
        png_encoder.set_depth(png::BitDepth::Eight);
        png_encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut writer = png_encoder
            .write_header()
            .map_err(|error| CaptureWriteError::Png(error.to_string()))?;
        writer
            .write_image_data(&frame.pixels)
            .map_err(|error| CaptureWriteError::Png(error.to_string()))?;
        writer
            .finish()
            .map_err(|error| CaptureWriteError::Png(error.to_string()))?;
    }
    write_synced(&path, &encoded)?;
    let pixel_hash = ArtifactHash::digest(&frame.pixels);
    let png_hash = ArtifactHash::digest(&encoded);
    let metadata = CaptureMetadata {
        schema: "meridian.capture-metadata/v1",
        width: frame.width,
        height: frame.height,
        format: "rgba8-srgb",
        source: match frame.source {
            CaptureSource::PresentedSurface => "presented-surface",
            CaptureSource::Offscreen => "offscreen",
        },
        frame_id: frame.frame_id.get(),
        capture_id: frame.capture_id.get(),
        pixel_hash: pixel_hash.to_string(),
        png_hash: png_hash.to_string(),
        surface_outcome: frame
            .surface_outcome
            .map(frame_outcome_name)
            .map(str::to_owned),
    };
    let metadata_path = suffixed_path(&path, ".json");
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| CaptureWriteError::Json(error.to_string()))?;
    write_synced(&metadata_path, &metadata_bytes)?;
    Ok(CaptureArtifact {
        png_path: path,
        metadata_path,
        metadata,
    })
}

/// Writes one tightly packed RGBA8 sRGB capture without image-container
/// encoding.
///
/// The path is intended for profile-specific golden fixtures and is therefore
/// not a replacement for [`write_capture_png`]'s reviewable PNG plus metadata.
///
/// # Errors
///
/// Rejects invalid pixel lengths, unsupported formats, or IO failures.
pub fn write_capture_rgba(
    path: impl AsRef<Path>,
    frame: &CapturedFrame,
) -> Result<RawCaptureArtifact, CaptureWriteError> {
    validate_capture_pixels(frame)?;
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| CaptureWriteError::io("create parent", &error))?;
        }
    }
    write_synced(&path, &frame.pixels)?;
    Ok(RawCaptureArtifact {
        rgba_path: path,
        pixel_hash: ArtifactHash::digest(&frame.pixels).to_string(),
    })
}

/// Writes a pretty, durable JSON evidence artifact.
///
/// Evidence reports intentionally live outside runtime crates. Callers must
/// describe unavailable measurements explicitly instead of fabricating values.
///
/// # Errors
///
/// Returns serialization or IO failures.
pub fn write_evidence_json(
    path: impl AsRef<Path>,
    value: &impl Serialize,
) -> Result<PathBuf, CaptureWriteError> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| CaptureWriteError::io("create parent", &error))?;
        }
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| CaptureWriteError::Json(error.to_string()))?;
    write_synced(&path, &bytes)?;
    Ok(path)
}

/// Appends one canonical JSON object followed by a newline and synchronizes it.
///
/// The qualification runner uses JSONL so a terminated native process still
/// leaves every completed iteration inspectable. Callers retain responsibility
/// for a unique evidence path per invocation.
///
/// # Errors
///
/// Returns serialization or IO failures.
pub fn append_evidence_json_line(
    path: impl AsRef<Path>,
    value: &impl Serialize,
) -> Result<PathBuf, CaptureWriteError> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| CaptureWriteError::io("create parent", &error))?;
        }
    }
    let mut bytes =
        serde_json::to_vec(value).map_err(|error| CaptureWriteError::Json(error.to_string()))?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| CaptureWriteError::io("open append", &error))?;
    file.write_all(&bytes)
        .map_err(|error| CaptureWriteError::io("append", &error))?;
    file.sync_all()
        .map_err(|error| CaptureWriteError::io("sync append", &error))?;
    Ok(path)
}

#[must_use]
pub fn has_multiple_pixel_values(frame: &CapturedFrame) -> bool {
    let mut pixels = frame.pixels.chunks_exact(4);
    let Some(first) = pixels.next() else {
        return false;
    };
    pixels.any(|pixel| pixel != first)
}

fn frame_outcome_name(outcome: FrameOutcome) -> &'static str {
    match outcome {
        FrameOutcome::Presented => "presented",
        FrameOutcome::PresentedSuboptimal => "presented-suboptimal",
        FrameOutcome::SkippedZeroSize => "skipped-zero-size",
        FrameOutcome::SkippedTimeout => "skipped-timeout",
        FrameOutcome::SkippedOccluded => "skipped-occluded",
        FrameOutcome::ReconfiguredOutdated => "reconfigured-outdated",
        FrameOutcome::RecreatedLostSurface => "recreated-lost-surface",
        FrameOutcome::DeviceLost => "device-lost",
        FrameOutcome::UnsupportedSurface => "unsupported-surface",
    }
}

fn validate_capture_pixels(frame: &CapturedFrame) -> Result<(), CaptureWriteError> {
    if frame.format != CapturedPixelFormat::Rgba8Srgb {
        return Err(CaptureWriteError::UnsupportedFormat);
    }
    let expected = u64::from(frame.width)
        .checked_mul(u64::from(frame.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(CaptureWriteError::SizeOverflow)?;
    if frame.pixels.len() != expected {
        return Err(CaptureWriteError::InvalidPixelLength {
            actual: frame.pixels.len(),
            expected,
        });
    }
    Ok(())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), CaptureWriteError> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| CaptureWriteError::io("open", &error))?;
    file.write_all(bytes)
        .map_err(|error| CaptureWriteError::io("write", &error))?;
    file.sync_all()
        .map_err(|error| CaptureWriteError::io("sync", &error))?;
    Ok(())
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureWriteError {
    Io {
        operation: &'static str,
        message: String,
    },
    Png(String),
    Json(String),
    UnsupportedFormat,
    SizeOverflow,
    InvalidPixelLength {
        actual: usize,
        expected: usize,
    },
}

impl CaptureWriteError {
    fn io(operation: &'static str, error: &std::io::Error) -> Self {
        Self::Io {
            operation,
            message: error.to_string(),
        }
    }
}

impl Display for CaptureWriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, message } => {
                write!(formatter, "capture {operation} failed: {message}")
            }
            Self::Png(message) => write!(formatter, "PNG encoding failed: {message}"),
            Self::Json(message) => write!(formatter, "JSON serialization failed: {message}"),
            Self::UnsupportedFormat => {
                formatter.write_str("capture format is unsupported; expected rgba8-srgb")
            }
            Self::SizeOverflow => formatter.write_str("capture dimensions overflow host size"),
            Self::InvalidPixelLength { actual, expected } => write!(
                formatter,
                "capture has {actual} pixel bytes; expected {expected}"
            ),
        }
    }
}

impl Error for CaptureWriteError {}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::FrameId;
    use meridian_rhi::CaptureId;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn png_and_metadata_preserve_dimensions_hash_source_and_nonuniform_pixels() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("meridian-capture-{nonce}.png"));
        let frame = CapturedFrame {
            capture_id: CaptureId::new(3),
            frame_id: FrameId::new(4),
            width: 2,
            height: 1,
            format: CapturedPixelFormat::Rgba8Srgb,
            source: CaptureSource::Offscreen,
            surface_outcome: None,
            pixels: vec![255, 0, 0, 255, 0, 255, 0, 255],
        };
        let artifact = write_capture_png(&path, &frame).expect("capture writes");

        assert!(has_multiple_pixel_values(&frame));
        assert_eq!(artifact.metadata.width, 2);
        assert_eq!(artifact.metadata.height, 1);
        assert_eq!(artifact.metadata.source, "offscreen");
        assert_eq!(artifact.metadata.pixel_hash.len(), 64);
        assert!(artifact.png_path.metadata().expect("png metadata").len() > 8);
        fs::remove_file(artifact.png_path).expect("remove png");
        fs::remove_file(artifact.metadata_path).expect("remove metadata");
    }

    #[test]
    fn raw_rgba_preserves_exact_capture_bytes() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("meridian-capture-{nonce}.rgba"));
        let frame = CapturedFrame {
            capture_id: CaptureId::new(7),
            frame_id: FrameId::new(8),
            width: 2,
            height: 1,
            format: CapturedPixelFormat::Rgba8Srgb,
            source: CaptureSource::Offscreen,
            surface_outcome: None,
            pixels: vec![1, 2, 3, 255, 4, 5, 6, 255],
        };

        let artifact = write_capture_rgba(&path, &frame).expect("raw capture writes");

        assert_eq!(
            fs::read(&artifact.rgba_path).expect("raw bytes"),
            frame.pixels
        );
        assert_eq!(artifact.pixel_hash.len(), 64);
        fs::remove_file(artifact.rgba_path).expect("remove raw capture");
    }
}
